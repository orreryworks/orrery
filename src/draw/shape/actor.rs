use std::borrow::Cow;

use svg::{self, node::element as svg_element};

use super::ShapeDefinition;
use crate::{
    color::Color,
    draw::{StrokeDefinition, TextDefinition},
    geometry::{Insets, Point, Size},
};

/// UML Actor shape definition - a stick figure representation
/// This is a content-free shape that cannot contain nested elements
#[derive(Debug, Clone)]
pub struct ActorDefinition {
    fill_color: Option<Color>,
    stroke: Cow<'static, StrokeDefinition>,
    text: Cow<'static, TextDefinition>,
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
            fill_color: Some(Color::new("white").expect("Failed to create white color")),
            stroke: Cow::Borrowed(StrokeDefinition::default_solid_borrowed()),
            text: Cow::Borrowed(TextDefinition::default_borrowed()),
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

    fn stroke(&self) -> &StrokeDefinition {
        &self.stroke
    }

    fn mut_stroke(&mut self) -> &mut StrokeDefinition {
        self.stroke.to_mut()
    }

    fn set_fill_color(&mut self, color: Option<Color>) -> Result<(), &'static str> {
        self.fill_color = color;
        Ok(())
    }

    fn set_stroke(&mut self, stroke: Cow<'static, StrokeDefinition>) -> Result<(), &'static str> {
        self.stroke = stroke;
        Ok(())
    }

    fn with_fill_color(
        &self,
        color: Option<Color>,
    ) -> Result<Box<dyn ShapeDefinition>, &'static str> {
        let mut cloned = self.clone();
        cloned.set_fill_color(color)?;
        Ok(Box::new(cloned))
    }

    fn with_stroke(
        &self,
        stroke: Cow<'static, StrokeDefinition>,
    ) -> Result<Box<dyn ShapeDefinition>, &'static str> {
        let mut cloned = self.clone();
        cloned.set_stroke(stroke)?;
        Ok(Box::new(cloned))
    }

    fn set_text(&mut self, text: Cow<'static, TextDefinition>) {
        self.text = text;
    }

    fn text(&self) -> &TextDefinition {
        &self.text
    }

    fn mut_text(&mut self) -> &mut TextDefinition {
        self.text.to_mut()
    }

    fn with_text(&self, text: Cow<'static, TextDefinition>) -> Box<dyn ShapeDefinition> {
        let mut cloned = self.clone();
        cloned.set_text(text);
        Box::new(cloned)
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
        let head = svg_element::Circle::new()
            .set("cx", head_center.x())
            .set("cy", head_center.y())
            .set("r", head_radius)
            .set("fill", "white");

        let mut head = crate::apply_stroke!(head, &self.stroke);

        if let Some(fill_color) = self.fill_color() {
            head = head
                .set("fill", fill_color.to_string())
                .set("fill-opacity", fill_color.alpha());
        }

        group = group.add(head);

        // Body (vertical line)
        let body_top = position.with_y(head_center.y() + head_radius);
        let body_bottom = position.with_y(body_top.y() + body_length);

        let body = crate::apply_stroke!(
            svg_element::Line::new()
                .set("x1", body_top.x())
                .set("y1", body_top.y())
                .set("x2", body_bottom.x())
                .set("y2", body_bottom.y()),
            &self.stroke
        );

        group = group.add(body);

        // Arms (two diagonal lines from upper body)
        let arm_center = position.with_y(body_top.y() + 6.0);

        // Left arm
        let left_arm = crate::apply_stroke!(
            svg_element::Line::new()
                .set("x1", arm_center.x())
                .set("y1", arm_center.y())
                .set("x2", arm_center.x() - arm_length)
                .set("y2", arm_center.y() + 8.0),
            &self.stroke
        );

        group = group.add(left_arm);

        // Right arm
        let right_arm = crate::apply_stroke!(
            svg_element::Line::new()
                .set("x1", arm_center.x())
                .set("y1", arm_center.y())
                .set("x2", arm_center.x() + arm_length)
                .set("y2", arm_center.y() + 8.0),
            &self.stroke
        );

        group = group.add(right_arm);

        // Legs (two diagonal lines from bottom of body)
        // Left leg
        let left_leg = crate::apply_stroke!(
            svg_element::Line::new()
                .set("x1", body_bottom.x())
                .set("y1", body_bottom.y())
                .set("x2", body_bottom.x() - 10.0)
                .set("y2", body_bottom.y() + leg_length),
            &self.stroke
        );

        group = group.add(left_leg);

        // Right leg
        let right_leg = crate::apply_stroke!(
            svg_element::Line::new()
                .set("x1", body_bottom.x())
                .set("y1", body_bottom.y())
                .set("x2", body_bottom.x() + 10.0)
                .set("y2", body_bottom.y() + leg_length),
            &self.stroke
        );

        group = group.add(right_leg);

        group.into()
    }
}
