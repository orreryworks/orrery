//! Fragment Drawable Implementation
//!
//! This module provides drawable components for rendering fragment blocks in sequence diagrams.
//! Fragments group related interactions into labeled sections (e.g., "alt" for alternatives,
//! "loop" for iterations, "opt" for optional flows).
//!
//! # Architecture
//!
//! - [`FragmentDefinition`]: Contains styling configuration (borders, colors, text definitions)
//! - [`Fragment`]: The main drawable that implements the [`Drawable`] trait
//! - [`FragmentSection`]: Represents individual sections within a fragment
//!
//! # Visual Structure
//!
//! Fragments render as rectangular boxes with:
//! - An operation label in the upper-left corner (e.g., "alt", "loop", "opt")
//! - Optional section titles for each section
//! - Dashed horizontal separators between sections
//! - Content area with padding for nested elements

use crate::{
    color::Color,
    draw::{Drawable, StrokeDefinition, Text, TextDefinition},
    geometry::{Bounds, Insets, Point, Size},
};
use std::borrow::Cow;

#[cfg(test)]
use crate::draw::StrokeStyle;
use svg::{self, node::element as svg_element};

/// Styling configuration for fragment blocks in sequence diagrams.
///
/// This struct contains all visual properties needed to render fragments,
/// including border styling, background colors, text definitions for labels,
/// and section separators. It follows the same pattern as other definition
/// structs in the codebase (e.g., `ActivationBoxDefinition`, `LifelineDefinition`).
#[derive(Debug, Clone)]
pub struct FragmentDefinition {
    /// The stroke styling for the fragment border
    border_stroke: Cow<'static, StrokeDefinition>,
    /// Optional background color for the entire fragment
    background_color: Option<Color>,

    /// Text definition for the operation label (e.g., "alt", "loop")
    operation_label_text_definition: Cow<'static, TextDefinition>,
    /// Text definition for section titles
    section_title_text_definition: Cow<'static, TextDefinition>,

    /// The stroke styling for section separator lines
    separator_stroke: Cow<'static, StrokeDefinition>,

    /// Fill color for the pentagonal operation label tab
    pentagon_fill_color: Color,

    /// Padding around the fragment content
    content_padding: Insets,
    /// Padding added to fragment bounds for visual separation from lifelines and messages
    bounds_padding: Insets,
}

impl FragmentDefinition {
    /// Creates a new FragmentDefinition with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the background color
    pub fn set_background_color(&mut self, color: Option<Color>) {
        self.background_color = color;
    }

    /// Sets the operation label text definition
    pub fn set_operation_label_text_definition(&mut self, text_def: Cow<'static, TextDefinition>) {
        self.operation_label_text_definition = text_def;
    }

    /// Sets the section title text definition
    pub fn set_section_title_text_definition(&mut self, text_def: Cow<'static, TextDefinition>) {
        self.section_title_text_definition = text_def;
    }

    /// Sets the content padding
    pub fn set_content_padding(&mut self, padding: Insets) {
        self.content_padding = padding;
    }

    /// Sets the bounds padding
    pub fn set_bounds_padding(&mut self, padding: Insets) {
        self.bounds_padding = padding;
    }

    /// Sets the border stroke definition
    pub fn set_border_stroke(&mut self, stroke: Cow<'static, StrokeDefinition>) {
        self.border_stroke = stroke;
    }

    /// Sets the separator stroke definition
    pub fn set_separator_stroke(&mut self, stroke: Cow<'static, StrokeDefinition>) {
        self.separator_stroke = stroke;
    }

    /// Returns the border stroke definition
    pub fn border_stroke(&self) -> &StrokeDefinition {
        &self.border_stroke
    }

    /// Gets the background color
    fn background_color(&self) -> Option<&Color> {
        self.background_color.as_ref()
    }

    /// Returns the separator stroke definition
    pub fn separator_stroke(&self) -> &StrokeDefinition {
        &self.separator_stroke
    }

    /// Gets the content padding
    fn content_padding(&self) -> Insets {
        self.content_padding
    }

    /// Gets the bounds padding
    fn bounds_padding(&self) -> Insets {
        self.bounds_padding
    }

