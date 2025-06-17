use crate::{
    ast,
    layout::{Point, Size},
    shape::Shape,
};
use svg::node::element::{Ellipse, Group, Rectangle as SvgRectangle, Text};

// Constants for rendering configuration
const DEFAULT_TEXT_PADDING_FROM_TOP: f32 = 20.0;
const OVAL_TEXT_POSITION_FACTOR: f32 = 0.5;
const DEFAULT_FONT_FAMILY: &str = "Arial";

/// Trait for rendering shapes to SVG
pub trait ShapeRenderer {
    /// Render a shape to SVG based on the given properties
    ///
    /// * `position` - The center position of the shape
    /// * `size` - The size of the shape
    /// * `type_def` - The type definition containing styling information
    /// * `text` - The text to display (component name)
    /// * `has_nested_blocks` - Whether this component contains nested blocks
    fn render_to_svg(
        &self,
        position: Point,
        size: Size,
        type_def: &ast::TypeDefinition,
        text: &str,
        has_nested_blocks: bool,
    ) -> Group;
}

/// Helper function to get a shape renderer based on the shape type
pub fn get_renderer(shape_type: &Shape) -> &'static dyn ShapeRenderer {
    match shape_type.name() {
        "Rectangle" => &RECTANGLE_RENDERER,
        "Oval" => &OVAL_RENDERER,
        _ => &RECTANGLE_RENDERER, // Default to rectangle if unknown
    }
}

struct RectangleRenderer;
struct OvalRenderer;

static RECTANGLE_RENDERER: RectangleRenderer = RectangleRenderer;
static OVAL_RENDERER: OvalRenderer = OvalRenderer;

// Implement ShapeRenderer directly for RectangleShape
impl ShapeRenderer for RectangleRenderer {
    fn render_to_svg(
        &self,
        position: Point,
        size: Size,
        type_def: &ast::TypeDefinition,
        text: &str,
        has_nested_blocks: bool,
    ) -> Group {
        let group = Group::new();

        // Calculate the actual top-left position for the rectangle
        // (position is the center of the component)
        let rect_bounds = position.to_bounds(size);

        // Main rectangle
        let mut rect = SvgRectangle::new()
            .set("x", rect_bounds.min_x())
            .set("y", rect_bounds.min_y())
            .set("width", size.width())
            .set("height", size.height())
            .set("stroke", type_def.line_color.to_string())
            .set("stroke-width", type_def.line_width)
            .set("fill", "white")
            .set("rx", type_def.rounded);

        if let Some(fill_color) = &type_def.fill_color {
            rect = rect.set("fill", fill_color.to_string());
        }

        // Component name
        // If this component has nested blocks, position the text near the top
        // otherwise place it in the center of the rectangle
        let text_y = if has_nested_blocks {
            // Position text near the top with a small padding
            rect_bounds.min_y() + DEFAULT_TEXT_PADDING_FROM_TOP
        } else {
            // Center the text vertically
            position.y()
        };

        let text_element = Text::new(text)
            .set("x", position.x())
            .set("y", text_y)
            .set("text-anchor", "middle")
            .set("dominant-baseline", "middle")
            .set("font-family", DEFAULT_FONT_FAMILY)
            .set("font-size", type_def.font_size);

        group.add(rect).add(text_element)
    }
}

// Implement ShapeRenderer directly for OvalShape
impl ShapeRenderer for OvalRenderer {
    fn render_to_svg(
        &self,
        position: Point,
        size: Size,
        type_def: &ast::TypeDefinition,
        text: &str,
        has_nested_blocks: bool,
    ) -> Group {
        let group = Group::new();

        // Use ellipse which takes center point (cx, cy) plus radiuses (rx, ry)
        let rx = size.width() / 2.0;
        let ry = size.height() / 2.0;

        let mut ellipse = Ellipse::new()
            .set("cx", position.x())
            .set("cy", position.y())
            .set("rx", rx)
            .set("ry", ry)
            .set("stroke", type_def.line_color.to_string())
            .set("stroke-width", type_def.line_width)
            .set("fill", "white");

        if let Some(fill_color) = &type_def.fill_color {
            ellipse = ellipse.set("fill", fill_color.to_string());
        }

        // Component name
        // If this component has nested blocks, position the text near the top
        // otherwise place it in the center of the ellipse
        let text_y = if has_nested_blocks {
            // Position text near the top with a small padding (adjust based on oval shape)
            ry.mul_add(-OVAL_TEXT_POSITION_FACTOR, position.y())
        } else {
            // Center the text vertically
            position.y()
        };

        let text_element = Text::new(text)
            .set("x", position.x())
            .set("y", text_y)
            .set("text-anchor", "middle")
            .set("dominant-baseline", "middle")
            .set("font-family", DEFAULT_FONT_FAMILY)
            .set("font-size", type_def.font_size);

        group.add(ellipse).add(text_element)
    }
}
