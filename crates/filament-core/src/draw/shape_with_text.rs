//! Shape-with-text composite drawable.
//!
//! This module provides [`ShapeWithText`], which combines a [`Shape`] with an
//! optional header [`Text`] label, handling size calculation and positioning.

use crate::{
    draw::{Drawable, LayeredOutput, Shape, Text, text_positioning::TextPositioningStrategy},
    geometry::{Point, Size},
};

/// A drawable that combines a shape with optional header text.
///
/// This struct provides a way to render shapes (rectangles, ovals, etc.) with optional
/// text positioned at the top. The text is automatically factored into the overall size
/// calculations and positioning.
#[derive(Debug, Clone)]
pub struct ShapeWithText<'a> {
    shape: Shape,
    text: Option<Text<'a>>,
    text_positioning_strategy: TextPositioningStrategy,
    // Stores the pure embedded content size (without text) when set via set_inner_content_size
    inner_content_size: Option<Size>,
}

impl<'a> ShapeWithText<'a> {
    /// Creates a new ShapeWithText with the given shape and optional text.
    ///
    /// If text is provided, the shape's content size is automatically updated
    /// to accommodate the text dimensions.
    pub fn new(shape: Shape, text: Option<Text<'a>>) -> Self {
        let text_positioning_strategy = shape.text_positioning_strategy();
        let mut instance = Self {
            shape,
            text,
            text_positioning_strategy,
            inner_content_size: None,
        };
        if instance.text.is_some()
            && instance
                .text_positioning_strategy
                .text_affects_shape_content()
        {
            // For content-supporting shapes, expand the shape to fit the text
            if let Err(e) = instance.update_shape_content_size() {
                panic!("Failed to assign text to a content-supporting shape: {e}");
            }
        }
        instance
    }

    /// Sets the inner content size, accounting for both text and additional content.
    pub fn set_inner_content_size(&mut self, size: Size) -> Result<(), &'static str> {
        // Content-free shapes cannot contain content
        if !self.shape.supports_content() {
            return Err("Cannot set inner content size on content-free shapes");
        }

        // Store the pure embedded content size
        self.inner_content_size = Some(size);

        let text_size = self.text_size();
        let total = Size::new(
            size.width().max(text_size.width()),
            text_size.height() + size.height(),
        );
        self.shape
            .expand_content_size_to(total)
            .expect("Shape should support content at this point");

        if !size.is_zero() {
            // Adjust shape padding to account for text height
            let current_padding = self.shape.padding();
            let adjusted_top = (current_padding.top() - text_size.height()).max(0.0);
            let new_padding = current_padding.with_top(adjusted_top);
            self.shape.set_padding(new_padding);
        }

        Ok(())
    }

    /// Returns the size of the text component, or zero size if no text is present.
    pub fn text_size(&self) -> Size {
        self.text.as_ref().map(|t| t.size()).unwrap_or_default()
    }

    /// Returns the minimum point where inner content (excluding text) can be placed.
    pub fn shape_to_inner_content_min_point(&self) -> Point {
        let base = self.shape.shape_to_container_min_point();
        let text_size = self.text_size();
        self.text_positioning_strategy
            .calculate_inner_content_min_point(base, text_size)
    }

    /// Returns the size of the inner content area where inner content should be placed.
    /// Returns None if no inner content size was set via set_inner_content_size.
    pub fn content_size(&self) -> Option<Size> {
        self.inner_content_size
    }

    /// Finds the intersection point of a line (from point a to point b) with the shape boundary.
    pub fn find_intersection(&self, a: Point, b: Point) -> Point {
        self.shape.find_intersection(a, b, self.size())
    }

    /// Updates the shape's content size to accommodate the text dimensions.
    /// Only works for content-supporting shapes.
    fn update_shape_content_size(&mut self) -> Result<(), &'static str> {
        let text_size = self.text_size();
        self.shape.expand_content_size_to(text_size)
    }

    /// Returns a Point representing the (x, y) offset from the shape's top-left corner
    /// to where content should be positioned, but without including top padding.
    /// This is useful for positioning text at the very top of the content area.
    fn shape_to_container_min_point_no_top_padding(&self) -> Point {
        let additional_space = self.shape.calculate_additional_space();
        let padding = self.shape.padding();

        Point::new(
            padding.left() + additional_space.width() / 2.0,
            additional_space.height() / 2.0,
        )
    }

    /// Calculates the position where text should be rendered relative to the shape.
    fn calculate_text_position(&self, total_position: Point) -> Point {
        if self.text.is_none() {
            return Point::default();
        }

        let shape_size = self.shape.inner_size();
        let text_size = self.text_size();
        let has_inner_content = text_size != self.shape.content_size();

        self.text_positioning_strategy.calculate_text_position(
            total_position,
            shape_size,
            text_size,
            self.shape.shape_to_container_min_point(),
            self.shape_to_container_min_point_no_top_padding(),
            has_inner_content,
        )
    }
}

