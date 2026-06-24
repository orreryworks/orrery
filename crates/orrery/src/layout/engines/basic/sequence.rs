//! Basic sequence layout engine.
//!
//! Lays out sequence diagrams with a simple, deterministic algorithm.

use std::{cell::RefCell, cmp::Ordering, collections::HashMap, f32, rc::Rc};

use orrery_core::{
    draw::{
        Arrow, ArrowPath, ArrowStyle, ArrowWithText, Drawable, Fragment, Lifeline,
        Note as DrawNote, PositionedArrowWithText, PositionedDrawable, Shape, ShapeWithText, Text,
    },
    geometry::{Insets, Point, Size},
    identifier::Id,
    semantic::{Block, Note, NoteAlign, Relation},
};

use crate::{
    error::RenderError,
    layout::{
        component::Component,
        engines::{EmbeddedLayouts, SequenceEngine},
        layer::{ContentStack, PositionedContent},
        sequence::{ActivationBox, ActivationTiming, FragmentTiming, Layout, Participant},
    },
    structure::{SequenceEvent, SequenceGraph},
};

/// Horizontal gap between the right edge of a self-loop's hook and the left
/// edge of its label.
const SELF_LOOP_LABEL_GAP: f32 = 4.0;

/// A message being positioned during sequence-event processing.
///
/// Stores the participant components and the active activation timings captured
/// at the message event, so later path construction attaches to the correct box
/// edges even before activation boxes are finalized.
struct Message<'a, 'b> {
    source: &'b Component<'a>,
    target: &'b Component<'a>,
    source_activation: Option<Rc<RefCell<ActivationTiming>>>,
    target_activation: Option<Rc<RefCell<ActivationTiming>>>,
    y_position: f32,
    arrow_with_text: ArrowWithText<'a>,
}

impl<'a, 'b> Message<'a, 'b> {
    /// Creates a message from a semantic relation and its endpoint components.
    ///
    /// Self-loops clone the relation's arrow definition and force
    /// [`ArrowStyle::Curved`] so they render as loops even when the source style
    /// was straight.
    fn from_relation(
        relation: &'a Relation,
        source: &'b Component<'a>,
        target: &'b Component<'a>,
        source_activation: Option<Rc<RefCell<ActivationTiming>>>,
        target_activation: Option<Rc<RefCell<ActivationTiming>>>,
    ) -> Self {
        let mut arrow_def = Rc::clone(relation.arrow_definition());
        if source.node_id() == target.node_id() && *arrow_def.style() != ArrowStyle::Curved {
            Rc::make_mut(&mut arrow_def).set_style(ArrowStyle::Curved);
        }
        let arrow = Arrow::new(arrow_def, relation.arrow_direction());
        let arrow_with_text = ArrowWithText::new(arrow, relation.text());
        Self {
            source,
            target,
            source_activation,
            target_activation,
            y_position: 0.0,
            arrow_with_text,
        }
    }

    /// Returns the minimum size needed to render this message's arrow and text.
    fn min_size(&self) -> Size {
        self.arrow_with_text.min_size()
    }

    /// Sets the center Y coordinate for this message.
    fn set_y_position(&mut self, y_position: f32) {
        self.y_position = y_position;
    }

    /// Returns the center Y coordinate of this message.
    fn y_position(&self) -> f32 {
        self.y_position
    }

    /// Returns `true` if this message renders as a self-loop on a single participant.
    fn is_self_loop(&self) -> bool {
        self.source.node_id() == self.target.node_id()
    }

    /// Consumes self and returns the inner [`ArrowWithText`].
    fn into_arrow_with_text(self) -> ArrowWithText<'a> {
        self.arrow_with_text
    }

    /// Returns the source X coordinate, shifted to an active activation edge when present.
    fn calculate_message_source_endpoint_x(&self) -> f32 {
        self.calculate_message_endpoint_x(
            &self.source_activation,
            self.source,
            self.target.position().x(),
        )
    }

    /// Returns the target X coordinate, shifted to an active activation edge when present.
    fn calculate_message_target_endpoint_x(&self) -> f32 {
        self.calculate_message_endpoint_x(
            &self.target_activation,
            self.target,
            self.source.position().x(),
        )
    }

    /// Uses an activation edge for this endpoint when the message was emitted inside one.
    fn calculate_message_endpoint_x(
        &self,
        activation_timing: &Option<Rc<RefCell<ActivationTiming>>>,
        participant: &'b Component<'a>,
        mut target_x: f32,
    ) -> f32 {
        if self.is_self_loop() {
            target_x = f32::INFINITY;
        }

        if let Some(activation_timing) = activation_timing {
            activation_timing
                .borrow()
                .to_activation_box(0.0)
                .intersection_x(participant.position(), target_x)
        } else {
            participant.position().x()
        }
    }
}

/// Collected output from [`Engine::process_events`]: positioned arrows,
/// activation boxes, fragments, notes, and the final lifeline Y coordinate.
type ProcessEventsResult<'a> = (
    Vec<PositionedArrowWithText<'a>>,
    Vec<ActivationBox>,
    Vec<PositionedDrawable<Fragment>>,
    Vec<PositionedDrawable<DrawNote>>,
    f32,
);

