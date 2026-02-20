//! Stroke and line-style definitions.
//!
//! This module provides a unified, comprehensive stroke/line definition system for all drawable
//! elements.
//!
//! # Overview
//!
//! Exported types:
//! - [`StrokeDefinition`]: The main struct containing all stroke properties (color, width, style, cap, join)
//! - [`StrokeStyle`]: Enum defining line patterns (solid, dashed, dotted, etc.)
//! - [`StrokeCap`]: Enum defining how line endpoints are rendered (butt, round, square)
//! - [`StrokeJoin`]: Enum defining how line corners are rendered (miter, round, bevel)
//! - [`apply_stroke!`](crate::apply_stroke!): Macro for applying stroke attributes to SVG elements
//!
//! # Design Philosophy
//!
//! The stroke system follows SVG/CSS terminology and semantics for consistency with web
//! graphics standards. Provides both mutable (`set_*`) and immutable (`with_*`) APIs.
//!
//! # Quick Start
//!
//! ## Creating Strokes
//!
//! ```
//! use orrery_core::draw::{StrokeDefinition, StrokeStyle, StrokeCap, StrokeJoin};
//! use orrery_core::color::Color;
//!
//! // Simple solid stroke
//! let stroke = StrokeDefinition::solid(Color::new("black").unwrap(), 2.0);
//!
//! // Dashed stroke with custom cap
//! let mut stroke = StrokeDefinition::dashed(Color::new("blue").unwrap(), 1.5);
//! stroke.set_cap(StrokeCap::Round);
//!
//! // Custom dash pattern
//! let mut stroke = StrokeDefinition::new(Color::new("red").unwrap(), 2.0);
//! stroke.set_style(StrokeStyle::Custom("10,5,2,5".to_string()));
//! ```
//!
//! ## Applying to SVG Elements
//!
//! Use the [`apply_stroke!`](crate::apply_stroke!) macro to apply all stroke attributes at once:
//!
//! ```
//! use orrery_core::draw::StrokeDefinition;
//! use orrery_core::color::Color;
//! use svg::node::element as svg_element;
//!
//! let stroke = StrokeDefinition::solid(Color::new("black").unwrap(), 2.0);
//! let rect = svg_element::Rectangle::new()
//!     .set("x", 0)
//!     .set("y", 0);
//!
//! // Apply all stroke attributes (color, opacity, width, cap, join, dasharray)
//! let rect = orrery_core::apply_stroke!(rect, &stroke);
//! ```
//!
//! # SVG Attribute Mapping
//!
//! The stroke system maps directly to SVG attributes:
//!
//! | Rust Property | SVG Attribute | Example Values |
//! |--------------|---------------|----------------|
//! | `color` | `stroke`, `stroke-opacity` | `"#000000"`, `0.5` |
//! | `width` | `stroke-width` | `2.0` |
//! | `style` | `stroke-dasharray` | `"5,5"`, `"10,5,2,5"` |
//! | `cap` | `stroke-linecap` | `"butt"`, `"round"`, `"square"` |
//! | `join` | `stroke-linejoin` | `"miter"`, `"round"`, `"bevel"` |

use std::str::FromStr;

use crate::color::Color;

// =============================================================================
// Type Definitions
// =============================================================================

/// Defines the visual style of a stroke, including dash patterns.
///
/// This enum integrates both the concept of "style" (solid vs patterned) and
/// the specific dash pattern into a single type.
///
/// # SVG Mapping
///
/// Each variant maps to specific SVG `stroke-dasharray` values:
/// - `Solid`: No dasharray attribute
/// - `Dashed`: "5,5"
/// - `Dotted`: "2,3"
/// - `DashDot`: "10,5,2,5"
/// - `DashDotDot`: "10,5,2,5,2,5"
/// - `Custom(pattern)`: Uses the provided pattern string
#[derive(Debug, Default, Clone, PartialEq)]
pub enum StrokeStyle {
    /// Solid continuous line (default)
    #[default]
    Solid,
    /// Dashed line with equal dash and gap lengths (5px dash, 5px gap)
    Dashed,
    /// Dotted line with small dots (2px dot, 3px gap)
    Dotted,
    /// Dash-dot pattern (10px dash, 5px gap, 2px dot, 5px gap)
    DashDot,
    /// Dash-dot-dot pattern (10px dash, 5px gap, 2px dot, 5px gap, 2px dot, 5px gap)
    DashDotDot,
    /// Custom SVG dasharray pattern
    /// Format: comma or space-separated list of dash/gap lengths
    /// Example: "10,5,2,3" = 10px dash, 5px gap, 2px dash, 3px gap (repeating)
    Custom(String),
}

