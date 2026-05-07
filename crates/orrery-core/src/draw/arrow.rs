//! Arrow drawable types and SVG marker generation.
//!
//! This module provides types for defining and rendering arrows in diagrams,
//! including stroke styling, path shapes, direction markers, and SVG output.

use std::{collections::HashMap, fmt, rc::Rc, str};

use svg::{self, node::element as svg_element};

use crate::{
    color::Color,
    draw::{StrokeDefinition, TextDefinition},
    geometry::{Point, Size},
};

/// Defines the visual style of arrow paths.
///
/// # Variants
///
/// - `Straight`: Creates direct line segments between points
/// - `Curved`: Creates smooth bezier curves between points
/// - `Orthogonal`: Creates only horizontal and vertical line segments
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ArrowStyle {
    Straight,
    #[default]
    Curved,
    Orthogonal,
}

impl str::FromStr for ArrowStyle {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "straight" => Ok(Self::Straight),
            "curved" => Ok(Self::Curved),
            "orthogonal" => Ok(Self::Orthogonal),
            _ => Err("Invalid arrow style"),
        }
    }
}

/// Defines the visual properties of an arrow.
///
/// This struct encapsulates all the styling information needed to render
/// an arrow, including stroke properties and path style.
#[derive(Debug, Clone)]
pub struct ArrowDefinition {
    stroke: Rc<StrokeDefinition>,
    style: ArrowStyle,
    text: Rc<TextDefinition>,
}

impl ArrowDefinition {
    /// Creates a new ArrowDefinition with the given stroke
    /// Style defaults to Straight and can be changed with set_style()
    pub fn new(stroke: Rc<StrokeDefinition>) -> Self {
        Self {
            stroke,
            style: ArrowStyle::default(),
            text: Rc::new(TextDefinition::default()),
        }
    }

    /// Gets the arrow stroke definition
    pub fn stroke(&self) -> &Rc<StrokeDefinition> {
        &self.stroke
    }

    /// Gets the arrow style
    pub fn style(&self) -> &ArrowStyle {
        &self.style
    }

    /// Sets the arrow style
    pub fn set_style(&mut self, style: ArrowStyle) {
        self.style = style;
    }

    /// Gets the text definition.
    pub fn text(&self) -> &Rc<TextDefinition> {
        &self.text
    }

    /// Set text definition using Rc.
    pub fn set_text(&mut self, text: Rc<TextDefinition>) {
        self.text = text;
    }

    /// Set stroke definition using Rc.
    pub fn set_stroke(&mut self, stroke: Rc<StrokeDefinition>) {
        self.stroke = stroke;
    }
}

impl Default for ArrowDefinition {
    fn default() -> Self {
        let mut text_def = TextDefinition::default();
        text_def.set_background_color(Some(
            Color::new("rgba(255, 255, 255, 0.85)").expect("valid color"),
        ));
        Self {
            stroke: Rc::new(StrokeDefinition::default()),
            style: ArrowStyle::default(),
            text: Rc::new(text_def),
        }
    }
}

/// Defines the direction of arrow markers.
///
/// - `Forward`: Creates `->` arrows pointing from source to destination
/// - `Backward`: Creates `<-` arrows pointing from destination to source
/// - `Bidirectional`: Creates `<->` arrows with markers at both ends
/// - `Plain`: Creates `-` simple lines without arrow markers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowDirection {
    Forward,       // ->
    Backward,      // <-
    Bidirectional, // <->
    Plain,         // -
}

impl ArrowDirection {
    fn to_string(self) -> &'static str {
        match self {
            Self::Forward => "->",
            Self::Backward => "<-",
            Self::Bidirectional => "<->",
            Self::Plain => "-",
        }
    }
}

impl str::FromStr for ArrowDirection {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "->" => Ok(Self::Forward),
            "<-" => Ok(Self::Backward),
            "<->" => Ok(Self::Bidirectional),
            "-" => Ok(Self::Plain),
            _ => Err("Invalid arrow direction"),
        }
    }
}

