use crate::{
    draw::{self, Drawable},
    geometry::Point,
};
use svg::node::element as svg_element;

// Constants for rendering configuration
const DEFAULT_TEXT_PADDING_FROM_TOP: f32 = 20.0;
const OVAL_TEXT_POSITION_FACTOR: f32 = 0.5;

/// Trait for rendering shapes to SVG
pub trait ShapeRenderer {
    /// Render a shape to SVG based on the given properties
    ///
    /// # Arguments
    /// * `position` - The center position of the shape in the coordinate system
    /// * `shape` - The shape definition including size, type, and styling properties
    /// * `text` - The text content and formatting to be rendered within the shape
    /// * `has_nested_blocks` - Whether this component contains nested diagram elements
    ///
    /// # Returns
    /// An SVG `Group` element containing the rendered shape and text
    fn render_to_svg(
        &self,
        position: Point,
        shape: &draw::Shape,
        text: &draw::Text,
        has_nested_blocks: bool,
    ) -> svg_element::Group;
}

/// Returns the appropriate shape renderer for the given shape type.
///
/// This function acts as a factory method that selects the correct renderer
/// implementation based on the shape's type.
///
/// # Arguments
/// * `shape_type` - The shape whose renderer should be returned
///
/// # Returns
/// A reference to a static renderer instance that can handle the given shape type.
pub fn get_renderer(shape_type: &draw::Shape) -> &'static dyn ShapeRenderer {
    match shape_type.name() {
        "Rectangle" => &RECTANGLE_RENDERER,
        "Oval" => &OVAL_RENDERER,
        _ => &RECTANGLE_RENDERER, // Default to rectangle if unknown
    }
}

/// Renderer implementation for rectangular shapes.
struct RectangleRenderer;

/// Renderer implementation for oval/elliptical shapes.
struct OvalRenderer;

static RECTANGLE_RENDERER: RectangleRenderer = RectangleRenderer;
static OVAL_RENDERER: OvalRenderer = OvalRenderer;

// Implement ShapeRenderer directly for RectangleShape
impl ShapeRenderer for RectangleRenderer {
    fn render_to_svg(
        &self,
        position: Point,
        shape: &draw::Shape,
        text: &draw::Text,
        has_nested_blocks: bool,
    ) -> svg_element::Group {
        let group = svg_element::Group::new();
        let size = shape.shape_size();

        let rect = shape.render_to_svg(position);

        // Component name
        // If this component has nested blocks, position the text near the top
        // otherwise place it in the center of the rectangle
        let text_y = if has_nested_blocks {
            let rect_bounds = position.to_bounds(size);

            // Position text near the top with a small padding
            rect_bounds.min_y() + DEFAULT_TEXT_PADDING_FROM_TOP
        } else {
            // Center the text vertically
            position.y()
        };

        let text_element = text.render_to_svg(Point::new(position.x(), text_y));

        group.add(rect).add(text_element)
    }
}

// Implement ShapeRenderer directly for OvalShape
impl ShapeRenderer for OvalRenderer {
    fn render_to_svg(
        &self,
        position: Point,
        shape: &draw::Shape,
        text: &draw::Text,
        has_nested_blocks: bool,
    ) -> svg_element::Group {
        let group = svg_element::Group::new();
        let size = shape.shape_size();

        let ellipse = shape.render_to_svg(position);

        // Component name
        // If this component has nested blocks, position the text near the top
        // otherwise place it in the center of the ellipse
        let text_y = if has_nested_blocks {
            let ry = size.height() / 2.0;

            // Position text near the top with a small padding (adjust based on oval shape)
            ry.mul_add(-OVAL_TEXT_POSITION_FACTOR, position.y())
        } else {
            // Center the text vertically
            position.y()
        };

        let text_element = text.render_to_svg(Point::new(position.x(), text_y));

        group.add(ellipse).add(text_element)
    }
}
