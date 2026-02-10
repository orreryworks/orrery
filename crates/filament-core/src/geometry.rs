//! Geometric primitives for diagram layout and positioning.
//!
//! This module provides fundamental geometric types used throughout Filament
//! for calculating positions, sizes, and bounding boxes of diagram elements.
//!
//! # Overview
//!
//! - [`Point`] - A 2D coordinate in diagram space
//! - [`Size`] - Width and height dimensions
//! - [`Bounds`] - A rectangular bounding box defined by minimum and maximum coordinates
//! - [`Insets`] - Padding/margin values for four sides
//!
//! # Coordinate System
//!
//! Filament uses a coordinate system consistent with SVG:
//!
//! ```text
//!   (0,0) ────────► +X
//!     │
//!     │
//!     │
//!     ▼
//!    +Y
//! ```
//!
//! - **Origin**: Top-left corner at `(0, 0)`
//! - **X-axis**: Increases rightward (positive to the right)
//! - **Y-axis**: Increases downward (positive downward)
//!
//! This convention matches SVG and most screen coordinate systems.

/// A 2D point representing a position in diagram coordinate space.
///
/// Points use `f32` coordinates and provide operations for basic vector math.
/// The coordinate system has origin at top-left with Y increasing downward,
/// (see [module documentation](self) for details).
///
/// # Examples
///
/// ```
/// # use filament_core::geometry::Point;
/// let p1 = Point::new(10.0, 20.0);
/// let p2 = Point::new(5.0, 5.0);
///
/// // Vector addition
/// let sum = p1.add_point(p2);
/// assert_eq!(sum.x(), 15.0);
/// assert_eq!(sum.y(), 25.0);
///
/// // Midpoint calculation
/// let mid = p1.midpoint(p2);
/// assert_eq!(mid.x(), 7.5);
/// assert_eq!(mid.y(), 12.5);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Point {
    x: f32,
    y: f32,
}

impl Point {
    /// Creates a new point with the specified coordinates
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Returns the x-coordinate of the point
    pub fn x(self) -> f32 {
        self.x
    }

    /// Returns the y-coordinate of the point
    pub fn y(self) -> f32 {
        self.y
    }

    /// Creates a new point with the specified x-coordinate
    pub fn with_x(mut self, x: f32) -> Self {
        self.x = x;
        self
    }

    /// Creates a new point with the specified y-coordinate
    pub fn with_y(mut self, y: f32) -> Self {
        self.y = y;
        self
    }

    /// Checks if both x and y coordinates are zero
    pub fn is_zero(self) -> bool {
        self.x == 0.0 && self.y == 0.0
    }

    /// Adds another point to this point, returning a new point.
    ///
    /// # Examples
    ///
    /// ```
    /// # use filament_core::geometry::Point;
    /// let position = Point::new(100.0, 50.0);
    /// let offset = Point::new(10.0, -5.0);
    ///
    /// let moved = position.add_point(offset);
    /// assert_eq!(moved.x(), 110.0);
    /// assert_eq!(moved.y(), 45.0);
    /// ```
    pub fn add_point(self, other: Point) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }

    /// Subtracts another point to this point, returning a new point
    pub fn sub_point(self, other: Point) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }

    /// Calculates the midpoint between this point and another point
    pub fn midpoint(self, other: Point) -> Self {
        Self {
            x: (self.x + other.x) / 2.0,
            y: (self.y + other.y) / 2.0,
        }
    }

    /// Calculates the hypotenuse (Euclidean distance from origin)
    pub fn hypot(self) -> f32 {
        self.x.hypot(self.y)
    }

    /// Multiplies both coordinates by the given factor.
    ///
    /// # Examples
    ///
    /// ```
    /// # use filament_core::geometry::Point;
    /// let point = Point::new(10.0, 20.0);
    ///
    /// let doubled = point.scale(2.0);
    /// assert_eq!(doubled.x(), 20.0);
    /// assert_eq!(doubled.y(), 40.0);
    ///
    /// let halved = point.scale(0.5);
    /// assert_eq!(halved.x(), 5.0);
    /// assert_eq!(halved.y(), 10.0);
    /// ```
    pub fn scale(self, factor: f32) -> Self {
        Self {
            x: self.x * factor,
            y: self.y * factor,
        }
    }

    /// Returns a new point with absolute values of both coordinates
    pub fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }

    /// Converts a point and size into a bounds rectangle
    ///
    /// The point is treated as the center of the bounds, and the size
    /// is distributed equally in all directions around that center.
    pub fn to_bounds(self, size: Size) -> Bounds {
        Bounds::new_from_center(self, size)
    }
}

