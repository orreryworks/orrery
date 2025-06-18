use crate::{
    ast,
    layout::{
        layer::ContentStack,
        sequence, text, {Bounds, Point},
    },
};
use svg::node::element::{Group, Line, Rectangle, Text};

use super::{Svg, arrows, renderer};

impl Svg {
    pub fn render_participant(&self, participant: &sequence::Participant) -> Group {
        let group = Group::new();
        let component = &participant.component;
        let shape_def = component.shape.definition();
        let type_def = &*component.node.type_definition;

        let has_nested_blocks = component.node.block.has_nested_blocks();

        // Use the shape from the component to render the appropriate shape via the renderer
        let renderer = renderer::get_renderer(&component.shape);

        // Use the renderer to generate the SVG for the participant
        let shape_group = renderer.render_to_svg(
            component.position,
            &component.shape,
            type_def,
            component.node.display_text(),
            has_nested_blocks,
        );

        // Calculate where the lifeline should start (bottom of the shape)
        let component_bounds = component.bounds();
        let lifeline_start_y = component_bounds.max_y();

        // Lifeline
        let lifeline = Line::new()
            .set("x1", component.position.x())
            .set("y1", lifeline_start_y)
            .set("x2", component.position.x())
            .set("y2", participant.lifeline_end)
            .set("stroke", &shape_def.line_color())
            .set("stroke-width", 1)
            .set("stroke-dasharray", "4");

        group.add(shape_group).add(lifeline)
    }

    pub fn render_message(&self, message: &sequence::Message, layout: &sequence::Layout) -> Group {
        let mut group = Group::new();

        let source = &layout.participants[message.source_index];
        let target = &layout.participants[message.target_index];

        let source_x = source.component.position.x();
        let target_x = target.component.position.x();
        let message_y = message.y_position;

        // Create points for the message line
        let start_point = Point::new(source_x, message_y);
        let end_point = Point::new(target_x, message_y);

        // Create the path with appropriate markers - always use straight style for sequence diagrams
        let path = arrows::create_path(
            start_point,
            end_point,
            &message.relation.relation_type,
            &message.relation.color,
            message.relation.width,
            &ast::ArrowStyle::Straight,
        );

        // Add the path to the group
        group = group.add(path);

        // Add label if it exists
        if let Some(label) = &message.relation.label {
            // Calculate position for the label (slightly above the message line)
            let mid_x = (source_x + target_x) / 2.0;
            let label_y = message_y - 15.0; // 15px above the message line

            // Calculate text dimensions using cosmic-text
            let text_size = text::calculate_text_size(label, 14);

            // Create a white background rectangle for better readability with correct dimensions
            let bg = Rectangle::new()
                .set("x", mid_x - (text_size.width() / 2.0) - 5.0) // Center and add padding
                .set("y", label_y - (text_size.height() / 2.0) - 5.0) // Position above the line
                .set("width", text_size.width() + 10.0) // Add padding to text width
                .set("height", text_size.height() + 10.0) // Add padding to text height
                .set("fill", "white")
                .set("fill-opacity", 0.8)
                .set("rx", 3.0); // Slightly rounded corners

            // Create the text label
            let text = Text::new("Text")
                .set("x", mid_x)
                .set("y", label_y)
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
