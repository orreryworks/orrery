use crate::{
    ast,
    layout::common::{Component, LayoutSizing, Size},
};

#[derive(Debug, Clone)]
pub struct LayoutRelation<'a> {
    pub relation: &'a ast::Relation,
    pub(crate) source_index: usize,
    pub(crate) target_index: usize,
}

impl<'a> LayoutRelation<'a> {
    /// Creates a new LayoutRelation
    pub fn new(relation: &'a ast::Relation, source_index: usize, target_index: usize) -> Self {
        Self {
            relation,
            source_index,
            target_index,
        }
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
