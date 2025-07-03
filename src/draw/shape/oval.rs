use super::ShapeDefinition;
use crate::{
    color::Color,
    geometry::{Insets, Point, Size},
};
use std::{cell::RefCell, rc::Rc};
use svg::{self, node::element as svg_element};

/// Oval shape definition
#[derive(Debug, Clone)]
pub struct OvalDefinition {
    fill_color: Option<Color>,
    line_color: Color,
    line_width: usize,
}

impl OvalDefinition {
    /// Create a new oval definition with default values
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for OvalDefinition {
    fn default() -> Self {
        Self {
            fill_color: None,
            line_color: Color::default(),
            line_width: 2,
        }
    }
}

impl ShapeDefinition for OvalDefinition {
    fn find_intersection(&self, a: Point, b: Point, a_size: &Size) -> Point {
        // For an ellipse, finding the intersection is more complex than for a rectangle
        // We use a parametric approach based on the direction vector

        let half_width = a_size.width() / 2.0;
        let half_height = a_size.height() / 2.0;

        let dist = b.sub_point(a);

        // Normalize the direction vector
        let length = dist.hypot(); // (dx * dx + dy * dy).sqrt()
        if length < 0.001 {
            // Avoid division by zero
            return b;
        }

        let dx_norm = dist.x() / length;
        let dy_norm = dist.y() / length;

        // We need to solve for the intersection of a ray (a + t * direction) with an ellipse
        // This is a quadratic equation

        // For simplicity, we'll use an approximation that works well for most cases
        // This calculates the intersection with a normalized ellipse and then scales back
        // First we find the angle of the direction vector
        let angle = dy_norm.atan2(dx_norm);

        // Then find the radius of the ellipse at that angle
        // r = (a*b) / sqrt((b*cos(θ))² + (a*sin(θ))²)
        // where a is half_width and b is half_height
        let cos_angle = angle.cos();
        let sin_angle = angle.sin();
        let radius =
            (half_width * half_height) / (half_height * cos_angle).hypot(half_width * sin_angle);

        // Calculate the intersection point
        Point::new(
            dx_norm.mul_add(radius, a.x()), // a.x + dx_norm * radius
            dy_norm.mul_add(radius, a.y()), // a.y + dy_norm * radius
        )
    }

    fn calculate_shape_size(&self, content_size: Size, padding: Insets) -> Size {
        // The largest rectangle that fits in an ellipse with semi-axes (a,b) has dimensions:
        // width = a√2, height = b√2
        // So we need to scale up the content to create an ellipse that can contain it
        let min_size = Size::new(10.0, 10.0);
        let sqrt_2 = 2.0_f32.sqrt();
        content_size
            .scale(sqrt_2)
            .add_padding(padding)
            .max(min_size)
    }

    fn clone_new_rc(&self) -> Rc<RefCell<dyn ShapeDefinition>> {
        Rc::new(RefCell::new(self.clone()))
    }

    fn fill_color(&self) -> Option<Color> {
        self.fill_color
    }

    fn line_color(&self) -> Color {
        self.line_color
    }

    fn line_width(&self) -> usize {
        self.line_width
    }

    fn set_fill_color(&mut self, color: Option<Color>) -> Result<(), &'static str> {
        self.fill_color = color;
        Ok(())
    }

    fn set_line_color(&mut self, color: Color) -> Result<(), &'static str> {
        self.line_color = color;
        Ok(())
    }

    fn set_line_width(&mut self, width: usize) -> Result<(), &'static str> {
        self.line_width = width;
        Ok(())
    }

    fn render_to_svg(&self, size: Size, position: Point) -> Box<dyn svg::Node> {
        // Use ellipse which takes center point (cx, cy) plus radiuses (rx, ry)
        let rx = size.width() / 2.0;
        let ry = size.height() / 2.0;

        let mut ellipse = svg_element::Ellipse::new()
            .set("cx", position.x())
            .set("cy", position.y())
            .set("rx", rx)
            .set("ry", ry)
            .set("stroke", self.line_color().to_string())
            .set("stroke-width", self.line_width())
            .set("fill", "white");

        if let Some(fill_color) = self.fill_color() {
            ellipse = ellipse.set("fill", fill_color.to_string());
        }

        ellipse.into()
    }
}
