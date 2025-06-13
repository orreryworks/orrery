use crate::{
    ast,
    layout::geometry::{Component, LayoutSizing, Size},
};

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
