use crate::geometry::{Insets, Point, Size};

const BLOW_SHAPE_TEXT_GAP: f32 = 8.0;

/// Enum defining different text positioning strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
                // Position text below shape with gap, centered vertically within text area
                let text_y = total_position.y() + (total_size.height() - text_size.height()) / 2.0;

                total_position.with_y(text_y)
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

                let text_with_gap =
                    text_size.add_padding(Insets::new(BLOW_SHAPE_TEXT_GAP, 0.0, 0.0, 0.0));
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
                let shape_y =
                    total_position.y() - (total_size.height() - shape_size.height()) / 2.0;
                total_position.with_y(shape_y)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_content_total_size() {
        let strategy = TextPositioningStrategy::InContent;
        let shape_size = Size::new(100.0, 50.0);
        let text_size = Size::new(30.0, 10.0);

        let total_size = strategy.calculate_total_size(shape_size, text_size);

        // For InContent, the total size is just the shape size (text is inside)
        assert_eq!(total_size.width(), shape_size.width());
        assert_eq!(total_size.height(), shape_size.height());
    }

    #[test]
    fn test_below_shape_total_size() {
        let strategy = TextPositioningStrategy::BelowShape;
        let shape_size = Size::new(50.0, 40.0);
        let text_size = Size::new(30.0, 10.0);

        let total_size = strategy.calculate_total_size(shape_size, text_size);

        // Width should be max of shape and text widths (shaph is wider)
        assert_eq!(total_size.width(), 50.0);
        // Height = shape (40) + gap (8) + text (10) = 58
        assert_eq!(total_size.height(), 58.0);
    }

    #[test]
    fn test_below_shape_total_size_wide_text() {
        let strategy = TextPositioningStrategy::BelowShape;
        let shape_size = Size::new(30.0, 40.0);
        let text_size = Size::new(80.0, 10.0);

        let total_size = strategy.calculate_total_size(shape_size, text_size);

        // Width should be max of shape and text widths (text is wider)
        assert_eq!(total_size.width(), 80.0);
        // Height = shape (40) + gap (8) + text (10) = 58
        assert_eq!(total_size.height(), 58.0);
    }

    #[test]
    fn test_below_shape_total_size_zero_text() {
        let strategy = TextPositioningStrategy::BelowShape;
        let shape_size = Size::new(50.0, 40.0);
        let text_size = Size::new(0.0, 0.0);

        let total_size = strategy.calculate_total_size(shape_size, text_size);

        // When text is zero, should return shape size without gap
        assert_eq!(total_size.width(), shape_size.width());
        assert_eq!(total_size.height(), shape_size.height());
    }

    #[test]
    fn test_text_affects_shape_content() {
        // InContent: text is inside shape, so it affects content size
        assert!(
            TextPositioningStrategy::InContent.text_affects_shape_content(),
            "InContent should return true - text affects shape content"
        );

        // BelowShape: text is outside shape, so it doesn't affect content size
        assert!(
            !TextPositioningStrategy::BelowShape.text_affects_shape_content(),
            "BelowShape should return false - text doesn't affect shape content"
        );
    }

    #[test]
    fn test_calculate_shape_position_in_content() {
        let strategy = TextPositioningStrategy::InContent;
        let total_position = Point::new(100.0, 100.0);
        let shape_size = Size::new(80.0, 60.0);
        let text_size = Size::new(40.0, 12.0);

        let shape_position =
            strategy.calculate_shape_position(total_position, shape_size, text_size);

        assert_eq!(shape_position, total_position);
    }

    #[test]
    fn test_calculate_shape_position_below_shape() {
        let strategy = TextPositioningStrategy::BelowShape;
        let total_position = Point::new(100.0, 100.0);
        let shape_size = Size::new(50.0, 40.0);
        let text_size = Size::new(30.0, 10.0);

        let shape_position =
            strategy.calculate_shape_position(total_position, shape_size, text_size);

        // Y should be higher (smaller value) since text is below
        // Total height = 40 + 8 + 10 = 58
        // Shape offset = (58 - 40) / 2 = 9
        // Shape Y = 100 - 9 = 91
        assert_eq!(shape_position, Point::new(100.0, 91.0));
    }

    #[test]
    fn test_calculate_inner_content_min_point() {
        let base_point = Point::new(10.0, 20.0);
        let text_size = Size::new(50.0, 15.0);

        // InContent: inner content starts below the text (20 + 15 = 35)
        let in_content_result = TextPositioningStrategy::InContent
            .calculate_inner_content_min_point(base_point, text_size);
        assert_eq!(in_content_result, Point::new(10.0, 35.0));

        // BelowShape: inner content starts at base point (text is outside shape)
        let below_shape_result = TextPositioningStrategy::BelowShape
            .calculate_inner_content_min_point(base_point, text_size);
        assert_eq!(below_shape_result, base_point);
    }
}
