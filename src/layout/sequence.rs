use crate::{
    ast,
    graph::Graph,
    layout::common::{Component, Point, calculate_element_size},
};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Participant<'a> {
    pub component: Component<'a>,
    pub lifeline_end: f32, // y-coordinate where lifeline ends
}

#[derive(Debug)]
pub struct Message<'a> {
    pub relation: &'a ast::Relation,
    pub source_index: usize,
    pub target_index: usize,
    pub y_position: f32,
}

#[derive(Debug)]
pub struct Layout<'a> {
    pub participants: Vec<Participant<'a>>,
    pub messages: Vec<Message<'a>>,
}

pub struct Engine {
    min_participant_width: f32,
    min_participant_height: f32,
    min_spacing: f32, // Minimum space between participants
    message_spacing: f32,
    top_margin: f32,
    text_padding: f32,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            min_participant_width: 80.0,
            min_participant_height: 30.0,
            min_spacing: 40.0, // Minimum spacing between participants
            message_spacing: 50.0,
            top_margin: 60.0,
            text_padding: 15.0,
        }
    }

    pub fn calculate<'a>(&self, graph: &'a Graph) -> Layout<'a> {
        let mut participants: Vec<Participant<'a>> = Vec::new();
        let mut participant_indices = HashMap::new();

        // Calculate text-based sizes for participants
        let participant_sizes: HashMap<_, _> = graph
            .node_indices()
            .map(|node_idx| {
                let node = graph.node_weight(node_idx).unwrap();
                let size = calculate_element_size(
                    node,
                    self.min_participant_width,
                    self.min_participant_height,
                    self.text_padding,
                );
                (node_idx, size)
            })
            .collect();

        // Process nodes in order and calculate individual positions
        let mut x_position = 0.0;

        for (i, node_idx) in graph.node_indices().enumerate() {
            let node = graph.node_weight(node_idx).unwrap();
            let size = participant_sizes.get(&node_idx).unwrap().clone();

            // For the first participant, we start at half its width
            if i == 0 {
                x_position = size.width / 2.0;
            }
            // For subsequent participants, we position based on previous participant and spacing
            else if let Some(last_participant) = participants.last() {
                // Get previous participant's width
                let prev_width = last_participant.component.size.width;

                // Move position by half of previous width + minimum spacing + half of current width
                x_position += (prev_width / 2.0) + self.min_spacing + (size.width / 2.0);
            }

            // Create participant at calculated position
            participants.push(Participant {
                component: Component {
                    node,
                    position: Point {
                        x: x_position,
                        y: self.top_margin,
                    },
                    size,
                },
                lifeline_end: self.top_margin, // Will be updated later
            });

            participant_indices.insert(node_idx, i);
        }

        // Calculate message positions and update lifeline ends
        let mut messages = Vec::new();
        let mut current_y = self.top_margin
            + participants
                .iter()
                .map(|p| p.component.size.height)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(self.min_participant_height)
            + self.message_spacing;

        for edge_idx in graph.edge_indices() {
            let (source_idx, target_idx) = graph.edge_endpoints(edge_idx).unwrap();
            let relation = graph.edge_weight(edge_idx).unwrap();

            let source_index = *participant_indices.get(&source_idx).unwrap();
            let target_index = *participant_indices.get(&target_idx).unwrap();

            messages.push(Message {
                relation,
                source_index,
                target_index,
                y_position: current_y,
            });

            // Update lifeline end for both source and target participants
            participants[source_index].lifeline_end = current_y;
            participants[target_index].lifeline_end = current_y;

            current_y += self.message_spacing;
        }

        // Update lifeline ends to match diagram height
        for participant in &mut participants {
            participant.lifeline_end = current_y + self.message_spacing;
        }

        Layout {
            participants,
            messages,
        }
    }
}
