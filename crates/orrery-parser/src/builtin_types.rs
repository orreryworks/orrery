//! Built-in type definitions for the Orrery type system.
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

use orrery_core::{
    draw::{
        ActivationBoxDefinition, ActorDefinition, ArrowDefinition, BoundaryDefinition,
        ComponentDefinition, ControlDefinition, EntityDefinition, FragmentDefinition,
        InterfaceDefinition, NoteDefinition, OvalDefinition, RectangleDefinition, ShapeDefinition,
        StrokeDefinition, TextDefinition,
    },
    identifier::Id,
};

use crate::{
    Span,
    elaborate_utils::TypeDefinition as ElaborateTypeDefinition,
    parser_types::{Attribute, AttributeValue, TypeDefinition as ParserTypeDefinition, TypeSpec},
    span::Spanned,
};

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

/// Builder for creating built-in type definitions.
///
/// Registration order matters: a type that carries `stroke`/`text` attributes
/// only picks up references to the `Stroke`/`Text` types that were added
/// *before* it. Add [`add_stroke`](Self::add_stroke) and
/// [`add_text`](Self::add_text) first so the types that depend on them resolve
/// correctly.
///
/// # Examples
///
/// ```text
/// let types = BuiltinTypeBuilder::new()
///     .add_stroke(StrokeDefinition::default())
///     .add_text(TextDefinition::default())
///     .add_shape(RECTANGLE, RectangleDefinition::new())
///     .add_arrow(ArrowDefinition::default())
///     .add_note(NoteDefinition::new())
///     .into_elaborate_type_definitions();
/// ```
#[derive(Debug, Default)]
pub struct BuiltinTypeBuilder {
    stroke_id: Option<Spanned<Id>>,
    text_id: Option<Spanned<Id>>,
    parser_types: Vec<ParserTypeDefinition<'static>>,
    elaborate_types: Vec<ElaborateTypeDefinition>,
}

impl BuiltinTypeBuilder {
    /// Creates an empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a stroke type definition.
    ///
    /// Must be called before any type that references a `stroke` attribute.
    pub fn add_stroke(mut self, stroke_definition: StrokeDefinition) -> Self {
        let id = self.push_parser_type(STROKE, &[], &[]);
        self.stroke_id = Some(Spanned::new(id, Span::empty()));

        self.elaborate_types
            .push(ElaborateTypeDefinition::new_stroke(id, stroke_definition));

        self
    }

    /// Adds a text type definition.
    ///
    /// Must be called before any type that references a `text` attribute.
    pub fn add_text(mut self, text_definition: TextDefinition) -> Self {
        let id = self.push_parser_type(TEXT, &[], &[]);
        self.text_id = Some(Spanned::new(id, Span::empty()));

        self.elaborate_types
            .push(ElaborateTypeDefinition::new_text(id, text_definition));

        self
    }

    /// Adds an arrow type definition with default text styling.
    pub fn add_arrow(mut self, arrow_definition: ArrowDefinition) -> Self {
        let id = self.push_parser_type(ARROW, &["stroke"], &["text"]);

        self.elaborate_types
            .push(ElaborateTypeDefinition::new_arrow(
                id,
                Rc::new(arrow_definition),
            ));

        self
    }

    /// Adds a note type definition.
    pub fn add_note(mut self, note_definition: NoteDefinition) -> Self {
        let id = self.push_parser_type(NOTE, &["stroke"], &["text"]);

        self.elaborate_types.push(ElaborateTypeDefinition::new_note(
            id,
            Rc::new(note_definition),
        ));

        self
    }

    /// Adds an activation box type definition.
    pub fn add_activation_box(
        mut self,
        activation_box_definition: ActivationBoxDefinition,
    ) -> Self {
        let id = self.push_parser_type(ACTIVATE, &["stroke"], &[]);

        self.elaborate_types
            .push(ElaborateTypeDefinition::new_activation_box(
                id,
                Rc::new(activation_box_definition),
            ));

        self
    }

    /// Adds a fragment type definition with default text styling.
    pub fn add_fragment(mut self, name: &str, fragment_definition: FragmentDefinition) -> Self {
        let id = self.push_parser_type(
            name,
            &["border_stroke", "separator_stroke"],
            &["operation_label_text", "section_title_text"],
        );

        self.elaborate_types
            .push(ElaborateTypeDefinition::new_fragment(
                id,
                Rc::new(fragment_definition),
            ));

        self
    }

