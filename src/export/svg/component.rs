use super::renderer;
use crate::{
    ast::elaborate::RelationType,
    export,
    layout::common::{Bounds, Component, Point},
    layout::component,
};
use log::{debug, info, trace};
use std::collections::HashSet;
use svg::{
    node::element::{Definitions, Group, Marker, Path},
    Document,
};

use super::Svg;

impl Svg {
    // Find the point where a line from the shape entity to an external point intersects with the shape entity's boundary
    fn find_intersection(&self, shape_entity: &Component, external_point: &Point) -> Point {
        let type_def = &*shape_entity.node.type_definition;
        type_def.shape_type.find_intersection(
            &shape_entity.position,
            external_point,
            &shape_entity.size,
        )
    }

    fn render_component(&self, component: &Component) -> Group {
        // Use the shape_type to render the appropriate shape via the renderer
        let type_def = &*component.node.type_definition;

        // Get the appropriate renderer based on the shape type
        let renderer = renderer::get_renderer(&*type_def.shape_type);

        // Use the renderer to generate the SVG for the main component
        let mut component_group = renderer.render_to_svg(
            &component.position,
            &component.size,
            type_def,
            &component.node.name,
        );

        // Check if the component has an inner block and render it
        if let crate::ast::elaborate::Block::Scope(scope) = &component.node.block {
            // Render inner block elements and add them to the component group
            let inner_block_group = self.render_inner_block(component, scope);
            component_group = component_group.add(inner_block_group);
        } else if let crate::ast::elaborate::Block::Diagram(diagram) = &component.node.block {
            // Render inner diagram and add it to the component group
            let inner_diagram_group = self.render_inner_diagram(component, diagram);
            component_group = component_group.add(inner_diagram_group);
        }

        component_group
    }

    fn render_relation(
        &self,
        source: &Component,
        target: &Component,
        relation: &crate::ast::elaborate::Relation,
    ) -> Path {
        // Calculate intersection points where the line meets each shape's boundary
        let source_edge = self.find_intersection(source, &target.position);
        let target_edge = self.find_intersection(target, &source.position);

        // Get marker references for this specific color
        let (start_marker, end_marker) = match &relation.relation_type {
            RelationType::Forward => (
                None,
                Some(format!(
                    "url(#arrow-right-{})",
                    relation.color.to_id_safe_string()
                )),
            ),
            RelationType::Backward => (
                Some(format!(
                    "url(#arrow-left-{})",
                    relation.color.to_id_safe_string()
                )),
                None,
            ),
            RelationType::Bidirectional => (
                Some(format!(
                    "url(#arrow-left-{})",
                    relation.color.to_id_safe_string()
                )),
                Some(format!(
                    "url(#arrow-right-{})",
                    relation.color.to_id_safe_string()
                )),
            ),
            RelationType::Plain => (None, None),
        };

        // Create the path
        let mut path = Path::new()
            .set(
                "d",
                self.create_path_data_from_points(&source_edge, &target_edge),
            )
            .set("fill", "none")
            .set("stroke", relation.color.to_string())
            .set("stroke-width", relation.width);

        // Add markers if they exist
        if let Some(marker) = start_marker {
            path = path.set("marker-start", marker);
        }

        if let Some(marker) = end_marker {
            path = path.set("marker-end", marker);
        }

        path
    }

    fn calculate_component_diagram_bounds(&self, l: &component::Layout) -> Bounds {
        // If there are no components, return default bounds
        if l.components.is_empty() {
            return Bounds::default();
        }

        l.components
            .iter()
            .skip(1)
            .map(|component| component.bounds())
            .fold(l.components[0].bounds(), |acc, bounds| acc.merge(&bounds))
    }

    /// Render an inner block (Scope) within a component
    fn render_inner_block(&self, parent: &Component, scope: &crate::ast::elaborate::Scope) -> Group {
        // Create a layout engine for the inner components
        let layout_engine = component::Engine::new();
        
        // Extract nodes from the scope and convert to a graph
        let mut graph = petgraph::graph::DiGraph::new();
        let mut node_map = std::collections::HashMap::new();
        let mut relations = Vec::new();
        
        // Process all elements in the scope
        for element in &scope.elements {
            match element {
                crate::ast::elaborate::Element::Node(node) => {
                    let node_idx = graph.add_node(node.clone());
                    node_map.insert(&node.id, node_idx);
                },
                crate::ast::elaborate::Element::Relation(relation) => {
                    relations.push(relation);
                }
            }
        }
        
        // Add relations after all nodes are added
        for relation in relations {
            if let (Some(&source_idx), Some(&target_idx)) = (
                node_map.get(&relation.source),
                node_map.get(&relation.target),
            ) {
                graph.add_edge(source_idx, target_idx, relation.clone());
            }
        }
        
        // If the scope is empty, return an empty group
        if graph.node_count() == 0 {
            return Group::new();
        }
        
        // Calculate layout for inner components
        let layout = layout_engine.calculate(&graph);
        
        // Create a group for inner components with appropriate scaling and positioning
        let mut inner_group = Group::new();
        
        // Calculate inner block bounds
        let inner_bounds = self.calculate_component_diagram_bounds(&layout);
        let inner_size = inner_bounds.to_size();
        
        // Skip rendering if inner size is zero to avoid scaling issues
        if inner_size.width <= 0.0 || inner_size.height <= 0.0 {
            return Group::new();
        }
        
        // Calculate scaling factor based on parent component size
        // Leave some padding inside the parent component
        let padding_ratio = 0.8; // Use 80% of the parent component to leave padding
        
        // Calculate scaling factor to fit inner diagram within parent component
        let scale_x = (parent.size.width * padding_ratio) / inner_size.width.max(1.0);
        let scale_y = (parent.size.height * padding_ratio) / inner_size.height.max(1.0);
        let scale = scale_x.min(scale_y);
        
        // Calculate center point of inner content before scaling
        let inner_center_x = inner_bounds.min_x + (inner_size.width / 2.0);
        let inner_center_y = inner_bounds.min_y + (inner_size.height / 2.0);
        
        // Calculate translation to position inner block within parent
        // Center the inner block in the parent component after scaling
        let translate_x = parent.position.x - (inner_center_x * scale);
        let translate_y = parent.position.y - (inner_center_y * scale);
        
        // Render each inner component
        for component in &layout.components {
            let component_group = self.render_component(component);
            inner_group = inner_group.add(component_group);
        }
        
        // Render relations
        for relation in &layout.relations {
            inner_group = inner_group.add(self.render_relation(
                layout.source(relation),
                layout.target(relation),
                &relation.relation,
            ));
        }
        
        // Apply transform to scale and position inner block
        inner_group.set(
            "transform", 
            format!("translate({}, {}) scale({})", translate_x, translate_y, scale)
        )
    }
    
