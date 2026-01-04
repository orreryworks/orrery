use std::{collections::HashMap, rc::Rc};

use log::warn;

use crate::{
    draw, semantic,
    geometry::{Bounds, Point, Size},
    identifier::Id,
    layout::{component, positioning::LayoutBounds},
};

/// Sequence diagram participant that holds its drawable component and lifeline.
#[derive(Debug, Clone)]
pub struct Participant<'a> {
    component: component::Component<'a>,
    lifeline: draw::PositionedDrawable<draw::Lifeline>,
}

impl<'a> Participant<'a> {
    /// Create a participant from its component and lifeline.
    pub fn new(
        component: component::Component<'a>,
        lifeline: draw::PositionedDrawable<draw::Lifeline>,
    ) -> Self {
        Self {
            component,
            lifeline,
        }
    }

    /// Borrow the underlying component for this participant.
    pub fn component(&self) -> &component::Component<'_> {
        &self.component
    }

    /// Borrow the positioned lifeline drawable.
    pub fn lifeline(&self) -> &draw::PositionedDrawable<draw::Lifeline> {
        &self.lifeline
    }
}

#[derive(Debug, Clone)]
/// A rendered message between two participants at a specific Y position.
pub struct Message<'a> {
    source: Id,
    target: Id,
    y_position: f32,
    arrow_with_text: draw::ArrowWithText<'a>,
}

impl<'a> Message<'a> {
    /// Creates a new Message from an AST relation and participant IDs.
    ///
    /// This method extracts the arrow definition and text from the AST relation
    /// and creates a self-contained Message that doesn't depend on the
    /// original AST lifetime.
    ///
    /// # Arguments
    /// * `relation` - Reference to the AST relation being laid out
    /// * `source` - ID of the source participant in the layout
    /// * `target` - ID of the target participant in the layout
    /// * `y_position` - The y-coordinate where this message appears
    ///
    /// # Returns
    /// A new Message containing all necessary rendering information
    pub fn from_ast(relation: &'a semantic::Relation, source: Id, target: Id, y_position: f32) -> Self {
        let arrow_def = Rc::clone(relation.arrow_definition());
        let arrow = draw::Arrow::new(arrow_def, relation.arrow_direction());
        let mut arrow_with_text = draw::ArrowWithText::new(arrow);
        if let Some(text) = relation.text() {
            arrow_with_text.set_text(text);
        }
        Self {
            source,
            target,
            y_position,
            arrow_with_text,
        }
    }

    /// Returns a reference to the arrow with text for this message.
    pub fn arrow_with_text(&self) -> &draw::ArrowWithText<'a> {
        &self.arrow_with_text
    }

    /// Id of the source participant in the layout
    pub fn source(&self) -> Id {
        self.source
    }

    /// Id of the target participant in the layout
    pub fn target(&self) -> Id {
        self.target
    }

    /// The y-coordinate where this message appears
    pub fn y_position(&self) -> f32 {
        self.y_position
    }
}

/// Represents a rendered activation box in a sequence diagram.
///
/// An [`ActivationBox`] is the final result of activation timing calculations, containing
/// all the information needed to render an activation period on a participant's lifeline.
/// It represents the visual rectangle that appears on the lifeline to indicate when
/// a participant is active (has control focus).
///
/// # Key Features
///
/// - **Precise Positioning**: Contains exact center Y coordinate for rendering
/// - **Proper Encapsulation**: Private fields with controlled access through methods
/// - **Ready for Rendering**: Contains drawable object with styling and dimensions
/// - **Participant Association**: Tracks which participant this activation belongs to
///
/// # Creation
///
/// `ActivationBox` objects are created directly from [`ActivationTiming`] objects
/// during the ordered events processing using the [`ActivationTiming::to_activation_box`] method.
/// This conversion happens at the exact moment of deactivation with the precise end position.
#[derive(Debug, Clone)]
pub struct ActivationBox {
    participant_id: Id,
    center_y: f32,
    drawable: draw::ActivationBox,
}

