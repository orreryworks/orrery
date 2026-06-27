//! Built-in type definitions for the Orrery type system.
//!
//! The `builtin_types!` table is the single source of truth for every built-in
//! base type, expanding into three views of that table: the parser-level
//! prelude ([`parser_type_definitions`]), the elaborated defaults
//! ([`elaborate_type_definitions`]), and the registered names ([`ids`]).
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
        ComponentDefinition, ControlDefinition, DiagramDefinition, EntityDefinition,
        FragmentDefinition, InterfaceDefinition, LifelineDefinition, NoteDefinition,
        OvalDefinition, RectangleDefinition, StrokeDefinition, TextDefinition,
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

/// Built-in base type for sequence-diagram lifelines
pub const LIFELINE: &str = "Lifeline";

/// Built-in configuration type for diagram-wide styling
pub const DIAGRAM: &str = "Diagram";

/// Built-in base type for stroke attribute groups
pub const STROKE: &str = "Stroke";

/// Built-in base type for text attribute groups
pub const TEXT: &str = "Text";

/// Builds a slot that references another built-in by its constant `type_name`
/// (e.g. [`STROKE`]), re-applying any `inline` overrides so the referencing
/// built-in's own non-default styling survives the wiring.
fn type_ref(
    name: &'static str,
    type_name: &str,
    inline: Vec<Attribute<'static>>,
) -> Attribute<'static> {
    Attribute {
        name: Spanned::new(name, Span::empty()),
        value: AttributeValue::TypeSpec(TypeSpec {
            type_name: Some(Spanned::new(Id::new(type_name), Span::empty())),
            attributes: inline,
        }),
    }
}

