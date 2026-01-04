//! Built-in type definitions for the Filament type system
//!
//! This module provides:
//! - String constants for all built-in base types
//! - Builder pattern for creating default type definitions
//! - Centralized location for all built-in type logic
//!
//! # Built-in Type Categories
//!
//! - **Shapes**: Rectangle, Oval, Component, Boundary, Actor, Entity, Control, Interface
//! - **Relations**: Arrow
//! - **Fragments**: Fragment, FragmentAlt, FragmentOpt, FragmentLoop, FragmentPar
//! - **Annotations**: Note
//! - **Activations**: Activate

use std::rc::Rc;

use super::elaborate_utils::TypeDefinition;
use crate::{draw, identifier::Id};

/// Built-in base type for rectangular shapes
pub const RECTANGLE: &str = "Rectangle";

/// Built-in base type for oval/elliptical shapes
pub const OVAL: &str = "Oval";

/// Built-in base type for UML component shapes
pub const COMPONENT: &str = "Component";

/// Built-in base type for UML boundary shapes (system boundaries)
pub const BOUNDARY: &str = "Boundary";

/// Built-in base type for UML actor shapes (stick figures)
pub const ACTOR: &str = "Actor";

/// Built-in base type for UML entity shapes (data entities)
pub const ENTITY: &str = "Entity";

/// Built-in base type for UML control shapes (control logic)
pub const CONTROL: &str = "Control";

/// Built-in base type for UML interface shapes (system interfaces)
pub const INTERFACE: &str = "Interface";

/// Built-in base type for relations (arrows)
pub const ARROW: &str = "Arrow";

/// Built-in base type for generic fragments
pub const FRAGMENT: &str = "Fragment";

/// Built-in type for alternative fragments (alt/else)
pub const FRAGMENT_ALT: &str = "FragmentAlt";

/// Built-in type for optional fragments (opt)
pub const FRAGMENT_OPT: &str = "FragmentOpt";

/// Built-in type for loop fragments (loop)
pub const FRAGMENT_LOOP: &str = "FragmentLoop";

/// Built-in type for parallel fragments (par)
pub const FRAGMENT_PAR: &str = "FragmentPar";

/// Built-in base type for notes
pub const NOTE: &str = "Note";

/// Built-in base type for activations
pub const ACTIVATE: &str = "Activate";

/// Built-in base type for stroke attribute groups
pub const STROKE: &str = "Stroke";

/// Built-in base type for text attribute groups
pub const TEXT: &str = "Text";

/// Builder for creating built-in type definitions
///
/// # Example
///
/// ```text
/// let types = BuiltinTypeBuilder::new()
///     .add_shape(RECTANGLE, RectangleDefinition::new())
///     .add_arrow(ARROW, ArrowDefinition::default())
///     .add_note(NOTE, NoteDefinition::new())
///     .build();
/// ```
#[derive(Debug, Default)]
pub struct BuiltinTypeBuilder {
    types: Vec<TypeDefinition>,
}

impl BuiltinTypeBuilder {
    /// Create a new empty builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a shape type definition with default text styling
    ///
    /// Returns `self` for method chaining.
    pub fn add_shape(
        mut self,
        name: &str,
        shape_definition: impl draw::ShapeDefinition + 'static,
    ) -> Self {
        self.types.push(TypeDefinition::new_shape(
            Id::new(name),
            Rc::new(Box::new(shape_definition)),
        ));
        self
    }

    /// Add an arrow type definition with default text styling
    ///
    /// Returns `self` for method chaining.
    pub fn add_arrow(mut self, name: &str, arrow_definition: draw::ArrowDefinition) -> Self {
        self.types.push(TypeDefinition::new_arrow(
            Id::new(name),
            Rc::new(arrow_definition),
        ));
        self
    }

    /// Add a fragment type definition with default text styling
    ///
    /// Returns `self` for method chaining.
    pub fn add_fragment(
        mut self,
        name: &str,
        fragment_definition: draw::FragmentDefinition,
    ) -> Self {
        self.types.push(TypeDefinition::new_fragment(
            Id::new(name),
            Rc::new(fragment_definition),
        ));
        self
    }

    /// Add a note type definition
    ///
    /// Returns `self` for method chaining.
    pub fn add_note(mut self, name: &str, note_definition: draw::NoteDefinition) -> Self {
        self.types.push(TypeDefinition::new_note(
            Id::new(name),
            Rc::new(note_definition),
        ));
        self
    }

