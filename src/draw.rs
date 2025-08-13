//! Drawable Components for Diagram Rendering
//!
//! This module provides drawable abstractions for rendering various diagram elements.
//! All drawable components implement the [`Drawable`] trait, which provides a consistent
//! interface for rendering to SVG and calculating size.
mod activation_box;
mod arrow;
mod arrow_with_text;
mod group;
mod lifeline;
mod positioned;
mod shape;
mod shape_with_text;
mod text;
mod text_positioning;

pub use activation_box::{ActivationBox, ActivationBoxDefinition};
pub use arrow::{Arrow, ArrowDefinition, ArrowDirection, ArrowDrawer, ArrowStyle};
pub use arrow_with_text::{ArrowWithText, ArrowWithTextDrawer};
pub use lifeline::{Lifeline, LifelineDefinition};
pub use positioned::PositionedDrawable;
pub use shape::{
    ActorDefinition, BoundaryDefinition, ComponentDefinition, ControlDefinition, EntityDefinition,
    InterfaceDefinition, OvalDefinition, RectangleDefinition, Shape, ShapeDefinition,
};
pub use shape_with_text::ShapeWithText;
pub use text::{Text, TextDefinition};

use crate::geometry::{Point, Size};

pub trait Drawable: std::fmt::Debug {
    fn render_to_svg(&self, position: Point) -> Box<dyn svg::Node>;
    fn size(&self) -> Size;
}
