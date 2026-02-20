//! Lifeline drawable for sequence diagrams.
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
//! Lifelines use relative positioning for consistency with other drawables:
//! 1. The lifeline stores only its height (no absolute coordinates).
//! 2. Positioning is handled by wrapping with `PositionedDrawable`.
//! 3. The lifeline renders as a vertical line from `(0,0)` to `(0, height)`.

use std::rc::Rc;

use svg::node::element as svg_element;

use crate::{
    draw::{Drawable, LayeredOutput, RenderLayer, StrokeDefinition},
    geometry::{Point, Size},
};

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
    pub fn stroke(&self) -> &Rc<StrokeDefinition> {
        &self.stroke
    }

    /// Set stroke definition using Rc.
    pub fn set_stroke(&mut self, stroke: Rc<StrokeDefinition>) {
        self.stroke = stroke;
    }
}

impl Default for LifelineDefinition {
    fn default() -> Self {
        Self {
            stroke: Rc::new(StrokeDefinition::default_dashed()),
        }
    }
}

/// A drawable lifeline for sequence diagrams.
///
/// A vertical line (typically dashed) showing a participant's
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
    /// Creates a new Lifeline with the given definition and height.
    pub fn new(definition: Rc<LifelineDefinition>, height: f32) -> Self {
        Self { definition, height }
    }
}

impl Drawable for Lifeline {
    fn render_to_layers(&self, position: Point) -> LayeredOutput {
        let mut output = LayeredOutput::new();

        // The lifeline renders as a vertical line from the given position
        // extending downward by its height
        let line = svg_element::Line::new()
            .set("x1", position.x())
            .set("y1", position.y())
            .set("x2", position.x())
            .set("y2", position.y() + self.height)
            .set("fill-opacity", self.definition.stroke().color().alpha());

        let line = crate::apply_stroke!(line, self.definition.stroke());

        output.add_to_layer(RenderLayer::Lifeline, Box::new(line));
        output
    }

    fn size(&self) -> Size {
        // The lifeline has minimal width (just the stroke width) and its height
        Size::new(self.definition.stroke().width(), self.height)
    }
}

#[cfg(test)]
mod tests {
    use float_cmp::assert_approx_eq;

    use super::*;
    use crate::draw::StrokeStyle;

    #[test]
    fn test_lifeline_definition_new() {
        let custom_stroke = Rc::new(StrokeDefinition::default_solid());
        let def = LifelineDefinition::new(custom_stroke);

        assert_eq!(*def.stroke().style(), StrokeStyle::Solid);
        assert_approx_eq!(f32, def.stroke().width(), 2.0);
    }

    #[test]
    fn test_lifeline_definition_set_stroke() {
        let mut def = LifelineDefinition::default();

        assert_eq!(*def.stroke().style(), StrokeStyle::Dashed);

        let new_stroke = Rc::new(StrokeDefinition::default_solid());
        def.set_stroke(new_stroke);

        assert_eq!(*def.stroke().style(), StrokeStyle::Solid);
        assert_approx_eq!(f32, def.stroke().width(), 2.0);
    }

    #[test]
    fn test_lifeline_size() {
        let def = Rc::new(LifelineDefinition::default());
        let lifeline = Lifeline::new(def, 100.0);

        let size = lifeline.size();
        assert_approx_eq!(f32, size.width(), 1.0);
        assert_approx_eq!(f32, size.height(), 100.0);
    }

    #[test]
    fn test_lifeline_render_to_layers() {
        let def = Rc::new(LifelineDefinition::default());
        let lifeline = Lifeline::new(def, 200.0);

        let output = lifeline.render_to_layers(Point::new(50.0, 10.0));
        assert!(!output.is_empty());
    }
}
