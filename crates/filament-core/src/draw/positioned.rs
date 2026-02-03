//! Provides `PositionedDrawable`, a wrapper for a Drawable and its absolute position.

use crate::{
    draw::{Drawable, LayeredOutput},
    geometry::{Bounds, Point, Size},
};

/// A drawable object together with an absolute position.
///
/// Calls `render_to_svg` on the wrapped drawable, passing in the stored position.
#[derive(Debug, Clone)]
pub struct PositionedDrawable<D: Drawable> {
    drawable: D,
    position: Point,
}

impl<D: Drawable> PositionedDrawable<D> {
    /// Construct a new `PositionedDrawable` from a drawable (position defaults to zero).
    pub fn new(drawable: D) -> Self {
        Self {
            drawable,
            position: Point::default(),
        }
    }

    /// Set the position for this drawable (builder style).
    pub fn with_position(mut self, position: Point) -> Self {
        self.position = position;
        self
    }

    /// Render this positioned drawable to layers, using the inner drawable's implementation.
    pub fn render_to_layers(&self) -> LayeredOutput {
        self.drawable.render_to_layers(self.position)
    }

    /// Calculate the bounds of this positioned drawable.
    pub fn bounds(&self) -> Bounds {
        self.position.to_bounds(self.drawable.size())
    }

    /// Get a reference to the inner drawable
    pub fn inner(&self) -> &D {
        &self.drawable
    }

    /// Get the position of this drawable
    pub fn position(&self) -> Point {
        self.position
    }
}

impl<'a> PositionedDrawable<crate::draw::ShapeWithText<'a>> {
    /// Calculate the bounds of the content area.
    /// Returns None if no inner content size was set on the ShapeWithText.
    pub fn content_bounds(&self) -> Option<Bounds> {
        let content_size = self.drawable.content_size()?;
        let outer_bounds = self.bounds();
        let content_min_point = outer_bounds
            .min_point()
            .add_point(self.drawable.shape_to_inner_content_min_point());
        Some(Bounds::new_from_top_left(content_min_point, content_size))
    }
}

impl<D: Drawable> Drawable for PositionedDrawable<D> {
    fn render_to_layers(&self, _position: Point) -> LayeredOutput {
        // Ignore the passed position and use our stored position
        self.render_to_layers()
    }

    fn size(&self) -> Size {
        self.drawable.size()
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use float_cmp::assert_approx_eq;

    use super::*;
    use crate::draw::{
        Shape, ShapeWithText, Text, TextDefinition,
        shape::{RectangleDefinition, ShapeDefinition},
    };
    use crate::geometry::Size;

    fn create_rectangle_shape() -> Shape {
        let rect_def: Rc<Box<dyn ShapeDefinition>> = Rc::new(Box::new(RectangleDefinition::new()));
        Shape::new(rect_def)
    }

    #[test]
    fn test_positioned_drawable_new_default_position() {
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Hello");

        let positioned = PositionedDrawable::new(text);

        assert_approx_eq!(f32, positioned.position().x(), 0.0);
        assert_approx_eq!(f32, positioned.position().y(), 0.0);
    }

    #[test]
    fn test_positioned_drawable_with_position() {
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Hello");

        let positioned = PositionedDrawable::new(text).with_position(Point::new(100.0, 50.0));

        assert_approx_eq!(f32, positioned.position().x(), 100.0);
        assert_approx_eq!(f32, positioned.position().y(), 50.0);
    }

    #[test]
    fn test_positioned_drawable_inner_reference() {
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "TestContent");

        let positioned = PositionedDrawable::new(text);

        let inner = positioned.inner();
        assert_eq!(inner.content(), "TestContent");
    }

    #[test]
    fn test_positioned_drawable_size_delegates() {
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "SizeTest");
        let expected_size = text.size();

        let positioned = PositionedDrawable::new(text);

