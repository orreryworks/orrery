//! Shape definitions and rendering traits.
//!
//! This module provides the [`ShapeDefinition`] trait and [`Shape`] wrapper
//! for rendering diagram node shapes (rectangles, ovals, actors, etc.).

use std::rc::Rc;

use crate::{
    color::Color,
    draw::{
        Drawable, LayeredOutput, RenderLayer, StrokeDefinition, TextDefinition,
        text_positioning::TextPositioningStrategy,
    },
    geometry::{Insets, Point, Size},
};

mod actor;
mod boundary;
mod component;
mod control;
mod entity;
mod interface;
mod oval;
mod rectangle;

pub use actor::ActorDefinition;
pub use boundary::BoundaryDefinition;
pub use component::ComponentDefinition;
pub use control::ControlDefinition;
pub use entity::EntityDefinition;
pub use interface::InterfaceDefinition;
pub use oval::OvalDefinition;
pub use rectangle::RectangleDefinition;

/// A trait for shape definitions that provide stateless calculations.
pub trait ShapeDefinition: std::fmt::Debug {
    /// Returns true if this shape supports containing content
    /// Default implementation returns false for safety
    fn supports_content(&self) -> bool {
        false
    }
    /// Find the intersection point where a line from point a to point b intersects with this shape
    /// centered at point a with the given size
    fn find_intersection(&self, a: Point, b: Point, a_size: Size) -> Point {
        find_rectangle_intersection(a, b, a_size)
    }

    /// Calculate the inner shape size needed to contain the given content size with padding.
    /// This is the size of the shape boundary excluding the stroke.
    /// For content-free shapes, content_size and padding may be ignored.
    fn calculate_inner_size(&self, content_size: Size, padding: Insets) -> Size;

    /// Calculate the outer shape size including the stroke.
    /// By default, this adds the stroke width to the inner size in both dimensions.
    fn calculate_outer_size(&self, content_size: Size, padding: Insets) -> Size {
        let inner_size = self.calculate_inner_size(content_size, padding);
        let stroke_width = self.stroke().width();
        Size::new(
            inner_size.width() + stroke_width,
            inner_size.height() + stroke_width,
        )
    }

    /// Renders this shape to an SVG node element.
    ///
    /// # Arguments
    ///
    /// * `size` - The dimensions of the shape to render.
    /// * `position` - The center position of the shape.
    ///
    /// # Returns
    ///
    /// A boxed SVG node representing the rendered shape.
    fn render_to_svg(&self, size: Size, position: Point) -> Box<dyn svg::Node>;

    /// Creates a boxed clone of this shape definition.
    fn clone_box(&self) -> Box<dyn ShapeDefinition>;

    /// Set the fill color for the rectangle
    fn set_fill_color(&mut self, _color: Option<Color>) -> Result<(), &'static str> {
        Err("fill_color is not supported for this shape")
    }

    /// Set the corner rounding for the rectangle
    fn set_rounded(&mut self, _radius: usize) -> Result<(), &'static str> {
        Err("rounded corners are not supported for this shape")
    }

    /// Get the stroke definition for the shape.
    fn stroke(&self) -> &Rc<StrokeDefinition>;

    /// Get the text definition for the shape.
    fn text(&self) -> &Rc<TextDefinition>;

    /// Set text definition using Rc.
    fn set_text(&mut self, text: Rc<TextDefinition>);

    /// Set stroke definition using Rc.
    fn set_stroke(&mut self, stroke: Rc<StrokeDefinition>);

    /// Returns the minimum content size required for the shape.
    ///
    /// For shapes that support content, this returns a minimum size
    /// to ensure the shape has adequate space. For content-free shapes,
    /// returns a zero size.
    fn min_content_size(&self) -> Size {
        if self.supports_content() {
            Size::new(10.0, 10.0)
        } else {
            Size::default() // Content-free shapes don't need content space
        }
    }

    /// Get the text positioning strategy for this shape
    fn text_positioning_strategy(&self) -> TextPositioningStrategy {
        TextPositioningStrategy::BelowShape
    }
}

/// Enable cloning of `Box<dyn ShapeDefinition>` by delegating to the clone_box method.
/// This allows `Rc::make_mut` to work with `Rc<Box<dyn ShapeDefinition>>`.
impl Clone for Box<dyn ShapeDefinition> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// A shape instance that combines a definition with content size and padding
#[derive(Debug, Clone)]
pub struct Shape {
    definition: Rc<Box<dyn ShapeDefinition>>,
    content_size: Size,
    padding: Insets,
}

