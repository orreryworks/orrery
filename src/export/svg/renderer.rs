use crate::ast::elaborate::TypeDefinition;
use crate::layout::common::{Point, Size};
use crate::shape::{Oval, Rectangle, Shape};
use svg::node::element::{Ellipse, Group, Rectangle as SvgRectangle, Text};

/// Trait for rendering shapes to SVG
pub trait ShapeRenderer {
    /// Render a shape to SVG based on the given properties
    fn render_to_svg(
        &self,
        position: &Point,
        size: &Size,
        type_def: &TypeDefinition,
        text: &str,
    ) -> Group;
}

/// Helper function to get a shape renderer based on the shape type
pub fn get_renderer(shape_type: &dyn Shape) -> &dyn ShapeRenderer {
    match shape_type.name() {
        "Rectangle" => &Rectangle,
        "Oval" => &Oval,
        _ => &Rectangle, // Default to rectangle if unknown
    }
}

// Implement ShapeRenderer directly for RectangleShape
impl ShapeRenderer for Rectangle {
    fn render_to_svg(
        &self,
        position: &Point,
        size: &Size,
        type_def: &TypeDefinition,
        text: &str,
    ) -> Group {
        let group = Group::new();

        // Calculate the actual top-left position for the rectangle
        // (position is the center of the component)
        let rect_x = position.x - (size.width / 2.0);
        let rect_y = position.y - (size.height / 2.0);

        // Main rectangle
        let mut rect = SvgRectangle::new()
            .set("x", rect_x)
            .set("y", rect_y)
            .set("width", size.width)
            .set("height", size.height)
            .set("stroke", type_def.line_color.as_str())
            .set("stroke-width", type_def.line_width)
            .set("fill", "white")
            .set("rx", type_def.rounded);

        if let Some(fill_color) = &type_def.fill_color {
            rect = rect.set("fill", fill_color.as_str());
        }

        // Component name
        let text_element = Text::new(text)
            .set("x", position.x)
            .set("y", position.y)
            .set("text-anchor", "middle")
            .set("dominant-baseline", "middle")
            .set("font-family", "Arial")
            .set("font-size", type_def.font_size);

        group.add(rect).add(text_element)
    }
}

// Implement ShapeRenderer directly for OvalShape
impl ShapeRenderer for Oval {
    fn render_to_svg(
        &self,
        position: &Point,
        size: &Size,
        type_def: &TypeDefinition,
        text: &str,
    ) -> Group {
        let group = Group::new();

        // Use ellipse which takes center point (cx, cy) plus radiuses (rx, ry)
        let rx = size.width / 2.0;
        let ry = size.height / 2.0;

        let mut ellipse = Ellipse::new()
            .set("cx", position.x)
            .set("cy", position.y)
            .set("rx", rx)
            .set("ry", ry)
            .set("stroke", type_def.line_color.as_str())
            .set("stroke-width", type_def.line_width)
            .set("fill", "white");

        if let Some(fill_color) = &type_def.fill_color {
            ellipse = ellipse.set("fill", fill_color.as_str());
        }

        // Component name
        let text_element = Text::new(text)
            .set("x", position.x)
            .set("y", position.y)
            .set("text-anchor", "middle")
            .set("dominant-baseline", "middle")
            .set("font-family", "Arial")
            .set("font-size", type_def.font_size);

        group.add(ellipse).add(text_element)
    }
}
