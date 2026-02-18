//! SVG rendering for sequence diagrams.

use filament_core::{
    draw::{self, Drawable as _, LayeredOutput},
    geometry::{Bounds, Point},
};

use super::Svg;
use crate::layout::{layer::ContentStack, sequence};

impl Svg {
    pub fn render_participant(&self, participant: &sequence::Participant) -> LayeredOutput {
        let mut output = LayeredOutput::new();
        let component = participant.component();

        // Use the renderer to generate the SVG for the participant
        let shape_output = component.drawable().render_to_layers();
        output.merge(shape_output);

        // Render the pre-positioned lifeline from the participant
        let lifeline_output = participant.lifeline().render_to_layers();
        output.merge(lifeline_output);

        output
    }

    pub fn render_message(
        &mut self,
        message: &sequence::Message,
        layout: &sequence::Layout,
    ) -> LayeredOutput {
        let source = &layout.participants()[&message.source()];
        let target = &layout.participants()[&message.target()];
        let message_y = message.y_position();

        // Calculate source X coordinate with activation box intersection if active
        let source_x = sequence::calculate_message_endpoint_x(
            layout.activations(),
            source.component(),
            message.source(),
            message_y,
            target.component().position().x(), // Use target center X for direction detection
        );

        // Calculate target X coordinate with activation box intersection if active
        let target_x = sequence::calculate_message_endpoint_x(
            layout.activations(),
            target.component(),
            message.target(),
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

    /// Renders a fragment box in a sequence diagram.
    ///
    /// Converts a fragment into its SVG representation.
    ///
    /// # Arguments
    ///
    /// * `fragment` - The fragment to render with its sections and bounds.
    ///
    /// # Returns
    ///
    /// A [`LayeredOutput`] representing the fragment.
    pub fn render_fragment(
        &self,
        fragment: &draw::PositionedDrawable<draw::Fragment>,
    ) -> LayeredOutput {
        fragment.render_to_layers()
    }

    /// Renders a note in a sequence diagram.
    ///
    /// Converts a note into its SVG representation.
    ///
    /// # Arguments
    ///
    /// * `note` - The positioned note to render.
    ///
    /// # Returns
    ///
    /// A [`LayeredOutput`] representing the note.
    pub fn render_note(&self, note: &draw::PositionedDrawable<draw::Note>) -> LayeredOutput {
        note.render_to_layers()
    }

    pub fn render_activation_box(
        &self,
        activation_box: &sequence::ActivationBox,
        layout: &sequence::Layout,
    ) -> LayeredOutput {
        // Calculate the center position for the activation box
        let participant = &layout.participants()[&activation_box.participant_id()];
        let participant_position = participant.component().position();
        let center_y = activation_box.center_y();
        let position = participant_position.with_y(center_y);

        // Use the drawable to render the activation box
        activation_box.drawable().render_to_layers(position)
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
                    .values()
                    .map(|p| p.component().bounds())
                    .reduce(|acc, bounds| acc.merge(&bounds))
                    .unwrap_or_default();

                // Include notes in bounds calculation
                for note in layout.notes() {
                    let note_bounds = note.position().to_bounds(note.size());
                    content_bounds = content_bounds.merge(&note_bounds);
                }

                content_bounds.with_max_y(layout.max_lifeline_end())
            })
            .unwrap_or_default()
    }
}
