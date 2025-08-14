use super::ShapeDefinition;
use crate::{
    color::Color,
    geometry::{Insets, Point, Size},
};
use std::rc::Rc;
use svg::{self, node::element as svg_element};

/// UML Control shape definition - a circle with an arrow pointing right
/// This is a content-free shape that cannot contain nested elements
#[derive(Debug, Clone)]
pub struct ControlDefinition {
    fill_color: Option<Color>,
    line_color: Color,
    line_width: usize,
}

impl ControlDefinition {
    /// Create a new control definition with default values
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for ControlDefinition {
    fn default() -> Self {
        Self {
            fill_color: Some(Color::new("white").unwrap()),
            line_color: Color::default(),
            line_width: 2,
        }
    }
}

impl ShapeDefinition for ControlDefinition {
    fn calculate_shape_size(&self, _content_size: Size, _padding: Insets) -> Size {
        Size::new(34.0, 30.0)
    }

    fn clone_box(&self) -> Box<dyn ShapeDefinition> {
        Box::new(self.clone())
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

    fn with_fill_color(
        &self,
        color: Option<Color>,
    ) -> Result<Rc<dyn ShapeDefinition>, &'static str> {
        let mut cloned = self.clone();
        cloned.set_fill_color(color)?;
        Ok(Rc::new(cloned))
    }

    fn with_line_color(&self, color: Color) -> Result<Rc<dyn ShapeDefinition>, &'static str> {
        let mut cloned = self.clone();
        cloned.set_line_color(color)?;
        Ok(Rc::new(cloned))
    }

    fn with_line_width(&self, width: usize) -> Result<Rc<dyn ShapeDefinition>, &'static str> {
        let mut cloned = self.clone();
        cloned.set_line_width(width)?;
        Ok(Rc::new(cloned))
    }

    fn render_to_svg(&self, _size: Size, position: Point) -> Box<dyn svg::Node> {
        // Create group element to contain circle and arrow
        let mut group = svg_element::Group::new().set("id", "control-group");

        let radius = 15.0;

        // Main circle
        let mut circle = svg_element::Circle::new()
            .set("cx", position.x())
            .set("cy", position.y())
            .set("r", radius)
            .set("stroke", self.line_color().to_string())
            .set("stroke-opacity", self.line_color().alpha())
            .set("stroke-width", self.line_width())
            .set("fill", "white");

        if let Some(fill_color) = self.fill_color() {
            circle = circle
                .set("fill", fill_color.to_string())
                .set("fill-opacity", fill_color.alpha());
        }

        group = group.add(circle);

        let arrow_x = position.x();
        let arrow_y = position.y() - radius;

        let arrow_path_data = format!(
            "M {} {} L {} {} M {} {} L {} {}",
            arrow_x,
            arrow_y,
            arrow_x + 4.0,
            arrow_y - 4.0,
            arrow_x,
            arrow_y,
            arrow_x + 4.0,
            arrow_y + 4.0
        );

        let arrow_head = svg_element::Path::new()
            .set("d", arrow_path_data)
            .set("stroke", self.line_color().to_string())
            .set("stroke-opacity", self.line_color().alpha())
            .set("stroke-width", self.line_width())
            .set("stroke-linecap", "round")
            .set("fill", "none");

        group = group.add(arrow_head);

        group.into()
    }
}
