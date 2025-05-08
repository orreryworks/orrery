use crate::{
    ast,
    layout::{
        common::{Bounds, Point},
        sequence,
    },
};
use svg::{
    Document,
    node::element::{Group, Line, Rectangle, Text},
};

use super::{Svg, arrows, renderer};

impl Svg {
    fn render_participant(&self, participant: &sequence::Participant) -> Group {
        let group = Group::new();
        let component = &participant.component;
        let type_def = &*component.node.type_definition;

        let has_nested_blocks = component.node.block.has_nested_blocks();

        // Use the shape_type to render the appropriate shape via the renderer
        let renderer = renderer::get_renderer(&*type_def.shape_type);

        // Use the renderer to generate the SVG for the participant
        let shape_group = renderer.render_to_svg(
            &component.position,
            &component.size,
            type_def,
            component.node.display_text(),
            has_nested_blocks,
        );

        // Calculate where the lifeline should start (bottom of the shape)
        let lifeline_start_y = component.position.y + component.size.height / 2.0;

        // Lifeline
        let lifeline = Line::new()
            .set("x1", component.position.x)
            .set("y1", lifeline_start_y)
            .set("x2", component.position.x)
            .set("y2", participant.lifeline_end)
            .set("stroke", &type_def.line_color)
            .set("stroke-width", 1)
            .set("stroke-dasharray", "4");

        group.add(shape_group).add(lifeline)
    }

    fn render_message(&self, message: &sequence::Message, layout: &sequence::Layout) -> Group {
        let mut group = Group::new();

        let source = &layout.participants[message.source_index];
        let target = &layout.participants[message.target_index];

        let source_x = source.component.position.x;
        let target_x = target.component.position.x;
        let message_y = message.y_position;

        // Create points for the message line
        let start_point = Point {
            x: source_x,
            y: message_y,
        };
        let end_point = Point {
            x: target_x,
            y: message_y,
        };

        // Create the path with appropriate markers - always use straight style for sequence diagrams
        let path = arrows::create_path(
            &start_point,
            &end_point,
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

            // Create a white background rectangle for better readability
            let bg = Rectangle::new()
                .set("x", mid_x - (label.len() as f32 * 3.5) - 5.0) // Add some padding
                .set("y", label_y - 15.0) // Position above the line
                .set("width", label.len() as f32 * 7.0 + 10.0) // Approximate width based on text length
                .set("height", 20.0)
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

    fn calculate_sequence_diagram_bounds(&self, layout: &sequence::Layout) -> Bounds {
        // Start with default bounds
        if layout.participants.is_empty() {
            return Bounds::default();
        }

        // For sequence diagrams, the bounds are defined by:
        // - The leftmost and rightmost participant positions
        // - The top of the first participant and the bottom of the lifelines
        let mut bounds = layout
            .participants
            .iter()
            .skip(1)
            .map(|p| p.component.bounds())
            .fold(layout.participants[0].component.bounds(), |acc, bounds| {
                acc.merge(&bounds)
            });

        bounds.max_y = layout
            .participants
            .iter()
            .map(|p| p.lifeline_end) // Bottom of lifelines
            .fold(f32::MIN, f32::max);

        bounds
    }

    pub fn render_sequence_diagram(&self, layout: &sequence::Layout) -> Document {
        // Calculate content bounds
        let content_bounds = self.calculate_sequence_diagram_bounds(layout);
        let content_size = content_bounds.to_size();

        // Calculate final SVG dimensions with margins
        let svg_size = self.calculate_svg_dimensions(&content_size);

        // Create the SVG document with the calculated dimensions
        let mut doc = Document::new()
            .set(
                "viewBox",
                format!("0 0 {} {}", svg_size.width, svg_size.height),
            )
            .set("width", svg_size.width)
            .set("height", svg_size.height);

        // Create marker definitions iterator for each color used in messages
        let message_colors = layout.messages.iter().map(|m| &m.relation.color);

        // Create marker definitions from collected colors
        let defs = arrows::create_marker_definitions(message_colors);

        doc = doc.add(defs);

        // Calculate margins for centering
        let margin_x = (svg_size.width - content_size.width) / 2.0;
        let margin_y = (svg_size.height - content_size.height) / 2.0;

        // Create group for all diagram elements with offset to account for min_x and min_y
        let mut diagram_group = Group::new();

        // Render participants
        for participant in &layout.participants {
            diagram_group = diagram_group.add(self.render_participant(participant));
        }

        // Render messages
        for message in &layout.messages {
            diagram_group = diagram_group.add(self.render_message(message, layout));
        }

        // Add transformation to center the diagram
        let transform = format!(
            "translate({}, {})",
            margin_x - content_bounds.min_x,
            margin_y - content_bounds.min_y
        );

        let main_group = Group::new().set("transform", transform).add(diagram_group);

        doc.add(main_group)
    }
}
