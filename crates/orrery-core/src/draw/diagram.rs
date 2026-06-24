//! Diagram-wide styling shared across a whole diagram.
//!
//! Unlike most types in this module, [`DiagramDefinition`] is a configuration
//! container, not a [`Drawable`](crate::draw::Drawable): there is no diagram
//! shape to render, only settings (canvas color, lifeline) that apply diagram-wide.

use std::rc::Rc;

use crate::{color::Color, draw::LifelineDefinition};

/// Diagram-wide styling configuration.
///
/// Defaults to a transparent canvas (`canvas_color` is `None`) and a default
/// [`LifelineDefinition`].
#[derive(Debug, Clone, Default)]
pub struct DiagramDefinition {
    canvas_color: Option<Color>,
    lifeline: Rc<LifelineDefinition>,
}

impl DiagramDefinition {
    /// Creates a new diagram definition with default values.
    ///
    /// Delegates to [`DiagramDefinition::default`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the canvas (background) color, if any.
    pub fn canvas_color(&self) -> Option<Color> {
        self.canvas_color
    }

    /// Returns the lifeline definition.
    pub fn lifeline(&self) -> &Rc<LifelineDefinition> {
        &self.lifeline
    }

    /// Sets the canvas (background) color.
    ///
    /// Use `None` to leave the diagram background transparent.
    pub fn set_canvas_color(&mut self, color: Option<Color>) {
        self.canvas_color = color;
    }

    /// Sets the lifeline definition.
    pub fn set_lifeline(&mut self, lifeline: Rc<LifelineDefinition>) {
        self.lifeline = lifeline;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::StrokeDefinition;

    #[test]
    fn test_set_canvas_color() {
        let mut def = DiagramDefinition::new();
        let color = Color::new("red").expect("valid color");
        def.set_canvas_color(Some(color));

        assert_eq!(def.canvas_color(), Some(color));

        def.set_canvas_color(None);
        assert!(def.canvas_color().is_none());
    }

    #[test]
    fn test_set_lifeline() {
        let mut def = DiagramDefinition::new();
        let lifeline = Rc::new(LifelineDefinition::new(Rc::new(
            StrokeDefinition::default_solid(),
        )));
        def.set_lifeline(Rc::clone(&lifeline));

        assert_eq!(
            *def.lifeline().stroke().style(),
            crate::draw::StrokeStyle::Solid
        );
    }
}
