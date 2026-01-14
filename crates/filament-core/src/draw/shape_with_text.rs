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
