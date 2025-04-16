use crate::layout::common::{Point, Size};

/// A trait for different shape types that can be used in diagrams
pub trait Shape {
    /// Find the intersection point where a line from point a to point b intersects with this shape
    /// centered at point a with the given size
    fn find_intersection(&self, a: &Point, b: &Point, a_size: &Size) -> Point;

    /// Get a string identifier for this shape type
    fn name(&self) -> &'static str;
}

pub struct Rectangle;

pub struct Oval;

impl Shape for Rectangle {
    fn find_intersection(&self, a: &Point, b: &Point, a_size: &Size) -> Point {
        let half_width = a_size.width / 2.0;
        let half_height = a_size.height / 2.0;

        // Rectangle center is at a
        let rect_center = a;

        // Direction vector from a to b
        let dx = b.x - a.x;
        let dy = b.y - a.y;

        // Normalize the direction vector
        let length = (dx * dx + dy * dy).sqrt();
        if length < 0.001 {
            // Avoid division by zero
            return *b;
        }

        let dx_norm = dx / length;
        let dy_norm = dy / length;

        // Find intersection with each edge of the rectangle
        // We're calculating how far we need to go along the ray to hit each edge

        // Distance to horizontal edges (top and bottom)
        let t_top = (rect_center.y - half_height - a.y) / dy_norm;
        let t_bottom = (rect_center.y + half_height - a.y) / dy_norm;

        // Distance to vertical edges (left and right)
        let t_left = (rect_center.x - half_width - a.x) / dx_norm;
        let t_right = (rect_center.x + half_width - a.x) / dx_norm;

        // Find the smallest positive t value (first intersection with rectangle)
        let mut t = f32::MAX;

        // Check each edge and find the closest valid intersection
        if t_top.is_finite() && t_top > 0.0 {
            let x = a.x + t_top * dx_norm;
            if x >= rect_center.x - half_width && x <= rect_center.x + half_width {
                t = t_top;
            }
        }

        if t_bottom.is_finite() && t_bottom > 0.0 && t_bottom < t {
            let x = a.x + t_bottom * dx_norm;
            if x >= rect_center.x - half_width && x <= rect_center.x + half_width {
                t = t_bottom;
            }
        }

        if t_left.is_finite() && t_left > 0.0 && t_left < t {
            let y = a.y + t_left * dy_norm;
            if y >= rect_center.y - half_height && y <= rect_center.y + half_height {
                t = t_left;
            }
        }

        if t_right.is_finite() && t_right > 0.0 && t_right < t {
            let y = a.y + t_right * dy_norm;
            if y >= rect_center.y - half_height && y <= rect_center.y + half_height {
                t = t_right;
            }
        }

        if t == f32::MAX || !t.is_finite() {
            return *b; // Fallback if no intersection found
        }

        // Calculate the intersection point
        Point {
            x: a.x + dx_norm * t,
            y: a.y + dy_norm * t,
        }
    }

    fn name(&self) -> &'static str {
        "Rectangle"
    }
}

impl Shape for Oval {
    fn find_intersection(&self, a: &Point, b: &Point, a_size: &Size) -> Point {
        // For an ellipse, finding the intersection is more complex than for a rectangle
        // We use a parametric approach based on the direction vector

        let half_width = a_size.width / 2.0;
        let half_height = a_size.height / 2.0;

        // Direction vector from a to b
        let dx = b.x - a.x;
        let dy = b.y - a.y;

        // Normalize the direction vector
        let length = (dx * dx + dy * dy).sqrt();
        if length < 0.001 {
            // Avoid division by zero
            return *b;
        }

        let dx_norm = dx / length;
        let dy_norm = dy / length;

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
        let radius = (half_width * half_height)
            / ((half_height * cos_angle).powi(2) + (half_width * sin_angle).powi(2)).sqrt();

        // Calculate the intersection point
        Point {
            x: a.x + dx_norm * radius,
            y: a.y + dy_norm * radius,
        }
    }

    fn name(&self) -> &'static str {
        "Oval"
    }
}