/// Basic deterministic layout engine for sequence diagrams.
///
/// Distributes participants horizontally, processes events sequentially
/// to place messages and activations, and builds lifelines from
/// participant boxes to the final event position.
pub struct Engine {
    /// Minimum horizontal space between participants.
    min_spacing: f32,
    /// Vertical padding between consecutive events.
    event_padding: f32,
    /// Vertical margin above participant boxes.
    top_margin: f32,
    /// Padding inside participant shapes.
    padding: Insets,
    /// Extra horizontal padding to accommodate message labels.
    label_padding: f32,
    /// Minimum bounding [`Size`] reserved for a self-loop.
    ///
    /// The width is how far the loop bows out beyond the participant's lifeline; the
    /// height is the minimum vertical span between the loop's source and destination.
    self_loop_min_size: Size,
    /// Radius of the rounded corners of a self-loop.
    ///
    /// [`Self::self_loop_min_size`]'s width must be at least this value, and
    /// its height must be at least twice this value.
    self_loop_corner_radius: f32,
}

impl Engine {
    /// Create a new basic sequence layout engine
    pub fn new() -> Self {
        Self {
            min_spacing: 40.0, // Minimum spacing between participants
            event_padding: 15.0,
            top_margin: 60.0,
            padding: Insets::uniform(15.0),
            label_padding: 20.0, // Extra padding for labels
            self_loop_min_size: Size::new(30.0, 20.0),
            self_loop_corner_radius: 8.0,
        }
    }

    /// Set the minimum spacing between participants
    pub fn set_min_spacing(&mut self, spacing: f32) -> &mut Self {
        self.min_spacing = spacing;
        self
    }

