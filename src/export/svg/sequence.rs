use super::Svg;
use crate::{
    geometry::{Bounds, Point},
    layout::{layer::ContentStack, sequence},
};
use svg::node::element as svg_element;

impl Svg {
    pub fn render_participant(&self, participant: &sequence::Participant) -> Box<dyn svg::Node> {
        let group = svg_element::Group::new();
        let component = &participant.component;

        // Use the renderer to generate the SVG for the participant
        let shape_group = component.drawable().render_to_svg();

        // Calculate where the lifeline should start (bottom of the shape)
        let component_bounds = component.bounds();
        let lifeline_start_y = component_bounds.max_y();
        let position = component.position();

        // Lifeline
        let lifeline = svg_element::Line::new()
            .set("x1", position.x())
            .set("y1", lifeline_start_y)
            .set("x2", position.x())
            .set("y2", participant.lifeline_end)
            .set("stroke", "Black") // TODO: &shape_def.line_color())
            .set("stroke-width", 1)
            .set("stroke-dasharray", "4");

        group.add(shape_group).add(lifeline).into()
    }

    pub fn render_message(
        &mut self,
        message: &sequence::Message,
        layout: &sequence::Layout,
    ) -> Box<dyn svg::Node> {
        let source = &layout.participants[message.source_index];
        let target = &layout.participants[message.target_index];

        let source_x = source.component.position().x();
        let target_x = target.component.position().x();
        let message_y = message.y_position;

        // Create points for the message line
        let start_point = Point::new(source_x, message_y);
        let end_point = Point::new(target_x, message_y);

        // Use the arrow_with_text from the message
        self.arrow_with_text_drawer.draw_arrow_with_text(
            message.arrow_with_text(),
            start_point,
            end_point,
        )
    }

    pub fn calculate_sequence_diagram_bounds(
        &self,
        content_stack: &ContentStack<sequence::Layout>,
    ) -> Bounds {
        let last_positioned_content = content_stack.iter().last();
        last_positioned_content
            .map(|positioned_content| {
                let layout = &positioned_content.content();

                if layout.participants.is_empty() {
                    return Bounds::default();
                }

                let mut content_bounds = layout
                    .participants
                    .iter()
                    .skip(1)
                    .map(|p| p.component.bounds())
                    .fold(layout.participants[0].component.bounds(), |acc, bounds| {
                        acc.merge(&bounds)
                    });

                content_bounds.set_max_y(
                    layout
                        .participants
                        .iter()
                        .map(|p| p.lifeline_end)
                        .fold(0.0, f32::max),
                );

                content_bounds
            })
            .unwrap_or_default()
    }
}
