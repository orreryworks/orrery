//! Desugaring pass for the Filament AST
//!
//! This module implements a fold-based rewriting system for AST transformations.
//! It sits between the parser and elaboration phases, allowing for syntactic
//! desugaring and AST normalization.
//!
//! The design follows the Fold (Catamorphism) pattern, similar to the Rust
//! compiler's folder, where each AST node type has a corresponding fold method
//! that consumes the node and produces a transformed version.
//!
//! ## Identifier Resolution
//!
//! Component identifiers are resolved from relative names to fully qualified paths
//! using a path stack that tracks the current namespace context during traversal.
//! For example, a reference to "child1" inside "parent" becomes "parent::child1".
//! This enables the validation phase to perform comprehensive cross-reference checks.

use super::{
    builtin_types,
    parser_types::{
        Attribute, AttributeValue, Diagram, DiagramKind, Element, Fragment, FragmentSection, Note,
        TypeDefinition, TypeSpec,
    },
    span::Spanned,
};

use crate::identifier::Id;

/// Stack tracking the current namespace path for identifier resolution
#[derive(Debug)]
struct PathStack {
    /// Stack of parent paths
    /// Example: entering "parent" then "child" gives [Id("parent"), Id("parent::child")]
    stack: Vec<Id>,
}

impl PathStack {
    fn new() -> Self {
        Self { stack: Vec::new() }
    }

    /// Push a new level onto the path
    fn push(&mut self, id: Id) {
        self.stack.push(self.qualify(id));
    }

    /// Pop the current level
    fn pop(&mut self) {
        self.stack.pop();
    }

    /// Get the current qualified path (None if at root)
    fn current(&self) -> Option<Id> {
        self.stack.last().copied()
    }

