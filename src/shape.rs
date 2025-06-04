use crate::layout::{Point, Size};

/// A trait for different shape types that can be used in diagrams
pub trait Shape {
    /// Find the intersection point where a line from point a to point b intersects with this shape
    /// centered at point a with the given size
    fn find_intersection(&self, a: Point, b: Point, a_size: &Size) -> Point;

    /// Get a string identifier for this shape type
    fn name(&self) -> &'static str;

    /// Calculate the size of the shape needed to contain the given content size
    /// This allows shapes to add padding or adjust dimensions based on their specific requirements
    fn calculate_shape_size(&self, content_size: Size) -> Size;
}

pub struct Rectangle;

pub struct Oval;

impl Shape for Rectangle {
    fn find_intersection(&self, a: Point, b: Point, a_size: &Size) -> Point {
        let half_width = a_size.width() / 2.0;
        let half_height = a_size.height() / 2.0;

        // Rectangle center is at a
        let rect_center = a;

        let dist = b.sub(a);

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

    fn name(&self) -> &'static str {
        "Rectangle"
    }

    fn calculate_shape_size(&self, content_size: Size) -> Size {
        content_size.max(Size::new(10.0, 10.0))
    }
}

impl Shape for Oval {
    fn find_intersection(&self, a: Point, b: Point, a_size: &Size) -> Point {
        // For an ellipse, finding the intersection is more complex than for a rectangle
        // We use a parametric approach based on the direction vector

        let half_width = a_size.width() / 2.0;
        let half_height = a_size.height() / 2.0;

        let dist = b.sub(a);

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

    fn name(&self) -> &'static str {
        "Oval"
    }

    fn calculate_shape_size(&self, content_size: Size) -> Size {
        // The largest rectangle that fits in an ellipse with semi-axes (a,b) has dimensions:
        // width = a√2, height = b√2
        let sqrt_2 = 2.0_f32.sqrt();
        content_size.scale(sqrt_2).max(Size::new(10.0, 10.0))
    }
}