impl FromStr for StrokeStyle {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "solid" => Ok(Self::Solid),
            "dashed" => Ok(Self::Dashed),
            "dotted" => Ok(Self::Dotted),
            "dash-dot" | "dashdot" => Ok(Self::DashDot),
            "dash-dot-dot" | "dashdotdot" => Ok(Self::DashDotDot),
            // Any other value is treated as a custom dasharray pattern
            _ => Ok(Self::Custom(s.to_string())),
        }
    }
}

impl StrokeStyle {
    /// Returns the SVG dasharray value for this style, or None for solid lines
    pub fn to_svg_value(&self) -> Option<String> {
        match self {
            Self::Solid => None,
            Self::Dashed => Some("5,5".to_string()),
            Self::Dotted => Some("2,3".to_string()),
            Self::DashDot => Some("10,5,2,5".to_string()),
            Self::DashDotDot => Some("10,5,2,5,2,5".to_string()),
            Self::Custom(pattern) => Some(pattern.clone()),
        }
    }
}

/// Defines how line endpoints are rendered.
///
/// Maps directly to SVG `stroke-linecap` attribute values.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum StrokeCap {
    /// Flat cap at the exact endpoint (SVG default)
    #[default]
    Butt,
    /// Rounded cap extending beyond the endpoint by half the stroke width
    Round,
    /// Square cap extending beyond the endpoint by half the stroke width
    Square,
}

impl StrokeCap {
    /// Returns the SVG stroke-linecap value
    pub fn to_svg_value(&self) -> &'static str {
        match self {
            Self::Butt => "butt",
            Self::Round => "round",
            Self::Square => "square",
        }
    }
}

impl FromStr for StrokeCap {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "butt" => Ok(Self::Butt),
            "round" => Ok(Self::Round),
            "square" => Ok(Self::Square),
            _ => Err(format!(
                "invalid stroke cap `{s}`, valid values: butt, round, square"
            )),
        }
    }
}

/// Defines how line corners (joins) are rendered.
///
/// Maps directly to SVG `stroke-linejoin` attribute values.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum StrokeJoin {
    /// Sharp corner with mitered point (SVG default)
    #[default]
    Miter,
    /// Rounded corner
    Round,
    /// Beveled (cut-off) corner
    Bevel,
}

impl StrokeJoin {
    /// Returns the SVG stroke-linejoin value
    pub fn to_svg_value(&self) -> &'static str {
        match self {
            Self::Miter => "miter",
            Self::Round => "round",
            Self::Bevel => "bevel",
        }
    }
}

impl FromStr for StrokeJoin {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "miter" => Ok(Self::Miter),
            "round" => Ok(Self::Round),
            "bevel" => Ok(Self::Bevel),
            _ => Err(format!(
                "invalid stroke join `{s}`, valid values: miter, round, bevel"
            )),
        }
    }
}

/// A stroke definition for rendering lines and borders.
///
/// This struct consolidates all properties needed to render strokes across
/// different drawable elements, providing a consistent API.
///
/// # Fields
///
/// - `color`: The stroke color (required)
/// - `width`: The stroke width in pixels (required, f32 for sub-pixel precision)
/// - `style`: The stroke pattern (solid, dashed, etc.)
/// - `cap`: How line endpoints are rendered
/// - `join`: How line corners are rendered
///
/// # Examples
///
/// ```
/// use orrery_core::draw::{StrokeDefinition, StrokeStyle, StrokeCap, StrokeJoin};
/// use orrery_core::color::Color;
///
/// // Default stroke (black, 1px, solid)
/// let stroke = StrokeDefinition::default();
///
/// // Simple solid stroke
/// let stroke = StrokeDefinition::solid(Color::new("red").unwrap(), 2.0);
///
/// // Dashed stroke with rounded caps
/// let mut stroke = StrokeDefinition::dashed(Color::new("blue").unwrap(), 1.5);
/// stroke.set_cap(StrokeCap::Round);
///
/// // Fully customized stroke
/// let mut stroke = StrokeDefinition::new(Color::new("green").unwrap(), 3.0);
/// stroke.set_style(StrokeStyle::DashDot);
/// stroke.set_cap(StrokeCap::Round);
/// stroke.set_join(StrokeJoin::Round);
/// ```
#[derive(Debug, Clone)]
pub struct StrokeDefinition {
    color: Color,
    width: f32,
    style: StrokeStyle,
    cap: StrokeCap,
    join: StrokeJoin,
}

