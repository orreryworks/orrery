//! Basic sequence layout engine
//!
//! This module provides a layout engine for sequence diagrams
//! using a simple, deterministic algorithm.

use std::{collections::HashMap, rc::Rc};

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
    pub fn calculate_layout<'a>(
        &self,
        graph: &'a SequenceGraph<'a>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> ContentStack<Layout<'a>> {
        // Create shapes with text for participants
        let mut participant_shapes: HashMap<_, _> = graph
            .nodes()
            .map(|node| {
                let mut shape = draw::Shape::new(Rc::clone(node.shape_definition()));
                shape.set_padding(self.padding);
                let text = draw::Text::new(node.shape_definition().text(), node.display_text());
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

        let (messages, activations, fragments, notes, lifeline_end) =
            self.process_events(graph, participants_height, &components);

        // Update lifeline ends to match diagram height and finalize lifelines
        let participants: HashMap<Id, Participant<'a>> = components
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

        let layout = Layout::new(
            participants,
            messages,
            activations,
            fragments,
            notes,
            lifeline_end,
        );

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
    /// 9. For `SequenceEvent::Note`: Create positioned Note at current Y, update fragment bounds if inside a fragment, then advance Y by note height
    ///
    /// # Parameters
    /// * `graph` - The sequence diagram graph containing ordered events
    /// * `participants_height` - Height of the participant boxes for calculating initial Y position
    /// * `components` - Map of participant IDs to their positioned components, used for fragment bounds tracking
    ///
    /// # Returns
    /// A tuple containing:
    /// * `Vec<Message<'a>>` - All messages with their positions and arrow information
    /// * `Vec<ActivationBox>` - All activation boxes with precise positioning and nesting levels
    /// * `Vec<draw::PositionedDrawable<draw::Fragment>>` - All fragments with their sections and bounds
    /// * `Vec<draw::PositionedDrawable<draw::Note>>` - All notes with their positions and content
    /// * `f32` - The final Y coordinate (lifeline end position)
    fn process_events<'a>(
        &self,
        graph: &SequenceGraph<'a>,
        participants_height: f32,
        components: &HashMap<Id, Component<'a>>,
    ) -> (
        Vec<Message<'a>>,
        Vec<ActivationBox>,
        Vec<draw::PositionedDrawable<draw::Fragment>>,
        Vec<draw::PositionedDrawable<draw::Note>>,
        f32,
    ) {
        let mut messages: Vec<Message<'a>> = Vec::new();
        let mut activation_boxes: Vec<ActivationBox> = Vec::new();
        let mut fragments: Vec<draw::PositionedDrawable<draw::Fragment>> = Vec::new();
        let mut notes: Vec<draw::PositionedDrawable<draw::Note>> = Vec::new();

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
                SequenceEvent::Note(note) => {
                    let positioned_note = self.create_positioned_note(note, components, current_y);
                    let note_height = positioned_note.size().height();

                    notes.push(positioned_note);
                    current_y += note_height + self.message_spacing;
                }
            }
        }

        (messages, activation_boxes, fragments, notes, current_y)
    }

    /// Create a positioned note drawable for a sequence diagram.
    ///
    /// Calculates the appropriate position and width for a note based on the participants
    /// it spans. If `note.on()` is empty, the note spans all participants in the diagram.
    ///
    /// # Arguments
    ///
    /// * `note` - The note element from the AST
    /// * `components` - Map of all participant components in the diagram
    /// * `current_y` - Current Y position in the sequence diagram
    ///
    /// # Returns
    ///
    /// A `PositionedDrawable<Note>` ready to be added to the layout
    fn create_positioned_note<'a>(
        &self,
        note: &ast::Note,
        components: &HashMap<Id, Component<'a>>,
        current_y: f32,
    ) -> draw::PositionedDrawable<draw::Note> {
        const NOTE_SPACING: f32 = 20.0; // Spacing between note and participant lifeline

        // Select appropriate components: all if on=[], otherwise specified ones
        let filtered_components: Box<dyn Iterator<Item = &Component>> = if note.on().is_empty() {
            Box::new(components.values())
        } else {
            Box::new(
                note.on()
                    .iter()
                    .map(|id| components.get(id).expect("component not found")),
            )
        };

        let edge_map: fn(&Component) -> (f32, f32) = match note.align() {
            ast::NoteAlign::Over => |component| {
                let center_x = component.position().x();
                let width = component.drawable().size().width();
                let left_edge = center_x - width / 2.0;
                let right_edge = center_x + width / 2.0;
                (left_edge, right_edge)
            },
            ast::NoteAlign::Left | ast::NoteAlign::Right => {
                |component| (component.position().x(), component.position().x())
            }
            ast::NoteAlign::Top | ast::NoteAlign::Bottom => {
                unreachable!("Alignments is not supported for sequence diagrams")
            }
        };

        let (min_x, max_x) = filtered_components
            .map(edge_map)
            .reduce(|(min_x, max_x), (left_x, right_x)| (min_x.min(left_x), max_x.max(right_x)))
            .expect("note should have at least one participant");

        let mut new_note_def = Rc::clone(note.definition());
        let note_def_mut = Rc::make_mut(&mut new_note_def);
        if note.align() == ast::NoteAlign::Over {
            note_def_mut.set_min_width(Some(max_x - min_x));
        }

        let note_drawable = draw::Note::new(new_note_def, note.content().to_string());

        let note_size = note_drawable.size();
        let center_x = match note.align() {
            ast::NoteAlign::Over => (min_x + max_x) / 2.0,
            ast::NoteAlign::Left => min_x - (note_size.width() / 2.0) - NOTE_SPACING,
            ast::NoteAlign::Right => max_x + (note_size.width() / 2.0) + NOTE_SPACING,
            ast::NoteAlign::Top | ast::NoteAlign::Bottom => {
                unreachable!("Alignments is not supported for sequence diagrams")
            }
        };
        let position = Point::new(center_x, current_y + note_size.height() / 2.0);

        draw::PositionedDrawable::new(note_drawable).with_position(position)
    }
}

impl SequenceEngine for Engine {
    fn calculate<'a>(
        &self,
        graph: &'a SequenceGraph<'a>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> ContentStack<Layout<'a>> {
        self.calculate_layout(graph, embedded_layouts)
    }
}
