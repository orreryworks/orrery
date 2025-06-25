//! Provides a container to group multiple `Drawable` objects with relative positioning.

use crate::draw::Drawable;
use crate::geometry::Point;
use svg::{self, node::element as svg_element};

/// A drawable group for holding multiple children with relative offsets.
///
/// When rendered, it produces an SVG `<g>` with each child positioned at its own offset
/// relative to the group's origin.
#[derive(Default)]
pub struct Group {
    items: Vec<GroupItem>,
}

impl Group {
    /// Creates a new, empty group.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Adds a child drawable at a given relative offset.
    pub fn add<D: Drawable + 'static>(&mut self, drawable: D, offset: Point) {
        self.items.push(GroupItem::new(Box::new(drawable), offset));
    }

    /// Returns the number of children.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns true if empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Drawable for Group {
    /// Render the group as an SVG `<g>`, with each child positioned at its relative offset.
    fn render_to_svg(&self, _group_position: Point) -> Box<dyn svg::Node> {
        let mut group = svg_element::Group::new();
        for item in &self.items {
            // let pos = group_position.add(item.offset);
            group = group.add(item.drawable.render_to_svg(item.offset));
        }
        Box::new(group)
    }
}

/// Private item in a Group: wraps a child Drawable and its offset.
struct GroupItem {
    drawable: Box<dyn Drawable>,
    offset: Point,
}

impl GroupItem {
    fn new(drawable: Box<dyn Drawable>, offset: Point) -> Self {
        Self { drawable, offset }
    }
}
