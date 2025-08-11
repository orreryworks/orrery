use crate::{
    ast, draw,
    geometry::{Bounds, Point, Size},
    graph,
    layout::{component, layer, positioning::LayoutSizing},
};
use log::{debug, error, warn};
use std::{collections::HashMap, rc::Rc};

#[derive(Debug, Clone)]
pub struct Participant<'a> {
    pub component: component::Component<'a>,
    pub lifeline_end: f32, // y-coordinate where lifeline ends
}

#[derive(Debug, Clone)]
pub struct Message {
    pub source_index: usize,
    pub target_index: usize,
    pub y_position: f32,
    arrow_with_text: draw::ArrowWithText,
}

impl Message {
    /// Creates a new Message from an AST relation and participant indices.
    ///
    /// This method extracts the arrow definition and text from the AST relation
    /// and creates a self-contained Message that doesn't depend on the
    /// original AST lifetime.
    ///
    /// # Arguments
    /// * `relation` - Reference to the AST relation being laid out
    /// * `source_index` - Index of the source participant in the layout
    /// * `target_index` - Index of the target participant in the layout
    /// * `y_position` - The y-coordinate where this message appears
    ///
    /// # Returns
    /// A new Message containing all necessary rendering information
    pub fn from_ast(
        relation: &ast::Relation,
        source_index: usize,
        target_index: usize,
        y_position: f32,
    ) -> Self {
        let arrow_def = relation.clone_arrow_definition();
        let arrow = draw::Arrow::new(arrow_def, relation.arrow_direction);
        let mut arrow_with_text = draw::ArrowWithText::new(arrow);
        if let Some(text) = relation.text() {
            arrow_with_text.set_text(text);
        }
        Self {
            source_index,
            target_index,
            y_position,
            arrow_with_text,
        }
    }