    /// Render an inner diagram within a component
    fn render_inner_diagram(&self, parent: &Component, diagram: &crate::ast::elaborate::Diagram) -> Group {
        // Create a layout engine based on the diagram kind
        match diagram.kind {
            crate::ast::elaborate::DiagramKind::Component => {
                // For component diagrams, we can use the scope directly
                self.render_inner_block(parent, &diagram.scope)
            }
            crate::ast::elaborate::DiagramKind::Sequence => {
                // For sequence diagrams, we need a different approach
                // This is simplified as sequence diagrams are more complex to nest
                let group = Group::new();
                let text = svg::node::element::Text::new("Nested sequence diagram")
                    .set("x", parent.position.x)
                    .set("y", parent.position.y)
                    .set("text-anchor", "middle")
                    .set("dominant-baseline", "middle")
                    .set("font-size", parent.node.type_definition.font_size);
                group.add(text)
            }
        }
    }

    fn render_component_diagram(&self, l: &component::Layout) -> Document {
        // Calculate content dimensions using the bounds method
        let content_bounds = self.calculate_component_diagram_bounds(l);
        let content_size = content_bounds.to_size();

        // Calculate final SVG dimensions (including margins)
        let svg_size = self.calculate_svg_dimensions(&content_size);

        // Create new document with calculated dimensions
        let mut doc = Document::new()
            .set(
                "viewBox",
                format!("0 0 {} {}", svg_size.width, svg_size.height),
            )
            .set("width", svg_size.width)
            .set("height", svg_size.height);

        // Create marker definitions for each color used in the relationships
        let mut defs = Definitions::new();
        let mut marker_colors = HashSet::new();

        // Collect all unique colors used in relations
        for relation in &l.relations {
            marker_colors.insert(&relation.relation.color);
        }

        // Create markers for each color
        for color in &marker_colors {
            // Right-pointing arrow marker for this color
            let arrow_right = Marker::new()
                .set("id", format!("arrow-right-{}", color.to_id_safe_string()))
                .set("viewBox", "0 0 10 10")
                .set("refX", 9)
                .set("refY", 5)
                .set("markerWidth", 6)
                .set("markerHeight", 6)
                .set("orient", "auto")
                .add(
                    Path::new()
                        .set("d", "M 0 0 L 10 5 L 0 10 z")
                        .set("fill", color.to_string()),
                );

            // Left-pointing arrow marker for this color
            let arrow_left = Marker::new()
                .set("id", format!("arrow-left-{}", color.to_id_safe_string()))
                .set("viewBox", "0 0 10 10")
                .set("refX", 1)
                .set("refY", 5)
                .set("markerWidth", 6)
                .set("markerHeight", 6)
                .set("orient", "auto")
                .add(
                    Path::new()
                        .set("d", "M 10 0 L 0 5 L 10 10 z")
                        .set("fill", color.to_string()),
                );

            defs = defs.add(arrow_right).add(arrow_left);
        }

        doc = doc.add(defs);

        // Calculate padding to center the content
        let margin = (svg_size.width - content_size.width) / 2.0;

        // Position all components with a translation to account for the margins
        let mut main_group = Group::new();

        for component in &l.components {
            // Create a positioned component by adjusting for margin
            let positioned_component = self.render_component(component);
            main_group = main_group.add(positioned_component);
        }

        for relation in &l.relations {
            main_group = main_group.add(self.render_relation(
                l.source(relation),
                l.target(relation),
                &relation.relation,
            ));
        }

        // Apply a translation to center the diagram and add margins
        let transform_group = Group::new()
            .set("transform", format!("translate({}, {})", margin, margin))
            .add(main_group);

        doc.add(transform_group)
    }

    /// Export a component diagram layout to SVG
    pub fn export_component_layout(&self, l: &component::Layout) -> Result<(), export::Error> {
        debug!("Starting Component SVG export to file: {}", self.file_name);

        // Render the SVG document
        let doc = self.render_component_diagram(l);
        trace!("SVG document rendered");

        // Create the output file
        info!("Creating SVG file: {}", self.file_name);

        // Write the document to file
        self.write_document(doc)
    }
}
