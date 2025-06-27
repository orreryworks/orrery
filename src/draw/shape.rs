use crate::{
    color::Color,
    draw::Drawable,
    geometry::{Insets, Point, Size},
};
use std::cell::RefCell;
use std::rc::Rc;

mod oval;
mod rectangle;

pub use oval::OvalDefinition;
pub use rectangle::RectangleDefinition;

/// A trait for shape definitions that provide stateless calculations
pub trait ShapeDefinition: std::fmt::Debug {
    /// Find the intersection point where a line from point a to point b intersects with this shape
    /// centered at point a with the given size
    fn find_intersection(&self, a: Point, b: Point, a_size: &Size) -> Point;

    /// Calculate the shape size needed to contain the given content size with padding
    fn calculate_shape_size(&self, content_size: Size, padding: Insets) -> Size;

    fn render_to_svg(&self, size: Size, position: Point) -> Box<dyn svg::Node>;

    fn clone_new_rc(&self) -> Rc<RefCell<dyn ShapeDefinition>>;

    /// Set the fill color for the rectangle
    fn set_fill_color(&mut self, _color: Option<Color>) -> Result<(), &'static str> {
        Err("fill_color is not supported for this shape")
    }

    /// Set the line color for the rectangle
    fn set_line_color(&mut self, _color: Color) -> Result<(), &'static str> {
        Err("line_color is not supported for this shape")
    }

    /// Set the line width for the rectangle
    fn set_line_width(&mut self, _width: usize) -> Result<(), &'static str> {
        Err("line_width is not supported for this shape")
    }

    /// Set the corner rounding for the rectangle
    fn set_rounded(&mut self, _radius: usize) -> Result<(), &'static str> {
        Err("rounded corners are not supported for this shape")
    }

    /// Get the fill color of the rectangle
    fn fill_color(&self) -> Option<Color> {
        unimplemented!("fill_color is not supported for this shape")
    }

    /// Get the line color of the rectangle
    fn line_color(&self) -> Color {
        unimplemented!("line_color is not supported for this shape")
    }

    /// Get the line width of the rectangle
    fn line_width(&self) -> usize {
        unimplemented!("line_width is not supported for this shape")
    }

    /// Get the corner rounding of the rectangle
    fn rounded(&self) -> usize {
        unimplemented!("rounded corners are not supported for this shape")
    }

    fn min_content_size(&self) -> Size {
        Size::new(10.0, 10.0) // Default minimum size for content
    }
}

/// A shape instance that combines a definition with content size and padding
#[derive(Debug, Clone)]
pub struct Shape {
    definition: Rc<RefCell<dyn ShapeDefinition>>,
    content_size: Size,
    padding: Insets,
}

impl Shape {
    pub fn new(definition: Rc<RefCell<dyn ShapeDefinition>>) -> Self {
        let content_size = definition.borrow().min_content_size();
        Self {
            definition,
            content_size,
            padding: Insets::default(),
        }
    }

    pub fn content_size(&self) -> Size {
        self.content_size
    }

    /// Size of the shape needed to contain the given content size
    pub fn shape_size(&self) -> Size {
        self.definition
            .borrow()
            .calculate_shape_size(self.content_size, self.padding)
    }

    /// Expand the content size for this shape to the given size if it's bigger
    pub fn expand_content_size_to(&mut self, content_size: Size) {
        self.content_size = self.content_size.max(content_size);
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
    pub fn find_intersection(&self, a: Point, b: Point) -> Point {
        self.definition
            .borrow()
            .find_intersection(a, b, &self.shape_size())
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
        let shape_def = self.definition.borrow();
        shape_def.render_to_svg(size, position)
    }

    fn size(&self) -> Size {
        self.shape_size() // TODO merge them.
    }
}
