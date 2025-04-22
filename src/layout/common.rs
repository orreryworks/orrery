use crate::ast::elaborate::Node;

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
    pub fn merge(&self, other: &Bounds) -> Bounds {
        Bounds {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }
}

#[derive(Debug)]
pub struct Component<'a> {
    pub node: &'a Node,
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

/// Utility function to estimate the size of text in pixels
/// This is a simple approximation based on the font size and text length
pub fn estimate_text_size(text: &str, font_size: usize) -> Size {
    // Convert usize font_size to f32 for calculations
    let font_size_f32 = font_size as f32;

    // Roughly estimate based on average character width (usually 0.6x font size)
    let char_width = font_size_f32 * 0.6;
    let width = text.len() as f32 * char_width;

    // Font height is approximately 1.2-1.5x the font size
    let height = font_size_f32 * 1.2;

    Size { width, height }
}

/// Calculate the size of a component or participant based on its text content
// TODO: This is not a good estimation. Consider using resvg + usvg to get abs_bounding_box
// or ab_glyph or other font libraries for a better estimation.
pub fn calculate_element_size(node: &Node, min_width: f32, min_height: f32, padding: f32) -> Size {
    // Calculate text size based on the node's name and font size
    let text_size = estimate_text_size(&node.name, node.type_definition.font_size);

    // Add padding around the text and ensure minimum size
    let width = (text_size.width + padding * 2.0).max(min_width);
    let height = (text_size.height + padding * 2.0).max(min_height);

    Size { width, height }
}
