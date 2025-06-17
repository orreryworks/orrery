use crate::layout::{Bounds, Point, Size};

/// A trait for shape definitions that provide stateless calculations
pub trait ShapeDefinition: std::fmt::Debug {
    /// Get a string identifier for this shape type
    fn name(&self) -> &'static str;

    /// Find the intersection point where a line from point a to point b intersects with this shape
    /// centered at point a with the given size
    fn find_intersection(&self, a: Point, b: Point, a_size: &Size) -> Point;

    /// Calculate the shape size needed to contain the given content size with padding
    fn calculate_shape_size(&self, content_size: Size, padding: f32) -> Size;

    fn new_shape(&self) -> Box<dyn Shape>;
}

/// A trait for different shape types that can be used in diagrams (for polymorphic usage)
pub trait Shape: std::fmt::Debug {
    /// Get a string identifier for this shape type
    fn name(&self) -> &'static str;

    /// Find the intersection point where a line from point a to point b intersects with this shape
    /// centered at point a with the given size
    fn find_intersection(&self, a: Point, b: Point) -> Point;

    /// Return the content size of this shape
    fn content_size(&self) -> Size;

    /// Size of the shape needed to contain the given content size
    /// This allows shapes to add padding or adjust dimensions based on their specific requirements
    fn shape_size(&self) -> Size;

    /// Set the content size for this shape
    fn set_content_size(&mut self, content_size: Size);

    /// Set the padding for this shape
    fn set_padding(&mut self, padding: f32);

    /// Clone this shape instance into a new boxed trait object
    fn clone_box(&self) -> Box<dyn Shape>;

    /// Calculate the minimum point offset for positioning content within this shape's container.
    ///
    /// This method computes the offset needed to position embedded content within a shape,
    /// taking into account the difference between the shape's total size and its content size.
    /// The result represents the padding/margin space that should be applied when positioning
    /// nested content within this shape.
    ///
    /// Returns a Point representing the (x, y) offset from the shape's top-left corner
    /// to where the content area begins.
    fn shape_to_container_min_point(&self) -> Point {
        let shape_size = self.shape_size();
        let content_size = self.content_size();
        Point::new(
            shape_size.width() - content_size.width(),
            shape_size.height() - content_size.height(),
        )
        .scale(0.5)
    }

    /// Calculates the bounds of this shape based on the center position.
    fn bounds(&self, position: Point) -> Bounds {
        position.to_bounds(self.shape_size())
    }
}

/// A shape instance that combines a definition with content size and padding
#[derive(Debug, Clone)]
pub struct ShapeInstance<T: ShapeDefinition> {
    definition: T,
    content_size: Size,
    padding: f32,
}

impl<T: ShapeDefinition + Clone + 'static> ShapeInstance<T> {
    pub fn new(definition: T) -> Self {
        Self {
            definition,
            content_size: Size::default(),
            padding: 0.0,
        }
    }
}

impl<T: ShapeDefinition + Clone + 'static> Shape for ShapeInstance<T> {
    fn name(&self) -> &'static str {
        self.definition.name()
    }

    fn content_size(&self) -> Size {
        self.content_size
    }

    /// Size of the shape needed to contain the given content size
    fn shape_size(&self) -> Size {
        self.definition
            .calculate_shape_size(self.content_size, self.padding)
    }

    /// Set the content size for this shape
    fn set_content_size(&mut self, content_size: Size) {
        self.content_size = content_size;
    }

    /// Set the padding for this shape
    fn set_padding(&mut self, padding: f32) {
        self.padding = padding;
    }

    /// Find the intersection point where a line from point a to point b intersects with this shape
    fn find_intersection(&self, a: Point, b: Point) -> Point {
        self.definition.find_intersection(a, b, &self.shape_size())
    }

    /// Clone this shape instance into a new boxed trait object
    fn clone_box(&self) -> Box<dyn Shape> {
        let mut cloned = ShapeInstance::new(self.definition.clone());
        cloned.set_content_size(self.content_size);
        cloned.set_padding(self.padding);
        Box::new(cloned)
    }
}

/// Rectangle shape definition
#[derive(Default, Clone)]
pub struct RectangleDefinition;

/// Oval shape definition
#[derive(Default, Clone)]
pub struct OvalDefinition;

impl ShapeDefinition for RectangleDefinition {
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

    fn calculate_shape_size(&self, content_size: Size, padding: f32) -> Size {
        let min_size = Size::new(10.0, 10.0);
        content_size.add_padding(padding).max(min_size)
    }

    fn new_shape(&self) -> Box<dyn Shape> {
        Box::new(ShapeInstance::new(self.clone()))
    }
}

impl std::fmt::Debug for RectangleDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShapeDefinition")
            .field("name", &self.name())
            .finish()
    }
}

impl ShapeDefinition for OvalDefinition {
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

    fn calculate_shape_size(&self, content_size: Size, padding: f32) -> Size {
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

    fn new_shape(&self) -> Box<dyn Shape> {
        Box::new(ShapeInstance::new(self.clone()))
    }
}

impl std::fmt::Debug for OvalDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShapeDefinition")
            .field("name", &self.name())
            .finish()
    }
}

/// Type aliases for commonly used shape instances
pub type Rectangle = ShapeInstance<RectangleDefinition>;
pub type Oval = ShapeInstance<OvalDefinition>;

impl Default for Rectangle {
    fn default() -> Self {
        ShapeInstance::new(RectangleDefinition)
    }
}

impl Default for Oval {
    fn default() -> Self {
        ShapeInstance::new(OvalDefinition)
    }
}
