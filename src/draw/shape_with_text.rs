use crate::{
    draw::{Drawable, Shape, Text},
    geometry::{Point, Size},
};

use super::group::Group;

/// A drawable that combines a shape with optional header text.
///
/// This struct provides a way to render shapes (rectangles, ovals, etc.) with optional
/// text positioned at the top. The text is automatically factored into the overall size
/// calculations and positioning.
#[derive(Debug, Clone)]
pub struct ShapeWithText {
    shape: Shape,
    text: Option<Text>,
}

impl ShapeWithText {
    /// Creates a new ShapeWithText with the given shape and optional text.
    ///
    /// If text is provided, the shape's content size is automatically updated
    /// to accommodate the text dimensions.
    pub fn new(shape: Shape, text: Option<Text>) -> Self {
        let mut instance = Self { shape, text };
        if instance.text.is_some() {
            instance.update_shape_content_size();
        }
        instance
    }

    /// Sets the inner content size, accounting for both text and additional content.
    pub fn set_inner_content_size(&mut self, size: Size) {
        let text_size = self.text_size();
        let total = Size::new(
            size.width().max(text_size.width()),
            text_size.height() + size.height(),
        );
        self.shape.expand_content_size_to(total);
    }

    /// Returns the size of the text component, or zero size if no text is present.
    pub fn text_size(&self) -> Size {
        self.text.as_ref().map(|t| t.size()).unwrap_or_default()
    }

    /// Returns the total size of the underlying shape.
    pub fn shape_size(&self) -> Size {
        self.shape.shape_size()
    }

    /// Returns the minimum point where inner content (excluding text) can be placed.
    pub fn shape_to_inner_content_min_point(&self) -> Point {
        let base = self.shape.shape_to_container_min_point();
        Point::new(base.x(), base.y() + self.text_size().height())
    }

    /// Finds the intersection point of a line (from point a to point b) with the shape boundary.
    pub fn find_intersection(&self, a: Point, b: Point) -> Point {
        self.shape.find_intersection(a, b)
    }

    /// Updates the shape's content size to accommodate the text dimensions.
    fn update_shape_content_size(&mut self) {
        let text_size = self.text_size();
        self.shape.expand_content_size_to(text_size);
    }

    /// Calculates the position where text should be rendered relative to the shape.
    ///
    /// The text is positioned at the top of the shape, centered horizontally
    /// and vertically within the text area.
    fn calculate_text_position(&self, shape_position: Point) -> Point {
        if self.text.is_none() {
            return Point::default();
        }

        let bounds = shape_position.to_bounds(self.shape_size());
        let offset = self.shape.shape_to_container_min_point();
        let text_size = self.text_size();

        Point::new(
            shape_position.x(),
            bounds.min_y() + offset.y() + text_size.height() / 2.0,
        )
    }
}

impl Drawable for ShapeWithText {
    fn render_to_svg(&self, position: Point) -> Box<dyn svg::Node> {
        let mut group = Group::new();

        group.add(&self.shape, position);

        if let Some(text) = &self.text {
            let text_pos = self.calculate_text_position(position);
            group.add(text, text_pos);
        }

        group.render()
    }

    fn size(&self) -> Size {
        self.shape_size()
    }
}