/// Represents an activation period with precise timing information during processing.
///
/// [`ActivationTiming`] is a lightweight processing object used by the ordered events
/// system to track activation periods as they are being built. It contains the minimal
/// information needed during event processing and converts to an [`ActivationBox`]
/// when the activation period is complete.
///
/// # Lifecycle
///
/// 1. **Creation**: Created immediately when [`SequenceEvent::Activate`](crate::structure::SequenceEvent::Activate) occurs with exact start position
/// 2. **Stack Management**: Stored in participant-specific activation stacks
/// 3. **Conversion**: Converted to [`ActivationBox`] when [`SequenceEvent::Deactivate`](crate::structure::SequenceEvent::Deactivate) occurs
#[derive(Debug, Clone)]
pub struct ActivationTiming {
    participant_id: Id,
    start_y: f32,
    nesting_level: u32,
    definition: Rc<draw::ActivationBoxDefinition>,
}

impl ActivationTiming {
    /// Creates a new ActivationTiming with the given participant ID, start position, nesting level, and definition
    pub fn new(
        participant_id: Id,
        start_y: f32,
        nesting_level: u32,
        definition: Rc<draw::ActivationBoxDefinition>,
    ) -> Self {
        Self {
            participant_id,
            start_y,
            nesting_level,
            definition,
        }
    }

    /// Converts this ActivationTiming to an ActivationBox with the given end_y position
    pub fn to_activation_box(&self, end_y: f32) -> ActivationBox {
        const EDGE_CASE_BUFFER: f32 = 15.0;

        let end_y = if end_y <= self.start_y {
            self.start_y + EDGE_CASE_BUFFER
        } else {
            end_y
        };

        let center_y = (self.start_y() + end_y) / 2.0;
        let height = end_y - self.start_y();
        let drawable =
            draw::ActivationBox::new(Rc::clone(&self.definition), height, self.nesting_level());

        ActivationBox {
            participant_id: self.participant_id(),
            center_y,
            drawable,
        }
    }

    /// Returns the participant Id
    fn participant_id(&self) -> Id {
        self.participant_id
    }

    /// Returns the start Y coordinate
    fn start_y(&self) -> f32 {
        self.start_y
    }

    /// Returns the nesting level
    fn nesting_level(&self) -> u32 {
        self.nesting_level
    }
}

impl ActivationBox {
    /// Returns the participant Id for this activation box
    pub fn participant_id(&self) -> Id {
        self.participant_id
    }

    /// Returns the center Y coordinate for this activation box
    pub fn center_y(&self) -> f32 {
        self.center_y
    }

    /// Returns a reference to the drawable activation box
    pub fn drawable(&self) -> &draw::ActivationBox {
        &self.drawable
    }

    /// Calculate the bounds of this activation box when positioned at the given participant position.
    fn calculate_bounds(&self, participant_position: Point) -> Bounds {
        // Use the participant position but with the activation box's center_y
        let position_with_center_y = participant_position.with_y(self.center_y);

        self.drawable.calculate_bounds(position_with_center_y)
    }

    /// Check if this activation box is active at the given Y coordinate.
    /// An activation box is active if the Y coordinate falls within its vertical range.
    fn is_active_at_y(&self, y: f32) -> bool {
        let half_height = self.drawable.height() / 2.0;
        let min_y = self.center_y - half_height;
        let max_y = self.center_y + half_height;
        y >= min_y && y <= max_y
    }

    /// Get the X coordinate for the appropriate edge of this activation box based on message direction.
    /// For rightward messages (target_x > participant_x), returns the right edge.
    /// For leftward messages (target_x < participant_x), returns the left edge.
    fn intersection_x(&self, participant_position: Point, target_x: f32) -> f32 {
        let bounds = self.calculate_bounds(participant_position);

        if target_x > participant_position.x() {
            // Message going right, use right edge
            bounds.max_x()
        } else {
            // Message going left, use left edge
            bounds.min_x()
        }
    }
}

