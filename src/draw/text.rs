use crate::{
    color::Color,
    draw::Drawable,
    geometry::{Insets, Point, Size},
};
use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping};
use log::info;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};
use svg::{self, node::element as svg_element};

const DEFAULT_FONT_FAMILY: &str = "Arial";

#[derive(Debug, Clone)]
pub struct TextDefinition {
    font_size: u16,
    background_color: Option<Color>,
    padding: Insets,
}

impl TextDefinition {
    /// Create a new text definition with default values
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_font_size(&mut self, size: u16) {
        self.font_size = size;
    }

    pub fn font_size(&self) -> u16 {
        self.font_size
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

    /// Sets the padding around the text content.
    ///
    /// Padding affects the size of the background rectangle (if present) and creates
    /// space between the text and the background edges. Padding is applied even when
    /// no background color is set, affecting the overall size calculations.
    pub fn set_padding(&mut self, padding: Insets) {
        self.padding = padding;
    }

    /// Returns a reference to the background color, if set.
    fn background_color(&self) -> Option<&Color> {
        self.background_color.as_ref()
    }

    /// Returns the current padding configuration.
    fn padding(&self) -> Insets {
        self.padding
    }
}

impl Default for TextDefinition {
    fn default() -> Self {
        Self {
            font_size: 15, // Default font size
            background_color: None,
            padding: Insets::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Text {
    definition: Rc<RefCell<TextDefinition>>,
    content: String,
}

impl Text {
    pub fn new(definition: Rc<RefCell<TextDefinition>>, content: String) -> Self {
        Self {
            definition,
            content,
        }
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    /// Calculate the total size required to display this text, including padding.
    pub fn calculate_size(&self) -> Size {
        let padding = self.definition.borrow().padding();
        self.calculate_size_without_padding().add_padding(padding)
    }

    /// Calculate the size required to display this text content without padding.
    fn calculate_size_without_padding(&self) -> Size {
        TEXT_MANAGER.calculate_text_size(&self.content, self.definition.borrow().font_size())
    }
}

impl Drawable for Text {
    fn render_to_svg(&self, position: Point) -> Box<dyn svg::Node> {
        let text_def = self.definition.borrow();

        let mut group = svg_element::Group::new();
        let text_size = self.calculate_size();
        let padding = text_def.padding();

        let rendered_text = svg_element::Text::new(self.content())
            .set("x", position.x())
            .set("y", position.y())
            .set("text-anchor", "middle")
            .set("dominant-baseline", "middle")
            .set("font-family", DEFAULT_FONT_FAMILY)
            .set("font-size", text_def.font_size());

        // Add background rectangle if color is specified
        if let Some(bg_color) = text_def.background_color() {
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

            group = group.add(bg);
            group = group.add(rendered_text);
            return group.into();
        }

        rendered_text.into()
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

    /// Calculate the actual size of text in pixels using cosmic-text
    /// This provides an accurate measurement based on real font metrics and shaping
    fn calculate_text_size(&self, text: &str, font_size: u16) -> Size {
        if text.is_empty() {
            return Size::default();
        }

        // Lock the FontSystem for use
        let mut font_system = self.font_system.lock().unwrap();

        // Convert font size from points to pixels (roughly 1.33x multiplier for standard DPI)
        let font_size_px = font_size as f32 * 1.33;

        // Create metrics with font size and approximate line height
        let line_height = font_size_px * 1.15;
        let metrics = Metrics::new(font_size_px, line_height);

        // Create a buffer with the metrics
        let mut buffer = Buffer::new(&mut font_system, metrics);
        let mut buffer = buffer.borrow_with(&mut font_system);

        // Set up text attributes
        let attrs = Attrs::new().family(Family::Name("Arial"));

        // Set the buffer's size to unlimited to allow text to flow naturally
        buffer.set_size(None, None);

        // Set the text with advanced shaping for accurate text metrics
        // Advanced shaping handles ligatures, kerning, etc.
        buffer.set_text(text, &attrs, Shaping::Advanced);

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
lazy_static::lazy_static! {
    static ref TEXT_MANAGER: TextManager = TextManager::new();
}
