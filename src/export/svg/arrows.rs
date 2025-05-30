use crate::{
    ast::{ArrowStyle, RelationType},
    color::Color,
    layout::Point,
};
use svg::node::element::{Definitions, Marker, Path};

/// Creates marker definitions for SVG arrows based on the colors in use
pub fn create_marker_definitions<'a, I>(colors: I) -> Definitions
where
    I: Iterator<Item = &'a Color>,
{
    let mut defs = Definitions::new();

    // Create markers for each color
    for color in colors {
        // Right-pointing arrow marker for this color
        let arrow_right = Marker::new()
            .set("id", format!("arrow-right-{}", color.to_id_safe_string()))
            .set("viewBox", "0 0 10 10")
            .set("refX", 9)
            .set("refY", 5)
            .set("markerWidth", 6)
            .set("markerHeight", 6)
            .set("orient", "auto")
            .add(
                Path::new()
                    .set("d", "M 0 0 L 10 5 L 0 10 z")
                    .set("fill", color.to_string()),
            );

        // Left-pointing arrow marker for this color
        let arrow_left = Marker::new()
            .set("id", format!("arrow-left-{}", color.to_id_safe_string()))
            .set("viewBox", "0 0 10 10")
            .set("refX", 1)
            .set("refY", 5)
            .set("markerWidth", 6)
            .set("markerHeight", 6)
            .set("orient", "auto")
            .add(
                Path::new()
                    .set("d", "M 10 0 L 0 5 L 10 10 z")
                    .set("fill", color.to_string()),
            );

        defs = defs.add(arrow_right).add(arrow_left);
    }

    defs
}

/// Get marker references for a specific relation type and color
pub fn get_markers_for_relation(
    relation_type: &RelationType,
    color: &Color,
) -> (Option<String>, Option<String>) {
    match relation_type {
        RelationType::Forward => (
            None,
            Some(format!("url(#arrow-right-{})", color.to_id_safe_string())),
        ),
        RelationType::Backward => (
            Some(format!("url(#arrow-left-{})", color.to_id_safe_string())),
            None,
        ),
        RelationType::Bidirectional => (
            Some(format!("url(#arrow-left-{})", color.to_id_safe_string())),
            Some(format!("url(#arrow-right-{})", color.to_id_safe_string())),
        ),
        RelationType::Plain => (None, None),
    }
}

/// Create a path data string for the given arrow style
pub fn create_path_data_for_style(start: Point, end: Point, style: &ArrowStyle) -> String {
    match style {
        ArrowStyle::Straight => create_path_data_from_points(start, end),
        ArrowStyle::Curved => create_curved_path_data_from_points(start, end),
        ArrowStyle::Orthogonal => create_orthogonal_path_data_from_points(start, end),
    }
}

/// Create a path data string from two points
pub fn create_path_data_from_points(start: Point, end: Point) -> String {
    format!("M {} {} L {} {}", start.x(), start.y(), end.x(), end.y())
}

/// Create a curved path data string from two points
/// Creates a cubic bezier curve with control points positioned to create a nice arc
pub fn create_curved_path_data_from_points(start: Point, end: Point) -> String {
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
pub fn create_orthogonal_path_data_from_points(start: Point, end: Point) -> String {
    // Determine whether to go horizontal first then vertical, or vertical first then horizontal
    // This decision is based on the relative positions of the start and end points

    let abs_dist = end.sub(start).abs();
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

/// Create a path for connecting two points with appropriate markers
pub fn create_path(
    start: Point,
    end: Point,
    relation_type: &RelationType,
    color: &Color,
    width: usize,
    arrow_style: &ArrowStyle,
) -> Path {
    // Generate path data based on arrow style
    let path_data = create_path_data_for_style(start, end, arrow_style);

    // Create the path with the generated data
    let mut path = Path::new()
        .set("d", path_data)
        .set("fill", "none")
        .set("stroke", color.to_string())
        .set("stroke-width", width);

    // Get marker references for this specific color
    let (start_marker, end_marker) = get_markers_for_relation(relation_type, color);

    // Add markers if they exist
    if let Some(marker) = start_marker {
        path = path.set("marker-start", marker);
    }

    if let Some(marker) = end_marker {
        path = path.set("marker-end", marker);
    }

    path
}