impl fmt::Display for ArrowDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).to_string())
    }
}

/// The geometric path of an arrow: source, destination, and optional control points.
///
/// For straight arrows, `control_points` is empty. For curved arrows (bezier paths),
/// `control_points` contains the intermediate curve points.
///
/// # Examples
///
/// ```
/// # use orrery_core::geometry::Point;
/// # use orrery_core::draw::ArrowPath;
/// // A straight arrow from (0,0) to (100,50)
/// let path = ArrowPath::straight(Point::new(0.0, 0.0), Point::new(100.0, 50.0));
///
/// // A curved arrow with one control point (quadratic bezier)
/// let curved = ArrowPath::new(
///     Point::new(0.0, 0.0),
///     Point::new(100.0, 0.0),
///     vec![Point::new(50.0, -30.0)],
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ArrowPath {
    source: Point,
    destination: Point,
    control_points: Vec<Point>,
}

impl ArrowPath {
    /// Creates a new arrow path.
    ///
    /// # Arguments
    ///
    /// * `source` - The starting point of the arrow.
    /// * `destination` - The ending point of the arrow.
    /// * `control_points` - Bezier control points for curved paths. Empty for straight arrows.
    pub fn new(source: Point, destination: Point, control_points: Vec<Point>) -> Self {
        Self {
            source,
            destination,
            control_points,
        }
    }

    /// Creates a straight arrow path.
    ///
    /// # Arguments
    ///
    /// * `source` - The starting point of the arrow.
    /// * `destination` - The ending point of the arrow.
    pub fn straight(source: Point, destination: Point) -> Self {
        Self {
            source,
            destination,
            control_points: Vec::new(),
        }
    }

    /// Returns the starting point of the arrow.
    pub fn source(&self) -> Point {
        self.source
    }

    /// Returns the ending point of the arrow.
    pub fn destination(&self) -> Point {
        self.destination
    }

    /// Returns the control points for curved paths.
    pub fn control_points(&self) -> &[Point] {
        &self.control_points
    }
}

/// A drawable arrow with styling and direction markers.
///
/// An Arrow combines an `ArrowDefinition` (containing visual properties
/// like color, width, and style) with an `ArrowDirection` that determines
/// which markers to display and where.
#[derive(Debug, Clone)]
pub struct Arrow {
    definition: Rc<ArrowDefinition>,
    direction: ArrowDirection,
}

/// Manages arrow rendering and SVG marker generation.
///
/// The ArrowDrawer collects color information from arrows to generate
/// the necessary SVG marker definitions upfront, which can then be
/// referenced by individual arrow elements.
#[derive(Debug, Default)]
pub struct ArrowDrawer {
    heads: HashMap<String, Color>,
    tails: HashMap<String, Color>,
}

impl ArrowDrawer {
    /// Draws an arrow and collects its color for marker generation.
    ///
    /// Only arrows with [`ArrowStyle::Curved`] use the path's control points.
    /// Other styles ([`ArrowStyle::Straight`], [`ArrowStyle::Orthogonal`]) ignore them entirely.
    ///
    /// # Arguments
    ///
    /// * `arrow` - The arrow to render.
    /// * `path` - The geometric path of the arrow.
    pub fn draw_arrow(&mut self, arrow: &Arrow, path: &ArrowPath) -> Box<dyn svg::Node> {
        self.register_arrow_markers(arrow);
        arrow.render_to_svg(path)
    }

    /// Generates SVG marker definitions for all collected arrow colors.
    pub fn draw_marker_definitions(&self) -> Box<dyn svg::Node> {
        let mut defs = svg_element::Definitions::new();
        for color in self.heads.values() {
            defs = defs.add(Arrow::create_arrow_left(*color));
        }
        for color in self.tails.values() {
            defs = defs.add(Arrow::create_arrow_right(*color));
        }
        defs.into()
    }

