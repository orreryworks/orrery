use std::borrow::Cow;

use svg::{self, node::element as svg_element};

use super::ShapeDefinition;
use crate::{
    color::Color,
    draw::{StrokeDefinition, TextDefinition, text_positioning::TextPositioningStrategy},
    geometry::{Insets, Point, Size},
};

/// Rectangle shape definition
#[derive(Debug, Clone)]
pub struct RectangleDefinition {
    fill_color: Option<Color>,
    stroke: Cow<'static, StrokeDefinition>,
    rounded: usize,
    text: Cow<'static, TextDefinition>,
}

impl RectangleDefinition {
    /// Create a new rectangle definition with default values
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for RectangleDefinition {
    fn default() -> Self {
        Self {
            fill_color: None,
            stroke: Cow::Borrowed(StrokeDefinition::default_solid_borrowed()),
            rounded: 0,
            text: Cow::Borrowed(TextDefinition::default_borrowed()),
        }
    }
}

impl ShapeDefinition for RectangleDefinition {
    fn supports_content(&self) -> bool {
        true
    }

    fn calculate_shape_size(&self, content_size: Size, padding: Insets) -> Size {
        let min_size = Size::new(10.0, 10.0);
        content_size.add_padding(padding).max(min_size)
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

    fn rounded(&self) -> usize {
        self.rounded
    }

    fn set_fill_color(&mut self, color: Option<Color>) -> Result<(), &'static str> {
        self.fill_color = color;
        Ok(())
    }

    fn set_rounded(&mut self, radius: usize) -> Result<(), &'static str> {
        self.rounded = radius;
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

    fn with_rounded(&self, radius: usize) -> Result<Box<dyn ShapeDefinition>, &'static str> {
        let mut cloned = self.clone();
        cloned.set_rounded(radius)?;
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

    fn with_text(&self, text: Cow<'static, TextDefinition>) -> Box<dyn ShapeDefinition> {
        let mut cloned = self.clone();
        cloned.set_text(text);
        Box::new(cloned)
    }

    fn text_positioning_strategy(&self) -> TextPositioningStrategy {
        TextPositioningStrategy::InContent
    }

    fn render_to_svg(&self, size: Size, position: Point) -> Box<dyn svg::Node> {
        // Calculate the actual top-left position for the rectangle
        // (position is the center of the component)
        let bounds = position.to_bounds(size);

        // Main rectangle
        let rect = svg_element::Rectangle::new()
            .set("x", bounds.min_x())
            .set("y", bounds.min_y())
            .set("width", size.width())
            .set("height", size.height())
            .set("fill", "white")
            .set("rx", self.rounded());

        let mut rect = crate::apply_stroke!(rect, &self.stroke);

        if let Some(fill_color) = self.fill_color() {
            rect = rect
                .set("fill", fill_color.to_string())
                .set("fill-opacity", fill_color.alpha());
        }

        rect.into()
    }
}
