//! Sequence diagram layout algorithms.
//!
//! This module computes positions for participants, messages, activations,
//! and fragments in sequence diagrams.

use std::{collections::HashMap, rc::Rc};

use orrery_core::{
    draw::{
        ActivationBox as DrawActivationBox, ActivationBoxDefinition, Fragment as DrawFragment,
        FragmentSection as DrawFragmentSection, Lifeline, Note, PositionedArrowWithText,
        PositionedDrawable,
    },
    geometry::{Bounds, Point, Size},
    identifier::Id,
    semantic::{Fragment, FragmentSection},
};

use crate::layout::{component::Component, positioning::LayoutBounds};

/// Sequence diagram participant that holds its drawable component and lifeline.
#[derive(Debug, Clone)]
pub struct Participant<'a> {
    component: Component<'a>,
    lifeline: PositionedDrawable<Lifeline>,
}

impl<'a> Participant<'a> {
    /// Create a participant from its component and lifeline.
    pub fn new(component: Component<'a>, lifeline: PositionedDrawable<Lifeline>) -> Self {
        Self {
            component,
            lifeline,
        }
    }

    /// Borrow the underlying component for this participant.
    pub fn component(&self) -> &Component<'_> {
        &self.component
    }

    /// Borrow the positioned lifeline drawable.
    pub fn lifeline(&self) -> &PositionedDrawable<Lifeline> {
        &self.lifeline
    }
}

/// A finalized activation box on a sequence participant's lifeline.
///
/// Carries the participant's lifeline X and the box's center Y so it can be
/// positioned independently of the participant map.
#[derive(Debug, Clone)]
pub struct ActivationBox {
    center_y: f32,
    participant_x: f32,
    drawable: DrawActivationBox,
}

/// Start-side record of an activation, paired with an end Y to produce an [`ActivationBox`].
#[derive(Debug, Clone)]
pub struct ActivationTiming {
    participant_x: f32,
    start_y: f32,
    nesting_level: u32,
    definition: Rc<ActivationBoxDefinition>,
}

impl ActivationTiming {
    /// Creates an activation timing from a participant X coordinate and start Y.
    pub fn new(
        participant_x: f32,
        start_y: f32,
        nesting_level: u32,
        definition: Rc<ActivationBoxDefinition>,
    ) -> Self {
        Self {
            participant_x,
            start_y,
            nesting_level,
            definition,
        }
    }

    /// Converts this timing into a finalized [`ActivationBox`].
    ///
    /// If `end_y <= start_y`, a small buffer gives same-line activations a
    /// visible height.
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
            DrawActivationBox::new(Rc::clone(&self.definition), height, self.nesting_level());

        ActivationBox {
            participant_x: self.participant_x,
            center_y,
            drawable,
        }
    }

    /// Returns the start Y coordinate.
    fn start_y(&self) -> f32 {
        self.start_y
    }

    /// Returns the activation nesting level.
    fn nesting_level(&self) -> u32 {
        self.nesting_level
    }
}

impl ActivationBox {
    /// Returns the participant lifeline X coordinate used to place this box.
    pub fn participant_x(&self) -> f32 {
        self.participant_x
    }

    /// Returns the center Y coordinate of this activation box.
    pub fn center_y(&self) -> f32 {
        self.center_y
    }

    /// Returns the drawable activation box.
    pub fn drawable(&self) -> &DrawActivationBox {
        &self.drawable
    }

    /// Returns the activation edge facing a message endpoint at `target_x`.
    pub fn intersection_x(&self, participant_position: Point, target_x: f32) -> f32 {
        let bounds = self.calculate_bounds(participant_position);

        if target_x > participant_position.x() {
            // Message going right, use right edge
            bounds.max_x()
        } else {
            // Message going left, use left edge
            bounds.min_x()
        }
    }

    /// Calculates bounds at `participant_position.x()` and this box's center Y.
    fn calculate_bounds(&self, participant_position: Point) -> Bounds {
        let position_with_center_y = participant_position.with_y(self.center_y);

        self.drawable.calculate_bounds(position_with_center_y)
    }
}

