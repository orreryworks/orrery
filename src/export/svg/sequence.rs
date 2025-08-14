use super::Svg;
use crate::{
    draw::Drawable,
    geometry::{Bounds, Point},
    layout::{layer::ContentStack, sequence},
};
use svg::node::element as svg_element;

impl Svg {
    pub fn render_participant(&self, participant: &sequence::Participant) -> Box<dyn svg::Node> {
        let group = svg_element::Group::new();
        let component = participant.component();

        // Use the renderer to generate the SVG for the participant
        let shape_group = component.drawable().render_to_svg();

        // Render the pre-positioned lifeline from the participant
        let lifeline_svg = participant.lifeline().render_to_svg();

        group.add(shape_group).add(lifeline_svg).into()
    }

    pub fn render_message(
        &mut self,
        message: &sequence::Message,
        layout: &sequence::Layout,
    ) -> Box<dyn svg::Node> {
        let source = &layout.participants()[message.source_index()];
        let target = &layout.participants()[message.target_index()];
        let message_y = message.y_position();

        // Calculate source X coordinate with activation box intersection if active
        let source_x = sequence::calculate_message_endpoint_x(
            layout.activations(),
            source.component(),
            message.source_index(),
            message_y,
            target.component().position().x(), // Use target center X for direction detection
        );

        // Calculate target X coordinate with activation box intersection if active
        let target_x = sequence::calculate_message_endpoint_x(
            layout.activations(),
            target.component(),
            message.target_index(),
            message_y,
            source.component().position().x(), // Use source center X for direction detection
        );

        // Create points for the message line (Y coordinate unchanged)
        let start_point = Point::new(source_x, message_y);
        let end_point = Point::new(target_x, message_y);

        // Use the arrow_with_text from the message
        self.arrow_with_text_drawer.draw_arrow_with_text(
            message.arrow_with_text(),
            start_point,
            end_point,
        )
    }

    pub fn render_activation_box(
        &self,
        activation_box: &sequence::ActivationBox,
        layout: &sequence::Layout,
    ) -> Box<dyn svg::Node> {
        // Calculate the center position for the activation box
        let participant = &layout.participants()[activation_box.participant_index()];
        let participant_position = participant.component().position();
        let center_y = activation_box.center_y();
        let position = participant_position.with_y(center_y);

        // Use the drawable to render the activation box
        activation_box.drawable().render_to_svg(position)
    }

    pub fn calculate_sequence_diagram_bounds(
        &self,
        content_stack: &ContentStack<sequence::Layout>,
    ) -> Bounds {
        let last_positioned_content = content_stack.iter().last();
        last_positioned_content
            .map(|positioned_content| {
                let layout = &positioned_content.content();

                if layout.participants().is_empty() {
                    return Bounds::default();
                }

                let mut content_bounds = layout
                    .participants()
                    .iter()
                    .skip(1)
                    .map(|p| p.component().bounds())
                    .fold(
                        layout.participants()[0].component().bounds(),
                        |acc, bounds| acc.merge(&bounds),
                    );

                content_bounds.set_max_y(layout.max_lifeline_end());

                content_bounds
            })
            .unwrap_or_default()
    }
}
