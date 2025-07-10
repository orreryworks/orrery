use super::ShapeDefinition;
use crate::{
    color::Color,
    geometry::{Insets, Point, Size},
};
use std::{cell::RefCell, rc::Rc};
use svg::{self, node::element as svg_element};

/// Rectangle shape definition
#[derive(Debug, Clone)]
pub struct RectangleDefinition {
    fill_color: Option<Color>,
    line_color: Color,
    line_width: usize,
    rounded: usize,
}

impl RectangleDefinition {
    /// Create a new rectangle definition with default values
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for RectangleDefinition {
    fn default() -> Self {
        Self {
            fill_color: None,
            line_color: Color::default(),
            line_width: 2,
            rounded: 0,
        }
    }
}

impl ShapeDefinition for RectangleDefinition {
    fn supports_content(&self) -> bool {
        true
    }

    fn find_intersection(&self, a: Point, b: Point, a_size: &Size) -> Point {
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

    fn calculate_shape_size(&self, content_size: Size, padding: Insets) -> Size {
        let min_size = Size::new(10.0, 10.0);
        content_size.add_padding(padding).max(min_size)
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

    fn rounded(&self) -> usize {
        self.rounded
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

    fn set_rounded(&mut self, radius: usize) -> Result<(), &'static str> {
        self.rounded = radius;
        Ok(())
    }

    fn render_to_svg(&self, size: Size, position: Point) -> Box<dyn svg::Node> {
        // Calculate the actual top-left position for the rectangle
        // (position is the center of the component)
        let bounds = position.to_bounds(size);

        // Main rectangle
        let mut rect = svg_element::Rectangle::new()
            .set("x", bounds.min_x())
            .set("y", bounds.min_y())
            .set("width", size.width())
            .set("height", size.height())
            .set("stroke", self.line_color().to_string())
            .set("stroke-width", self.line_width())
            .set("fill", "white")
            .set("rx", self.rounded());

        if let Some(fill_color) = self.fill_color() {
            rect = rect.set("fill", fill_color.to_string());
        }

        rect.into()
    }
}
