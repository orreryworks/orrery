//! Basic sequence layout engine
//!
//! This module provides a layout engine for sequence diagrams
//! using a simple, deterministic algorithm.

use crate::{
    ast,
    draw::{self, Drawable as _},
    geometry::{Insets, Point, Size},
    graph::{Event, Graph},
    layout::{
        component::Component,
        engines::{EmbeddedLayouts, SequenceEngine},
        layer::{ContentStack, PositionedContent},
        sequence::{
            ActivationBox, ActivationTiming, Layout, Message, Participant,
            adjust_positioned_contents_offset,
        },
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
                ) && ((src_idx == source_idx && tgt_idx == target_idx)
                    || (src_idx == target_idx && tgt_idx == source_idx))
                {
                    return Some(*relation);
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
        embedded_layouts: &EmbeddedLayouts,
    ) -> ContentStack<Layout> {
        let mut components_indices = HashMap::new();

        // Create shapes with text for participants
        let mut participant_shapes: HashMap<_, _> = graph
            .nodes_with_indices()
            .map(|(node_idx, node)| {
                let mut shape = draw::Shape::new(Rc::clone(
                    node.type_definition()
                        .shape_definition_rc()
                        .expect("Node must have a shape definition for sequence layout"),
                ));
                shape.set_padding(self.padding);
                let text = draw::Text::new(
                    Rc::clone(node.type_definition().text_definition_rc()),
                    node.display_text().to_string(),
                );
                let mut shape_with_text = draw::ShapeWithText::new(shape, Some(text));

                if let ast::Block::Diagram(_) = node.block() {
                    // If this participant has an embedded diagram, use its layout size
                    let content_size = if let Some(layout) = embedded_layouts.get(&node.id()) {
                        layout.calculate_size()
                    } else {
                        Size::default()
                    };

                    shape_with_text
                        .set_inner_content_size(content_size)
                        .expect("Diagram blocks should always support content sizing");
                }
                // For non-Diagram blocks, don't call set_inner_content_size
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
                self.calculate_message_label_spacing(i - 1, i, &messages_vec, &components_indices);
            spacings.push(spacing);
        }

        // Get list of node indices and their sizes
        let sizes: Vec<_> = graph
            .node_indices()
            .map(|idx| {
                let shape_with_text = participant_shapes.get(&idx).unwrap();
                shape_with_text.size()
            })
            .collect();

        // Calculate horizontal positions using positioning algorithms
        let x_positions = crate::layout::positioning::distribute_horizontally(
            &sizes,
            self.min_spacing,
            Some(&spacings),
        );

        let mut components = Vec::new();
        // Create participants and store their indices
        for (i, (node_idx, node)) in graph.nodes_with_indices().enumerate() {
            let shape_with_text = participant_shapes.remove(&node_idx).unwrap();
            let position = Point::new(x_positions[i], self.top_margin);

            let component = Component::new(node, shape_with_text, position);

            components.push(component);

            components_indices.insert(node_idx, i);
        }

        // Calculate message positions and update lifeline ends
        let mut messages = Vec::new();
        let participants_height = components
            .iter()
            .map(|component| component.drawable().size().height())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or_default();

        let mut current_y = self.top_margin + participants_height + self.message_spacing;

        for edge_idx in graph.ordered_relations() {
            let (source_idx, target_idx, relation) = graph.relation_message_info(edge_idx).unwrap();

            let source_index = *components_indices.get(&source_idx).unwrap();
            let target_index = *components_indices.get(&target_idx).unwrap();

            messages.push(Message::from_ast(
                relation,
                source_index,
                target_index,
                current_y,
            ));

            current_y += self.message_spacing;
        }

        let mut max_lifeline_end = 0.0f32;

        // Update lifeline ends to match diagram height and finalize lifelines
        let participants: Vec<_> = components
            .into_iter()
            .map(|component| {
                let lifeline_end = current_y + self.message_spacing;
                max_lifeline_end = max_lifeline_end.max(lifeline_end);

                // Rebuild the positioned lifeline with the final height
                let position = component.position();
                let lifeline_start_y = component.bounds().max_y();
                let height = (lifeline_end - lifeline_start_y).max(0.0);
                let lifeline =
                    draw::PositionedDrawable::new(draw::Lifeline::with_default_style(height))
                        .with_position(Point::new(position.x(), lifeline_start_y));

                Participant::new(component, lifeline)
            })
            .collect();

        let activations =
            self.calculate_activation_boxes(graph, &components_indices, participants_height);

        let layout = Layout::new(participants, messages, activations, max_lifeline_end);

        let mut content_stack = ContentStack::new();
        content_stack.push(PositionedContent::new(layout));

        adjust_positioned_contents_offset(&mut content_stack, graph);

        content_stack
    }

    /// Calculate activation boxes from ordered events using message-based positioning.
    ///
    /// This method processes ordered events sequentially to create activation boxes with
    /// precise timing based on the Y positions of messages contained within each activation.
    /// It uses a HashMap-based stack approach (NodeIndex -> Vec<ActivationTiming>) to track
    /// activation periods per participant and converts them to ActivationBox objects when
    /// deactivation occurs, calculating bounds from first and last message positions.
    ///
    /// # Algorithm
    /// 1. Iterate through ordered events sequentially
    /// 2. For `Event::Relation`: Add message Y position to all active activations, then advance current Y position
    /// 3. For `Event::Activate`: Create ActivationTiming with activate Y position, push to participant's stack
    /// 4. For `Event::Deactivate`: Pop activation, convert to ActivationBox using message-based bounds calculation
    ///
    /// # Parameters
    /// * `graph` - The sequence diagram graph containing ordered events
    /// * `participant_indices` - Mapping from NodeIndex to participant index
    ///
    /// # Returns
    /// Vector of `ActivationBox` objects ready for rendering with precise positioning and nesting levels
    fn calculate_activation_boxes(
        &self,
        graph: &crate::graph::Graph,
        participant_indices: &HashMap<petgraph::graph::NodeIndex, usize>,
        participants_height: f32,
    ) -> Vec<ActivationBox> {
        let mut activation_boxes: Vec<_> = Vec::new();
        let mut activation_stack: HashMap<NodeIndex, Vec<ActivationTiming>> = HashMap::new();

        // Calculate initial Y position using same calculation as messages
        let mut current_y = self.top_margin + participants_height + self.message_spacing;

        for event in graph.ordered_events() {
            match event {
                Event::Relation(..) => {
                    current_y += self.message_spacing;
                }
                Event::Activate(node_idx) => {
                    if let Some(&participant_index) = participant_indices.get(node_idx) {
                        // Calculate nesting level for this node
                        let nesting_level = activation_stack
                            .get(node_idx)
                            .map(|stack| stack.len() as u32)
                            .unwrap_or(0);

                        // Create new ActivationTiming with current Y position
                        let new_timing =
                            ActivationTiming::new(participant_index, current_y, nesting_level);

                        // Add to the stack for this node
                        activation_stack
                            .entry(*node_idx)
                            .or_insert_with(Vec::new)
                            .push(new_timing);
                    }
                }
                Event::Deactivate(node_idx) => {
                    // Pop the most recent activation for this node
                    if let Some(node_stack) = activation_stack.get_mut(node_idx) {
                        if let Some(completed_timing) = node_stack.pop() {
                            // Convert to ActivationBox using last message position as end
                            // Subtract message_spacing because current_y is at deactivate event position,
                            // but we want activation box to end at the last message position
                            let activation_box = completed_timing
                                .to_activation_box(current_y - self.message_spacing);
                            activation_boxes.push(activation_box);
                        }

                        // Clean up empty stacks to avoid memory bloat
                        if node_stack.is_empty() {
                            activation_stack.remove(node_idx);
                        }
                    }
                }
            }
        }

        activation_boxes
    }
}

impl SequenceEngine for Engine {
    fn calculate<'a>(
        &self,
        graph: &'a Graph<'a>,
        embedded_layouts: &EmbeddedLayouts,
    ) -> ContentStack<Layout> {
        self.calculate_layout(graph, embedded_layouts)
    }
}
