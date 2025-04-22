use crate::{
    ast::elaborate::RelationType,
    layout::{
        common::{Bounds, Point},
        sequence,
    },
};
use svg::{
    node::element::{Definitions, Group, Line, Marker, Path},
    Document,
};

use super::{renderer, Svg};

impl Svg {
    fn render_participant(&self, participant: &sequence::Participant) -> Group {
        let group = Group::new();
        let component = &participant.component;
        let type_def = &*component.node.type_definition;

        // Use the shape_type to render the appropriate shape via the renderer
        let renderer = renderer::get_renderer(&*type_def.shape_type);

        // Use the renderer to generate the SVG for the participant
        let shape_group = renderer.render_to_svg(
            &component.position,
            &component.size,
            type_def,
            &component.node.name,
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
        let group = Group::new();

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

        // Get marker references for this specific color
        let (start_marker, end_marker) = match &message.relation.relation_type {
            RelationType::Forward => (
                None,
                Some(format!(
                    "url(#arrow-right-{})",
                    message.relation.color.to_id_safe_string()
                )),
            ),
            RelationType::Backward => (
                Some(format!(
                    "url(#arrow-left-{})",
                    message.relation.color.to_id_safe_string()
                )),
                None,
            ),
            RelationType::Bidirectional => (
                Some(format!(
                    "url(#arrow-left-{})",
                    message.relation.color.to_id_safe_string()
                )),
                Some(format!(
                    "url(#arrow-right-{})",
                    message.relation.color.to_id_safe_string()
                )),
            ),
            RelationType::Plain => (None, None),
        };

        // Create the path
        let mut path = Path::new()
            .set(
                "d",
                self.create_path_data_from_points(&start_point, &end_point),
            )
            .set("fill", "none")
            .set("stroke", message.relation.color.to_string())
            .set("stroke-width", message.relation.width);

        // Add markers if they exist
        if let Some(marker) = start_marker {
            path = path.set("marker-start", marker);
        }

        if let Some(marker) = end_marker {
            path = path.set("marker-end", marker);
        }

        group.add(path)
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
            .fold(f32::MIN, |a, b| a.max(b));

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

        // Create marker definitions for each color used in the messages
        let mut defs = Definitions::new();
        let mut marker_colors = std::collections::HashSet::new();

        // Collect all unique colors used in messages
        for message in &layout.messages {
            marker_colors.insert(&message.relation.color);
        }

        // Create markers for each color
        for color in &marker_colors {
            // Right-pointing arrow marker for this color
            let arrow_right = Marker::new()
                .set("id", format!("arrow-right-{}", color.to_id_safe_string()))
                .set("viewBox", "0 0 10 10")
                .set("refX", 9)
                .set("refY", 5)
                .set("markerWidth", 6)
                .set("markerHeight", 6)
                .set("orient", "auto")
                .add(
                    Path::new()
                        .set("d", "M 0 0 L 10 5 L 0 10 z")
                        .set("fill", color.to_string()),
                );

            // Left-pointing arrow marker for this color
            let arrow_left = Marker::new()
                .set("id", format!("arrow-left-{}", color.to_id_safe_string()))
                .set("viewBox", "0 0 10 10")
                .set("refX", 1)
                .set("refY", 5)
                .set("markerWidth", 6)
                .set("markerHeight", 6)
                .set("orient", "auto")
                .add(
                    Path::new()
                        .set("d", "M 10 0 L 0 5 L 10 10 z")
                        .set("fill", color.to_string()),
                );

            defs = defs.add(arrow_right).add(arrow_left);
        }

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