/// Tracks the timing and layout information for a fragment during sequence diagram layout.
///
/// This struct accumulates information about a fragment as it's being processed during
/// layout calculation, including its vertical position, horizontal bounds, and sections.
/// It's converted to a [`Fragment`](draw::Fragment) once processing is complete.
///
/// # Fields
/// - `start_y`: The Y coordinate where this fragment begins
/// - `min_x`: The minimum X coordinate covered by this fragment (updated as messages are added)
/// - `max_x`: The maximum X coordinate covered by this fragment (updated as messages are added)
/// - `fragment`: Reference to the AST fragment being processed
/// - `active_section`: Currently open section being processed (if any)
/// - `sections`: Completed sections within this fragment
pub struct FragmentTiming<'a> {
    start_y: f32,
    min_x: f32,
    max_x: f32,
    fragment: &'a semantic::Fragment,
    active_section: Option<(&'a semantic::FragmentSection, f32)>,
    sections: Vec<draw::FragmentSection>,
}

impl<'a> FragmentTiming<'a> {
    /// Creates a new `FragmentTiming` for the given fragment starting at the specified Y position.
    pub fn new(fragment: &'a semantic::Fragment, start_y: f32) -> Self {
        Self {
            start_y,
            min_x: f32::MAX,
            max_x: f32::MIN,
            fragment,
            active_section: None,
            sections: Vec::new(),
        }
    }

    /// Begins tracking a new section within this fragment.
    ///
    /// # Panics
    /// Panics in debug builds if there's already an active section.
    pub fn start_section(&mut self, section: &'a semantic::FragmentSection, start_y: f32) {
        #[cfg(debug_assertions)]
        assert!(self.active_section.is_none());

        self.active_section = Some((section, start_y));
    }

    /// Ends the currently active section and adds it to the completed sections list.
    ///
    /// # Returns
    /// - `Ok(())` if a section was successfully ended
    /// - `Err` if there's no active section to end
    pub fn end_section(&mut self, end_y: f32) -> Result<(), &'static str> {
        let (ast_section, start_y) = self
            .active_section
            .take()
            .ok_or("There is no active fragment section")?;
        let section = draw::FragmentSection::new(
            ast_section.title().map(|title| title.to_string()),
            end_y - start_y,
        );
        self.sections.push(section);
        Ok(())
    }

    /// Updates the horizontal bounds of this fragment as messages are processed.
    ///
    /// This method expands the fragment's X-axis coverage to include new min/max coordinates,
    /// ensuring the fragment encompasses all relevant messages.
    ///
    /// # Arguments
    /// * `source_x` - X coordinate of the message source participant
    /// * `target_x` - X coordinate of the message target participant
    pub fn update_x(&mut self, source_x: f32, target_x: f32) {
        self.min_x = self.min_x.min(source_x.min(target_x));
        self.max_x = self.max_x.max(source_x.max(target_x));
    }

    /// Converts this timing information into a final positioned Fragment.
    ///
    /// This consumes the `FragmentTiming` and creates a `Fragment` with complete bounds
    /// calculated from the accumulated start/end Y positions and min/max X coordinates.
    ///
    /// # Arguments
    /// * `end_y` - The final Y coordinate where this fragment ends
    ///
    /// # Panics
    /// Panics in debug builds if there's still an active section (all sections must be ended before conversion).
    ///
    /// # Returns
    /// A positioned `Fragment` ready for rendering
    pub fn into_fragment(self, end_y: f32) -> draw::PositionedDrawable<draw::Fragment> {
        #[cfg(debug_assertions)]
        assert!(self.active_section.is_none());

        let drawable = draw::Fragment::new(
            Rc::clone(self.fragment.definition()),
            self.fragment.operation().to_string(),
            self.sections,
            Size::new(self.max_x - self.min_x, end_y - self.start_y),
        );

        // Calculate the center position of the fragment
        let center_x = (self.min_x + self.max_x) / 2.0;
        let center_y = (self.start_y + end_y) / 2.0;
        let position = Point::new(center_x, center_y);

        draw::PositionedDrawable::new(drawable).with_position(position)
    }
}

