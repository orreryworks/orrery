use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping};
use log::info;
use std::sync::{Arc, Mutex};

use crate::layout::geometry::Size;

/// TextManager handles text measurement and font operations
/// It maintains a reusable FontSystem instance to avoid expensive recreation
pub struct TextManager {
    font_system: Arc<Mutex<FontSystem>>,
}

impl Default for TextManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TextManager {
    /// Create a new TextManager with a default FontSystem
    pub fn new() -> Self {
        info!("Initializing FontSystem");
        Self {
            font_system: Arc::new(Mutex::new(FontSystem::new())),
        }
    }

    /// Calculate the actual size of text in pixels using cosmic-text
    /// This provides an accurate measurement based on real font metrics and shaping
    pub fn calculate_text_size(&self, text: &str, font_size: usize) -> Size {
        // Lock the FontSystem for use
        let mut font_system = self.font_system.lock().unwrap();

        // Convert font size from points to pixels (roughly 1.33x multiplier for standard DPI)
        let font_size_px = font_size as f32 * 1.33;

        // Create metrics with font size and approximate line height
        let line_height = font_size_px * 1.2;
        let metrics = Metrics::new(font_size_px, line_height);

        // Create a buffer with the metrics
        let mut buffer = Buffer::new(&mut font_system, metrics);

        // Borrow buffer with font system for more convenient method calls
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
            max_width = text.len() as f32 * (font_size_px * 0.6);
            total_height = metrics.line_height;
        }

        Size::new(max_width, total_height)
    }
}

// Create a global instance for use throughout the application
lazy_static::lazy_static! {
    static ref TEXT_MANAGER: TextManager = TextManager::new();
}

/// Simplified convenience function that delegates to the TEXT_MANAGER
pub fn calculate_text_size(text: &str, font_size: usize) -> Size {
    TEXT_MANAGER.calculate_text_size(text, font_size)
}
