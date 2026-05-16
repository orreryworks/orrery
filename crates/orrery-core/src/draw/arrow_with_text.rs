//! Arrow-with-text composite drawable.
//!
//! This module combines an [`Arrow`] with a [`Text`] label, positioning the
//! text along the arrow path.

use crate::{
    draw::{Arrow, ArrowDrawer, ArrowPath, ArrowStyle, Drawable, LayeredOutput, RenderLayer, Text},
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
    pub fn new(arrow: Arrow, text: Option<Text<'a>>) -> Self {
        Self { arrow, text }
    }

    /// Returns the minimum [`Size`] needed to render this arrow with its text.
    ///
    /// Combines the arrow's minimum size with the text label size.
    pub fn min_size(&self) -> Size {
        let text_size = self.text.as_ref().map(|t| t.size()).unwrap_or_default();
        self.arrow.min_size().max(text_size)
    }

    /// Renders the arrow with optional text to layered output.
    ///
    /// When control points are present in `path`, the arrow follows the
    /// provided Bézier curve and the text label is positioned at the curve's
    /// visual midpoint (`t = 0.5`) rather than the geometric midpoint of the
    /// endpoints.
    ///
    /// # Arguments
    ///
    /// * `arrow_drawer` - Drawer that manages SVG marker generation.
    /// * `path` - The geometric path of the arrow.
    /// * `text_position_override` - Explicit label position. When `None`, the
    ///   renderer computes the position from `path`'s geometry.
    ///
    /// # Returns
    ///
    /// The rendered arrow (and optional label) as a [`LayeredOutput`].
    // TODO: borrowing arrow_drawer is not good in here.
    pub fn render_to_layers(
        &self,
        arrow_drawer: &mut ArrowDrawer,
        path: &ArrowPath,
        text_position_override: Option<Point>,
    ) -> LayeredOutput {
        let mut output = LayeredOutput::new();

        let rendered_arrow = arrow_drawer.draw_arrow(&self.arrow, path);
        output.add_to_layer(RenderLayer::Arrow, rendered_arrow);

        if let Some(text) = &self.text {
            let text_pos = self.calculate_text_position(path, text_position_override);
            let text_output = text.render_to_layers(text_pos);
            output.merge(text_output);
        }

        output
    }

    /// Calculates the position where text should be rendered relative to the arrow.
    ///
    /// If `text_position_override` is `Some`, that explicit position is returned
    /// verbatim. Otherwise, only [`ArrowStyle::Curved`] uses control points for
    /// positioning; other styles always place text at the geometric midpoint of
    /// source and destination.
    fn calculate_text_position(
        &self,
        path: &ArrowPath,
        text_position_override: Option<Point>,
    ) -> Point {
        if self.text.is_none() {
            return Point::zero();
        }

        if let Some(position) = text_position_override {
            return position;
        }

        let source = path.source();
        let destination = path.destination();

        if self.arrow.style() != ArrowStyle::Curved {
            return source.midpoint(destination);
        };

        let control_points = path.control_points();
        match control_points {
            [] => source.midpoint(destination),
            [cp] => quadratic_bezier_midpoint(source, *cp, destination),
            [cp1, cp2] => cubic_bezier_midpoint(source, *cp1, *cp2, destination),
            _ => {
                // For chained curves, approximate by evaluating the midpoint
                // of the middle segment's local neighborhood.
                let mid_idx = control_points.len() / 2;
                let mid_cp = control_points[mid_idx];
                // unwrap_or cases are unreachable when control_points has 3+ elements.
                let before = control_points.get(mid_idx - 1).unwrap_or(&source);
                let after = control_points.get(mid_idx + 1).unwrap_or(&destination);
                quadratic_bezier_midpoint(*before, mid_cp, *after)
            }
        }
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
    /// Creates a new [`ArrowWithTextDrawer`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Draws an arrow with optional text and returns the layered output.
    ///
    /// # Arguments
    ///
    /// * `arrow_with_text` - The arrow and text composite to render.
    /// * `path` - The geometric path of the arrow.
    /// * `text_position_override` - Explicit label position. When `None`, the
    ///   renderer computes the position from `path`'s geometry.
    pub fn draw_arrow_with_text(
        &mut self,
        arrow_with_text: &ArrowWithText,
        path: &ArrowPath,
        text_position_override: Option<Point>,
    ) -> LayeredOutput {
        arrow_with_text.render_to_layers(&mut self.0, path, text_position_override)
    }

    /// Generates SVG marker definitions for all rendered arrows.
    pub fn draw_marker_definitions(&self) -> Box<dyn svg::Node> {
        self.0.draw_marker_definitions()
    }
}

/// An [`ArrowWithText`] paired with its [`ArrowPath`] and an optional explicit
/// text position.
///
/// # Examples
///
/// ```
/// # use std::rc::Rc;
/// # use orrery_core::draw::{Arrow, ArrowDefinition, ArrowDirection, ArrowPath,
/// #     ArrowWithText, PositionedArrowWithText, StrokeDefinition};
/// # use orrery_core::geometry::Point;
/// let stroke = Rc::new(StrokeDefinition::default());
/// let arrow = Arrow::new(Rc::new(ArrowDefinition::new(stroke)), ArrowDirection::Forward);
/// let arrow_with_text = ArrowWithText::new(arrow, None);
/// let path = ArrowPath::straight(Point::new(0.0, 0.0), Point::new(100.0, 0.0));
///
/// let positioned = PositionedArrowWithText::new(arrow_with_text, path);
/// let _override = positioned.with_text_position(Some(Point::new(40.0, 20.0)));
/// ```
#[derive(Debug, Clone)]
pub struct PositionedArrowWithText<'a> {
    arrow_with_text: ArrowWithText<'a>,
    path: ArrowPath,
    /// Explicit position for the text. When `None`, the renderer computes the
    /// position from `path`'s geometry.
    text_position: Option<Point>,
}

