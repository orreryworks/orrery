//! Text rendering definitions for diagram labels and content.
//!
//! This module provides types for configuring text appearance and rendering
//! text elements in diagrams. Text is rendered as SVG `<text>` elements with
//! optional background rectangles.
//!
//! # Overview
//!
//! - [`TextDefinition`] - Reusable text style configuration
//! - [`Text`] - A renderable text element combining content with a [`TextDefinition`]
//!
//! # Quick Start
//!
//! ```
//! # use orrery_core::draw::{TextDefinition, Text};
//! // Create a text style
//! let mut style = TextDefinition::new();
//! style.set_font_family("Helvetica");
//! style.set_font_size(14);
//!
//! // Create a text element
//! let text = Text::new(&style, "Hello, Diagram!");
//! let size = text.calculate_size();
//! assert!(size.width() > 0.0);
//! ```
//!
//! # Rendering
//!
//! When rendered via the [`Drawable`] trait, [`Text`] produces:
//! - An SVG `<text>` element on the [`Text`](crate::draw::RenderLayer::Text) layer
//! - An optional background on the
//!   [`Background`](crate::draw::RenderLayer::Background) layer (depending on
//!   the [`TextDefinition`] configuration)

use std::sync::{Arc, Mutex, OnceLock};

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping};
use log::info;
use svg::{self, node::Text as SvgText, node::element as svg_element};

use crate::{
    color::Color,
    draw::{Drawable, LayeredOutput, RenderLayer},
    geometry::{Insets, Point, Size},
};

// =============================================================================
// Static Default Definitions
// =============================================================================

/// Default text definition with standard settings.
///
/// - Font family: "sans-serif"
/// - Font size: 12
/// - Background color: None
/// - Text color: None (uses SVG default, typically black)
/// - Padding: 4px on all sides
///
/// Use with `Cow::Borrowed(&DEFAULT_TEXT)` for zero-allocation defaults.
static DEFAULT_TEXT: OnceLock<TextDefinition> = OnceLock::new();

// =============================================================================
// Type Definitions
// =============================================================================

/// Defines the visual style for text elements in diagrams.
///
/// `TextDefinition` configures font properties, colors, and padding for text
/// rendered in diagram nodes, labels, and annotations. Multiple [`Text`]
/// elements can share the same definition for consistent styling.
///
/// # Default Values
///
/// | Property | Default |
/// |----------|---------|
/// | Font family | `"Arial"` |
/// | Font size | `15` |
/// | Background color | `None` |
/// | Text color | `None` (SVG default, typically black) |
/// | Padding | Zero on all sides |
///
/// # Examples
///
/// ```
/// # use orrery_core::draw::TextDefinition;
/// # use orrery_core::color::Color;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut style = TextDefinition::new();
/// style.set_font_family("Helvetica");
/// style.set_font_size(14);
/// style.set_color(Some(Color::new("navy")?));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct TextDefinition {
    font_family: String,
    font_size: u16,
    background_color: Option<Color>,
    color: Option<Color>,
    padding: Insets,
}

