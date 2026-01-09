use std::{
    hash::{Hash, Hasher},
    str::FromStr,
};

use color::DynamicColor;

/// Wrapper around the `DynamicColor` type from the color crate
/// This provides convenience methods for working with colors in the Filament project
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Color {
    color: DynamicColor,
}

impl Eq for Color {}

impl Hash for Color {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.to_string().hash(state);
    }
}

impl Color {
    /// Create a new `FilamentColor` from a string
    /// This will parse CSS color strings such as "#ff0000", "rgb(255, 0, 0)", "red", etc.
    pub fn new(color_str: &str) -> Result<Self, String> {
        match DynamicColor::from_str(color_str) {
            Ok(color) => Ok(Self { color }),
            Err(err) => Err(format!("Invalid color '{color_str}': {err}")),
        }
    }

    /// Get the sanitized ID-safe string for this color (for use in markers)
    pub fn to_id_safe_string(self) -> String {
        let color_str = self.to_string();
        // Replace invalid ID characters with underscores
        let mut sanitized = color_str
            .replace('#', "hex")
            .replace(['(', ')', ',', ' ', ';'], "_");

        // Ensure the ID starts with a letter (required for valid SVG IDs)
        if sanitized.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            sanitized = format!("c_{sanitized}");
        }

        sanitized
    }

    /// Creates a new color with the specified alpha (transparency) value.
    ///
    /// # Arguments
    ///
    /// * `alpha` - The alpha value to set, typically between 0.0 (fully transparent)
    ///   and 1.0 (fully opaque)
    ///
    /// # Returns
    ///
    /// A new `Color` instance with the updated alpha value.
    pub fn with_alpha(self, alpha: f32) -> Self {
        Color {
            color: self.color.with_alpha(alpha),
        }
    }

    /// Returns the alpha (transparency) component of this color.
    ///
    /// # Returns
    ///
    /// The alpha value as a `f32` between 0.0 and 1.0, where:
    /// - 0.0 = fully transparent
    /// - 1.0 = fully opaque
    pub fn alpha(&self) -> f32 {
        self.color.components[3]
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::new("black").expect("'black' is a valid CSS color")
    }
}

// For compatibility with the existing codebase that uses colors as strings
impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.color)
    }
}

impl From<&Color> for svg::node::Value {
    fn from(color: &Color) -> Self {
        Self::from(color.to_string())
    }
}
