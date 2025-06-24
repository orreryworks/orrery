use crate::{
    draw::Drawable,
    geometry::{Point, Size},
};
use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping};
use log::info;
use std::{
    cell::{Ref, RefCell},
    rc::Rc,
    sync::{Arc, Mutex},
};
use svg::{self, node::element as svg_element};

const DEFAULT_FONT_FAMILY: &str = "Arial";

#[derive(Debug, Clone)]
pub struct TextDefinition {
    font_size: u16,
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
}

impl Default for TextDefinition {
    fn default() -> Self {
        Self {
            font_size: 15, // Default font size
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

    pub fn definition(&self) -> Ref<TextDefinition> {
        self.definition.borrow()
    }

    /// Calculate the size required to display this text
    pub fn calculate_size(&self) -> Size {
        TEXT_MANAGER.calculate_text_size(&self.content, self.definition.borrow().font_size())
    }
}

impl Drawable for Text {
    fn render_to_svg(&self, position: Point) -> Box<dyn svg::Node> {
        let text_def = self.definition.borrow();

        let rendered = svg_element::Text::new(self.content())
            .set("x", position.x())
            .set("y", position.y())
            .set("text-anchor", "middle")
            .set("dominant-baseline", "middle")
            .set("font-family", DEFAULT_FONT_FAMILY)
            .set("font-size", text_def.font_size());

        rendered.into()
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
