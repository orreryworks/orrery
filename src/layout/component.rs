use crate::{
    ast, graph,
    layout::{
        geometry::{Component, LayoutSizing, Size},
        layer,
    },
};
use log::{debug, error};
use std::collections::HashMap;

/// Represents a relation (connection) in a component layout with positional information.
///
/// LayoutRelation wraps an AST relation with additional layout-specific data,
/// including the indices of the source and target components within the layout.
/// This allows the layout system to efficiently reference components when
/// positioning and rendering relations.
#[derive(Debug, Clone)]
pub struct LayoutRelation<'a> {
    relation: &'a ast::Relation,
    source_index: usize,
    target_index: usize,
}

impl<'a> LayoutRelation<'a> {
    /// Creates a new LayoutRelation with the given relation and component indices.
    ///
    /// # Arguments
    /// * `relation` - Reference to the AST relation being laid out
    /// * `source_index` - Index of the source component in the layout
    /// * `target_index` - Index of the target component in the layout
    pub fn new(relation: &'a ast::Relation, source_index: usize, target_index: usize) -> Self {
        Self {
            relation,
            source_index,
            target_index,
        }
    }

    /// Returns a reference to the underlying AST relation.
    ///
    /// This provides access to the relation's properties such as type,
    /// attributes, and labels for rendering purposes.
    pub fn relation(&self) -> &ast::Relation {
        self.relation
    }
}

#[derive(Debug, Clone)]
pub struct Layout<'a> {
    pub components: Vec<Component<'a>>,
    pub relations: Vec<LayoutRelation<'a>>,
}

impl<'a> Layout<'a> {
    pub fn source(&self, lr: &LayoutRelation<'a>) -> &Component<'a> {
        &self.components[lr.source_index]
    }

    pub fn target(&self, lr: &LayoutRelation<'a>) -> &Component<'a> {
        &self.components[lr.target_index]
    }
}

impl<'a> LayoutSizing for Layout<'a> {
    fn layout_size(&self) -> Size {
        // For component layouts, get the bounding box of all components
        if self.components.is_empty() {
            return Size::default();
        }

        // Calculate bounds from all components
        let bounds = self
            .components
            .iter()
            .skip(1)
            .fold(self.components[0].bounds(), |acc, comp| {
                acc.merge(&comp.bounds())
            });

        bounds.to_size()
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

            // Find the component in the source layer that matches the node
            let source_component = source
                .content()
                .components
                .iter()
                .find(|component| component.node.id == node.id)
                .expect("Component must exist in source layer");
            let target_offset = source
                .offset()
                .add(source_component.bounds().min_point())
                .add(source_component.shape.shape_to_container_min_point()); // TODO: This does not account for text.
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
