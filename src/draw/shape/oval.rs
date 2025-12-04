use std::rc::Rc;

use svg::{self, node::element as svg_element};

use super::ShapeDefinition;
use crate::{
    color::Color,
    draw::{StrokeDefinition, TextDefinition, text_positioning::TextPositioningStrategy},
    geometry::{Insets, Point, Size},
};

/// Oval shape definition
#[derive(Debug, Clone)]
pub struct OvalDefinition {
    fill_color: Option<Color>,
    stroke: Rc<StrokeDefinition>,
    text: Rc<TextDefinition>,
}

impl OvalDefinition {
    /// Create a new oval definition with default values
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for OvalDefinition {
    fn default() -> Self {
        Self {
            fill_color: None,
            stroke: Rc::new(StrokeDefinition::default_solid()),
            text: Rc::new(TextDefinition::default()),
        }
    }
}

impl ShapeDefinition for OvalDefinition {
    fn supports_content(&self) -> bool {
        true
    }

    fn find_intersection(&self, a: Point, b: Point, a_size: Size) -> Point {
        // For an ellipse, finding the intersection is more complex than for a rectangle
        // We use a parametric approach based on the direction vector

        let half_width = a_size.width() / 2.0;
        let half_height = a_size.height() / 2.0;

        let dist = b.sub_point(a);

        // Normalize the direction vector
        let length = dist.hypot(); // (dx * dx + dy * dy).sqrt()
        if length < 0.001 {
            // Avoid division by zero
            return b;
        }

        let dx_norm = dist.x() / length;
        let dy_norm = dist.y() / length;

        // We need to solve for the intersection of a ray (a + t * direction) with an ellipse
        // This is a quadratic equation

        // For simplicity, we'll use an approximation that works well for most cases
        // This calculates the intersection with a normalized ellipse and then scales back
        // First we find the angle of the direction vector
        let angle = dy_norm.atan2(dx_norm);

        // Then find the radius of the ellipse at that angle
        // r = (a*b) / sqrt((b*cos(θ))² + (a*sin(θ))²)
        // where a is half_width and b is half_height
        let cos_angle = angle.cos();
        let sin_angle = angle.sin();
        let radius =
            (half_width * half_height) / (half_height * cos_angle).hypot(half_width * sin_angle);

        // Calculate the intersection point
        Point::new(
            dx_norm.mul_add(radius, a.x()), // a.x + dx_norm * radius
            dy_norm.mul_add(radius, a.y()), // a.y + dy_norm * radius
        )
    }

    fn calculate_shape_size(&self, content_size: Size, padding: Insets) -> Size {
        // The largest rectangle that fits in an ellipse with semi-axes (a,b) has dimensions:
        // width = a√2, height = b√2
        // So we need to scale up the content to create an ellipse that can contain it
        let min_size = Size::new(10.0, 10.0);
        let sqrt_2 = 2.0_f32.sqrt();
        content_size
            .scale(sqrt_2)
            .add_padding(padding)
            .max(min_size)
    }

    fn clone_box(&self) -> Box<dyn ShapeDefinition> {
        Box::new(self.clone())
    }

    fn fill_color(&self) -> Option<Color> {
        self.fill_color
    }

    fn stroke(&self) -> &Rc<StrokeDefinition> {
        &self.stroke
    }


    fn set_fill_color(&mut self, color: Option<Color>) -> Result<(), &'static str> {
        self.fill_color = color;
        Ok(())
    }

    fn text(&self) -> &Rc<TextDefinition> {
        &self.text
    }


    fn set_text(&mut self, text: Rc<TextDefinition>) {
        self.text = text;
    }

    fn set_stroke(&mut self, stroke: Rc<StrokeDefinition>) {
        self.stroke = stroke;
    }

    fn text_positioning_strategy(&self) -> TextPositioningStrategy {
        TextPositioningStrategy::InContent
    }

    fn render_to_svg(&self, size: Size, position: Point) -> Box<dyn svg::Node> {
        // Use ellipse which takes center point (cx, cy) plus radiuses (rx, ry)
        let rx = size.width() / 2.0;
        let ry = size.height() / 2.0;

        let ellipse = svg_element::Ellipse::new()
            .set("cx", position.x())
            .set("cy", position.y())
            .set("rx", rx)
            .set("ry", ry)
            .set("fill", "white");

        let mut ellipse = crate::apply_stroke!(ellipse, &self.stroke);

        if let Some(fill_color) = self.fill_color() {
            ellipse = ellipse
                .set("fill", fill_color.to_string())
                .set("fill-opacity", fill_color.alpha());
        }

        ellipse.into()
    }
}