/// Represents the dimensions of an element with width and height
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Size {
    width: f32,
    height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    /// Returns the width dimension of this size
    pub fn width(self) -> f32 {
        self.width
    }

    /// Returns the height dimension of this size
    pub fn height(self) -> f32 {
        self.height
    }

    /// Returns a new Size with the maximum width and height between this size and another
    pub fn max(self, other: Size) -> Self {
        Self {
            width: self.width.max(other.width),
            height: self.height.max(other.height),
        }
    }

    /// Returns a new Size with padding added to both width and height
    ///
    /// The padding is applied according to the specified Insets values
    pub fn add_padding(self, insets: Insets) -> Self {
        Self {
            width: self.width + insets.horizontal_sum(),
            height: self.height + insets.vertical_sum(),
        }
    }

    /// Multiplies both dimension by the given factor
    pub fn scale(self, factor: f32) -> Self {
        Self {
            width: self.width * factor,
            height: self.height * factor,
        }
    }

    /// Returns true if both width and height are zero
    pub fn is_zero(self) -> bool {
        self.width == 0.0 && self.height == 0.0
    }

    /// Merges two sizes horizontally by adding their widths and taking the maximum height
    pub fn merge_horizontal(self, other: Size) -> Self {
        Self {
            width: self.width + other.width,
            height: self.height.max(other.height),
        }
    }

    /// Merges two sizes vertically by adding their heights and taking the maximum width
    pub fn merge_vertical(self, other: Size) -> Self {
        Self {
            width: self.width.max(other.width),
            height: self.height + other.height,
        }
    }
}

/// Represents a rectangular bounding box with minimum and maximum coordinates
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Bounds {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl Bounds {
    /// Creates a new bounds from a center point and a size
    pub fn new_from_center(center: Point, size: Size) -> Self {
        let half_width = size.width / 2.0;
        let half_height = size.height / 2.0;
        Self {
            min_x: center.x - half_width,
            min_y: center.y - half_height,
            max_x: center.x + half_width,
            max_y: center.y + half_height,
        }
    }

    /// Creates a new bounds from a top-left point and a size
    pub fn new_from_top_left(top_left: Point, size: Size) -> Self {
        Self {
            min_x: top_left.x,
            min_y: top_left.y,
            max_x: top_left.x + size.width,
            max_y: top_left.y + size.height,
        }
    }

    /// Returns the minimum x-coordinate of the bounds
    pub fn min_x(self) -> f32 {
        self.min_x
    }

    /// Returns the minimum y-coordinate of the bounds
    pub fn min_y(self) -> f32 {
        self.min_y
    }

    /// Returns the maximum x-coordinate of the bounds
    #[allow(dead_code)]
    pub fn max_x(self) -> f32 {
        self.max_x
    }

    /// Returns the maximum y-coordinate of the bounds
    pub fn max_y(self) -> f32 {
        self.max_y
    }

    /// Returns the center point of the bounds
    pub fn center(self) -> Point {
        Point::new(
            (self.min_x + self.max_x) / 2.0,
            (self.min_y + self.max_y) / 2.0,
        )
    }

    /// Sets the maximum y-coordinate of the bounds and returns the modified bounds
    pub fn with_max_y(mut self, max_y: f32) -> Self {
        self.max_y = max_y;
        self
    }

    /// Returns the width of the bounds
    pub fn width(self) -> f32 {
        self.max_x - self.min_x
    }

    /// Returns the height of the bounds
    pub fn height(self) -> f32 {
        self.max_y - self.min_y
    }

    /// Returns the top-left corner as a Point
    pub fn min_point(self) -> Point {
        Point {
            x: self.min_x,
            y: self.min_y,
        }
    }

    /// Converts bounds to a Size object
    pub fn to_size(self) -> Size {
        Size {
            width: self.width(),
            height: self.height(),
        }
    }

    /// Merges two bounds to create a larger bounds that contains both.
    ///
    /// The resulting bounds will have the minimum values of both bounds for min_x and min_y,
    /// and the maximum values of both bounds for max_x and max_y.
    ///
    /// # Examples
    ///
    /// ```
    /// # use filament_core::geometry::{Bounds, Point, Size};
    /// let header = Bounds::new_from_top_left(Point::new(0.0, 0.0), Size::new(100.0, 30.0));
    /// let content = Bounds::new_from_top_left(Point::new(10.0, 40.0), Size::new(120.0, 80.0));
    ///
    /// let combined = header.merge(&content);
    /// assert_eq!(combined.min_x(), 0.0);   // From header
    /// assert_eq!(combined.min_y(), 0.0);   // From header
    /// assert_eq!(combined.width(), 130.0); // Spans both (0 to 130)
    /// assert_eq!(combined.height(), 120.0); // Spans both (0 to 120)
    /// ```
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }

    /// Moves the bounds by the specified offset.
    ///
    /// This translates both the minimum and maximum coordinates by the given amount.
    ///
    /// # Examples
    ///
    /// ```
    /// # use filament_core::geometry::{Bounds, Point, Size};
    /// let bounds = Bounds::new_from_top_left(Point::new(10.0, 20.0), Size::new(50.0, 30.0));
    /// let offset = Point::new(100.0, 50.0);
    ///
    /// let moved = bounds.translate(offset);
    /// assert_eq!(moved.min_x(), 110.0);
    /// assert_eq!(moved.min_y(), 70.0);
    /// assert_eq!(moved.width(), 50.0);  // Size unchanged
    /// assert_eq!(moved.height(), 30.0); // Size unchanged
    /// ```
    pub fn translate(&self, offset: Point) -> Self {
        Self {
            min_x: self.min_x + offset.x,
            min_y: self.min_y + offset.y,
            max_x: self.max_x + offset.x,
            max_y: self.max_y + offset.y,
        }
    }

    /// Moves the bounds in the opposite direction of the specified offset
    ///
    /// This subtracts the offset from both minimum and maximum coordinates.
    pub fn inverse_translate(&self, offset: Point) -> Self {
        Self {
            min_x: self.min_x - offset.x,
            min_y: self.min_y - offset.y,
            max_x: self.max_x - offset.x,
            max_y: self.max_y - offset.y,
        }
    }

    /// Expands the bounds by adding insets.
    ///
    /// This decreases the minimum coordinates by left/top insets and increases
    /// the maximum coordinates by right/bottom insets, effectively growing the bounds.
    #[allow(dead_code)]
    pub fn add_padding(&self, insets: Insets) -> Self {
        Self {
            min_x: self.min_x - insets.left(),
            min_y: self.min_y - insets.top(),
            max_x: self.max_x + insets.right(),
            max_y: self.max_y + insets.bottom(),
        }
    }
}

