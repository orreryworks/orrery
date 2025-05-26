use crate::ast;
use crate::layout::common::Component;

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
