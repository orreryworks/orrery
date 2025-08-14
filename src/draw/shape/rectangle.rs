use super::ShapeDefinition;
use crate::{
    color::Color,
    draw::text_positioning::TextPositioningStrategy,
    geometry::{Insets, Point, Size},
};
use std::rc::Rc;
use svg::{self, node::element as svg_element};

/// Rectangle shape definition
#[derive(Debug, Clone)]
pub struct RectangleDefinition {
    fill_color: Option<Color>,
    line_color: Color,
    line_width: usize,
    rounded: usize,
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
            line_color: Color::default(),
            line_width: 2,
            rounded: 0,
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

    fn line_color(&self) -> Color {
        self.line_color
    }

    fn line_width(&self) -> usize {
        self.line_width
    }

    fn rounded(&self) -> usize {
        self.rounded
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

    fn set_rounded(&mut self, radius: usize) -> Result<(), &'static str> {
        self.rounded = radius;
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

    fn with_rounded(&self, radius: usize) -> Result<Rc<dyn ShapeDefinition>, &'static str> {
        let mut cloned = self.clone();
        cloned.set_rounded(radius)?;
        Ok(Rc::new(cloned))
    }

    fn text_positioning_strategy(&self) -> TextPositioningStrategy {
        TextPositioningStrategy::InContent
    }

    fn render_to_svg(&self, size: Size, position: Point) -> Box<dyn svg::Node> {
        // Calculate the actual top-left position for the rectangle
        // (position is the center of the component)
        let bounds = position.to_bounds(size);

        // Main rectangle
        let mut rect = svg_element::Rectangle::new()
            .set("x", bounds.min_x())
            .set("y", bounds.min_y())
            .set("width", size.width())
            .set("height", size.height())
            .set("stroke", self.line_color().to_string())
            .set("stroke-opacity", self.line_color().alpha())
            .set("stroke-width", self.line_width())
            .set("fill", "white")
            .set("rx", self.rounded());

        if let Some(fill_color) = self.fill_color() {
            rect = rect
                .set("fill", fill_color.to_string())
                .set("fill-opacity", fill_color.alpha());
        }

        rect.into()
    }
}
