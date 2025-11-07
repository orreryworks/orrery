//! Provides a container to group multiple `Drawable` objects with relative positioning.

use svg::{self, node::element as svg_element};

use crate::{
    draw::Drawable,
    geometry::{Point, Size},
};

/// A group of drawable objects that can be rendered together as an SVG group.
///
/// The Group holds references to drawable objects and their relative positions.
/// When rendered, it produces an SVG `<g>` with each child positioned at its own position
/// relative to the group's origin.
#[derive(Debug)]
pub struct Group<'a> {
    items: Vec<GroupItem<'a>>,
}

impl<'a> Group<'a> {
    /// Creates a new, empty group.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Adds a child drawable at a given relative position.
    pub fn add<D: Drawable>(&mut self, drawable: &'a D, position: Point) {
        self.items.push(GroupItem::new(drawable, position));
    }

    pub fn render(&self) -> Box<dyn svg::Node> {
        let mut group = svg_element::Group::new();
        for item in &self.items {
            group = group.add(item.drawable.render_to_svg(item.position));
        }
        Box::new(group)
    }
}

impl<'a> Drawable for Group<'a> {
    /// Render the group as an SVG `<g>`, with each child positioned at its relative position.
    fn render_to_svg(&self, _group_position: Point) -> Box<dyn svg::Node> {
        self.render()
    }

    fn size(&self) -> Size {
        // Calculate the bounding box of all items in the group
        if self.items.is_empty() {
            return Size::default();
        }

        // Start with the first item's bounds
        let first_item = &self.items[0];
        let first_size = first_item.drawable.size();
        let mut combined_bounds = first_item.position.to_bounds(first_size);

        // Merge bounds for all remaining items
        for item in &self.items[1..] {
            let item_size = item.drawable.size();
            let item_bounds = item.position.to_bounds(item_size);
            combined_bounds = combined_bounds.merge(&item_bounds);
        }

        combined_bounds.to_size()
    }
}

/// Private item in a Group: wraps a child Drawable and its position.
#[derive(Debug)]
struct GroupItem<'a> {
    drawable: &'a dyn Drawable,
    position: Point,
}

impl<'a> GroupItem<'a> {
    fn new(drawable: &'a dyn Drawable, position: Point) -> Self {
        Self { drawable, position }
    }
}