    fn register_arrow_markers(&mut self, arrow: &Arrow) {
        let color = arrow.definition.stroke().color();
        let (head, tail) = Arrow::get_markers(arrow.direction, color);
        if let Some(head) = head {
            self.heads.insert(head, color);
        }
        if let Some(tail) = tail {
            self.tails.insert(tail, color);
        }
    }
}

/// Size of the SVG arrow markers (matches `markerWidth`/`markerHeight` attributes).
///
/// Markers are square, so this applies to both dimensions.
const MARKER_SIZE: f32 = 6.0;

impl Arrow {
    /// Creates a new [`Arrow`] with the given definition and direction.
    pub fn new(definition: Rc<ArrowDefinition>, direction: ArrowDirection) -> Self {
        Self {
            definition,
            direction,
        }
    }

    /// Returns the arrow's [`ArrowStyle`].
    pub fn style(&self) -> ArrowStyle {
        self.definition.style
    }

    /// Returns the minimum [`Size`] needed to render this arrow.
    pub fn min_size(&self) -> Size {
        let (marker_width, marker_height) = match self.direction {
            ArrowDirection::Forward | ArrowDirection::Backward => (MARKER_SIZE, MARKER_SIZE),
            ArrowDirection::Bidirectional => (2.0 * MARKER_SIZE, MARKER_SIZE),
            ArrowDirection::Plain => (0.0, 0.0),
        };
        let stroke_width = self.definition.stroke().width();
        Size::new(marker_width, marker_height.max(stroke_width))
    }

    /// Renders this arrow to an SVG path element.
    ///
    /// Only [`ArrowStyle::Curved`] respects external `control_points`. When
    /// no control points are provided, it falls back to a straight line.
    /// Other styles (`Straight`, `Orthogonal`) ignore control points entirely.
    fn render_to_svg(&self, path: &ArrowPath) -> Box<dyn svg::Node> {
        let path_data = match self.definition.style {
            ArrowStyle::Curved => Self::curved_path_data(path),
            ArrowStyle::Straight => Self::straight_path_data(path.source(), path.destination()),
            ArrowStyle::Orthogonal => Self::orthogonal_path_data(path.source(), path.destination()),
        };

        let color = self.definition.stroke().color();

        // Create the base path
        let path = svg_element::Path::new()
            .set("d", path_data)
            .set("fill", "none");

        let mut path = crate::apply_stroke!(path, self.definition.stroke());

        // Get marker references for this specific color and direction
        let (start_marker, end_marker) = Self::get_markers(self.direction, color);

        // Add markers if they exist
        if let Some(marker) = start_marker {
            path = path.set("marker-start", marker);
        }

        if let Some(marker) = end_marker {
            path = path.set("marker-end", marker);
        }

        Box::new(path)
    }

    fn marker_left_id(color: Color) -> String {
        format!("arrow-left-{}", color.to_id_safe_string())
    }

    fn marker_right_id(color: Color) -> String {
        format!("arrow-right-{}", color.to_id_safe_string())
    }

    /// Get marker references for a specific relation type and color
    fn get_markers(direction: ArrowDirection, color: Color) -> (Option<String>, Option<String>) {
        match direction {
            ArrowDirection::Forward => (
                None,
                Some(format!("url(#{})", Self::marker_right_id(color))),
            ),
            ArrowDirection::Backward => {
                (Some(format!("url(#{})", Self::marker_left_id(color))), None)
            }
            ArrowDirection::Bidirectional => (
                Some(format!("url(#{})", Self::marker_left_id(color))),
                Some(format!("url(#{})", Self::marker_right_id(color))),
            ),
            ArrowDirection::Plain => (None, None),
        }
    }