    /// Sets the vertical padding between sequence events.
    pub fn set_event_padding(&mut self, padding: f32) -> &mut Self {
        self.event_padding = padding;
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

    /// Sets the minimum bounding [`Size`] for a self-loop.
    ///
    /// The width is how far the loop bows out beyond the lifeline; the height
    /// is the minimum vertical span between the loop's source and destination.
    ///
    /// # Panics
    ///
    /// Panics if `size.width()` is less than the configured corner radius or
    /// `size.height()` is less than twice it.
    #[allow(dead_code)]
    pub fn set_self_loop_min_size(&mut self, size: Size) -> &mut Self {
        self.self_loop_min_size = size;
        self.assert_self_loop_min_size_radius_is_valid();
        self
    }

    /// Sets the rounded-corner radius for self-loops.
    ///
    /// # Panics
    ///
    /// Panics if `radius` exceeds the configured
    /// [`Self::self_loop_min_size`]'s width, or if `2 * radius` exceeds its
    /// height.
    #[allow(dead_code)]
    pub fn set_self_loop_corner_radius(&mut self, radius: f32) -> &mut Self {
        self.self_loop_corner_radius = radius;
        self.assert_self_loop_min_size_radius_is_valid();
        self
    }

    /// Asserts that the configured self-loop minimum size is large enough to
    /// fit the configured rounded-corner radius.
    ///
    /// The loop is a rounded rectangle with arcs only on the right side
    /// (the lifeline closes the left). Horizontally, at most one arc spans
    /// the loop in any given row, so the width must be at least the corner
    /// radius. Vertically, the top-right and bottom-right arcs share the
    /// vertical extent, so the height must be at least twice the corner
    /// radius.
    ///
    /// # Panics
    ///
    /// Panics if [`Self::self_loop_min_size`]'s width is less than
    /// [`Self::self_loop_corner_radius`], or if its height is less than
    /// twice it.
    fn assert_self_loop_min_size_radius_is_valid(&self) {
        let is_valid = self.self_loop_min_size.width() >= self.self_loop_corner_radius
            && self.self_loop_min_size.height() >= 2.0 * self.self_loop_corner_radius;
        assert!(
            is_valid,
            "`self_loop_min_size` ({}x{}) is too small to fit \
             `self_loop_corner_radius` ({}); width must be ≥ the corner \
             radius and height must be ≥ twice the corner radius",
            self.self_loop_min_size.width(),
            self.self_loop_min_size.height(),
            self.self_loop_corner_radius
        );
    }

    /// Calculate layout for a sequence diagram.
    ///
    /// # Arguments
    ///
    /// * `graph` - The sequence diagram graph to lay out.
    /// * `embedded_layouts` - Pre-calculated layouts for embedded diagrams.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Layout`] if position or shape calculation fails.
    pub fn calculate_layout<'a>(
        &self,
        graph: &'a SequenceGraph<'a>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> Result<ContentStack<Layout<'a>>, RenderError> {
        // Create shapes with text for participants
        let mut participant_shapes: HashMap<_, _> = HashMap::new();
        for node in graph.nodes() {
            let mut shape = Shape::new(Rc::clone(node.shape_definition()));
            shape.set_padding(self.padding);
            let text = Text::new(node.shape_definition().text(), node.display_text());
            let mut shape_with_text = ShapeWithText::new(shape, Some(text));

            if let Block::Diagram(_) = node.block() {
                // If this participant has an embedded diagram, use its layout size
                let content_size = if let Some(layout) = embedded_layouts.get(&node.id()) {
                    layout.calculate_size()
                } else {
                    Size::zero()
                };

                shape_with_text
                    .set_inner_content_size(content_size)
                    .map_err(|err| {
                        RenderError::Layout(format!(
                            "Failed to set content size for participant '{}': {err}",
                            node.display_text()
                        ))
                    })?;
            }
            // For non-Diagram blocks, don't call set_inner_content_size
            participant_shapes.insert(node.id(), shape_with_text);
        }

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
                let spacing = self.calculate_inter_participant_spacing(
                    last_node.id(),
                    node.id(),
                    &messages_vec,
                );
                spacings.push(spacing);
                last_node = node;
            }
        }

        // Get list of node indices and their sizes
        let mut sizes: Vec<Size> = Vec::new();
        for id in graph.node_ids() {
            let shape_with_text = participant_shapes.get(id).ok_or_else(|| {
                RenderError::Layout("Participant shape not found for node".to_string())
            })?;
            sizes.push(shape_with_text.size());
        }

        // Calculate horizontal positions using positioning algorithms
        let x_positions = crate::layout::positioning::distribute_horizontally(
            &sizes,
            self.min_spacing,
            Some(&spacings),
        );

        // Create participants and store their indices
        let mut components: HashMap<Id, Component> = HashMap::new();
        for (i, node) in graph.nodes().enumerate() {
            let shape_with_text = participant_shapes.remove(&node.id()).ok_or_else(|| {
                RenderError::Layout(format!("Participant shape not found for node '{node}'"))
            })?;
            let position = Point::new(x_positions[i], self.top_margin);

            let component = Component::new(node, shape_with_text, position);
            components.insert(node.id(), component);
        }

        // Calculate message positions and update lifeline ends
        let participants_height = components
            .values()
            .map(|component| component.drawable().size().height())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            .unwrap_or_default();

        let (arrows, activations, fragments, notes, lifeline_end) =
            self.process_events(graph, participants_height, &components)?;

        // Update lifeline ends to match diagram height and finalize lifelines
        let participants: HashMap<Id, Participant<'a>> = components
            .into_iter()
            .map(|(id, component)| {
                // Rebuild the positioned lifeline with the final height
                let position = component.position();
                let lifeline_start_y = component.bounds().max_y();
                let height = (lifeline_end - lifeline_start_y).max(0.0);
                let lifeline = PositionedDrawable::new(Lifeline::new(
                    Rc::clone(graph.lifeline_definition()),
                    height,
                ))
                .with_position(Point::new(position.x(), lifeline_start_y));

                (id, Participant::new(component, lifeline))
            })
            .collect();

        let layout = Layout::new(
            participants,
            arrows,
            activations,
            fragments,
            notes,
            lifeline_end,
        );

        let mut content_stack = ContentStack::new();
        content_stack.push(PositionedContent::new(layout));

        Ok(content_stack)
    }

    /// Calculates additional spacing needed between two consecutive
    /// participants (`source_id` on the left, `target_id` on the right) so
    /// that message labels and arrows fit between them.
    ///
    /// # Arguments
    ///
    /// * `source_id` - ID of the left-hand participant.
    /// * `target_id` - ID of the right-hand participant.
    /// * `messages` - All relations in the diagram. Only the ones that
    ///   appear between source and target contribute to the result.
    ///
    /// # Returns
    ///
    /// The required horizontal spacing between source and target.
    fn calculate_inter_participant_spacing(
        &self,
        source_id: Id,
        target_id: Id,
        messages: &[&Relation],
    ) -> f32 {
        let relevant_messages = messages
            .iter()
            .filter(|relation| {
                (relation.source() == source_id
                    && (relation.target() == target_id || relation.is_self_loop()))
                    || (relation.source() == target_id && relation.target() == source_id)
            })
            .copied();
        let has_self_loop = relevant_messages.clone().any(Relation::is_self_loop);
        let mut max_size = relevant_messages
            .flat_map(|relation| relation.text().map(|t| t.calculate_size()))
            .fold(Size::zero(), Size::max);
        if has_self_loop {
            max_size = max_size.max(self.self_loop_min_size)
        }

        if max_size.width() == 0.0 {
            0.0
        } else {
            max_size.width() + self.label_padding
        }
    }

    /// Converts intermediate messages into positioned arrows.
    ///
    /// Endpoint X coordinates come from each [`Message`]'s activation timing
    /// snapshots, so paths attach to activation-box edges captured at the
    /// message's event.
    fn position_messages<'a, 'b>(
        &self,
        messages: Vec<Message<'a, 'b>>,
    ) -> Vec<PositionedArrowWithText<'a>> {
        messages
            .into_iter()
            .map(|msg| self.position_message(msg))
            .collect()
    }

    /// Positions one message using either the self-loop or cross-participant path builder.
    fn position_message<'a, 'b>(&self, msg: Message<'a, 'b>) -> PositionedArrowWithText<'a> {
        if msg.is_self_loop() {
            let (path, label_position) = self.self_loop_path(&msg);
            PositionedArrowWithText::new(msg.into_arrow_with_text(), path)
                .with_text_position(label_position)
        } else {
            let path = Self::cross_participant_path(&msg);
            PositionedArrowWithText::new(msg.into_arrow_with_text(), path)
        }
    }

    /// Computes a straight-line [`ArrowPath`] between two distinct participants.
    ///
    /// Endpoint X coordinates attach to the active activation-box edge when the
    /// message snapshot includes an activation timing; otherwise they use the
    /// participant center.
    fn cross_participant_path(msg: &Message<'_, '_>) -> ArrowPath {
        let source_x = msg.calculate_message_source_endpoint_x();
        let target_x = msg.calculate_message_target_endpoint_x();

        let start_point = Point::new(source_x, msg.y_position());
        let end_point = Point::new(target_x, msg.y_position());
        ArrowPath::straight(start_point, end_point)
    }

    /// Computes a rounded-rectangular [`ArrowPath`] for a self-loop on a single participant.
    ///
    /// ```text
    ///       lifeline
    ///         |
    ///  source o-----┐
    ///         |     |  label text spans
    ///         |     |  one or more lines
    ///    dest o<----┘
    ///         |
    /// ```
    ///
    /// The path is a five-segment chain (line, arc, line, arc, line) encoded
    /// as a chain of cubic Béziers in the [`ArrowPath`]'s control points. The
    /// two arcs approximate the quarter-circles at the top-right and
    /// bottom-right corners.
    ///
    /// Forward arrows terminate at the returning bottom point, so the arrowhead
    /// points back into the lifeline along the curve tangent.
    ///
    /// # Returns
    ///
    /// `(path, label_position)` — the loop's arrow path and an explicit label
    /// position, or `None` when the message has no label.
    fn self_loop_path(&self, msg: &Message<'_, '_>) -> (ArrowPath, Option<Point>) {
        debug_assert!(msg.is_self_loop(), "expected self-loop message");

        // Geometry:
        // - `x_anchor` = right edge of the most-nested active activation box at
        //   `y_position`, or the participant's center X if no activation box is
        //   active.
        // - The loop's **horizontal** extent (`arm_length`) is fixed by
        //   `self.self_loop_min_size.width()`.
        // - The loop's **vertical** extent (`separation`) grows to fit
        //   multi-line labels.
        // - The label is positioned to the right of the loop's hook, offset by
        //   [`SELF_LOOP_LABEL_GAP`].

        let x_anchor = msg.calculate_message_source_endpoint_x();

        let content_size = msg.min_size();
        let arrow_size = self.self_loop_with_content_min_arrow_size(content_size);

        let y_center = msg.y_position();
        let y_top = y_center - arrow_size.height() / 2.0;
        let y_bottom = y_top + arrow_size.height();
        let corner_x = x_anchor + arrow_size.width();

        let r = self.self_loop_corner_radius;

        // Anchor points along the loop's perimeter.
        let a0 = Point::new(x_anchor, y_top); // source
        let a1 = Point::new(corner_x - r, y_top); // before top-right corner
        let a2 = Point::new(corner_x, y_top + r); // after top-right corner
        let a3 = Point::new(corner_x, y_bottom - r); // before bottom-right corner
        let a4 = Point::new(corner_x - r, y_bottom); // after bottom-right corner
        let a5 = Point::new(x_anchor, y_bottom); // destination

        // Cubic-Bézier circular-arc constant: control points at distance
        // `kappa * r` from the anchors along the tangent direction.
        // (kappa = 4 * (sqrt(2) - 1) / 3, truncated to f32 precision.)
        const KAPPA: f32 = 0.552_284_8;
        let off = r * (1.0 - KAPPA);

        // Quarter-arc control points: top-right (right → down) and bottom-right
        // (down → left).
        let arc_top_cp1 = Point::new(corner_x - off, y_top);
        let arc_top_cp2 = Point::new(corner_x, y_top + off);
        let arc_bot_cp1 = Point::new(corner_x, y_bottom - off);
        let arc_bot_cp2 = Point::new(corner_x - off, y_bottom);

        let (s1_cp1, s1_cp2) = line_segment_cubic_cps(a0, a1);
        let (s3_cp1, s3_cp2) = line_segment_cubic_cps(a2, a3);
        let (s5_cp1, s5_cp2) = line_segment_cubic_cps(a4, a5);

        let control_points = vec![
            s1_cp1,
            s1_cp2,
            a1,
            arc_top_cp1,
            arc_top_cp2,
            a2,
            s3_cp1,
            s3_cp2,
            a3,
            arc_bot_cp1,
            arc_bot_cp2,
            a4,
            s5_cp1,
            s5_cp2,
        ];

        let path = ArrowPath::new(a0, a5, control_points);

        let label_position = if content_size.is_zero() {
            None
        } else {
            let x_center = x_anchor
                + self.self_loop_min_size.width()
                + SELF_LOOP_LABEL_GAP
                + content_size.width() / 2.0;
            Some(Point::new(x_center, y_center))
        };

        (path, label_position)
    }

    /// Walks the ordered events and produces positioned arrows, activation
    /// boxes, fragments, and notes, advancing `current_y` for each event.
    ///
    /// Per event:
    /// - Relation: builds a [`Message`] centered in its slot, snapshots the
    ///   currently active source/target activation timings, and updates the
    ///   enclosing fragment's X bounds.
    /// - Activate: pushes an [`ActivationTiming`] onto the participant's stack
    ///   at the last relation's Y, with nesting equal to the existing depth.
    /// - Deactivate: pops the top timing for the participant and finalizes it
    ///   into an [`ActivationBox`] ending at the last relation's Y.
    /// - FragmentStart / FragmentSectionStart / FragmentSectionEnd /
    ///   FragmentEnd: maintain a [`FragmentTiming`] stack; the closing event
    ///   emits a positioned [`Fragment`].
    /// - Note: emits a positioned [`DrawNote`] at `current_y`.
    ///
    /// After all events are processed, queued messages are converted into
    /// [`PositionedArrowWithText`] using the activation snapshots taken at
    /// emission time.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Layout`] when a note references a participant
    /// that is absent from `components`.
    ///
    /// # Panics
    ///
    /// Panics if a relation or activate event references a participant absent
    /// from `components`, or if fragment section/end events are unbalanced.
    fn process_events<'a>(
        &self,
        graph: &SequenceGraph<'a>,
        participants_height: f32,
        components: &HashMap<Id, Component<'a>>,
    ) -> Result<ProcessEventsResult<'a>, RenderError> {
        let mut messages: Vec<Message<'a, '_>> = Vec::new();
        let mut activation_boxes: Vec<ActivationBox> = Vec::new();
        let mut fragments: Vec<PositionedDrawable<Fragment>> = Vec::new();
        let mut notes: Vec<PositionedDrawable<DrawNote>> = Vec::new();

        let mut activation_stack: HashMap<Id, Vec<Rc<RefCell<ActivationTiming>>>> = HashMap::new();
        let mut fragment_stack: Vec<FragmentTiming> = Vec::new();

        // Initial Y is the top edge of the first event area.
        let mut current_y = self.top_margin + participants_height + self.event_padding;
        // Track the Y position of the last placed relation (before spacing advance).
        let mut last_relation_y = current_y;

        for event in graph.events() {
            match event {
                SequenceEvent::Relation(relation) => {
                    let source = &components[&relation.source()];
                    let target = &components[&relation.target()];
                    let source_activation = activation_stack
                        .get(&source.node_id())
                        .and_then(|activation_timings| activation_timings.last())
                        .cloned();
                    let target_activation = activation_stack
                        .get(&target.node_id())
                        .and_then(|activation_timings| activation_timings.last())
                        .cloned();

                    let mut ir_message = Message::from_relation(
                        relation,
                        source,
                        target,
                        source_activation,
                        target_activation,
                    );

                    let message_height = self.message_min_size(&ir_message).height();

                    // Center the arrow line within the message's vertical extent.
                    let center_y = current_y + message_height / 2.0;
                    ir_message.set_y_position(center_y);

                    messages.push(ir_message);

                    // Update fragment bounds if we're inside a fragment
                    // NOTE: For perfectly accurate bounds, this should use calculate_message_endpoint_x()
                    // to account for activation box offsets. Currently using participant center positions
                    // as a simpler approximation that is adequate for most cases.
                    if let Some(fragment_timing) = fragment_stack.last_mut() {
                        fragment_timing.update_x(source.position().x(), target.position().x());
                    }

                    last_relation_y = center_y;
                    current_y += message_height + self.event_padding;
                }
                SequenceEvent::Activate(activate) => {
                    let node_id = activate.component();
                    let participant_x = components[&node_id].position().x();
                    // Calculate nesting level for this node
                    let nesting_level = activation_stack
                        .get(&node_id)
                        .map(|stack| stack.len() as u32)
                        .unwrap_or(0);

                    let new_timing = ActivationTiming::new(
                        participant_x,
                        last_relation_y,
                        nesting_level,
                        Rc::clone(activate.definition()),
                    );

                    // Add to the stack for this node
                    activation_stack
                        .entry(node_id)
                        .or_default()
                        .push(Rc::new(RefCell::new(new_timing)));
                }
                SequenceEvent::Deactivate(node_id) => {
                    // Pop the most recent activation for this node
                    if let Some(node_stack) = activation_stack.get_mut(node_id) {
                        if let Some(completed_timing) = node_stack.pop() {
                            let activation_box =
                                completed_timing.borrow().to_activation_box(last_relation_y);
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
                    // No current_y advance here; handled in FragmentSectionStart
                }
                SequenceEvent::FragmentSectionStart(fragment_section) => {
                    let fragment_timing = fragment_stack
                        .last_mut()
                        .expect("fragment_timing stack is empty");
                    fragment_timing.start_section(fragment_section, current_y);
                    current_y += fragment_timing.section_header_height(fragment_section)
                        + self.event_padding;
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
                    let fragment_bottom_padding = fragment_timing.bottom_padding();
                    let fragment = fragment_timing.into_fragment(current_y);
                    fragments.push(fragment);
                    current_y += fragment_bottom_padding + self.event_padding;
                }
                SequenceEvent::Note(note) => {
                    let positioned_note =
                        self.create_positioned_note(note, components, current_y)?;
                    let note_height = positioned_note.size().height();

                    notes.push(positioned_note);
                    current_y += note_height + self.event_padding;
                }
            }
        }
        let arrows = self.position_messages(messages);

        Ok((arrows, activation_boxes, fragments, notes, current_y))
    }

    /// Computes the minimum bounding [`Size`] for a message slot.
    ///
    /// - For cross-participant messages this is just the message's intrinsic
    ///   content size.
    /// - For self-loops, the slot reserves enough horizontal space for
    ///   the loop and any overflowing label sitting next to it.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to size.
    ///
    /// # Returns
    ///
    /// The minimum [`Size`] this message will occupy in the layout.
    fn message_min_size(&self, message: &Message) -> Size {
        let content_size = message.min_size();
        if message.is_self_loop() {
            if content_size.is_zero() {
                self.self_loop_min_size
            } else {
                let arrow_size = self.self_loop_with_content_min_arrow_size(content_size);
                let width =
                    self.self_loop_min_size.width() + SELF_LOOP_LABEL_GAP + content_size.width();
                Size::new(width, arrow_size.height())
            }
        } else {
            content_size
        }
    }

    /// Computes the minimum bounding [`Size`] for the rounded-rectangle arrow
    /// of a self-loop given the message's `content_size`.
    ///
    /// # Arguments
    ///
    /// * `content_size` - The intrinsic size of the message's content.
    ///
    /// # Returns
    ///
    /// The minimum bounding [`Size`] of the loop's arrow path.
    fn self_loop_with_content_min_arrow_size(&self, content_size: Size) -> Size {
        let height = (content_size.height() + 2.0 * self.self_loop_corner_radius)
            .max(self.self_loop_min_size.height());
        Size::new(self.self_loop_min_size.width(), height)
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
        note: &Note,
        components: &HashMap<Id, Component<'a>>,
        current_y: f32,
    ) -> Result<PositionedDrawable<DrawNote>, RenderError> {
        const NOTE_SPACING: f32 = 20.0; // Spacing between note and participant lifeline

        // Select appropriate components: all if on=[], otherwise specified ones
        let filtered_components: Vec<&Component> = if note.on().is_empty() {
            components.values().collect()
        } else {
            note.on()
                .iter()
                .map(|id| {
                    components.get(id).ok_or_else(|| {
                        RenderError::Layout("Component not found for note".to_string())
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        let edge_map: fn(&Component) -> (f32, f32) = match note.align() {
            NoteAlign::Over => |component| {
                let center_x = component.position().x();
                let width = component.drawable().size().width();
                let left_edge = center_x - width / 2.0;
                let right_edge = center_x + width / 2.0;
                (left_edge, right_edge)
            },
            NoteAlign::Left | NoteAlign::Right => {
                |component| (component.position().x(), component.position().x())
            }
            NoteAlign::Top | NoteAlign::Bottom => {
                unreachable!("Alignments is not supported for sequence diagrams")
            }
        };

        let (min_x, max_x) = filtered_components
            .into_iter()
            .map(edge_map)
            .reduce(|(min_x, max_x), (left_x, right_x)| (min_x.min(left_x), max_x.max(right_x)))
            .ok_or_else(|| {
                RenderError::Layout("Note should have at least one participant".to_string())
            })?;

        let mut new_note_def = Rc::clone(note.definition());
        if note.align() == NoteAlign::Over {
            let note_def_mut = Rc::make_mut(&mut new_note_def);
            note_def_mut.set_min_width(Some(max_x - min_x));
        }

        let note_drawable = DrawNote::new(new_note_def, note.content().to_string());

        let note_size = note_drawable.size();
        let center_x = match note.align() {
            NoteAlign::Over => (min_x + max_x) / 2.0,
            NoteAlign::Left => min_x - (note_size.width() / 2.0) - NOTE_SPACING,
            NoteAlign::Right => max_x + (note_size.width() / 2.0) + NOTE_SPACING,
            NoteAlign::Top | NoteAlign::Bottom => {
                unreachable!("Alignments is not supported for sequence diagrams")
            }
        };
        let position = Point::new(center_x, current_y + note_size.height() / 2.0);

        Ok(PositionedDrawable::new(note_drawable).with_position(position))
    }
}

impl SequenceEngine for Engine {
    fn calculate<'a>(
        &self,
        graph: &'a SequenceGraph<'a>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> Result<ContentStack<Layout<'a>>, RenderError> {
        self.calculate_layout(graph, embedded_layouts)
    }
}

/// Computes the two control points for a cubic-Bézier rendering of a straight
/// line from `a` to `b`.
///
/// # Returns
///
/// A tuple `(cp1, cp2)` of the two cubic-Bézier control points.
fn line_segment_cubic_cps(start: Point, end: Point) -> (Point, Point) {
    let dx = (end.x() - start.x()) / 3.0;
    let dy = (end.y() - start.y()) / 3.0;
    (
        Point::new(start.x() + dx, start.y() + dy),
        Point::new(start.x() + 2.0 * dx, start.y() + 2.0 * dy),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use orrery_core::{
        draw::{ActivationBoxDefinition, ArrowDefinition, ArrowDirection, RectangleDefinition},
        semantic::Node,
    };

    fn make_relation(source: Id, target: Id, label: Option<&str>) -> Relation {
        let mut def = ArrowDefinition::default();
        def.set_style(ArrowStyle::Straight);
        Relation::new(
            source,
            target,
            ArrowDirection::Forward,
            label.map(str::to_string),
            Rc::new(def),
        )
    }

    fn make_node(name: &str) -> Node {
        let id = Id::new(name);
        let shape_def = Rc::new(
            Box::new(RectangleDefinition::new()) as Box<dyn orrery_core::draw::ShapeDefinition>
        );
        Node::new(id, None, Block::None, shape_def)
    }

    fn make_component<'a>(node: &'a Node, position: Point) -> Component<'a> {
        let shape = Shape::new(Rc::clone(node.shape_definition()));
        let shape_with_text = ShapeWithText::new(shape, None);
        Component::new(node, shape_with_text, position)
    }

    #[test]
    fn test_position_messages_multiple() {
        let a_id = Id::new("a");
        let b_id = Id::new("b");
        let a_node = make_node("a");
        let b_node = make_node("b");
        let a_comp = make_component(&a_node, Point::new(50.0, 50.0));
        let b_comp = make_component(&b_node, Point::new(150.0, 50.0));

        let relation1 = make_relation(a_id, b_id, None);
        let mut msg1 = Message::from_relation(&relation1, &a_comp, &b_comp, None, None);
        msg1.set_y_position(100.0);

        let relation2 = make_relation(a_id, a_id, None);
        let mut msg2 = Message::from_relation(&relation2, &a_comp, &a_comp, None, None);
        msg2.set_y_position(200.0);

        let engine = Engine::new();
        let results = engine.position_messages(vec![msg1, msg2]);

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_position_message_dispatches_self_loop() {
        let id = Id::new("a");
        let node = make_node("a");
        let component = make_component(&node, Point::new(100.0, 50.0));

        let relation = make_relation(id, id, None);
        let mut msg = Message::from_relation(&relation, &component, &component, None, None);
        msg.set_y_position(200.0);

        let engine = Engine::new();
        // Should not panic; dispatches to self_loop_path
        let _positioned = engine.position_message(msg);
    }

    #[test]
    fn test_position_message_dispatches_cross_participant() {
        let a_id = Id::new("a");
        let b_id = Id::new("b");
        let a_node = make_node("a");
        let b_node = make_node("b");
        let a_comp = make_component(&a_node, Point::new(50.0, 50.0));
        let b_comp = make_component(&b_node, Point::new(150.0, 50.0));

        let relation = make_relation(a_id, b_id, None);
        let mut msg = Message::from_relation(&relation, &a_comp, &b_comp, None, None);
        msg.set_y_position(200.0);

        let engine = Engine::new();
        // Should not panic; dispatches to cross_participant_path
        let _positioned = engine.position_message(msg);
    }

    #[test]
    fn test_cross_participant_path() {
        let a_id = Id::new("a");
        let b_id = Id::new("b");
        let a_node = make_node("a");
        let b_node = make_node("b");
        let a_comp = make_component(&a_node, Point::new(50.0, 50.0));
        let b_comp = make_component(&b_node, Point::new(150.0, 50.0));

        let relation = make_relation(a_id, b_id, None);
        let mut msg = Message::from_relation(&relation, &a_comp, &b_comp, None, None);
        msg.set_y_position(200.0);

        let path = Engine::cross_participant_path(&msg);

        assert_eq!(path.source(), Point::new(50.0, 200.0));
        assert_eq!(path.destination(), Point::new(150.0, 200.0));
        assert!(path.control_points().is_empty());
    }

    #[test]
    fn test_cross_participant_path_with_activations() {
        let a_id = Id::new("a");
        let b_id = Id::new("b");
        let a_node = make_node("a");
        let b_node = make_node("b");
        let a_comp = make_component(&a_node, Point::new(50.0, 50.0));
        let b_comp = make_component(&b_node, Point::new(150.0, 50.0));

        // Default activation-box width = 8 → half-width = 4.
        let source_timing = Rc::new(RefCell::new(ActivationTiming::new(
            50.0,
            180.0,
            0,
            Rc::new(ActivationBoxDefinition::default()),
        )));
        let target_timing = Rc::new(RefCell::new(ActivationTiming::new(
            150.0,
            180.0,
            0,
            Rc::new(ActivationBoxDefinition::default()),
        )));

        let relation = make_relation(a_id, b_id, None);
        let mut msg = Message::from_relation(
            &relation,
            &a_comp,
            &b_comp,
            Some(Rc::clone(&source_timing)),
            Some(Rc::clone(&target_timing)),
        );
        msg.set_y_position(200.0);

        let path = Engine::cross_participant_path(&msg);

        // Source (a at x=50) sending rightward → right edge = 50 + 4 = 54
        assert_eq!(path.source(), Point::new(54.0, 200.0));
        // Target (b at x=150) receiving from left → left edge = 150 - 4 = 146
        assert_eq!(path.destination(), Point::new(146.0, 200.0));
    }

    #[test]
    fn test_cross_participant_path_nested_activation() {
        let a_id = Id::new("a");
        let b_id = Id::new("b");
        let a_node = make_node("a");
        let b_node = make_node("b");
        let a_comp = make_component(&a_node, Point::new(50.0, 50.0));
        let b_comp = make_component(&b_node, Point::new(150.0, 50.0));

        // Nesting level 1 → offset by nesting_offset (default 4.0)
        // Position shifts right: center_x = 50 + 4 (nesting offset)
        // Right edge = 50 + 4 + 4 (half-width) = 58
        let source_timing = Rc::new(RefCell::new(ActivationTiming::new(
            50.0,
            180.0,
            1,
            Rc::new(ActivationBoxDefinition::default()),
        )));

        let relation = make_relation(a_id, b_id, None);
        let mut msg = Message::from_relation(
            &relation,
            &a_comp,
            &b_comp,
            Some(Rc::clone(&source_timing)),
            None,
        );
        msg.set_y_position(200.0);

        let path = Engine::cross_participant_path(&msg);

        // Source nesting level 1: right edge = 50 + 4 (nesting) + 4 (half-width) = 58
        assert_eq!(path.source(), Point::new(58.0, 200.0));
        assert_eq!(path.destination(), Point::new(150.0, 200.0));
    }

    #[test]
    fn test_self_loop_path_no_activation() {
        let id = Id::new("a");
        let node = make_node("a");
        let component = make_component(&node, Point::new(100.0, 50.0));

        let relation = make_relation(id, id, None);
        let mut msg = Message::from_relation(&relation, &component, &component, None, None);
        msg.set_y_position(200.0);

        let mut engine = Engine::new();
        engine.set_self_loop_min_size(Size::new(40.0, 30.0));
        let (path, _label) = engine.self_loop_path(&msg);

        // Both endpoints sit on the lifeline X.
        assert_eq!(path.source().x(), 100.0);
        assert_eq!(path.destination().x(), 100.0);
        // 5 cubic-Bézier segments → 14 control points (4 anchors + 5 * 2 cps).
        assert_eq!(path.control_points().len(), 14);
    }

    #[test]
    fn test_self_loop_path_with_activation() {
        let id = Id::new("a");
        let node = make_node("a");
        let component = make_component(&node, Point::new(100.0, 50.0));

        // Default activation-box width = 8 → right edge sits at participant_x + 4.
        let timing = Rc::new(RefCell::new(ActivationTiming::new(
            100.0,
            180.0,
            0,
            Rc::new(ActivationBoxDefinition::default()),
        )));

        let relation = make_relation(id, id, None);
        let mut msg = Message::from_relation(
            &relation,
            &component,
            &component,
            Some(Rc::clone(&timing)),
            Some(Rc::clone(&timing)),
        );
        msg.set_y_position(200.0); // Inside the activation-box vertical range.

        let engine = Engine::new();
        let (path, _label) = engine.self_loop_path(&msg);

        assert_eq!(path.source().x(), 104.0);
        assert_eq!(path.destination().x(), 104.0);
    }

    #[test]
    fn test_message_min_size_cross() {
        let a = Id::new("a");
        let b = Id::new("b");
        let a_node = make_node("a");
        let b_node = make_node("b");
        let a_comp = make_component(&a_node, Point::new(50.0, 50.0));
        let b_comp = make_component(&b_node, Point::new(150.0, 50.0));
        let relation = make_relation(a, b, None);
        let msg = Message::from_relation(&relation, &a_comp, &b_comp, None, None);

        let size = Engine::new().message_min_size(&msg);
        assert_eq!(size, msg.min_size());
    }

    #[test]
    fn test_message_min_size_self_loop() {
        let id = Id::new("a");
        let node = make_node("a");
        let component = make_component(&node, Point::new(100.0, 50.0));
        let relation = make_relation(id, id, None);
        let msg = Message::from_relation(&relation, &component, &component, None, None);

        let mut engine = Engine::new();
        engine.set_self_loop_min_size(Size::new(30.0, 20.0));
        // Default corner_radius = 8. Content for an unlabeled `Forward` arrow is
        // (MARKER_SIZE, MARKER_SIZE) = (6, 6).
        // width  = self_loop_min_size.width() + SELF_LOOP_LABEL_GAP + content.width()
        //        = 30 + 4 + 6 = 40
        // height = max(content.height() + 2 * radius, self_loop_min_size.height())
        //        = max(6 + 16, 20) = 22
        assert_eq!(engine.message_min_size(&msg), Size::new(40.0, 22.0));
    }

    #[test]
    fn test_self_loop_with_content_min_arrow_size() {
        let mut engine = Engine::new();
        engine.set_self_loop_min_size(Size::new(30.0, 50.0));
        // Short content: content_height (10) + 2 * radius (8) = 26 < min_height (50).
        assert_eq!(
            engine.self_loop_with_content_min_arrow_size(Size::new(20.0, 10.0)),
            Size::new(30.0, 50.0),
        );

        // Tall content: content_height (40) + 16 = 56 > min_height (50).
        assert_eq!(
            engine.self_loop_with_content_min_arrow_size(Size::new(70.0, 40.0)),
            Size::new(30.0, 56.0),
        );
    }

    #[test]
    fn test_line_segment_cubic_cps() {
        let (cp1, cp2) = line_segment_cubic_cps(Point::new(0.0, 0.0), Point::new(30.0, 60.0));
        assert_eq!(cp1, Point::new(10.0, 20.0));
        assert_eq!(cp2, Point::new(20.0, 40.0));
    }
}
