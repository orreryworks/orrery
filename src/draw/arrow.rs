use crate::color::Color;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrowStyle {
    Straight,
    Curved,
    Orthogonal,
}

impl Default for ArrowStyle {
    fn default() -> Self {
        Self::Straight
    }
}

impl FromStr for ArrowStyle {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "straight" => Ok(Self::Straight),
            "curved" => Ok(Self::Curved),
            "orthogonal" => Ok(Self::Orthogonal),
            _ => Err("Invalid arrow style"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ArrowDefinition {
    color: Color,
    width: usize,
    arrow_style: ArrowStyle,
}

impl ArrowDefinition {
    /// Creates a new ArrowDefinition with default values
    /// Use setter methods to configure the arrow properties
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the arrow color
    pub fn color(&self) -> Color {
        self.color
    }

    /// Gets the arrow line width
    pub fn width(&self) -> usize {
        self.width
    }

    /// Gets the arrow style
    pub fn arrow_style(&self) -> &ArrowStyle {
        &self.arrow_style
    }

    /// Sets the arrow color
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    /// Sets the arrow line width
    pub fn set_width(&mut self, width: usize) {
        self.width = width;
    }

    /// Sets the arrow style
    pub fn set_arrow_style(&mut self, style: ArrowStyle) {
        self.arrow_style = style;
    }
}

impl Default for ArrowDefinition {
    fn default() -> Self {
        Self {
            color: Color::default(),
            width: 1,
            arrow_style: ArrowStyle::default(),
        }
    }
}