impl StrokeDefinition {
    /// Returns an owned default solid stroke (cloned from static).
    ///
    /// - Color: black
    /// - Width: 2.0
    /// - Style: Solid
    pub fn default_solid() -> Self {
        Self {
            color: Color::default(),
            width: 2.0,
            style: StrokeStyle::Solid,
            cap: StrokeCap::Butt,
            join: StrokeJoin::Miter,
        }
    }

    /// Returns an owned default dashed stroke (cloned from static).
    ///
    /// - Color: black
    /// - Width: 1.0
    /// - Style: Dashed
    pub fn default_dashed() -> Self {
        Self {
            color: Color::default(),
            width: 1.0,
            style: StrokeStyle::Dashed,
            cap: StrokeCap::Butt,
            join: StrokeJoin::Miter,
        }
    }

    /// Creates a new stroke with the given color and width.
    ///
    /// Other properties use their default values:
    /// - style: Solid
    /// - cap: Butt
    /// - join: Miter
    ///
    /// # Arguments
    ///
    /// * `color` - The stroke color
    /// * `width` - The stroke width in pixels
    ///
    /// # Examples
    ///
    /// ```
    /// use orrery_core::draw::StrokeDefinition;
    /// use orrery_core::color::Color;
    ///
    /// let stroke = StrokeDefinition::new(Color::new("black").unwrap(), 2.0);
    /// ```
    pub fn new(color: Color, width: f32) -> Self {
        Self {
            color,
            width,
            ..Self::default()
        }
    }

    /// Creates a solid stroke (convenience constructor).
    ///
    /// This is equivalent to `StrokeDefinition::new(color, width)` since solid is the default style.
    ///
    /// # Examples
    ///
    /// ```
    /// use orrery_core::draw::StrokeDefinition;
    /// use orrery_core::color::Color;
    ///
    /// let stroke = StrokeDefinition::solid(Color::new("black").unwrap(), 1.0);
    /// ```
    pub fn solid(color: Color, width: f32) -> Self {
        Self::new(color, width)
    }

    /// Creates a dashed stroke (convenience constructor).
    ///
    /// # Examples
    ///
    /// ```
    /// use orrery_core::draw::StrokeDefinition;
    /// use orrery_core::color::Color;
    ///
    /// let stroke = StrokeDefinition::dashed(Color::new("blue").unwrap(), 1.5);
    /// ```
    pub fn dashed(color: Color, width: f32) -> Self {
        let mut stroke = Self::new(color, width);
        stroke.set_style(StrokeStyle::Dashed);
        stroke
    }

    /// Creates a dotted stroke (convenience constructor).
    ///
    /// # Examples
    ///
    /// ```
    /// use orrery_core::draw::StrokeDefinition;
    /// use orrery_core::color::Color;
    ///
    /// let stroke = StrokeDefinition::dotted(Color::new("red").unwrap(), 1.0);
    /// ```
    pub fn dotted(color: Color, width: f32) -> Self {
        let mut stroke = Self::new(color, width);
        stroke.set_style(StrokeStyle::Dotted);
        stroke
    }

    /// Returns the stroke color.
    pub fn color(&self) -> Color {
        self.color
    }

    /// Returns the stroke width.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Returns the stroke style.
    pub fn style(&self) -> &StrokeStyle {
        &self.style
    }

    /// Returns the stroke cap style.
    pub fn cap(&self) -> StrokeCap {
        self.cap
    }

