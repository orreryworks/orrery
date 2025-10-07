use super::ShapeDefinition;
use crate::{
    color::Color,
    draw::StrokeDefinition,
    geometry::{Insets, Point, Size},
};
use std::rc::Rc;
use svg::{self, node::element as svg_element};

/// UML Interface shape definition - a circle with an "I" symbol
/// This is a content-free shape that cannot contain nested elements
#[derive(Debug, Clone)]
pub struct InterfaceDefinition {
    fill_color: Option<Color>,
    stroke: Rc<StrokeDefinition>,
}

impl InterfaceDefinition {
    /// Create a new interface definition with default values
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for InterfaceDefinition {
    fn default() -> Self {
        Self {
            fill_color: Some(Color::new("white").unwrap()),
            stroke: Rc::new(StrokeDefinition::solid(Color::default(), 2.0)),
        }
    }
}

impl ShapeDefinition for InterfaceDefinition {
    fn calculate_shape_size(&self, _content_size: Size, _padding: Insets) -> Size {
        Size::new(30.0, 30.0)
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
        let radius = 15.0;

        // Main circle
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

        circle.into()
    }
}
