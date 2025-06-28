use super::{Svg, arrows};
use crate::{
    draw::Drawable,
    geometry::{Bounds, Point},
    layout::component,
    layout::layer::ContentStack,
};
use svg::node::element::Group;

impl Svg {
    // Find the point where a line from the shape entity to an external point intersects with the shape entity's boundary
    fn find_intersection(
        &self,
        shape_entity: &component::Component,
        external_point: Point,
    ) -> Point {
        shape_entity
            .drawable()
            .inner()
            .find_intersection(shape_entity.position(), external_point)
    }

    pub fn render_component(&self, component: &component::Component) -> Box<dyn svg::Node> {
        component.drawable().render_to_svg()
    }

    pub fn render_relation(
        &self,
        source: &component::Component,
        target: &component::Component,
        relation: &component::LayoutRelation,
    ) -> Box<dyn svg::Node> {
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
            let mid = source_edge.midpoint(target_edge);

            // Add a small offset to position the label above the line
            let offset_y = -10.0;
            let text_position = Point::new(mid.x(), mid.y() + offset_y);

            let rendered_text = text.render_to_svg(text_position);

            group = group.add(rendered_text);
        }

        group.into()
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
