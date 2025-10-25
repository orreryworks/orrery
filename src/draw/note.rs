//! Note drawable for diagram annotations.
//!
//! This module provides a note drawable that renders as a rectangle with a "dog-eared"
//! (bent) top-right corner. Notes are commonly used for annotations and comments in diagrams.
//!
//! # Visual Appearance
//!
//! A note consists of three visual layers:
//! - **Main body**: A rectangle with the top-right corner cut at a 45° angle
//! - **Fold triangle**: A small triangle showing the folded corner (slightly darker)
//! - **Fold line**: A diagonal line emphasizing where the corner bends
//!
//! # Examples
//!
//! ```
//! # use std::rc::Rc;
//! # use filament::draw::{Note, NoteDefinition};
//! # use filament::geometry::Point;
//! # use filament::draw::Drawable;
//! #
//! // Create a note with default styling
//! let definition = Rc::new(NoteDefinition::new());
//! let note = Note::new(definition, "This is a note".to_string());
//!
//! // Get the size
//! let size = note.size();
//!
//! // Render to SVG
//! let position = Point::new(100.0, 100.0);
//! let svg_node = note.render_to_svg(position);
//! ```
//!
//! # Customization
//!
//! ```
//! # use filament::draw::{NoteDefinition, StrokeDefinition};
//! # use filament::color::Color;
//! #
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut definition = NoteDefinition::new();
//!
//! // Customize background color
//! definition.set_background_color(Some(Color::new("#ffebcd")?));
//!
//! // Customize border
//! let stroke = StrokeDefinition::dashed(Color::new("blue")?, 2.0);
//! definition.set_stroke(stroke);
//! # Ok(())
//! # }
//! ```

use crate::{
    color::Color,
    draw::{Drawable, StrokeDefinition, Text, TextDefinition},
    geometry::{Insets, Point, Size},
};
use std::rc::Rc;
use svg::{self, node::element as svg_element};

/// Fixed size for the dog-eared corner fold in pixels.
///
/// This constant defines the size of the cut corner at the top-right of the note.
/// The fold appears as a 12px x 12px triangle that has been "bent over" to create
/// the dog-eared effect.
const CORNER_FOLD_SIZE: f32 = 12.0;

/// Definition for note styling and appearance.
///
/// `NoteDefinition` is a configuration struct that defines how a note should be styled,
/// including its background color, border stroke, and text styling. Multiple notes can
/// share the same definition for consistent styling.
///
/// # Default Values
///
/// - **Background**: Light yellow (`#fffacd`)
/// - **Stroke**: Default stroke
/// - **Text**: Default text definition
///
/// # Examples
///
/// ```
/// # use filament::draw::{NoteDefinition, StrokeDefinition};
/// # use filament::color::Color;
/// #
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Create with defaults
/// let mut definition = NoteDefinition::new();
///
/// // Customize
/// definition.set_background_color(Some(Color::new("lightblue")?));
/// let stroke = StrokeDefinition::new(Color::new("navy")?, 2.0);
/// definition.set_stroke(stroke);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct NoteDefinition {
    background_color: Option<Color>,
    stroke: Rc<StrokeDefinition>,
    text: Rc<TextDefinition>,
}

impl NoteDefinition {
    /// Creates a new note definition with default values.
    ///
    /// This is equivalent to calling [`NoteDefinition::default()`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use filament::draw::NoteDefinition;
    /// let definition = NoteDefinition::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the background color for the note.
    ///
    /// When set to `Some(color)`, the note will be filled with the specified color.
    /// When set to `None`, the note will not have a background color.
    ///
    /// # Arguments
    ///
    /// * `color` - Optional background color. Use `None` for no background.
    ///
    /// # Examples
    ///
    /// ```
    /// # use filament::draw::NoteDefinition;
    /// # use filament::color::Color;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut definition = NoteDefinition::new();
    /// definition.set_background_color(Some(Color::new("lightyellow")?));
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_background_color(&mut self, color: Option<Color>) {
        self.background_color = color;
    }

    /// Sets the stroke definition for the note border.
    ///
    /// The stroke is applied to the main note body, the fold triangle, and the fold line,
    /// creating a consistent border around all elements of the note.
    ///
    /// # Arguments
    ///
    /// * `stroke` - The stroke definition to apply to the note's border.
    ///
    /// # Examples
    ///
    /// ```
    /// # use filament::draw::{NoteDefinition, StrokeDefinition};
    /// # use filament::color::Color;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut definition = NoteDefinition::new();
    /// let stroke = StrokeDefinition::dashed(Color::new("blue")?, 1.5);
    /// definition.set_stroke(stroke);
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_stroke(&mut self, stroke: StrokeDefinition) {
        self.stroke = Rc::new(stroke);
    }

