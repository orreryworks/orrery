//! Color handling for Filament diagrams
//!
//! This module provides the [`Color`] type which wraps the `DynamicColor` type
//! from the color crate, providing convenience methods for working with colors
//! in the Filament project.

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
    /// Create a new `Color` from a string
    /// This will parse CSS color strings such as "#ff0000", "rgb(255, 0, 0)", "red", etc.
    ///
    /// # Examples
    ///
    /// ```
    /// use filament_core::color::Color;
    ///
    /// let red = Color::new("#ff0000").unwrap();
    /// let blue = Color::new("blue").unwrap();
    /// ```
    pub fn new(color_str: &str) -> Result<Self, String> {
        match DynamicColor::from_str(color_str) {
            Ok(color) => Ok(Self { color }),
            Err(err) => Err(format!("invalid color `{color_str}`: {err}")),
        }
    }

    /// Returns a sanitized, ID-safe string representation of this color.
    ///
    /// Converts the color to a string suitable for use as an SVG ID attribute
    /// (e.g., in marker definitions). The result contains only alphanumeric
    /// characters and underscores, with a letter prefix guaranteed.
    ///
    /// # Examples
    ///
    /// ```
    /// use filament_core::color::Color;
    ///
    /// let color = Color::new("#ff8000").unwrap();
    /// let id_str = color.to_id_safe_string();
    /// assert!(id_str.chars().all(|c| c.is_alphanumeric() || c == '_'));
    /// assert!(!id_str.contains('#'));
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use filament_core::color::Color;
    ///
    /// let red = Color::new("red").unwrap();
    /// let semi_transparent_red = red.with_alpha(0.5);
    /// assert_eq!(semi_transparent_red.alpha(), 0.5);
    /// ```
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_new() {
        let red = Color::new("#ff0000");
        assert!(red.is_ok());

        let invalid = Color::new("not-a-color");
        assert!(invalid.is_err());
    }

    #[test]
    fn test_color_default() {
        let color = Color::default();
        assert_eq!(color.to_string(), "black");
    }

    #[test]
    fn test_color_with_alpha() {
        let color = Color::new("red").unwrap();
        let transparent = color.with_alpha(0.5);
        assert!((transparent.alpha() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_color_to_id_safe_string() {
        let color = Color::new("#ff0000").unwrap();
        let safe_id = color.to_id_safe_string();
        assert!(!safe_id.contains('#'));
        assert!(!safe_id.contains('('));
        assert!(!safe_id.contains(')'));
        assert!(!safe_id.contains(','));
        assert!(!safe_id.contains(' '));
    }

    #[test]
    fn test_color_display() {
        let color = Color::new("blue").unwrap();
        let display = format!("{}", color);
        assert!(!display.is_empty());
    }

    #[test]
    fn test_color_eq_hash() {
        use std::collections::HashSet;

        let color1 = Color::new("red").unwrap();
        let color2 = Color::new("red").unwrap();
        let color3 = Color::new("blue").unwrap();

        assert_eq!(color1, color2);
        assert_ne!(color1, color3);

        let mut set = HashSet::new();
        set.insert(color1);
        assert!(set.contains(&color2));
        assert!(!set.contains(&color3));
    }
}
