use std::{borrow::Cow, collections::HashMap, fmt, str};

use svg::{self, node::element as svg_element};

use crate::{color::Color, draw::StrokeDefinition, geometry::Point};

/// Defines the visual style of arrow paths.
///
/// # Variants
///
/// - `Straight`: Creates direct line segments between points
/// - `Curved`: Creates smooth bezier curves between points
/// - `Orthogonal`: Creates only horizontal and vertical line segments
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ArrowStyle {
    #[default]
    Straight,
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
    stroke: Cow<'static, StrokeDefinition>,
    style: ArrowStyle,
}

impl ArrowDefinition {
    /// Creates a new ArrowDefinition with the given stroke
    /// Style defaults to Straight and can be changed with set_style()
    pub fn new(stroke: Cow<'static, StrokeDefinition>) -> Self {
        Self {
            stroke,
            style: ArrowStyle::default(),
        }
    }

    /// Gets the arrow stroke definition
    pub fn stroke(&self) -> &StrokeDefinition {
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
}

impl Default for ArrowDefinition {
    fn default() -> Self {
        Self {
            stroke: Cow::Borrowed(StrokeDefinition::default_borrowed()),
            style: ArrowStyle::default(),
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

/// Represents a drawable arrow with styling and specific direction.
///
/// An Arrow combines an `ArrowDefinition` (containing visual properties
/// like color, width, and style) with an `ArrowDirection` that determines
/// which markers to display and where.
#[derive(Debug, Clone)]
pub struct Arrow {
    definition: Cow<'static, ArrowDefinition>,
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
    /// Creates a new ArrowDrawer
    pub fn new() -> Self {
        Self::default()
    }

    /// Draws an arrow and collects its color for marker generation
    pub fn draw_arrow(
        &mut self,
        arrow: &Arrow,
        source: Point,
        destination: Point,
    ) -> Box<dyn svg::Node> {
        self.register_arrow_markers(arrow);
        arrow.render_to_svg(source, destination)
    }

    /// Generates SVG marker definitions for all collected colors
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

impl Arrow {
    /// Creates a new Arrow
    pub fn new(definition: Cow<'static, ArrowDefinition>, direction: ArrowDirection) -> Self {
        Self {
            definition,
            direction,
        }
    }

    fn render_to_svg(&self, source: Point, destination: Point) -> Box<dyn svg::Node> {
        // Create path data based on arrow style
        let path_data =
            Self::create_path_data_for_style(source, destination, self.definition.style);

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

    /// Create a path data string for the given arrow style
    fn create_path_data_for_style(start: Point, end: Point, style: ArrowStyle) -> String {
        match style {
            ArrowStyle::Straight => Self::create_path_data_from_points(start, end),
            ArrowStyle::Curved => Self::create_curved_path_data_from_points(start, end),
            ArrowStyle::Orthogonal => Self::create_orthogonal_path_data_from_points(start, end),
        }
    }

    /// Create a path data string from two points
    pub fn create_path_data_from_points(start: Point, end: Point) -> String {
        format!("M {} {} L {} {}", start.x(), start.y(), end.x(), end.y())
    }

    /// Create a curved path data string from two points
    /// Creates a cubic bezier curve with control points positioned to create a nice arc
    fn create_curved_path_data_from_points(start: Point, end: Point) -> String {
        // For the control points, we'll use points positioned to create a smooth arc
        // between the start and end points
        let ctrl1_x = start.x() + (end.x() - start.x()) / 4.0;
        let ctrl1_y = start.y() - (end.y() - start.y()) / 2.0;

        let ctrl2_x = end.x() - (end.x() - start.x()) / 4.0;
        let ctrl2_y = end.y() + (start.y() - end.y()) / 2.0;

        format!(
            "M {} {} C {} {}, {} {}, {} {}",
            start.x(),
            start.y(),
            ctrl1_x,
            ctrl1_y,
            ctrl2_x,
            ctrl2_y,
            end.x(),
            end.y()
        )
    }

    /// Create an orthogonal path data string from two points
    /// Creates a path with only horizontal and vertical line segments
    fn create_orthogonal_path_data_from_points(start: Point, end: Point) -> String {
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
            .set("markerWidth", 6)
            .set("markerHeight", 6)
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
            .set("markerWidth", 6)
            .set("markerHeight", 6)
            .set("orient", "auto")
            .add(
                svg_element::Path::new()
                    .set("d", "M 10 0 L 0 5 L 10 10 z")
                    .set("fill", color.to_string())
                    .set("fill-opacity", color.alpha()),
            )
    }
}
