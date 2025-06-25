use crate::{
    draw::{self, Drawable},
    geometry::Point,
};

pub fn render_shape_and_text_to_svg(
    position: Point,
    shape: &draw::PositionedDrawable<draw::Shape>,
    text: &draw::Text,
) -> Box<dyn svg::Node> {
    let mut group = draw::Group::new();
    group.add(shape.inner().clone(), position);
    group.add(text.clone(), position);

    group.render_to_svg(Point::default()) // TODO: Fix position
}
