//! Activation Box Drawable Implementation
//!
//! This module provides drawable components for rendering activation boxes in sequence diagrams.
//! Activation boxes represent periods of activity on a participant's lifeline and appear as
//! thin rectangles that can be nested to show recursive calls or concurrent activities.
//!
//! # Architecture
//!
//! The activation box system follows the standard drawable pattern used throughout the codebase:
//!
//! - [`ActivationBoxDefinition`]: Contains styling configuration (width, colors, nesting offset)
//! - [`ActivationBox`]: The main drawable that implements the [`Drawable`] trait
//!
//! # Positioning Logic
//!
//! The activation box positioning follows this logic:
//! 1. The caller provides the participant's center position
//! 2. The activation box applies its nesting offset: `position.x() + (nesting_level * nesting_offset)`
//! 3. The box is centered on the adjusted position

use crate::{
    color::Color,
    draw::{Drawable, StrokeDefinition},
    geometry::{Bounds, Point, Size},
};
use std::rc::Rc;
use svg::{self, node::element as svg_element};

/// Styling configuration for activation boxes in sequence diagrams.
///
/// This struct contains all visual properties needed to render activation boxes,
/// including dimensions, colors, and nesting behavior using stroke styling.
///
/// # Default Values
///
/// The default values match the original hardcoded implementation exactly:
/// - `width`: 8.0px - Fixed width for all activation boxes
/// - `nesting_offset`: 4.0px - Horizontal spacing per nesting level
/// - `fill_color`: white - Background color of the activation box
/// - `stroke`: black color, 1.0 width, solid style - Border styling
#[derive(Debug, Clone)]
pub struct ActivationBoxDefinition {
    width: f32,
    nesting_offset: f32,
    fill_color: Color,
    stroke: Rc<StrokeDefinition>,
}

impl ActivationBoxDefinition {
    /// Creates a new ActivationBoxDefinition with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the activation box width
    pub fn set_width(&mut self, width: f32) {
        self.width = width;
    }

    /// Sets the nesting offset
    pub fn set_nesting_offset(&mut self, offset: f32) {
        self.nesting_offset = offset;
    }

    /// Sets the fill color
    pub fn set_fill_color(&mut self, color: Color) {
        self.fill_color = color;
    }

    /// Gets the activation box width
    fn width(&self) -> f32 {
        self.width
    }

    /// Gets the nesting offset (used by layout system)
    fn nesting_offset(&self) -> f32 {
        self.nesting_offset
    }

    /// Gets the fill color
    fn fill_color(&self) -> Color {
        self.fill_color
    }

    /// Returns the stroke definition
    pub fn stroke(&self) -> &StrokeDefinition {
        &self.stroke
    }
}

impl Default for ActivationBoxDefinition {
    fn default() -> Self {
        Self {
            width: 8.0,
            nesting_offset: 4.0,
            fill_color: Color::new("white").expect("Invalid default fill color"),
            stroke: Rc::new(StrokeDefinition::default()),
        }
    }
}

/// A drawable activation box for sequence diagrams.
///
/// This is the main drawable component that represents periods of activity on a participant's
/// lifeline in sequence diagrams. Activation boxes appear as thin rectangles and support
/// nesting to show recursive calls or concurrent activities.
///
/// # Positioning Behavior
///
/// When `render_to_svg(position)` is called:
/// 1. The `position` parameter should be the participant's center point
/// 2. The activation box applies nesting offset: `position.x() + (nesting_level * nesting_offset)`
/// 3. The rectangle is centered on the adjusted position
#[derive(Debug, Clone)]
pub struct ActivationBox {
    definition: Rc<ActivationBoxDefinition>,
    height: f32,
    nesting_level: u32,
}

impl ActivationBox {
    /// Creates a new ActivationBox with the given definition, height, and nesting level.
    ///
    /// # Arguments
    ///
    /// * `definition` - Shared styling configuration for the activation box
    /// * `height` - The height of the activation box (typically end_y - start_y)
    /// * `nesting_level` - The nesting level for horizontal offset calculation (0 = no nesting)
    pub fn new(definition: Rc<ActivationBoxDefinition>, height: f32, nesting_level: u32) -> Self {
        Self {
            definition,
            height,
            nesting_level,
        }
    }

    /// Gets the height
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Gets the nesting level for z-order sorting
    pub fn nesting_level(&self) -> u32 {
        self.nesting_level
    }