impl TextDefinition {
    /// Returns a reference to the default text definition (borrowed from static).
    ///
    /// - Font family: "sans-serif"
    /// - Font size: 12
    /// - Background color: None
    /// - Text color: None
    /// - Padding: 4px on all sides
    pub fn default_borrowed() -> &'static Self {
        DEFAULT_TEXT.get_or_init(|| TextDefinition {
            font_family: String::from("sans-serif"),
            font_size: 12,
            background_color: None,
            color: None,
            padding: Insets::uniform(4.0),
        })
    }

    /// Creates a new text definition with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the font size in points.
    ///
    /// # Arguments
    ///
    /// * `size` - The font size in points.
    pub fn set_font_size(&mut self, size: u16) {
        self.font_size = size;
    }

    /// Sets the font family for the text.
    ///
    /// # Arguments
    ///
    /// * `family` - The font family name (e.g., "Arial", "Times New Roman", "monospace")
    pub fn set_font_family(&mut self, family: &str) {
        self.font_family = family.to_string();
    }

    /// Sets the background color for the text.
    ///
    /// When set to `Some(color)`, text will be rendered with a rounded rectangle background
    /// in the specified color. When set to `None`, no background will be rendered.
    ///
    /// # Arguments
    ///
    /// * `color` - Optional background color. Use `None` for no background.
    pub fn set_background_color(&mut self, color: Option<Color>) {
        self.background_color = color;
    }

    /// Sets the text color for the text content.
    ///
    /// When set to `Some(color)`, text will be rendered in the specified color.
    /// When set to `None`, the default text color (usually black) will be used.
    ///
    /// # Arguments
    ///
    /// * `color` - Optional text color. Use `None` for default color.
    pub fn set_color(&mut self, color: Option<Color>) {
        self.color = color;
    }

    /// Sets the padding around the text content.
    ///
    /// Padding affects the size of the background rectangle (if present) and creates
    /// space between the text and the background edges. Padding is applied even when
    /// no background color is set, affecting the overall size calculations.
    ///
    /// # Arguments
    ///
    /// * `padding` - The [`Insets`] defining padding on each side.
    pub fn set_padding(&mut self, padding: Insets) {
        self.padding = padding;
    }

    fn font_size(&self) -> u16 {
        self.font_size
    }

    fn font_family(&self) -> &str {
        &self.font_family
    }

    /// Returns a reference to the background color, if set.
    fn background_color(&self) -> Option<&Color> {
        self.background_color.as_ref()
    }

    /// Returns a reference to the text color, if set.
    fn color(&self) -> Option<&Color> {
        self.color.as_ref()
    }

    /// Returns the current padding configuration.
    fn padding(&self) -> Insets {
        self.padding
    }
}

impl Default for TextDefinition {
    fn default() -> Self {
        Self {
            font_size: 15,
            background_color: None,
            color: None,
            padding: Insets::default(),
            font_family: "Arial".to_string(),
        }
    }
}

/// A renderable text element combining content with styling.
///
/// `Text` pairs a string value with a [`TextDefinition`] to produce a
/// measurable and renderable text element. It is used for node labels,
/// edge annotations, and other textual content in diagrams.
///
/// # Examples
///
/// ```
/// # use orrery_core::draw::{TextDefinition, Text};
/// let style = TextDefinition::new();
/// let text = Text::new(&style, "Component");
///
/// // Measure the text
/// let size = text.calculate_size();
/// assert!(size.width() > 0.0);
/// assert!(size.height() > 0.0);
///
/// // Access the content
/// assert_eq!(text.content(), "Component");
/// ```
#[derive(Debug, Clone)]
pub struct Text<'a> {
    definition: &'a TextDefinition,
    content: &'a str,
}

impl<'a> Text<'a> {
    /// Creates a new text element with the given definition and content.
    ///
    /// # Arguments
    ///
    /// * `definition` - The [`TextDefinition`] controlling text appearance.
    /// * `content` - The text string to render.
    pub fn new(definition: &'a TextDefinition, content: &'a str) -> Self {
        Self {
            definition,
            content,
        }
    }

    /// Returns the text content of this element.
    pub fn content(&self) -> &str {
        self.content
    }

    /// Calculate the total size required to display this text, including padding.
    pub fn calculate_size(&self) -> Size {
        let padding = self.definition.padding();
        self.calculate_size_without_padding().add_padding(padding)
    }

    /// Calculate the size required to display this text content without padding.
    fn calculate_size_without_padding(&self) -> Size {
        TEXT_MANAGER
            .get_or_init(TextManager::new)
            .calculate_text_size(self.content, self.definition)
    }
}

