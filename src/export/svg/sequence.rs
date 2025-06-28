use super::{Svg, arrows};
use crate::{
    draw::Drawable,
    geometry::{Bounds, Point},
    layout::{layer::ContentStack, sequence},
};
use svg::node::element::{Group, Line};

impl Svg {
    pub fn render_participant(&self, participant: &sequence::Participant) -> Box<dyn svg::Node> {
        let group = Group::new();
        let component = &participant.component;

        // Use the renderer to generate the SVG for the participant
        let shape_group = component.drawable().render_to_svg();

        // Calculate where the lifeline should start (bottom of the shape)
        let component_bounds = component.bounds();
        let lifeline_start_y = component_bounds.max_y();
        let position = component.position();

        // Lifeline
        let lifeline = Line::new()
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
        &self,
        message: &sequence::Message,
        layout: &sequence::Layout,
    ) -> Box<dyn svg::Node> {
        let mut group = Group::new();

        let source = &layout.participants[message.source_index];
        let target = &layout.participants[message.target_index];

        let source_x = source.component.position().x();
        let target_x = target.component.position().x();
        let message_y = message.y_position;

        // Create points for the message line
        let start_point = Point::new(source_x, message_y);
        let end_point = Point::new(target_x, message_y);

        // Create the path with appropriate markers - always use straight style for sequence diagrams
        let arrow_def = message.relation.arrow_definition();
        let path = arrows::create_path(
            start_point,
            end_point,
            &message.relation.relation_type,
            arrow_def,
        );

        // Add the path to the group
        group = group.add(path);

        // Add label if it exists
        if let Some(text) = message.relation.text() {
            // Calculate position for the label (slightly above the message line)
            let mid_x = (source_x + target_x) / 2.0;
            let label_y = message_y - 15.0; // 15px above the message line
            let text_position = Point::new(mid_x, label_y);

            let rendered_text = text.render_to_svg(text_position);

            group = group.add(rendered_text);
        }

        group.into()
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

    // This method was removed as it's no longer used directly - sequence diagram rendering
    // is now handled through the layered layout system
}
