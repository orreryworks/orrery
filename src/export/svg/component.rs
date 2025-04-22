use super::renderer;
use crate::{
    ast::elaborate::RelationType,
    layout::common::{Bounds, Component, Point},
    layout::component,
};
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
        renderer.render_to_svg(
            &component.position,
            &component.size,
            type_def,
            &component.node.name,
        )
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

    pub fn render_component_diagram(&self, l: &component::Layout) -> Document {
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
                relation.relation,
            ));
        }

        // Apply a translation to center the diagram and add margins
        let transform_group = Group::new()
            .set("transform", format!("translate({}, {})", margin, margin))
            .add(main_group);

        doc.add(transform_group)
    }
}