impl<'a> PositionedArrowWithText<'a> {
    /// Creates a new positioned arrow with text and no text-position override.
    ///
    /// # Arguments
    ///
    /// * `arrow_with_text` - The visual definition of the arrow.
    /// * `path` - The geometric path of the arrow.
    pub fn new(arrow_with_text: ArrowWithText<'a>, path: ArrowPath) -> Self {
        Self {
            arrow_with_text,
            path,
            text_position: None,
        }
    }

    /// Sets the label-position override.
    ///
    /// Use this when the path's geometric midpoint is not where the label
    /// should appear visually. Pass `None` to let the renderer auto-compute
    /// the position from the path's geometry.
    ///
    /// # Arguments
    ///
    /// * `position` - The explicit position for the label, or `None` to
    ///   the renderer's placement.
    pub fn with_text_position(mut self, position: Option<Point>) -> Self {
        self.text_position = position;
        self
    }

    /// Renders this positioned arrow to layered SVG output.
    ///
    /// # Arguments
    ///
    /// * `drawer` - Manages arrow rendering and SVG marker generation.
    pub fn render_to_layers(&self, drawer: &mut ArrowWithTextDrawer) -> LayeredOutput {
        drawer.draw_arrow_with_text(&self.arrow_with_text, &self.path, self.text_position)
    }
}

/// Evaluates a quadratic bezier curve at t=0.5 (parametric midpoint).
fn quadratic_bezier_midpoint(start: Point, cp: Point, end: Point) -> Point {
    Point::new(
        0.25 * start.x() + 0.5 * cp.x() + 0.25 * end.x(),
        0.25 * start.y() + 0.5 * cp.y() + 0.25 * end.y(),
    )
}

