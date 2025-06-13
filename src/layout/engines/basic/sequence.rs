//! Basic sequence layout engine
//!
//! This module provides a layout engine for sequence diagrams
//! using a simple, deterministic algorithm.

use crate::{
    ast,
    graph::Graph,
    layout::{
        engines::{self, EmbeddedLayouts, SequenceEngine},
        geometry::{Component, Point},
        layer::{ContentStack, PositionedContent},
        positioning::{self, calculate_bounded_text_size},
        sequence::{Layout, Message, Participant},
    },
};
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

/// Basic sequence layout engine implementation that implements the SequenceLayoutEngine trait
pub struct Engine {
    min_participant_width: f32,
    min_participant_height: f32,
    min_spacing: f32, // Minimum space between participants
    message_spacing: f32,
    top_margin: f32,
    padding: f32,
    label_padding: f32, // Padding to add for message labels
}

impl Engine {
    /// Create a new basic sequence layout engine
    pub fn new() -> Self {
        Self {
            min_participant_width: 80.0,
            min_participant_height: 30.0,
            min_spacing: 40.0, // Minimum spacing between participants
            message_spacing: 50.0,
            top_margin: 60.0,
            padding: 15.0,
            label_padding: 20.0, // Extra padding for labels
        }
    }

    /// Set the minimum width for participants
    #[allow(dead_code)]
    pub fn set_min_width(&mut self, width: f32) -> &mut Self {
        self.min_participant_width = width;
        self
    }

    /// Set the minimum height for participants
    #[allow(dead_code)]
    pub fn set_min_height(&mut self, height: f32) -> &mut Self {
        self.min_participant_height = height;
        self
    }

    /// Set the minimum spacing between participants
    pub fn set_min_spacing(&mut self, spacing: f32) -> &mut Self {
        self.min_spacing = spacing;
        self
    }

    /// Set the vertical spacing between messages
    pub fn set_message_spacing(&mut self, spacing: f32) -> &mut Self {
        self.message_spacing = spacing;
        self
    }

    /// Set the top margin of the diagram
    #[allow(dead_code)]
    pub fn set_top_margin(&mut self, margin: f32) -> &mut Self {
        self.top_margin = margin;
        self
    }

    /// Set the text padding for participants
    #[allow(dead_code)]
    pub fn set_text_padding(&mut self, padding: f32) -> &mut Self {
        self.padding = padding;
        self
    }

    /// Set the padding for message labels
    #[allow(dead_code)]
    pub fn set_label_padding(&mut self, padding: f32) -> &mut Self {
        self.label_padding = padding;
        self
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

    /// Calculate layout for a sequence diagram
    pub fn calculate_layout<'a>(
        &self,
        graph: &'a Graph<'a>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> ContentStack<Layout<'a>> {
        let mut participants: Vec<Participant<'a>> = Vec::new();
        let mut participant_indices = HashMap::new();

        // Calculate sizes for participants, accounting for embedded diagrams
        let participant_sizes: HashMap<_, _> = graph
            .nodes_with_indices()
            .map(|(node_idx, node)| {
                // Check if this node has an embedded diagram
                let size = if let ast::Block::Diagram(_) = &node.block {
                    // If this participant has an embedded diagram, use its layout size
                    if let Some(layout) = embedded_layouts.get(&node.id) {
                        // Use the shared utility function to calculate size
                        engines::embedded_layout_size(
                            layout,
                            node,
                            self.min_participant_width,
                            self.min_participant_height,
                            self.padding,
                            self.padding, // Using padding for text_padding too
                        )
                    } else {
                        // Fallback to text-based sizing if no embedded layout found
                        calculate_bounded_text_size(
                            node,
                            self.min_participant_width,
                            self.min_participant_height,
                            self.padding,
                        )
                    }
                } else {
                    // Regular participant with no embedded diagram
                    calculate_bounded_text_size(
                        node,
                        self.min_participant_width,
                        self.min_participant_height,
                        self.padding,
                    )
                };

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
        let sizes: Vec<_> = graph
            .node_indices()
            .map(|idx| *participant_sizes.get(&idx).unwrap())
            .collect();

        // Calculate horizontal positions using positioning algorithms
        let x_positions =
            positioning::distribute_horizontally(&sizes, self.min_spacing, Some(&spacings));

        // Create participants and store their indices
        for (i, (node_idx, node)) in graph.nodes_with_indices().enumerate() {
            let size = *participant_sizes.get(&node_idx).unwrap();

            participants.push(Participant {
                component: Component {
                    node,
                    position: Point::new(x_positions[i], self.top_margin),
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
                .map(|p| p.component.size.height())
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

        let layout = Layout {
            participants,
            messages,
        };

        let mut content_stack = ContentStack::new();
        content_stack.push(PositionedContent::new(layout));
        content_stack
    }
}

impl Engine {}

impl SequenceEngine for Engine {
    fn calculate<'a>(
        &self,
        graph: &'a Graph<'a>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> ContentStack<Layout<'a>> {
        self.calculate_layout(graph, embedded_layouts)
    }
}