    /// Sets the text definition for the note content.
    ///
    /// This controls the styling of the text rendered inside the note, including
    /// font family, size, color, and other text properties.
    ///
    /// # Arguments
    ///
    /// * `text` - The text definition to use for rendering the note's content.
    ///
    /// # Examples
    ///
    /// ```
    /// # use filament::draw::{NoteDefinition, TextDefinition};
    /// # use filament::color::Color;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut definition = NoteDefinition::new();
    /// let mut text_def = TextDefinition::new();
    /// text_def.set_font_size(16);
    /// text_def.set_color(Some(Color::new("darkblue")?));
    /// definition.set_text_definition(text_def);
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_text_definition(&mut self, text: TextDefinition) {
        self.text = Rc::new(text);
    }

    /// Returns the background color of the note.
    fn background_color(&self) -> Option<Color> {
        self.background_color
    }

    /// Returns a reference to the stroke definition.
    pub fn stroke(&self) -> &StrokeDefinition {
        &self.stroke
    }
}

impl Default for NoteDefinition {
    fn default() -> Self {
        Self {
            background_color: Some(Color::new("#fffacd").expect("valid color")), // Light yellow
            stroke: Rc::new(StrokeDefinition::default()),
            text: Rc::new(TextDefinition::new()),
        }
    }
}

/// A note drawable with a dog-eared corner.
///
/// `Note` represents a renderable note that combines a [`NoteDefinition`] with text content.
/// The note is rendered as a rectangle with a bent top-right corner, commonly used for
/// annotations in diagrams.
///
/// # Rendering
///
/// When rendered, the note creates an SVG group containing:
/// 1. The main note body (rectangle with cut corner)
/// 2. A small triangle showing the folded corner
/// 3. A diagonal fold line
/// 4. The text content
///
/// # Examples
///
/// ```
/// # use std::rc::Rc;
/// # use filament::draw::{Note, NoteDefinition, Drawable};
/// # use filament::geometry::Point;
/// #
/// let definition = Rc::new(NoteDefinition::new());
/// let note = Note::new(definition, "Important note".to_string());
///
/// // The note calculates its own size based on text
/// let size = note.size();
///
/// // Render at a specific position (center point)
/// let position = Point::new(150.0, 200.0);
/// let svg_node = note.render_to_svg(position);
/// ```
#[derive(Debug, Clone)]
pub struct Note {
    definition: Rc<NoteDefinition>,
    content: String,
}

impl Note {
    /// Creates a new note with the given definition and content.
    ///
    /// # Arguments
    ///
    /// * `definition` - The styling definition for the note (can be shared among multiple notes)
    /// * `content` - The text content to display inside the note
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::rc::Rc;
    /// # use filament::draw::{Note, NoteDefinition};
    /// #
    /// let definition = Rc::new(NoteDefinition::new());
    /// let note = Note::new(definition, "My note text".to_string());
    /// ```
    pub fn new(definition: Rc<NoteDefinition>, content: String) -> Self {
        Self {
            definition,
            content,
        }
    }

    /// Calculates the size of the text content without padding.
    fn text_size(&self) -> Size {
        if self.content.is_empty() {
            return Size::default();
        }
        let text = Text::new(self.definition.text.clone(), self.content.clone());
        text.size()
    }

    /// Calculates the total size of the note including padding.
    fn calculate_size(&self) -> Size {
        let text_size = self.text_size();
        text_size.add_padding(Insets::new(10.0, 10.0 + CORNER_FOLD_SIZE, 10.0, 10.0))
    }

    /// Creates the SVG path element for the main note body with dog-eared corner.
    fn create_dog_eared_path(&self, size: Size, position: Point) -> svg_element::Path {
        let bounds = position.to_bounds(size);
        let min_x = bounds.min_x();
        let min_y = bounds.min_y();
        let max_x = bounds.max_x();
        let max_y = bounds.max_y();

        // The dog-ear is at the top-right corner
        let fold_x = max_x - CORNER_FOLD_SIZE;
        let fold_y = min_y + CORNER_FOLD_SIZE;

        // Create path for the main body (rectangle with top-right corner cut)
        let path_data = format!(
            "M {} {} L {} {} L {} {} L {} {} L {} {} L {} {} Z",
            min_x,
            min_y, // Start at top-left
            fold_x,
            min_y, // Go to where fold starts (top edge)
            max_x,
            fold_y, // Diagonal to right edge at fold height
            max_x,
            max_y, // Down to bottom-right
            min_x,
            max_y, // Across to bottom-left
            min_x,
            min_y // Back up to top-left
        );

        let path = svg_element::Path::new().set("d", path_data);
        crate::apply_stroke!(path, self.definition.stroke())
    }