    /// Sets the pentagon fill color
    pub fn set_pentagon_fill_color(&mut self, color: Color) {
        self.pentagon_fill_color = color;
    }

    /// Gets the pentagon fill color
    pub fn pentagon_fill_color(&self) -> &Color {
        &self.pentagon_fill_color
    }

    /// Creates an SVG path element for a pentagonal tab (rectangle with triangular point on right).
    ///
    /// The triangle width is calculated as half the height to maintain proper proportions.
    ///
    /// # Arguments
    ///
    /// * `content_bounds` - Bounds of the rectangular content area (determines pentagon position and size)
    ///
    /// # Returns
    ///
    /// A tuple containing the SVG path element and the complete pentagon bounds (including triangle)
    fn create_pentagon_path(&self, content_bounds: Bounds) -> (svg_element::Path, Bounds) {
        let top_left = content_bounds.min_point();

        // Calculate triangle width as half the height for proper proportions
        let triangle_width = 10.0f32
            .min(content_bounds.height() / 2.0)
            .max(content_bounds.height() / 4.0);

        // Create path: rectangle + triangle point
        // L x+w+t,y - start top-right with triangle
        // L x+w+t,y+h/2 - triangle point (middle-right)
        // L x+w,y+h - bottom-right of rectangle
        // L x,y+h - bottom-left
        // Z - close path
        let path_data = format!(
            "M {} {} L {} {} L {} {} L {} {}",
            content_bounds.max_x() + triangle_width,
            content_bounds.min_y(),
            content_bounds.max_x() + triangle_width,
            content_bounds.max_y() - triangle_width,
            content_bounds.max_x(),
            content_bounds.max_y(),
            content_bounds.min_x(),
            content_bounds.max_y()
        );

        let path = svg_element::Path::new()
            .set("d", path_data)
            .set("fill", self.pentagon_fill_color.to_string())
            .set("fill-opacity", self.pentagon_fill_color.alpha());

        let path = crate::apply_stroke!(path, &self.border_stroke);

        let pentagon_size = Size::new(
            content_bounds.width() + triangle_width,
            content_bounds.height(),
        );
        let pentagon_bounds = Bounds::new_from_top_left(top_left, pentagon_size);
        (path, pentagon_bounds)
    }
}

impl Default for FragmentDefinition {
    fn default() -> Self {
        // Create default text definition for operation label
        let mut operation_label_text_definition = TextDefinition::new();
        operation_label_text_definition.set_font_size(12);
        operation_label_text_definition.set_color(Some(Color::default()));
        operation_label_text_definition.set_padding(Insets::new(4.0, 8.0, 4.0, 8.0));

        // Create default text definition for section titles
        let mut section_title_text_definition = TextDefinition::new();
        section_title_text_definition.set_font_size(11);
        section_title_text_definition
            .set_color(Some(Color::new("#666666").expect("Invalid color")));
        section_title_text_definition.set_padding(Insets::new(2.0, 4.0, 2.0, 4.0));

        Self {
            border_stroke: Cow::Borrowed(StrokeDefinition::default_borrowed()),
            background_color: None,

            operation_label_text_definition: Cow::Owned(operation_label_text_definition),
            section_title_text_definition: Cow::Owned(section_title_text_definition),

            separator_stroke: Cow::Borrowed(StrokeDefinition::default_dashed_borrowed()),

            pentagon_fill_color: Color::new("white").expect("Invalid color"),

            content_padding: Insets::new(8.0, 8.0, 8.0, 8.0),
            bounds_padding: Insets::new(20.0, 20.0, 20.0, 20.0),
        }
    }
}

/// Represents a section within a fragment block.
///
/// Each section can have an optional title and a specific height
/// determined by its content. Sections are visually separated by
/// dashed horizontal lines in the rendered fragment.
#[derive(Debug, Clone)]
pub struct FragmentSection {
    /// Optional title for this section (e.g., "successful login", "failed login")
    title: Option<String>, // PERF: This can be ref.
    /// Height of this section's content area in pixels
    height: f32,
}

impl FragmentSection {
    /// Creates a new FragmentSection with the given title and height
    pub fn new(title: Option<String>, height: f32) -> Self {
        Self { title, height }
    }

    /// Returns the section title, if present
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Returns the height of this section
    pub fn height(&self) -> f32 {
        self.height
    }
}