impl Shape {
    pub fn new(definition: Rc<Box<dyn ShapeDefinition>>) -> Self {
        let content_size = definition.min_content_size();
        Self {
            definition,
            content_size,
            padding: Insets::default(),
        }
    }

    /// Returns true if this shape supports containing content.
    /// This is intended for use within the `draw` module.
    pub(super) fn supports_content(&self) -> bool {
        self.definition.supports_content()
    }

    pub fn content_size(&self) -> Size {
        self.content_size
    }

    /// Get the text positioning strategy for this shape
    pub fn text_positioning_strategy(&self) -> TextPositioningStrategy {
        self.definition.text_positioning_strategy()
    }

    /// Returns the inner size of the shape boundary, excluding stroke.
    /// This is the size needed to contain the content with padding.
    pub fn inner_size(&self) -> Size {
        self.definition
            .calculate_inner_size(self.content_size, self.padding)
    }

    /// Returns the outer size of the shape, including stroke.
    /// This is the full size the shape occupies when rendered.
    pub fn outer_size(&self) -> Size {
        self.definition
            .calculate_outer_size(self.content_size, self.padding)
    }

    /// Expand the content size for this shape to the given size if it's bigger
    /// This is only valid for content-supporting shapes
    pub fn expand_content_size_to(&mut self, content_size: Size) -> Result<(), &'static str> {
        if self.supports_content() {
            self.content_size = self.content_size.max(content_size);
            Ok(())
        } else {
            Err("Cannot expand content size on content-free shapes")
        }
    }

    /// Set the padding for this shape
    pub fn set_padding(&mut self, padding: Insets) {
        self.padding = padding;
    }

    /// Get the current padding for this shape
    pub fn padding(&self) -> Insets {
        self.padding
    }

    /// Find the intersection point where a line from point a to point b intersects with this shape
    pub fn find_intersection(&self, a: Point, b: Point, a_size: Size) -> Point {
        self.definition.find_intersection(a, b, a_size)
    }

    /// Calculate the minimum point offset for positioning content within this shape's container.
    ///
    /// This method computes the offset needed to position embedded content within a shape,
    /// taking into account the difference between the shape's total size and its content size.
    /// The result represents the padding/margin space that should be applied when positioning
    /// nested content within this shape.
    ///
    /// Calculate any additional space the shape needs beyond content + padding.
    /// This accounts for shapes like ovals that need extra room beyond just padding.
    // TODO: Validate we need this. i.e. it is not always zero.
    pub(super) fn calculate_additional_space(&self) -> Size {
        let shape_size = self.inner_size();
        let content_size = self.content_size();
        let total_padding_size = content_size.add_padding(self.padding);

        Size::new(
            shape_size.width() - total_padding_size.width(),
            shape_size.height() - total_padding_size.height(),
        )
        .max(Size::default())
    }

    /// Returns a Point representing the (x, y) offset from the shape's top-left corner
    /// to where the content area begins.
    pub fn shape_to_container_min_point(&self) -> Point {
        let additional_space = self.calculate_additional_space();

        Point::new(
            self.padding.left() + additional_space.width() / 2.0,
            self.padding.top() + additional_space.height() / 2.0,
        )
    }
}

impl Drawable for Shape {
    fn render_to_layers(&self, position: Point) -> LayeredOutput {
        let mut output = LayeredOutput::new();
        let size = self.inner_size();
        let node = self.definition.render_to_svg(size, position);
        output.add_to_layer(RenderLayer::Content, node);
        output
    }

    fn size(&self) -> Size {
        self.outer_size() // TODO merge them.
    }
}