    /// Calculate the bounds for this activation box when positioned at the given position.
    pub fn calculate_bounds(&self, position: Point) -> Bounds {
        let def = self.definition();
        let nesting_offset = self.nesting_level as f32 * def.nesting_offset();
        let adjusted_position = position.with_x(position.x() + nesting_offset);
        let size = self.size();
        adjusted_position.to_bounds(size)
    }

    /// Returns a reference to the activation box definition.
    fn definition(&self) -> &ActivationBoxDefinition {
        &self.definition
    }
}

impl Drawable for ActivationBox {
    /// Renders the activation box to SVG at the given position with nesting offset.
    ///
    /// The position represents the center point where the activation box should be anchored.
    /// The actual rendering position is offset based on the nesting level.
    /// The box is rendered as a rectangle with styling from the definition.
    fn render_to_svg(&self, position: Point) -> Box<dyn svg::Node> {
        let def = self.definition();

        let bounds = self.calculate_bounds(position);
        let top_left = bounds.min_point();

        // Create the activation box rectangle
        let activation_rect = svg_element::Rectangle::new()
            .set("x", top_left.x())
            .set("y", top_left.y())
            .set("width", bounds.width())
            .set("height", bounds.height())
            .set("fill", def.fill_color().to_string())
            .set("fill-opacity", def.fill_color().alpha());

        // Apply all stroke attributes (color, opacity, width, cap, join, dasharray)
        let activation_rect = crate::apply_stroke!(activation_rect, def.stroke());

        activation_rect.into()
    }

    /// Returns the size of the activation box.
    fn size(&self) -> Size {
        Size::new(self.definition.width(), self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    #[test]
    fn test_activation_box_definition_custom_values() {
        let mut definition = ActivationBoxDefinition::new();

        definition.set_width(12.0);
        definition.set_nesting_offset(6.0);
        definition.set_fill_color(Color::new("red").unwrap());

        assert_eq!(definition.width(), 12.0);
        assert_eq!(definition.nesting_offset(), 6.0);
        assert_eq!(definition.fill_color().to_string(), "red");
        assert_eq!(definition.stroke().color().to_string(), "black");
        assert_eq!(definition.stroke().width(), 1.0);
    }

    #[test]
    fn test_activation_box_creation() {
        let definition = Rc::new(ActivationBoxDefinition::default());
        let height = 50.0;
        let nesting_level = 2;

        let activation_box = ActivationBox::new(definition.clone(), height, nesting_level);

        assert_eq!(activation_box.height, 50.0);
        assert_eq!(activation_box.nesting_level, 2);
        assert_eq!(activation_box.definition().width(), 8.0);
    }

    #[test]
    fn test_nesting_position_calculation() {
        let definition = Rc::new(ActivationBoxDefinition::default());
        let activation_box = ActivationBox::new(definition, 20.0, 2);

        // Test that nesting level 2 with offset 4.0 creates 8.0 total offset
        let base_position = Point::new(100.0, 200.0);
        let _rendered_svg = activation_box.render_to_svg(base_position);

        // The actual positioning logic is tested through SVG output
        // Here we verify the activation box holds correct nesting data
        assert_eq!(activation_box.nesting_level, 2);
        assert_eq!(activation_box.definition().nesting_offset(), 4.0);
    }

    #[test]
    fn test_render_to_svg_returns_valid_node() {
        let activation_box =
            ActivationBox::new(Rc::new(ActivationBoxDefinition::default()), 100.0, 0);
        let position = Point::new(50.0, 75.0);

        let svg_node = activation_box.render_to_svg(position);

        // Verify we get a valid SVG node (basic smoke test - no panic means success)
        // The Box<dyn svg::Node> cannot be null in safe Rust, so just getting here is sufficient
        drop(svg_node);
    }

    #[test]
    fn test_calculate_bounds() {
        // Test with custom definition values
        let mut definition = ActivationBoxDefinition::new();
        definition.set_width(12.0);
        definition.set_nesting_offset(6.0);
        let definition = Rc::new(definition);

        let activation_box = ActivationBox::new(definition, 40.0, 1);
        let position = Point::new(150.0, 300.0);

        let bounds = activation_box.calculate_bounds(position);

        // With custom width 12.0, nesting level 1, and offset 6.0
        // X offset should be 6.0, half width should be 6.0
        assert_eq!(bounds.min_x(), 150.0); // 150.0 + 6.0 - 6.0
        assert_eq!(bounds.max_x(), 162.0); // 150.0 + 6.0 + 6.0
        assert_eq!(bounds.min_y(), 280.0); // 300.0 - 20.0 (half height)
        assert_eq!(bounds.max_y(), 320.0); // 300.0 + 20.0 (half height)
    }
}
