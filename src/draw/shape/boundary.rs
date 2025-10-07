use super::ShapeDefinition;
use crate::{
    color::Color,
    draw::StrokeDefinition,
    geometry::{Insets, Point, Size},
};
use std::rc::Rc;
use svg::{self, node::element as svg_element};

/// UML Boundary shape definition - a circle with a vertical line on the left
/// This is a content-free shape that cannot contain nested elements
#[derive(Debug, Clone)]
pub struct BoundaryDefinition {
    fill_color: Option<Color>,
    stroke: Rc<StrokeDefinition>,
}

impl BoundaryDefinition {
    /// Create a new boundary definition with default values
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for BoundaryDefinition {
    fn default() -> Self {
        Self {
            fill_color: Some(Color::new("white").unwrap()),
            stroke: Rc::new(StrokeDefinition::solid(Color::default(), 2.0)),
        }
    }
}

impl ShapeDefinition for BoundaryDefinition {
    fn calculate_shape_size(&self, _content_size: Size, _padding: Insets) -> Size {
        Size::new(43.0, 30.0)
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

    fn set_fill_color(&mut self, color: Option<Color>) -> Result<(), &'static str> {
        self.fill_color = color;
        Ok(())
    }

    fn set_stroke(&mut self, stroke: StrokeDefinition) -> Result<(), &'static str> {
        self.stroke = Rc::new(stroke);
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

    fn with_stroke(
        &self,
        stroke: StrokeDefinition,
    ) -> Result<Rc<dyn ShapeDefinition>, &'static str> {
        let mut cloned = self.clone();
        cloned.set_stroke(stroke)?;
        Ok(Rc::new(cloned))
    }

    fn render_to_svg(&self, _size: Size, position: Point) -> Box<dyn svg::Node> {
        let size_half_x = 21.5;
        let circle_radius = 15.0;
        let circle_x_offset = size_half_x - circle_radius;
        let circle_position = position.with_x(position.x() + circle_x_offset);

        // Create group element to contain circle and boundary line
        let mut group = svg_element::Group::new().set("id", "boundary-group");

        // Main circle
        let circle = svg_element::Circle::new()
            .set("cx", circle_position.x())
            .set("cy", circle_position.y())
            .set("r", circle_radius)
            .set("fill", "white");

        let mut circle = crate::apply_stroke!(circle, &self.stroke);

        if let Some(fill_color) = self.fill_color() {
            circle = circle
                .set("fill", fill_color.to_string())
                .set("fill-opacity", fill_color.alpha());
        }

        group = group.add(circle);

        // Add vertical line on the left side extending beyond the circle
        let line_x1 = position.add_point(Point::new(-size_half_x, -circle_radius));
        let line_x2 = position.add_point(Point::new(-size_half_x, circle_radius));

        let boundary_line = crate::apply_stroke!(
            svg_element::Line::new()
                .set("x1", line_x1.x())
                .set("y1", line_x1.y())
                .set("x2", line_x2.x())
                .set("y2", line_x2.y()),
            &self.stroke
        );

        group = group.add(boundary_line);

        // Add horizontal connector line
        let line_x1 = position.add_point(Point::new(-size_half_x, 0.0));
        let line_x2 = position.add_point(Point::new(-8.5, 0.0));

        let connector_line = crate::apply_stroke!(
            svg_element::Line::new()
                .set("x1", line_x1.x())
                .set("y1", line_x1.y())
                .set("x2", line_x2.x())
                .set("y2", line_x2.y()),
            &self.stroke
        );

        group = group.add(connector_line);

        group.into()
    }
}