    /// Returns the stroke join style.
    pub fn join(&self) -> StrokeJoin {
        self.join
    }

    /// Sets the stroke color.
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    /// Sets the stroke width.
    pub fn set_width(&mut self, width: f32) {
        self.width = width;
    }

    /// Sets the stroke style.
    pub fn set_style(&mut self, style: StrokeStyle) {
        self.style = style;
    }

    /// Sets the stroke cap style.
    pub fn set_cap(&mut self, cap: StrokeCap) {
        self.cap = cap;
    }

    /// Sets the stroke join style.
    pub fn set_join(&mut self, join: StrokeJoin) {
        self.join = join;
    }
}

impl Default for StrokeDefinition {
    fn default() -> Self {
        Self {
            color: Color::default(),
            width: 1.0,
            style: StrokeStyle::default(),
            cap: StrokeCap::default(),
            join: StrokeJoin::default(),
        }
    }
}

/// Apply all stroke attributes to an SVG element.
///
/// This macro applies the complete stroke definition including color, opacity,
/// width, line cap, line join, and dash pattern (if not solid) to any SVG element.
///
/// # Examples
///
/// ```
/// use orrery_core::draw::StrokeDefinition;
/// use orrery_core::color::Color;
/// use svg::node::element as svg_element;
///
/// let stroke = StrokeDefinition::solid(Color::new("black").unwrap(), 2.0);
/// let rect = svg_element::Rectangle::new()
///     .set("x", 0)
///     .set("y", 0)
///     .set("width", 100)
///     .set("height", 50);
///
/// let rect = orrery_core::apply_stroke!(rect, &stroke);
/// ```
#[macro_export]
macro_rules! apply_stroke {
    ($element:expr, $stroke:expr) => {{
        let mut elem = $element
            .set("stroke", $stroke.color().to_string())
            .set("stroke-opacity", $stroke.color().alpha())
            .set("stroke-width", $stroke.width())
            .set("stroke-linecap", $stroke.cap().to_svg_value())
            .set("stroke-linejoin", $stroke.join().to_svg_value());

        if let Some(dasharray) = $stroke.style().to_svg_value() {
            elem = elem.set("stroke-dasharray", dasharray);
        }

        elem
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stroke_default() {
        let stroke = StrokeDefinition::default();
        assert_eq!(stroke.width(), 1.0);
        assert_eq!(stroke.color().to_string(), "black");
        assert_eq!(*stroke.style(), StrokeStyle::Solid);
        assert_eq!(stroke.cap(), StrokeCap::Butt);
        assert_eq!(stroke.join(), StrokeJoin::Miter);
    }

    #[test]
    fn test_stroke_constructors() {
        let color = Color::new("red").unwrap();

        let solid = StrokeDefinition::solid(color, 2.0);
        assert_eq!(solid.width(), 2.0);
        assert_eq!(*solid.style(), StrokeStyle::Solid);

        let dashed = StrokeDefinition::dashed(color, 1.5);
        assert_eq!(*dashed.style(), StrokeStyle::Dashed);

        let dotted = StrokeDefinition::dotted(color, 1.0);
        assert_eq!(*dotted.style(), StrokeStyle::Dotted);
    }

    #[test]
    fn test_stroke_setters_builder_style() {
        let mut stroke = StrokeDefinition::new(Color::new("blue").unwrap(), 3.0);
        stroke.set_style(StrokeStyle::DashDot);
        stroke.set_cap(StrokeCap::Round);
        stroke.set_join(StrokeJoin::Round);

        assert_eq!(stroke.width(), 3.0);
        assert_eq!(*stroke.style(), StrokeStyle::DashDot);
        assert_eq!(stroke.cap(), StrokeCap::Round);
        assert_eq!(stroke.join(), StrokeJoin::Round);
    }

    #[test]
    fn test_stroke_setters() {
        let mut stroke = StrokeDefinition::default();

        stroke.set_color(Color::new("green").unwrap());
        stroke.set_width(2.5);
        stroke.set_style(StrokeStyle::Dashed);
        stroke.set_cap(StrokeCap::Square);
        stroke.set_join(StrokeJoin::Bevel);

        assert_eq!(stroke.color().to_string(), "green");
        assert_eq!(stroke.width(), 2.5);
        assert_eq!(*stroke.style(), StrokeStyle::Dashed);
        assert_eq!(stroke.cap(), StrokeCap::Square);
        assert_eq!(stroke.join(), StrokeJoin::Bevel);
    }

    #[test]
    fn test_stroke_style_dasharray() {
        assert_eq!(StrokeStyle::Solid.to_svg_value(), None);
        assert_eq!(StrokeStyle::Dashed.to_svg_value(), Some("5,5".to_string()));
        assert_eq!(StrokeStyle::Dotted.to_svg_value(), Some("2,3".to_string()));
        assert_eq!(
            StrokeStyle::DashDot.to_svg_value(),
            Some("10,5,2,5".to_string())
        );
        assert_eq!(
            StrokeStyle::DashDotDot.to_svg_value(),
            Some("10,5,2,5,2,5".to_string())
        );

        let custom = StrokeStyle::Custom("15,3,3,3".to_string());
        assert_eq!(custom.to_svg_value(), Some("15,3,3,3".to_string()));
    }

    #[test]
    fn test_stroke_cap_svg_values() {
        assert_eq!(StrokeCap::Butt.to_svg_value(), "butt");
        assert_eq!(StrokeCap::Round.to_svg_value(), "round");
        assert_eq!(StrokeCap::Square.to_svg_value(), "square");
    }

    #[test]
    fn test_stroke_join_svg_values() {
        assert_eq!(StrokeJoin::Miter.to_svg_value(), "miter");
        assert_eq!(StrokeJoin::Round.to_svg_value(), "round");
        assert_eq!(StrokeJoin::Bevel.to_svg_value(), "bevel");
    }

    #[test]
    fn test_stroke_cap_from_str() {
        use std::str::FromStr;

        assert_eq!(StrokeCap::from_str("butt").unwrap(), StrokeCap::Butt);
        assert_eq!(StrokeCap::from_str("round").unwrap(), StrokeCap::Round);
        assert_eq!(StrokeCap::from_str("square").unwrap(), StrokeCap::Square);

        let result = StrokeCap::from_str("invalid");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid stroke cap"));
    }

    #[test]
    fn test_stroke_join_from_str() {
        use std::str::FromStr;

        assert_eq!(StrokeJoin::from_str("miter").unwrap(), StrokeJoin::Miter);
        assert_eq!(StrokeJoin::from_str("round").unwrap(), StrokeJoin::Round);
        assert_eq!(StrokeJoin::from_str("bevel").unwrap(), StrokeJoin::Bevel);

        let result = StrokeJoin::from_str("invalid");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid stroke join"));
    }

    #[test]
    fn test_stroke_style_from_str() {
        use std::str::FromStr;

        // Test valid style strings
        assert_eq!(StrokeStyle::from_str("solid").unwrap(), StrokeStyle::Solid);
        assert_eq!(
            StrokeStyle::from_str("dashed").unwrap(),
            StrokeStyle::Dashed
        );
        assert_eq!(
            StrokeStyle::from_str("dotted").unwrap(),
            StrokeStyle::Dotted
        );
        assert_eq!(
            StrokeStyle::from_str("dash-dot").unwrap(),
            StrokeStyle::DashDot
        );
        assert_eq!(
            StrokeStyle::from_str("dashdot").unwrap(),
            StrokeStyle::DashDot
        );
        assert_eq!(
            StrokeStyle::from_str("dash-dot-dot").unwrap(),
            StrokeStyle::DashDotDot
        );
        assert_eq!(
            StrokeStyle::from_str("dashdotdot").unwrap(),
            StrokeStyle::DashDotDot
        );

        // Test custom patterns (any unrecognized string becomes Custom)
        assert_eq!(
            StrokeStyle::from_str("10,5,2,5").unwrap(),
            StrokeStyle::Custom("10,5,2,5".to_string())
        );
        assert_eq!(
            StrokeStyle::from_str("5,5").unwrap(),
            StrokeStyle::Custom("5,5".to_string())
        );
        assert_eq!(
            StrokeStyle::from_str("arbitrary-pattern").unwrap(),
            StrokeStyle::Custom("arbitrary-pattern".to_string())
        );
    }
}