fn find_rectangle_intersection(a: Point, b: Point, a_size: Size) -> Point {
    let half_width = a_size.width() / 2.0;
    let half_height = a_size.height() / 2.0;

    // Rectangle center is at a
    let rect_center = a;

    let dist = b.sub_point(a);

    // Normalize the direction vector
    let length = dist.hypot();
    if length < 0.001 {
        // Avoid division by zero
        return b;
    }

    let dx_norm = dist.x() / length;
    let dy_norm = dist.y() / length;

    // Find intersection with each edge of the rectangle
    // We're calculating how far we need to go along the ray to hit each edge

    // Distance to horizontal edges (top and bottom)
    let t_top = (rect_center.y() - half_height - a.y()) / dy_norm;
    let t_bottom = (rect_center.y() + half_height - a.y()) / dy_norm;

    // Distance to vertical edges (left and right)
    let t_left = (rect_center.x() - half_width - a.x()) / dx_norm;
    let t_right = (rect_center.x() + half_width - a.x()) / dx_norm;

    // Find the smallest positive t value (first intersection with rectangle)
    let mut t = f32::MAX;

    // Check each edge and find the closest valid intersection
    if t_top.is_finite() && t_top > 0.0 {
        let x = dx_norm.mul_add(t_top, a.x()); // a.x + t_top * dx_norm
        if x >= rect_center.x() - half_width && x <= rect_center.x() + half_width {
            t = t_top;
        }
    }

    if t_bottom.is_finite() && t_bottom > 0.0 && t_bottom < t {
        let x = dx_norm.mul_add(t_bottom, a.x()); // a.x + t_bottom * dx_norm
        if x >= rect_center.x() - half_width && x <= rect_center.x() + half_width {
            t = t_bottom;
        }
    }

    if t_left.is_finite() && t_left > 0.0 && t_left < t {
        let y = dy_norm.mul_add(t_left, a.y()); // a.y + t_left * dy_norm
        if y >= rect_center.y() - half_height && y <= rect_center.y() + half_height {
            t = t_left;
        }
    }

    if t_right.is_finite() && t_right > 0.0 && t_right < t {
        let y = dy_norm.mul_add(t_right, a.y()); // a.y + t_right * dy_norm
        if y >= rect_center.y() - half_height && y <= rect_center.y() + half_height {
            t = t_right;
        }
    }

    if t == f32::MAX || !t.is_finite() {
        return b; // Fallback if no intersection found
    }

    // Calculate the intersection point
    Point::new(
        dx_norm.mul_add(t, a.x()), //a.x + dx_norm * t
        dy_norm.mul_add(t, a.y()), // a.y + dy_norm * t
    )
}

#[cfg(test)]
mod tests {
    use float_cmp::assert_approx_eq;

    use super::*;

    fn assert_point_eq(actual: Point, expected: Point) {
        assert_approx_eq!(f32, actual.x(), expected.x());
        assert_approx_eq!(f32, actual.y(), expected.y());
    }

    #[test]
    fn test_intersection_from_right() {
        // Ray from center (100,100) going right to (200,100)
        // Should intersect right edge at (120, 100)
        let a = Point::new(100.0, 100.0);
        let b = Point::new(200.0, 100.0);
        let size = Size::new(40.0, 40.0);

        let result = find_rectangle_intersection(a, b, size);

        assert_point_eq(result, Point::new(120.0, 100.0));
    }

    #[test]
    fn test_intersection_from_left() {
        // Ray from center (100,100) going left to (0,100)
        // Should intersect left edge at (80, 100)
        let a = Point::new(100.0, 100.0);
        let b = Point::new(0.0, 100.0);
        let size = Size::new(40.0, 40.0);

        let result = find_rectangle_intersection(a, b, size);

        assert_point_eq(result, Point::new(80.0, 100.0));
    }

    #[test]
    fn test_intersection_from_bottom() {
        // Ray from center (100,100) going down to (100,200)
        // Should intersect bottom edge at (100, 120)
        let a = Point::new(100.0, 100.0);
        let b = Point::new(100.0, 200.0);
        let size = Size::new(40.0, 40.0);

        let result = find_rectangle_intersection(a, b, size);

        assert_point_eq(result, Point::new(100.0, 120.0));
    }

    #[test]
    fn test_intersection_from_top() {
        // Ray from center (100,100) going up to (100,0)
        // Should intersect top edge at (100, 80)
        let a = Point::new(100.0, 100.0);
        let b = Point::new(100.0, 0.0);
        let size = Size::new(40.0, 40.0);

        let result = find_rectangle_intersection(a, b, size);

        assert_point_eq(result, Point::new(100.0, 80.0));
    }

    #[test]
    fn test_intersection_diagonal() {
        // Ray from center (100,100) going diagonally to (200,200)
        // For a square, 45-degree diagonal hits corner region
        // Should intersect at (120, 120) - the corner of the rectangle
        let a = Point::new(100.0, 100.0);
        let b = Point::new(200.0, 200.0);
        let size = Size::new(40.0, 40.0);

        let result = find_rectangle_intersection(a, b, size);

        assert_point_eq(result, Point::new(120.0, 120.0));
    }