    /// Returns a reference to the arrow with text for this message.
    pub fn arrow_with_text(&self) -> &draw::ArrowWithText {
        &self.arrow_with_text
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
    participant_index: usize,
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
/// 1. **Creation**: Created immediately when [`Event::Activate`] occurs with exact start position
/// 2. **Stack Management**: Stored in participant-specific activation stacks
/// 3. **Conversion**: Converted to [`ActivationBox`] when [`Event::Deactivate`] occurs
#[derive(Debug, Clone)]
pub struct ActivationTiming {
    participant_index: usize,
    start_y: f32,
    nesting_level: u32,
}

impl ActivationTiming {
    /// Creates a new ActivationTiming with the given participant index, start position, and nesting level
    pub fn new(participant_index: usize, start_y: f32, nesting_level: u32) -> Self {
        Self {
            participant_index,
            start_y,
            nesting_level,
        }
    }

    /// Converts this ActivationTiming to an ActivationBox with the given end_y position
    pub fn to_activation_box(&self, end_y: f32) -> ActivationBox {
        let center_y = (self.start_y() + end_y) / 2.0;
        let height = end_y - self.start_y();
        let definition = Rc::new(draw::ActivationBoxDefinition::default());
        let drawable = draw::ActivationBox::new(definition, height, self.nesting_level());

        ActivationBox {
            participant_index: self.participant_index(),
            center_y,
            drawable,
        }
    }

    /// Returns the participant index
    fn participant_index(&self) -> usize {
        self.participant_index
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
    /// Returns the participant index for this activation box
    pub fn participant_index(&self) -> usize {
        self.participant_index
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
    participant_index: usize,
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
        .filter(|activation_box| activation_box.participant_index() == participant_index)
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
    participant_index: usize,
    message_y: f32,
    target_x: f32,
) -> f32 {
    if let Some(activation_box) =
        find_active_activation_box_for_participant(activation_boxes, participant_index, message_y)
    {
        activation_box.intersection_x(participant.position(), target_x)
    } else {
        participant.position().x()
    }
}

#[derive(Debug, Clone)]
pub struct Layout<'a> {
    pub participants: Vec<Participant<'a>>,
    pub messages: Vec<Message>,
    pub activations: Vec<ActivationBox>,
}

impl<'a> LayoutSizing for Layout<'a> {
    fn layout_size(&self) -> Size {
        // For sequence layouts, calculate bounds based on participants and messages
        if self.participants.is_empty() {
            return Size::default();
        }

        // Find max lifeline end for height
        let max_y = self
            .participants
            .iter()
            .map(|p| p.lifeline_end)
            .fold(0.0, f32::max);

        // Find bounds for width
        let bounds = self
            .participants
            .iter()
            .skip(1)
            .fold(self.participants[0].component.bounds(), |acc, p| {
                acc.merge(&p.component.bounds())
            });

        Size::new(
            bounds.width(),
            max_y - bounds.min_y(), // Height from top to bottom lifeline
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activation_box_is_active_at_y() {
        // Create a test activation box with center_y=100.0 and height=20.0
        let definition = Rc::new(draw::ActivationBoxDefinition::default());
        let drawable = draw::ActivationBox::new(definition, 20.0, 0);
        let activation_box = ActivationBox {
            participant_index: 0,
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
        let definition = Rc::new(draw::ActivationBoxDefinition::default());
        let drawable = draw::ActivationBox::new(definition, 20.0, 0);
        let activation_box = ActivationBox {
            participant_index: 0,
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
        let definition = Rc::new(draw::ActivationBoxDefinition::default());
        let drawable = draw::ActivationBox::new(definition, 20.0, 2);
        let activation_box = ActivationBox {
            participant_index: 0,
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
        let definition = Rc::new(draw::ActivationBoxDefinition::default());

        // Activation box 1: participant 0, Y range [90-110], nesting level 0
        let drawable1 = draw::ActivationBox::new(definition.clone(), 20.0, 0);
        let activation_box1 = ActivationBox {
            participant_index: 0,
            center_y: 100.0,
            drawable: drawable1,
        };

        // Activation box 2: participant 0, Y range [95-105], nesting level 1 (nested)
        let drawable2 = draw::ActivationBox::new(definition.clone(), 10.0, 1);
        let activation_box2 = ActivationBox {
            participant_index: 0,
            center_y: 100.0,
            drawable: drawable2,
        };

        // Activation box 3: participant 1, Y range [120-140], nesting level 0
        let drawable3 = draw::ActivationBox::new(definition, 20.0, 0);
        let activation_box3 = ActivationBox {
            participant_index: 1,
            center_y: 130.0,
            drawable: drawable3,
        };

        let activation_boxes = vec![activation_box1, activation_box2, activation_box3];

        // Test finding activation box for participant 0 at Y=100 (both boxes active, should return nested one)
        let result = find_active_activation_box_for_participant(&activation_boxes, 0, 100.0);
        assert!(result.is_some());
        assert_eq!(result.unwrap().drawable().nesting_level(), 1); // Should return the more nested box

        // Test finding activation box for participant 0 at Y=92 (only first box active)
        let result = find_active_activation_box_for_participant(&activation_boxes, 0, 92.0);
        assert!(result.is_some());
        assert_eq!(result.unwrap().drawable().nesting_level(), 0);

        // Test finding activation box for participant 0 at Y=80 (no boxes active)
        let result = find_active_activation_box_for_participant(&activation_boxes, 0, 80.0);
        assert!(result.is_none());

        // Test finding activation box for participant 1 at Y=130 (different participant)
        let result = find_active_activation_box_for_participant(&activation_boxes, 1, 130.0);
        assert!(result.is_some());
        assert_eq!(result.unwrap().participant_index(), 1);

        // Test finding activation box for non-existent participant
        let result = find_active_activation_box_for_participant(&activation_boxes, 2, 100.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_active_activation_box_edge_cases() {
        // Test with empty activation boxes list
        let result = find_active_activation_box_for_participant(&[], 0, 100.0);
        assert!(result.is_none());

        // Test with multiple boxes at different nesting levels
        let definition = Rc::new(draw::ActivationBoxDefinition::default());

        let boxes: Vec<ActivationBox> = (0..5)
            .map(|i| {
                let drawable = draw::ActivationBox::new(definition.clone(), 20.0, i);
                ActivationBox {
                    participant_index: 0,
                    center_y: 100.0,
                    drawable,
                }
            })
            .collect();

        // Should return the highest nesting level (4)
        let result = find_active_activation_box_for_participant(&boxes, 0, 100.0);
        assert!(result.is_some());
        assert_eq!(result.unwrap().drawable().nesting_level(), 4);
    }

    #[test]
    fn test_find_active_activation_box_error_handling() {
        // Create a test activation box for error handling tests
        let definition = Rc::new(draw::ActivationBoxDefinition::default());
        let drawable = draw::ActivationBox::new(definition, 20.0, 0);
        let activation_box = ActivationBox {
            participant_index: 0,
            center_y: 100.0,
            drawable,
        };
        let activation_boxes = vec![activation_box];

        // Test with NaN Y coordinate
        let result = find_active_activation_box_for_participant(&activation_boxes, 0, f32::NAN);
        assert!(result.is_none());

        // Test with infinite Y coordinate
        let result =
            find_active_activation_box_for_participant(&activation_boxes, 0, f32::INFINITY);
        assert!(result.is_none());

        // Test with negative infinite Y coordinate
        let result =
            find_active_activation_box_for_participant(&activation_boxes, 0, f32::NEG_INFINITY);
        assert!(result.is_none());

        // Test with valid Y coordinate (should work normally)
        let result = find_active_activation_box_for_participant(&activation_boxes, 0, 100.0);
        assert!(result.is_some());
    }

    #[test]
    fn test_message_endpoint_fallback_behavior() {
        // Test with empty activation boxes (should fallback)
        let empty_boxes: Vec<ActivationBox> = vec![];
        let result = find_active_activation_box_for_participant(&empty_boxes, 0, 100.0);
        assert!(result.is_none());

        // Test with activation boxes for different participant (should fallback)
        let definition = Rc::new(draw::ActivationBoxDefinition::default());
        let drawable = draw::ActivationBox::new(definition, 20.0, 0);
        let activation_box = ActivationBox {
            participant_index: 1, // Different participant
            center_y: 100.0,
            drawable,
        };
        let activation_boxes = vec![activation_box];

        let result = find_active_activation_box_for_participant(&activation_boxes, 0, 100.0);
        assert!(result.is_none());

        // Test with activation box not active at Y coordinate (should fallback)
        let result = find_active_activation_box_for_participant(&activation_boxes, 1, 200.0);
        assert!(result.is_none());
    }
}

/// Adjusts the offset of positioned contents in a content stack based on containment relationships.
///
/// This method handles the proper positioning of nested elements within their containers.
// TODO: Once added enough abstractions, make this a method on ContentStack.
pub fn adjust_positioned_contents_offset<'a>(
    content_stack: &mut layer::ContentStack<Layout<'a>>,
    graph: &'a graph::Graph<'a>,
) {
    let container_indices: HashMap<_, _> = graph
        .containment_scopes()
        .iter()
        .enumerate()
        .filter_map(|(idx, scope)| scope.container().map(|container| (container, idx)))
        .collect();

    for (source_idx, source_scope) in graph.containment_scopes().iter().enumerate().rev() {
        for (node_idx, destination_idx) in source_scope.node_indices().filter_map(|node_idx| {
            container_indices
                .get(&node_idx)
                .map(|&destination_idx| (node_idx, destination_idx))
        }) {
            if source_idx == destination_idx {
                // If the source and destination are the same, skip
                error!(index = source_idx; "Source and destination indices are the same");
                continue;
            }
            let source = content_stack.get_unchecked(source_idx);
            let node = graph.node_from_idx(node_idx);

            // Find the participant in the source layer that matches the node
            let source_participant = source
                .content()
                .participants
                .iter()
                .find(|participant| *participant.component.node_id() == node.id)
                .expect("Participant must exist in source layer");

            let target_offset = source
                .offset()
                .add_point(source_participant.component.bounds().min_point())
                .add_point(
                    source_participant
                        .component
                        .drawable()
                        .inner()
                        .shape_to_inner_content_min_point(),
                ); // TODO: This does not account for text.

            debug!(
                node_id:? = node.id,
                source_offset:? = source.offset();
                "Adjusting positioned content offset [source]",
            );

            let target = content_stack.get_mut_unchecked(destination_idx);
            debug!(
                node_id:? = node.id,
                original_offset:? = target.offset(),
                new_offset:? = target_offset;
                "Adjusting positioned content offset [target]",
            );

            target.set_offset(target_offset);
        }
    }
}
