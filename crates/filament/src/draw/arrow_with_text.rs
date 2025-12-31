use crate::{
    draw::{Arrow, ArrowDrawer, Drawable, LayeredOutput, RenderLayer, Text},
    geometry::{Point, Size},
};

/// A drawable that combines an arrow with optional text positioned at the midpoint.
///
/// This struct provides a way to render arrows with optional text labels
/// positioned at the center of the arrow line.
#[derive(Debug, Clone)]
pub struct ArrowWithText<'a> {
    arrow: Arrow,
    text: Option<Text<'a>>,
}

impl<'a> ArrowWithText<'a> {
    /// Creates a new ArrowWithText with the given arrow and no text.
    pub fn new(arrow: Arrow) -> Self {
        Self { arrow, text: None }
    }

    /// Creates a new ArrowWithText with the given arrow and text.
    pub fn with_text(arrow: Arrow, text: Text<'a>) -> Self {
        Self {
            arrow,
            text: Some(text),
        }
    }

    /// Sets the text for this arrow.
    pub fn set_text(&mut self, text: Text<'a>) {
        self.text = Some(text);
    }

    /// Removes the text from this arrow.
    pub fn clear_text(&mut self) {
        self.text = None;
    }

    /// Returns a reference to the text, if any.
    pub fn text(&self) -> Option<&Text<'a>> {
        self.text.as_ref()
    }

    /// Returns a reference to the underlying arrow.
    pub fn arrow(&self) -> &Arrow {
        &self.arrow
    }

    /// Returns the size of the text component, or zero size if no text is present.
    pub fn text_size(&self) -> Size {
        self.text.as_ref().map(|t| t.size()).unwrap_or_default()
    }

    /// Calculates the position where text should be rendered relative to the arrow.
    ///
    /// The text is positioned at the midpoint of the arrow line.
    fn calculate_text_position(&self, source: Point, destination: Point) -> Point {
        if self.text.is_none() {
            return Point::default();
        }

        // Position text at the midpoint of the arrow
        source.midpoint(destination)
    }

    /// Renders the arrow with optional text to layered output.
    // TODO: borrowing arrow_drawer is not good in here.
    pub fn render_to_layers(
        &self,
        arrow_drawer: &mut ArrowDrawer,
        source: Point,
        destination: Point,
    ) -> LayeredOutput {
        let mut output = LayeredOutput::new();

        let rendered_arrow = arrow_drawer.draw_arrow(&self.arrow, source, destination);
        output.add_to_layer(RenderLayer::Arrow, rendered_arrow);

        if let Some(text) = &self.text {
            let text_pos = self.calculate_text_position(source, destination);
            let text_output = text.render_to_layers(text_pos);
            output.merge(text_output);
        }

        output
    }
}

/// ArrowWithTextDrawer manages arrow rendering with text and marker generation.
///
/// The ArrowWithTextDrawer collects color information from arrows to generate
/// the necessary SVG marker definitions upfront, which can then be
/// referenced by individual arrow elements.
///
/// This approach ensures that all required markers are defined once
/// in the SVG document, improving efficiency and avoiding duplication.
#[derive(Debug, Default)]
pub struct ArrowWithTextDrawer(ArrowDrawer);

impl ArrowWithTextDrawer {
    /// Creates a new ArrowWithTextDrawer
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an arrow with optional text to be rendered later
    pub fn draw_arrow_with_text(
        &mut self,
        arrow_with_text: &ArrowWithText,
        source: Point,
        destination: Point,
    ) -> LayeredOutput {
        arrow_with_text.render_to_layers(&mut self.0, source, destination)
    }

    /// Generates SVG marker definitions for all arrows
    pub fn draw_marker_definitions(&self) -> Box<dyn svg::Node> {
        self.0.draw_marker_definitions()
    }
}
