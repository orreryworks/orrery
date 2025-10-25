//! Basic sequence layout engine
//!
//! This module provides a layout engine for sequence diagrams
//! using a simple, deterministic algorithm.

use crate::{
    ast,
    draw::{self, Drawable as _},
    geometry::{Insets, Point, Size},
    identifier::Id,
    layout::{
        component::Component,
        engines::{EmbeddedLayouts, SequenceEngine},
        layer::{ContentStack, PositionedContent},
        sequence::{ActivationBox, ActivationTiming, FragmentTiming, Layout, Message, Participant},
    },
    structure::{SequenceEvent, SequenceGraph},
};
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
        source_id: Id,
        target_id: Id,
        messages: &[&ast::Relation],
    ) -> f32 {
        // Filter messages to only those between the two participants
        let relevant_messages = messages.iter().filter(|relation| {
            (relation.source() == source_id && relation.target() == target_id)
                || (relation.source() == target_id && relation.target() == source_id)
        });

        // Extract labels from relations and use shared function to calculate spacing
        let labels = relevant_messages.map(|relation| relation.text());
        crate::layout::positioning::calculate_label_spacing(labels, self.label_padding)
    }

    /// Calculate layout for a sequence diagram
    pub fn calculate_layout(
        &self,
        graph: &SequenceGraph,
        embedded_layouts: &EmbeddedLayouts,
    ) -> ContentStack<Layout> {
        // Create shapes with text for participants
        let mut participant_shapes: HashMap<_, _> = graph
            .nodes()
            .map(|node| {
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
                (node.id(), shape_with_text)
            })
            .collect();

        // Collect all messages to consider their labels for spacing
        let mut messages_vec = Vec::new();
        for relation in graph.relations() {
            messages_vec.push(relation);
        }

        // Calculate additional spacings based on message labels
        let mut spacings = Vec::<f32>::new();
        let mut nodes_iter = graph.nodes();
        if let Some(mut last_node) = nodes_iter.next() {
            for node in nodes_iter {
                let spacing =
                    self.calculate_message_label_spacing(last_node.id(), node.id(), &messages_vec);
                spacings.push(spacing);
                last_node = node;
            }
        }

        // Get list of node indices and their sizes
        let sizes: Vec<_> = graph
            .node_ids()
            .map(|id| {
                let shape_with_text = participant_shapes.get(id).unwrap();
                shape_with_text.size()
            })
            .collect();

        // Calculate horizontal positions using positioning algorithms
        let x_positions = crate::layout::positioning::distribute_horizontally(
            &sizes,
            self.min_spacing,
            Some(&spacings),
        );

        // Create participants and store their indices
        let components: HashMap<Id, Component> = graph
            .nodes()
            .enumerate()
            .map(|(i, node)| {
                let shape_with_text = participant_shapes.remove(&node.id()).unwrap();
                let position = Point::new(x_positions[i], self.top_margin);

                let component = Component::new(node, shape_with_text, position);

                (node.id(), component)
            })
            .collect();

        // Calculate message positions and update lifeline ends
        let participants_height = components
            .values()
            .map(|component| component.drawable().size().height())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or_default();

        let (messages, activations, fragments, lifeline_end) =
            self.process_events(graph, participants_height, &components);

        // Update lifeline ends to match diagram height and finalize lifelines
        let participants: HashMap<Id, Participant> = components
            .into_iter()
            .map(|(id, component)| {
                // Rebuild the positioned lifeline with the final height
                let position = component.position();
                let lifeline_start_y = component.bounds().max_y();
                let height = (lifeline_end - lifeline_start_y).max(0.0);
                let lifeline =
                    draw::PositionedDrawable::new(draw::Lifeline::with_default_style(height))
                        .with_position(Point::new(position.x(), lifeline_start_y));

                (id, Participant::new(component, lifeline))
            })
            .collect();

        let layout = Layout::new(participants, messages, activations, fragments, lifeline_end);

        let mut content_stack = ContentStack::new();
        content_stack.push(PositionedContent::new(layout));

        content_stack
    }

    /// Process all sequence diagram events to create layout components.
    ///
    /// This method processes ordered events sequentially to create messages, activation boxes,
    /// and fragments with precise timing and positioning. It uses a HashMap-based stack approach
    /// (Id -> [`Vec<ActivationTiming>`]) to track activation periods per participant and converts
    /// them to ActivationBox objects when deactivation occurs.
    ///
    /// # Algorithm
    /// 1. Iterate through ordered events sequentially
    /// 2. For `SequenceEvent::Relation`: Create Message at current Y position, update fragment bounds if inside a fragment, then advance Y
    /// 3. For `SequenceEvent::Activate`: Create ActivationTiming with current Y position, push to participant's stack
    /// 4. For `SequenceEvent::Deactivate`: Pop activation, convert to ActivationBox with precise bounds
    /// 5. For `SequenceEvent::FragmentStart`: Create FragmentTiming and push to fragment stack
    /// 6. For `SequenceEvent::FragmentSectionStart`: Start new section in current fragment
    /// 7. For `SequenceEvent::FragmentSectionEnd`: End current section in current fragment
    /// 8. For `SequenceEvent::FragmentEnd`: Pop fragment, convert to Fragment with final bounds
    ///
    /// # Parameters
    /// * `graph` - The sequence diagram graph containing ordered events
    /// * `participants_height` - Height of the participant boxes for calculating initial Y position
    /// * `components` - Map of participant IDs to their positioned components, used for fragment bounds tracking
    ///
    /// # Returns
    /// A tuple containing:
    /// * `Vec<Message>` - All messages with their positions and arrow information
    /// * `Vec<ActivationBox>` - All activation boxes with precise positioning and nesting levels
    /// * `Vec<draw::PositionedDrawable<draw::Fragment>>` - All fragments with their sections and bounds
    /// * `f32` - The final Y coordinate (lifeline end position)
    fn process_events(
        &self,
        graph: &SequenceGraph,
        participants_height: f32,
        components: &HashMap<Id, Component>,
    ) -> (
        Vec<Message>,
        Vec<ActivationBox>,
        Vec<draw::PositionedDrawable<draw::Fragment>>,
        f32,
    ) {
        let mut messages: Vec<Message> = Vec::new();
        let mut activation_boxes: Vec<ActivationBox> = Vec::new();
        let mut fragments: Vec<draw::PositionedDrawable<draw::Fragment>> = Vec::new();

        let mut activation_stack: HashMap<Id, Vec<ActivationTiming>> = HashMap::new();
        let mut fragment_stack: Vec<FragmentTiming> = Vec::new();

        // Calculate initial Y position using same calculation as messages
        let mut current_y = self.top_margin + participants_height + self.message_spacing;

        for event in graph.events() {
            match event {
                SequenceEvent::Relation(relation) => {
                    let message = Message::from_ast(
                        relation,
                        relation.source(),
                        relation.target(),
                        current_y,
                    );

                    messages.push(message);

                    // Update fragment bounds if we're inside a fragment
                    // NOTE: For perfectly accurate bounds, this should use calculate_message_endpoint_x()
                    // to account for activation box offsets. Currently using participant center positions
                    // as a simpler approximation that is adequate for most cases.
                    if let Some(fragment_timing) = fragment_stack.last_mut() {
                        let source_x = components[&relation.source()].position().x();
                        let target_x = components[&relation.target()].position().x();
                        fragment_timing.update_x(source_x, target_x);
                    }

                    current_y += self.message_spacing;
                }
                SequenceEvent::Activate(node_id) => {
                    // Calculate nesting level for this node
                    let nesting_level = activation_stack
                        .get(node_id)
                        .map(|stack| stack.len() as u32)
                        .unwrap_or(0);

                    // Create new ActivationTiming with current Y position
                    let new_timing = ActivationTiming::new(*node_id, current_y, nesting_level);

                    // Add to the stack for this node
                    activation_stack
                        .entry(*node_id)
                        .or_default()
                        .push(new_timing);
                }
                SequenceEvent::Deactivate(node_id) => {
                    // Pop the most recent activation for this node
                    if let Some(node_stack) = activation_stack.get_mut(node_id) {
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
                            activation_stack.remove(node_id);
                        }
                    }
                }
                SequenceEvent::FragmentStart(fragment) => {
                    let fragment_timing = FragmentTiming::new(fragment, current_y);
                    fragment_stack.push(fragment_timing);
                }
                SequenceEvent::FragmentSectionStart(fragment_section) => {
                    let fragment_timing = fragment_stack
                        .last_mut()
                        .expect("fragment_timing stack is empty");
                    fragment_timing.start_section(fragment_section, current_y);
                }
                SequenceEvent::FragmentSectionEnd => {
                    let fragment_timing = fragment_stack
                        .last_mut()
                        .expect("fragment_timing stack is empty");
                    fragment_timing.end_section(current_y).unwrap();
                }
                SequenceEvent::FragmentEnd => {
                    let fragment_timing = fragment_stack
                        .pop()
                        .expect("fragment_timing stack is empty");
                    let fragment = fragment_timing.into_fragment(current_y);
                    fragments.push(fragment);
                }
                SequenceEvent::Note(_note) => {
                    // TODO: Implement note layout
                    // Notes don't affect the vertical position or timing for now
                }
            }
        }

        (messages, activation_boxes, fragments, current_y)
    }
}

impl SequenceEngine for Engine {
    fn calculate<'a>(
        &self,
        graph: &'a SequenceGraph<'a>,
        embedded_layouts: &EmbeddedLayouts,
    ) -> ContentStack<Layout> {
        self.calculate_layout(graph, embedded_layouts)
    }
}
