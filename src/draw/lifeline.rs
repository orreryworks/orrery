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
    draw::Drawable,
    geometry::{Point, Size},
};
use std::rc::Rc;
use svg::{self, node::element as svg_element};

/// Styling configuration for lifelines in sequence diagrams.
///
/// This struct contains all visual properties needed to render lifelines,
/// including colors, line width, and dash pattern. It follows the same pattern
/// as other definition structs in the codebase (e.g., `ActivationBoxDefinition`, `ArrowDefinition`).
///
/// # Default Values
///
/// The default values match the original hardcoded implementation:
/// - `stroke_color`: black - Color of the lifeline
/// - `stroke_width`: 1.0 - Width of the lifeline in pixels
/// - `stroke_dasharray`: Some("4") - Dash pattern for the line (4px dashes)
#[derive(Debug, Clone)]
pub struct LifelineDefinition {
    /// The color of the lifeline stroke
    stroke_color: Color,
    /// The width of the lifeline in pixels
    stroke_width: f32,
    /// Optional dash pattern for the lifeline (e.g., "4" for 4px dashes, "5,3" for 5px dash, 3px gap)
    /// If None, the line will be solid
    stroke_dasharray: Option<String>,
}

impl LifelineDefinition {
    /// Creates a new LifelineDefinition with the given properties
    pub fn new(stroke_color: Color, stroke_width: f32, stroke_dasharray: Option<String>) -> Self {
        Self {
            stroke_color,
            stroke_width,
            stroke_dasharray,
        }
    }

    /// Returns the stroke color
    pub fn stroke_color(&self) -> &Color {
        &self.stroke_color
    }

    /// Returns the stroke width
    pub fn stroke_width(&self) -> f32 {
        self.stroke_width
    }

    /// Returns the stroke dash array pattern
    pub fn stroke_dasharray(&self) -> Option<&str> {
        self.stroke_dasharray.as_deref()
    }
}

impl Default for LifelineDefinition {
    fn default() -> Self {
        Self {
            stroke_color: Color::default(),
            stroke_width: 1.0,
            stroke_dasharray: Some("4".to_string()),
        }
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
        let mut line = svg_element::Line::new()
            .set("x1", position.x())
            .set("y1", position.y())
            .set("x2", position.x())
            .set("y2", position.y() + self.height)
            .set("stroke", self.definition.stroke_color().to_string())
            .set("stroke-opacity", self.definition.stroke_color().alpha())
            .set("fill-opacity", self.definition.stroke_color().alpha())
            .set("stroke-width", self.definition.stroke_width());

        // Add dash pattern if specified
        if let Some(dasharray) = self.definition.stroke_dasharray() {
            line = line.set("stroke-dasharray", dasharray);
        }

        Box::new(line)
    }

    fn size(&self) -> Size {
        // The lifeline has minimal width (just the stroke width) and its height
        Size::new(self.definition.stroke_width(), self.height)
    }
}