/// Represents spacing around an element (padding, margin, etc.)
/// with potentially different values for each side
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Insets {
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
}

impl Insets {
    /// Creates new insets with specified values for each side
    #[allow(dead_code)]
    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Creates uniform insets with the same value for all sides
    pub fn uniform(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// Returns the top inset value
    pub fn top(self) -> f32 {
        self.top
    }

    /// Returns the right inset value
    pub fn right(self) -> f32 {
        self.right
    }

    /// Returns the bottom inset value
    pub fn bottom(self) -> f32 {
        self.bottom
    }

    /// Returns the left inset value
    pub fn left(self) -> f32 {
        self.left
    }

    /// Returns a new Insets with the specified top value
    pub fn with_top(self, top: f32) -> Self {
        Self { top, ..self }
    }

    /// Returns the sum of left and right insets
    pub fn horizontal_sum(self) -> f32 {
        self.left + self.right
    }

    /// Returns the sum of top and bottom insets
    pub fn vertical_sum(self) -> f32 {
        self.top + self.bottom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_new() {
        let point = Point::new(3.5, 4.2);
        assert_eq!(point.x(), 3.5);
        assert_eq!(point.y(), 4.2);
    }

    #[test]
    fn test_point_default() {
        let point = Point::default();
        assert_eq!(point.x(), 0.0);
        assert_eq!(point.y(), 0.0);
        assert!(point.is_zero());
    }

    #[test]
    fn test_point_is_zero() {
        assert!(Point::new(0.0, 0.0).is_zero());
        assert!(!Point::new(1.0, 0.0).is_zero());
        assert!(!Point::new(0.0, 1.0).is_zero());
        assert!(!Point::new(1.0, 1.0).is_zero());
    }

    #[test]
    fn test_point_add() {
        let p1 = Point::new(1.0, 2.0);
        let p2 = Point::new(3.0, 4.0);
        let result = p1.add_point(p2);
        assert_eq!(result.x(), 4.0);
        assert_eq!(result.y(), 6.0);
    }

    #[test]
    fn test_point_sub() {
        let p1 = Point::new(5.0, 8.0);
        let p2 = Point::new(2.0, 3.0);
        let result = p1.sub_point(p2);
        assert_eq!(result.x(), 3.0);
        assert_eq!(result.y(), 5.0);
    }

    #[test]
    fn test_point_midpoint() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(4.0, 6.0);
        let midpoint = p1.midpoint(p2);
        assert_eq!(midpoint.x(), 2.0);
        assert_eq!(midpoint.y(), 3.0);
    }

    #[test]
    fn test_point_hypot() {
        let point = Point::new(3.0, 4.0);
        assert_eq!(point.hypot(), 5.0);

        let origin = Point::new(0.0, 0.0);
        assert_eq!(origin.hypot(), 0.0);
    }

    #[test]
    fn test_point_scale() {
        let point = Point::new(2.0, 3.0);
        let scaled = point.scale(2.5);
        assert_eq!(scaled.x(), 5.0);
        assert_eq!(scaled.y(), 7.5);
    }

    #[test]
    fn test_point_abs() {
        let point = Point::new(-2.5, 3.0);
        let abs_point = point.abs();
        assert_eq!(abs_point.x(), 2.5);
        assert_eq!(abs_point.y(), 3.0);

        let point2 = Point::new(1.0, -4.0);
        let abs_point2 = point2.abs();
        assert_eq!(abs_point2.x(), 1.0);
        assert_eq!(abs_point2.y(), 4.0);
    }

    #[test]
    fn test_point_to_bounds() {
        let center = Point::new(10.0, 20.0);
        let size = Size::new(6.0, 8.0);
        let bounds = center.to_bounds(size);

        assert_eq!(bounds.min_x(), 7.0); // 10 - 3
        assert_eq!(bounds.min_y(), 16.0); // 20 - 4
        assert_eq!(bounds.max_x(), 13.0); // 10 + 3
        assert_eq!(bounds.max_y(), 24.0); // 20 + 4
    }

    #[test]
    fn test_bounds_new_from_center() {
        let center = Point::new(50.0, 60.0);
        let size = Size::new(20.0, 30.0);
        let bounds = Bounds::new_from_center(center, size);

        // Center at (50, 60), size (20, 30)
        // min_x = 50 - 10 = 40, max_x = 50 + 10 = 60
        // min_y = 60 - 15 = 45, max_y = 60 + 15 = 75
        assert_eq!(bounds.min_x(), 40.0);
        assert_eq!(bounds.min_y(), 45.0);
        assert_eq!(bounds.max_x(), 60.0);
        assert_eq!(bounds.max_y(), 75.0);
        assert_eq!(bounds.width(), 20.0);
        assert_eq!(bounds.height(), 30.0);
        assert_eq!(bounds.center(), center);
    }

    #[test]
    fn test_bounds_new_from_center_zero_size() {
        let center = Point::new(10.0, 20.0);
        let size = Size::new(0.0, 0.0);
        let bounds = Bounds::new_from_center(center, size);

        assert_eq!(bounds.min_x(), 10.0);
        assert_eq!(bounds.min_y(), 20.0);
        assert_eq!(bounds.max_x(), 10.0);
        assert_eq!(bounds.max_y(), 20.0);
        assert_eq!(bounds.width(), 0.0);
        assert_eq!(bounds.height(), 0.0);
    }

    #[test]
    fn test_bounds_new_from_top_left() {
        let top_left = Point::new(10.0, 20.0);
        let size = Size::new(30.0, 40.0);
        let bounds = Bounds::new_from_top_left(top_left, size);

        // Top-left at (10, 20), size (30, 40)
        // min_x = 10, max_x = 10 + 30 = 40
        // min_y = 20, max_y = 20 + 40 = 60
        assert_eq!(bounds.min_x(), 10.0);
        assert_eq!(bounds.min_y(), 20.0);
        assert_eq!(bounds.max_x(), 40.0);
        assert_eq!(bounds.max_y(), 60.0);
        assert_eq!(bounds.width(), 30.0);
        assert_eq!(bounds.height(), 40.0);
        assert_eq!(bounds.min_point(), top_left);
    }

    #[test]
    fn test_bounds_new_from_top_left_zero_size() {
        let top_left = Point::new(5.0, 15.0);
        let size = Size::new(0.0, 0.0);
        let bounds = Bounds::new_from_top_left(top_left, size);

        assert_eq!(bounds.min_x(), 5.0);
        assert_eq!(bounds.min_y(), 15.0);
        assert_eq!(bounds.max_x(), 5.0);
        assert_eq!(bounds.max_y(), 15.0);
        assert_eq!(bounds.width(), 0.0);
        assert_eq!(bounds.height(), 0.0);
    }

    #[test]
    fn test_size_new() {
        let size = Size::new(100.0, 200.0);
        assert_eq!(size.width(), 100.0);
        assert_eq!(size.height(), 200.0);
    }

    #[test]
    fn test_size_default() {
        let size = Size::default();
        assert_eq!(size.width(), 0.0);
        assert_eq!(size.height(), 0.0);
    }

    #[test]
    fn test_size_max() {
        let size1 = Size::new(10.0, 20.0);
        let size2 = Size::new(15.0, 18.0);
        let max_size = size1.max(size2);

        assert_eq!(max_size.width(), 15.0);
        assert_eq!(max_size.height(), 20.0);
    }

    #[test]
    fn test_size_add_padding() {
        let size = Size::new(10.0, 20.0);
        let padded = size.add_padding(Insets::uniform(5.0));

        assert_eq!(padded.width(), 20.0); // 10 + 5*2
        assert_eq!(padded.height(), 30.0); // 20 + 5*2
    }

    #[test]
    fn test_size_scale() {
        let size = Size::new(10.0, 20.0);

        // Test positive scaling
        let scaled = size.scale(2.0);
        assert_eq!(scaled.width(), 20.0);
        assert_eq!(scaled.height(), 40.0);

        // Test fractional scaling
        let scaled_half = size.scale(0.5);
        assert_eq!(scaled_half.width(), 5.0);
        assert_eq!(scaled_half.height(), 10.0);

        // Test zero scaling
        let scaled_zero = size.scale(0.0);
        assert_eq!(scaled_zero.width(), 0.0);
        assert_eq!(scaled_zero.height(), 0.0);

        // Test negative scaling
        let scaled_neg = size.scale(-1.0);
        assert_eq!(scaled_neg.width(), -10.0);
        assert_eq!(scaled_neg.height(), -20.0);

        // Test scaling by 1 (identity)
        let scaled_one = size.scale(1.0);
        assert_eq!(scaled_one.width(), size.width());
        assert_eq!(scaled_one.height(), size.height());
    }

    #[test]
    fn test_bounds_accessors() {
        let bounds = Bounds {
            min_x: 1.0,
            min_y: 2.0,
            max_x: 5.0,
            max_y: 8.0,
        };

        assert_eq!(bounds.min_x(), 1.0);
        assert_eq!(bounds.min_y(), 2.0);
        assert_eq!(bounds.max_x(), 5.0);
        assert_eq!(bounds.max_y(), 8.0);
    }

    #[test]
    fn test_bounds_with_max_y() {
        let bounds = Bounds {
            min_x: 2.0,
            min_y: 5.0,
            max_x: 10.0,
            max_y: 12.0,
        };

        let new_bounds = bounds.with_max_y(15.0);
        assert_eq!(
            new_bounds,
            Bounds {
                min_x: 2.0,
                min_y: 5.0,
                max_x: 10.0,
                max_y: 15.0,
            }
        );
    }

    #[test]
    fn test_bounds_dimensions() {
        let bounds = Bounds {
            min_x: 2.0,
            min_y: 3.0,
            max_x: 7.0,
            max_y: 11.0,
        };

        assert_eq!(bounds.width(), 5.0);
        assert_eq!(bounds.height(), 8.0);
    }

    #[test]
    fn test_bounds_min_point() {
        let bounds = Bounds {
            min_x: 2.0,
            min_y: 3.0,
            max_x: 7.0,
            max_y: 11.0,
        };

        let min_point = bounds.min_point();
        assert_eq!(min_point.x(), 2.0);
        assert_eq!(min_point.y(), 3.0);
    }

    #[test]
    fn test_bounds_to_size() {
        let bounds = Bounds {
            min_x: 1.0,
            min_y: 2.0,
            max_x: 6.0,
            max_y: 9.0,
        };

        let size = bounds.to_size();
        assert_eq!(size.width(), 5.0);
        assert_eq!(size.height(), 7.0);
    }

    #[test]
    fn test_bounds_merge() {
        let bounds1 = Bounds {
            min_x: 1.0,
            min_y: 2.0,
            max_x: 5.0,
            max_y: 6.0,
        };

        let bounds2 = Bounds {
            min_x: 3.0,
            min_y: 0.0,
            max_x: 8.0,
            max_y: 4.0,
        };

        let merged = bounds1.merge(&bounds2);
        assert_eq!(merged.min_x(), 1.0);
        assert_eq!(merged.min_y(), 0.0);
        assert_eq!(merged.max_x(), 8.0);
        assert_eq!(merged.max_y(), 6.0);
    }

    #[test]
    fn test_bounds_translate() {
        let bounds = Bounds {
            min_x: 1.0,
            min_y: 2.0,
            max_x: 5.0,
            max_y: 6.0,
        };

        let offset = Point::new(3.0, -1.0);
        let translated = bounds.translate(offset);

        assert_eq!(translated.min_x(), 4.0);
        assert_eq!(translated.min_y(), 1.0);
        assert_eq!(translated.max_x(), 8.0);
        assert_eq!(translated.max_y(), 5.0);
    }

    #[test]
    fn test_bounds_inverse_translate() {
        let bounds = Bounds {
            min_x: 5.0,
            min_y: 3.0,
            max_x: 9.0,
            max_y: 7.0,
        };

        let offset = Point::new(2.0, 1.0);
        let inverse_translated = bounds.inverse_translate(offset);

        assert_eq!(inverse_translated.min_x(), 3.0);
        assert_eq!(inverse_translated.min_y(), 2.0);
        assert_eq!(inverse_translated.max_x(), 7.0);
        assert_eq!(inverse_translated.max_y(), 6.0);
    }

    #[test]
    fn test_bounds_add_padding() {
        let bounds = Bounds {
            min_x: 2.0,
            min_y: 3.0,
            max_x: 6.0,
            max_y: 8.0,
        };

        let padded = bounds.add_padding(Insets::uniform(1.0));

        assert_eq!(padded.min_x(), 1.0);
        assert_eq!(padded.min_y(), 2.0);
        assert_eq!(padded.max_x(), 7.0);
        assert_eq!(padded.max_y(), 9.0);
    }

    #[test]
    fn test_bounds_default() {
        let bounds = Bounds::default();
        assert_eq!(bounds.min_x(), 0.0);
        assert_eq!(bounds.min_y(), 0.0);
        assert_eq!(bounds.max_x(), 0.0);
        assert_eq!(bounds.max_y(), 0.0);
    }

    #[test]
    fn test_component_bounds() {
        // We need to create a mock AST node for testing
        // Since we can't easily create ast::Node here, we'll test the bounds calculation logic
        // through Point::to_bounds which is the same implementation
        let position = Point::new(10.0, 15.0);
        let size = Size::new(8.0, 12.0);
        let bounds = position.to_bounds(size);

        // Verify bounds calculation (position as center)
        assert_eq!(bounds.min_x(), 6.0); // 10 - 4
        assert_eq!(bounds.min_y(), 9.0); // 15 - 6
        assert_eq!(bounds.max_x(), 14.0); // 10 + 4
        assert_eq!(bounds.max_y(), 21.0); // 15 + 6
        assert_eq!(bounds.width(), 8.0);
        assert_eq!(bounds.height(), 12.0);
    }

    #[test]
    fn test_edge_cases() {
        // Test with zero values
        let zero_point = Point::new(0.0, 0.0);
        let zero_size = Size::new(0.0, 0.0);
        let zero_bounds = zero_point.to_bounds(zero_size);

        assert_eq!(zero_bounds.width(), 0.0);
        assert_eq!(zero_bounds.height(), 0.0);

        // Test with negative values
        let neg_point = Point::new(-5.0, -3.0);
        let abs_neg = neg_point.abs();
        assert_eq!(abs_neg.x(), 5.0);
        assert_eq!(abs_neg.y(), 3.0);

        // Test scaling by zero
        let point = Point::new(10.0, 20.0);
        let scaled_zero = point.scale(0.0);
        assert!(scaled_zero.is_zero());

        // Test scaling by negative value
        let scaled_neg = point.scale(-1.0);
        assert_eq!(scaled_neg.x(), -10.0);
        assert_eq!(scaled_neg.y(), -20.0);
    }

    #[test]
    fn test_mathematical_properties() {
        let p1 = Point::new(3.0, 4.0);
        let p2 = Point::new(1.0, 2.0);

        // Test addition commutativity
        assert_eq!(p1.add_point(p2).x(), p2.add_point(p1).x());
        assert_eq!(p1.add_point(p2).y(), p2.add_point(p1).y());

        // Test subtraction
        let diff = p1.sub_point(p2);
        let sum_back = diff.add_point(p2);
        assert!((sum_back.x() - p1.x()).abs() < f32::EPSILON);
        assert!((sum_back.y() - p1.y()).abs() < f32::EPSILON);

        // Test midpoint properties
        let mid = p1.midpoint(p2);
        let dist1 = p1.sub_point(mid).hypot();
        let dist2 = p2.sub_point(mid).hypot();
        assert!((dist1 - dist2).abs() < f32::EPSILON);
    }

    #[test]
    fn test_insets_new() {
        let insets = Insets::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(insets.top(), 1.0);
        assert_eq!(insets.right(), 2.0);
        assert_eq!(insets.bottom(), 3.0);
        assert_eq!(insets.left(), 4.0);
    }

    #[test]
    fn test_bounds_add_insets() {
        let bounds = Bounds {
            min_x: 2.0,
            min_y: 3.0,
            max_x: 6.0,
            max_y: 8.0,
        };
        let insets = Insets::new(1.0, 2.0, 3.0, 4.0);
        let padded_custom = bounds.add_padding(insets);
        assert_eq!(padded_custom.min_x(), -2.0); // 2.0 - 4.0 (left)
        assert_eq!(padded_custom.min_y(), 2.0); // 3.0 - 1.0 (top)
        assert_eq!(padded_custom.max_x(), 8.0); // 6.0 + 2.0 (right)
        assert_eq!(padded_custom.max_y(), 11.0); // 8.0 + 3.0 (bottom)
    }

    #[test]
    fn test_insets_uniform() {
        let insets = Insets::uniform(5.0);
        assert_eq!(insets.top(), 5.0);
        assert_eq!(insets.right(), 5.0);
        assert_eq!(insets.bottom(), 5.0);
        assert_eq!(insets.left(), 5.0);
    }

    #[test]
    fn test_insets_default() {
        let insets = Insets::default();
        assert_eq!(insets.top(), 0.0);
        assert_eq!(insets.right(), 0.0);
        assert_eq!(insets.bottom(), 0.0);
        assert_eq!(insets.left(), 0.0);
    }

    #[test]
    fn test_insets_sums() {
        let insets = Insets::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(insets.horizontal_sum(), 6.0); // 2.0 + 4.0
        assert_eq!(insets.vertical_sum(), 4.0); // 1.0 + 3.0
    }

    #[test]
    fn test_size_is_zero() {
        // Test zero size
        let zero_size = Size::new(0.0, 0.0);
        assert!(zero_size.is_zero());

        // Test default size (should be zero)
        let default_size = Size::default();
        assert!(default_size.is_zero());

        // Test non-zero width
        let non_zero_width = Size::new(1.0, 0.0);
        assert!(!non_zero_width.is_zero());

        // Test non-zero height
        let non_zero_height = Size::new(0.0, 1.0);
        assert!(!non_zero_height.is_zero());

        // Test both non-zero
        let non_zero_both = Size::new(5.0, 3.0);
        assert!(!non_zero_both.is_zero());

        // Test negative values (should not be zero)
        let negative_size = Size::new(-1.0, -1.0);
        assert!(!negative_size.is_zero());

        // Test mixed positive/negative
        let mixed_size = Size::new(-1.0, 0.0);
        assert!(!mixed_size.is_zero());
    }

    #[test]
    fn test_insets_with_top() {
        let original = Insets::new(1.0, 2.0, 3.0, 4.0);

        // Test basic with_top functionality
        let modified = original.with_top(10.0);
        assert_eq!(modified.top(), 10.0);
        assert_eq!(modified.right(), 2.0); // Should remain unchanged
        assert_eq!(modified.bottom(), 3.0); // Should remain unchanged
        assert_eq!(modified.left(), 4.0); // Should remain unchanged

        // Test with zero value
        let with_zero = original.with_top(0.0);
        assert_eq!(with_zero.top(), 0.0);
        assert_eq!(with_zero.right(), 2.0);
        assert_eq!(with_zero.bottom(), 3.0);
        assert_eq!(with_zero.left(), 4.0);

        // Test with negative value
        let with_negative = original.with_top(-5.0);
        assert_eq!(with_negative.top(), -5.0);
        assert_eq!(with_negative.right(), 2.0);
        assert_eq!(with_negative.bottom(), 3.0);
        assert_eq!(with_negative.left(), 4.0);

        // Test chaining (returns new instance)
        let original_copy = Insets::new(1.0, 2.0, 3.0, 4.0);
        let chained = original_copy.with_top(15.0).with_top(20.0);
        assert_eq!(chained.top(), 20.0);
        assert_eq!(chained.right(), 2.0);
        assert_eq!(chained.bottom(), 3.0);
        assert_eq!(chained.left(), 4.0);

        // Test that original remains unchanged
        assert_eq!(original.top(), 1.0);
        assert_eq!(original.right(), 2.0);
        assert_eq!(original.bottom(), 3.0);
        assert_eq!(original.left(), 4.0);

        // Test with uniform insets
        let uniform = Insets::uniform(5.0);
        let uniform_modified = uniform.with_top(10.0);
        assert_eq!(uniform_modified.top(), 10.0);
        assert_eq!(uniform_modified.right(), 5.0);
        assert_eq!(uniform_modified.bottom(), 5.0);
        assert_eq!(uniform_modified.left(), 5.0);
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

    fn bounds_strategy() -> impl Strategy<Value = Bounds> {
        (
            -1000.0f32..1000.0,
            -1000.0f32..1000.0,
            1.0f32..500.0,
            1.0f32..500.0,
        )
            .prop_map(|(x, y, w, h)| Bounds::new_from_top_left(Point::new(x, y), Size::new(w, h)))
    }

    fn size_strategy() -> impl Strategy<Value = Size> {
        (0.0f32..1000.0, 0.0f32..1000.0).prop_map(|(w, h)| Size::new(w, h))
    }

    fn point_strategy() -> impl Strategy<Value = Point> {
        (-1000.0f32..1000.0, -1000.0f32..1000.0).prop_map(|(x, y)| Point::new(x, y))
    }

    fn scale_strategy() -> impl Strategy<Value = f32> {
        0.1f32..10.0
    }

    // ===================
    // Property Test Functions
    // ===================

    /// Point addition should be commutative: p1 + p2 == p2 + p1.
    fn check_point_add_is_commutative(p1: Point, p2: Point) -> Result<(), TestCaseError> {
        let result1 = p1.add_point(p2);
        let result2 = p2.add_point(p1);

        prop_assert!(approx_eq!(f32, result1.x(), result2.x()));
        prop_assert!(approx_eq!(f32, result1.y(), result2.y()));
        Ok(())
    }

    /// Midpoint should always be between (or equal to) both points.
    fn check_midpoint_is_between_points(p1: Point, p2: Point) -> Result<(), TestCaseError> {
        let mid = p1.midpoint(p2);

        let min_x = p1.x().min(p2.x());
        let max_x = p1.x().max(p2.x());
        let min_y = p1.y().min(p2.y());
        let max_y = p1.y().max(p2.y());

        prop_assert!(mid.x() >= min_x && mid.x() <= max_x);
        prop_assert!(mid.y() >= min_y && mid.y() <= max_y);
        Ok(())
    }

    /// Scaling then unscaling should return approximately the original point.
    fn check_scale_inverse_roundtrip(p: Point, scale: f32) -> Result<(), TestCaseError> {
        let scaled = p.scale(scale);
        let unscaled = scaled.scale(1.0 / scale);

        prop_assert!(approx_eq!(f32, unscaled.x(), p.x()));
        prop_assert!(approx_eq!(f32, unscaled.y(), p.y()));
        Ok(())
    }

    /// Adding then subtracting a point should return the original.
    fn check_add_sub_inverse(p1: Point, p2: Point) -> Result<(), TestCaseError> {
        let result = p1.add_point(p2).sub_point(p2);

        prop_assert!(approx_eq!(f32, result.x(), p1.x(), epsilon = 0.001));
        prop_assert!(approx_eq!(f32, result.y(), p1.y(), epsilon = 0.001));
        Ok(())
    }

    /// Bounds merge should be commutative: a.merge(b) == b.merge(a).
    fn check_bounds_merge_is_commutative(b1: Bounds, b2: Bounds) -> Result<(), TestCaseError> {
        let merged1 = b1.merge(&b2);
        let merged2 = b2.merge(&b1);

        prop_assert!(approx_eq!(f32, merged1.min_x(), merged2.min_x()));
        prop_assert!(approx_eq!(f32, merged1.min_y(), merged2.min_y()));
        prop_assert!(approx_eq!(f32, merged1.max_x(), merged2.max_x()));
        prop_assert!(approx_eq!(f32, merged1.max_y(), merged2.max_y()));
        Ok(())
    }

    /// Bounds merge should be associative: (a.merge(b)).merge(c) == a.merge(b.merge(c)).
    fn check_bounds_merge_is_associative(
        b1: Bounds,
        b2: Bounds,
        b3: Bounds,
    ) -> Result<(), TestCaseError> {
        let left_assoc = b1.merge(&b2).merge(&b3);
        let right_assoc = b1.merge(&b2.merge(&b3));

        prop_assert!(approx_eq!(f32, left_assoc.min_x(), right_assoc.min_x()));
        prop_assert!(approx_eq!(f32, left_assoc.min_y(), right_assoc.min_y()));
        prop_assert!(approx_eq!(f32, left_assoc.max_x(), right_assoc.max_x()));
        prop_assert!(approx_eq!(f32, left_assoc.max_y(), right_assoc.max_y()));
        Ok(())
    }

    /// Merged bounds should contain both original bounds.
    fn check_bounds_merge_contains_both(b1: Bounds, b2: Bounds) -> Result<(), TestCaseError> {
        let merged = b1.merge(&b2);

        // Merged bounds should contain b1
        prop_assert!(merged.min_x() <= b1.min_x() + 0.001);
        prop_assert!(merged.min_y() <= b1.min_y() + 0.001);
        prop_assert!(merged.max_x() >= b1.max_x() - 0.001);
        prop_assert!(merged.max_y() >= b1.max_y() - 0.001);

        // Merged bounds should contain b2
        prop_assert!(merged.min_x() <= b2.min_x() + 0.001);
        prop_assert!(merged.min_y() <= b2.min_y() + 0.001);
        prop_assert!(merged.max_x() >= b2.max_x() - 0.001);
        prop_assert!(merged.max_y() >= b2.max_y() - 0.001);
        Ok(())
    }

    /// Translating then inverse translating should return the original bounds.
    fn check_translate_inverse_roundtrip(
        bounds: Bounds,
        offset: Point,
    ) -> Result<(), TestCaseError> {
        let roundtrip = bounds.translate(offset).inverse_translate(offset);

        prop_assert!(approx_eq!(
            f32,
            roundtrip.min_x(),
            bounds.min_x(),
            epsilon = 0.001
        ));
        prop_assert!(approx_eq!(
            f32,
            roundtrip.min_y(),
            bounds.min_y(),
            epsilon = 0.001
        ));
        prop_assert!(approx_eq!(
            f32,
            roundtrip.max_x(),
            bounds.max_x(),
            epsilon = 0.001
        ));
        prop_assert!(approx_eq!(
            f32,
            roundtrip.max_y(),
            bounds.max_y(),
            epsilon = 0.001
        ));
        Ok(())
    }

    /// Size max should be commutative: a.max(b) == b.max(a).
    fn check_size_max_is_commutative(s1: Size, s2: Size) -> Result<(), TestCaseError> {
        let max1 = s1.max(s2);
        let max2 = s2.max(s1);

        prop_assert!(approx_eq!(f32, max1.width(), max2.width()));
        prop_assert!(approx_eq!(f32, max1.height(), max2.height()));
        Ok(())
    }

    /// Size max should be idempotent: a.max(a) == a.
    fn check_size_max_is_idempotent(s: Size) -> Result<(), TestCaseError> {
        let max_self = s.max(s);

        prop_assert!(approx_eq!(f32, max_self.width(), s.width()));
        prop_assert!(approx_eq!(f32, max_self.height(), s.height()));
        Ok(())
    }

    // ===================
    // Proptest Wrappers
    // ===================

    proptest! {
        #[test]
        fn point_add_is_commutative(p1 in point_strategy(), p2 in point_strategy()) {
            check_point_add_is_commutative(p1, p2)?;
        }

        #[test]
        fn midpoint_is_between_points(p1 in point_strategy(), p2 in point_strategy()) {
            check_midpoint_is_between_points(p1, p2)?;
        }

        #[test]
        fn scale_inverse_roundtrip(p in point_strategy(), scale in scale_strategy()) {
            check_scale_inverse_roundtrip(p, scale)?;
        }

        #[test]
        fn add_sub_inverse(p1 in point_strategy(), p2 in point_strategy()) {
            check_add_sub_inverse(p1, p2)?;
        }

        #[test]
        fn bounds_merge_is_commutative(b1 in bounds_strategy(), b2 in bounds_strategy()) {
            check_bounds_merge_is_commutative(b1, b2)?;
        }

        #[test]
        fn bounds_merge_is_associative(b1 in bounds_strategy(), b2 in bounds_strategy(), b3 in bounds_strategy()) {
            check_bounds_merge_is_associative(b1, b2, b3)?;
        }

        #[test]
        fn bounds_merge_contains_both(b1 in bounds_strategy(), b2 in bounds_strategy()) {
            check_bounds_merge_contains_both(b1, b2)?;
        }

        #[test]
        fn translate_inverse_roundtrip(bounds in bounds_strategy(), offset in point_strategy()) {
            check_translate_inverse_roundtrip(bounds, offset)?;
        }

        #[test]
        fn size_max_is_commutative(s1 in size_strategy(), s2 in size_strategy()) {
            check_size_max_is_commutative(s1, s2)?;
        }

        #[test]
        fn size_max_is_idempotent(s in size_strategy()) {
            check_size_max_is_idempotent(s)?;
        }
    }
}
