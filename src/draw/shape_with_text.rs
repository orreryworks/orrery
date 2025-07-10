use crate::{
    draw::{Drawable, Shape, Text, group::Group},
    geometry::{Insets, Point, Size},
};

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
            if instance.shape.supports_content() {
                // For content-supporting shapes, expand the shape to fit the text
                if let Err(e) = instance.update_shape_content_size() {
                    panic!("Failed to assign text to a content-supporting shape: {}", e);
                }
            }
            // For content-free shapes, we don't modify the shape size - text goes below
        }
        instance
    }

    /// Sets the inner content size, accounting for both text and additional content.
    pub fn set_inner_content_size(&mut self, size: Size) -> Result<(), &'static str> {
        // Content-free shapes cannot contain content
        if !self.shape.supports_content() {
            return Err("Cannot set inner content size on content-free shapes");
        }

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

    /// Returns the total size of the underlying shape.
    /// For content-free shapes with text, this includes the text below the shape.
    pub fn shape_size(&self) -> Size {
        if self.shape.supports_content() {
            // Content-supporting shapes: text is inside, so shape size is the total size
            self.shape.shape_size()
        } else {
            // Content-free shapes: text is below, so we need to add text height and gap
            self.calculate_total_size_with_text_below()
        }
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
    /// Only works for content-supporting shapes.
    fn update_shape_content_size(&mut self) -> Result<(), &'static str> {
        let text_size = self.text_size();
        self.shape.expand_content_size_to(text_size)
    }

    /// Calculates the total size when text is positioned below the shape.
    /// Used for content-free shapes.
    fn calculate_total_size_with_text_below(&self) -> Size {
        let shape_size = self.shape.shape_size();
        let text_size = self.text_size();

        if text_size.is_zero() {
            return shape_size;
        }

        let text_gap = 8.0; // Gap between shape and text below
        shape_size.merge_vertical(text_size.add_padding(Insets::new(text_gap, 0.0, 0.0, 0.0)))
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
    ///
    /// For content-supporting shapes, text is positioned at the top of the shape's inner content area.
    /// For content-free shapes, text is positioned below the shape with a gap.
    fn calculate_text_position(&self, shape_position: Point) -> Point {
        if self.text.is_none() {
            return Point::default();
        }

        let shape_size = self.shape.shape_size();
        let text_size = self.text_size();

        if self.shape.supports_content() {
            // Content-supporting shapes: position text inside/on top
            let bounds = shape_position.to_bounds(shape_size);
            let has_inner_content = text_size != self.shape.content_size();

            let content_offset = if has_inner_content {
                // With inner content, position text at the very top (no top padding)
                self.shape_to_container_min_point_no_top_padding()
            } else {
                // Without inner content, respect top padding to separate text from shape edge
                self.shape.shape_to_container_min_point()
            };

            shape_position.with_y(bounds.min_y() + content_offset.y() + text_size.height() / 2.0)
        } else {
            // Content-free shapes: position text below the shape
            let total_size = self.calculate_total_size_with_text_below();
            let text_gap = 8.0; // Gap between shape and text

            // Center the text horizontally within the total width
            let text_x = shape_position.x() + (total_size.width() - text_size.width()) / 2.0;
            // Position text below shape with gap, centered vertically within text area
            let text_y =
                shape_position.y() + shape_size.height() + text_gap + text_size.height() / 2.0;

            Point::new(text_x, text_y)
        }
    }
}

impl Drawable for ShapeWithText {
    fn render_to_svg(&self, position: Point) -> Box<dyn svg::Node> {
        let mut group = Group::new();

        if self.shape.supports_content() {
            // Content-supporting shapes: render shape at the given position
            group.add(&self.shape, position);
        } else {
            // Content-free shapes: center the shape within the total area
            let total_size = self.calculate_total_size_with_text_below();
            let shape_size = self.shape.shape_size();
            let shape_x = position.x() + (total_size.width() - shape_size.width()) / 2.0;
            let shape_pos = position.with_x(shape_x);
            group.add(&self.shape, shape_pos);
        }

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
