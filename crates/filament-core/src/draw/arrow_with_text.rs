use crate::{
    draw::{Arrow, ArrowDrawer, Drawable, LayeredOutput, RenderLayer, Text},
    geometry::Point,
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
    pub fn new(arrow: Arrow, text: Option<Text<'a>>) -> Self {
        Self { arrow, text }
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

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::draw::{ArrowDefinition, ArrowDirection, StrokeDefinition, TextDefinition};

    /// Helper function to create a test arrow with default settings
    fn create_test_arrow(direction: ArrowDirection) -> Arrow {
        let stroke = Rc::new(StrokeDefinition::default());
        let definition = Rc::new(ArrowDefinition::new(stroke));
        Arrow::new(definition, direction)
    }

    #[test]
    fn test_arrow_with_text_new() {
        // With text label
        let arrow = create_test_arrow(ArrowDirection::Forward);
        let text_def = TextDefinition::default();
        let label = "Test Label";
        let text = Text::new(&text_def, label);
        let arrow_with_text = ArrowWithText::new(arrow, Some(text));
        let text = arrow_with_text
            .text
            .as_ref()
            .expect("text should be present");
        assert_eq!(text.content(), label);

        // Without text label
        let arrow = create_test_arrow(ArrowDirection::Forward);
        let arrow_with_text = ArrowWithText::new(arrow, None);
        assert!(arrow_with_text.text.is_none());
    }

    #[test]
    fn test_arrow_with_text_calculate_text_position() {
        let arrow = create_test_arrow(ArrowDirection::Forward);
        let arrow_with_text = ArrowWithText::new(arrow, None);

        // Without text, should return default point
        let pos =
            arrow_with_text.calculate_text_position(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
        assert_eq!(pos, Point::default());

        // With text, should return midpoint
        let arrow = create_test_arrow(ArrowDirection::Forward);
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Label");
        let arrow_with_text = ArrowWithText::new(arrow, Some(text));

        let pos =
            arrow_with_text.calculate_text_position(Point::new(0.0, 0.0), Point::new(100.0, 50.0));
        assert_eq!(pos, Point::new(50.0, 25.0));
    }

    #[test]
    fn test_arrow_with_text_clone() {
        let arrow = create_test_arrow(ArrowDirection::Forward);
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Cloned Label");
        let original = ArrowWithText::new(arrow, Some(text));

        let cloned = original.clone();

        let original_text = original.text.as_ref().unwrap().content();
        let cloned_text = cloned.text.as_ref().unwrap().content();
        assert_eq!(original_text, cloned_text);
    }

    #[test]
    fn test_arrow_with_text_render_to_layers() {
        let mut arrow_drawer = ArrowDrawer::default();
        let source = Point::new(0.0, 0.0);
        let destination = Point::new(100.0, 50.0);

        // With text label - should have arrow and text layers
        let arrow = create_test_arrow(ArrowDirection::Forward);
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Label");
        let arrow_with_text = ArrowWithText::new(arrow, Some(text));

        let output = arrow_with_text.render_to_layers(&mut arrow_drawer, source, destination);
        assert!(!output.is_empty());

        // Without text label - should still have arrow layer
        let arrow = create_test_arrow(ArrowDirection::Forward);
        let arrow_with_text = ArrowWithText::new(arrow, None);

        let output = arrow_with_text.render_to_layers(&mut arrow_drawer, source, destination);
        assert!(!output.is_empty());
    }

    #[test]
    fn test_arrow_with_text_render_all_directions() {
        let source = Point::new(0.0, 0.0);
        let destination = Point::new(100.0, 50.0);
        let text_def = TextDefinition::default();

        let directions = [
            ArrowDirection::Forward,
            ArrowDirection::Backward,
            ArrowDirection::Bidirectional,
            ArrowDirection::Plain,
        ];

        for direction in directions {
            let mut arrow_drawer = ArrowDrawer::default();
            let arrow = create_test_arrow(direction);
            let text = Text::new(&text_def, "Label");
            let arrow_with_text = ArrowWithText::new(arrow, Some(text));

            let output = arrow_with_text.render_to_layers(&mut arrow_drawer, source, destination);
            assert!(
                !output.is_empty(),
                "Rendering failed for direction: {:?}",
                direction
            );
        }
    }
}