    #[test]
    fn test_intersection_same_point() {
        // Edge case: start and end are the same point
        // Should return b as fallback (avoid division by zero)
        let a = Point::new(100.0, 100.0);
        let b = Point::new(100.0, 100.0);
        let size = Size::new(40.0, 40.0);

        let result = find_rectangle_intersection(a, b, size);

        assert_point_eq(result, b);
    }

    #[test]
    fn test_intersection_zero_size() {
        // Edge case: zero-size shape (degenerate rectangle)
        // All edges collapse to center, no valid intersection possible
        // Should return b as fallback
        let a = Point::new(100.0, 100.0);
        let b = Point::new(200.0, 100.0);
        let size = Size::new(0.0, 0.0);

        let result = find_rectangle_intersection(a, b, size);

        assert_point_eq(result, b);
    }

    #[test]
    fn test_intersection_very_close_points() {
        // Edge case: points extremely close together (distance < 0.001)
        // Algorithm returns b as fallback to avoid numerical instability
        let a = Point::new(100.0, 100.0);
        let b = Point::new(100.0005, 100.0005);
        let size = Size::new(40.0, 40.0);

        let result = find_rectangle_intersection(a, b, size);

        // Distance ≈ 0.000707 < 0.001 threshold, so returns b
        assert_point_eq(result, b);
    }
}

#[cfg(test)]
mod proptest_tests {
    use float_cmp::approx_eq;
    use proptest::prelude::*;

    use super::*;

    // ===================
    // Strategies
    // ===================

    fn point_strategy() -> impl Strategy<Value = Point> {
        (-1000.0f32..1000.0, -1000.0f32..1000.0).prop_map(|(x, y)| Point::new(x, y))
    }

    fn size_strategy() -> impl Strategy<Value = Size> {
        (0.0f32..1000.0, 0.0f32..1000.0).prop_map(|(w, h)| Size::new(w, h))
    }

    // ===================
    // Property Test Functions
    // ===================

    /// The intersection result should always have finite coordinates (no NaN or infinity).
    fn check_intersection_result_is_finite(
        a: Point,
        b: Point,
        size: Size,
    ) -> Result<(), TestCaseError> {
        let result = find_rectangle_intersection(a, b, size);

        let x = result.x();
        let y = result.y();
        prop_assert!(x.is_finite(), "x coordinate is not finite: {x}");
        prop_assert!(y.is_finite(), "y coordinate is not finite: {y}");
        Ok(())
    }

    /// The intersection should either be on the rectangle boundary or equal to b (fallback).
    fn check_intersection_on_boundary_or_fallback(
        a: Point,
        b: Point,
        size: Size,
    ) -> Result<(), TestCaseError> {
        let result = find_rectangle_intersection(a, b, size);

        let half_w = size.width() / 2.0;
        let half_h = size.height() / 2.0;

        // Check if result is on any of the four edges (with tolerance)
        let on_left = approx_eq!(f32, result.x(), a.x() - half_w, epsilon = 0.1);
        let on_right = approx_eq!(f32, result.x(), a.x() + half_w, epsilon = 0.1);
        let on_top = approx_eq!(f32, result.y(), a.y() - half_h, epsilon = 0.1);
        let on_bottom = approx_eq!(f32, result.y(), a.y() + half_h, epsilon = 0.1);

        // Or check if result equals b (fallback case)
        let is_fallback = approx_eq!(f32, result.x(), b.x(), epsilon = 0.1)
            && approx_eq!(f32, result.y(), b.y(), epsilon = 0.1);

        prop_assert!(
            on_left || on_right || on_top || on_bottom || is_fallback,
            "Result {result:?} is neither on boundary of rect at {a:?} with size {size:?} nor fallback to {b:?}"
        );
        Ok(())
    }

    // ===================
    // Proptest Wrappers
    // ===================

    proptest! {
        #[test]
        fn intersection_result_is_finite(a in point_strategy(), b in point_strategy(), size in size_strategy()) {
            check_intersection_result_is_finite(a, b, size)?;
        }

        #[test]
        fn intersection_on_boundary_or_fallback(a in point_strategy(), b in point_strategy(), size in size_strategy()) {
            check_intersection_on_boundary_or_fallback(a, b, size)?;
        }
    }
}