    /// Creates an SVG path data string from external control points.
    ///
    /// Returns a straight line if `control_points` is empty. Otherwise, the
    /// slice length determines the curve type:
    /// - 1 point: quadratic bezier (SVG `Q` command).
    /// - 2 points: cubic bezier (SVG `C` command).
    /// - 3+ points: chained cubic bezier segments with midpoint anchors.
    ///
    /// For 3+ points, control points are grouped into pairs. Each pair defines
    /// one cubic bezier segment. Intermediate anchor points (where segments join)
    /// are computed as midpoints between consecutive control points at segment
    /// boundaries. If the count is odd, the final segment is a quadratic bezier.
    ///
    /// See [SVG curve commands](https://developer.mozilla.org/en-US/docs/Web/SVG/Tutorial/Paths#curve_commands)
    /// for details on how bezier control points map to SVG path data.
    fn curved_path_data(path: &ArrowPath) -> String {
        if path.control_points().is_empty() {
            return Self::straight_path_data(path.source(), path.destination());
        }

        let control_points = path.control_points();
        let end = path.destination();
        let mut d = format!("M {} {}", path.source().x(), path.source().y());
        let mut i = 0;
        let len = control_points.len();

        while i < len {
            let remaining = len - i;
            if remaining >= 2 {
                let cp1 = control_points[i];
                let cp2 = control_points[i + 1];
                // Determine the endpoint of this segment
                let segment_end = if i + 2 < len {
                    // More points follow: anchor at midpoint between cp2 and next cp
                    cp2.midpoint(control_points[i + 2])
                } else {
                    // Last pair: end at the destination
                    end
                };
                d.push_str(&format!(
                    " C {} {}, {} {}, {} {}",
                    cp1.x(),
                    cp1.y(),
                    cp2.x(),
                    cp2.y(),
                    segment_end.x(),
                    segment_end.y()
                ));
                i += 2;
            } else {
                // Odd trailing point: quadratic bezier to destination
                let cp = control_points[i];
                d.push_str(&format!(
                    " Q {} {}, {} {}",
                    cp.x(),
                    cp.y(),
                    end.x(),
                    end.y()
                ));
                i += 1;
            }
        }

        d
    }

    /// Creates a straight-line path data string from two points.
    pub fn straight_path_data(start: Point, end: Point) -> String {
        format!("M {} {} L {} {}", start.x(), start.y(), end.x(), end.y())
    }

    /// Creates an orthogonal path data string from two points.
    ///
    /// Produces a path with only horizontal and vertical line segments.
    fn orthogonal_path_data(start: Point, end: Point) -> String {
        // Determine whether to go horizontal first then vertical, or vertical first then horizontal
        // This decision is based on the relative positions of the start and end points

        let abs_dist = end.sub_point(start).abs();
        let mid = start.midpoint(end);

        // If we're more horizontal than vertical, go horizontal first
        if abs_dist.x() > abs_dist.y() {
            format!(
                "M {} {} L {} {} L {} {} L {} {}",
                start.x(),
                start.y(),
                mid.x(),
                start.y(),
                mid.x(),
                end.y(),
                end.x(),
                end.y()
            )
        } else {
            format!(
                "M {} {} L {} {} L {} {} L {} {}",
                start.x(),
                start.y(),
                start.x(),
                mid.y(),
                end.x(),
                mid.y(),
                end.x(),
                end.y()
            )
        }
    }

    fn create_arrow_right(color: Color) -> svg_element::Marker {
        svg_element::Marker::new()
            .set("id", Self::marker_right_id(color))
            .set("viewBox", "0 0 10 10")
            .set("refX", 9)
            .set("refY", 5)
            .set("markerWidth", MARKER_SIZE)
            .set("markerHeight", MARKER_SIZE)
            .set("orient", "auto")
            .add(
                svg_element::Path::new()
                    .set("d", "M 0 0 L 10 5 L 0 10 z")
                    .set("fill", color.to_string())
                    .set("fill-opacity", color.alpha()),
            )
    }