    /// Adds a shape type definition with default text styling.
    pub fn add_shape(
        mut self,
        name: &str,
        shape_definition: impl ShapeDefinition + 'static,
    ) -> Self {
        let id = self.push_parser_type(name, &["stroke"], &["text"]);

        self.elaborate_types
            .push(ElaborateTypeDefinition::new_shape(
                id,
                Rc::new(Box::new(shape_definition)),
            ));

        self
    }

    /// Consumes the builder, returning the [`Id`] of every registered type.
    pub fn into_ids(self) -> Vec<Id> {
        self.parser_types
            .into_iter()
            .map(|type_def| *type_def.name.inner())
            .collect()
    }

    /// Consumes the builder, returning the [`ParserTypeDefinition`]s injected as
    /// the built-in prelude during desugaring.
    pub fn into_parser_type_definitions(self) -> Vec<ParserTypeDefinition<'static>> {
        self.parser_types
    }

    /// Consumes the builder, returning the accumulated elaborated type
    /// definitions.
    pub fn into_elaborate_type_definitions(self) -> Vec<ElaborateTypeDefinition> {
        self.elaborate_types
    }

    /// Builds the `stroke`/`text` attributes that reference the registered
    /// `Stroke` and `Text` types under the given attribute names.
    ///
    /// References are emitted only for the `Stroke`/`Text` types already added,
    /// so [`add_stroke`](Self::add_stroke) and [`add_text`](Self::add_text) must
    /// precede any type that depends on them.
    ///
    /// # Panics
    ///
    /// In debug builds, panics if `stroke_names` (or `text_names`) is non-empty
    /// before the corresponding `Stroke` (or `Text`) type has been registered.
    fn attributes(
        &self,
        stroke_names: &[&'static str],
        text_names: &[&'static str],
    ) -> Vec<Attribute<'static>> {
        debug_assert!(
            stroke_names.is_empty() || self.stroke_id.is_some(),
            "stroke_names must be empty or stroke_id must be set"
        );
        debug_assert!(
            text_names.is_empty() || self.text_id.is_some(),
            "text_names must be empty or text_id must be set"
        );

        let mut attrs = Vec::new();
        if let Some(id) = self.stroke_id {
            for name in stroke_names {
                attrs.push(Attribute {
                    name: Spanned::new(name, Span::empty()),
                    value: AttributeValue::TypeSpec(TypeSpec {
                        type_name: Some(id),
                        attributes: vec![],
                    }),
                });
            }
        }
        if let Some(id) = self.text_id {
            for name in text_names {
                attrs.push(Attribute {
                    name: Spanned::new(name, Span::empty()),
                    value: AttributeValue::TypeSpec(TypeSpec {
                        type_name: Some(id),
                        attributes: vec![],
                    }),
                });
            }
        }
        attrs
    }

    /// Registers a parser-level type definition for `name` and returns its [`Id`].
    fn push_parser_type(
        &mut self,
        name: &str,
        stroke_names: &[&'static str],
        text_names: &[&'static str],
    ) -> Id {
        let id = Id::new(name);
        let spanned_id = Spanned::new(id, Span::empty());

        self.parser_types.push(ParserTypeDefinition {
            name: spanned_id,
            type_spec: TypeSpec {
                type_name: Some(spanned_id),
                attributes: self.attributes(stroke_names, text_names),
            },
        });
        id
    }
}

/// Creates a builder pre-populated with all default built-in types.
///
/// Single source of truth for the built-in types in the Orrery type system,
/// assembling the standard set via [`BuiltinTypeBuilder`]. `Stroke` and `Text`
/// are registered first so the types that reference them resolve their
/// `stroke`/`text` attributes.
pub fn defaults() -> BuiltinTypeBuilder {
    BuiltinTypeBuilder::new()
        // Attribute group types — must come first; later types reference these.
        .add_stroke(StrokeDefinition::default())
        .add_text(TextDefinition::default())
        // Relation type
        .add_arrow(ArrowDefinition::default())
        // Annotation type
        .add_note(NoteDefinition::new())
        // Activation type
        .add_activation_box(ActivationBoxDefinition::new())
        // Fragment type definitions for common operations
        .add_fragment(FRAGMENT_ALT, FragmentDefinition::new())
        .add_fragment(FRAGMENT_OPT, FragmentDefinition::new())
        .add_fragment(FRAGMENT_LOOP, FragmentDefinition::new())
        .add_fragment(FRAGMENT_PAR, FragmentDefinition::new())
        .add_fragment(FRAGMENT, FragmentDefinition::new())
        // Shape types
        .add_shape(RECTANGLE, RectangleDefinition::new())
        .add_shape(OVAL, OvalDefinition::new())
        .add_shape(COMPONENT, ComponentDefinition::new())
        .add_shape(BOUNDARY, BoundaryDefinition::new())
        .add_shape(ACTOR, ActorDefinition::new())
        .add_shape(ENTITY, EntityDefinition::new())
        .add_shape(CONTROL, ControlDefinition::new())
        .add_shape(INTERFACE, InterfaceDefinition::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_starts_empty() {
        let builder = BuiltinTypeBuilder::new();
        let types = builder.into_elaborate_type_definitions();
        assert_eq!(types.len(), 0);
    }

    #[test]
    fn test_builder_chaining() {
        // `Stroke`/`Text` must be registered before types that reference them.
        let types = BuiltinTypeBuilder::new()
            .add_stroke(StrokeDefinition::default())
            .add_text(TextDefinition::default())
            .add_arrow(ArrowDefinition::default())
            .add_note(NoteDefinition::new())
            .add_shape(RECTANGLE, RectangleDefinition::new())
            .into_elaborate_type_definitions();

        assert_eq!(types.len(), 5);
    }

    /// All built-in type names in [`defaults`] registration order.
    const DEFAULT_TYPE_NAMES: [&str; 18] = [
        STROKE,
        TEXT,
        ARROW,
        NOTE,
        ACTIVATE,
        FRAGMENT_ALT,
        FRAGMENT_OPT,
        FRAGMENT_LOOP,
        FRAGMENT_PAR,
        FRAGMENT,
        RECTANGLE,
        OVAL,
        COMPONENT,
        BOUNDARY,
        ACTOR,
        ENTITY,
        CONTROL,
        INTERFACE,
    ];

    #[test]
    fn test_defaults_creates_all_types() {
        assert_eq!(defaults().into_ids(), DEFAULT_TYPE_NAMES);
    }

    #[test]
    fn test_into_parser_type_definitions_wires_attribute_references() {
        let parser_types = defaults().into_parser_type_definitions();

        // Same types, in the same order, as the registered ids.
        let names: Vec<Id> = parser_types
            .iter()
            .map(|type_def| *type_def.name.inner())
            .collect();
        assert_eq!(names, DEFAULT_TYPE_NAMES);

        // `Stroke` is a leaf attribute group with no references of its own.
        let stroke = &parser_types[0];
        assert_eq!(*stroke.name.inner(), STROKE);
        assert!(stroke.type_spec.attributes.is_empty());

        // `Arrow` references `Stroke` and `Text` via its `stroke`/`text` attrs.
        let arrow = parser_types
            .iter()
            .find(|type_def| *type_def.name.inner() == ARROW)
            .expect("Arrow must be registered");
        let refs: Vec<(&str, Id)> = arrow
            .type_spec
            .attributes
            .iter()
            .map(|attr| {
                let type_spec = attr.value.as_type_spec().expect("attribute is a type ref");
                let type_name = *type_spec
                    .type_name
                    .as_ref()
                    .expect("type ref has a type name")
                    .inner();
                (*attr.name.inner(), type_name)
            })
            .collect();
        assert_eq!(
            refs,
            vec![("stroke", Id::new(STROKE)), ("text", Id::new(TEXT))]
        );
    }

    #[test]
    fn test_into_elaborate_type_definitions_match_kinds() {
        let types = defaults().into_elaborate_type_definitions();

        // Same types, in the same order, as the registered ids.
        let names: Vec<Id> = types.iter().map(|type_def| type_def.id()).collect();
        assert_eq!(names, DEFAULT_TYPE_NAMES);

        let find = |name: &str| {
            types
                .iter()
                .find(|type_def| type_def.id() == name)
                .unwrap_or_else(|| panic!("{name} must be registered"))
        };

        // Each built-in elaborates to the matching draw definition.
        assert!(find(STROKE).stroke_definition().is_ok());
        assert!(find(TEXT).text_definition_from_draw().is_ok());
        assert!(find(ARROW).arrow_definition().is_ok());
        assert!(find(NOTE).note_definition().is_ok());
        assert!(find(ACTIVATE).activation_box_definition().is_ok());
        assert!(find(FRAGMENT).fragment_definition().is_ok());
        assert!(find(RECTANGLE).shape_definition().is_ok());
    }
}
