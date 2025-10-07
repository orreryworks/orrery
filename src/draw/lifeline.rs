//! Lifeline Drawable Implementation
//!
//! This module provides drawable components for rendering lifelines in sequence diagrams.
//! Lifelines represent the existence of a participant over time and appear as vertical
//! dashed lines extending from the participant shape downward.
//!
//! # Architecture
//!
//! The lifeline system follows the standard drawable pattern used throughout the codebase:
//!
//! - [`LifelineDefinition`]: Contains styling configuration (color, width, dash pattern)
//! - [`Lifeline`]: The main drawable that implements the [`Drawable`] trait
//!
//! # Positioning Logic
//!
//! The lifeline uses relative positioning for consistency with other drawables:
//! 1. The lifeline stores only its height (no absolute coordinates)
//! 2. Positioning is handled by wrapping with PositionedDrawable
//! 3. The lifeline renders as a vertical line from (0,0) to (0,height)

use crate::{
    color::Color,
    draw::{Drawable, StrokeDefinition},
    geometry::{Point, Size},
};
use std::rc::Rc;
use svg::{self, node::element as svg_element};

/// Styling configuration for lifelines in sequence diagrams.
///
/// This struct contains all visual properties needed to render lifelines,
/// using a shared `StrokeDefinition` for consistent stroke styling.
///
/// # Default Values
///
/// The default values match the original hardcoded implementation:
/// - `stroke`: dashed style (4px dash pattern) with black color and 1.0 width
#[derive(Debug, Clone)]
pub struct LifelineDefinition {
    /// The stroke styling for the lifeline
    stroke: Rc<StrokeDefinition>,
}

impl LifelineDefinition {
    /// Creates a new LifelineDefinition with the given stroke definition
    pub fn new(stroke: Rc<StrokeDefinition>) -> Self {
        Self { stroke }
    }

    /// Returns the stroke definition
    pub fn stroke(&self) -> &StrokeDefinition {
        &self.stroke
    }
}

impl Default for LifelineDefinition {
    fn default() -> Self {
        Self::new(Rc::new(StrokeDefinition::dashed(Color::default(), 1.0)))
    }
}

/// A drawable lifeline for sequence diagrams.
///
/// Represents a vertical line (typically dashed) that shows a participant's
/// lifetime in the sequence. The lifeline is a simple vertical line with a
/// specified height, designed to be used with PositionedDrawable for
/// absolute positioning.
#[derive(Debug, Clone)]
pub struct Lifeline {
    /// The styling definition for this lifeline
    definition: Rc<LifelineDefinition>,
    /// The height of the lifeline
    height: f32,
}

impl Lifeline {
    /// Creates a new Lifeline with the given definition and height
    pub fn new(definition: Rc<LifelineDefinition>, height: f32) -> Self {
        Self { definition, height }
    }

    /// Creates a new Lifeline with default styling and specified height
    pub fn with_default_style(height: f32) -> Self {
        Self::new(Rc::new(LifelineDefinition::default()), height)
    }

    /// Returns the height of the lifeline
    pub fn height(&self) -> f32 {
        self.height
    }
}

impl Drawable for Lifeline {
    fn render_to_svg(&self, position: Point) -> Box<dyn svg::Node> {
        // The lifeline renders as a vertical line from the given position
        // extending downward by its height
        let line = svg_element::Line::new()
            .set("x1", position.x())
            .set("y1", position.y())
            .set("x2", position.x())
            .set("y2", position.y() + self.height)
            .set("fill-opacity", self.definition.stroke().color().alpha());

        let line = crate::apply_stroke!(line, self.definition.stroke());

        Box::new(line)
    }

    fn size(&self) -> Size {
        // The lifeline has minimal width (just the stroke width) and its height
        Size::new(self.definition.stroke().width(), self.height)
    }
}
