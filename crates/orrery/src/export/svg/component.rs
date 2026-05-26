//! SVG rendering for component diagrams.

use orrery_core::{
    draw::{LayeredOutput, PositionedArrowWithText},
    geometry::Bounds,
};

use super::Svg;
use crate::layout::{
    component::{Component, Layout},
    layer::ContentStack,
};

impl Svg {
    /// Renders a positioned component to layered SVG output.
    pub fn render_component(&self, component: &Component) -> LayeredOutput {
        component.drawable().render_to_layers()
    }

    /// Renders a positioned relation arrow to layered SVG output.
    pub fn render_relation(&mut self, relation: &PositionedArrowWithText) -> LayeredOutput {
        relation.render_to_layers(&mut self.arrow_with_text_drawer)
    }

    pub fn calculate_component_diagram_bounds(
        &self,
        content_stack: &ContentStack<Layout>,
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
