//! Provides `PositionedDrawable`, a wrapper for a Drawable and its absolute position.

use crate::{
    draw::Drawable,
    geometry::{Bounds, Point, Size},
};

/// A drawable object together with an absolute position.
///
/// Calls `render_to_svg` on the wrapped drawable, passing in the stored position.
#[derive(Debug, Clone)]
pub struct PositionedDrawable<D: Drawable> {
    drawable: D,
    position: Point,
}

impl<D: Drawable> PositionedDrawable<D> {
    /// Construct a new `PositionedDrawable` from a drawable (position defaults to zero).
    pub fn new(drawable: D) -> Self {
        Self {
            drawable,
            position: Point::default(),
        }
    }

    /// Set the position for this drawable (builder style).
    pub fn with_position(mut self, position: Point) -> Self {
        self.position = position;
        self
    }

    /// Render this positioned drawable to SVG, using the inner drawable\'s implementation.
    pub fn render_to_svg(&self) -> Box<dyn svg::Node> {
        self.drawable.render_to_svg(self.position)
    }

    /// Calculate the bounds of this positioned drawable.
    pub fn bounds(&self) -> Bounds {
        self.position.to_bounds(self.drawable.size())
    }

    /// Get a reference to the inner drawable
    pub fn inner(&self) -> &D {
        &self.drawable
    }

    /// Get the position of this drawable
    pub fn position(&self) -> Point {
        self.position
    }
}

impl<D: Drawable> Drawable for PositionedDrawable<D> {
    fn render_to_svg(&self, _position: Point) -> Box<dyn svg::Node> {
        // Ignore the passed position and use our stored position
        self.render_to_svg()
    }

    fn size(&self) -> Size {
        self.drawable.size()
    }
}