/// Turns an inline-override literal into an [`Attribute`].
///
/// This lets `builtin_types!` stay agnostic about the value's type: `&str`
/// values build a string attribute (`name="value"`) and floating-point values
/// build a float attribute (`name=value`), so the macro can emit a single
/// uniform call per override.
trait InlineValue {
    /// Builds the override attribute named `name` from `self`.
    fn into_attribute(self, name: &'static str) -> Attribute<'static>;
}

impl InlineValue for &str {
    /// Builds an inline string attribute (`name="value"`).
    fn into_attribute(self, name: &'static str) -> Attribute<'static> {
        Attribute {
            name: Spanned::new(name, Span::empty()),
            value: AttributeValue::String(Spanned::new(self.to_string(), Span::empty())),
        }
    }
}

impl InlineValue for f64 {
    /// Builds an inline float attribute (`name=value`).
    fn into_attribute(self, name: &'static str) -> Attribute<'static> {
        Attribute {
            name: Spanned::new(name, Span::empty()),
            value: AttributeValue::Float(Spanned::new(self as f32, Span::empty())),
        }
    }
}

/// Declares the full table of built-in types.
///
/// Each entry is `NAME => { parser: { .. }, elaborate: .. }`.
///
/// The `parser` block lists references declaratively:
///
/// ```text
/// "attr_name" => TargetType,                          // plain reference
/// "attr_name" => TargetType { "override" = "value" }, // string override
/// "attr_name" => TargetType { "override" = 2.0 },     // float override
/// ```
///
/// The `elaborate` field joins the constructor and the bare definition with a
/// `=>` arrow (not call syntax — the macro, not the constructor, takes the
/// definition):
///
/// ```text
/// elaborate: ElaborateTypeDefinition::new_shape => RectangleDefinition::new(),
/// ```
///
/// From the one table it expands a standalone function per consumer outcome —
/// [`ids`], [`parser_type_definitions`] and [`elaborate_type_definitions`] —
/// plus the test-only `DEFAULT_TYPE_NAMES`.
macro_rules! builtin_types {
    // Internal: build one elaborated definition, applying the wrapping each
    // constructor expects around the bare `*Definition` value.
    (@elaborate new_stroke, $definition:expr, $id:expr) => {
        ElaborateTypeDefinition::new_stroke($id, $definition)
    };
    (@elaborate new_text, $definition:expr, $id:expr) => {
        ElaborateTypeDefinition::new_text($id, $definition)
    };
    (@elaborate new_shape, $definition:expr, $id:expr) => {
        ElaborateTypeDefinition::new_shape($id, Rc::new(Box::new($definition)))
    };
    (@elaborate $constructor:ident, $definition:expr, $id:expr) => {
        ElaborateTypeDefinition::$constructor($id, Rc::new($definition))
    };

    // Public: the built-in type table.
    (
        $(
            $name:expr => {
                parser: {
                    $(
                        $attr:literal => $target:path
                        $( { $( $ovr_name:literal = $ovr_value:expr ),* $(,)? } )?
                    ),* $(,)?
                },
                elaborate: ElaborateTypeDefinition::$constructor:ident => $definition:expr $(,)?
            }
        ),* $(,)?
    ) => {

        /// The [`Id`] of every built-in type, in declaration order.
        ///
        /// These names are reserved: they resolve as built-ins everywhere and
        /// are never namespace-qualified.
        pub fn ids() -> Vec<Id> {
            vec![ $( Id::new($name) ),* ]
        }

        /// The built-in prelude, as parser-level [`ParserTypeDefinition`]s.
        ///
        /// Inter-built-in references resolve by constant name, so a referenced
        /// target (e.g. `Stroke`) need not precede the type that uses it.
        pub fn parser_type_definitions() -> Vec<ParserTypeDefinition<'static>> {
            vec![
                $(
                    {
                        let spanned_id = Spanned::new(Id::new($name), Span::empty());
                        ParserTypeDefinition {
                            name: spanned_id,
                            type_spec: TypeSpec {
                                type_name: Some(spanned_id),
                                attributes: vec![
                                    $(
                                        type_ref($attr, $target, vec![
                                            $(
                                                $(
                                                    InlineValue::into_attribute($ovr_value, $ovr_name)
                                                ),*
                                            )?
                                        ])
                                    ),*
                                ],
                            },
                        }
                    }
                ),*
            ]
        }

        /// The elaborated default definition for each built-in type.
        pub fn elaborate_type_definitions() -> Vec<ElaborateTypeDefinition> {
            vec![
                $(
                    builtin_types!(@elaborate $constructor, $definition, Id::new($name))
                ),*
            ]
        }
    };
}

builtin_types! {
    STROKE => {
        parser: {},
        elaborate: ElaborateTypeDefinition::new_stroke => StrokeDefinition::default(),
    },
    TEXT => {
        parser: {},
        elaborate: ElaborateTypeDefinition::new_text => TextDefinition::default(),
    },
    LIFELINE => {
        parser: {
            "stroke" => STROKE { "style" = "dashed" },
        },
        elaborate: ElaborateTypeDefinition::new_lifeline => LifelineDefinition::default(),
    },
    ARROW => {
        parser: {
            "stroke" => STROKE,
            "text" => TEXT { "background_color" = "rgba(255, 255, 255, 0.85)" },
        },
        elaborate: ElaborateTypeDefinition::new_arrow => ArrowDefinition::default(),
    },
    NOTE => {
        parser: {
            "stroke" => STROKE,
            "text" => TEXT,
        },
        elaborate: ElaborateTypeDefinition::new_note => NoteDefinition::new(),
    },
    ACTIVATE => {
        parser: {
            "stroke" => STROKE,
        },
        elaborate: ElaborateTypeDefinition::new_activation_box => ActivationBoxDefinition::new(),
    },
    FRAGMENT_ALT => {
        parser: {
            "border_stroke" => STROKE,
            "separator_stroke" => STROKE { "style" = "dashed" },
        },
        elaborate: ElaborateTypeDefinition::new_fragment => FragmentDefinition::new(),
    },
    FRAGMENT_OPT => {
        parser: {
            "border_stroke" => STROKE,
            "separator_stroke" => STROKE { "style" = "dashed" },
        },
        elaborate: ElaborateTypeDefinition::new_fragment => FragmentDefinition::new(),
    },
    FRAGMENT_LOOP => {
        parser: {
            "border_stroke" => STROKE,
            "separator_stroke" => STROKE { "style" = "dashed" },
        },
        elaborate: ElaborateTypeDefinition::new_fragment => FragmentDefinition::new(),
    },
    FRAGMENT_PAR => {
        parser: {
            "border_stroke" => STROKE,
            "separator_stroke" => STROKE { "style" = "dashed" },
        },
        elaborate: ElaborateTypeDefinition::new_fragment => FragmentDefinition::new(),
    },
    FRAGMENT => {
        parser: {
            "border_stroke" => STROKE,
            "separator_stroke" => STROKE { "style" = "dashed" },
        },
        elaborate: ElaborateTypeDefinition::new_fragment => FragmentDefinition::new(),
    },
    RECTANGLE => {
        parser: {
            "stroke" => STROKE { "width" = 2.0 },
            "text" => TEXT,
        },
        elaborate: ElaborateTypeDefinition::new_shape => RectangleDefinition::new(),
    },
    OVAL => {
        parser: {
            "stroke" => STROKE { "width" = 2.0 },
            "text" => TEXT,
        },
        elaborate: ElaborateTypeDefinition::new_shape => OvalDefinition::new(),
    },
    COMPONENT => {
        parser: {
            "stroke" => STROKE { "width" = 2.0 },
            "text" => TEXT,
        },
        elaborate: ElaborateTypeDefinition::new_shape => ComponentDefinition::new(),
    },
    BOUNDARY => {
        parser: {
            "stroke" => STROKE { "width" = 2.0 },
            "text" => TEXT,
        },
        elaborate: ElaborateTypeDefinition::new_shape => BoundaryDefinition::new(),
    },
    ACTOR => {
        parser: {
            "stroke" => STROKE { "width" = 2.0 },
            "text" => TEXT,
        },
        elaborate: ElaborateTypeDefinition::new_shape => ActorDefinition::new(),
    },
    ENTITY => {
        parser: {
            "stroke" => STROKE { "width" = 2.0 },
            "text" => TEXT,
        },
        elaborate: ElaborateTypeDefinition::new_shape => EntityDefinition::new(),
    },
    CONTROL => {
        parser: {
            "stroke" => STROKE { "width" = 2.0 },
            "text" => TEXT,
        },
        elaborate: ElaborateTypeDefinition::new_shape => ControlDefinition::new(),
    },
    INTERFACE => {
        parser: {
            "stroke" => STROKE { "width" = 2.0 },
            "text" => TEXT,
        },
        elaborate: ElaborateTypeDefinition::new_shape => InterfaceDefinition::new(),
    },
    DIAGRAM => {
        parser: {
            "lifeline" => LIFELINE,
        },
        elaborate: ElaborateTypeDefinition::new_diagram => DiagramDefinition::new(),
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_type_definitions_wires_attribute_references() {
        let parser_types = parser_type_definitions();

        // Same types, in the same order, as `ids`.
        let names: Vec<Id> = parser_types
            .iter()
            .map(|type_def| *type_def.name.inner())
            .collect();
        assert_eq!(names, ids());

        // `Stroke` is a leaf attribute group with no references of its own.
        let stroke = &parser_types[0];
        assert_eq!(*stroke.name.inner(), STROKE);
        assert!(stroke.type_spec.attributes.is_empty());

        // `Lifeline` wires its `stroke` to `Stroke` with an inline `style="dashed"`.
        let lifeline = parser_types
            .iter()
            .find(|type_def| *type_def.name.inner() == LIFELINE)
            .expect("Lifeline must be registered");
        assert_eq!(lifeline.type_spec.attributes.len(), 1);
        let stroke_attr = &lifeline.type_spec.attributes[0];
        assert_eq!(*stroke_attr.name.inner(), "stroke");
        let stroke_ref = stroke_attr
            .value
            .as_type_spec()
            .expect("stroke attribute is a type ref");
        assert_eq!(
            stroke_ref.type_name.as_ref().map(|name| *name.inner()),
            Some(Id::new(STROKE))
        );
        assert_eq!(stroke_ref.attributes.len(), 1);
        assert_eq!(*stroke_ref.attributes[0].name.inner(), "style");
        assert_eq!(stroke_ref.attributes[0].value.as_str(), Ok("dashed"));

        // `Diagram` references `Lifeline` via its `lifeline` attr.
        let diagram = parser_types
            .iter()
            .find(|type_def| *type_def.name.inner() == DIAGRAM)
            .expect("Diagram must be registered");
        let diagram_refs: Vec<(&str, Id)> = diagram
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
        assert_eq!(diagram_refs, vec![("lifeline", Id::new(LIFELINE))]);

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

        // `Arrow`'s `text` re-applies the translucent label background so the
        // shared `Text` wiring doesn't drop it.
        let arrow_text = arrow
            .type_spec
            .attributes
            .iter()
            .find(|attr| *attr.name.inner() == "text")
            .expect("Arrow must wire a `text` attribute");
        let arrow_text_ref = arrow_text
            .value
            .as_type_spec()
            .expect("text attribute is a type ref");
        assert_eq!(arrow_text_ref.attributes.len(), 1);
        assert_eq!(
            *arrow_text_ref.attributes[0].name.inner(),
            "background_color"
        );
        assert_eq!(
            arrow_text_ref.attributes[0].value.as_str(),
            Ok("rgba(255, 255, 255, 0.85)")
        );

        // `Rectangle` (a shape) re-applies its 2.0-wide outline on top of the
        // shared `Stroke`, which defaults to 1.0.
        let rectangle = parser_types
            .iter()
            .find(|type_def| *type_def.name.inner() == RECTANGLE)
            .expect("Rectangle must be registered");
        let rectangle_stroke = rectangle
            .type_spec
            .attributes
            .iter()
            .find(|attr| *attr.name.inner() == "stroke")
            .expect("Rectangle must wire a `stroke` attribute");
        let rectangle_stroke_ref = rectangle_stroke
            .value
            .as_type_spec()
            .expect("stroke attribute is a type ref");
        assert_eq!(
            rectangle_stroke_ref
                .type_name
                .as_ref()
                .map(|name| *name.inner()),
            Some(Id::new(STROKE))
        );
        assert_eq!(rectangle_stroke_ref.attributes.len(), 1);
        assert_eq!(*rectangle_stroke_ref.attributes[0].name.inner(), "width");
        assert_eq!(rectangle_stroke_ref.attributes[0].value.as_float(), Ok(2.0));

        // `Fragment` wires its strokes (re-applying the dashed separator) but
        // leaves the label-text slots unwired so their bespoke defaults survive.
        let fragment = parser_types
            .iter()
            .find(|type_def| *type_def.name.inner() == FRAGMENT)
            .expect("Fragment must be registered");
        let fragment_refs: Vec<(&str, Id)> = fragment
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
            fragment_refs,
            vec![
                ("border_stroke", Id::new(STROKE)),
                ("separator_stroke", Id::new(STROKE)),
            ]
        );
        let separator_ref = fragment
            .type_spec
            .attributes
            .iter()
            .find(|attr| *attr.name.inner() == "separator_stroke")
            .expect("Fragment must wire a `separator_stroke`")
            .value
            .as_type_spec()
            .expect("separator_stroke is a type ref");
        assert_eq!(separator_ref.attributes.len(), 1);
        assert_eq!(*separator_ref.attributes[0].name.inner(), "style");
        assert_eq!(separator_ref.attributes[0].value.as_str(), Ok("dashed"));
    }

    #[test]
    fn test_elaborate_type_definitions_match_kinds() {
        let types = elaborate_type_definitions();

        // Same types, in the same order, as `ids`.
        let names: Vec<Id> = types.iter().map(|type_def| type_def.id()).collect();
        assert_eq!(names, ids());

        let find = |name: &str| {
            types
                .iter()
                .find(|type_def| type_def.id() == name)
                .unwrap_or_else(|| panic!("{name} must be registered"))
        };

        // Each built-in elaborates to the matching draw definition.
        assert!(find(STROKE).stroke_definition().is_ok());
        assert!(find(TEXT).text_definition_from_draw().is_ok());
        assert!(find(LIFELINE).lifeline_definition().is_ok());
        assert!(find(ARROW).arrow_definition().is_ok());
        assert!(find(NOTE).note_definition().is_ok());
        assert!(find(ACTIVATE).activation_box_definition().is_ok());
        assert!(find(FRAGMENT).fragment_definition().is_ok());
        assert!(find(RECTANGLE).shape_definition().is_ok());
        assert!(find(DIAGRAM).diagram_definition().is_ok());
    }
}
