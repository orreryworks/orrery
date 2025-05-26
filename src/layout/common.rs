use crate::ast;

#[derive(Debug, Copy, Clone)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Default)]
pub struct Bounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl Bounds {
    pub fn width(&self) -> f32 {
        self.max_x - self.min_x
    }

    pub fn height(&self) -> f32 {
        self.max_y - self.min_y
    }

    pub fn to_size(&self) -> Size {
        Size {
            width: self.width(),
            height: self.height(),
        }
    }

    /// Merge two bounds to create a larger bounds that contains both
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Component<'a> {
    pub node: &'a ast::Node,
    pub position: Point,
    pub size: Size,
}

impl Component<'_> {
    /// Calculate the bounds of this component
    pub fn bounds(&self) -> Bounds {
        let half_width = self.size.width / 2.0;
        let half_height = self.size.height / 2.0;

        Bounds {
            min_x: self.position.x - half_width,
            min_y: self.position.y - half_height,
            max_x: self.position.x + half_width,
            max_y: self.position.y + half_height,
        }
    }
}


