use std::fmt;

use svg::{self, node::element as svg_element};

use super::{RectangleDefinition, ShapeDefinition, rectangle};
use crate::{
    color::Color,
    draw::{StrokeDefinition, text_positioning::TextPositioningStrategy},
    geometry::{Insets, Point, Size},
};

/// Component shape definition with UML component styling
pub type ComponentDefinition = RectangleWithIconDefinition<ComponentIcon>;

impl ComponentDefinition {
    /// Create a new component definition with default values
    pub fn new() -> Self {
        Self::default()
    }

    // fn render_icon(&self) -> svg_element::Group {
    //     let mut group = svg_element::Group::new();
    //     // Add your SVG icon code here
    //     group
    // }
}

impl Default for ComponentDefinition {
    fn default() -> Self {
        let mut rectangle_definition = RectangleDefinition::default();
        rectangle_definition
            .set_fill_color(Some(Color::new("#FEFECE").unwrap()))
            .expect("Failed to set fill color");
        rectangle_definition
            .set_rounded(10)
            .expect("Failed to set rounded");

        Self {
            rectangle_definition,
            icon: ComponentIcon,
        }
    }
}

pub trait Icon {
    fn render_to_svg(
        &self,
        stroke: &StrokeDefinition,
        fill_color: Option<Color>,
    ) -> Box<dyn svg::Node>;
    fn size(&self) -> Size;
}

#[derive(Debug, Clone)]
pub struct ComponentIcon;

impl Icon for ComponentIcon {
    fn render_to_svg(
        &self,
        stroke: &StrokeDefinition,
        fill_color: Option<Color>,
    ) -> Box<dyn svg::Node> {
        let path_data = "M 8 0 L 40 0 L 40 30 L 8 30 L 8 24 L 12 24 L 12 18 L 8 18 L 8 12 L 12 12
            L 12 6 L 8 6 L 8 0 Z M 0 6 L 0 12 L 12 12 L 12 6 L 0 6 Z M 0 18 L 0 24 L 12 24 L 12 18 L 0 18 Z";

        let component_icon = svg_element::Path::new()
            .set("d", path_data)
            .set("fill", "white")
            .set("fill-rule", "evenodd");

        let mut component_icon = crate::apply_stroke!(component_icon, stroke);

        if let Some(fill_color) = fill_color {
            component_icon = component_icon
                .set("fill", fill_color.to_string())
                .set("fill-opacity", fill_color.alpha());
        }

        component_icon.into()
    }

    fn size(&self) -> Size {
        Size::new(40.0, 30.0)
    }
}

#[derive(Debug, Clone)]
pub struct RectangleWithIconDefinition<I>
where
    I: Icon + fmt::Debug + Clone + 'static,
{
    rectangle_definition: rectangle::RectangleDefinition,
    icon: I,
}

impl<I> ShapeDefinition for RectangleWithIconDefinition<I>
where
    I: Icon + fmt::Debug + Clone + 'static,
{
    fn supports_content(&self) -> bool {
        true
    }

    fn find_intersection(&self, a: Point, b: Point, a_size: Size) -> Point {
        self.rectangle_definition.find_intersection(a, b, a_size)
    }

    fn calculate_shape_size(&self, content_size: Size, padding: Insets) -> Size {
        let min_size = self.icon.size().add_padding(Insets::uniform(10.0));
        let padded_icon_size = self
            .icon
            .size()
            .add_padding(Insets::new(10.0, 10.0, 0.0, 0.0));
        let padded_content_size = content_size.add_padding(padding);
        padded_content_size
            .merge_horizontal(padded_icon_size)
            .max(min_size)
    }

    fn clone_box(&self) -> Box<dyn ShapeDefinition> {
        Box::new(self.clone())
    }

    fn fill_color(&self) -> Option<Color> {
        self.rectangle_definition.fill_color()
    }

    fn stroke(&self) -> &StrokeDefinition {
        self.rectangle_definition.stroke()
    }

    fn mut_stroke(&mut self) -> &mut StrokeDefinition {
        self.rectangle_definition.mut_stroke()
    }

    fn text(&self) -> &crate::draw::TextDefinition {
        self.rectangle_definition.text()
    }

    fn mut_text(&mut self) -> &mut crate::draw::TextDefinition {
        self.rectangle_definition.mut_text()
    }

    fn rounded(&self) -> usize {
        self.rectangle_definition.rounded()
    }

    fn set_fill_color(&mut self, color: Option<Color>) -> Result<(), &'static str> {
        self.rectangle_definition.set_fill_color(color)
    }

    fn set_rounded(&mut self, radius: usize) -> Result<(), &'static str> {
        self.rectangle_definition.set_rounded(radius)
    }

    fn text_positioning_strategy(&self) -> TextPositioningStrategy {
        TextPositioningStrategy::InContent
    }

    fn render_to_svg(&self, size: Size, position: Point) -> Box<dyn svg::Node> {
        // Calculate the actual bounds for the component
        let bounds = position.to_bounds(size);

        let padded_icon_size = self
            .icon
            .size()
            .add_padding(Insets::new(10.0, 10.0, 0.0, 0.0));
        let top_right_point = Point::new(bounds.max_x(), bounds.min_y());
        let icon_position = Point::new(
            top_right_point.x() - padded_icon_size.width(),
            top_right_point.y() + 10.0,
        );

        // Create group element to contain all component parts
        let mut group = svg_element::Group::new().set("id", "component-group");

        // Main rectangle
        let rect = svg_element::Rectangle::new()
            .set("x", bounds.min_x())
            .set("y", bounds.min_y())
            .set("width", size.width())
            .set("height", size.height())
            .set("fill", "white")
            .set("rx", self.rounded());

        let mut rect = crate::apply_stroke!(rect, self.stroke());

        if let Some(fill_color) = self.fill_color() {
            rect = rect
                .set("fill", fill_color.to_string())
                .set("fill-opacity", fill_color.alpha());
        }

        group = group.add(rect);

        let component_icon = self.icon.render_to_svg(self.stroke(), self.fill_color());

        // Create a group for the icon with transform to position it
        let icon_group = svg_element::Group::new()
            .set(
                "transform",
                format!("translate({}, {})", icon_position.x(), icon_position.y()),
            )
            .add(component_icon);

        group = group.add(icon_group);

        group.into()
    }
}
