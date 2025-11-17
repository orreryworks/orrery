//! Validation module for AST elements using the visitor pattern
//!
//! This module implements a visitor-based (read-only) traversal system for AST validation.
//! It sits between the desugar and elaboration phases, allowing for semantic
//! validation of the AST before elaboration.
//!
//! ## Validations Performed
//!
//! - **Component Identifier References**: Validates that all component identifiers referenced
//!   in relations, notes, and activation statements are defined in the diagram
//! - **Activate/Deactivate Pairing**: Ensures activate statements have corresponding deactivate
//!   statements in sequence diagrams
//! - **Note Alignment**: Validates that note alignment values are appropriate for the diagram type

use std::collections::HashMap;

use super::{
    parser_types::{
        Attribute, AttributeValue, Diagram, Element, Fragment, FragmentSection, Note,
        TypeDefinition, TypeSpec,
    },
    span::{Span, Spanned},
};
use crate::{
    error::diagnostic::{DiagnosticError, Result},
    identifier::Id,
};

/// Visitor trait for traversing/analyzing AST nodes.
///
/// Each method takes a reference to its input and can accumulate state or errors.
/// Default implementations perform recursive traversal so implementors can override
/// only the methods they care about.
pub trait Visitor<'a> {
    /// Visit a complete diagram
    fn visit_diagram(&mut self, diagram: &Diagram<'a>) {
        self.visit_diagram_kind(&diagram.kind);
        self.visit_attributes(&diagram.attributes);
        self.visit_type_definitions(&diagram.type_definitions);
        self.visit_elements(&diagram.elements);
    }

    /// Visit the diagram kind (component, sequence, etc.)
    fn visit_diagram_kind(&mut self, _kind: &Spanned<&'a str>) {}

    /// Visit a list of attributes
    fn visit_attributes(&mut self, attributes: &[Attribute<'a>]) {
        for attr in attributes {
            self.visit_attribute(attr);
        }
    }

    /// Visit a single attribute
    fn visit_attribute(&mut self, attribute: &Attribute<'a>) {
        self.visit_attribute_name(&attribute.name);
        self.visit_attribute_value(&attribute.value);
    }

    /// Visit an attribute name
    fn visit_attribute_name(&mut self, _name: &Spanned<&'a str>) {}

    /// Visit an attribute value
    fn visit_attribute_value(&mut self, value: &AttributeValue<'a>) {
        match value {
            AttributeValue::String(s) => self.visit_string_value(s),
            AttributeValue::Float(f) => self.visit_float_value(f),
            AttributeValue::Attributes(attrs) => self.visit_attributes(attrs),
            AttributeValue::Identifiers(ids) => self.visit_identifiers(ids),
            AttributeValue::Empty => {}
        }
    }

    /// Visit a string attribute value
    fn visit_string_value(&mut self, _value: &Spanned<String>) {}

    /// Visit a float attribute value
    fn visit_float_value(&mut self, _value: &Spanned<f32>) {}

    /// Visit a single identifier (component reference)
    fn visit_identifier(&mut self, _identifier: &Spanned<Id>) {}

    /// Visit an identifiers attribute value (list of identifiers)
    fn visit_identifiers(&mut self, identifiers: &[Spanned<Id>]) {
        for identifier in identifiers {
            self.visit_identifier(identifier);
        }
    }

    /// Visit a list of type definitions
    fn visit_type_definitions(&mut self, type_definitions: &[TypeDefinition<'a>]) {
        for td in type_definitions {
            self.visit_type_definition(td);
        }
    }

    /// Visit a single type definition
    fn visit_type_definition(&mut self, type_def: &TypeDefinition<'a>) {
        self.visit_type_name(&type_def.name);
        self.visit_type_spec(&type_def.type_spec);
    }

    /// Visit a type name
    fn visit_type_name(&mut self, _name: &Spanned<Id>) {}

    /// Visit a base type
    fn visit_base_type(&mut self, _base_type: &Spanned<Id>) {}

    /// Visit a type specification
    fn visit_type_spec(&mut self, type_spec: &TypeSpec<'a>) {
        if let Some(ref type_name) = type_spec.type_name {
            self.visit_base_type(type_name);
        }
        self.visit_attributes(&type_spec.attributes);
    }

    /// Visit a list of elements
    fn visit_elements(&mut self, elements: &[Element<'a>]) {
        for elem in elements {
            self.visit_element(elem);
        }
    }

    /// Visit a single element
    fn visit_element(&mut self, element: &Element<'a>) {
        match *element {
            Element::Component {
                ref name,
                ref display_name,
                ref type_spec,
                ref nested_elements,
            } => self.visit_component(name, display_name, type_spec, nested_elements),
            Element::Relation {
                ref source,
                ref target,
                ref relation_type,
                ref type_spec,
                ref label,
            } => self.visit_relation(source, target, relation_type, type_spec, label),
            Element::Diagram(ref diagram) => self.visit_diagram(diagram),
            Element::ActivateBlock {
                ref component,
                ref type_spec,
                ref elements,
            } => self.visit_activate_block(component, type_spec, elements),
            Element::Activate {
                ref component,
                ref type_spec,
            } => self.visit_activate(component, type_spec),
            Element::Deactivate { ref component } => self.visit_deactivate(component),
            Element::Fragment(ref fragment) => self.visit_fragment(fragment),
            Element::AltElseBlock {
                keyword_span: _,
                ref type_spec,
                ref sections,
            } => {
                self.visit_type_spec(type_spec);
                for section in sections {
                    self.visit_elements(&section.elements);
                }
            }
            Element::OptBlock {
                keyword_span: _,
                ref type_spec,
                ref section,
            } => {
                self.visit_type_spec(type_spec);
                self.visit_elements(&section.elements);
            }
            Element::LoopBlock {
                keyword_span: _,
                ref type_spec,
                ref section,
            } => {
                self.visit_type_spec(type_spec);
                self.visit_elements(&section.elements);
            }
            Element::ParBlock {
                keyword_span: _,
                ref type_spec,
                ref sections,
            } => {
                self.visit_type_spec(type_spec);
                for section in sections {
                    self.visit_elements(&section.elements);
                }
            }
            Element::BreakBlock {
                keyword_span: _,
                ref type_spec,
                ref section,
            } => {
                self.visit_type_spec(type_spec);
                self.visit_elements(&section.elements);
            }
            Element::CriticalBlock {
                keyword_span: _,
                ref type_spec,
                ref section,
            } => {
                self.visit_type_spec(type_spec);
                self.visit_elements(&section.elements);
            }
            Element::Note(ref note) => {
                self.visit_note(note);
            }
        }
    }

    /// Visit a fragment
    fn visit_fragment(&mut self, fragment: &Fragment<'a>) {
        for section in &fragment.sections {
            self.visit_fragment_section(section);
        }
    }

    /// Visit a fragment section
    fn visit_fragment_section(&mut self, section: &FragmentSection<'a>) {
        // Traverse section title as a string literal and its elements
        if let Some(title) = &section.title {
            self.visit_string_value(title);
        }
        self.visit_elements(&section.elements);
    }

    /// Visit a component element
    fn visit_component(
        &mut self,
        name: &Spanned<Id>,
        display_name: &Option<Spanned<String>>,
        type_spec: &TypeSpec<'a>,
        nested_elements: &[Element<'a>],
    ) {
        self.visit_component_name(name);
        if let Some(dn) = display_name {
            self.visit_display_name(dn);
        }
        self.visit_type_spec(type_spec);
        self.visit_elements(nested_elements);
    }

    /// Visit a component name
    fn visit_component_name(&mut self, _name: &Spanned<Id>) {}

    /// Visit a display name
    fn visit_display_name(&mut self, _display_name: &Spanned<String>) {}

    /// Visit a relation element
    fn visit_relation(
        &mut self,
        source: &Spanned<Id>,
        target: &Spanned<Id>,
        relation_type: &Spanned<&'a str>,
        type_spec: &TypeSpec<'a>,
        label: &Option<Spanned<String>>,
    ) {
        self.visit_relation_source(source);
        self.visit_relation_target(target);
        self.visit_relation_type(relation_type);
        self.visit_type_spec(type_spec);
        if let Some(l) = label {
            self.visit_relation_label(l);
        }
    }

    /// Visit a relation source
    fn visit_relation_source(&mut self, source: &Spanned<Id>) {
        self.visit_identifier(source);
    }

    /// Visit a relation target
    fn visit_relation_target(&mut self, target: &Spanned<Id>) {
        self.visit_identifier(target);
    }

    /// Visit a relation type
    fn visit_relation_type(&mut self, _relation_type: &Spanned<&'a str>) {}

    /// Visit a relation label
    fn visit_relation_label(&mut self, _label: &Spanned<String>) {}

    /// Visit an activate block element
    fn visit_activate_block(
        &mut self,
        component: &Spanned<Id>,
        type_spec: &TypeSpec<'a>,
        elements: &[Element<'a>],
    ) {
        self.visit_activate_component(component);
        self.visit_type_spec(type_spec);
        self.visit_elements(elements);
    }

    /// Visit an activate block component reference
    fn visit_activate_component(&mut self, _component: &Spanned<Id>) {}

    /// Visit an activate statement
    fn visit_activate(&mut self, component: &Spanned<Id>, type_spec: &TypeSpec<'a>) {
        self.visit_identifier(component);
        self.visit_type_spec(type_spec);
    }

    /// Visit a deactivate statement
    fn visit_deactivate(&mut self, component: &Spanned<Id>) {
        self.visit_identifier(component);
    }

    /// Visit a note element
    fn visit_note(&mut self, note: &Note<'a>) {
        self.visit_type_spec(&note.type_spec);
        self.visit_note_content(&note.content);
    }

    /// Visit note content
    fn visit_note_content(&mut self, _content: &Spanned<String>) {}
}

/// Entry point for running a visitor on a diagram
pub fn visit_diagram<'a, V: Visitor<'a>>(visitor: &mut V, diagram: &Diagram<'a>) {
    visitor.visit_diagram(diagram)
}

/// Validator that checks all diagram semantic constraints
///
/// Uses a visitor-based traversal to validate:
/// - Component identifier references (relations, notes, activate/deactivate)
/// - Activate/deactivate pairing in sequence diagrams
/// - Note attribute values (align)
///
/// The validator collects all errors during traversal for reporting after traversal.
///
/// ## Component Registry
///
/// The validator maintains a component registry (`Vec<HashMap<String, Span>>`) with one
/// registry per diagram. Components are registered as they are visited, and all subsequent
/// identifier references (in relations, activations, and notes) are validated against this
/// registry. Since identifiers are fully qualified after desugaring, all components in a
/// diagram are accessible regardless of nesting depth.
pub struct Validator<'a> {
    // Activation validation state
    activation_stack: Vec<HashMap<Id, Vec<Span>>>,

    // Component identifier registry for validation
    component_registry: Vec<HashMap<Id, Span>>,

    // Note validation state
    diagram_kind: Option<&'a str>,

    // Shared error collection
    errors: Vec<DiagnosticError>,
}

impl<'a> Validator<'a> {
    pub fn new() -> Self {
        Self {
            activation_stack: Vec::new(),
            component_registry: Vec::new(),
            diagram_kind: None,
            errors: Vec::new(),
        }
    }

    fn activation_state_mut(&mut self) -> &mut HashMap<Id, Vec<Span>> {
        self.activation_stack
            .last_mut()
            .expect("activation scope not initialized")
    }

    /// Validate that align value is appropriate for the diagram type
    ///
    /// Sequence diagrams support: over, left, right
    /// Component diagrams support: left, right, top, bottom
    ///
    /// Note: The None and unknown diagram type cases are defensive programming.
    /// The parser enforces valid diagram types, but we handle these cases
    /// to fail gracefully if the validation is called incorrectly.
    fn validate_align_for_diagram_type(&mut self, align_value: &str, span: Span) {
        match self.diagram_kind {
            Some("sequence") => {
                if !matches!(align_value, "over" | "left" | "right") {
                    self.errors.push(DiagnosticError::from_span(
                        format!("Invalid align value '{}' for sequence diagram. Valid values: over, left, right", align_value),
                        span,
                        "invalid align value",
                        None,
                    ));
                }
            }
            Some("component") => {
                if !matches!(align_value, "left" | "right" | "top" | "bottom") {
                    self.errors.push(DiagnosticError::from_span(
                        format!("Invalid align value '{}' for component diagram. Valid values: left, right, top, bottom", align_value),
                        span,
                        "invalid align value",
                        None,
                    ));
                }
            }
            Some(kind) => {
                self.errors.push(DiagnosticError::from_span(
                    format!(
                        "Unknown diagram type '{}'. Cannot validate align attribute",
                        kind
                    ),
                    span,
                    "unknown diagram type",
                    None,
                ));
            }
            None => {
                self.errors.push(DiagnosticError::from_span(
                    "Diagram type not set. Cannot validate align attribute".to_string(),
                    span,
                    "missing diagram type",
                    None,
                ));
            }
        }
    }
}

impl<'a> Visitor<'a> for Validator<'a> {
    fn visit_diagram(&mut self, diagram: &Diagram<'a>) {
        // Begin component registry for this diagram
        self.component_registry.push(HashMap::new());

        // Call default traversal
        self.visit_diagram_kind(&diagram.kind);
        self.visit_attributes(&diagram.attributes);
        self.visit_type_definitions(&diagram.type_definitions);
        self.visit_elements(&diagram.elements);

        // End component registry scope
        self.component_registry.pop();
    }

    fn visit_diagram_kind(&mut self, kind: &Spanned<&'a str>) {
        self.diagram_kind = Some(kind.inner());
    }

    fn visit_elements(&mut self, elements: &[Element<'a>]) {
        // Begin new activation scope
        self.activation_stack.push(HashMap::new());

        // Traverse elements
        for elem in elements {
            self.visit_element(elem);
        }

        // End scope
        // Validate any remaining unpaired activations in this scope
        if let Some(state) = self.activation_stack.pop() {
            for (component_id, spans) in state.iter() {
                if !spans.is_empty() {
                    let span = spans.last().cloned().unwrap_or_default();
                    self.errors.push(DiagnosticError::from_span(
                        format!(
                            "Component '{}' was activated but never deactivated",
                            component_id
                        ),
                        span,
                        "unpaired activate",
                        Some(
                            "Every activate statement must have a corresponding deactivate statement"
                                .to_string(),
                        ),
                    ));
                }
            }
        }
    }

    fn visit_component_name(&mut self, name: &Spanned<Id>) {
        // Register this component in the current diagram's registry
        let registry = self
            .component_registry
            .last_mut()
            .expect("component registry not initialized");
        registry.insert(*name.inner(), name.span());
    }

    fn visit_activate(&mut self, component: &Spanned<Id>, type_spec: &TypeSpec<'a>) {
        // Validate component identifier exists
        self.visit_identifier(component);

        self.visit_type_spec(type_spec);

        // Then handle activation stack logic
        let state = self.activation_state_mut();
        state
            .entry(*component.inner())
            .or_default()
            .push(component.span());
    }

    fn visit_deactivate(&mut self, component: &Spanned<Id>) {
        // Validate component identifier exists
        self.visit_identifier(component);

        // Then handle activation stack logic
        let state = self.activation_state_mut();
        match state.get_mut(component.inner()) {
            Some(spans) if !spans.is_empty() => {
                // Remove the most recent activation span (LIFO)
                let _ = spans.pop();
            }
            _ => {
                // No matching activate
                self.errors.push(DiagnosticError::from_span(
                    format!(
                        "Cannot deactivate component '{}': no matching activate statement",
                        component.inner()
                    ),
                    component.span(),
                    "unpaired deactivate",
                    Some(
                        "Deactivate statements must be preceded by a corresponding activate statement"
                            .to_string(),
                    ),
                ));
            }
        }
    }

    fn visit_note(&mut self, note: &Note<'a>) {
        self.visit_type_spec(&note.type_spec);

        // Validation for align attribute
        for attr in &note.type_spec.attributes {
            if *attr.name.inner() == "align"
                && let Ok(align_value) = attr.value.as_str()
            {
                self.validate_align_for_diagram_type(align_value, attr.value.span());
            }
        }

        // Visit content
        self.visit_note_content(&note.content);
    }

    fn visit_identifier(&mut self, identifier: &Spanned<Id>) {
        let registry = self
            .component_registry
            .last()
            .expect("component registry not initialized");

        if !registry.contains_key(identifier.inner()) {
            self.errors.push(DiagnosticError::from_span(
                format!("Component '{}' not found", identifier.inner()),
                identifier.span(),
                "undefined component",
                Some("Component must be defined before it can be referenced".to_string()),
            ));
        }
    }
}

/// Convenience function to run all diagram validations
///
/// Returns:
/// - Ok(()) when no validation issues are found
/// - Err(DiagnosticError) with the first collected error otherwise
pub fn validate_diagram(diagram: &Diagram<'_>) -> Result<()> {
    let mut validator = Validator::new();
    visit_diagram(&mut validator, diagram);
    // TODO: Support multi error.
    if let Some(err) = validator.errors.into_iter().next() {
        Err(err)
    } else {
        Ok(())
    }
}

/// Deprecated: Use validate_diagram instead
///
/// Convenience function to run activation validation for a diagram
///
/// Returns:
/// - Ok(()) when no activation pairing issues are found
/// - Err(DiagnosticError) with the first collected error otherwise
#[allow(dead_code)]
#[deprecated(since = "0.1.0", note = "Use validate_diagram instead")]
pub fn validate_activation_pairs(diagram: &Diagram<'_>) -> Result<()> {
    validate_diagram(diagram)
}

/// Deprecated: Use validate_diagram instead
///
/// Convenience function to run note validation for a diagram
///
/// Returns:
/// - Ok(()) when no note validation issues are found
/// - Err(DiagnosticError) with the first collected error otherwise
#[allow(dead_code)]
#[deprecated(since = "0.1.0", note = "Use validate_diagram instead")]
pub fn validate_notes(diagram: &Diagram<'_>) -> Result<()> {
    validate_diagram(diagram)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test visitor that counts different element types
    struct CountingVisitor {
        component_count: usize,
        relation_count: usize,
        activate_count: usize,
        deactivate_count: usize,
    }

    impl CountingVisitor {
        fn new() -> Self {
            Self {
                component_count: 0,
                relation_count: 0,
                activate_count: 0,
                deactivate_count: 0,
            }
        }
    }

    impl<'a> Visitor<'a> for CountingVisitor {
        fn visit_component(
            &mut self,
            name: &Spanned<Id>,
            display_name: &Option<Spanned<String>>,
            type_spec: &TypeSpec<'a>,
            nested_elements: &[Element<'a>],
        ) {
            self.component_count += 1;
            // Call default traversal
            self.visit_component_name(name);
            if let Some(dn) = display_name {
                self.visit_display_name(dn);
            }
            self.visit_type_spec(type_spec);
            self.visit_elements(nested_elements);
        }

        fn visit_relation(
            &mut self,
            source: &Spanned<Id>,
            target: &Spanned<Id>,
            relation_type: &Spanned<&'a str>,
            type_spec: &TypeSpec<'a>,
            label: &Option<Spanned<String>>,
        ) {
            self.relation_count += 1;
            // Call default traversal
            self.visit_relation_source(source);
            self.visit_relation_target(target);
            self.visit_relation_type(relation_type);
            self.visit_type_spec(type_spec);
            if let Some(l) = label {
                self.visit_relation_label(l);
            }
        }

        fn visit_activate(&mut self, component: &Spanned<Id>, type_spec: &TypeSpec<'a>) {
            self.visit_identifier(component);
            self.visit_type_spec(type_spec);
            self.activate_count += 1;
        }

        fn visit_deactivate(&mut self, component: &Spanned<Id>) {
            self.deactivate_count += 1;
            self.visit_identifier(component);
        }
    }

    #[test]
    fn test_visitor_traversal() {
        // Create a simple test diagram
        let diagram = Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("user"), Span::new(10..14)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(16..25))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Activate {
                    component: Spanned::new(Id::new("user"), Span::new(30..34)),
                    type_spec: TypeSpec::default(),
                },
                Element::Relation {
                    source: Spanned::new(Id::new("user"), Span::new(40..44)),
                    target: Spanned::new(Id::new("server"), Span::new(48..54)),
                    relation_type: Spanned::new("->", Span::new(45..47)),
                    type_spec: TypeSpec::default(),
                    label: None,
                },
                Element::Deactivate {
                    component: Spanned::new(Id::new("user"), Span::new(60..64)),
                },
            ],
        };

        let mut visitor = CountingVisitor::new();
        visit_diagram(&mut visitor, &diagram);

        assert_eq!(visitor.component_count, 1);
        assert_eq!(visitor.relation_count, 1);
        assert_eq!(visitor.activate_count, 1);
        assert_eq!(visitor.deactivate_count, 1);
    }

    #[test]
    fn test_validate_ok_pair() {
        let diagram = Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("user"), Span::new(0..4)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(6..15))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Activate {
                    component: Spanned::new(Id::new("user"), Span::new(17..21)),
                    type_spec: TypeSpec::default(),
                },
                Element::Deactivate {
                    component: Spanned::new(Id::new("user"), Span::new(23..27)),
                },
            ],
        };

        let result = super::validate_diagram(&diagram);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_unpaired_deactivate() {
        let diagram = Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![Element::Deactivate {
                component: Spanned::new(Id::new("user"), Span::new(0..4)),
            }],
        };

        let result = super::validate_diagram(&diagram);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_unpaired_activate_end_of_scope() {
        let diagram = Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![Element::Activate {
                component: Spanned::new(Id::new("user"), Span::new(0..4)),
                type_spec: TypeSpec::default(),
            }],
        };

        let result = super::validate_diagram(&diagram);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_nested_activations_ok() {
        let diagram = Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("user"), Span::new(0..4)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(6..15))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Activate {
                    component: Spanned::new(Id::new("user"), Span::new(17..21)),
                    type_spec: TypeSpec::default(),
                },
                Element::Activate {
                    component: Spanned::new(Id::new("user"), Span::new(23..27)),
                    type_spec: TypeSpec::default(),
                },
                Element::Deactivate {
                    component: Spanned::new(Id::new("user"), Span::new(29..33)),
                },
                Element::Deactivate {
                    component: Spanned::new(Id::new("user"), Span::new(35..39)),
                },
            ],
        };

        let result = super::validate_diagram(&diagram);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_interleaved_components_ok() {
        let diagram = Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("user"), Span::new(0..4)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(6..15))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Component {
                    name: Spanned::new(Id::new("server"), Span::new(17..23)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(25..34))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Activate {
                    component: Spanned::new(Id::new("user"), Span::new(36..40)),
                    type_spec: TypeSpec::default(),
                },
                Element::Activate {
                    component: Spanned::new(Id::new("server"), Span::new(42..48)),
                    type_spec: TypeSpec::default(),
                },
                Element::Deactivate {
                    component: Spanned::new(Id::new("user"), Span::new(50..54)),
                },
                Element::Deactivate {
                    component: Spanned::new(Id::new("server"), Span::new(56..62)),
                },
            ],
        };

        let result = super::validate_diagram(&diagram);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_out_of_order_deactivate_first() {
        let diagram = Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Deactivate {
                    component: Spanned::new(Id::new("user"), Span::new(0..4)),
                },
                Element::Activate {
                    component: Spanned::new(Id::new("user"), Span::new(5..9)),
                    type_spec: TypeSpec::default(),
                },
            ],
        };

        let result = super::validate_diagram(&diagram);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod note_validation_tests {
    use super::*;
    use crate::ast::{lexer::tokenize, parser::build_diagram};

    #[test]
    fn test_valid_note_sequence_diagram() {
        let input = r#"
        diagram sequence;
        client: Rectangle;
        server: Rectangle;

        note [on=[client], align="left"]: "Valid note";
        note [on=[server], align="right"]: "Another valid note";
        note [align="over"]: "Margin note";
        "#;

        let tokens = tokenize(input).expect("Failed to tokenize");
        let element = build_diagram(&tokens).expect("Failed to parse");
        if let Element::Diagram(diagram) = element.inner() {
            let result = validate_diagram(diagram);
            assert!(result.is_ok(), "Valid notes should pass validation");
        } else {
            panic!("Expected Diagram element");
        }
    }

    #[test]
    fn test_valid_note_component_diagram() {
        let input = r#"
        diagram component;
        api: Rectangle;
        db: Rectangle;

        note [on=[api], align="top"]: "Valid note";
        note [on=[db], align="bottom"]: "Another valid note";
        note [align="left"]: "Margin note";
        "#;

        let tokens = tokenize(input).expect("Failed to tokenize");
        let element = build_diagram(&tokens).expect("Failed to parse");
        if let Element::Diagram(diagram) = element.inner() {
            let result = validate_diagram(diagram);
            assert!(result.is_ok(), "Valid notes should pass validation");
        } else {
            panic!("Expected Diagram element");
        }
    }

    #[test]
    fn test_invalid_align_sequence_diagram() {
        let input = r#"
        diagram sequence;
        client: Rectangle;

        note [on=[client], align="top"]: "Invalid align for sequence";
        "#;

        let tokens = tokenize(input).expect("Failed to tokenize");
        let element = build_diagram(&tokens).expect("Failed to parse");
        if let Element::Diagram(diagram) = element.inner() {
            let result = validate_diagram(diagram);
            assert!(result.is_err(), "Invalid align should fail validation");

            let err = result.unwrap_err();
            assert!(format!("{}", err).contains("Invalid align value 'top' for sequence diagram"));
        } else {
            panic!("Expected Diagram element");
        }
    }

    #[test]
    fn test_invalid_align_component_diagram() {
        let input = r#"
        diagram component;
        api: Rectangle;

        note [on=[api], align="over"]: "Invalid align for component";
        "#;

        let tokens = tokenize(input).expect("Failed to tokenize");
        let element = build_diagram(&tokens).expect("Failed to parse");
        if let Element::Diagram(diagram) = element.inner() {
            let result = validate_diagram(diagram);
            assert!(result.is_err(), "Invalid align should fail validation");

            let err = result.unwrap_err();
            assert!(
                format!("{}", err).contains("Invalid align value 'over' for component diagram")
            );
        } else {
            panic!("Expected Diagram element");
        }
    }

    #[test]
    fn test_multiple_component_references() {
        let input = r#"
        diagram sequence;
        client: Rectangle;
        server: Rectangle;

        note [on=[client, server]]: "Valid spanning note";
        "#;

        let tokens = tokenize(input).expect("Failed to tokenize");
        let element = build_diagram(&tokens).expect("Failed to parse");
        if let Element::Diagram(diagram) = element.inner() {
            let result = validate_diagram(diagram);
            assert!(result.is_ok(), "Valid spanning note should pass validation");
        } else {
            panic!("Expected Diagram element");
        }
    }

    #[test]
    fn test_empty_on_attribute() {
        let input = r#"
        diagram sequence;
        client: Rectangle;

        note [on=[]]: "Margin note with empty on";
        "#;

        let tokens = tokenize(input).expect("Failed to tokenize");
        let element = build_diagram(&tokens).expect("Failed to parse");
        if let Element::Diagram(diagram) = element.inner() {
            let result = validate_diagram(diagram);
            assert!(
                result.is_ok(),
                "Empty on attribute should be valid (margin note)"
            );
        } else {
            panic!("Expected Diagram element");
        }
    }
}

