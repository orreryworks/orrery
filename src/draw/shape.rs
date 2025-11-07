use std::borrow::Cow;

use crate::{
    color::Color,
    draw::{Drawable, StrokeDefinition, text_positioning::TextPositioningStrategy},
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

/// A trait for shape definitions that provide stateless calculations
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

    /// Calculate the shape size needed to contain the given content size with padding
    /// For content-free shapes, content_size and padding may be ignored
    fn calculate_shape_size(&self, content_size: Size, padding: Insets) -> Size;

    fn render_to_svg(&self, size: Size, position: Point) -> Box<dyn svg::Node>;

    fn clone_box(&self) -> Box<dyn ShapeDefinition>;

    /// Create a new shape definition with the fill color changed
    /// Default implementation returns error - override in concrete implementations
    fn with_fill_color(
        &self,
        _color: Option<Color>,
    ) -> Result<Box<dyn ShapeDefinition>, &'static str> {
        Err("with_fill_color is not supported for this shape")
    }

    /// Create a new shape definition with the corner rounding changed
    /// Default implementation returns error - override in concrete implementations
    fn with_rounded(&self, _radius: usize) -> Result<Box<dyn ShapeDefinition>, &'static str> {
        Err("with_rounded is not supported for this shape")
    }

    /// Set the fill color for the rectangle
    fn set_fill_color(&mut self, _color: Option<Color>) -> Result<(), &'static str> {
        Err("fill_color is not supported for this shape")
    }

    /// Set the corner rounding for the rectangle
    fn set_rounded(&mut self, _radius: usize) -> Result<(), &'static str> {
        Err("rounded corners are not supported for this shape")
    }

    /// Set the stroke definition for the shape
    fn set_stroke(&mut self, _stroke: Cow<'static, StrokeDefinition>) -> Result<(), &'static str> {
        Err("set_stroke is not supported for this shape")
    }

    /// Create a new shape definition with the stroke changed
    /// Default implementation returns error - override in concrete implementations
    fn with_stroke(
        &self,
        _stroke: Cow<'static, StrokeDefinition>,
    ) -> Result<Box<dyn ShapeDefinition>, &'static str> {
        Err("with_stroke is not supported for this shape")
    }

    /// Get the fill color of the rectangle
    fn fill_color(&self) -> Option<Color> {
        unimplemented!("fill_color is not supported for this shape")
    }

    /// Get the stroke definition for the shape
    fn stroke(&self) -> &StrokeDefinition {
        unimplemented!("stroke is not supported for this shape")
    }

    /// Get the corner rounding of the rectangle
    fn rounded(&self) -> usize {
        unimplemented!("rounded corners are not supported for this shape")
    }

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

/// A shape instance that combines a definition with content size and padding
#[derive(Debug)]
pub struct Shape {
    definition: Box<dyn ShapeDefinition>,
    content_size: Size,
    padding: Insets,
}

impl Clone for Shape {
    fn clone(&self) -> Self {
        Self {
            definition: self.definition.clone_box(),
            ..*self
        }
    }
}
impl Shape {
    pub fn new(definition: Box<dyn ShapeDefinition>) -> Self {
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

    /// Size of the shape needed to contain the given content size
    pub fn shape_size(&self) -> Size {
        self.definition
            .calculate_shape_size(self.content_size, self.padding)
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
    pub(super) fn calculate_additional_space(&self) -> Size {
        let shape_size = self.shape_size();
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
    fn render_to_svg(&self, position: Point) -> Box<dyn svg::Node> {
        let size = self.shape_size();
        self.definition.render_to_svg(size, position)
    }

    fn size(&self) -> Size {
        self.shape_size() // TODO merge them.
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
