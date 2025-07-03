use crate::{
    ast,
    geometry::Size,
    graph,
    layout::{component, layer, positioning::LayoutSizing},
};
use log::{debug, error};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Participant<'a> {
    pub component: component::Component<'a>,
    pub lifeline_end: f32, // y-coordinate where lifeline ends
}

#[derive(Debug, Clone)]
pub struct Message<'a> {
    pub relation: &'a ast::Relation,
    pub source_index: usize,
    pub target_index: usize,
    pub y_position: f32,
}

#[derive(Debug, Clone)]
pub struct Layout<'a> {
    pub participants: Vec<Participant<'a>>,
    pub messages: Vec<Message<'a>>,
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
