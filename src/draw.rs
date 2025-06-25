mod arrow;
mod group;
mod positioned;
mod shape;
mod text;

pub use arrow::{ArrowDefinition, ArrowStyle};
pub use group::Group;
pub use positioned::PositionedDrawable;
pub use shape::{OvalDefinition, RectangleDefinition, Shape, ShapeDefinition};
pub use text::{Text, TextDefinition};

use crate::geometry::Point;

/// Trait for rendering objects to SVG format
///
/// This trait provides a common interface for converting geometric objects
/// (shapes, text, etc.) into SVG elements that can be included in the final diagram.
pub trait Drawable {
    /// Render this object to an SVG node at the specified position
    fn render_to_svg(&self, position: Point) -> Box<dyn svg::Node>;
}
