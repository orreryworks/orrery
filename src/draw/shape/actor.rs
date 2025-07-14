use super::ShapeDefinition;
use crate::{
    color::Color,
    geometry::{Insets, Point, Size},
};
use std::rc::Rc;
use svg::{self, node::element as svg_element};

/// UML Actor shape definition - a stick figure representation
/// This is a content-free shape that cannot contain nested elements
#[derive(Debug, Clone)]
pub struct ActorDefinition {
    fill_color: Option<Color>,
    line_color: Color,
    line_width: usize,
}

impl ActorDefinition {
    /// Create a new actor definition with default values
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for ActorDefinition {
    fn default() -> Self {
        Self {
            fill_color: Some(Color::new("white").unwrap()),
            line_color: Color::default(),
            line_width: 2,
        }
    }
}

impl ShapeDefinition for ActorDefinition {
    fn calculate_shape_size(&self, _content_size: Size, _padding: Insets) -> Size {
        Size::new(24.0, 54.0)
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
        // Create group element to contain all stick figure parts
        let mut group = svg_element::Group::new().set("id", "actor-group");

        let head_radius = 8.0;
        let body_length = 20.0;
        let arm_length = 12.0;
        let leg_length = 18.0;

        // Head (circle at the top)
        let head_center = position.with_y(position.y() - 22.0);
        let mut head = svg_element::Circle::new()
            .set("cx", head_center.x())
            .set("cy", head_center.y())
            .set("r", head_radius)
            .set("stroke", self.line_color().to_string())
            .set("stroke-width", self.line_width())
            .set("fill", "white");

        if let Some(fill_color) = self.fill_color() {
            head = head.set("fill", fill_color.to_string());
        }

        group = group.add(head);

        // Body (vertical line)
        let body_top = position.with_y(head_center.y() + head_radius);
        let body_bottom = position.with_y(body_top.y() + body_length);

        let body = svg_element::Line::new()
            .set("x1", body_top.x())
            .set("y1", body_top.y())
            .set("x2", body_bottom.x())
            .set("y2", body_bottom.y())
            .set("stroke", self.line_color().to_string())
            .set("stroke-width", self.line_width())
            .set("stroke-linecap", "round");

        group = group.add(body);

        // Arms (two diagonal lines from upper body)
        let arm_center = position.with_y(body_top.y() + 6.0);

        // Left arm
        let left_arm = svg_element::Line::new()
            .set("x1", arm_center.x())
            .set("y1", arm_center.y())
            .set("x2", arm_center.x() - arm_length)
            .set("y2", arm_center.y() + 8.0)
            .set("stroke", self.line_color().to_string())
            .set("stroke-width", self.line_width())
            .set("stroke-linecap", "round");

        group = group.add(left_arm);

        // Right arm
        let right_arm = svg_element::Line::new()
            .set("x1", arm_center.x())
            .set("y1", arm_center.y())
            .set("x2", arm_center.x() + arm_length)
            .set("y2", arm_center.y() + 8.0)
            .set("stroke", self.line_color().to_string())
            .set("stroke-width", self.line_width())
            .set("stroke-linecap", "round");

        group = group.add(right_arm);

        // Legs (two diagonal lines from bottom of body)
        // Left leg
        let left_leg = svg_element::Line::new()
            .set("x1", body_bottom.x())
            .set("y1", body_bottom.y())
            .set("x2", body_bottom.x() - 10.0)
            .set("y2", body_bottom.y() + leg_length)
            .set("stroke", self.line_color().to_string())
            .set("stroke-width", self.line_width())
            .set("stroke-linecap", "round");

        group = group.add(left_leg);

        // Right leg
        let right_leg = svg_element::Line::new()
            .set("x1", body_bottom.x())
            .set("y1", body_bottom.y())
            .set("x2", body_bottom.x() + 10.0)
            .set("y2", body_bottom.y() + leg_length)
            .set("stroke", self.line_color().to_string())
            .set("stroke-width", self.line_width())
            .set("stroke-linecap", "round");

        group = group.add(right_leg);

        group.into()
    }
}