impl<'a> Drawable for ShapeWithText<'a> {
    fn render_to_layers(&self, position: Point) -> LayeredOutput {
        let mut output = LayeredOutput::new();

        let shape_size = self.shape.inner_size();
        let text_size = self.text_size();
        let shape_position = self
            .text_positioning_strategy
            .calculate_shape_position(position, shape_size, text_size);

        let shape_output = self.shape.render_to_layers(shape_position);
        output.merge(shape_output);

        if let Some(text) = &self.text {
            let text_pos = self.calculate_text_position(position);
            let text_output = text.render_to_layers(text_pos);
            output.merge(text_output);
        }

        output
    }

    /// Returns the total size of the underlying shape.
    /// For content-free shapes with text, this includes the text below the shape.
    fn size(&self) -> Size {
        let shape_size = self.shape.outer_size();
        let text_size = self.text_size();
        self.text_positioning_strategy
            .calculate_total_size(shape_size, text_size)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::draw::{
        TextDefinition,
        shape::{ActorDefinition, RectangleDefinition, ShapeDefinition},
    };

    /// Helper function to create a Rectangle shape (content-supporting)
    fn create_rectangle_shape() -> Shape {
        let rect_def: Rc<Box<dyn ShapeDefinition>> = Rc::new(Box::new(RectangleDefinition::new()));
        Shape::new(rect_def)
    }

    /// Helper function to create an Actor shape (content-free)
    fn create_actor_shape() -> Shape {
        let actor_def: Rc<Box<dyn ShapeDefinition>> = Rc::new(Box::new(ActorDefinition::new()));
        Shape::new(actor_def)
    }

    #[test]
    fn test_shape_with_text_new_content_supporting() {
        // Rectangle is a content-supporting shape - text is inside
        let shape = create_rectangle_shape();
        let shape_only_outer_size = shape.outer_size();

        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Test Label");

        let shape_with_text = ShapeWithText::new(shape, Some(text));

        let total_size = shape_with_text.size();

        // Shape should expand to fit text inside
        assert!(
            total_size.width() >= shape_only_outer_size.width(),
            "Width should be at least shape width"
        );
        assert!(
            total_size.height() >= shape_only_outer_size.height(),
            "Height should be at least shape height"
        );
    }

    #[test]
    fn test_shape_with_text_new_content_free() {
        // Actor is a content-free shape - text is below
        let shape = create_actor_shape();
        let shape_only_outer_size = shape.outer_size();

        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "User");

        let shape_with_text = ShapeWithText::new(shape, Some(text));

        let total_size = shape_with_text.size();

        assert!(
            total_size.width() >= shape_only_outer_size.width(),
            "Width should be at least shape width"
        );
        assert!(
            total_size.height() > shape_only_outer_size.height(),
            "Height should be greater than shape-only height (text is below)"
        );
    }

    #[test]
    fn test_shape_with_text_new_no_text() {
        // Test content-supporting shape (Rectangle)
        let rect = create_rectangle_shape();
        let rect_outer_size = rect.outer_size();

        let rect_with_text = ShapeWithText::new(rect, None);

        assert_eq!(
            rect_with_text.text_size(),
            Size::default(),
            "text_size should be zero when no text (Rectangle)"
        );
        assert_eq!(
            rect_with_text.size(),
            rect_outer_size,
            "Total size should equal shape outer size when no text (Rectangle)"
        );
        assert!(
            rect_with_text.content_size().is_none(),
            "content_size should be None initially (Rectangle)"
        );

        // Test content-free shape (Actor)
        let actor = create_actor_shape();
        let actor_outer_size = actor.outer_size();

        let actor_with_text = ShapeWithText::new(actor, None);

        assert_eq!(
            actor_with_text.text_size(),
            Size::default(),
            "text_size should be zero when no text (Actor)"
        );
        assert_eq!(
            actor_with_text.size(),
            actor_outer_size,
            "Total size should equal shape outer size when no text (Actor)"
        );
        assert!(
            actor_with_text.content_size().is_none(),
            "content_size should be None initially (Actor)"
        );
    }

    #[test]
    fn test_shape_with_text_text_size() {
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Sample Text");
        let expected_text_size = text.size();

        // With text
        let shape = create_rectangle_shape();
        let shape_with_text = ShapeWithText::new(shape, Some(text));
        assert_eq!(
            shape_with_text.text_size(),
            expected_text_size,
            "text_size should return the text dimensions"
        );

        // Without text
        let shape = create_rectangle_shape();
        let shape_with_text = ShapeWithText::new(shape, None);
        assert_eq!(
            shape_with_text.text_size(),
            Size::default(),
            "text_size should return zero when no text"
        );
    }