/// Evaluates a cubic bezier curve at t=0.5 (parametric midpoint).
fn cubic_bezier_midpoint(start: Point, cp1: Point, cp2: Point, end: Point) -> Point {
    Point::new(
        0.125 * start.x() + 0.375 * cp1.x() + 0.375 * cp2.x() + 0.125 * end.x(),
        0.125 * start.y() + 0.375 * cp1.y() + 0.375 * cp2.y() + 0.125 * end.y(),
    )
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

    /// Helper function to create a test arrow with a specific style.
    fn create_test_arrow_with_style(direction: ArrowDirection, style: ArrowStyle) -> Arrow {
        let stroke = Rc::new(StrokeDefinition::default());
        let mut definition = ArrowDefinition::new(stroke);
        definition.set_style(style);
        Arrow::new(Rc::new(definition), direction)
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
    fn test_arrow_with_text_size() {
        // Without text, size should reflect the arrow's minimum
        let arrow = create_test_arrow(ArrowDirection::Forward);
        let min_arrow_size = arrow.min_size();
        let arrow_with_text = ArrowWithText::new(arrow, None);
        assert_eq!(arrow_with_text.min_size(), min_arrow_size);
        assert!(arrow_with_text.min_size().height() > 0.0);
        assert!(arrow_with_text.min_size().width() > 0.0);

        // With text, size should be at least the text's size
        let arrow = create_test_arrow(ArrowDirection::Forward);
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Label");
        let text_size = text.size();
        let arrow_with_text = ArrowWithText::new(arrow, Some(text));
        assert!(arrow_with_text.min_size().height() >= text_size.height());
        assert!(arrow_with_text.min_size().width() >= text_size.width());
        assert!(arrow_with_text.min_size().height() > 0.0);
        assert!(arrow_with_text.min_size().width() > 0.0);
    }

    #[test]
    fn test_arrow_with_text_calculate_text_position() {
        let arrow = create_test_arrow(ArrowDirection::Forward);
        let arrow_with_text = ArrowWithText::new(arrow, None);

        // Without text, should return zero point.
        let path = ArrowPath::straight(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
        let pos = arrow_with_text.calculate_text_position(&path, None);
        assert_eq!(pos, Point::zero());

        // With text and no control points, should return midpoint.
        let arrow = create_test_arrow(ArrowDirection::Forward);
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Label");
        let arrow_with_text = ArrowWithText::new(arrow, Some(text));

        let path = ArrowPath::straight(Point::new(0.0, 0.0), Point::new(100.0, 50.0));
        let pos = arrow_with_text.calculate_text_position(&path, None);
        assert_eq!(pos, Point::new(50.0, 25.0));

        // With text and quadratic control point
        let arrow = create_test_arrow(ArrowDirection::Forward);
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Label");
        let arrow_with_text = ArrowWithText::new(arrow, Some(text));

        let path = ArrowPath::new(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            vec![Point::new(50.0, -30.0)],
        );
        let pos = arrow_with_text.calculate_text_position(&path, None);
        // 0.25*0 + 0.5*50 + 0.25*100 = 50, 0.25*0 + 0.5*(-30) + 0.25*0 = -15
        assert_eq!(pos, Point::new(50.0, -15.0));

        // With text and cubic control points
        let arrow = create_test_arrow(ArrowDirection::Forward);
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Label");
        let arrow_with_text = ArrowWithText::new(arrow, Some(text));

        let path = ArrowPath::new(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            vec![Point::new(30.0, -40.0), Point::new(70.0, -40.0)],
        );
        let pos = arrow_with_text.calculate_text_position(&path, None);
        // 0.125*0 + 0.375*30 + 0.375*70 + 0.125*100 = 0+11.25+26.25+12.5 = 50
        // 0.125*0 + 0.375*(-40) + 0.375*(-40) + 0.125*0 = -15-15 = -30
        assert_eq!(pos, Point::new(50.0, -30.0));
    }

    #[test]
    fn test_calculate_text_position_override_takes_precedence() {
        let arrow = create_test_arrow(ArrowDirection::Forward);
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Label");
        let arrow_with_text = ArrowWithText::new(arrow, Some(text));

        // A curved path whose auto-computed midpoint is at (50, -30)...
        let path = ArrowPath::new(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            vec![Point::new(30.0, -40.0), Point::new(70.0, -40.0)],
        );

        // ...is overridden to a custom location.
        let override_pos = Point::new(200.0, 200.0);
        let pos = arrow_with_text.calculate_text_position(&path, Some(override_pos));
        assert_eq!(pos, override_pos);
    }

    #[test]
    fn test_calculate_text_position_straight_ignores_control_points() {
        let arrow = create_test_arrow_with_style(ArrowDirection::Forward, ArrowStyle::Straight);
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Label");
        let arrow_with_text = ArrowWithText::new(arrow, Some(text));

        // Even with control points, Straight style should use geometric midpoint
        let path = ArrowPath::new(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            vec![Point::new(50.0, -30.0)],
        );
        let pos = arrow_with_text.calculate_text_position(&path, None);
        assert_eq!(pos, Point::new(50.0, 0.0));
    }

    #[test]
    fn test_calculate_text_position_orthogonal_ignores_control_points() {
        let arrow = create_test_arrow_with_style(ArrowDirection::Forward, ArrowStyle::Orthogonal);
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Label");
        let arrow_with_text = ArrowWithText::new(arrow, Some(text));

        // Even with control points, Orthogonal style should use geometric midpoint
        let path = ArrowPath::new(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            vec![Point::new(30.0, -40.0), Point::new(70.0, -40.0)],
        );
        let pos = arrow_with_text.calculate_text_position(&path, None);
        assert_eq!(pos, Point::new(50.0, 0.0));
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

        // With text label and control points
        let arrow = create_test_arrow(ArrowDirection::Forward);
        let text_def = TextDefinition::default();
        let text = Text::new(&text_def, "Label");
        let arrow_with_text = ArrowWithText::new(arrow, Some(text));
        let cp = Point::new(50.0, -20.0);

        let path = ArrowPath::new(source, destination, vec![cp]);
        let output = arrow_with_text.render_to_layers(&mut arrow_drawer, &path, None);
        assert!(!output.is_empty());

        // Without text label - should still have arrow layer
        let arrow = create_test_arrow(ArrowDirection::Forward);
        let arrow_with_text = ArrowWithText::new(arrow, None);

        let path = ArrowPath::straight(source, destination);
        let output = arrow_with_text.render_to_layers(&mut arrow_drawer, &path, None);
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

            let path = ArrowPath::straight(source, destination);
            let output = arrow_with_text.render_to_layers(&mut arrow_drawer, &path, None);
            assert!(
                !output.is_empty(),
                "Rendering failed for direction: {:?}",
                direction
            );
        }
    }

    #[test]
    fn test_quadratic_bezier_midpoint() {
        // Symmetric curve: midpoint should be directly above the line midpoint
        let start = Point::new(0.0, 0.0);
        let cp = Point::new(50.0, -60.0);
        let end = Point::new(100.0, 0.0);

        let mid = quadratic_bezier_midpoint(start, cp, end);
        // 0.25*0 + 0.5*50 + 0.25*100 = 50
        // 0.25*0 + 0.5*(-60) + 0.25*0 = -30
        assert_eq!(mid, Point::new(50.0, -30.0));

        // Control point on the line: midpoint should equal the straight midpoint
        let cp_on_line = Point::new(50.0, 25.0);
        let mid = quadratic_bezier_midpoint(start, cp_on_line, Point::new(100.0, 50.0));
        // 0.25*0 + 0.5*50 + 0.25*100 = 50
        // 0.25*0 + 0.5*25 + 0.25*50 = 25
        assert_eq!(mid, Point::new(50.0, 25.0));

        // Degenerate: all points the same
        let p = Point::new(10.0, 20.0);
        let mid = quadratic_bezier_midpoint(p, p, p);
        assert_eq!(mid, p);
    }

    #[test]
    fn test_cubic_bezier_midpoint() {
        // Symmetric S-curve
        let start = Point::new(0.0, 0.0);
        let cp1 = Point::new(30.0, -40.0);
        let cp2 = Point::new(70.0, -40.0);
        let end = Point::new(100.0, 0.0);

        let mid = cubic_bezier_midpoint(start, cp1, cp2, end);
        // 0.125*0 + 0.375*30 + 0.375*70 + 0.125*100 = 11.25 + 26.25 + 12.5 = 50
        // 0.125*0 + 0.375*(-40) + 0.375*(-40) + 0.125*0 = -15 + -15 = -30
        assert_eq!(mid, Point::new(50.0, -30.0));

        // Control points on the line: should equal straight midpoint
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 100.0);
        let cp1 = Point::new(25.0, 25.0);
        let cp2 = Point::new(75.0, 75.0);

        let mid = cubic_bezier_midpoint(start, cp1, cp2, end);
        // 0.125*0 + 0.375*25 + 0.375*75 + 0.125*100 = 9.375 + 28.125 + 12.5 = 50
        assert_eq!(mid, Point::new(50.0, 50.0));

        // Degenerate: all points the same
        let p = Point::new(5.0, 5.0);
        let mid = cubic_bezier_midpoint(p, p, p, p);
        assert_eq!(mid, p);
    }
}