/// A drawable fragment block for sequence diagrams.
///
/// Fragments group related interactions into labeled sections, supporting
/// operations like "alt" (alternatives), "loop" (iterations), "opt" (optional),
/// and "par" (parallel). The fragment renders as a rectangular box with an
/// operation label, optional section titles, and separators between sections.
///
/// # Positioning Behavior
///
/// When `render_to_svg(position)` is called:
/// 1. The `position` parameter represents the center point of the fragment box
/// 2. The fragment renders its border, operation label, and section separators
/// 3. Content within sections is handled by the layout engine (not rendered here)
#[derive(Debug, Clone)]
pub struct Fragment {
    /// The styling definition for this fragment
    definition: Cow<'static, FragmentDefinition>,
    /// The operation type (e.g., "alt", "loop", "opt", "par")
    operation: String,
    /// The sections within this fragment
    sections: Vec<FragmentSection>,
    /// The total size of the fragment box
    size: Size,
}

impl Fragment {
    /// Creates a new Fragment with the given definition, operation, sections, and size.
    ///
    /// # Arguments
    ///
    /// * `definition` - Shared styling configuration for the fragment
    /// * `operation` - The operation type string (e.g., "alt", "loop")
    /// * `sections` - Vector of sections within the fragment
    /// * `size` - Total size of the fragment box (calculated externally by layout)
    pub fn new(
        definition: Cow<'static, FragmentDefinition>,
        operation: String,
        sections: Vec<FragmentSection>,
        size: Size,
    ) -> Self {
        Self {
            definition,
            operation,
            sections,
            size,
        }
    }

    /// Returns the operation type
    pub fn operation(&self) -> &str {
        &self.operation
    }

    /// Returns the sections
    pub fn sections(&self) -> &[FragmentSection] {
        &self.sections
    }

    /// Returns the total size
    pub fn size(&self) -> Size {
        self.size
    }
}

impl Drawable for Fragment {
    fn render_to_svg(&self, position: Point) -> Box<dyn svg::Node> {
        let mut group = svg_element::Group::new();
        let padding = self.definition.content_padding();
        let bounds_padding = self.definition.bounds_padding();

        // Apply bounds padding to expand the fragment beyond its content
        let expanded_size = self.size.add_padding(bounds_padding);
        let bounds = position.to_bounds(expanded_size);
        let top_left = bounds.min_point();

        // 1. Render background if specified
        if let Some(bg_color) = self.definition.background_color() {
            let background = svg_element::Rectangle::new()
                .set("x", top_left.x())
                .set("y", top_left.y())
                .set("width", bounds.width())
                .set("height", bounds.height())
                .set("fill", bg_color.to_string())
                .set("fill-opacity", bg_color.alpha());
            group = group.add(background);
        }

        // 2. Render border
        let border = svg_element::Rectangle::new()
            .set("x", top_left.x())
            .set("y", top_left.y())
            .set("width", bounds.width())
            .set("height", bounds.height())
            .set("fill", "none");

        let border = crate::apply_stroke!(border, self.definition.border_stroke());
        group = group.add(border);

        // 3. Render operation label in upper-left corner as pentagonal tab
        // First, measure the text to determine pentagon size
        let operation_text = Text::new(
            self.definition.operation_label_text_definition.clone(),
            self.operation.clone(),
        );
        let pentagon_content_size = operation_text.size();
        let pentagon_content_bounds = Bounds::new_from_top_left(top_left, pentagon_content_size);

        // Render pentagon path (returns complete pentagon bounds including triangle)
        let (pentagon, pentagon_bounds) = self
            .definition
            .create_pentagon_path(pentagon_content_bounds);

        group = group.add(pentagon);
        group = group.add(operation_text.render_to_svg(pentagon_content_bounds.center()));

        // 4. Render section separators and titles
        let mut current_y = pentagon_bounds.max_y();

        for (i, section) in self.sections.iter().enumerate() {
            // Skip separator for the first section
            if i > 0 {
                // Draw separator line
                let separator = svg_element::Line::new()
                    .set("x1", top_left.x() + padding.left())
                    .set("y1", current_y)
                    .set("x2", top_left.x() + self.size.width() - padding.right())
                    .set("y2", current_y);

                let separator = crate::apply_stroke!(separator, self.definition.separator_stroke());
                group = group.add(separator);
            }

            // Render section title if present (wrapped in square brackets per UML 2.5)
            if let Some(title) = section.title() {
                let title_text = Text::new(
                    self.definition.section_title_text_definition.clone(),
                    format!("[{}]", title),
                );
                let title_size = title_text.size();

                let title_position = if i == 0 {
                    // First section: position to the right of the pentagon
                    Point::new(
                        pentagon_bounds.max_x() + padding.left() + title_size.width() / 2.0,
                        pentagon_bounds.center().y(),
                    )
                } else {
                    // Other sections: position below the separator
                    Point::new(
                        top_left.x() + padding.left() + title_size.width() / 2.0 + 10.0,
                        current_y + title_size.height() / 2.0 + 5.0,
                    )
                };

                group = group.add(title_text.render_to_svg(title_position));
            }

            // Move to next section position
            current_y += section.height();
        }

        group.into()
    }