        assert_eq!(positioned.size(), expected_size);
    }

    #[test]
    fn test_positioned_drawable_bounds() {
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "BoundsTest");
        let size = text.size();

        let position = Point::new(100.0, 50.0);
        let positioned = PositionedDrawable::new(text).with_position(position);

        let bounds = positioned.bounds();

        let half_width = size.width() / 2.0;
        let half_height = size.height() / 2.0;

        assert_approx_eq!(f32, bounds.min_x(), position.x() - half_width);
        assert_approx_eq!(f32, bounds.min_y(), position.y() - half_height);
        assert_approx_eq!(f32, bounds.max_x(), position.x() + half_width);
        assert_approx_eq!(f32, bounds.max_y(), position.y() + half_height);
    }

    #[test]
    fn test_positioned_drawable_render_to_layers() {
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "RenderTest");

        let positioned = PositionedDrawable::new(text).with_position(Point::new(50.0, 50.0));

        let output = positioned.render_to_layers();

        assert!(!output.is_empty());
    }

    #[test]
    fn test_positioned_drawable_trait_ignores_position() {
        let text_def = TextDefinition::default();
        let text1 = Text::new(&text_def, "TraitTest");
        let text2 = Text::new(&text_def, "TraitTest");

        let positioned1 = PositionedDrawable::new(text1).with_position(Point::new(100.0, 100.0));
        let positioned2 = PositionedDrawable::new(text2).with_position(Point::new(100.0, 100.0));

        // Call the trait method
        let output_via_trait: LayeredOutput =
            Drawable::render_to_layers(&positioned1, Point::new(999.0, 999.0));

        // Call the inherent method directly
        let output_direct = positioned2.render_to_layers();

        let svg_via_trait: String = output_via_trait
            .render()
            .iter()
            .map(|n| n.to_string())
            .collect();
        let svg_direct: String = output_direct
            .render()
            .iter()
            .map(|n| n.to_string())
            .collect();

        assert_eq!(svg_via_trait, svg_direct);
    }

    #[test]
    fn test_positioned_shape_with_text_content_bounds() {
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Header");

        let shape = create_rectangle_shape();
        let mut shape_with_text = ShapeWithText::new(shape, Some(text));

        let inner_content_size = Size::new(200.0, 100.0);
        shape_with_text
            .set_inner_content_size(inner_content_size)
            .expect("Rectangle supports content");

        let position = Point::new(150.0, 100.0);
        let positioned = PositionedDrawable::new(shape_with_text).with_position(position);

        let outer_bounds = positioned.bounds();
        let content_bounds = positioned
            .content_bounds()
            .expect("content_bounds should return Some when inner content size is set");

        // Content bounds should have exact size we set
        assert_approx_eq!(f32, content_bounds.width(), inner_content_size.width());
        assert_approx_eq!(f32, content_bounds.height(), inner_content_size.height());

        // Content bounds should be vertically offset from outer bounds (text header takes space)
        assert!(
            content_bounds.min_y() > outer_bounds.min_y(),
            "Content should be offset from top edge due to text header"
        );

        // Content bounds should be fully contained within outer bounds
        assert!(
            content_bounds.min_x() >= outer_bounds.min_x(),
            "Content should not extend past left edge"
        );
        assert!(
            content_bounds.max_x() <= outer_bounds.max_x(),
            "Content should not extend past right edge"
        );
        assert!(
            content_bounds.max_y() <= outer_bounds.max_y(),
            "Content should not extend past bottom edge"
        );
    }

    #[test]
    fn test_positioned_shape_with_text_content_bounds_none() {
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Header");

        let shape = create_rectangle_shape();
        // Do NOT set inner content size
        let shape_with_text = ShapeWithText::new(shape, Some(text));

        let position = Point::new(150.0, 100.0);
        let positioned = PositionedDrawable::new(shape_with_text).with_position(position);

        let content_bounds = positioned.content_bounds();
        assert!(
            content_bounds.is_none(),
            "content_bounds should return None when inner content size is not set"
        );
    }
}