impl<'a> Drawable for Text<'a> {
    fn render_to_layers(&self, position: Point) -> LayeredOutput {
        let mut output = LayeredOutput::new();
        let text_size = self.calculate_size();
        let padding = self.definition.padding();

        let lines: Vec<&str> = self.content.lines().collect();

        // Calculate uniform line height by dividing total height by line count
        let text_size_without_padding = self.calculate_size_without_padding();
        let line_height = if lines.is_empty() {
            0.0
        } else {
            text_size_without_padding.height() / lines.len() as f32
        };

        let total_height = text_size_without_padding.height();
        let y_offset = -(total_height + line_height) / 2.0;

        let mut rendered_text = svg_element::Text::new("")
            .set("x", position.x())
            .set("y", position.y() + y_offset)
            .set("text-anchor", "middle")
            .set("dominant-baseline", "central")
            .set("font-family", self.definition.font_family())
            .set("font-size", self.definition.font_size());

        // Set text color if specified
        if let Some(color) = self.definition.color() {
            rendered_text = rendered_text
                .set("fill", color.to_string())
                .set("fill-opacity", color.alpha());
        }

        for line in lines.into_iter() {
            let tspan = svg_element::TSpan::new("")
                .set("x", position.x())
                .set("dy", line_height)
                .add(SvgText::new(line));
            rendered_text = rendered_text.add(tspan);
        }

        // Add background rectangle if color is specified
        if let Some(bg_color) = self.definition.background_color() {
            let bg_bounds = position.to_bounds(text_size).add_padding(padding);
            let bg_size = bg_bounds.to_size();
            let bg_min_point = bg_bounds.min_point();

            let bg = svg_element::Rectangle::new()
                .set("x", bg_min_point.x())
                .set("y", bg_min_point.y())
                .set("width", bg_size.width())
                .set("height", bg_size.height())
                .set("fill", bg_color.to_string())
                .set("fill-opacity", bg_color.alpha())
                .set("rx", 3.0); // Slightly rounded corners

            output.add_to_layer(RenderLayer::Background, Box::new(bg));
        }

        output.add_to_layer(RenderLayer::Text, Box::new(rendered_text));
        output
    }

    fn size(&self) -> Size {
        self.calculate_size() // TODO: merge them.
    }
}

/// TextManager handles text measurement and font operations
/// It maintains a reusable FontSystem instance to avoid expensive recreation
struct TextManager {
    font_system: Arc<Mutex<FontSystem>>,
}

impl Default for TextManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TextManager {
    /// Create a new TextManager with a default FontSystem
    fn new() -> Self {
        info!("Initializing FontSystem");
        Self {
            font_system: Arc::new(Mutex::new(FontSystem::new())),
        }
    }

    /// Calculate the actual size of text in pixels using cosmic-text.
    ///
    /// This provides an accurate measurement based on real font metrics and shaping,
    /// including proper handling of ligatures, kerning, and other advanced typography features.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content to measure
    /// * `text_def` - Text definition containing font family, size, and other styling
    ///
    /// # Returns
    ///
    /// The calculated size in pixels, or default size if measurement fails
    fn calculate_text_size(&self, text: &str, text_def: &TextDefinition) -> Size {
        if text.is_empty() {
            return Size::default();
        }

        // Lock the FontSystem for use
        let mut font_system = self.font_system.lock().expect("failed to lock FontSystem");

        // Convert font size from points to pixels (roughly 1.33x multiplier for standard DPI)
        let font_size_px = text_def.font_size() as f32 * 1.33;

        // Create metrics with font size and approximate line height
        let line_height = font_size_px * 1.15;
        let metrics = Metrics::new(font_size_px, line_height);

        // Create a buffer with the metrics
        let mut buffer = Buffer::new(&mut font_system, metrics);
        let mut buffer = buffer.borrow_with(&mut font_system);

        // Set up text attributes
        let attrs = Attrs::new().family(Family::Name(text_def.font_family()));

        // Set the buffer's size to unlimited to allow text to flow naturally
        buffer.set_size(None, None);

        // Set the text with advanced shaping for accurate text metrics
        // Advanced shaping handles ligatures, kerning, etc.
        buffer.set_text(text, &attrs, Shaping::Advanced, None);

        // Shape the text to calculate layout
        buffer.shape_until_scroll(true);

        // Calculate bounds by examining layout runs to determine actual rendered size
        let mut max_width: f32 = 0.0;
        let mut total_height: f32 = 0.0;

        // Get height from line metrics or use default
        let layout_runs: Vec<_> = buffer.layout_runs().collect();
        if !layout_runs.is_empty() {
            for last in layout_runs.iter().map(|run| run.glyphs.last()) {
                // Find rightmost glyph position
                if let Some(last) = last {
                    let run_width = last.x + last.w;
                    max_width = max_width.max(run_width);
                }
                // Add line height for this run
                total_height += metrics.line_height;
            }
        } else {
            // Default size if no runs available
            max_width = text.len() as f32 * (font_size_px * 0.55);
            total_height = metrics.line_height;
        }

        Size::new(max_width, total_height)
    }
}

