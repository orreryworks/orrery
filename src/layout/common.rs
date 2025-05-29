use crate::ast;

/// A trait for types that can calculate their own size
pub trait LayoutSizing {
    /// Calculate the size of this layout, possibly adding padding
    fn layout_size(&self) -> Size;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    /// Creates a new point with the specified coordinates
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Adds another point to this point, returning a new point
    pub fn add(self, other: Point) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }

    /// Converts a point and size into a bounds rectangle
    ///
    /// The point is treated as the center of the bounds, and the size
    /// is distributed equally in all directions around that center.
    pub fn to_bounds(self, size: Size) -> Bounds {
        let half_width = size.width / 2.0;
        let half_height = size.height / 2.0;

        Bounds {
            min_x: self.x - half_width,
            min_y: self.y - half_height,
            max_x: self.x + half_width,
            max_y: self.y + half_height,
        }
    }
}

/// Represents the dimensions of an element with width and height
#[derive(Debug, Clone, Copy, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn max(self, other: Size) -> Self {
        Self {
            width: self.width.max(other.width),
            height: self.height.max(other.height),
        }
    }

    pub fn add_padding(self, padding: f32) -> Self {
        Self {
            width: self.width + padding * 2.0,
            height: self.height + padding * 2.0,
        }
    }
}

/// Represents a rectangular bounding box with minimum and maximum coordinates
#[derive(Debug, Default)]
pub struct Bounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl Bounds {
    /// Returns the width of the bounds
    pub fn width(&self) -> f32 {
        self.max_x - self.min_x
    }

    /// Returns the height of the bounds
    pub fn height(&self) -> f32 {
        self.max_y - self.min_y
    }

    /// Returns the top-left corner as a Point
    pub fn min_point(&self) -> Point {
        Point {
            x: self.min_x,
            y: self.min_y,
        }
    }

    /// Converts bounds to a Size object
    pub fn to_size(&self) -> Size {
        Size {
            width: self.width(),
            height: self.height(),
        }
    }

    /// Merges two bounds to create a larger bounds that contains both
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }

    /// Moves the bounds by the specified offset
    ///
    /// This translates both the minimum and maximum coordinates by the given amount.
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
    #[allow(dead_code)]
    pub fn inverse_translate(&self, offset: Point) -> Self {
        Self {
            min_x: self.min_x - offset.x,
            min_y: self.min_y - offset.y,
            max_x: self.max_x - offset.x,
            max_y: self.max_y - offset.y,
        }
    }
}

/// Represents a diagram component with a reference to its AST node and positioning information
#[derive(Debug, Clone)]
pub struct Component<'a> {
    pub node: &'a ast::Node,
    pub position: Point,
    pub size: Size,
}

impl Component<'_> {
    /// Calculates the bounds of this component
    ///
    /// The position is treated as the center of the component,
    /// and the bounds extend half the width/height in each direction.
    pub fn bounds(&self) -> Bounds {
        self.position.to_bounds(self.size)
    }
}