/// Find the active activation box for a given participant at a specific Y coordinate.
///
/// This function searches through all activation boxes to find ones that:
/// 1. Belong to the specified participant (by participant_index)
/// 2. Are active at the given Y coordinate (message_y falls within their vertical range)
///
/// If multiple activation boxes are nested and active at the same Y coordinate,
/// returns the most nested one (highest nesting level) to ensure messages connect
/// to the outermost active activation box.
///
/// # Arguments
///
/// * `activation_boxes` - Slice of all activation boxes in the sequence diagram
/// * `participant_index` - Index of the participant to search for (0-based)
/// * `message_y` - Y coordinate where the message appears
///
/// # Returns
///
/// * `Some(&ActivationBox)` - Reference to the most nested active activation box
/// * `None` - If no activation boxes are active for this participant at this Y coordinate
pub fn find_active_activation_box_for_participant(
    activation_boxes: &[ActivationBox],
    participant_id: Id,
    message_y: f32,
) -> Option<&ActivationBox> {
    if activation_boxes.is_empty() {
        return None;
    }

    if !message_y.is_finite() {
        warn!("Invalid message_y coordinate: {message_y}. Skipping activation box search.");
        return None;
    }

    let mut active_boxes: Vec<&ActivationBox> = activation_boxes
        .iter()
        .filter(|activation_box| activation_box.participant_id() == participant_id)
        .filter(|activation_box| activation_box.is_active_at_y(message_y))
        .collect();

    if active_boxes.is_empty() {
        return None;
    }

    active_boxes.sort_by_key(|activation_box| activation_box.drawable().nesting_level());
    active_boxes.last().copied()
}

/// Calculate the X coordinate for a message endpoint, considering activation box intersections.
///
/// This function encapsulates the logic for determining where a message should start or end
/// by checking if there's an active activation box at the message Y coordinate. If an active
/// activation box is found, it calculates the appropriate edge intersection. Otherwise, it
/// falls back to using the participant's center X coordinate.
///
/// # Arguments
///
/// * `activation_boxes` - Slice of all activation boxes in the sequence diagram
/// * `participant` - The participant component for this endpoint
/// * `participant_index` - Index of the participant (0-based)
/// * `message_y` - Y coordinate where the message appears
/// * `target_x` - X coordinate of the target endpoint (used for direction detection)
///
/// # Returns
///
/// The X coordinate where the message should connect to this participant
pub fn calculate_message_endpoint_x(
    activation_boxes: &[ActivationBox],
    participant: &component::Component,
    participant_id: Id,
    message_y: f32,
    target_x: f32,
) -> f32 {
    if let Some(activation_box) =
        find_active_activation_box_for_participant(activation_boxes, participant_id, message_y)
    {
        activation_box.intersection_x(participant.position(), target_x)
    } else {
        participant.position().x()
    }
}

/// Sequence layout containing participants, messages, activation boxes, notes and metrics.
#[derive(Debug, Clone)]
pub struct Layout<'a> {
    participants: HashMap<Id, Participant<'a>>,
    messages: Vec<Message<'a>>,
    activations: Vec<ActivationBox>,
    fragments: Vec<draw::PositionedDrawable<draw::Fragment>>,
    notes: Vec<draw::PositionedDrawable<draw::Note>>,
    max_lifeline_end: f32, // TODO: Consider calculating on the fly.
    bounds: Bounds,
}

impl<'a> Layout<'a> {
    /// Construct a new sequence layout.
    pub fn new(
        participants: HashMap<Id, Participant<'a>>,
        messages: Vec<Message<'a>>,
        activations: Vec<ActivationBox>,
        fragments: Vec<draw::PositionedDrawable<draw::Fragment>>,
        notes: Vec<draw::PositionedDrawable<draw::Note>>,
        max_lifeline_end: f32,
    ) -> Self {
        let bounds = participants
            .values()
            .map(|participant| participant.component().bounds())
            .reduce(|acc, bounds| acc.merge(&bounds))
            .unwrap_or_default()
            .with_max_y(max_lifeline_end);

        Self {
            participants,
            messages,
            activations,
            fragments,
            notes,
            max_lifeline_end,
            bounds,
        }
    }