    /// Add an activation box type definition
    ///
    /// Returns `self` for method chaining.
    pub fn add_activation_box(
        mut self,
        name: &str,
        activation_box_definition: draw::ActivationBoxDefinition,
    ) -> Self {
        self.types.push(TypeDefinition::new_activation_box(
            Id::new(name),
            Rc::new(activation_box_definition),
        ));
        self
    }

    /// Add a stroke type definition
    ///
    /// Returns `self` for method chaining.
    pub fn add_stroke(mut self, name: &str, stroke_definition: draw::StrokeDefinition) -> Self {
        self.types
            .push(TypeDefinition::new_stroke(Id::new(name), stroke_definition));
        self
    }

    /// Add a text type definition
    ///
    /// Returns `self` for method chaining.
    pub fn add_text(mut self, name: &str, text_definition: draw::TextDefinition) -> Self {
        self.types
            .push(TypeDefinition::new_text(Id::new(name), text_definition));
        self
    }

    /// Build and return all registered type definitions
    ///
    /// Consumes the builder and returns the accumulated type definitions.
    pub fn build(self) -> Vec<TypeDefinition> {
        self.types
    }
}

/// Create all default built-in type definitions
///
/// This function acts as the single source of truth for built-in types in the
/// Filament type system. It uses the `BuiltinTypeBuilder` to construct the
/// standard set of types.
pub fn defaults() -> Vec<TypeDefinition> {
    BuiltinTypeBuilder::new()
        // Attribute group types
        .add_stroke(STROKE, draw::StrokeDefinition::default())
        .add_text(TEXT, draw::TextDefinition::default())
        // Shape types
        .add_shape(RECTANGLE, draw::RectangleDefinition::new())
        .add_shape(OVAL, draw::OvalDefinition::new())
        .add_shape(COMPONENT, draw::ComponentDefinition::new())
        .add_shape(BOUNDARY, draw::BoundaryDefinition::new())
        .add_shape(ACTOR, draw::ActorDefinition::new())
        .add_shape(ENTITY, draw::EntityDefinition::new())
        .add_shape(CONTROL, draw::ControlDefinition::new())
        .add_shape(INTERFACE, draw::InterfaceDefinition::new())
        // Relation type
        .add_arrow(ARROW, draw::ArrowDefinition::default())
        // Fragment type definitions for common operations
        .add_fragment(FRAGMENT_ALT, draw::FragmentDefinition::new())
        .add_fragment(FRAGMENT_OPT, draw::FragmentDefinition::new())
        .add_fragment(FRAGMENT_LOOP, draw::FragmentDefinition::new())
        .add_fragment(FRAGMENT_PAR, draw::FragmentDefinition::new())
        .add_fragment(FRAGMENT, draw::FragmentDefinition::new())
        // Annotation type
        .add_note(NOTE, draw::NoteDefinition::new())
        // Activation type
        .add_activation_box(ACTIVATE, draw::ActivationBoxDefinition::new())
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_starts_empty() {
        let builder = BuiltinTypeBuilder::new();
        let types = builder.build();
        assert_eq!(types.len(), 0);
    }

    #[test]
    fn test_builder_chaining() {
        let types = BuiltinTypeBuilder::new()
            .add_shape(RECTANGLE, draw::RectangleDefinition::new())
            .add_arrow(ARROW, draw::ArrowDefinition::default())
            .add_note(NOTE, draw::NoteDefinition::new())
            .build();

        assert_eq!(types.len(), 3);
    }

    #[test]
    fn test_defaults_creates_all_types() {
        let types = defaults();
        assert_eq!(types.len(), 18);

        let has_type = |name: &str| types.iter().any(|t| t.id() == name);

        // Attribute groups
        assert!(has_type(STROKE));
        assert!(has_type(TEXT));

        // Shapes
        assert!(has_type(RECTANGLE));
        assert!(has_type(OVAL));
        assert!(has_type(COMPONENT));
        assert!(has_type(BOUNDARY));
        assert!(has_type(ACTOR));
        assert!(has_type(ENTITY));
        assert!(has_type(CONTROL));
        assert!(has_type(INTERFACE));

        // Relation
        assert!(has_type(ARROW));

        // Fragments
        assert!(has_type(FRAGMENT));
        assert!(has_type(FRAGMENT_ALT));
        assert!(has_type(FRAGMENT_OPT));
        assert!(has_type(FRAGMENT_LOOP));
        assert!(has_type(FRAGMENT_PAR));

        // Note & Activation
        assert!(has_type(NOTE));
        assert!(has_type(ACTIVATE));
    }
}