/// Tracks the timing and layout information for a fragment during sequence diagram layout.
///
/// This struct accumulates information about a fragment as it's being processed during
/// layout calculation, including its vertical position, horizontal bounds, and sections.
/// It's converted to a [`Fragment`](DrawFragment) once processing is complete.
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
    fragment: &'a Fragment,
    active_section: Option<(&'a FragmentSection, f32)>,
    sections: Vec<DrawFragmentSection>,
}

impl<'a> FragmentTiming<'a> {
    /// Creates a new `FragmentTiming` for the given fragment starting at the specified Y position.
    pub fn new(fragment: &'a Fragment, start_y: f32) -> Self {
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
    pub fn start_section(&mut self, section: &'a FragmentSection, start_y: f32) {
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
        let section = DrawFragmentSection::new(
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

    /// Returns the height needed for a section's header.
    ///
    /// For the first section, returns the max of the fragment header height and the
    /// section header height, since they render side-by-side. For subsequent sections,
    /// returns just the section header height.
    ///
    /// # Arguments
    ///
    /// * `section` - The semantic fragment section to measure.
    pub fn section_header_height(&self, section: &FragmentSection) -> f32 {
        let definition = self.fragment.definition();
        let section_header_height = definition.section_header_size(section.title()).height();

        if self.sections.is_empty() {
            let fragment_header_height = definition.header_size(self.fragment.operation()).height();
            section_header_height.max(fragment_header_height)
        } else {
            section_header_height
        }
    }

    /// Returns the bottom bounds padding of the fragment.
    pub fn bottom_padding(&self) -> f32 {
        self.fragment.definition().bottom_padding()
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
    pub fn into_fragment(self, end_y: f32) -> PositionedDrawable<DrawFragment> {
        #[cfg(debug_assertions)]
        assert!(self.active_section.is_none());

        let drawable = DrawFragment::new(
            Rc::clone(self.fragment.definition()),
            self.fragment.operation().to_string(),
            self.sections,
            Size::new(self.max_x - self.min_x, end_y - self.start_y),
        );

        // Calculate the center position of the fragment
        let center_x = (self.min_x + self.max_x) / 2.0;
        let center_y = (self.start_y + end_y) / 2.0;
        let position = Point::new(center_x, center_y);

        PositionedDrawable::new(drawable).with_position(position)
    }
}

/// Sequence layout containing participants, messages, activation boxes, notes and metrics.
#[derive(Debug, Clone)]
pub struct Layout<'a> {
    participants: HashMap<Id, Participant<'a>>,
    messages: Vec<PositionedArrowWithText<'a>>,
    activations: Vec<ActivationBox>,
    fragments: Vec<PositionedDrawable<DrawFragment>>,
    notes: Vec<PositionedDrawable<Note>>,
    max_lifeline_end: f32, // TODO: Consider calculating on the fly.
    bounds: Bounds,
}

impl<'a> Layout<'a> {
    /// Construct a new sequence layout.
    pub fn new(
        participants: HashMap<Id, Participant<'a>>,
        messages: Vec<PositionedArrowWithText<'a>>,
        activations: Vec<ActivationBox>,
        fragments: Vec<PositionedDrawable<DrawFragment>>,
        notes: Vec<PositionedDrawable<Note>>,
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
    pub fn messages(&self) -> &[PositionedArrowWithText<'a>] {
        &self.messages
    }

    /// Borrow all activation boxes in this sequence layout.
    pub fn activations(&self) -> &[ActivationBox] {
        &self.activations
    }

    /// Borrow all fragments in this sequence layout.
    pub fn fragments(&self) -> &[PositionedDrawable<DrawFragment>] {
        &self.fragments
    }

    /// Borrow all notes in this sequence layout.
    pub fn notes(&self) -> &[PositionedDrawable<Note>] {
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
    use orrery_core::draw::{Drawable, FragmentDefinition};

    use super::*;

    #[test]
    fn test_activation_box_get_intersection_x() {
        // Create a test activation box with nesting level 0
        let definition = ActivationBoxDefinition::default();
        let drawable = DrawActivationBox::new(Rc::new(definition), 20.0, 0);
        let activation_box = ActivationBox {
            center_y: 100.0,
            participant_x: 50.0,
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
        let definition = ActivationBoxDefinition::default();
        let drawable = DrawActivationBox::new(Rc::new(definition), 20.0, 2);
        let activation_box = ActivationBox {
            center_y: 100.0,
            participant_x: 50.0,
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
    fn test_fragment_timing_lifecycle() {
        // Create a mock Fragment for testing
        let fragment_def = Rc::new(FragmentDefinition::default());

        let section1 = FragmentSection::new(Some("section 1".to_string()), vec![]);
        let section2 = FragmentSection::new(Some("section 2".to_string()), vec![]);

        let fragment = Fragment::new("alt".to_string(), vec![section1, section2], fragment_def);

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
        // Create a mock Fragment
        let fragment_def = Rc::new(FragmentDefinition::default());

        let fragment = Fragment::new("opt".to_string(), vec![], fragment_def);

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

    #[test]
    fn test_section_header_height_first_section_header_dominates() {
        // Use a short section title so the fragment header (pentagon) is taller.
        let fragment_def = Rc::new(FragmentDefinition::default());
        let section = FragmentSection::new(Some("x".to_string()), vec![]);
        let fragment = Fragment::new("alt".to_string(), vec![section], fragment_def.clone());

        let fragment_timing = FragmentTiming::new(&fragment, 0.0);

        let height = fragment_timing.section_header_height(&fragment.sections()[0]);
        let header_height = fragment_def.header_size("alt").height();

        assert_eq!(height, header_height);
    }

    #[test]
    fn test_section_header_height_first_section_title_dominates() {
        // Use a multi-line section title so the title is taller than the header.
        let fragment_def = Rc::new(FragmentDefinition::default());
        let tall_title = "line1\nline2\nline3\nline4\nline5";
        let section = FragmentSection::new(Some(tall_title.to_string()), vec![]);
        let fragment = Fragment::new("alt".to_string(), vec![section], fragment_def.clone());

        let fragment_timing = FragmentTiming::new(&fragment, 0.0);

        let height = fragment_timing.section_header_height(&fragment.sections()[0]);
        let title_height = fragment_def.section_header_size(Some(tall_title)).height();

        assert_eq!(height, title_height);
    }

    #[test]
    fn test_section_header_height_subsequent_section() {
        let fragment_def = Rc::new(FragmentDefinition::default());
        let section1 = FragmentSection::new(Some("guard1".to_string()), vec![]);
        let section2 = FragmentSection::new(Some("guard2".to_string()), vec![]);
        let fragment = Fragment::new(
            "alt".to_string(),
            vec![section1, section2],
            fragment_def.clone(),
        );

        let mut fragment_timing = FragmentTiming::new(&fragment, 0.0);

        // Complete the first section so sections is no longer empty.
        fragment_timing.start_section(&fragment.sections()[0], 0.0);
        fragment_timing.end_section(50.0).unwrap();

        // Second section: should be just the title height.
        let height = fragment_timing.section_header_height(&fragment.sections()[1]);
        let title_height = fragment_def.section_header_size(Some("guard2")).height();

        assert_eq!(height, title_height);
    }

    #[test]
    fn test_section_header_height_no_title() {
        let fragment_def = Rc::new(FragmentDefinition::default());
        let section = FragmentSection::new(None, vec![]);
        let fragment = Fragment::new("opt".to_string(), vec![section], fragment_def.clone());

        let mut fragment_timing = FragmentTiming::new(&fragment, 0.0);

        // Complete first section.
        fragment_timing.start_section(&fragment.sections()[0], 0.0);
        fragment_timing.end_section(50.0).unwrap();

        // Subsequent section with no title: height should be 0.
        let no_title_section = FragmentSection::new(None, vec![]);
        let height = fragment_timing.section_header_height(&no_title_section);

        assert_eq!(height, 0.0);
    }

    #[test]
    fn test_bottom_padding_delegates_to_definition() {
        let mut fragment_def = FragmentDefinition::default();
        fragment_def.set_bounds_padding(orrery_core::geometry::Insets::new(5.0, 10.0, 15.0, 20.0));
        let fragment_def = Rc::new(fragment_def);

        let fragment = Fragment::new("loop".to_string(), vec![], fragment_def);

        let fragment_timing = FragmentTiming::new(&fragment, 0.0);

        assert_eq!(fragment_timing.bottom_padding(), 15.0);
    }
}
