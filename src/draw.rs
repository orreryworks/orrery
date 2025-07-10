mod arrow;
mod arrow_with_text;
mod group;
mod positioned;
mod shape;
mod shape_with_text;
mod text;

pub use arrow::{Arrow, ArrowDefinition, ArrowDirection, ArrowDrawer, ArrowStyle};
pub use arrow_with_text::{ArrowWithText, ArrowWithTextDrawer};
pub use positioned::PositionedDrawable;
pub use shape::{
    BoundaryDefinition, ComponentDefinition, OvalDefinition, RectangleDefinition, Shape,
    ShapeDefinition,
};
pub use shape_with_text::ShapeWithText;
pub use text::{Text, TextDefinition};

use crate::geometry::{Point, Size};

pub trait Drawable: std::fmt::Debug {
    fn render_to_svg(&self, position: Point) -> Box<dyn svg::Node>;
    fn size(&self) -> Size;
}
