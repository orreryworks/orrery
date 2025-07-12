use super::{ShapeDefinition, rectangle};
use crate::{
    color::Color,
    geometry::{Insets, Point, Size},
};
use std::{cell::RefCell, rc::Rc};
use svg::{self, node::element as svg_element};

/// UML Entity shape definition - a circle representation
/// This is a content-free shape that cannot contain nested elements
#[derive(Debug, Clone)]
pub struct EntityDefinition {
    fill_color: Option<Color>,
    line_color: Color,
    line_width: usize,
}

impl EntityDefinition {
    /// Create a new entity definition with default values
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for EntityDefinition {
    fn default() -> Self {
        Self {
            fill_color: Some(Color::new("white").unwrap()),
            line_color: Color::default(),
            line_width: 2,
        }
    }
}

impl ShapeDefinition for EntityDefinition {
    fn find_intersection(&self, a: Point, b: Point, a_size: Size) -> Point {
        rectangle::find_rectangle_intersection(a, b, a_size)
    }

    fn calculate_shape_size(&self, _content_size: Size, _padding: Insets) -> Size {
        Size::new(30.0 + self.line_width() as f32, 30.0)
    }

    fn clone_new_rc(&self) -> Rc<RefCell<dyn ShapeDefinition>> {
        Rc::new(RefCell::new(self.clone()))
    }

    fn fill_color(&self) -> Option<Color> {
        self.fill_color
    }

    fn line_color(&self) -> Color {
        self.line_color
    }

    fn line_width(&self) -> usize {
        self.line_width
    }

    fn set_fill_color(&mut self, color: Option<Color>) -> Result<(), &'static str> {
        self.fill_color = color;
        Ok(())
    }

    fn set_line_color(&mut self, color: Color) -> Result<(), &'static str> {
        self.line_color = color;
        Ok(())
    }

    fn set_line_width(&mut self, width: usize) -> Result<(), &'static str> {
        self.line_width = width;
        Ok(())
    }

    fn render_to_svg(&self, _size: Size, position: Point) -> Box<dyn svg::Node> {
        let radius = 15.0;

        let mut group = svg_element::Group::new().set("id", "component-group");

        // Create the main circle
        let mut circle = svg_element::Circle::new()
            .set("cx", position.x())
            .set("cy", position.y())
            .set("r", radius)
            .set("stroke", self.line_color().to_string())
            .set("stroke-width", self.line_width())
            .set("fill", "white");

        if let Some(fill_color) = self.fill_color() {
            circle = circle.set("fill", fill_color.to_string());
        }

        group = group.add(circle);

        let line_y = position.y() + radius + self.line_width() as f32;
        let line_x1 = position.x() - radius;
        let line_x2 = position.x() + radius;

        let line = svg_element::Line::new()
            .set("x1", line_x1)
            .set("y1", line_y)
            .set("x2", line_x2)
            .set("y2", line_y)
            .set("stroke", self.line_color().to_string())
            .set("stroke-width", self.line_width())
            .set("stroke-linecap", "round");

        group = group.add(line);

        group.into()
    }
}