    /// Borrow all participants in this sequence layout.
    pub fn participants(&self) -> &HashMap<Id, Participant<'a>> {
        &self.participants
    }

    /// Borrow all messages in this sequence layout.
    pub fn messages(&self) -> &[Message<'a>] {
        &self.messages
    }

    /// Borrow all activation boxes in this sequence layout.
    pub fn activations(&self) -> &[ActivationBox] {
        &self.activations
    }

    /// Borrow all fragments in this sequence layout.
    pub fn fragments(&self) -> &[draw::PositionedDrawable<draw::Fragment>] {
        &self.fragments
    }

    /// Borrow all notes in this sequence layout.
    pub fn notes(&self) -> &[draw::PositionedDrawable<draw::Note>] {
        &self.notes
    }

    /// The maximum Y coordinate (bottom) reached by any lifeline.
    pub fn max_lifeline_end(&self) -> f32 {
        self.max_lifeline_end
    }
}

impl<'a> LayoutBounds for Layout<'a> {
    fn layout_bounds(&self) -> Bounds {
        self.bounds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activation_box_is_active_at_y() {
        // Create a test activation box with center_y=100.0 and height=20.0
        let definition = draw::ActivationBoxDefinition::default();
        let drawable = draw::ActivationBox::new(Rc::new(definition), 20.0, 0);
        let activation_box = ActivationBox {
            participant_id: Id::new("test"),
            center_y: 100.0,
            drawable,
        };

        // Should be active within the range [90.0, 110.0]
        assert!(activation_box.is_active_at_y(90.0)); // Bottom edge
        assert!(activation_box.is_active_at_y(100.0)); // Center
        assert!(activation_box.is_active_at_y(110.0)); // Top edge

        // Should not be active outside the range
        assert!(!activation_box.is_active_at_y(89.9));
        assert!(!activation_box.is_active_at_y(110.1));
    }

    #[test]
    fn test_activation_box_get_intersection_x() {
        // Create a test activation box with nesting level 0
        let definition = draw::ActivationBoxDefinition::default();
        let drawable = draw::ActivationBox::new(Rc::new(definition), 20.0, 0);
        let activation_box = ActivationBox {
            participant_id: Id::new("test"),
            center_y: 100.0,
            drawable,
        };

        let participant_position = Point::new(50.0, 80.0);

        // For rightward message (target_x > participant_x), should use right edge
        let rightward_x = activation_box.intersection_x(participant_position, 60.0);
        assert_eq!(rightward_x, 54.0); // 50.0 + 4.0 (half width)

        // For leftward message (target_x < participant_x), should use left edge
        let leftward_x = activation_box.intersection_x(participant_position, 40.0);
        assert_eq!(leftward_x, 46.0); // 50.0 - 4.0 (half width)
    }

    #[test]
    fn test_activation_box_nesting_offset() {
        // Test activation box with nesting level 2
        let definition = draw::ActivationBoxDefinition::default();
        let drawable = draw::ActivationBox::new(Rc::new(definition), 20.0, 2);
        let activation_box = ActivationBox {
            participant_id: Id::new("test"),
            center_y: 100.0,
            drawable,
        };

        let participant_position = Point::new(50.0, 80.0);
        let bounds = activation_box.calculate_bounds(participant_position);

        // With nesting level 2 and default nesting offset 4.0, X should be offset by 8.0
        // So bounds should span from 50.0 + 8.0 - 4.0 = 54.0 to 50.0 + 8.0 + 4.0 = 62.0
        assert_eq!(bounds.min_x(), 54.0);
        assert_eq!(bounds.max_x(), 62.0);
    }

    #[test]
    fn test_find_active_activation_box_for_participant() {
        // Create test activation boxes

        // Activation box 1: participant 0, Y range [90-110], nesting level 0
        let drawable1 =
            draw::ActivationBox::new(Rc::new(draw::ActivationBoxDefinition::default()), 20.0, 0);
        let activation_box1 = ActivationBox {
            participant_id: Id::new("test"),
            center_y: 100.0,
            drawable: drawable1,
        };

        // Activation box 2: participant 0, Y range [95-105], nesting level 1 (nested)
        let drawable2 =
            draw::ActivationBox::new(Rc::new(draw::ActivationBoxDefinition::default()), 10.0, 1);
        let activation_box2 = ActivationBox {
            participant_id: Id::new("test"),
            center_y: 100.0,
            drawable: drawable2,
        };

        // Activation box 3: participant 1, Y range [120-140], nesting level 0
        let drawable3 =
            draw::ActivationBox::new(Rc::new(draw::ActivationBoxDefinition::default()), 20.0, 0);
        let activation_box3 = ActivationBox {
            participant_id: Id::new("test"),
            center_y: 130.0,
            drawable: drawable3,
        };

        let activation_boxes = vec![activation_box1, activation_box2, activation_box3];

        // Test finding activation box for participant 0 at Y=100 (both boxes active, should return nested one)
        let result =
            find_active_activation_box_for_participant(&activation_boxes, Id::new("test"), 100.0);
        assert!(result.is_some());
        assert_eq!(result.unwrap().drawable().nesting_level(), 1); // Should return the more nested box

        // Test finding activation box for participant 0 at Y=92 (only first box active)
        let result =
            find_active_activation_box_for_participant(&activation_boxes, Id::new("test"), 92.0);
        assert!(result.is_some());
        assert_eq!(result.unwrap().drawable().nesting_level(), 0);

        // Test finding activation box for participant 0 at Y=80 (no boxes active)
        let result =
            find_active_activation_box_for_participant(&activation_boxes, Id::new("test"), 80.0);
        assert!(result.is_none());

        // Test finding activation box for participant 1 at Y=130 (different participant)
        let result =
            find_active_activation_box_for_participant(&activation_boxes, Id::new("test"), 130.0);
        assert!(result.is_some());
        assert_eq!(result.unwrap().participant_id(), "test");

        // Test finding activation box for non-existent participant
        let result = find_active_activation_box_for_participant(
            &activation_boxes,
            Id::new("invalid"),
            100.0,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_find_active_activation_box_edge_cases() {
        // Test with empty activation boxes list
        let result = find_active_activation_box_for_participant(&[], Id::new("test"), 100.0);
        assert!(result.is_none());

        // Test with multiple boxes at different nesting levels
        let boxes: Vec<ActivationBox> = (0..5)
            .map(|i| {
                let definition = draw::ActivationBoxDefinition::default();
                let drawable = draw::ActivationBox::new(Rc::new(definition), 20.0, i);
                ActivationBox {
                    participant_id: Id::new("test"),
                    center_y: 100.0,
                    drawable,
                }
            })
            .collect();

        // Should return the highest nesting level (4)
        let result = find_active_activation_box_for_participant(&boxes, Id::new("test"), 100.0);
        assert!(result.is_some());
        assert_eq!(result.unwrap().drawable().nesting_level(), 4);
    }

    #[test]
    fn test_find_active_activation_box_error_handling() {
        // Create a test activation box for error handling tests
        let definition = draw::ActivationBoxDefinition::default();
        let drawable = draw::ActivationBox::new(Rc::new(definition), 20.0, 0);
        let activation_box = ActivationBox {
            participant_id: Id::new("test"),
            center_y: 100.0,
            drawable,
        };
        let activation_boxes = vec![activation_box];

        // Test with NaN Y coordinate
        let result = find_active_activation_box_for_participant(
            &activation_boxes,
            Id::new("test"),
            f32::NAN,
        );
        assert!(result.is_none());

        // Test with infinite Y coordinate
        let result = find_active_activation_box_for_participant(
            &activation_boxes,
            Id::new("test"),
            f32::INFINITY,
        );
        assert!(result.is_none());

        // Test with negative infinite Y coordinate
        let result = find_active_activation_box_for_participant(
            &activation_boxes,
            Id::new("test"),
            f32::NEG_INFINITY,
        );
        assert!(result.is_none());

        // Test with valid Y coordinate (should work normally)
        let result =
            find_active_activation_box_for_participant(&activation_boxes, Id::new("test"), 100.0);
        assert!(result.is_some());
    }

    #[test]
    fn test_message_endpoint_fallback_behavior() {
        // Test with empty activation boxes (should fallback)
        let empty_boxes: Vec<ActivationBox> = vec![];
        let result =
            find_active_activation_box_for_participant(&empty_boxes, Id::new("test_0"), 100.0);
        assert!(result.is_none());

        // Test with activation boxes for different participant (should fallback)
        let definition = draw::ActivationBoxDefinition::default();
        let drawable = draw::ActivationBox::new(Rc::new(definition), 20.0, 0);
        let activation_box = ActivationBox {
            participant_id: Id::new("test_1"), // Different participant
            center_y: 100.0,
            drawable,
        };
        let activation_boxes = vec![activation_box];

        let result =
            find_active_activation_box_for_participant(&activation_boxes, Id::new("test_0"), 100.0);
        assert!(result.is_none());

        // Test with activation box not active at Y coordinate (should fallback)
        let result =
            find_active_activation_box_for_participant(&activation_boxes, Id::new("test_1"), 200.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_fragment_timing_lifecycle() {
        // Create a mock semantic::Fragment for testing
        let fragment_def = Rc::new(draw::FragmentDefinition::default());

        let section1 = semantic::FragmentSection::new(Some("section 1".to_string()), vec![]);
        let section2 = semantic::FragmentSection::new(Some("section 2".to_string()), vec![]);

        let fragment =
            semantic::Fragment::new("alt".to_string(), vec![section1, section2], fragment_def);

        // Create FragmentTiming
        let start_y = 100.0;
        let mut fragment_timing = FragmentTiming::new(&fragment, start_y);

        // Start first section
        fragment_timing.start_section(&fragment.sections()[0], 120.0);

        // End first section
        let result = fragment_timing.end_section(180.0);
        assert!(result.is_ok());

        // Start second section
        fragment_timing.start_section(&fragment.sections()[1], 180.0);

        // End second section
        let result = fragment_timing.end_section(240.0);
        assert!(result.is_ok());

        // Update bounds
        fragment_timing.update_x(50.0, 200.0);

        // Convert to final Fragment
        let end_y = 250.0;
        let final_fragment = fragment_timing.into_fragment(end_y);

        // Verify the final fragment has a drawable
        assert!(final_fragment.inner().size().height() > 0.0);
        assert!(final_fragment.inner().size().width() > 0.0);
    }

    #[test]
    fn test_fragment_timing_bounds_tracking() {
        // Create a mock semantic::Fragment
        let fragment_def = Rc::new(draw::FragmentDefinition::default());

        let fragment = semantic::Fragment::new("opt".to_string(), vec![], fragment_def);

        let mut fragment_timing = FragmentTiming::new(&fragment, 100.0);

        // Initially, bounds should be at extremes
        assert_eq!(fragment_timing.min_x, f32::MAX);
        assert_eq!(fragment_timing.max_x, f32::MIN);

        // Update with first message (source at 50.0, target at 150.0)
        fragment_timing.update_x(50.0, 150.0);
        assert_eq!(fragment_timing.min_x, 50.0);
        assert_eq!(fragment_timing.max_x, 150.0);

        // Update with message extending left (source at 30.0, target at 100.0)
        fragment_timing.update_x(30.0, 100.0);
        assert_eq!(fragment_timing.min_x, 30.0);
        assert_eq!(fragment_timing.max_x, 150.0); // max unchanged

        // Update with message extending right (source at 60.0, target at 200.0)
        fragment_timing.update_x(60.0, 200.0);
        assert_eq!(fragment_timing.min_x, 30.0); // min unchanged
        assert_eq!(fragment_timing.max_x, 200.0);

        // Update with message within current bounds (source at 40.0, target at 180.0)
        fragment_timing.update_x(40.0, 180.0);
        assert_eq!(fragment_timing.min_x, 30.0);
        assert_eq!(fragment_timing.max_x, 200.0);
    }
}
