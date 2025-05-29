use crate::{
    ast,
    layout::common::{Component, LayoutSizing, Size},
};

#[derive(Debug, Clone)]
pub struct Participant<'a> {
    pub component: Component<'a>,
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
            max_y - bounds.min_y, // Height from top to bottom lifeline
        )
    }
}
