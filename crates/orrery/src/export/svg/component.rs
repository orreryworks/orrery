//! SVG rendering for component diagrams.

use orrery_core::{
    draw::{self, LayeredOutput},
    geometry::{Bounds, Point},
};

use super::Svg;
use crate::{layout::component, layout::layer::ContentStack};

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

    pub fn render_component(&self, component: &component::Component) -> LayeredOutput {
        component.drawable().render_to_layers()
    }

    pub fn render_relation(
        &mut self,
        source: &component::Component,
        target: &component::Component,
        arrow_with_text: &draw::ArrowWithText,
    ) -> LayeredOutput {
        // Calculate intersection points where the line meets each shape's boundary
        let source_edge = self.find_intersection(source, target.position());
        let target_edge = self.find_intersection(target, source.position());

        self.arrow_with_text_drawer
            .draw_arrow_with_text(arrow_with_text, source_edge, target_edge)
    }

    pub fn calculate_component_diagram_bounds(
        &self,
        content_stack: &ContentStack<component::Layout>,
    ) -> Bounds {
        let last_positioned_content = content_stack.iter().last();
        last_positioned_content
            .map(|positioned_content| {
                let layout = &positioned_content.content();

                if layout.components().is_empty() {
                    return Bounds::default();
                }

                layout
                    .components()
                    .iter()
                    .skip(1)
                    .map(|component| component.bounds())
                    .fold(layout.components()[0].bounds(), |acc, bounds| {
                        acc.merge(&bounds)
                    })
            })
            .unwrap_or_default()
    }
}