// Create a global instance for use throughout the application
static TEXT_MANAGER: OnceLock<TextManager> = OnceLock::new();

#[cfg(test)]
mod tests {
    use std::ptr;

    use float_cmp::assert_approx_eq;

    use super::*;

    #[test]
    fn test_text_definition_default_borrowed_returns_static() {
        let borrowed1 = TextDefinition::default_borrowed();
        let borrowed2 = TextDefinition::default_borrowed();
        // Should return same static reference
        assert!(ptr::eq(borrowed1, borrowed2));
    }

    #[test]
    fn test_text_definition_default_borrowed_values() {
        let borrowed = TextDefinition::default_borrowed();
        // Static default has different values than Default trait
        assert_eq!(borrowed.font_size(), 12);
        assert_eq!(borrowed.font_family(), "sans-serif");
        assert!(borrowed.background_color().is_none());
        assert!(borrowed.color().is_none());
        // Static default has 4px uniform padding
        assert_approx_eq!(f32, borrowed.padding().top(), 4.0);
        assert_approx_eq!(f32, borrowed.padding().right(), 4.0);
        assert_approx_eq!(f32, borrowed.padding().bottom(), 4.0);
        assert_approx_eq!(f32, borrowed.padding().left(), 4.0);
    }

    #[test]
    fn test_text_definition_set_font_size() {
        let mut def = TextDefinition::new();
        assert_eq!(def.font_size(), 15); // default

        def.set_font_size(24);
        assert_eq!(def.font_size(), 24);

        def.set_font_size(8);
        assert_eq!(def.font_size(), 8);
    }

    #[test]
    fn test_text_definition_set_font_family() {
        let mut def = TextDefinition::new();
        assert_eq!(def.font_family(), "Arial"); // default

        def.set_font_family("Helvetica");
        assert_eq!(def.font_family(), "Helvetica");

        def.set_font_family("Times New Roman");
        assert_eq!(def.font_family(), "Times New Roman");

        def.set_font_family("monospace");
        assert_eq!(def.font_family(), "monospace");
    }

    #[test]
    fn test_text_definition_set_background_color() {
        let mut def = TextDefinition::new();
        assert!(def.background_color().is_none()); // default

        let yellow = Color::new("yellow").unwrap();
        def.set_background_color(Some(yellow));
        assert!(def.background_color().is_some());

        def.set_background_color(None);
        assert!(def.background_color().is_none());
    }

    #[test]
    fn test_text_definition_set_color() {
        let mut def = TextDefinition::new();
        assert!(def.color().is_none()); // default

        let blue = Color::new("blue").unwrap();
        def.set_color(Some(blue));
        assert!(def.color().is_some());

        def.set_color(None);
        assert!(def.color().is_none());
    }

    #[test]
    fn test_text_definition_set_padding() {
        let mut def = TextDefinition::new();
        // Default padding is all zeros
        assert_approx_eq!(f32, def.padding().horizontal_sum(), 0.0);
        assert_approx_eq!(f32, def.padding().vertical_sum(), 0.0);

        def.set_padding(Insets::uniform(10.0));
        assert_approx_eq!(f32, def.padding().top(), 10.0);
        assert_approx_eq!(f32, def.padding().right(), 10.0);
        assert_approx_eq!(f32, def.padding().bottom(), 10.0);
        assert_approx_eq!(f32, def.padding().left(), 10.0);

        def.set_padding(Insets::new(5.0, 10.0, 15.0, 20.0));
        assert_approx_eq!(f32, def.padding().top(), 5.0);
        assert_approx_eq!(f32, def.padding().right(), 10.0);
        assert_approx_eq!(f32, def.padding().bottom(), 15.0);
        assert_approx_eq!(f32, def.padding().left(), 20.0);
    }

