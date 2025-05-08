use super::{arrows, renderer};
use crate::{
    ast,
    layout::common::{Bounds, Component, Point},
    layout::component,
    layout::text,
};
use svg::{
    Document,
    node::element::{Group, Rectangle, Text},
};

use super::Svg;

impl Svg {
    // Find the point where a line from the shape entity to an external point intersects with the shape entity's boundary
    fn find_intersection(&self, shape_entity: &Component, external_point: Point) -> Point {
        let type_def = &*shape_entity.node.type_definition;
        type_def.shape_type.find_intersection(
            shape_entity.position,
            external_point,
            &shape_entity.size,
        )
    }

    fn render_component(&self, component: &Component) -> Group {
        // Use the shape_type to render the appropriate shape via the renderer
        let type_def = &*component.node.type_definition;

        let has_nested_blocks = component.node.block.has_nested_blocks();

        // Get the appropriate renderer based on the shape type
        let renderer = renderer::get_renderer(&*type_def.shape_type);

        // Use the renderer to generate the SVG for the main component
        renderer.render_to_svg(
            component.position,
            &component.size,
            type_def,
            component.node.display_text(),
            has_nested_blocks,
        )
    }

    fn render_relation(
        &self,
        source: &Component,
        target: &Component,
        relation: &ast::Relation,
    ) -> Group {
        // Create a group to hold both the path and label
        let mut group = Group::new();

        // Calculate intersection points where the line meets each shape's boundary
        let source_edge = self.find_intersection(source, target.position);
        let target_edge = self.find_intersection(target, source.position);

        // Create the path with appropriate markers
        let path = arrows::create_path(
            source_edge,
            target_edge,
            &relation.relation_type,
            &relation.color,
            relation.width,
            &relation.arrow_style,
        );

        // Add the path to the group
        group = group.add(path);

        // Add label if it exists
        if let Some(label) = &relation.label {
            // Calculate midpoint for the label
            let mid_x = (source_edge.x + target_edge.x) / 2.0;
            let mid_y = (source_edge.y + target_edge.y) / 2.0;

            // Add a small offset to position the label above the line
            let offset_y = -10.0;

            // Calculate text dimensions
            let text_size = text::calculate_text_size(label, 14);

            // Create a white background rectangle for better readability with correct dimensions
            let bg = Rectangle::new()
                .set("x", mid_x - (text_size.width / 2.0) - 5.0) // Center and add padding
                .set("y", mid_y + offset_y - (text_size.height / 2.0) - 5.0) // Position above the line
                .set("width", text_size.width + 10.0) // Add padding to text width
                .set("height", text_size.height + 10.0) // Add padding to text height
                .set("fill", "white")
                .set("fill-opacity", 0.8)
                .set("rx", 3.0); // Slightly rounded corners

            // Create the text label
            let text = Text::new("Text")
                .set("x", mid_x)
                .set("y", mid_y + offset_y)
                .set("text-anchor", "middle")
                .set("dominant-baseline", "middle")
                .set("font-family", "Arial")
                .set("font-size", 14)
                .add(svg::node::Text::new(label));

            // Add background and text to the group
            group = group.add(bg).add(text);
        }

        group
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

        // Create marker definitions iterator for each color used in relations
        let relation_colors = l.relations.iter().map(|r| &r.relation.color);

        // Create marker definitions from collected colors
        let defs = arrows::create_marker_definitions(relation_colors);

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
            .set("transform", format!("translate({margin}, {margin})"))
            .add(main_group);

        doc.add(transform_group)
    }
}