    fn create_arrow_left(color: Color) -> svg_element::Marker {
        svg_element::Marker::new()
            .set("id", Self::marker_left_id(color))
            .set("viewBox", "0 0 10 10")
            .set("refX", 1)
            .set("refY", 5)
            .set("markerWidth", MARKER_SIZE)
            .set("markerHeight", MARKER_SIZE)
            .set("orient", "auto")
            .add(
                svg_element::Path::new()
                    .set("d", "M 10 0 L 0 5 L 10 10 z")
                    .set("fill", color.to_string())
                    .set("fill-opacity", color.alpha()),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arrow_style_from_str_valid() {
        let straight: ArrowStyle = "straight".parse().unwrap();
        assert_eq!(straight, ArrowStyle::Straight);

        let curved: ArrowStyle = "curved".parse().unwrap();
        assert_eq!(curved, ArrowStyle::Curved);

        let orthogonal: ArrowStyle = "orthogonal".parse().unwrap();
        assert_eq!(orthogonal, ArrowStyle::Orthogonal);
    }

    #[test]
    fn test_arrow_style_from_str_invalid() {
        let result: Result<ArrowStyle, _> = "invalid".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_arrow_definition_setters() {
        // Test set_style
        let stroke = Rc::new(StrokeDefinition::default());
        let mut def = ArrowDefinition::new(stroke);
        def.set_style(ArrowStyle::Curved);
        assert_eq!(*def.style(), ArrowStyle::Curved);
        def.set_style(ArrowStyle::Orthogonal);
        assert_eq!(*def.style(), ArrowStyle::Orthogonal);

        // Test set_stroke
        let mut new_stroke = StrokeDefinition::default();
        new_stroke.set_width(5.0);
        def.set_stroke(Rc::new(new_stroke));
        assert_eq!(def.stroke().width(), 5.0);

        // Test set_text
        let new_text = Rc::new(TextDefinition::new());
        def.set_text(new_text.clone());
        assert!(Rc::ptr_eq(def.text(), &new_text));
    }

    #[test]
    fn test_arrow_direction_from_str_valid() {
        let forward: ArrowDirection = "->".parse().unwrap();
        assert_eq!(forward, ArrowDirection::Forward);

        let backward: ArrowDirection = "<-".parse().unwrap();
        assert_eq!(backward, ArrowDirection::Backward);

        let bidirectional: ArrowDirection = "<->".parse().unwrap();
        assert_eq!(bidirectional, ArrowDirection::Bidirectional);

        let plain: ArrowDirection = "-".parse().unwrap();
        assert_eq!(plain, ArrowDirection::Plain);
    }

    #[test]
    fn test_arrow_min_size() {
        let default_def = Rc::new(ArrowDefinition::new(Rc::new(StrokeDefinition::default())));

        // Forward: one marker width, marker height dominates default stroke (1.0)
        let arrow = Arrow::new(Rc::clone(&default_def), ArrowDirection::Forward);
        assert_eq!(arrow.min_size(), Size::new(MARKER_SIZE, MARKER_SIZE));

        // Backward: same as forward
        let arrow = Arrow::new(Rc::clone(&default_def), ArrowDirection::Backward);
        assert_eq!(arrow.min_size(), Size::new(MARKER_SIZE, MARKER_SIZE));

        // Bidirectional: double marker width
        let arrow = Arrow::new(Rc::clone(&default_def), ArrowDirection::Bidirectional);
        assert_eq!(arrow.min_size(), Size::new(2.0 * MARKER_SIZE, MARKER_SIZE));

        // Plain: no markers, height is just stroke width
        let arrow = Arrow::new(Rc::clone(&default_def), ArrowDirection::Plain);
        assert_eq!(arrow.min_size(), Size::new(0.0, 1.0));

        // Thick stroke: stroke (10.0) dominates marker height
        let mut thick_stroke = StrokeDefinition::default();
        thick_stroke.set_width(10.0);
        let thick_def = Rc::new(ArrowDefinition::new(Rc::new(thick_stroke)));
        let arrow = Arrow::new(thick_def, ArrowDirection::Forward);
        assert_eq!(arrow.min_size(), Size::new(MARKER_SIZE, 10.0));
    }

    #[test]
    fn test_arrow_direction_from_str_invalid() {
        let result: Result<ArrowDirection, _> = ">>".parse();
        assert!(result.is_err());

        let result: Result<ArrowDirection, _> = "invalid".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_arrow_direction_display() {
        assert_eq!(format!("{}", ArrowDirection::Forward), "->");
        assert_eq!(format!("{}", ArrowDirection::Backward), "<-");
        assert_eq!(format!("{}", ArrowDirection::Bidirectional), "<->");
        assert_eq!(format!("{}", ArrowDirection::Plain), "-");
    }

    #[test]
    fn test_straight_path_data() {
        let start = Point::new(10.0, 20.0);
        let end = Point::new(100.0, 50.0);

        let path = Arrow::straight_path_data(start, end);

        assert_eq!(path, "M 10 20 L 100 50");
    }

    #[test]
    fn test_curved_path_data_empty_falls_back_to_straight() {
        let path = ArrowPath::straight(Point::new(0.0, 0.0), Point::new(100.0, 50.0));

        let result = Arrow::curved_path_data(&path);
        assert_eq!(result, "M 0 0 L 100 50");
    }

    #[test]
    fn test_curved_path_data_quadratic_bezier() {
        let path = ArrowPath::new(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            vec![Point::new(50.0, -30.0)],
        );

        let result = Arrow::curved_path_data(&path);
        assert_eq!(result, "M 0 0 Q 50 -30, 100 0");
    }

    #[test]
    fn test_curved_path_data_cubic_bezier() {
        let path = ArrowPath::new(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            vec![Point::new(30.0, -40.0), Point::new(70.0, -40.0)],
        );

        let result = Arrow::curved_path_data(&path);
        assert_eq!(result, "M 0 0 C 30 -40, 70 -40, 100 0");
    }

    #[test]
    fn test_curved_path_data_chained_cubic_even() {
        // 4 control points = 2 cubic segments
        let path = ArrowPath::new(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            vec![
                Point::new(10.0, 20.0),
                Point::new(30.0, 40.0),
                Point::new(60.0, 40.0),
                Point::new(80.0, 20.0),
            ],
        );

        let result = Arrow::curved_path_data(&path);

        // First segment: start -> midpoint(cp2, cp3) with cp1, cp2 as control points
        // midpoint(30,40 and 60,40) = (45, 40)
        // Second segment: midpoint -> end with cp3, cp4 as control points
        assert_eq!(result, "M 0 0 C 10 20, 30 40, 45 40 C 60 40, 80 20, 100 0");
    }

    #[test]
    fn test_curved_path_data_chained_odd() {
        // 3 control points = 1 cubic segment + 1 quadratic segment
        let path = ArrowPath::new(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            vec![
                Point::new(20.0, -30.0),
                Point::new(50.0, -30.0),
                Point::new(80.0, -10.0),
            ],
        );

        let result = Arrow::curved_path_data(&path);

        // First segment: cubic with cp1, cp2, ending at midpoint(cp2, cp3) = (65, -20)
        // Second segment: quadratic with cp3, ending at destination
        assert_eq!(result, "M 0 0 C 20 -30, 50 -30, 65 -20 Q 80 -10, 100 0");
    }

    #[test]
    fn test_draw_arrow_with_control_points() {
        let mut drawer = ArrowDrawer::default();
        let stroke = Rc::new(StrokeDefinition::default());
        let def = Rc::new(ArrowDefinition::new(stroke));
        let arrow = Arrow::new(def, ArrowDirection::Forward);

        let source = Point::new(0.0, 0.0);
        let destination = Point::new(100.0, 50.0);
        let cp = Point::new(50.0, -20.0);

        // Should not panic with control points
        let path = ArrowPath::new(source, destination, vec![cp]);
        let _node = drawer.draw_arrow(&arrow, &path);

        // Should also work with empty (fallback)
        let path = ArrowPath::straight(source, destination);
        let _node = drawer.draw_arrow(&arrow, &path);
    }
}