    #[test]
    fn test_text_calculate_size_empty() {
        let def = TextDefinition::new();
        let text = Text::new(&def, "");
        let size = text.calculate_size();
        assert_approx_eq!(f32, size.width(), 0.0);
        assert_approx_eq!(f32, size.height(), 0.0);
    }

    #[test]
    fn test_text_calculate_size_single_line() {
        let def = TextDefinition::new();
        let text = Text::new(&def, "Hello World");
        let size = text.calculate_size();
        // Single line should have positive dimensions
        assert!(size.width() > 0.0, "Width should be positive");
        assert!(size.height() > 0.0, "Height should be positive");
    }

    #[test]
    fn test_text_calculate_size_multiline() {
        let def = TextDefinition::new();
        let single = Text::new(&def, "Line 1");
        let multi = Text::new(&def, "Line 1\nLine 2\nLine 3");

        let single_size = single.calculate_size();
        let multi_size = multi.calculate_size();

        // Multi-line should be taller than single line
        assert!(
            multi_size.height() > single_size.height(),
            "Multi-line text ({}) should be taller than single line ({})",
            multi_size.height(),
            single_size.height()
        );
    }

    #[test]
    fn test_text_calculate_size_includes_padding() {
        let mut def_no_padding = TextDefinition::new();
        def_no_padding.set_padding(Insets::uniform(0.0));
        let text_no_padding = Text::new(&def_no_padding, "Test");
        let size_no_padding = text_no_padding.calculate_size();

        let mut def_with_padding = TextDefinition::new();
        def_with_padding.set_padding(Insets::uniform(20.0));
        let text_with_padding = Text::new(&def_with_padding, "Test");
        let size_with_padding = text_with_padding.calculate_size();

        // Verify the difference is exactly the padding amount (40 total)
        let width_diff = size_with_padding.width() - size_no_padding.width();
        let height_diff = size_with_padding.height() - size_no_padding.height();
        assert_approx_eq!(f32, width_diff, 40.0);
        assert_approx_eq!(f32, height_diff, 40.0);
    }

    #[test]
    fn test_text_calculate_size_larger_font() {
        let mut small_def = TextDefinition::new();
        small_def.set_font_size(12);
        small_def.set_padding(Insets::uniform(0.0));

        let mut large_def = TextDefinition::new();
        large_def.set_font_size(24);
        large_def.set_padding(Insets::uniform(0.0));

        let small_text = Text::new(&small_def, "Test");
        let large_text = Text::new(&large_def, "Test");

        let small_size = small_text.calculate_size();
        let large_size = large_text.calculate_size();

        // Larger font should produce larger size
        assert!(
            large_size.height() > small_size.height(),
            "Larger font height ({}) should be greater than smaller font ({})",
            large_size.height(),
            small_size.height()
        );
        assert!(
            large_size.width() > small_size.width(),
            "Larger font width ({}) should be greater than smaller font ({})",
            large_size.width(),
            small_size.width()
        );
    }

    #[test]
    fn test_text_render_to_layers_has_content() {
        let def = TextDefinition::new();
        let text = Text::new(&def, "Hello");
        let output = text.render_to_layers(Point::new(100.0, 100.0));
        // Should have content (at least the Text layer)
        assert!(!output.is_empty());
    }

    #[test]
    fn test_text_render_with_background_adds_layer() {
        let mut def = TextDefinition::new();
        def.set_background_color(Some(Color::new("yellow").unwrap()));
        let text = Text::new(&def, "With Background");
        let output = text.render_to_layers(Point::new(0.0, 0.0));
        // Should have content on both Background and Text layers
        assert!(!output.is_empty());
        // Render and verify we get multiple layer groups
        let rendered = output.render();
        assert!(
            rendered.len() >= 2,
            "Should have at least 2 layers (Background and Text), got {}",
            rendered.len()
        );
    }

    #[test]
    fn test_text_content_accessor() {
        let def = TextDefinition::new();
        let text = Text::new(&def, "My Content");
        assert_eq!(text.content(), "My Content");

        let empty_text = Text::new(&def, "");
        assert_eq!(empty_text.content(), "");

        let multiline = Text::new(&def, "Line 1\nLine 2");
        assert_eq!(multiline.content(), "Line 1\nLine 2");
    }
}