    fn size(&self) -> Size {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fragment_definition_custom_values() {
        let mut definition = FragmentDefinition::new();

        definition.set_background_color(Some(Color::new("#f0f0f0").unwrap()));
        definition.set_content_padding(Insets::new(10.0, 12.0, 10.0, 12.0));

        // Verify background color
        assert!(definition.background_color().is_some());
        let bg_color = definition.background_color().unwrap().to_string();
        assert!(
            bg_color.contains("240"),
            "Background color should contain value 240"
        );

        // Verify content padding
        let padding = definition.content_padding();
        assert_eq!(padding.top(), 10.0);
        assert_eq!(padding.right(), 12.0);
        assert_eq!(padding.bottom(), 10.0);
        assert_eq!(padding.left(), 12.0);

        // Verify default border stroke properties (solid black, 1.0 width)
        assert_eq!(definition.border_stroke().color().to_string(), "black");
        assert_eq!(definition.border_stroke().width(), 1.0);
        assert_eq!(*definition.border_stroke().style(), StrokeStyle::Solid);

        // Verify default separator stroke properties (dashed black, 1.0 width)
        assert_eq!(definition.separator_stroke().color().to_string(), "black");
        assert_eq!(definition.separator_stroke().width(), 1.0);
        assert_eq!(*definition.separator_stroke().style(), StrokeStyle::Dashed);
    }

    #[test]
    fn test_fragment_section_creation() {
        let section1 = FragmentSection::new(Some("test section".to_string()), 100.0);
        assert_eq!(section1.title(), Some("test section"));
        assert_eq!(section1.height(), 100.0);

        let section2 = FragmentSection::new(None, 50.0);
        assert_eq!(section2.title(), None);
        assert_eq!(section2.height(), 50.0);
    }

    #[test]
    fn test_fragment_creation() {
        let definition = FragmentDefinition::default();
        let sections = vec![
            FragmentSection::new(Some("section 1".to_string()), 80.0),
            FragmentSection::new(Some("section 2".to_string()), 60.0),
            FragmentSection::new(None, 40.0),
        ];
        let fragment = Fragment::new(
            Cow::Owned(definition),
            "alt".to_string(),
            sections.clone(),
            Size::new(200.0, 180.0),
        );

        assert_eq!(fragment.operation(), "alt");
        assert_eq!(fragment.sections().len(), 3);
        assert_eq!(fragment.size(), Size::new(200.0, 180.0));
    }

    #[test]
    fn test_fragment_render_to_svg() {
        let definition = FragmentDefinition::default();
        let sections = vec![
            FragmentSection::new(Some("successful".to_string()), 100.0),
            FragmentSection::new(Some("failed".to_string()), 80.0),
        ];
        let fragment = Fragment::new(
            Cow::Owned(definition),
            "alt".to_string(),
            sections,
            Size::new(300.0, 200.0),
        );

        // The fragment should be 200x150
        assert_eq!(fragment.size(), Size::new(300.0, 200.0));

        let position = Point::new(50.0, 100.0);
        let svg_node = fragment.render_to_svg(position);

        // Basic smoke test - verify it returns a valid SVG node without panicking
        drop(svg_node);
    }
}
