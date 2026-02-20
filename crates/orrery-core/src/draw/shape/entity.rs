//! Entity shape definition for component diagram elements.

use std::rc::Rc;

use svg::{self, node::element as svg_element};

use super::ShapeDefinition;
use crate::{
    color::Color,
    draw::{StrokeDefinition, TextDefinition},
    geometry::{Insets, Point, Size},
};

/// UML Entity shape definition - a circle representation
/// This is a content-free shape that cannot contain nested elements
#[derive(Debug, Clone)]
pub struct EntityDefinition {
    fill_color: Option<Color>,
    stroke: Rc<StrokeDefinition>,
    text: Rc<TextDefinition>,
}

impl EntityDefinition {
    /// Create a new entity definition with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the fill color of the entity
    fn fill_color(&self) -> Option<Color> {
        self.fill_color
    }
}

impl Default for EntityDefinition {
    fn default() -> Self {
        Self {
            fill_color: Some(Color::new("white").expect("Failed to create white color")),
            stroke: Rc::new(StrokeDefinition::default_solid()),
            text: Rc::new(TextDefinition::default()),
        }
    }
}

impl ShapeDefinition for EntityDefinition {
    fn calculate_inner_size(&self, _content_size: Size, _padding: Insets) -> Size {
        Size::new(30.0 + self.stroke.width(), 30.0)
    }

    fn clone_box(&self) -> Box<dyn ShapeDefinition> {
        Box::new(self.clone())
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

    fn render_to_svg(&self, _size: Size, position: Point) -> Box<dyn svg::Node> {
        let radius = 15.0;

        let mut group = svg_element::Group::new().set("id", "component-group");

        // Create the main circle
        let circle = svg_element::Circle::new()
            .set("cx", position.x())
            .set("cy", position.y())
            .set("r", radius)
            .set("fill", "white");

        let mut circle = crate::apply_stroke!(circle, &self.stroke);

        if let Some(fill_color) = self.fill_color() {
            circle = circle
                .set("fill", fill_color.to_string())
                .set("fill-opacity", fill_color.alpha());
        }

        group = group.add(circle);

        let line_y = position.y() + radius + self.stroke.width();
        let line_x1 = position.x() - radius;
        let line_x2 = position.x() + radius;

        let line = crate::apply_stroke!(
            svg_element::Line::new()
                .set("x1", line_x1)
                .set("y1", line_y)
                .set("x2", line_x2)
                .set("y2", line_y),
            &self.stroke
        );

        group = group.add(line);

        group.into()
    }
}
