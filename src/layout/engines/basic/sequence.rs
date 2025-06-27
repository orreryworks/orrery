//! Basic sequence layout engine
//!
//! This module provides a layout engine for sequence diagrams
//! using a simple, deterministic algorithm.

use crate::draw::Drawable;
use crate::{
    ast, draw,
    geometry::{Insets, Point, Size},
    graph::Graph,
    layout::{
        component::Component,
        engines::{EmbeddedLayouts, SequenceEngine},
        layer::{ContentStack, PositionedContent},
        sequence::{Layout, Message, Participant, adjust_positioned_contents_offset},
    },
};
use petgraph::graph::NodeIndex;
use std::{collections::HashMap, rc::Rc};

/// Basic sequence layout engine implementation that implements the SequenceLayoutEngine trait
pub struct Engine {
    min_spacing: f32, // Minimum space between participants
    message_spacing: f32,
    top_margin: f32,
    padding: Insets,
    label_padding: f32, // Padding to add for message labels
}

impl Engine {
    /// Create a new basic sequence layout engine
    pub fn new() -> Self {
        Self {
            min_spacing: 40.0, // Minimum spacing between participants
            message_spacing: 50.0,
            top_margin: 60.0,
            padding: Insets::uniform(15.0),
            label_padding: 20.0, // Extra padding for labels
        }
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
    pub fn set_text_padding(&mut self, padding: Insets) -> &mut Self {
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
        let labels = relevant_messages.map(|relation| relation.text());
        crate::layout::positioning::calculate_label_spacing(labels, self.label_padding)
    }

    /// Calculate layout for a sequence diagram
    pub fn calculate_layout<'a>(
        &self,
        graph: &'a Graph<'a>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> ContentStack<Layout<'a>> {
        let mut participants: Vec<Participant<'a>> = Vec::new();
        let mut participant_indices = HashMap::new();

        // Create shapes with text for participants
        let mut participant_shapes: HashMap<_, _> = graph
            .nodes_with_indices()
            .map(|(node_idx, node)| {
                let mut shape = draw::Shape::new(Rc::clone(&node.type_definition.shape_definition));
                shape.set_padding(self.padding);
                let text = draw::Text::new(
                    Rc::clone(&node.type_definition.text_definition),
                    node.display_text().to_string(),
                );
                let mut shape_with_text = draw::ShapeWithText::new(shape, Some(text));

                let content_size = if let ast::Block::Diagram(_) = &node.block {
                    // If this participant has an embedded diagram, use its layout size
                    if let Some(layout) = embedded_layouts.get(&node.id) {
                        layout.calculate_size()
                    } else {
                        Size::default()
                    }
                } else {
                    // Regular participant with no embedded diagram
                    Size::default()
                };

                shape_with_text.set_inner_content_size(content_size);
                (node_idx, shape_with_text)
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
            .map(|idx| {
                let shape_with_text = participant_shapes.get(&idx).unwrap();
                shape_with_text.shape_size()
            })
            .collect();

        // Calculate horizontal positions using positioning algorithms
        let x_positions = crate::layout::positioning::distribute_horizontally(
            &sizes,
            self.min_spacing,
            Some(&spacings),
        );

        // Create participants and store their indices
        for (i, (node_idx, node)) in graph.nodes_with_indices().enumerate() {
            let shape_with_text = participant_shapes.remove(&node_idx).unwrap();
            let position = Point::new(x_positions[i], self.top_margin);

            participants.push(Participant {
                component: Component::new(node, shape_with_text, position),
                lifeline_end: self.top_margin, // Will be updated later
            });

            participant_indices.insert(node_idx, i);
        }

        // Calculate message positions and update lifeline ends
        let mut messages = Vec::new();
        let mut current_y = self.top_margin
            + participants
                .iter()
                .map(|p| p.component.drawable().size().height())
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or_default()
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

        adjust_positioned_contents_offset(&mut content_stack, graph);

        content_stack
    }
}

impl SequenceEngine for Engine {
    fn calculate<'a>(
        &self,
        graph: &'a Graph<'a>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> ContentStack<Layout<'a>> {
        self.calculate_layout(graph, embedded_layouts)
    }
}
