//! Desugaring pass for the Filament AST
//!
//! This module implements a fold-based rewriting system for AST transformations.
//! It sits between the parser and elaboration phases, allowing for syntactic
//! desugaring and AST normalization.
//!
//! The design follows the Fold (Catamorphism) pattern, similar to the Rust
//! compiler's folder, where each AST node type has a corresponding fold method
//! that consumes the node and produces a transformed version.

use super::{
    parser_types::{
        Attribute, AttributeValue, Diagram, Element, Fragment, FragmentSection, RelationTypeSpec,
        TypeDefinition,
    },
    span::Spanned,
};

/// The main trait for folding/rewriting AST nodes.
///
/// Each method takes ownership of its input and returns a transformed version.
/// The default implementations preserve the structure unchanged (identity transformation).
trait Folder<'a> {
    /// Fold a complete diagram
    fn fold_diagram(&mut self, diagram: Diagram<'a>) -> Diagram<'a> {
        Diagram {
            kind: self.fold_diagram_kind(diagram.kind),
            attributes: self.fold_attributes(diagram.attributes),
            type_definitions: self.fold_type_definitions(diagram.type_definitions),
            elements: self.fold_elements(diagram.elements),
        }
    }

    /// Fold the diagram kind (component, sequence, etc.)
    fn fold_diagram_kind(&mut self, kind: Spanned<&'a str>) -> Spanned<&'a str> {
        kind
    }

    /// Fold a list of attributes
    fn fold_attributes(&mut self, attributes: Vec<Attribute<'a>>) -> Vec<Attribute<'a>> {
        attributes
            .into_iter()
            .map(|attr| self.fold_attribute(attr))
            .collect()
    }

    /// Fold a single attribute
    fn fold_attribute(&mut self, attribute: Attribute<'a>) -> Attribute<'a> {
        Attribute {
            name: self.fold_attribute_name(attribute.name),
            value: self.fold_attribute_value(attribute.value),
        }
    }

    /// Fold an attribute name
    fn fold_attribute_name(&mut self, name: Spanned<&'a str>) -> Spanned<&'a str> {
        name
    }

    /// Fold an attribute value
    fn fold_attribute_value(&mut self, value: AttributeValue<'a>) -> AttributeValue<'a> {
        match value {
            AttributeValue::String(s) => AttributeValue::String(self.fold_string_value(s)),
            AttributeValue::Float(f) => AttributeValue::Float(self.fold_float_value(f)),
            AttributeValue::Attributes(attrs) => {
                AttributeValue::Attributes(self.fold_attributes(attrs))
            }
        }
    }

    /// Fold a string attribute value
    fn fold_string_value(&mut self, value: Spanned<String>) -> Spanned<String> {
        value
    }

    /// Fold a float attribute value
    fn fold_float_value(&mut self, value: Spanned<f32>) -> Spanned<f32> {
        value
    }

    /// Fold a list of type definitions
    fn fold_type_definitions(
        &mut self,
        type_definitions: Vec<TypeDefinition<'a>>,
    ) -> Vec<TypeDefinition<'a>> {
        type_definitions
            .into_iter()
            .map(|td| self.fold_type_definition(td))
            .collect()
    }

    /// Fold a single type definition
    fn fold_type_definition(&mut self, type_def: TypeDefinition<'a>) -> TypeDefinition<'a> {
        TypeDefinition {
            name: self.fold_type_name(type_def.name),
            base_type: self.fold_base_type(type_def.base_type),
            attributes: self.fold_attributes(type_def.attributes),
        }
    }

    /// Fold a type name
    fn fold_type_name(&mut self, name: Spanned<&'a str>) -> Spanned<&'a str> {
        name
    }

    /// Fold a base type
    fn fold_base_type(&mut self, base_type: Spanned<&'a str>) -> Spanned<&'a str> {
        base_type
    }

    /// Fold a list of elements
    fn fold_elements(&mut self, elements: Vec<Element<'a>>) -> Vec<Element<'a>> {
        elements
            .into_iter()
            .map(|elem| self.fold_element(elem))
            .collect()
    }

    /// Fold a single element
    fn fold_element(&mut self, element: Element<'a>) -> Element<'a> {
        match element {
            Element::Component {
                name,
                display_name,
                type_name,
                attributes,
                nested_elements,
            } => self.fold_component(name, display_name, type_name, attributes, nested_elements),
            Element::Relation {
                source,
                target,
                relation_type,
                type_spec,
                label,
            } => self.fold_relation(source, target, relation_type, type_spec, label),
            Element::Diagram(diagram) => Element::Diagram(self.fold_diagram(diagram)),
            Element::ActivateBlock {
                component,
                elements,
            } => self.fold_activate_block(component, elements),
            Element::Activate { component } => Element::Activate { component },
            Element::Deactivate { component } => Element::Deactivate { component },
            Element::Fragment(fragment) => Element::Fragment(self.fold_fragment(fragment)),
        }
    }

    /// Fold a component element
    fn fold_component(
        &mut self,
        name: Spanned<&'a str>,
        display_name: Option<Spanned<String>>,
        type_name: Spanned<&'a str>,
        attributes: Vec<Attribute<'a>>,
        nested_elements: Vec<Element<'a>>,
    ) -> Element<'a> {
        Element::Component {
            name: self.fold_component_name(name),
            display_name: display_name.map(|dn| self.fold_display_name(dn)),
            type_name: self.fold_component_type(type_name),
            attributes: self.fold_attributes(attributes),
            nested_elements: self.fold_elements(nested_elements),
        }
    }

    /// Fold a component name
    fn fold_component_name(&mut self, name: Spanned<&'a str>) -> Spanned<&'a str> {
        name
    }

    /// Fold a display name
    fn fold_display_name(&mut self, display_name: Spanned<String>) -> Spanned<String> {
        display_name
    }

    /// Fold a component type
    fn fold_component_type(&mut self, type_name: Spanned<&'a str>) -> Spanned<&'a str> {
        type_name
    }

    /// Fold a relation element
    fn fold_relation(
        &mut self,
        source: Spanned<String>,
        target: Spanned<String>,
        relation_type: Spanned<&'a str>,
        type_spec: Option<RelationTypeSpec<'a>>,
        label: Option<Spanned<String>>,
    ) -> Element<'a> {
        Element::Relation {
            source: self.fold_relation_source(source),
            target: self.fold_relation_target(target),
            relation_type: self.fold_relation_type(relation_type),
            type_spec: type_spec.map(|ts| self.fold_relation_type_spec(ts)),
            label: label.map(|l| self.fold_relation_label(l)),
        }
    }

    /// Fold a relation source
    fn fold_relation_source(&mut self, source: Spanned<String>) -> Spanned<String> {
        source
    }

    /// Fold a relation target
    fn fold_relation_target(&mut self, target: Spanned<String>) -> Spanned<String> {
        target
    }

    /// Fold a relation type
    fn fold_relation_type(&mut self, relation_type: Spanned<&'a str>) -> Spanned<&'a str> {
        relation_type
    }

    /// Fold a relation type specification
    fn fold_relation_type_spec(&mut self, type_spec: RelationTypeSpec<'a>) -> RelationTypeSpec<'a> {
        RelationTypeSpec {
            type_name: type_spec
                .type_name
                .map(|tn| self.fold_relation_type_name(tn)),
            attributes: self.fold_attributes(type_spec.attributes),
        }
    }

    /// Fold a relation type name
    fn fold_relation_type_name(&mut self, type_name: Spanned<&'a str>) -> Spanned<&'a str> {
        type_name
    }

    /// Fold a relation label
    fn fold_relation_label(&mut self, label: Spanned<String>) -> Spanned<String> {
        label
    }

    /// Fold an activate block element
    fn fold_activate_block(
        &mut self,
        component: Spanned<String>,
        elements: Vec<Element<'a>>,
    ) -> Element<'a> {
        Element::ActivateBlock {
            component: self.fold_activate_component(component),
            elements: self.fold_elements(elements),
        }
    }

    /// Fold a fragment section
    fn fold_fragment_section(&mut self, section: FragmentSection<'a>) -> FragmentSection<'a> {
        FragmentSection {
            title: section.title,
            elements: self.fold_elements(section.elements),
        }
    }

    /// Fold a fragment
    fn fold_fragment(&mut self, fragment: Fragment<'a>) -> Fragment<'a> {
        Fragment {
            operation: fragment.operation,
            sections: fragment
                .sections
                .into_iter()
                .map(|s| self.fold_fragment_section(s))
                .collect(),
        }
    }

    /// Fold an activate component identifier into an owned `String`
    fn fold_activate_component(&mut self, component: Spanned<String>) -> Spanned<String> {
        component
    }
}

pub struct DesugarActivateBlocks;

impl<'a> Folder<'a> for DesugarActivateBlocks {
    fn fold_elements(&mut self, elements: Vec<Element<'a>>) -> Vec<Element<'a>> {
        let mut out = Vec::with_capacity(elements.len());
        for elem in elements {
            match elem {
                Element::ActivateBlock {
                    component,
                    elements: inner,
                } => {
                    let comp = self.fold_activate_component(component);
                    out.push(Element::Activate {
                        component: comp.clone(),
                    });
                    let inner_folded = self.fold_elements(inner);
                    out.extend(inner_folded);
                    out.push(Element::Deactivate { component: comp });
                }
                _ => out.push(self.fold_element(elem)),
            }
        }
        out
    }
}

/// Main entry point for the desugaring pass.
///
/// This function applies desugaring transformations to the parsed AST
/// before it's passed to the elaboration phase.
///
/// It rewrites [`ActivateBlock`] elements into explicit
/// `activate`/`deactivate` statements while preserving order and spans.
pub fn desugar<'a>(diagram: Spanned<Element<'a>>) -> Spanned<Element<'a>> {
    let mut folder = DesugarActivateBlocks;
    let span = diagram.span();
    let desugared = folder.fold_element(diagram.into_inner());
    Spanned::new(desugared, span)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::span::Span;

    // Test-only IdentityFolder for verifying identity transformations
    struct IdentityFolder;

    impl<'a> Folder<'a> for IdentityFolder {
        // Use default methods: identity behavior for all nodes
    }

    /// Helper to create a spanned value for testing
    fn spanned<T>(value: T) -> Spanned<T> {
        Spanned::new(value, Span::new(0..1))
    }

    #[test]
    fn test_identity_folder_preserves_simple_diagram() {
        // Create a simple diagram wrapped in Element
        let diagram = Element::Diagram(Diagram {
            kind: spanned("component"),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![],
        });
        let wrapped = spanned(diagram);

        // Apply the identity folder
        let mut folder = IdentityFolder;
        let result_elem = folder.fold_element(wrapped.into_inner());

        // Verify the structure is unchanged
        match result_elem {
            Element::Diagram(d) => {
                assert_eq!(*d.kind.inner(), "component");
                assert!(d.attributes.is_empty());
                assert!(d.type_definitions.is_empty());
                assert!(d.elements.is_empty());
            }
            _ => panic!("Expected diagram element"),
        }
    }

    #[test]
    fn test_identity_folder_preserves_attributes() {
        // Create a diagram with attributes
        let diagram = Element::Diagram(Diagram {
            kind: spanned("component"),
            attributes: vec![
                Attribute {
                    name: spanned("background_color"),
                    value: AttributeValue::String(spanned("#ffffff".to_string())),
                },
                Attribute {
                    name: spanned("layout_engine"),
                    value: AttributeValue::String(spanned("force".to_string())),
                },
            ],
            type_definitions: vec![],
            elements: vec![],
        });
        let wrapped = spanned(diagram);

        // Apply the identity folder
        let mut folder = IdentityFolder;
        let result_elem = folder.fold_element(wrapped.into_inner());

        // Verify attributes are preserved
        match result_elem {
            Element::Diagram(d) => {
                assert_eq!(d.attributes.len(), 2);
                assert_eq!(*d.attributes[0].name.inner(), "background_color");
                match &d.attributes[0].value {
                    AttributeValue::String(s) => assert_eq!(s.inner(), "#ffffff"),
                    _ => panic!("Expected string attribute"),
                }
            }
            _ => panic!("Expected diagram element"),
        }
    }

    #[test]
    fn test_identity_folder_preserves_type_definitions() {
        // Create a diagram with type definitions
        let diagram = Element::Diagram(Diagram {
            kind: spanned("component"),
            attributes: vec![],
            type_definitions: vec![TypeDefinition {
                name: spanned("Database"),
                base_type: spanned("Rectangle"),
                attributes: vec![Attribute {
                    name: spanned("fill_color"),
                    value: AttributeValue::String(spanned("lightblue".to_string())),
                }],
            }],
            elements: vec![],
        });
        let wrapped = spanned(diagram);

        // Apply the identity folder
        let mut folder = IdentityFolder;
        let result_elem = folder.fold_element(wrapped.into_inner());

        // Verify type definitions are preserved
        match result_elem {
            Element::Diagram(d) => {
                assert_eq!(d.type_definitions.len(), 1);
                assert_eq!(*d.type_definitions[0].name.inner(), "Database");
                assert_eq!(*d.type_definitions[0].base_type.inner(), "Rectangle");
                assert_eq!(d.type_definitions[0].attributes.len(), 1);
            }
            _ => panic!("Expected diagram element"),
        }
    }

    #[test]
    fn test_identity_folder_preserves_components() {
        // Create a diagram with a component element
        let diagram = Element::Diagram(Diagram {
            kind: spanned("component"),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![Element::Component {
                name: spanned("frontend"),
                display_name: Some(spanned("Frontend App".to_string())),
                type_name: spanned("Rectangle"),
                attributes: vec![Attribute {
                    name: spanned("fill_color"),
                    value: AttributeValue::String(spanned("blue".to_string())),
                }],
                nested_elements: vec![],
            }],
        });
        let wrapped = spanned(diagram);

        // Apply the identity folder
        let mut folder = IdentityFolder;
        let result_elem = folder.fold_element(wrapped.into_inner());

        // Verify component is preserved
        match result_elem {
            Element::Diagram(d) => {
                assert_eq!(d.elements.len(), 1);
                match &d.elements[0] {
                    Element::Component {
                        name,
                        display_name,
                        type_name,
                        attributes,
                        nested_elements,
                    } => {
                        assert_eq!(*name.inner(), "frontend");
                        assert_eq!(display_name.as_ref().unwrap().inner(), "Frontend App");
                        assert_eq!(*type_name.inner(), "Rectangle");
                        assert_eq!(attributes.len(), 1);
                        assert!(nested_elements.is_empty());
                    }
                    _ => panic!("Expected component element"),
                }
            }
            _ => panic!("Expected diagram element"),
        }
    }

    #[test]
    fn test_identity_folder_preserves_relations() {
        // Create a diagram with a relation element
        let diagram = Element::Diagram(Diagram {
            kind: spanned("component"),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![Element::Relation {
                source: spanned("frontend".to_string()),
                target: spanned("backend".to_string()),
                relation_type: spanned("->"),
                type_spec: Some(RelationTypeSpec {
                    type_name: Some(spanned("Arrow")),
                    attributes: vec![Attribute {
                        name: spanned("color"),
                        value: AttributeValue::String(spanned("red".to_string())),
                    }],
                }),
                label: Some(spanned("API Call".to_string())),
            }],
        });
        let wrapped = spanned(diagram);

        // Apply the identity folder
        let mut folder = IdentityFolder;
        let result_elem = folder.fold_element(wrapped.into_inner());

        // Verify relation is preserved
        match result_elem {
            Element::Diagram(d) => {
                assert_eq!(d.elements.len(), 1);
                match &d.elements[0] {
                    Element::Relation {
                        source,
                        target,
                        relation_type,
                        type_spec,
                        label,
                    } => {
                        assert_eq!(source.inner(), "frontend");
                        assert_eq!(target.inner(), "backend");
                        assert_eq!(*relation_type.inner(), "->");
                        assert!(type_spec.is_some());
                        assert_eq!(label.as_ref().unwrap().inner(), "API Call");
                    }
                    _ => panic!("Expected relation element"),
                }
            }
            _ => panic!("Expected diagram element"),
        }
    }

    #[test]
    fn test_identity_folder_preserves_activate_block() {
        // Create a diagram with an activate block
        let diagram = Element::Diagram(Diagram {
            kind: spanned("sequence"),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![Element::ActivateBlock {
                component: spanned("user".to_string()),
                elements: vec![Element::Relation {
                    source: spanned("user".to_string()),
                    target: spanned("server".to_string()),
                    relation_type: spanned("->"),
                    type_spec: None,
                    label: Some(spanned("Request".to_string())),
                }],
            }],
        });
        let wrapped = spanned(diagram);

        // Apply the identity folder
        let mut folder = IdentityFolder;
        let result_elem = folder.fold_element(wrapped.into_inner());

        // Verify activate block is preserved
        match result_elem {
            Element::Diagram(d) => {
                assert_eq!(d.elements.len(), 1);
                match &d.elements[0] {
                    Element::ActivateBlock {
                        component,
                        elements,
                    } => {
                        assert_eq!(*component.inner(), "user");
                        assert_eq!(elements.len(), 1);
                        match &elements[0] {
                            Element::Relation { label, .. } => {
                                assert_eq!(label.as_ref().unwrap().inner(), "Request");
                            }
                            _ => panic!("Expected relation in activate block"),
                        }
                    }
                    _ => panic!("Expected ActivateBlock element"),
                }
            }
            _ => panic!("Expected diagram element"),
        }
    }

    #[test]
    fn test_desugar_rewrites_activate_blocks() {
        // Create a diagram with an activate block
        let diagram = Element::Diagram(Diagram {
            kind: spanned("sequence"),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![Element::ActivateBlock {
                component: spanned("user".to_string()),
                elements: vec![Element::Relation {
                    source: spanned("user".to_string()),
                    target: spanned("server".to_string()),
                    relation_type: spanned("->"),
                    type_spec: None,
                    label: Some(spanned("Request".to_string())),
                }],
            }],
        });
        let wrapped = spanned(diagram);

        // Apply DesugarActivateBlocks folder directly
        let mut folder = DesugarActivateBlocks;
        let result_elem = folder.fold_element(wrapped.into_inner());

        // Verify activate block was rewritten into explicit statements
        match result_elem {
            Element::Diagram(d) => {
                assert_eq!(d.elements.len(), 3, "Expected Activate, inner, Deactivate");
                match &d.elements[0] {
                    Element::Activate { component } => {
                        assert_eq!(*component.inner(), "user");
                    }
                    _ => panic!("Expected Activate element"),
                }
                match &d.elements[1] {
                    Element::Relation { label, .. } => {
                        assert_eq!(label.as_ref().unwrap().inner(), "Request");
                    }
                    _ => panic!("Expected inner Relation element"),
                }
                match &d.elements[2] {
                    Element::Deactivate { component } => {
                        assert_eq!(*component.inner(), "user");
                    }
                    _ => panic!("Expected Deactivate element"),
                }
            }
            _ => panic!("Expected diagram element"),
        }
    }
}
