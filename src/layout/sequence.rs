use crate::{
    ast, draw,
    geometry::Size,
    graph,
    layout::{component, layer, positioning::LayoutSizing},
};
use log::{debug, error};
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

#[derive(Debug, Clone)]
pub struct ActivationBox {
    pub participant_index: usize,
    pub start_y: f32,
    pub drawable: draw::ActivationBox,
}

impl ActivationBox {
    /// Creates a new activation box with a drawable component
    pub fn new(participant_index: usize, start_y: f32, end_y: f32, nesting_level: u32) -> Self {
        let height = end_y - start_y;
        let definition = Rc::new(draw::ActivationBoxDefinition::default());
        let drawable = draw::ActivationBox::new(definition, height, nesting_level);

        Self {
            participant_index,
            start_y,
            drawable,
        }
    }

    /// Returns the drawable activation box
    pub fn drawable(&self) -> &draw::ActivationBox {
        &self.drawable
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
