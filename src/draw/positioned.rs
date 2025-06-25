//! Provides `PositionedDrawable`, a wrapper for a Drawable and its absolute position.

use crate::draw::Drawable;
use crate::geometry::Point;

/// A drawable object together with an absolute position.
///
/// Calls `render_to_svg` on the wrapped drawable, passing in the stored position.
pub struct PositionedDrawable {
    drawable: Box<dyn Drawable>,
    position: Point,
}

impl PositionedDrawable {
    /// Construct a new `PositionedDrawable` from a drawable (position defaults to zero).
    pub fn new<D: Drawable + 'static>(drawable: D) -> Self {
        Self {
            drawable: Box::new(drawable),
            position: Point::default(),
        }
    }

    /// Set the position for this drawable (builder style).
    pub fn with_position(mut self, position: Point) -> Self {
        self.position = position;
        self
    }

    /// Render this positioned drawable to SVG, using the inner drawable's implementation.
    pub fn render_to_svg(&self) -> Box<dyn svg::Node> {
        self.drawable.render_to_svg(self.position)
    }
}
