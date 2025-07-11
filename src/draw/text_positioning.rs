use crate::geometry::{Insets, Point, Size};

/// Enum defining different text positioning strategies
#[derive(Debug, Clone, Copy)]
pub enum TextPositioningStrategy {
    /// Text is rendered inside/on top of the shape within its content area
    /// Used for content-supporting shapes
    InContent,
    /// Text is rendered below the shape with a gap
    /// Used for content-free shapes
    BelowShape,
}

impl TextPositioningStrategy {
    /// Calculate the position where text should be rendered relative to the shape position
    pub fn calculate_text_position(
        &self,
        total_position: Point,
        shape_size: Size,
        text_size: Size,
        shape_to_container_offset: Point,
        shape_to_container_offset_no_top_padding: Point,
        has_inner_content: bool,
    ) -> Point {
        let total_size = self.calculate_total_size(shape_size, text_size);

        match self {
            Self::InContent => {
                let bounds = total_position.to_bounds(total_size);

                let content_offset = if has_inner_content {
                    // With inner content, position text at the very top (no top padding)
                    shape_to_container_offset_no_top_padding
                } else {
                    // Without inner content, respect top padding to separate text from shape edge
                    shape_to_container_offset
                };

                total_position
                    .with_y(bounds.min_y() + content_offset.y() + text_size.height() / 2.0)
            }
            Self::BelowShape => {
                // Center the text horizontally within the total width
                let text_x = total_position.x() + (total_size.width() - text_size.width()) / 2.0;
                // Position text below shape with gap, centered vertically within text area
                let text_y =
                    total_position.y() + total_size.height() + 8.0 + text_size.height() / 2.0;

                Point::new(text_x, text_y)
            }
        }
    }

    /// Calculate the total size needed to contain both shape and text
    pub fn calculate_total_size(&self, shape_size: Size, text_size: Size) -> Size {
        match self {
            Self::InContent => {
                // For content-supporting shapes, text is inside, so shape size is the total size
                shape_size
            }
            Self::BelowShape => {
                if text_size.is_zero() {
                    return shape_size;
                }

                let text_with_gap = text_size.add_padding(Insets::new(8.0, 0.0, 0.0, 0.0));
                shape_size.merge_vertical(text_with_gap)
            }
        }
    }

    /// Determine if the text should be included in the shape's content size calculation
    pub fn text_affects_shape_content(&self) -> bool {
        match self {
            Self::InContent => true,
            Self::BelowShape => false,
        }
    }

    /// Calculate the minimum point where inner content (excluding text) can be placed
    pub fn calculate_inner_content_min_point(&self, base_point: Point, text_size: Size) -> Point {
        match self {
            Self::InContent => base_point.with_y(base_point.y() + text_size.height()),
            Self::BelowShape => base_point,
        }
    }

    /// Determine which shape position to use when rendering the shape within the total area
    pub fn calculate_shape_position(
        &self,
        total_position: Point,
        shape_size: Size,
        text_size: Size,
    ) -> Point {
        match self {
            Self::InContent => {
                // Content-supporting shapes: render shape at the given position
                total_position
            }
            Self::BelowShape => {
                // Content-free shapes: center the shape within the total area
                let total_size = self.calculate_total_size(shape_size, text_size);
                let shape_x = total_position.x() + (total_size.width() - shape_size.width()) / 2.0;
                total_position.with_x(shape_x)
            }
        }
    }
}
