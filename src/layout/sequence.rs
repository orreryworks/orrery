use crate::ast;
use crate::layout::common::Component;

#[derive(Debug)]
pub struct Participant<'a> {
    pub component: Component<'a>,
    pub lifeline_end: f32, // y-coordinate where lifeline ends
}

#[derive(Debug)]
pub struct Message<'a> {
    pub relation: &'a ast::Relation,
    pub source_index: usize,
    pub target_index: usize,
    pub y_position: f32,
}

#[derive(Debug)]
pub struct Layout<'a> {
    pub participants: Vec<Participant<'a>>,
    pub messages: Vec<Message<'a>>,
}