    /// Creates the SVG path element for the diagonal fold line.
    fn create_fold_line_path(&self, size: Size, position: Point) -> svg_element::Path {
        let bounds = position.to_bounds(size);
        let max_x = bounds.max_x();
        let min_y = bounds.min_y();

        // Fold line goes from the cut point on top edge to the cut point on right edge
        let fold_x = max_x - CORNER_FOLD_SIZE;
        let fold_y = min_y + CORNER_FOLD_SIZE;

        let path_data = format!(
            "M {} {} L {} {}",
            fold_x,
            min_y, // Start at top edge fold point
            max_x,
            fold_y // Go to right edge fold point
        );

        let path = svg_element::Path::new()
            .set("d", path_data)
            .set("fill", "none");
        crate::apply_stroke!(path, self.definition.stroke())
    }

    /// Creates the SVG path element for the small triangular fold-over.
    fn create_fold_triangle_path(&self, size: Size, position: Point) -> svg_element::Path {
        let bounds = position.to_bounds(size);
        let max_x = bounds.max_x();
        let min_y = bounds.min_y();

        let fold_x = max_x - CORNER_FOLD_SIZE;
        let fold_y = min_y + CORNER_FOLD_SIZE;

        // Small triangle to show the folded corner
        let path_data = format!(
            "M {} {} L {} {} L {} {} Z",
            fold_x,
            min_y, // Top edge fold point
            max_x,
            fold_y, // Right edge fold point
            fold_x,
            fold_y // Corner point of triangle
        );

        let path = svg_element::Path::new().set("d", path_data);
        crate::apply_stroke!(path, self.definition.stroke())
    }
}

impl Drawable for Note {
    fn render_to_svg(&self, position: Point) -> Box<dyn svg::Node> {
        let size = self.size();
        let mut group = svg_element::Group::new();

        // Create the main note body with dog-eared corner
        let mut note_body = self.create_dog_eared_path(size, position);

        // Apply background color
        if let Some(bg_color) = self.definition.background_color() {
            note_body = note_body
                .set("fill", bg_color.to_string())
                .set("fill-opacity", bg_color.alpha());
        }

        group = group.add(note_body);

        // Add the small triangle for the folded corner (slightly darker)
        let mut fold_triangle = self.create_fold_triangle_path(size, position);

        // Make the fold slightly darker than the background
        if let Some(bg_color) = self.definition.background_color() {
            let darker = bg_color.with_alpha(bg_color.alpha() * 0.8);
            fold_triangle = fold_triangle
                .set("fill", darker.to_string())
                .set("fill-opacity", darker.alpha());
        } else {
            fold_triangle = fold_triangle
                .set("fill", "#e0e0e0")
                .set("fill-opacity", 0.8);
        }

        group = group.add(fold_triangle);

        // Add the fold line
        let fold_line = self.create_fold_line_path(size, position);

        group = group.add(fold_line);

        // Render the text content if present
        if !self.content.is_empty() {
            let text = Text::new(self.definition.text.clone(), self.content.clone());
            let text_node = text.render_to_svg(position);
            group = group.add(text_node);
        }

        group.into()
    }

    fn size(&self) -> Size {
        self.calculate_size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_definition_default() {
        let def = NoteDefinition::default();
        assert!(def.background_color().is_some());
        assert_eq!(def.stroke().width(), 1.0);
    }

    #[test]
    fn test_note_creation() {
        let def = Rc::new(NoteDefinition::new());
        let note = Note::new(def, "Test note".to_string());
        let size = note.size();
        // Verify note was created with non-zero size (indicates content was stored)
        assert!(size.width() > 0.0);
        assert!(size.height() > 0.0);
    }

    #[test]
    fn test_note_size_calculation() {
        let def = Rc::new(NoteDefinition::new());
        let note = Note::new(def, "Test".to_string());
        let size = note.size();
        assert!(size.width() > 0.0);
        assert!(size.height() > 0.0);
    }

    #[test]
    fn test_empty_note() {
        let def = Rc::new(NoteDefinition::new());
        let note = Note::new(def, String::new());
        let size = note.size();
        // Even empty notes should have some size due to padding
        assert!(size.width() > 0.0);
        assert!(size.height() > 0.0);
    }

    #[test]
    fn test_note_definition_customization() {
        let mut def = NoteDefinition::new();
        def.set_background_color(Some(Color::new("blue").expect("valid color")));
        assert_eq!(
            def.background_color(),
            Some(Color::new("blue").expect("valid color"))
        );
    }
}