    #[test]
    fn test_shape_with_text_size_with_longer_text() {
        // Content-supporting shape: longer text should expand the shape
        let shape = create_rectangle_shape();
        let shape_only_size = shape.outer_size();

        let text_def = TextDefinition::default();
        let long_text = Text::new(&text_def, "This is a much longer text label for testing");

        let shape_with_text = ShapeWithText::new(shape, Some(long_text));
        let total_size = shape_with_text.size();

        assert!(
            total_size.width() > shape_only_size.width(),
            "Longer text should expand shape width"
        );
    }

    #[test]
    fn test_shape_with_text_set_inner_content_size() {
        // Content-supporting shape should accept inner content
        let shape = create_rectangle_shape();
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Header");

        let mut shape_with_text = ShapeWithText::new(shape, Some(text));
        let size_before = shape_with_text.size();

        let inner_content = Size::new(200.0, 100.0);
        let result = shape_with_text.set_inner_content_size(inner_content);

        assert!(
            result.is_ok(),
            "set_inner_content_size should succeed for content-supporting shapes"
        );
        assert_eq!(
            shape_with_text.content_size(),
            Some(inner_content),
            "content_size should return the set inner content size"
        );

        // Total size should have grown to accommodate inner content
        let size_after = shape_with_text.size();
        assert!(
            size_after.width() > size_before.width(),
            "Width should accommodate inner content"
        );
        assert!(
            size_after.height() > size_before.height(),
            "Height should accommodate inner content"
        );
    }

    #[test]
    fn test_shape_with_text_set_inner_content_size_error() {
        // Content-free shape should reject inner content
        let shape = create_actor_shape();
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "User");

        let mut shape_with_text = ShapeWithText::new(shape, Some(text));

        let inner_content = Size::new(50.0, 50.0);
        let result = shape_with_text.set_inner_content_size(inner_content);

        assert!(
            result.is_err(),
            "set_inner_content_size should fail for content-free shapes"
        );
        assert!(
            shape_with_text.content_size().is_none(),
            "content_size should remain None after failed set"
        );
    }

    #[test]
    fn test_shape_with_text_shape_to_inner_content_min_point() {
        // With text - inner content should be offset by text height for content-supporting shapes
        let shape = create_rectangle_shape();
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Header");
        let text_size = text.size();

        let shape_with_text = ShapeWithText::new(shape, Some(text));
        let min_point_with_text = shape_with_text.shape_to_inner_content_min_point();

        // Without text
        let shape = create_rectangle_shape();
        let shape_with_text_no_text = ShapeWithText::new(shape, None);
        let min_point_no_text = shape_with_text_no_text.shape_to_inner_content_min_point();

        // With text, the y offset should be greater (text takes space above content)
        assert!(
            min_point_with_text.y() >= min_point_no_text.y() + text_size.height(),
            "Inner content min point should account for text height"
        );
    }

    #[test]
    fn test_shape_with_text_render_to_layers() {
        // Content-supporting shape with text
        let shape = create_rectangle_shape();
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Label");

        let shape_with_text = ShapeWithText::new(shape, Some(text));
        let output = shape_with_text.render_to_layers(Point::new(100.0, 100.0));

        assert!(!output.is_empty(), "Render output should not be empty");

        // Content-free shape with text
        let actor = create_actor_shape();
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "User");

        let actor_with_text = ShapeWithText::new(actor, Some(text));
        let output = actor_with_text.render_to_layers(Point::new(100.0, 100.0));

        assert!(
            !output.is_empty(),
            "Render output for content-free shape should not be empty"
        );

        // Without text
        let shape = create_rectangle_shape();
        let shape_with_text = ShapeWithText::new(shape, None);
        let output = shape_with_text.render_to_layers(Point::new(100.0, 100.0));

        assert!(
            !output.is_empty(),
            "Render output without text should not be empty"
        );
    }

    #[test]
    fn test_shape_with_text_find_intersection() {
        let shape = create_rectangle_shape();
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Label");

        let shape_with_text = ShapeWithText::new(shape, Some(text));
        let total_size = shape_with_text.size();

        let center = Point::new(100.0, 100.0);
        let target = Point::new(200.0, 100.0);

        let shape_with_text_intersection = shape_with_text.find_intersection(center, target);
        let shape_intersection = shape_with_text
            .shape
            .find_intersection(center, target, total_size);

        // ShapeWithText should delegate to underlying shape
        assert_eq!(
            shape_with_text_intersection, shape_intersection,
            "find_intersection should delegate to underlying shape"
        );
    }
}
