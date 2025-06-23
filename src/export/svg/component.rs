use super::{Svg, arrows, renderer};
use crate::{
    layout::component,
    layout::layer::ContentStack,
    layout::{Bounds, Point},
};
use svg::node::element::{Group, Rectangle, Text};

impl Svg {
    // Find the point where a line from the shape entity to an external point intersects with the shape entity's boundary
    fn find_intersection(
        &self,
        shape_entity: &component::Component,
        external_point: Point,
    ) -> Point {
        shape_entity
            .shape()
            .find_intersection(shape_entity.position(), external_point)
    }

    pub fn render_component(&self, component: &component::Component) -> Group {
        let has_nested_blocks = component.has_nested_blocks();

        // Get the appropriate renderer based on the shape type
        let renderer = renderer::get_renderer(component.shape());

        // Use the renderer to generate the SVG for the main component
        renderer.render_to_svg(
            component.position(),
            component.shape(),
            component.text(),
            has_nested_blocks,
        )
    }

    pub fn render_relation(
        &self,
        source: &component::Component,
        target: &component::Component,
        relation: &component::LayoutRelation,
    ) -> Group {
        // Create a group to hold both the path and label
        let mut group = Group::new();

        // Calculate intersection points where the line meets each shape's boundary
        let source_edge = self.find_intersection(source, target.position());
        let target_edge = self.find_intersection(target, source.position());

        let ast_relation = relation.relation();
        let arrow_def = ast_relation.arrow_definition();
        // Create the path with appropriate markers
        let path = arrows::create_path(
            source_edge,
            target_edge,
            &ast_relation.relation_type,
            arrow_def,
        );

        // Add the path to the group
        group = group.add(path);

        // Add label if it exists
        if let Some(text) = relation.text() {
            let text_def = text.definition();
            let mid = source_edge.midpoint(target_edge);

            // Add a small offset to position the label above the line
            let offset_y = -10.0;

            let text_size = text.calculate_size();

            // Create a white background rectangle for better readability with correct dimensions
            let bg = Rectangle::new()
                .set("x", mid.x() - (text_size.width() / 2.0) - 5.0) // Center and add padding
                .set("y", mid.y() + offset_y - (text_size.height() / 2.0) - 5.0) // Position above the line
                .set("width", text_size.width() + 10.0) // Add padding to text width
                .set("height", text_size.height() + 10.0) // Add padding to text height
                .set("fill", "white")
                .set("fill-opacity", 0.8)
                .set("rx", 3.0); // Slightly rounded corners

            // Create the text label
            let text = Text::new("Text")
                .set("x", mid.x())
                .set("y", mid.y() + offset_y)
                .set("text-anchor", "middle")
                .set("dominant-baseline", "middle")
                .set("font-family", "Arial")
                .set("font-size", text_def.font_size())
                .add(svg::node::Text::new(text.content()));

            // Add background and text to the group
            group = group.add(bg).add(text);
        }

        group
    }

    pub fn calculate_component_diagram_bounds(
        &self,
        content_stack: &ContentStack<component::Layout>,
    ) -> Bounds {
        let last_positioned_content = content_stack.iter().last();
        last_positioned_content
            .map(|positioned_content| {
                let layout = &positioned_content.content();

                if layout.components.is_empty() {
                    return Bounds::default();
                }

                layout
                    .components
                    .iter()
                    .skip(1)
                    .map(|component| component.bounds())
                    .fold(layout.components[0].bounds(), |acc, bounds| {
                        acc.merge(&bounds)
                    })
            })
            .unwrap_or_default()
    }
}
