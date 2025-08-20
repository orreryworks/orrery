//! Validation module for AST elements using the visitor pattern
//!
//! This module implements a visitor-based (read-only) traversal system for AST validation.
//! It sits between the desugar and elaboration phases, allowing for semantic
//! validation of the AST before elaboration.

use super::{
    span::Span,
    {
        parser_types::{
            Attribute, AttributeValue, Diagram, Element, Fragment, FragmentSection,
            RelationTypeSpec, TypeDefinition,
        },
        span::Spanned,
    },
};
use crate::{error::ElaborationDiagnosticError, identifier::Id};
use std::collections::HashMap;

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
        }
    }

    /// Visit a string attribute value
    fn visit_string_value(&mut self, _value: &Spanned<String>) {}

    /// Visit a float attribute value
    fn visit_float_value(&mut self, _value: &Spanned<f32>) {}

    /// Visit a list of type definitions
    fn visit_type_definitions(&mut self, type_definitions: &[TypeDefinition<'a>]) {
        for td in type_definitions {
            self.visit_type_definition(td);
        }
    }

    /// Visit a single type definition
    fn visit_type_definition(&mut self, type_def: &TypeDefinition<'a>) {
        self.visit_type_name(&type_def.name);
        self.visit_base_type(&type_def.base_type);
        self.visit_attributes(&type_def.attributes);
    }

    /// Visit a type name
    fn visit_type_name(&mut self, _name: &Spanned<&'a str>) {}

    /// Visit a base type
    fn visit_base_type(&mut self, _base_type: &Spanned<&'a str>) {}

    /// Visit a list of elements
    fn visit_elements(&mut self, elements: &[Element<'a>]) {
        for elem in elements {
            self.visit_element(elem);
        }
    }

    /// Visit a single element
    fn visit_element(&mut self, element: &Element<'a>) {
        match element {
            Element::Component {
                name,
                display_name,
                type_name,
                attributes,
                nested_elements,
            } => self.visit_component(name, display_name, type_name, attributes, nested_elements),
            Element::Relation {
                source,
                target,
                relation_type,
                type_spec,
                label,
            } => self.visit_relation(source, target, relation_type, type_spec, label),
            Element::Diagram(diagram) => self.visit_diagram(diagram),
            Element::ActivateBlock {
                component,
                elements,
            } => self.visit_activate_block(component, elements),
            Element::Activate { component } => self.visit_activate(component),
            Element::Deactivate { component } => self.visit_deactivate(component),
            Element::Fragment(fragment) => self.visit_fragment(fragment),
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
        name: &Spanned<&'a str>,
        display_name: &Option<Spanned<String>>,
        type_name: &Spanned<&'a str>,
        attributes: &[Attribute<'a>],
        nested_elements: &[Element<'a>],
    ) {
        self.visit_component_name(name);
        if let Some(dn) = display_name {
            self.visit_display_name(dn);
        }
        self.visit_component_type(type_name);
        self.visit_attributes(attributes);
        self.visit_elements(nested_elements);
    }

    /// Visit a component name
    fn visit_component_name(&mut self, _name: &Spanned<&'a str>) {}

    /// Visit a display name
    fn visit_display_name(&mut self, _display_name: &Spanned<String>) {}

    /// Visit a component type
    fn visit_component_type(&mut self, _type_name: &Spanned<&'a str>) {}

    /// Visit a relation element
    fn visit_relation(
        &mut self,
        source: &Spanned<String>,
        target: &Spanned<String>,
        relation_type: &Spanned<&'a str>,
        type_spec: &Option<RelationTypeSpec<'a>>,
        label: &Option<Spanned<String>>,
    ) {
        self.visit_relation_source(source);
        self.visit_relation_target(target);
        self.visit_relation_type(relation_type);
        if let Some(ts) = type_spec {
            self.visit_relation_type_spec(ts);
        }
        if let Some(l) = label {
            self.visit_relation_label(l);
        }
    }

    /// Visit a relation source
    fn visit_relation_source(&mut self, _source: &Spanned<String>) {}

    /// Visit a relation target
    fn visit_relation_target(&mut self, _target: &Spanned<String>) {}

    /// Visit a relation type
    fn visit_relation_type(&mut self, _relation_type: &Spanned<&'a str>) {}

    /// Visit a relation type specification
    fn visit_relation_type_spec(&mut self, type_spec: &RelationTypeSpec<'a>) {
        if let Some(tn) = &type_spec.type_name {
            self.visit_relation_type_name(tn);
        }
        self.visit_attributes(&type_spec.attributes);
    }

    /// Visit a relation type name
    fn visit_relation_type_name(&mut self, _type_name: &Spanned<&'a str>) {}

    /// Visit a relation label
    fn visit_relation_label(&mut self, _label: &Spanned<String>) {}

    /// Visit an activate block element
    fn visit_activate_block(&mut self, component: &Spanned<String>, elements: &[Element<'a>]) {
        self.visit_activate_component(component);
        self.visit_elements(elements);
    }

    /// Visit an activate block component reference
    fn visit_activate_component(&mut self, _component: &Spanned<String>) {}

    /// Visit an activate statement
    fn visit_activate(&mut self, _component: &Spanned<String>) {}

    /// Visit a deactivate statement
    fn visit_deactivate(&mut self, _component: &Spanned<String>) {}
}

/// Entry point for running a visitor on a diagram
pub fn visit_diagram<'a, V: Visitor<'a>>(visitor: &mut V, diagram: &Diagram<'a>) {
    visitor.visit_diagram(diagram)
}

/// Validator that checks activate/deactivate pairing in sequence diagrams
///
/// Uses a visitor-based traversal to:
/// - Track activation state per component (nesting depth and spans)
/// - Validate ordering (deactivate must have a matching prior activate)
/// - Reset state per sequence diagram scope (handles embedded diagrams)
/// - Collect errors during traversal for reporting after traversal
pub struct ActivationValidator {
    activation_stack: Vec<HashMap<Id, Vec<Span>>>,
    errors: Vec<ElaborationDiagnosticError>,
}

impl ActivationValidator {
    pub fn new() -> Self {
        Self {
            activation_stack: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn current_state_mut(&mut self) -> &mut HashMap<Id, Vec<Span>> {
        self.activation_stack
            .last_mut()
            .expect("activation scope not initialized")
    }
}

impl<'a> Visitor<'a> for ActivationValidator {
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
                    self.errors.push(ElaborationDiagnosticError::from_span(
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

    fn visit_activate(&mut self, component: &Spanned<String>) {
        let id = Id::new(component.inner());
        let state = self.current_state_mut();
        state.entry(id).or_default().push(component.span());
    }

    fn visit_deactivate(&mut self, component: &Spanned<String>) {
        let id = Id::new(component.inner());
        let state = self.current_state_mut();
        match state.get_mut(&id) {
            Some(spans) if !spans.is_empty() => {
                // Remove the most recent activation span (LIFO)
                let _ = spans.pop();
            }
            _ => {
                // No matching activate
                self.errors.push(ElaborationDiagnosticError::from_span(
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
}

/// Convenience function to run activation validation for a diagram
///
/// Returns:
/// - Ok(()) when no activation pairing issues are found
/// - Err(ElaborationDiagnosticError) with the first collected error otherwise
pub fn validate_activation_pairs(diagram: &Diagram<'_>) -> Result<(), ElaborationDiagnosticError> {
    let mut validator = ActivationValidator::new();
    visit_diagram(&mut validator, diagram);
    // TODO: Support multi error.
    if let Some(err) = validator.errors.into_iter().next() {
        Err(err)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::span::Span;
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
            name: &Spanned<&'a str>,
            display_name: &Option<Spanned<String>>,
            type_name: &Spanned<&'a str>,
            attributes: &[Attribute<'a>],
            nested_elements: &[Element<'a>],
        ) {
            self.component_count += 1;
            // Call default traversal
            self.visit_component_name(name);
            if let Some(dn) = display_name {
                self.visit_display_name(dn);
            }
            self.visit_component_type(type_name);
            self.visit_attributes(attributes);
            self.visit_elements(nested_elements);
        }

        fn visit_relation(
            &mut self,
            source: &Spanned<String>,
            target: &Spanned<String>,
            relation_type: &Spanned<&'a str>,
            type_spec: &Option<RelationTypeSpec<'a>>,
            label: &Option<Spanned<String>>,
        ) {
            self.relation_count += 1;
            // Call default traversal
            self.visit_relation_source(source);
            self.visit_relation_target(target);
            self.visit_relation_type(relation_type);
            if let Some(ts) = type_spec {
                self.visit_relation_type_spec(ts);
            }
            if let Some(l) = label {
                self.visit_relation_label(l);
            }
        }

        fn visit_activate(&mut self, _component: &Spanned<String>) {
            self.activate_count += 1;
        }

        fn visit_deactivate(&mut self, _component: &Spanned<String>) {
            self.deactivate_count += 1;
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
                    name: Spanned::new("user", Span::new(10..14)),
                    display_name: None,
                    type_name: Spanned::new("Rectangle", Span::new(16..25)),
                    attributes: vec![],
                    nested_elements: vec![],
                },
                Element::Activate {
                    component: Spanned::new("user".to_string(), Span::new(30..34)),
                },
                Element::Relation {
                    source: Spanned::new("user".to_string(), Span::new(40..44)),
                    target: Spanned::new("server".to_string(), Span::new(48..54)),
                    relation_type: Spanned::new("->", Span::new(45..47)),
                    type_spec: None,
                    label: None,
                },
                Element::Deactivate {
                    component: Spanned::new("user".to_string(), Span::new(60..64)),
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
                Element::Activate {
                    component: Spanned::new("user".to_string(), Span::new(0..4)),
                },
                Element::Deactivate {
                    component: Spanned::new("user".to_string(), Span::new(5..9)),
                },
            ],
        };

        let result = super::validate_activation_pairs(&diagram);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_unpaired_deactivate() {
        let diagram = Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![Element::Deactivate {
                component: Spanned::new("user".to_string(), Span::new(0..4)),
            }],
        };

        let result = super::validate_activation_pairs(&diagram);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_unpaired_activate_end_of_scope() {
        let diagram = Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![Element::Activate {
                component: Spanned::new("user".to_string(), Span::new(0..4)),
            }],
        };

        let result = super::validate_activation_pairs(&diagram);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_nested_activations_ok() {
        let diagram = Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Activate {
                    component: Spanned::new("user".to_string(), Span::new(0..4)),
                },
                Element::Activate {
                    component: Spanned::new("user".to_string(), Span::new(5..9)),
                },
                Element::Deactivate {
                    component: Spanned::new("user".to_string(), Span::new(10..14)),
                },
                Element::Deactivate {
                    component: Spanned::new("user".to_string(), Span::new(15..19)),
                },
            ],
        };

        let result = super::validate_activation_pairs(&diagram);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_interleaved_components_ok() {
        let diagram = Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Activate {
                    component: Spanned::new("user".to_string(), Span::new(0..4)),
                },
                Element::Activate {
                    component: Spanned::new("server".to_string(), Span::new(5..11)),
                },
                Element::Deactivate {
                    component: Spanned::new("user".to_string(), Span::new(12..16)),
                },
                Element::Deactivate {
                    component: Spanned::new("server".to_string(), Span::new(17..23)),
                },
            ],
        };

        let result = super::validate_activation_pairs(&diagram);
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
                    component: Spanned::new("user".to_string(), Span::new(0..4)),
                },
                Element::Activate {
                    component: Spanned::new("user".to_string(), Span::new(5..9)),
                },
            ],
        };

        let result = super::validate_activation_pairs(&diagram);
        assert!(result.is_err());
    }
}
