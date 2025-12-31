//! Drawable Components for Diagram Rendering
//!
//! This module provides drawable abstractions for rendering various diagram elements.
//! All drawable components implement the [`Drawable`] trait, which provides a consistent
//! interface for rendering to layered SVG output and calculating size.
//!
//! # Layer-Based Rendering
//!
//! Drawables render to one or more [`RenderLayer`]s, which are automatically ordered
//! during final SVG generation.
mod activation_box;
mod arrow;
mod arrow_with_text;
mod fragment;
mod layer;
mod lifeline;
mod note;
mod positioned;
mod shape;
mod shape_with_text;
mod stroke;
mod text;
mod text_positioning;

pub use activation_box::{ActivationBox, ActivationBoxDefinition};
pub use arrow::{Arrow, ArrowDefinition, ArrowDirection, ArrowDrawer, ArrowStyle};
pub use arrow_with_text::{ArrowWithText, ArrowWithTextDrawer};
pub use fragment::{Fragment, FragmentDefinition, FragmentSection};
pub use layer::{LayeredOutput, RenderLayer, SvgNode};
pub use lifeline::{Lifeline, LifelineDefinition};
pub use note::{Note, NoteDefinition};
pub use positioned::PositionedDrawable;
pub use shape::{
    ActorDefinition, BoundaryDefinition, ComponentDefinition, ControlDefinition, EntityDefinition,
    InterfaceDefinition, OvalDefinition, RectangleDefinition, Shape, ShapeDefinition,
};
pub use shape_with_text::ShapeWithText;
pub use stroke::{StrokeCap, StrokeDefinition, StrokeJoin, StrokeStyle};
pub use text::{Text, TextDefinition};

use crate::geometry::{Point, Size};

/// Trait for drawable diagram elements that can be rendered to SVG layers.
pub trait Drawable: std::fmt::Debug {
    /// Renders this drawable to one or more layers.
    ///
    /// Implementations should create SVG nodes and add them to appropriate layers
    /// in the returned [`LayeredOutput`]. Simple drawables typically emit to a single
    /// layer, while complex drawables can emit different elements to different layers
    /// for proper z-ordering.
    ///
    /// # Arguments
    ///
    /// * `position` - The position where this drawable should be rendered
    ///
    /// # Returns
    ///
    /// A [`LayeredOutput`] containing the SVG nodes organized by layer.
    fn render_to_layers(&self, position: Point) -> LayeredOutput;

    /// Returns the size of this drawable.
    fn size(&self) -> Size;
}