#[cfg(test)]
mod identifier_validation_tests {
    use super::*;

    #[test]
    fn test_component_registry_fully_qualified_access() {
        // Test that fully qualified identifiers from nested components
        // are all accessible in a single diagram-level registry
        let diagram = Diagram {
            kind: Spanned::new("component", Span::new(0..9)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("frontend"), Span::new(0..8)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(10..19))),
                        attributes: vec![],
                    },
                    nested_elements: vec![
                        Element::Component {
                            name: Spanned::new(Id::new("frontend::app"), Span::new(20..33)),
                            display_name: None,
                            type_spec: TypeSpec {
                                type_name: Some(Spanned::new(
                                    Id::new("Rectangle"),
                                    Span::new(35..44),
                                )),
                                attributes: vec![],
                            },
                            nested_elements: vec![],
                        },
                        Element::Component {
                            name: Spanned::new(Id::new("frontend::ui"), Span::new(45..57)),
                            display_name: None,
                            type_spec: TypeSpec {
                                type_name: Some(Spanned::new(
                                    Id::new("Rectangle"),
                                    Span::new(59..68),
                                )),
                                attributes: vec![],
                            },
                            nested_elements: vec![],
                        },
                    ],
                },
                Element::Component {
                    name: Spanned::new(Id::new("backend"), Span::new(69..76)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(78..87))),
                        attributes: vec![],
                    },
                    nested_elements: vec![Element::Component {
                        name: Spanned::new(Id::new("backend::api"), Span::new(88..100)),
                        display_name: None,
                        type_spec: TypeSpec {
                            type_name: Some(Spanned::new(
                                Id::new("Rectangle"),
                                Span::new(102..111),
                            )),
                            attributes: vec![],
                        },
                        nested_elements: vec![],
                    }],
                },
            ],
        };

        let mut validator = Validator::new();
        visit_diagram(&mut validator, &diagram);

        // All components should be registered at diagram level with fully qualified names
        // This includes: frontend, frontend::app, frontend::ui, backend, backend::api
        assert!(validator.errors.is_empty());
    }

    #[test]
    fn test_visit_identifier_not_found() {
        let mut validator = Validator::new();

        // Set up registry with a component
        validator.component_registry.push(
            vec![(Id::new("app"), Span::new(0..3))]
                .into_iter()
                .collect(),
        );

        // Test visit_identifier with a non-existent component
        validator.visit_identifier(&Spanned::new(Id::new("unknown"), Span::new(10..17)));

        // Should have an error
        assert_eq!(validator.errors.len(), 1);
        assert!(
            validator.errors[0]
                .to_string()
                .contains("Component 'unknown' not found")
        );
    }

    #[test]
    fn test_visit_identifiers_multiple() {
        let mut validator = Validator::new();

        // Set up registry with multiple components
        validator.component_registry.push(
            vec![
                (Id::new("client"), Span::new(0..6)),
                (Id::new("server"), Span::new(18..24)),
            ]
            .into_iter()
            .collect(),
        );

        // Test visit_identifier with multiple components
        validator.visit_identifier(&Spanned::new(Id::new("client"), Span::new(40..46)));
        validator.visit_identifier(&Spanned::new(Id::new("server"), Span::new(48..54)));

        // Should not add any errors
        assert!(validator.errors.is_empty());
    }

    #[test]
    fn test_visit_identifiers_some_missing() {
        let mut validator = Validator::new();

        // Set up registry with one component
        validator.component_registry.push(
            vec![(Id::new("client"), Span::new(0..6))]
                .into_iter()
                .collect(),
        );

        // Test visit_identifier with one valid and one invalid component
        validator.visit_identifier(&Spanned::new(Id::new("client"), Span::new(40..46)));
        validator.visit_identifier(&Spanned::new(Id::new("unknown"), Span::new(48..55)));

        // Should have one error for the missing component
        assert_eq!(validator.errors.len(), 1);
        assert!(
            validator.errors[0]
                .to_string()
                .contains("Component 'unknown' not found")
        );
    }

    #[test]
    fn test_relation_with_valid_components() {
        let diagram = Diagram {
            kind: Spanned::new("component", Span::new(0..9)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("app"), Span::new(0..3)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(5..14))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Component {
                    name: Spanned::new(Id::new("db"), Span::new(15..17)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(19..28))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Relation {
                    source: Spanned::new(Id::new("app"), Span::new(30..33)),
                    target: Spanned::new(Id::new("db"), Span::new(37..39)),
                    relation_type: Spanned::new("->", Span::new(34..36)),
                    type_spec: TypeSpec::default(),
                    label: None,
                },
            ],
        };

        let result = validate_diagram(&diagram);
        assert!(result.is_ok(), "Valid relation should pass validation");
    }

    #[test]
    fn test_relation_with_invalid_source() {
        let diagram = Diagram {
            kind: Spanned::new("component", Span::new(0..9)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("db"), Span::new(15..17)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(19..28))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Relation {
                    source: Spanned::new(Id::new("unknown"), Span::new(30..37)),
                    target: Spanned::new(Id::new("db"), Span::new(41..43)),
                    relation_type: Spanned::new("->", Span::new(38..40)),
                    type_spec: TypeSpec::default(),
                    label: None,
                },
            ],
        };

        let result = validate_diagram(&diagram);
        assert!(result.is_err(), "Invalid source should fail validation");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Component 'unknown' not found"));
    }

    #[test]
    fn test_relation_with_invalid_target() {
        let diagram = Diagram {
            kind: Spanned::new("component", Span::new(0..9)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("app"), Span::new(0..3)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(5..14))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Relation {
                    source: Spanned::new(Id::new("app"), Span::new(30..33)),
                    target: Spanned::new(Id::new("missing"), Span::new(37..44)),
                    relation_type: Spanned::new("->", Span::new(34..36)),
                    type_spec: TypeSpec::default(),
                    label: None,
                },
            ],
        };

        let result = validate_diagram(&diagram);
        assert!(result.is_err(), "Invalid target should fail validation");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Component 'missing' not found"));
    }

    #[test]
    fn test_activate_with_valid_component() {
        let diagram = Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("server"), Span::new(0..6)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(8..17))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Activate {
                    component: Spanned::new(Id::new("server"), Span::new(20..26)),
                    type_spec: TypeSpec::default(),
                },
                Element::Deactivate {
                    component: Spanned::new(Id::new("server"), Span::new(30..36)),
                },
            ],
        };

        let result = validate_diagram(&diagram);
        assert!(result.is_ok(), "Valid activate should pass validation");
    }

    #[test]
    fn test_activate_with_invalid_component() {
        let diagram = Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("server"), Span::new(0..6)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(8..17))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Activate {
                    component: Spanned::new(Id::new("unknown"), Span::new(20..27)),
                    type_spec: TypeSpec::default(),
                },
            ],
        };

        let result = validate_diagram(&diagram);
        assert!(result.is_err(), "Invalid activate should fail validation");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Component 'unknown' not found"));
    }

    #[test]
    fn test_deactivate_with_invalid_component() {
        let diagram = Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("server"), Span::new(0..6)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(8..17))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Deactivate {
                    component: Spanned::new(Id::new("missing"), Span::new(20..27)),
                },
            ],
        };

        let result = validate_diagram(&diagram);
        assert!(result.is_err(), "Invalid deactivate should fail validation");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Component 'missing' not found"));
    }

    #[test]
    fn test_note_with_invalid_component() {
        let diagram = Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("client"), Span::new(0..6)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(8..17))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Note(Note {
                    type_spec: TypeSpec {
                        type_name: None,
                        attributes: vec![Attribute {
                            name: Spanned::new("on", Span::new(20..22)),
                            value: AttributeValue::Identifiers(vec![Spanned::new(
                                Id::new("unknown"),
                                Span::new(24..31),
                            )]),
                        }],
                    },
                    content: Spanned::new("Invalid note".to_string(), Span::new(33..47)),
                }),
            ],
        };

        let result = validate_diagram(&diagram);
        assert!(result.is_err(), "Note with invalid component should fail");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Component 'unknown' not found"));
    }

    #[test]
    fn test_note_with_multiple_components() {
        let diagram = Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("client"), Span::new(0..6)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(8..17))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Component {
                    name: Spanned::new(Id::new("server"), Span::new(19..25)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(27..36))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Note(Note {
                    type_spec: TypeSpec {
                        type_name: None,
                        attributes: vec![Attribute {
                            name: Spanned::new("on", Span::new(38..40)),
                            value: AttributeValue::Identifiers(vec![
                                Spanned::new(Id::new("client"), Span::new(42..48)),
                                Spanned::new(Id::new("server"), Span::new(50..56)),
                            ]),
                        }],
                    },
                    content: Spanned::new("Multi-component note".to_string(), Span::new(58..78)),
                }),
            ],
        };

        let result = validate_diagram(&diagram);
        assert!(
            result.is_ok(),
            "Note with multiple valid components should pass"
        );
    }

    #[test]
    fn test_note_with_empty_on_attribute() {
        let diagram = Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("client"), Span::new(0..6)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(8..17))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Note(Note {
                    type_spec: TypeSpec {
                        type_name: None,
                        attributes: vec![Attribute {
                            name: Spanned::new("on", Span::new(20..22)),
                            value: AttributeValue::Identifiers(vec![]),
                        }],
                    },
                    content: Spanned::new("Margin note".to_string(), Span::new(26..39)),
                }),
            ],
        };

        let result = validate_diagram(&diagram);
        assert!(result.is_ok(), "Note with empty on attribute should pass");
    }

    #[test]
    fn test_validation_with_typespec() {
        let diagram = Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![TypeDefinition {
                name: Spanned::new(Id::new("CustomArrow"), Span::new(10..21)),
                type_spec: TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Arrow"), Span::new(24..29))),
                    attributes: vec![Attribute {
                        name: Spanned::new("color", Span::new(30..35)),
                        value: AttributeValue::String(Spanned::new(
                            "red".to_string(),
                            Span::new(37..42),
                        )),
                    }],
                },
            }],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("client"), Span::new(50..56)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(58..67))),
                        attributes: vec![Attribute {
                            name: Spanned::new("fill", Span::new(68..72)),
                            value: AttributeValue::String(Spanned::new(
                                "blue".to_string(),
                                Span::new(74..80),
                            )),
                        }],
                    },
                    nested_elements: vec![],
                },
                Element::Component {
                    name: Spanned::new(Id::new("server"), Span::new(85..91)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(93..102))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Activate {
                    component: Spanned::new(Id::new("server"), Span::new(110..116)),
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Activation"), Span::new(118..128))),
                        attributes: vec![Attribute {
                            name: Spanned::new("fill", Span::new(129..133)),
                            value: AttributeValue::String(Spanned::new(
                                "yellow".to_string(),
                                Span::new(135..143),
                            )),
                        }],
                    },
                },
                Element::Relation {
                    source: Spanned::new(Id::new("client"), Span::new(150..156)),
                    target: Spanned::new(Id::new("server"), Span::new(160..166)),
                    relation_type: Spanned::new("->", Span::new(157..159)),
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("CustomArrow"), Span::new(168..179))),
                        attributes: vec![Attribute {
                            name: Spanned::new("width", Span::new(180..185)),
                            value: AttributeValue::Float(Spanned::new(2.0, Span::new(187..188))),
                        }],
                    },
                    label: Some(Spanned::new("request".to_string(), Span::new(190..199))),
                },
                Element::Note(Note {
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Note"), Span::new(205..209))),
                        attributes: vec![
                            Attribute {
                                name: Spanned::new("on", Span::new(210..212)),
                                value: AttributeValue::Identifiers(vec![
                                    Spanned::new(Id::new("client"), Span::new(214..220)),
                                    Spanned::new(Id::new("server"), Span::new(222..228)),
                                ]),
                            },
                            Attribute {
                                name: Spanned::new("align", Span::new(230..235)),
                                value: AttributeValue::String(Spanned::new(
                                    "right".to_string(),
                                    Span::new(237..245),
                                )),
                            },
                        ],
                    },
                    content: Spanned::new("Processing".to_string(), Span::new(247..258)),
                }),
                Element::Deactivate {
                    component: Spanned::new(Id::new("server"), Span::new(265..271)),
                },
            ],
        };

        let result = validate_diagram(&diagram);
        assert!(
            result.is_ok(),
            "Diagram with comprehensive TypeSpec usage should pass validation"
        );
    }
}