    /// Qualify a name with the current path
    fn qualify(&self, id: Id) -> Id {
        if let Some(current_path) = self.current() {
            // Nested - prepend current path
            current_path.create_nested(id)
        } else {
            // At root level
            id
        }
    }
}

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
    fn fold_diagram_kind(&mut self, kind: Spanned<DiagramKind>) -> Spanned<DiagramKind> {
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
            AttributeValue::TypeSpec(type_spec) => {
                AttributeValue::TypeSpec(self.fold_type_spec(type_spec))
            }
            AttributeValue::Identifiers(ids) => {
                AttributeValue::Identifiers(self.fold_identifiers(ids))
            }
            AttributeValue::Empty => AttributeValue::Empty,
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

    /// Fold an identifiers attribute value (list of identifiers)
    fn fold_identifiers(&mut self, identifiers: Vec<Spanned<Id>>) -> Vec<Spanned<Id>> {
        identifiers
            .into_iter()
            .map(|id| self.fold_identifier(id))
            .collect()
    }

    /// Fold a single identifier
    fn fold_identifier(&mut self, identifier: Spanned<Id>) -> Spanned<Id> {
        identifier
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
            type_spec: self.fold_type_spec(type_def.type_spec),
        }
    }

    /// Fold a type name
    fn fold_type_name(&mut self, name: Spanned<Id>) -> Spanned<Id> {
        name
    }

    /// Fold a TypeSpec
    fn fold_type_spec(&mut self, type_spec: TypeSpec<'a>) -> TypeSpec<'a> {
        TypeSpec {
            type_name: type_spec.type_name.map(|tn| self.fold_type_spec_name(tn)),
            attributes: self.fold_attributes(type_spec.attributes),
        }
    }

    /// Fold a type name within a TypeSpec
    fn fold_type_spec_name(&mut self, name: Spanned<Id>) -> Spanned<Id> {
        name
    }

    /// Fold a component's TypeSpec
    fn fold_component_type_spec(&mut self, type_spec: TypeSpec<'a>) -> TypeSpec<'a> {
        self.fold_type_spec(type_spec)
    }

    /// Fold a relation's TypeSpec
    fn fold_relation_type_spec(&mut self, type_spec: TypeSpec<'a>) -> TypeSpec<'a> {
        self.fold_type_spec(type_spec)
    }

    /// Fold a note's TypeSpec
    fn fold_note_type_spec(&mut self, type_spec: TypeSpec<'a>) -> TypeSpec<'a> {
        self.fold_type_spec(type_spec)
    }

    /// Fold a fragment's TypeSpec
    fn fold_fragment_type_spec(&mut self, type_spec: TypeSpec<'a>) -> TypeSpec<'a> {
        self.fold_type_spec(type_spec)
    }

    /// Fold an activate block's TypeSpec
    fn fold_activate_type_spec(&mut self, type_spec: TypeSpec<'a>) -> TypeSpec<'a> {
        self.fold_type_spec(type_spec)
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
                type_spec,
                nested_elements,
            } => self.fold_component(name, display_name, type_spec, nested_elements),
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
                type_spec,
            } => self.fold_activate_block(component, elements, type_spec),
            Element::Activate {
                component,
                type_spec,
            } => Element::Activate {
                component: self.fold_activate_component(component),
                type_spec: self.fold_activate_type_spec(type_spec),
            },
            Element::Deactivate { component } => Element::Deactivate {
                component: self.fold_activate_component(component),
            },
            Element::Note(note) => Element::Note(self.fold_note(note)),
            Element::Fragment(fragment) => Element::Fragment(self.fold_fragment(fragment)),
            // Fragment sugar syntax - default behavior is to fold sections recursively
            Element::AltElseBlock {
                keyword_span,
                sections,
                type_spec,
            } => Element::AltElseBlock {
                keyword_span,
                type_spec: self.fold_fragment_type_spec(type_spec),
                sections: sections
                    .into_iter()
                    .map(|s| self.fold_fragment_section(s))
                    .collect(),
            },
            Element::OptBlock {
                keyword_span,
                section,
                type_spec,
            } => Element::OptBlock {
                keyword_span,
                type_spec: self.fold_fragment_type_spec(type_spec),
                section: self.fold_fragment_section(section),
            },
            Element::LoopBlock {
                keyword_span,
                section,
                type_spec,
            } => Element::LoopBlock {
                keyword_span,
                type_spec: self.fold_fragment_type_spec(type_spec),
                section: self.fold_fragment_section(section),
            },
            Element::ParBlock {
                keyword_span,
                sections,
                type_spec,
            } => Element::ParBlock {
                keyword_span,
                type_spec: self.fold_fragment_type_spec(type_spec),
                sections: sections
                    .into_iter()
                    .map(|s| self.fold_fragment_section(s))
                    .collect(),
            },
            Element::BreakBlock {
                keyword_span,
                section,
                type_spec,
            } => Element::BreakBlock {
                keyword_span,
                type_spec: self.fold_fragment_type_spec(type_spec),
                section: self.fold_fragment_section(section),
            },
            Element::CriticalBlock {
                keyword_span,
                section,
                type_spec,
            } => Element::CriticalBlock {
                keyword_span,
                type_spec: self.fold_fragment_type_spec(type_spec),
                section: self.fold_fragment_section(section),
            },
        }
    }

    /// Fold a component element
    fn fold_component(
        &mut self,
        name: Spanned<Id>,
        display_name: Option<Spanned<String>>,
        type_spec: TypeSpec<'a>,
        nested_elements: Vec<Element<'a>>,
    ) -> Element<'a> {
        Element::Component {
            name: self.fold_component_name(name),
            display_name: display_name.map(|dn| self.fold_display_name(dn)),
            type_spec: self.fold_component_type_spec(type_spec),
            nested_elements: self.fold_elements(nested_elements),
        }
    }

    /// Fold a component name
    fn fold_component_name(&mut self, name: Spanned<Id>) -> Spanned<Id> {
        self.fold_identifier(name)
    }

    /// Fold a display name
    fn fold_display_name(&mut self, display_name: Spanned<String>) -> Spanned<String> {
        display_name
    }

    /// Fold a relation element
    fn fold_relation(
        &mut self,
        source: Spanned<Id>,
        target: Spanned<Id>,
        relation_type: Spanned<&'a str>,
        type_spec: TypeSpec<'a>,
        label: Option<Spanned<String>>,
    ) -> Element<'a> {
        Element::Relation {
            source: self.fold_relation_source(source),
            target: self.fold_relation_target(target),
            relation_type: self.fold_relation_type(relation_type),
            type_spec: self.fold_relation_type_spec(type_spec),
            label: label.map(|l| self.fold_relation_label(l)),
        }
    }

    /// Fold a relation source
    fn fold_relation_source(&mut self, source: Spanned<Id>) -> Spanned<Id> {
        self.fold_identifier(source)
    }

    /// Fold a relation target
    fn fold_relation_target(&mut self, target: Spanned<Id>) -> Spanned<Id> {
        self.fold_identifier(target)
    }

    /// Fold a relation type
    fn fold_relation_type(&mut self, relation_type: Spanned<&'a str>) -> Spanned<&'a str> {
        relation_type
    }

    /// Fold a relation label
    fn fold_relation_label(&mut self, label: Spanned<String>) -> Spanned<String> {
        label
    }

    /// Fold an activate block element
    fn fold_activate_block(
        &mut self,
        component: Spanned<Id>,
        elements: Vec<Element<'a>>,
        type_spec: TypeSpec<'a>,
    ) -> Element<'a> {
        Element::ActivateBlock {
            component: self.fold_activate_component(component),
            elements: self.fold_elements(elements),
            type_spec: self.fold_activate_type_spec(type_spec),
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
            type_spec: self.fold_fragment_type_spec(fragment.type_spec),
            sections: fragment
                .sections
                .into_iter()
                .map(|s| self.fold_fragment_section(s))
                .collect(),
        }
    }

    /// Fold a note element
    fn fold_note(&mut self, note: Note<'a>) -> Note<'a> {
        Note {
            type_spec: self.fold_note_type_spec(note.type_spec),
            content: self.fold_note_content(note.content),
        }
    }

    /// Fold note content
    fn fold_note_content(&mut self, content: Spanned<String>) -> Spanned<String> {
        content
    }
    /// Fold an activate component identifier
    fn fold_activate_component(&mut self, component: Spanned<Id>) -> Spanned<Id> {
        self.fold_identifier(component)
    }
}

/// Desugaring pass for the Filament AST
///
/// This folder performs desugaring transformations:
/// 1. `ActivateBlock` → explicit `activate`/`deactivate` statements
/// 2. Fragment keyword sugar syntax → base `Fragment` elements
///    - `alt`/`else` → `fragment "alt" { ... }`
///    - `opt` → `fragment "opt" { ... }`
///    - `loop` → `fragment "loop" { ... }`
///    - `par` → `fragment "par" { ... }`
///    - `break` → `fragment "break" { ... }`
///    - `critical` → `fragment "critical" { ... }`
/// 3. Identifier Resolution
///    - Qualifies component identifiers to fully qualified paths
///    - Uses a path stack to track the current namespace context
///    - Preserves original spans for accurate error reporting
///    - Example: "child" inside "parent" becomes "parent::child"
///
/// ## Path Stack
/// The `path_stack` field tracks the current position in the component hierarchy,
/// allowing nested identifiers to be qualified with their full path from the root.
pub struct Desugar {
    path_stack: PathStack,
}

impl Desugar {
    /// Create a new Desugar folder instance
    fn new() -> Self {
        Self {
            path_stack: PathStack::new(),
        }
    }
}

impl<'a> Folder<'a> for Desugar {
    /// Override fold_component to add path tracking for identifier resolution
    fn fold_component(
        &mut self,
        name: Spanned<Id>,
        display_name: Option<Spanned<String>>,
        type_spec: TypeSpec<'a>,
        nested_elements: Vec<Element<'a>>,
    ) -> Element<'a> {
        // Enter this component's namespace
        self.path_stack.push(*name.inner());

        // Process nested elements (they will be qualified with this component's path)
        let resolved_nested = self.fold_elements(nested_elements);

        // Exit this component's namespace
        self.path_stack.pop();

        Element::Component {
            name: self.fold_component_name(name),
            display_name: display_name.map(|dn| self.fold_display_name(dn)),
            type_spec: self.fold_component_type_spec(type_spec),
            nested_elements: resolved_nested,
        }
    }

    /// Override fold_identifier to qualify identifier with current path
    fn fold_identifier(&mut self, identifier: Spanned<Id>) -> Spanned<Id> {
        let original_span = identifier.span();
        let qualified = self.path_stack.qualify(*identifier.inner());
        Spanned::new(qualified, original_span)
    }

    fn fold_elements(&mut self, elements: Vec<Element<'a>>) -> Vec<Element<'a>> {
        let mut out = Vec::with_capacity(elements.len());
        for elem in elements {
            match elem {
                Element::ActivateBlock {
                    component,
                    elements: inner,
                    type_spec,
                } => {
                    let comp = self.fold_activate_component(component);
                    out.push(Element::Activate {
                        component: comp.clone(),
                        type_spec: self.fold_activate_type_spec(type_spec),
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

    /// Override fold_relation_type_spec to inject default "Arrow" type for sugar syntax
    fn fold_relation_type_spec(&mut self, mut type_spec: TypeSpec<'a>) -> TypeSpec<'a> {
        if type_spec.type_name.is_none() {
            type_spec.type_name = Some(Spanned::new(
                Id::new(builtin_types::ARROW),
                type_spec.span(),
            ));
        }
        self.fold_type_spec(type_spec)
    }

    /// Override fold_note_type_spec to inject default "Note" type for sugar syntax
    fn fold_note_type_spec(&mut self, mut type_spec: TypeSpec<'a>) -> TypeSpec<'a> {
        if type_spec.type_name.is_none() {
            type_spec.type_name =
                Some(Spanned::new(Id::new(builtin_types::NOTE), type_spec.span()));
        }
        self.fold_type_spec(type_spec)
    }

    /// Override fold_fragment_type_spec to inject default "Fragment" type for sugar syntax
    fn fold_fragment_type_spec(&mut self, mut type_spec: TypeSpec<'a>) -> TypeSpec<'a> {
        if type_spec.type_name.is_none() {
            type_spec.type_name = Some(Spanned::new(
                Id::new(builtin_types::FRAGMENT),
                type_spec.span(),
            ));
        }
        self.fold_type_spec(type_spec)
    }

    /// Override fold_activate_type_spec to inject default "Activate" type for sugar syntax
    fn fold_activate_type_spec(&mut self, mut type_spec: TypeSpec<'a>) -> TypeSpec<'a> {
        if type_spec.type_name.is_none() {
            type_spec.type_name = Some(Spanned::new(
                Id::new(builtin_types::ACTIVATE),
                type_spec.span(),
            ));
        }
        self.fold_type_spec(type_spec)
    }

    /// Fold an Element node, performing transformations and recursive descent
    fn fold_element(&mut self, element: Element<'a>) -> Element<'a> {
        match element {
            // ========================================================================
            // NO DESUGARING - Just recursive folding to process nested elements
            // ========================================================================
            Element::Component {
                name,
                display_name,
                type_spec,
                nested_elements,
            } => self.fold_component(name, display_name, type_spec, nested_elements),
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
                type_spec,
            } => self.fold_activate_block(component, elements, type_spec),
            Element::Fragment(fragment) => Element::Fragment(self.fold_fragment(fragment)),
            Element::Activate {
                component,
                type_spec,
            } => Element::Activate {
                component: self.fold_activate_component(component),
                type_spec: self.fold_activate_type_spec(type_spec),
            },
            Element::Deactivate { component } => Element::Deactivate {
                component: self.fold_activate_component(component),
            },
            Element::Note(note) => Element::Note(self.fold_note(note)),

            // ========================================================================
            // DESUGARING TRANSFORMATIONS - Sugar syntax → Base syntax
            // ========================================================================

            // Transform alt/else to fragment "alt"
            Element::AltElseBlock {
                keyword_span,
                sections,
                type_spec,
            } => {
                let operation = Spanned::new("alt".to_string(), keyword_span);
                Element::Fragment(Fragment {
                    operation,
                    type_spec: self.fold_fragment_type_spec(type_spec),
                    sections: sections
                        .into_iter()
                        .map(|s| self.fold_fragment_section(s))
                        .collect(),
                })
            }
            // Transform opt to fragment "opt"
            Element::OptBlock {
                keyword_span,
                section,
                type_spec,
            } => {
                let operation = Spanned::new("opt".to_string(), keyword_span);
                Element::Fragment(Fragment {
                    operation,
                    type_spec: self.fold_fragment_type_spec(type_spec),
                    sections: vec![self.fold_fragment_section(section)],
                })
            }
            // Transform loop to fragment "loop"
            Element::LoopBlock {
                keyword_span,
                section,
                type_spec,
            } => {
                let operation = Spanned::new("loop".to_string(), keyword_span);
                Element::Fragment(Fragment {
                    operation,
                    type_spec: self.fold_fragment_type_spec(type_spec),
                    sections: vec![self.fold_fragment_section(section)],
                })
            }
            // Transform par to fragment "par"
            Element::ParBlock {
                keyword_span,
                sections,
                type_spec,
            } => {
                let operation = Spanned::new("par".to_string(), keyword_span);
                Element::Fragment(Fragment {
                    operation,
                    type_spec: self.fold_fragment_type_spec(type_spec),
                    sections: sections
                        .into_iter()
                        .map(|s| self.fold_fragment_section(s))
                        .collect(),
                })
            }
            // Transform break to fragment "break"
            Element::BreakBlock {
                keyword_span,
                section,
                type_spec,
            } => {
                let operation = Spanned::new("break".to_string(), keyword_span);
                Element::Fragment(Fragment {
                    operation,
                    type_spec: self.fold_fragment_type_spec(type_spec),
                    sections: vec![self.fold_fragment_section(section)],
                })
            }
            // Transform critical to fragment "critical"
            Element::CriticalBlock {
                keyword_span,
                section,
                type_spec,
            } => {
                let operation = Spanned::new("critical".to_string(), keyword_span);
                Element::Fragment(Fragment {
                    operation,
                    type_spec: self.fold_fragment_type_spec(type_spec),
                    sections: vec![self.fold_fragment_section(section)],
                })
            }
        }
    }
}

/// Main entry point for the desugaring pass.
///
/// This function applies desugaring transformations to the parsed AST
/// before it's passed to the elaboration phase.
///
/// All desugaring happens in a single pass using the `Desugar` folder:
/// 1. `ActivateBlock` elements → explicit `activate`/`deactivate` statements
/// 2. Fragment keyword sugar syntax → base `Fragment` elements
/// 3. Component identifiers → fully qualified paths (e.g., "child" → "parent::child")
pub fn desugar<'a>(diagram: Spanned<Element<'a>>) -> Spanned<Element<'a>> {
    let span = diagram.span();
    let mut folder = Desugar::new();
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
    fn test_path_stack_operations() {
        let mut stack = PathStack::new();
        assert_eq!(stack.current(), None);

        stack.push(Id::new("parent"));
        assert_eq!(stack.current().unwrap(), "parent");
        assert_eq!(stack.qualify(Id::new("child")), "parent::child");

        stack.push(Id::new("child"));
        assert_eq!(stack.current().unwrap(), "parent::child");
        assert_eq!(
            stack.qualify(Id::new("grandchild")),
            "parent::child::grandchild"
        );

        stack.pop();
        assert_eq!(stack.current().unwrap(), "parent");

        stack.pop();
        assert_eq!(stack.current(), None);
    }

    #[test]
    fn test_identity_folder_preserves_simple_diagram() {
        // Create a simple diagram wrapped in Element
        let diagram = Element::Diagram(Diagram {
            kind: spanned(DiagramKind::Component),
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
                assert_eq!(*d.kind, DiagramKind::Component);
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
            kind: spanned(DiagramKind::Component),
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
            kind: spanned(DiagramKind::Component),
            attributes: vec![],
            type_definitions: vec![TypeDefinition {
                name: spanned(Id::new("Database")),
                type_spec: TypeSpec {
                    type_name: Some(spanned(Id::new("Rectangle"))),
                    attributes: vec![Attribute {
                        name: spanned("fill_color"),
                        value: AttributeValue::String(spanned("lightblue".to_string())),
                    }],
                },
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
                assert_eq!(
                    *d.type_definitions[0]
                        .type_spec
                        .type_name
                        .as_ref()
                        .unwrap()
                        .inner(),
                    "Rectangle"
                );
                assert_eq!(d.type_definitions[0].type_spec.attributes.len(), 1);
            }
            _ => panic!("Expected diagram element"),
        }
    }

    #[test]
    fn test_identity_folder_preserves_components() {
        // Create a diagram with a component element
        let diagram = Element::Diagram(Diagram {
            kind: spanned(DiagramKind::Component),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![Element::Component {
                name: spanned(Id::new("frontend")),
                display_name: Some(spanned("Frontend App".to_string())),
                type_spec: TypeSpec {
                    type_name: Some(spanned(Id::new("Rectangle"))),
                    attributes: vec![Attribute {
                        name: spanned("fill_color"),
                        value: AttributeValue::String(spanned("blue".to_string())),
                    }],
                },
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
                        type_spec,
                        nested_elements,
                    } => {
                        assert_eq!(*name.inner(), "frontend");
                        assert_eq!(display_name.as_ref().unwrap().inner(), "Frontend App");
                        assert_eq!(*type_spec.type_name.as_ref().unwrap().inner(), "Rectangle");
                        assert_eq!(type_spec.attributes.len(), 1);
                        assert!(nested_elements.is_empty());
                    }
                    _ => panic!("Expected component element"),
                }
            }
            _ => panic!("Expected diagram element"),
        }
    }

    #[test]
    fn test_identity_folder_preserves_activate_block() {
        // Create a diagram with an activate block
        let diagram = Element::Diagram(Diagram {
            kind: spanned(DiagramKind::Sequence),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![Element::ActivateBlock {
                component: spanned(Id::new("user")),
                type_spec: TypeSpec::default(),
                elements: vec![Element::Relation {
                    source: spanned(Id::new("user")),
                    target: spanned(Id::new("server")),
                    relation_type: spanned("->"),
                    type_spec: TypeSpec::default(),
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
                        ..
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
            kind: spanned(DiagramKind::Sequence),
            attributes: vec![],
            type_definitions: vec![],
            elements: vec![Element::ActivateBlock {
                component: spanned(Id::new("user")),
                type_spec: TypeSpec::default(),
                elements: vec![Element::Relation {
                    source: spanned(Id::new("user")),
                    target: spanned(Id::new("server")),
                    relation_type: spanned("->"),
                    type_spec: TypeSpec::default(),
                    label: Some(spanned("Request".to_string())),
                }],
            }],
        });
        let wrapped = spanned(diagram);

        // Apply Desugar folder directly
        let mut folder = Desugar::new();
        let result_elem = folder.fold_element(wrapped.into_inner());

        // Verify activate block was rewritten into explicit statements
        match result_elem {
            Element::Diagram(d) => {
                assert_eq!(d.elements.len(), 3, "Expected Activate, inner, Deactivate");
                match &d.elements[0] {
                    Element::Activate { component, .. } => {
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

    #[test]
    fn test_desugar_opt_block() {
        let opt_block = Element::OptBlock {
            keyword_span: Span::new(0..3),
            section: FragmentSection {
                title: Some(spanned("user authenticated".to_string())),
                elements: vec![Element::Relation {
                    source: spanned(Id::new("user")),
                    target: spanned(Id::new("profile")),
                    relation_type: spanned("->"),
                    type_spec: TypeSpec::default(),
                    label: Some(spanned("Load".to_string())),
                }],
            },
            type_spec: TypeSpec::default(),
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(opt_block);

        match result {
            Element::Fragment(fragment) => {
                assert_eq!(*fragment.operation.inner(), "opt");
                assert_eq!(fragment.sections.len(), 1);
                assert_eq!(
                    fragment.sections[0].title.as_ref().unwrap().inner(),
                    "user authenticated"
                );
                assert_eq!(fragment.sections[0].elements.len(), 1);
            }
            _ => panic!("Expected Fragment element"),
        }
    }

    #[test]
    fn test_desugar_loop_block() {
        let loop_block = Element::LoopBlock {
            keyword_span: Span::new(0..4),
            section: FragmentSection {
                title: Some(spanned("for each item".to_string())),
                elements: vec![Element::Relation {
                    source: spanned(Id::new("client")),
                    target: spanned(Id::new("server")),
                    relation_type: spanned("->"),
                    type_spec: TypeSpec::default(),
                    label: Some(spanned("Process".to_string())),
                }],
            },
            type_spec: TypeSpec::default(),
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(loop_block);

        match result {
            Element::Fragment(fragment) => {
                assert_eq!(*fragment.operation.inner(), "loop");
                assert_eq!(fragment.sections.len(), 1);
                assert_eq!(
                    fragment.sections[0].title.as_ref().unwrap().inner(),
                    "for each item"
                );
            }
            _ => panic!("Expected Fragment element"),
        }
    }

    #[test]
    fn test_desugar_break_block() {
        let break_block = Element::BreakBlock {
            keyword_span: Span::new(0..5),
            section: FragmentSection {
                title: Some(spanned("timeout".to_string())),
                elements: vec![Element::Relation {
                    source: spanned(Id::new("client")),
                    target: spanned(Id::new("server")),
                    relation_type: spanned("->"),
                    type_spec: TypeSpec::default(),
                    label: Some(spanned("Cancel".to_string())),
                }],
            },
            type_spec: TypeSpec::default(),
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(break_block);

        match result {
            Element::Fragment(fragment) => {
                assert_eq!(*fragment.operation.inner(), "break");
                assert_eq!(fragment.sections.len(), 1);
                assert_eq!(
                    fragment.sections[0].title.as_ref().unwrap().inner(),
                    "timeout"
                );
            }
            _ => panic!("Expected Fragment element"),
        }
    }

    #[test]
    fn test_desugar_critical_block() {
        let critical_block = Element::CriticalBlock {
            keyword_span: Span::new(0..8),
            section: FragmentSection {
                title: Some(spanned("database lock".to_string())),
                elements: vec![Element::Relation {
                    source: spanned(Id::new("app")),
                    target: spanned(Id::new("db")),
                    relation_type: spanned("->"),
                    type_spec: TypeSpec::default(),
                    label: Some(spanned("UPDATE".to_string())),
                }],
            },
            type_spec: TypeSpec::default(),
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(critical_block);

        match result {
            Element::Fragment(fragment) => {
                assert_eq!(*fragment.operation.inner(), "critical");
                assert_eq!(fragment.sections.len(), 1);
                assert_eq!(
                    fragment.sections[0].title.as_ref().unwrap().inner(),
                    "database lock"
                );
            }
            _ => panic!("Expected Fragment element"),
        }
    }

    #[test]
    fn test_desugar_alt_else_block() {
        let alt_else_block = Element::AltElseBlock {
            keyword_span: Span::new(0..3),
            sections: vec![
                FragmentSection {
                    title: Some(spanned("x > 0".to_string())),
                    elements: vec![Element::Relation {
                        source: spanned(Id::new("a")),
                        target: spanned(Id::new("b")),
                        relation_type: spanned("->"),
                        type_spec: TypeSpec::default(),
                        label: None,
                    }],
                },
                FragmentSection {
                    title: Some(spanned("x < 0".to_string())),
                    elements: vec![Element::Relation {
                        source: spanned(Id::new("b")),
                        target: spanned(Id::new("a")),
                        relation_type: spanned("->"),
                        type_spec: TypeSpec::default(),
                        label: None,
                    }],
                },
                FragmentSection {
                    title: None,
                    elements: vec![Element::Relation {
                        source: spanned(Id::new("a")),
                        target: spanned(Id::new("a")),
                        relation_type: spanned("->"),
                        type_spec: TypeSpec::default(),
                        label: None,
                    }],
                },
            ],
            type_spec: TypeSpec::default(),
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(alt_else_block);

        match result {
            Element::Fragment(fragment) => {
                assert_eq!(*fragment.operation.inner(), "alt");
                assert_eq!(fragment.sections.len(), 3);
                assert_eq!(
                    fragment.sections[0].title.as_ref().unwrap().inner(),
                    "x > 0"
                );
                assert_eq!(
                    fragment.sections[1].title.as_ref().unwrap().inner(),
                    "x < 0"
                );
                assert!(fragment.sections[2].title.is_none());
            }
            _ => panic!("Expected Fragment element"),
        }
    }

    #[test]
    fn test_desugar_par_block() {
        let par_block = Element::ParBlock {
            keyword_span: Span::new(0..3),
            sections: vec![
                FragmentSection {
                    title: Some(spanned("thread 1".to_string())),
                    elements: vec![Element::Relation {
                        source: spanned(Id::new("a")),
                        target: spanned(Id::new("b")),
                        relation_type: spanned("->"),
                        type_spec: TypeSpec::default(),
                        label: None,
                    }],
                },
                FragmentSection {
                    title: Some(spanned("thread 2".to_string())),
                    elements: vec![Element::Relation {
                        source: spanned(Id::new("c")),
                        target: spanned(Id::new("d")),
                        relation_type: spanned("->"),
                        type_spec: TypeSpec::default(),
                        label: None,
                    }],
                },
            ],
            type_spec: TypeSpec::default(),
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(par_block);

        match result {
            Element::Fragment(fragment) => {
                assert_eq!(*fragment.operation.inner(), "par");
                assert_eq!(fragment.sections.len(), 2);
                assert_eq!(
                    fragment.sections[0].title.as_ref().unwrap().inner(),
                    "thread 1"
                );
                assert_eq!(
                    fragment.sections[1].title.as_ref().unwrap().inner(),
                    "thread 2"
                );
            }
            _ => panic!("Expected Fragment element"),
        }
    }

    #[test]
    fn test_desugar_preserves_attributes() {
        let opt_block = Element::OptBlock {
            keyword_span: Span::new(0..3),
            section: FragmentSection {
                title: Some(spanned("condition".to_string())),
                elements: vec![],
            },
            type_spec: TypeSpec {
                type_name: None,
                attributes: vec![
                    Attribute {
                        name: spanned("background_color"),
                        value: AttributeValue::String(spanned("#f0f0f0".to_string())),
                    },
                    Attribute {
                        name: spanned("border_style"),
                        value: AttributeValue::String(spanned("dashed".to_string())),
                    },
                ],
            },
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(opt_block);

        match result {
            Element::Fragment(fragment) => {
                assert_eq!(fragment.type_spec.attributes.len(), 2);
                assert_eq!(
                    *fragment.type_spec.attributes[0].name.inner(),
                    "background_color"
                );
                assert_eq!(
                    *fragment.type_spec.attributes[1].name.inner(),
                    "border_style"
                );
            }
            _ => panic!("Expected Fragment element"),
        }
    }

    #[test]
    fn test_desugar_nested_fragments() {
        // Create an opt block containing a nested alt/else structure
        let nested_alt = Element::AltElseBlock {
            keyword_span: Span::new(5..8),
            sections: vec![
                FragmentSection {
                    title: Some(spanned("case 1".to_string())),
                    elements: vec![],
                },
                FragmentSection {
                    title: Some(spanned("case 2".to_string())),
                    elements: vec![],
                },
            ],
            type_spec: TypeSpec::default(),
        };

        let opt_block = Element::OptBlock {
            keyword_span: Span::new(0..3),
            section: FragmentSection {
                title: Some(spanned("outer condition".to_string())),
                elements: vec![nested_alt],
            },
            type_spec: TypeSpec::default(),
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(opt_block);

        match result {
            Element::Fragment(outer_fragment) => {
                assert_eq!(*outer_fragment.operation.inner(), "opt");
                assert_eq!(outer_fragment.sections.len(), 1);

                // Check that nested alt was also desugared
                match &outer_fragment.sections[0].elements[0] {
                    Element::Fragment(inner_fragment) => {
                        assert_eq!(*inner_fragment.operation.inner(), "alt");
                        assert_eq!(inner_fragment.sections.len(), 2);
                    }
                    _ => panic!("Expected nested Fragment element"),
                }
            }
            _ => panic!("Expected Fragment element"),
        }
    }

    #[test]
    fn test_desugar_keyword_span_preserved() {
        let keyword_span = Span::new(10..13);
        let opt_block = Element::OptBlock {
            keyword_span,
            section: FragmentSection {
                title: None,
                elements: vec![],
            },
            type_spec: TypeSpec::default(),
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(opt_block);

        match result {
            Element::Fragment(fragment) => {
                // The operation string should have the same span as the original keyword
                assert_eq!(fragment.operation.span(), keyword_span);
            }
            _ => panic!("Expected Fragment element"),
        }
    }

    #[test]
    fn test_desugar_empty_sections() {
        // Test that empty sections are handled correctly
        let opt_block = Element::OptBlock {
            keyword_span: Span::new(0..3),
            section: FragmentSection {
                title: None,
                elements: vec![],
            },
            type_spec: TypeSpec::default(),
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(opt_block);

        match result {
            Element::Fragment(fragment) => {
                assert_eq!(fragment.sections.len(), 1);
                assert!(fragment.sections[0].title.is_none());
                assert!(fragment.sections[0].elements.is_empty());
            }
            _ => panic!("Expected Fragment element"),
        }
    }

    #[test]
    fn test_desugar_preserves_note() {
        let note = Element::Note(Note {
            type_spec: TypeSpec::default(),
            content: spanned("Simple note".to_string()),
        });

        let mut folder = Desugar::new();
        let result = folder.fold_element(note);

        match result {
            Element::Note(note_result) => {
                assert_eq!(note_result.type_spec.attributes.len(), 0);
                assert_eq!(note_result.content.inner(), "Simple note");
            }
            _ => panic!("Expected Note element"),
        }
    }

    #[test]
    fn test_desugar_preserves_note_attributes() {
        let note = Element::Note(Note {
            type_spec: TypeSpec {
                type_name: None,
                attributes: vec![
                    Attribute {
                        name: spanned("align"),
                        value: AttributeValue::String(spanned("left".to_string())),
                    },
                    Attribute {
                        name: spanned("on"),
                        value: AttributeValue::Identifiers(vec![spanned(Id::new("component"))]),
                    },
                ],
            },
            content: spanned("Note with attributes".to_string()),
        });

        let mut folder = Desugar::new();
        let result = folder.fold_element(note);

        match result {
            Element::Note(note_result) => {
                assert_eq!(note_result.type_spec.attributes.len(), 2);
                assert_eq!(*note_result.type_spec.attributes[0].name.inner(), "align");
                assert_eq!(*note_result.type_spec.attributes[1].name.inner(), "on");
                assert_eq!(note_result.content.inner(), "Note with attributes");
            }
            _ => panic!("Expected Note element"),
        }
    }

    #[test]
    fn test_nested_component_relation_resolution() {
        // Create a nested component with a relation between siblings
        let parent_component = Element::Component {
            name: spanned(Id::new("parent")),
            display_name: None,
            type_spec: TypeSpec {
                type_name: Some(spanned(Id::new("Rectangle"))),
                attributes: vec![],
            },
            nested_elements: vec![
                Element::Component {
                    name: spanned(Id::new("child1")),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(spanned(Id::new("Oval"))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Component {
                    name: spanned(Id::new("child2")),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(spanned(Id::new("Rectangle"))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Relation {
                    source: spanned(Id::new("child1")),
                    target: spanned(Id::new("child2")),
                    relation_type: spanned("->"),
                    type_spec: TypeSpec::default(),
                    label: None,
                },
            ],
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(parent_component);

        // Extract the relation from the result
        if let Element::Component {
            nested_elements, ..
        } = result
        {
            let relation = nested_elements.iter().find_map(|e| match e {
                Element::Relation { source, target, .. } => Some((source, target)),
                _ => None,
            });

            if let Some((source, target)) = relation {
                assert_eq!(source.inner(), "parent::child1");
                assert_eq!(target.inner(), "parent::child2");
            } else {
                panic!("Expected to find a relation in nested elements");
            }
        } else {
            panic!("Expected Component element");
        }
    }

    #[test]
    fn test_deeply_nested_component_resolution() {
        // Create deeply nested components: level1 { level2 { level3 } }
        let level1 = Element::Component {
            name: spanned(Id::new("level1")),
            display_name: None,
            type_spec: TypeSpec {
                type_name: Some(spanned(Id::new("Rectangle"))),
                attributes: vec![],
            },
            nested_elements: vec![Element::Component {
                name: spanned(Id::new("level2")),
                display_name: None,
                type_spec: TypeSpec {
                    type_name: Some(spanned(Id::new("Rectangle"))),
                    attributes: vec![],
                },
                nested_elements: vec![
                    Element::Component {
                        name: spanned(Id::new("level3")),
                        display_name: None,
                        type_spec: TypeSpec {
                            type_name: Some(spanned(Id::new("Oval"))),
                            attributes: vec![],
                        },
                        nested_elements: vec![],
                    },
                    Element::Relation {
                        source: spanned(Id::new("level3")),
                        target: spanned(Id::new("sibling")),
                        relation_type: spanned("->"),
                        type_spec: TypeSpec::default(),
                        label: None,
                    },
                ],
            }],
        };

        let mut folder = Desugar {
            path_stack: PathStack::new(),
        };
        let result = folder.fold_element(level1);

        // Navigate to the deeply nested relation
        if let Element::Component {
            nested_elements: level1_nested,
            ..
        } = result
        {
            if let Some(Element::Component {
                nested_elements: level2_nested,
                ..
            }) = level1_nested.first()
            {
                let relation = level2_nested.iter().find_map(|e| match e {
                    Element::Relation { source, target, .. } => Some((source, target)),
                    _ => None,
                });

                if let Some((source, target)) = relation {
                    assert_eq!(source.inner(), "level1::level2::level3");
                    assert_eq!(target.inner(), "level1::level2::sibling");
                } else {
                    panic!("Expected to find a relation");
                }
            } else {
                panic!("Expected nested component");
            }
        } else {
            panic!("Expected Component element");
        }
    }

    #[test]
    fn test_activate_component_resolution() {
        let parent_component = Element::Component {
            name: spanned(Id::new("parent")),
            display_name: None,
            type_spec: TypeSpec {
                type_name: Some(spanned(Id::new("Rectangle"))),
                attributes: vec![],
            },
            nested_elements: vec![
                Element::Component {
                    name: spanned(Id::new("child")),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(spanned(Id::new("Oval"))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Activate {
                    component: spanned(Id::new("child")),
                    type_spec: TypeSpec::default(),
                },
            ],
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(parent_component);

        if let Element::Component {
            nested_elements, ..
        } = result
        {
            let activate = nested_elements.iter().find_map(|e| match e {
                Element::Activate { component, .. } => Some(component),
                _ => None,
            });

            if let Some(component) = activate {
                assert_eq!(component.inner(), "parent::child");
            } else {
                panic!("Expected to find Activate element");
            }
        } else {
            panic!("Expected Component element");
        }
    }

    #[test]
    fn test_note_on_attribute_resolution() {
        let parent_component = Element::Component {
            name: spanned(Id::new("parent")),
            display_name: None,
            type_spec: TypeSpec {
                type_name: Some(spanned(Id::new("Rectangle"))),
                attributes: vec![],
            },
            nested_elements: vec![
                Element::Component {
                    name: spanned(Id::new("child")),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(spanned(Id::new("Oval"))),
                        attributes: vec![],
                    },
                    nested_elements: vec![],
                },
                Element::Note(Note {
                    type_spec: TypeSpec {
                        type_name: None,
                        attributes: vec![Attribute {
                            name: spanned("on"),
                            value: AttributeValue::Identifiers(vec![spanned(Id::new("child"))]),
                        }],
                    },
                    content: spanned("Note about child".to_string()),
                }),
            ],
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(parent_component);

        if let Element::Component {
            nested_elements, ..
        } = result
        {
            let note = nested_elements.iter().find_map(|e| match e {
                Element::Note(n) => Some(n),
                _ => None,
            });

            if let Some(note) = note {
                let on_attr = note
                    .type_spec
                    .attributes
                    .iter()
                    .find(|a| *a.name.inner() == "on");
                if let Some(attr) = on_attr {
                    if let AttributeValue::Identifiers(ids) = &attr.value {
                        assert_eq!(ids[0].inner(), "parent::child");
                    } else {
                        panic!("Expected Identifiers value");
                    }
                } else {
                    panic!("Expected 'on' attribute");
                }
            } else {
                panic!("Expected Note element");
            }
        } else {
            panic!("Expected Component element");
        }
    }

    #[test]
    fn test_root_level_identifiers_unchanged() {
        // Relations at root level should remain unchanged
        let relation = Element::Relation {
            source: spanned(Id::new("system1")),
            target: spanned(Id::new("system2")),
            relation_type: spanned("->"),
            type_spec: TypeSpec::default(),
            label: None,
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(relation);

        if let Element::Relation { source, target, .. } = result {
            assert_eq!(source.inner(), "system1");
            assert_eq!(target.inner(), "system2");
        } else {
            panic!("Expected Relation element");
        }
    }

    #[test]
    fn test_span_preservation_during_resolution() {
        // Verify that spans are preserved when identifiers are qualified
        let original_span = Span::new(10..15);
        let parent_component = Element::Component {
            name: spanned(Id::new("parent")),
            display_name: None,
            type_spec: TypeSpec {
                type_name: Some(spanned(Id::new("Rectangle"))),
                attributes: vec![],
            },
            nested_elements: vec![Element::Relation {
                source: Spanned::new(Id::new("child"), original_span),
                target: spanned(Id::new("other")),
                relation_type: spanned("->"),
                type_spec: TypeSpec::default(),
                label: None,
            }],
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(parent_component);

        if let Element::Component {
            nested_elements, ..
        } = result
        {
            if let Some(Element::Relation { source, .. }) = nested_elements.first() {
                // The identifier should be qualified, but the span should be preserved
                assert_eq!(source.inner(), "parent::child");
                assert_eq!(source.span(), original_span);
            } else {
                panic!("Expected Relation element");
            }
        } else {
            panic!("Expected Component element");
        }
    }

    #[test]
    fn test_desugar_preserves_type_spec_in_relations() {
        // Verify that TypeSpec in relations is preserved during desugaring
        let relation = Element::Relation {
            source: spanned(Id::new("a")),
            target: spanned(Id::new("b")),
            relation_type: spanned("->"),
            type_spec: TypeSpec {
                type_name: Some(spanned(Id::new("Arrow"))),
                attributes: vec![Attribute {
                    name: spanned("color"),
                    value: AttributeValue::String(spanned("red".to_string())),
                }],
            },
            label: None,
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(relation);

        match result {
            Element::Relation { type_spec, .. } => {
                assert_eq!(*type_spec.type_name.as_ref().unwrap().inner(), "Arrow");
                assert_eq!(type_spec.attributes.len(), 1);
                assert_eq!(*type_spec.attributes[0].name.inner(), "color");
                match &type_spec.attributes[0].value {
                    AttributeValue::String(s) => assert_eq!(s.inner(), "red"),
                    _ => panic!("Expected string attribute value"),
                }
            }
            _ => panic!("Expected Relation"),
        }
    }

    #[test]
    fn test_fragment_sugar_preserves_type_spec() {
        // Verify opt/alt/loop blocks preserve TypeSpec when desugared to Fragment
        let opt_block = Element::OptBlock {
            keyword_span: Span::new(0..3),
            type_spec: TypeSpec {
                type_name: Some(spanned(Id::new("CustomFragment"))),
                attributes: vec![Attribute {
                    name: spanned("bg"),
                    value: AttributeValue::String(spanned("yellow".to_string())),
                }],
            },
            section: FragmentSection {
                title: Some(spanned("condition".to_string())),
                elements: vec![],
            },
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(opt_block);

        match result {
            Element::Fragment(fragment) => {
                assert_eq!(*fragment.operation.inner(), "opt");
                assert_eq!(
                    *fragment.type_spec.type_name.as_ref().unwrap().inner(),
                    "CustomFragment"
                );
                assert_eq!(fragment.type_spec.attributes.len(), 1);
                assert_eq!(*fragment.type_spec.attributes[0].name.inner(), "bg");
                assert_eq!(fragment.sections.len(), 1);
                assert_eq!(
                    fragment.sections[0].title.as_ref().unwrap().inner(),
                    "condition"
                );
            }
            _ => panic!("Expected Fragment"),
        }
    }

    #[test]
    fn test_desugar_preserves_type_spec_in_activate_statement() {
        // Verify TypeSpec in activate statement is preserved
        let activate = Element::Activate {
            component: spanned(Id::new("user")),
            type_spec: TypeSpec {
                type_name: Some(spanned(Id::new("CustomActivation"))),
                attributes: vec![Attribute {
                    name: spanned("fill"),
                    value: AttributeValue::String(spanned("green".to_string())),
                }],
            },
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(activate);

        match result {
            Element::Activate {
                component,
                type_spec,
            } => {
                assert_eq!(*component.inner(), "user");
                assert_eq!(
                    *type_spec.type_name.as_ref().unwrap().inner(),
                    "CustomActivation"
                );
                assert_eq!(type_spec.attributes.len(), 1);
                assert_eq!(*type_spec.attributes[0].name.inner(), "fill");
            }
            _ => panic!("Expected Activate"),
        }
    }

    #[test]
    fn test_desugar_loop_block_preserves_type_spec() {
        // Verify loop blocks preserve TypeSpec with attributes
        let loop_block = Element::LoopBlock {
            keyword_span: Span::new(0..4),
            type_spec: TypeSpec {
                type_name: Some(spanned(Id::new("RepeatFragment"))),
                attributes: vec![
                    Attribute {
                        name: spanned("border"),
                        value: AttributeValue::String(spanned("dashed".to_string())),
                    },
                    Attribute {
                        name: spanned("color"),
                        value: AttributeValue::String(spanned("blue".to_string())),
                    },
                ],
            },
            section: FragmentSection {
                title: Some(spanned("retry".to_string())),
                elements: vec![],
            },
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(loop_block);

        match result {
            Element::Fragment(fragment) => {
                assert_eq!(*fragment.operation.inner(), "loop");
                assert_eq!(
                    *fragment.type_spec.type_name.as_ref().unwrap().inner(),
                    "RepeatFragment"
                );
                assert_eq!(fragment.type_spec.attributes.len(), 2);
                assert_eq!(*fragment.type_spec.attributes[0].name.inner(), "border");
                assert_eq!(*fragment.type_spec.attributes[1].name.inner(), "color");
            }
            _ => panic!("Expected Fragment"),
        }
    }

    #[test]
    fn test_desugar_relation_sugar_injects_arrow_type() {
        // Verify relation without type_name gets "Arrow" injected
        let relation = Element::Relation {
            source: spanned(Id::new("client")),
            target: spanned(Id::new("server")),
            relation_type: spanned("->"),
            type_spec: TypeSpec {
                type_name: None,
                attributes: vec![],
            },
            label: None,
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(relation);

        match result {
            Element::Relation { type_spec, .. } => {
                assert!(type_spec.type_name.is_some());
                assert_eq!(*type_spec.type_name.unwrap().inner(), builtin_types::ARROW);
                assert_eq!(type_spec.attributes.len(), 0);
            }
            _ => panic!("Expected Relation"),
        }
    }

    #[test]
    fn test_desugar_relation_sugar_with_attributes_injects_arrow() {
        // Verify relation with attributes but no type_name gets "Arrow" injected
        let relation = Element::Relation {
            source: spanned(Id::new("api")),
            target: spanned(Id::new("db")),
            relation_type: spanned("->"),
            type_spec: TypeSpec {
                type_name: None,
                attributes: vec![Attribute {
                    name: spanned("color"),
                    value: AttributeValue::String(spanned("red".to_string())),
                }],
            },
            label: Some(spanned("query".to_string())),
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(relation);

        match result {
            Element::Relation { type_spec, .. } => {
                assert!(type_spec.type_name.is_some());
                assert_eq!(*type_spec.type_name.unwrap().inner(), builtin_types::ARROW);
                assert_eq!(type_spec.attributes.len(), 1);
                assert_eq!(*type_spec.attributes[0].name.inner(), "color");
            }
            _ => panic!("Expected Relation"),
        }
    }

    #[test]
    fn test_desugar_relation_with_explicit_type_unchanged() {
        // Verify relation with explicit type_name is not modified
        let relation = Element::Relation {
            source: spanned(Id::new("a")),
            target: spanned(Id::new("b")),
            relation_type: spanned("->"),
            type_spec: TypeSpec {
                type_name: Some(spanned(Id::new("DashedArrow"))),
                attributes: vec![],
            },
            label: None,
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(relation);

        match result {
            Element::Relation { type_spec, .. } => {
                assert_eq!(*type_spec.type_name.unwrap().inner(), "DashedArrow");
            }
            _ => panic!("Expected Relation"),
        }
    }

    #[test]
    fn test_desugar_note_sugar_injects_note_type() {
        // Verify note without type_name gets "Note" injected
        let note = Element::Note(Note {
            type_spec: TypeSpec {
                type_name: None,
                attributes: vec![],
            },
            content: spanned("Important message".to_string()),
        });

        let mut folder = Desugar::new();
        let result = folder.fold_element(note);

        match result {
            Element::Note(note) => {
                assert!(note.type_spec.type_name.is_some());
                assert_eq!(
                    *note.type_spec.type_name.unwrap().inner(),
                    builtin_types::NOTE
                );
                assert_eq!(note.type_spec.attributes.len(), 0);
            }
            _ => panic!("Expected Note"),
        }
    }

    #[test]
    fn test_desugar_note_sugar_with_attributes_injects_note() {
        // Verify note with attributes but no type_name gets "Note" injected
        let note = Element::Note(Note {
            type_spec: TypeSpec {
                type_name: None,
                attributes: vec![Attribute {
                    name: spanned("align"),
                    value: AttributeValue::String(spanned("left".to_string())),
                }],
            },
            content: spanned("Side note".to_string()),
        });

        let mut folder = Desugar::new();
        let result = folder.fold_element(note);

        match result {
            Element::Note(note) => {
                assert!(note.type_spec.type_name.is_some());
                assert_eq!(
                    *note.type_spec.type_name.unwrap().inner(),
                    builtin_types::NOTE
                );
                assert_eq!(note.type_spec.attributes.len(), 1);
                assert_eq!(*note.type_spec.attributes[0].name.inner(), "align");
            }
            _ => panic!("Expected Note"),
        }
    }

    #[test]
    fn test_desugar_fragment_sugar_injects_fragment_type() {
        // Verify opt block without type_name gets "Fragment" injected
        let opt_block = Element::OptBlock {
            keyword_span: Span::new(0..3),
            type_spec: TypeSpec {
                type_name: None,
                attributes: vec![],
            },
            section: FragmentSection {
                title: Some(spanned("condition".to_string())),
                elements: vec![],
            },
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(opt_block);

        match result {
            Element::Fragment(fragment) => {
                assert_eq!(*fragment.operation.inner(), "opt");
                assert!(fragment.type_spec.type_name.is_some());
                assert_eq!(
                    *fragment.type_spec.type_name.unwrap().inner(),
                    builtin_types::FRAGMENT
                );
            }
            _ => panic!("Expected Fragment"),
        }
    }

    #[test]
    fn test_desugar_activate_block_sugar_injects_activate_type() {
        // Verify activate block without type_name gets "Activate" injected
        let activate_block = Element::ActivateBlock {
            component: spanned(Id::new("service")),
            type_spec: TypeSpec {
                type_name: None,
                attributes: vec![],
            },
            elements: vec![],
        };

        let mut folder = Desugar::new();
        let result_elements = folder.fold_elements(vec![activate_block]);

        // Should desugar to: Activate, Deactivate
        assert_eq!(result_elements.len(), 2);
        match &result_elements[0] {
            Element::Activate { type_spec, .. } => {
                assert!(type_spec.type_name.is_some());
                assert_eq!(
                    *type_spec.type_name.as_ref().unwrap().inner(),
                    builtin_types::ACTIVATE
                );
            }
            _ => panic!("Expected Activate"),
        }
    }

    #[test]
    fn test_desugar_activate_statement_sugar_injects_activate_type() {
        // Verify activate statement without type_name gets "Activate" injected
        let activate = Element::Activate {
            component: spanned(Id::new("component")),
            type_spec: TypeSpec {
                type_name: None,
                attributes: vec![],
            },
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(activate);

        match result {
            Element::Activate { type_spec, .. } => {
                assert!(type_spec.type_name.is_some());
                assert_eq!(
                    *type_spec.type_name.unwrap().inner(),
                    builtin_types::ACTIVATE
                );
            }
            _ => panic!("Expected Activate"),
        }
    }
}
