use crate::{
    ast,
    graph::Graph,
    layout::{
        common::{Component, Point},
        positioning::{self, calculate_element_size},
    },
};
use petgraph::graph::NodeIndex;
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
    label_padding: f32, // Padding to add for message labels
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
            label_padding: 20.0, // Extra padding for labels
        }
    }

    /// Calculate additional spacing needed between participants based on message label sizes
    fn calculate_message_label_spacing(
        &self,
        source_idx: usize,
        target_idx: usize,
        messages: &[(NodeIndex, NodeIndex, &ast::Relation)],
        participant_indices: &HashMap<NodeIndex, usize>,
    ) -> f32 {
        // Filter messages to only those between the two participants
        let relevant_messages = messages
            .iter()
            .filter_map(|(src_node, tgt_node, relation)| {
                if let (Some(&src_idx), Some(&tgt_idx)) = (
                    participant_indices.get(src_node),
                    participant_indices.get(tgt_node),
                ) {
                    if (src_idx == source_idx && tgt_idx == target_idx)
                        || (src_idx == target_idx && tgt_idx == source_idx)
                    {
                        return Some(*relation);
                    }
                }
                None
            });

        // Extract labels from relations and use shared function to calculate spacing
        let labels = relevant_messages.map(|relation| relation.label.as_ref());
        positioning::calculate_label_spacing(labels, self.label_padding)
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

        // Collect all messages to consider their labels for spacing
        let mut messages_vec = Vec::new();
        for edge_idx in graph.edge_indices() {
            let (source_idx, target_idx) = graph.edge_endpoints(edge_idx).unwrap();
            let relation = graph.edge_weight(edge_idx).unwrap();
            messages_vec.push((source_idx, target_idx, relation));
        }

        // Calculate additional spacings based on message labels
        let node_count = graph.node_indices().count();
        let mut spacings = Vec::with_capacity(node_count.saturating_sub(1));
        for i in 1..node_count {
            let spacing =
                self.calculate_message_label_spacing(i - 1, i, &messages_vec, &participant_indices);
            spacings.push(spacing);
        }

        // Get list of node indices and their sizes
        let node_indices: Vec<_> = graph.node_indices().collect();
        let sizes: Vec<_> = node_indices
            .iter()
            .map(|&idx| participant_sizes.get(&idx).unwrap().clone())
            .collect();

        // Calculate horizontal positions using positioning algorithms
        let x_positions =
            positioning::distribute_horizontally(&sizes, self.min_spacing, Some(&spacings), 0.0);

        // Create participants and store their indices
        for (i, node_idx) in node_indices.iter().enumerate() {
            let node = graph.node_weight(*node_idx).unwrap();
            let size = participant_sizes.get(node_idx).unwrap().clone();

            participants.push(Participant {
                component: Component {
                    node,
                    position: Point {
                        x: x_positions[i],
                        y: self.top_margin,
                    },
                    size,
                },
                lifeline_end: self.top_margin, // Will be updated later
            });

            participant_indices.insert(*node_idx, i);
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
