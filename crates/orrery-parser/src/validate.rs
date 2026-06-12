//! Validation module for AST elements using the visitor pattern
//!
//! This module implements a visitor-based (read-only) traversal system for AST validation.
//! It sits between the desugar and elaboration phases, allowing for semantic
//! validation of the AST before elaboration.
//!
//! ## Validations Performed
//!
//! - **Component Identifier References**: Validates that all component identifiers referenced
//!   in relations, notes, and activation statements are defined in the diagram.
//! - **Activate/Deactivate Pairing**: Ensures activate statements have corresponding deactivate
//!   statements in sequence diagrams.
//! - **Note Alignment**: Validates that note alignment values are appropriate for the diagram type.
//! - **Embed Reference Resolution**: Validates that all `DiagramSource::Ref` nodes were resolved
//!   during desugaring. Surviving refs indicate an unknown embed reference.

use std::{
    collections::{HashMap, HashSet},
    mem,
};

use orrery_core::{identifier::Id, semantic::DiagramKind};

use crate::{
    builtin_types,
    error::{Diagnostic, DiagnosticCollector, ErrorCode},
    parser_types::{
        Attribute, AttributeValue, ComponentContent, DiagramSource, Element, FileAst, FileHeader,
        Fragment, FragmentSection, Import, Note, TypeDefinition, TypeSpec,
    },
    span::{Span, Spanned},
};

/// Visitor trait for traversing/analyzing AST nodes.
///
/// Each method takes a reference to its input and can accumulate state or errors.
/// Default implementations perform recursive traversal so implementors can override
/// only the methods they care about.
trait Visitor<'a> {
    /// Walks a complete [`FileAst`].
    fn visit_file_ast(&mut self, file_ast: &FileAst<'a>) {
        self.visit_header(&file_ast.header);
        self.visit_imports(&file_ast.imports);
        self.visit_type_definitions(&file_ast.type_definitions);
        self.visit_elements(&file_ast.elements);
    }

    /// Visits the file header.
    fn visit_header(&mut self, header: &FileHeader<'a>) {
        match header {
            FileHeader::Diagram { kind, attributes } => {
                self.visit_diagram_kind(kind);
                self.visit_attributes(attributes);
            }
            FileHeader::Library { .. } => {}
        }
    }

    /// Iterates over resolved imports and recursively visits each import's inner [`FileAst`].
    fn visit_imports(&mut self, imports: &[Import<'a>]) {
        for import in imports {
            self.visit_file_ast(&import.file_ast.borrow());
        }
    }

    /// Visits the diagram kind (component, sequence, etc.).
    fn visit_diagram_kind(&mut self, _kind: &Spanned<DiagramKind>) {}

    /// Visits a list of attributes.
    fn visit_attributes(&mut self, attributes: &[Attribute<'a>]) {
        for attr in attributes {
            self.visit_attribute(attr);
        }
    }

    /// Visits a single attribute.
    fn visit_attribute(&mut self, attribute: &Attribute<'a>) {
        self.visit_attribute_name(&attribute.name);
        self.visit_attribute_value(&attribute.value);
    }

    /// Visits an attribute name.
    fn visit_attribute_name(&mut self, _name: &Spanned<&'a str>) {}

    /// Visits an attribute value.
    fn visit_attribute_value(&mut self, value: &AttributeValue<'a>) {
        match value {
            AttributeValue::String(s) => self.visit_string_value(s),
            AttributeValue::Float(f) => self.visit_float_value(f),
            AttributeValue::TypeSpec(type_spec) => self.visit_type_spec(type_spec),
            AttributeValue::Identifiers(ids) => self.visit_identifiers(ids),
            AttributeValue::Empty => {}
        }
    }

    /// Visits a string attribute value.
    fn visit_string_value(&mut self, _value: &Spanned<String>) {}

    /// Visits a float attribute value.
    fn visit_float_value(&mut self, _value: &Spanned<f32>) {}

    /// Visits a single identifier (component reference).
    fn visit_identifier(&mut self, _identifier: &Spanned<Id>) {}

    /// Visits an identifiers attribute value (list of identifiers).
    fn visit_identifiers(&mut self, identifiers: &[Spanned<Id>]) {
        for identifier in identifiers {
            self.visit_identifier(identifier);
        }
    }

    /// Visits a list of type definitions.
    fn visit_type_definitions(&mut self, type_definitions: &[TypeDefinition<'a>]) {
        for td in type_definitions {
            self.visit_type_definition(td);
        }
    }

    /// Visits a single type definition.
    fn visit_type_definition(&mut self, type_def: &TypeDefinition<'a>) {
        self.visit_type_spec(&type_def.type_spec);
        self.visit_type_name(&type_def.name);
    }

    /// Visits a type name.
    fn visit_type_name(&mut self, _name: &Spanned<Id>) {}

    /// Visits a base type.
    fn visit_base_type(&mut self, _base_type: &Spanned<Id>) {}

    /// Visits a type specification.
    fn visit_type_spec(&mut self, type_spec: &TypeSpec<'a>) {
        if let Some(ref type_name) = type_spec.type_name {
            self.visit_base_type(type_name);
        }
        self.visit_attributes(&type_spec.attributes);
    }

    /// Visits a list of elements.
    fn visit_elements(&mut self, elements: &[Element<'a>]) {
        for elem in elements {
            self.visit_element(elem);
        }
    }

    /// Visits a single element by dispatching to the appropriate typed visitor method.
    ///
    /// Each [`Element`] variant is routed to its dedicated `visit_*` method.
    fn visit_element(&mut self, element: &Element<'a>) {
        match *element {
            Element::Component {
                ref name,
                ref display_name,
                ref type_spec,
                ref content,
            } => self.visit_component(name, display_name, type_spec, content),
            Element::Relation {
                ref source,
                ref target,
                ref relation_type,
                ref type_spec,
                ref label,
            } => self.visit_relation(source, target, relation_type, type_spec, label),
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

    /// Visits a fragment.
    fn visit_fragment(&mut self, fragment: &Fragment<'a>) {
        for section in &fragment.sections {
            self.visit_fragment_section(section);
        }
    }

    /// Visits a fragment section.
    fn visit_fragment_section(&mut self, section: &FragmentSection<'a>) {
        // Traverse section title as a string literal and its elements
        if let Some(title) = &section.title {
            self.visit_string_value(title);
        }
        self.visit_elements(&section.elements);
    }

    /// Visits a component element.
    fn visit_component(
        &mut self,
        name: &Spanned<Id>,
        display_name: &Option<Spanned<String>>,
        type_spec: &TypeSpec<'a>,
        content: &ComponentContent<'a>,
    ) {
        self.visit_component_name(name);
        if let Some(dn) = display_name {
            self.visit_display_name(dn);
        }
        self.visit_type_spec(type_spec);
        self.visit_component_content(content);
    }

    /// Visits the content of a component element.
    fn visit_component_content(&mut self, content: &ComponentContent<'a>) {
        match content {
            ComponentContent::None => {}
            ComponentContent::Scope(elements) => self.visit_elements(elements),
            ComponentContent::Diagram(source) => self.visit_diagram_source(source),
        }
    }

    /// Visits an embedded diagram source inside a component.
    fn visit_diagram_source(&mut self, source: &DiagramSource<'a>) {
        match source {
            DiagramSource::Inline(rc) => self.visit_file_ast(&rc.borrow()),
            DiagramSource::Ref(_) => {}
        }
    }

    /// Visits a component name.
    fn visit_component_name(&mut self, _name: &Spanned<Id>) {}

    /// Visits a display name.
    fn visit_display_name(&mut self, _display_name: &Spanned<String>) {}

    /// Visits a relation element.
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

    /// Visits a relation source.
    fn visit_relation_source(&mut self, source: &Spanned<Id>) {
        self.visit_identifier(source);
    }

    /// Visits a relation target.
    fn visit_relation_target(&mut self, target: &Spanned<Id>) {
        self.visit_identifier(target);
    }

    /// Visits a relation type.
    fn visit_relation_type(&mut self, _relation_type: &Spanned<&'a str>) {}

    /// Visits a relation label.
    fn visit_relation_label(&mut self, _label: &Spanned<String>) {}

    /// Visits an activate block element.
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

    /// Visits an activate block component reference.
    fn visit_activate_component(&mut self, _component: &Spanned<Id>) {}

    /// Visits an activate statement.
    fn visit_activate(&mut self, component: &Spanned<Id>, type_spec: &TypeSpec<'a>) {
        self.visit_identifier(component);
        self.visit_type_spec(type_spec);
    }

    /// Visits a deactivate statement.
    fn visit_deactivate(&mut self, component: &Spanned<Id>) {
        self.visit_identifier(component);
    }

    /// Visits a note element.
    fn visit_note(&mut self, note: &Note<'a>) {
        self.visit_type_spec(&note.type_spec);
        self.visit_note_content(&note.content);
    }

    /// Visits note content.
    fn visit_note_content(&mut self, _content: &Spanned<String>) {}
}

/// Entry point for running a visitor on a file AST.
fn visit_file_ast<'a, V: Visitor<'a>>(visitor: &mut V, file_ast: &FileAst<'a>) {
    visitor.visit_file_ast(file_ast)
}

struct FileAstState {
    type_registry: HashSet<Id>,
    activation_stack: HashMap<Id, Vec<Span>>,
    component_registry: HashMap<Id, Span>,
    diagram_kind: Option<DiagramKind>,
}

impl FileAstState {
    fn new() -> Self {
        let type_registry = builtin_types::defaults()
            .into_iter()
            .map(|type_def| type_def.id())
            .collect();

        Self {
            type_registry,
            activation_stack: HashMap::new(),
            component_registry: HashMap::new(),
            diagram_kind: None,
        }
    }
}

/// Validator that checks all file AST semantic constraints.
///
/// Uses a visitor-based traversal to validate:
/// - Component identifier references (relations, notes, activate/deactivate).
/// - Activate/deactivate pairing in sequence diagrams.
/// - Note attribute values (align).
/// - Embed reference resolution (diagram sources).
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
pub struct Validator {
    state: FileAstState,
    diagnostics: DiagnosticCollector,
}

impl Validator {
    /// Creates a new [`Validator`] with empty state.
    pub fn new() -> Self {
        Self {
            state: FileAstState::new(),
            diagnostics: DiagnosticCollector::new(),
        }
    }

    /// Validates that an `align` value is appropriate for the current diagram type.
    ///
    /// Sequence diagrams support: over, left, right
    /// Component diagrams support: left, right, top, bottom
    ///
    /// Note: The None and unknown diagram type cases are defensive programming.
    /// The parser enforces valid diagram types, but we handle these cases
    /// to fail gracefully if the validation is called incorrectly.
    fn validate_align_for_diagram_type(&mut self, align_value: &str, span: Span) {
        match self.state.diagram_kind {
            Some(DiagramKind::Sequence) => {
                if !matches!(align_value, "over" | "left" | "right") {
                    self.diagnostics.emit(
                        Diagnostic::error(format!(
                            "invalid align value `{align_value}` for sequence diagram"
                        ))
                        .with_code(ErrorCode::E203)
                        .with_label(span, "invalid align value")
                        .with_help("valid values: over, left, right"),
                    );
                }
            }
            Some(DiagramKind::Component) => {
                if !matches!(align_value, "left" | "right" | "top" | "bottom") {
                    self.diagnostics.emit(
                        Diagnostic::error(format!(
                            "invalid align value `{align_value}` for component diagram"
                        ))
                        .with_code(ErrorCode::E203)
                        .with_label(span, "invalid align value")
                        .with_help("valid values: left, right, top, bottom"),
                    );
                }
            }
            None => {
                self.diagnostics.emit(
                    Diagnostic::error("diagram type not set, cannot validate align attribute")
                        .with_code(ErrorCode::E203)
                        .with_label(span, "missing diagram type"),
                );
            }
        }
    }

    fn validate_file_ast_state(&mut self) {
        self.validate_activation_stack_pairs();
    }

    // Validate any remaining unpaired activations.
    fn validate_activation_stack_pairs(&mut self) {
        for (component_id, spans) in self.state.activation_stack.iter() {
            if !spans.is_empty() {
                let span = spans.last().cloned().unwrap_or_default();
                self.diagnostics.emit(
                    Diagnostic::error(format!(
                        "component `{component_id}` was activated but never deactivated"
                    ))
                    .with_code(ErrorCode::E201)
                    .with_label(span, "unpaired activate")
                    .with_help(
                        "every activate statement must have a corresponding deactivate statement",
                    ),
                );
            }
        }
    }
}

impl<'a> Visitor<'a> for Validator {
    /// Pushes a fresh component registry scope before visiting the file's children.
    fn visit_file_ast(&mut self, file_ast: &FileAst<'a>) {
        let last_state = mem::replace(&mut self.state, FileAstState::new());

        // Call default traversal
        self.visit_header(&file_ast.header);
        self.visit_imports(&file_ast.imports);
        self.visit_type_definitions(&file_ast.type_definitions);
        self.visit_elements(&file_ast.elements);

        self.validate_file_ast_state();

        // Restore the previous state
        self.state = last_state;
    }

    /// Records the current diagram kind for later validation.
    fn visit_diagram_kind(&mut self, kind: &Spanned<DiagramKind>) {
        self.state.diagram_kind = Some(**kind);
    }

    /// Visits a type name.
    fn visit_type_name(&mut self, name: &Spanned<Id>) {
        self.state.type_registry.insert(*name.inner());
    }

    /// Checks that the base type is a registered built-in or user-defined type,
    /// emitting `E205` if it is unknown.
    fn visit_base_type(&mut self, base_type: &Spanned<Id>) {
        if !self.state.type_registry.contains(base_type.inner()) {
            self.diagnostics.emit(
                Diagnostic::error(format!("unknown base type `{base_type}`"))
                    .with_code(ErrorCode::E205)
                    .with_label(base_type.span(), "unknown base type")
                    .with_help(format!(
                        "type `{base_type}` must be a built-in type or defined with a `type` statement before it can be used as a base type"
                    )),
            );
        }
    }

    /// Registers the component name in the current diagram's component registry.
    fn visit_component_name(&mut self, name: &Spanned<Id>) {
        self.state
            .component_registry
            .insert(*name.inner(), name.span());
    }

    /// Validates the activation target and pushes it onto the activation stack.
    fn visit_activate(&mut self, component: &Spanned<Id>, type_spec: &TypeSpec<'a>) {
        // Validate component identifier exists
        self.visit_identifier(component);

        self.visit_type_spec(type_spec);

        // Then handle activation stack logic
        self.state
            .activation_stack
            .entry(*component.inner())
            .or_default()
            .push(component.span());
    }

    /// Validates the deactivation target and pops it from the activation stack, emitting `E202` on mismatch.
    fn visit_deactivate(&mut self, component: &Spanned<Id>) {
        // Validate component identifier exists
        self.visit_identifier(component);

        // Then handle activation stack logic
        match self.state.activation_stack.get_mut(component.inner()) {
            Some(spans) if !spans.is_empty() => {
                // Remove the most recent activation span (LIFO)
                let _ = spans.pop();
            }
            _ => {
                // No matching activate
                self.diagnostics.emit(
                    Diagnostic::error(format!(
                        "cannot deactivate component `{}`: no matching activate statement",
                        component.inner()
                    ))
                    .with_code(ErrorCode::E202)
                    .with_label(component.span(), "unpaired deactivate")
                    .with_help("deactivate statements must be preceded by a corresponding activate statement"),
                );
            }
        }
    }

    /// Validates the `align` attribute against the current diagram type, emitting `E203` if invalid.
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

    /// Emits `E204` for any `DiagramSource::Ref` that survived desugaring.
    ///
    /// An `Inline` source is traversed normally. A `Ref` source means the embed
    /// reference was never resolved to a known namespaced import, so an
    /// "unknown embed reference" diagnostic is emitted.
    fn visit_diagram_source(&mut self, source: &DiagramSource<'a>) {
        match source {
            DiagramSource::Inline(rc) => self.visit_file_ast(&rc.borrow()),
            DiagramSource::Ref(id) => {
                // TODO: This reports E204 for all unresolved embed refs, but
                // doesn't distinguish between a truly unknown name and a name
                // that matches a *library* import (which can't be embedded).
                // The latter should ideally report ("cannot embed library
                // file") with a more targeted message.
                self.diagnostics.emit(
                    Diagnostic::error(format!(
                        "unknown embed reference `{}`",
                        id.inner()
                    ))
                    .with_code(ErrorCode::E204)
                    .with_label(
                        id.span(),
                        "no namespaced import matches this identifier",
                    )
                    .with_help(
                        "add a namespaced import: `import \"diagram_file\";` or `import \"path\" as name;`",
                    ),
                );
            }
        }
    }

    /// Checks that the referenced component exists in the registry, emitting `E200` if not found.
    fn visit_identifier(&mut self, identifier: &Spanned<Id>) {
        if !self
            .state
            .component_registry
            .contains_key(identifier.inner())
        {
            self.diagnostics.emit(
                Diagnostic::error(format!("component `{}` not found", identifier.inner()))
                    .with_code(ErrorCode::E200)
                    .with_label(identifier.span(), "undefined component")
                    .with_help("component must be defined before it can be referenced"),
            );
        }
    }
}

/// Convenience function to run all file AST validations.
///
/// Creates a [`Validator`], traverses the given [`FileAst`] with the visitor pattern,
/// and collects any semantic errors found during traversal.
///
/// # Arguments
///
/// * `ast` - The parsed and desugared [`FileAst`] to validate.
///
/// # Returns
///
/// - `Ok(())` when no validation issues are found.
/// - `Err(Vec<Diagnostic>)` with all collected diagnostics otherwise.
///
/// # Errors
///
/// Returns `Vec<Diagnostic>` if one or more semantic validation checks fail.
pub fn validate(ast: &FileAst<'_>) -> Result<(), Vec<Diagnostic>> {
    let mut validator = Validator::new();
    visit_file_ast(&mut validator, ast);
    validator.diagnostics.finish()
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
            content: &ComponentContent<'a>,
        ) {
            self.component_count += 1;
            // Call default traversal
            self.visit_component_name(name);
            if let Some(dn) = display_name {
                self.visit_display_name(dn);
            }
            self.visit_type_spec(type_spec);
            self.visit_component_content(content);
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
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Sequence, Span::new(0..8)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("user"), Span::new(10..14)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(16..25))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
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
            imports: vec![],
        };

        let mut visitor = CountingVisitor::new();
        visit_file_ast(&mut visitor, &diagram);

        assert_eq!(visitor.component_count, 1);
        assert_eq!(visitor.relation_count, 1);
        assert_eq!(visitor.activate_count, 1);
        assert_eq!(visitor.deactivate_count, 1);
    }

    #[test]
    fn test_validate_ok_pair() {
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Sequence, Span::new(0..8)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("user"), Span::new(0..4)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(6..15))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
                },
                Element::Activate {
                    component: Spanned::new(Id::new("user"), Span::new(17..21)),
                    type_spec: TypeSpec::default(),
                },
                Element::Deactivate {
                    component: Spanned::new(Id::new("user"), Span::new(23..27)),
                },
            ],
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_unpaired_deactivate() {
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Sequence, Span::new(0..8)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![Element::Deactivate {
                component: Spanned::new(Id::new("user"), Span::new(0..4)),
            }],
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_unpaired_activate_end_of_scope() {
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Sequence, Span::new(0..8)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![Element::Activate {
                component: Spanned::new(Id::new("user"), Span::new(0..4)),
                type_spec: TypeSpec::default(),
            }],
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_nested_activations_ok() {
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Sequence, Span::new(0..8)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("user"), Span::new(0..4)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(6..15))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
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
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_interleaved_components_ok() {
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Sequence, Span::new(0..8)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("user"), Span::new(0..4)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(6..15))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
                },
                Element::Component {
                    name: Spanned::new(Id::new("server"), Span::new(17..23)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(25..34))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
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
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_out_of_order_deactivate_first() {
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Sequence, Span::new(0..8)),
                attributes: vec![],
            },
            import_decls: vec![],
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
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod note_validation_tests {
    use super::*;
    use crate::{lexer::tokenize, parser::build_file};

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

        let tokens = tokenize(input, 0).expect("Failed to tokenize");
        let ast = build_file(&tokens).expect("Failed to parse");
        let result = validate(&ast);
        assert!(result.is_ok(), "Valid notes should pass validation");
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

        let tokens = tokenize(input, 0).expect("Failed to tokenize");
        let ast = build_file(&tokens).expect("Failed to parse");
        let result = validate(&ast);
        assert!(result.is_ok(), "Valid notes should pass validation");
    }

    #[test]
    fn test_invalid_align_sequence_diagram() {
        let input = r#"
        diagram sequence;
        client: Rectangle;

        note [on=[client], align="top"]: "Invalid align for sequence";
        "#;

        let tokens = tokenize(input, 0).expect("Failed to tokenize");
        let ast = build_file(&tokens).expect("Failed to parse");
        let result = validate(&ast);
        assert!(result.is_err(), "Invalid align should fail validation");

        let err = result.unwrap_err();
        assert!(format!("{}", err[0]).contains("invalid align value `top` for sequence diagram"));
    }

    #[test]
    fn test_invalid_align_component_diagram() {
        let input = r#"
        diagram component;
        api: Rectangle;

        note [on=[api], align="over"]: "Invalid align for component";
        "#;

        let tokens = tokenize(input, 0).expect("Failed to tokenize");
        let ast = build_file(&tokens).expect("Failed to parse");
        let result = validate(&ast);
        assert!(result.is_err(), "Invalid align should fail validation");

        let err = result.unwrap_err();
        assert!(format!("{}", err[0]).contains("invalid align value `over` for component diagram"));
    }

    #[test]
    fn test_multiple_component_references() {
        let input = r#"
        diagram sequence;
        client: Rectangle;
        server: Rectangle;

        note [on=[client, server]]: "Valid spanning note";
        "#;

        let tokens = tokenize(input, 0).expect("Failed to tokenize");
        let ast = build_file(&tokens).expect("Failed to parse");
        let result = validate(&ast);
        assert!(result.is_ok(), "Valid spanning note should pass validation");
    }

    #[test]
    fn test_empty_on_attribute() {
        let input = r#"
        diagram sequence;
        client: Rectangle;

        note [on=[]]: "Margin note with empty on";
        "#;

        let tokens = tokenize(input, 0).expect("Failed to tokenize");
        let ast = build_file(&tokens).expect("Failed to parse");
        let result = validate(&ast);
        assert!(
            result.is_ok(),
            "Empty on attribute should be valid (margin note)"
        );
    }

    #[test]
    fn test_align_validation_without_diagram_kind() {
        // Defensive case: align validation when no diagram kind has been set.
        // The parser enforces a diagram kind, so this exercises the graceful
        // fallback path directly.
        let mut validator = Validator::new();
        assert!(validator.state.diagram_kind.is_none());

        validator.validate_align_for_diagram_type("left", Span::new(0..4));

        let err = validator.diagnostics.finish().unwrap_err();
        assert_eq!(err.len(), 1);
        assert_eq!(err[0].code(), Some(ErrorCode::E203));
        assert!(
            err[0].to_string().contains("diagram type not set"),
            "unexpected message: {}",
            err[0]
        );
    }
}

#[cfg(test)]
mod identifier_validation_tests {
    use super::*;

    #[test]
    fn test_component_registry_fully_qualified_access() {
        // Test that fully qualified identifiers from nested components
        // are all accessible in a single diagram-level registry
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Component, Span::new(0..9)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("frontend"), Span::new(0..8)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(10..19))),
                        attributes: vec![],
                    },
                    content: ComponentContent::Scope(vec![
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
                            content: ComponentContent::None,
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
                            content: ComponentContent::None,
                        },
                    ]),
                },
                Element::Component {
                    name: Spanned::new(Id::new("backend"), Span::new(69..76)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(78..87))),
                        attributes: vec![],
                    },
                    content: ComponentContent::Scope(vec![Element::Component {
                        name: Spanned::new(Id::new("backend::api"), Span::new(88..100)),
                        display_name: None,
                        type_spec: TypeSpec {
                            type_name: Some(Spanned::new(
                                Id::new("Rectangle"),
                                Span::new(102..111),
                            )),
                            attributes: vec![],
                        },
                        content: ComponentContent::None,
                    }]),
                },
            ],
            imports: vec![],
        };

        let mut validator = Validator::new();
        visit_file_ast(&mut validator, &diagram);

        // All components should be registered at diagram level with fully qualified names
        // This includes: frontend, frontend::app, frontend::ui, backend, backend::api
        assert!(validator.diagnostics.finish().is_ok());
    }

    #[test]
    fn test_visit_identifier_not_found() {
        let mut validator = Validator::new();

        // Set up registry with a component
        validator
            .state
            .component_registry
            .insert(Id::new("app"), Span::new(0..3));

        // Test visit_identifier with a non-existent component
        validator.visit_identifier(&Spanned::new(Id::new("unknown"), Span::new(10..17)));

        // Should have an error
        let err = validator.diagnostics.finish().unwrap_err();
        assert_eq!(err.len(), 1);
        assert!(err[0].to_string().contains("component `unknown` not found"));
    }

    #[test]
    fn test_visit_identifiers_multiple() {
        let mut validator = Validator::new();

        // Set up registry with multiple components
        validator.state.component_registry.extend(vec![
            (Id::new("client"), Span::new(0..6)),
            (Id::new("server"), Span::new(18..24)),
        ]);

        // Test visit_identifier with multiple components
        validator.visit_identifier(&Spanned::new(Id::new("client"), Span::new(40..46)));
        validator.visit_identifier(&Spanned::new(Id::new("server"), Span::new(48..54)));

        // Should not add any errors
        assert!(validator.diagnostics.finish().is_ok());
    }

    #[test]
    fn test_visit_identifiers_some_missing() {
        let mut validator = Validator::new();

        // Set up registry with one component
        validator
            .state
            .component_registry
            .insert(Id::new("client"), Span::new(0..6));

        // Test visit_identifier with one valid and one invalid component
        validator.visit_identifier(&Spanned::new(Id::new("client"), Span::new(40..46)));
        validator.visit_identifier(&Spanned::new(Id::new("unknown"), Span::new(48..55)));

        // Should have one error for the missing component
        let err = validator.diagnostics.finish().unwrap_err();
        assert_eq!(err.len(), 1);
        assert!(err[0].to_string().contains("component `unknown` not found"));
    }

    #[test]
    fn test_relation_with_valid_components() {
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Component, Span::new(0..9)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("app"), Span::new(0..3)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(5..14))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
                },
                Element::Component {
                    name: Spanned::new(Id::new("db"), Span::new(15..17)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(19..28))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
                },
                Element::Relation {
                    source: Spanned::new(Id::new("app"), Span::new(30..33)),
                    target: Spanned::new(Id::new("db"), Span::new(37..39)),
                    relation_type: Spanned::new("->", Span::new(34..36)),
                    type_spec: TypeSpec::default(),
                    label: None,
                },
            ],
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(result.is_ok(), "Valid relation should pass validation");
    }

    #[test]
    fn test_relation_with_invalid_source() {
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Component, Span::new(0..9)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("db"), Span::new(15..17)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(19..28))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
                },
                Element::Relation {
                    source: Spanned::new(Id::new("unknown"), Span::new(30..37)),
                    target: Spanned::new(Id::new("db"), Span::new(41..43)),
                    relation_type: Spanned::new("->", Span::new(38..40)),
                    type_spec: TypeSpec::default(),
                    label: None,
                },
            ],
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(result.is_err(), "Invalid source should fail validation");
        let err = result.unwrap_err();
        assert!(err[0].to_string().contains("component `unknown` not found"));
    }

    #[test]
    fn test_relation_with_invalid_target() {
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Component, Span::new(0..9)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("app"), Span::new(0..3)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(5..14))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
                },
                Element::Relation {
                    source: Spanned::new(Id::new("app"), Span::new(30..33)),
                    target: Spanned::new(Id::new("missing"), Span::new(37..44)),
                    relation_type: Spanned::new("->", Span::new(34..36)),
                    type_spec: TypeSpec::default(),
                    label: None,
                },
            ],
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(result.is_err(), "Invalid target should fail validation");
        let err = result.unwrap_err();
        assert!(err[0].to_string().contains("component `missing` not found"));
    }

    #[test]
    fn test_activate_with_valid_component() {
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Sequence, Span::new(0..8)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("server"), Span::new(0..6)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(8..17))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
                },
                Element::Activate {
                    component: Spanned::new(Id::new("server"), Span::new(20..26)),
                    type_spec: TypeSpec::default(),
                },
                Element::Deactivate {
                    component: Spanned::new(Id::new("server"), Span::new(30..36)),
                },
            ],
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(result.is_ok(), "Valid activate should pass validation");
    }

    #[test]
    fn test_activate_with_invalid_component() {
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Sequence, Span::new(0..8)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("server"), Span::new(0..6)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(8..17))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
                },
                Element::Activate {
                    component: Spanned::new(Id::new("unknown"), Span::new(20..27)),
                    type_spec: TypeSpec::default(),
                },
            ],
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(result.is_err(), "Invalid activate should fail validation");
        let err = result.unwrap_err();
        assert!(err[0].to_string().contains("component `unknown` not found"));
    }

    #[test]
    fn test_deactivate_with_invalid_component() {
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Sequence, Span::new(0..8)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("server"), Span::new(0..6)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(8..17))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
                },
                Element::Deactivate {
                    component: Spanned::new(Id::new("missing"), Span::new(20..27)),
                },
            ],
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(result.is_err(), "Invalid deactivate should fail validation");
        let err = result.unwrap_err();
        assert!(err[0].to_string().contains("component `missing` not found"));
    }

    #[test]
    fn test_note_with_invalid_component() {
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Sequence, Span::new(0..8)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("client"), Span::new(0..6)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(8..17))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
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
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(result.is_err(), "Note with invalid component should fail");
        let err = result.unwrap_err();
        assert!(err[0].to_string().contains("component `unknown` not found"));
    }

    #[test]
    fn test_note_with_multiple_components() {
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Sequence, Span::new(0..8)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("client"), Span::new(0..6)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(8..17))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
                },
                Element::Component {
                    name: Spanned::new(Id::new("server"), Span::new(19..25)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(27..36))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
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
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(
            result.is_ok(),
            "Note with multiple valid components should pass"
        );
    }

    #[test]
    fn test_note_with_empty_on_attribute() {
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Sequence, Span::new(0..8)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![
                Element::Component {
                    name: Spanned::new(Id::new("client"), Span::new(0..6)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(8..17))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
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
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(result.is_ok(), "Note with empty on attribute should pass");
    }

    #[test]
    fn test_validation_with_typespec() {
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Sequence, Span::new(0..8)),
                attributes: vec![],
            },
            import_decls: vec![],
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
                    content: ComponentContent::None,
                },
                Element::Component {
                    name: Spanned::new(Id::new("server"), Span::new(85..91)),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(93..102))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
                },
                Element::Activate {
                    component: Spanned::new(Id::new("server"), Span::new(110..116)),
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Activate"), Span::new(118..128))),
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
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(
            result.is_ok(),
            "Diagram with comprehensive TypeSpec usage should pass validation"
        );
    }
}

#[cfg(test)]
mod embed_ref_validation_tests {
    use super::*;
    use crate::{lexer::tokenize, parser::build_file};

    #[test]
    fn test_embed_ref_unknown_produces_e204() {
        let input = r#"
        diagram component;
        box: Rectangle embed nonexistent;
        "#;

        let tokens = tokenize(input, 0).expect("Failed to tokenize");
        let ast = build_file(&tokens).expect("Failed to parse");
        // Note: desugar is NOT called here — we test the validator directly
        // against an AST with an unresolved DiagramSource::Ref.
        let result = validate(&ast);
        assert!(
            result.is_err(),
            "Unresolved embed ref should fail validation"
        );

        let err = result.unwrap_err();
        assert_eq!(err.len(), 1);
        assert_eq!(
            err[0].code(),
            Some(ErrorCode::E204),
            "Expected E204, got: {:?}",
            err[0].code()
        );
    }

    #[test]
    fn test_embed_inline_passes_validation() {
        let input = r#"
        diagram component;
        box: Rectangle embed { diagram sequence; };
        "#;

        let tokens = tokenize(input, 0).expect("Failed to tokenize");
        let ast = build_file(&tokens).expect("Failed to parse");
        let result = validate(&ast);
        assert!(
            result.is_ok(),
            "Inline embed should pass validation: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_embed_ref_multiple_unresolved() {
        let input = r#"
        diagram component;
        box1: Rectangle embed missing1;
        box2: Rectangle embed missing2;
        "#;

        let tokens = tokenize(input, 0).expect("Failed to tokenize");
        let ast = build_file(&tokens).expect("Failed to parse");
        let result = validate(&ast);
        assert!(
            result.is_err(),
            "Unresolved embed refs should fail validation"
        );

        let err = result.unwrap_err();
        assert_eq!(err.len(), 2, "Expected two E204 errors, got: {:?}", err);
        assert_eq!(err[0].code(), Some(ErrorCode::E204));
        assert_eq!(err[1].code(), Some(ErrorCode::E204));
    }
}

#[cfg(test)]
mod base_type_validation_tests {
    use super::*;

    #[test]
    fn test_unknown_base_type_produces_e205() {
        // A component whose type name is neither a built-in nor a user-defined
        // type should produce an E205 "unknown base type" diagnostic.
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Component, Span::new(0..9)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![Element::Component {
                name: Spanned::new(Id::new("box"), Span::new(10..13)),
                display_name: None,
                type_spec: TypeSpec {
                    type_name: Some(Spanned::new(Id::new("UnknownShape"), Span::new(15..27))),
                    attributes: vec![],
                },
                content: ComponentContent::None,
            }],
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(result.is_err(), "Unknown base type should fail validation");

        let err = result.unwrap_err();
        assert_eq!(err.len(), 1);
        assert_eq!(err[0].code(), Some(ErrorCode::E205));
        assert!(
            err[0].to_string().contains("unknown base type"),
            "unexpected message: {}",
            err[0]
        );
    }

    #[test]
    fn test_unknown_base_type_in_type_definition_produces_e205() {
        // A `type` definition whose base type is unknown should also produce E205.
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Component, Span::new(0..9)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![TypeDefinition {
                name: Spanned::new(Id::new("MyType"), Span::new(15..21)),
                type_spec: TypeSpec {
                    type_name: Some(Spanned::new(Id::new("DoesNotExist"), Span::new(24..36))),
                    attributes: vec![],
                },
            }],
            elements: vec![],
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.len(), 1);
        assert_eq!(err[0].code(), Some(ErrorCode::E205));
    }

    #[test]
    fn test_builtin_base_type_ok() {
        // A component using a built-in type (`Rectangle`) should pass validation.
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Component, Span::new(0..9)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![Element::Component {
                name: Spanned::new(Id::new("box"), Span::new(10..13)),
                display_name: None,
                type_spec: TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(15..24))),
                    attributes: vec![],
                },
                content: ComponentContent::None,
            }],
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(
            result.is_ok(),
            "Built-in base type should pass: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_user_defined_type_as_base_type_ok() {
        // A user-defined type registered via `visit_type_name` should be usable
        // as the base type of both a later type definition and a component.
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Component, Span::new(0..9)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![
                // type MyBase : Rectangle;
                TypeDefinition {
                    name: Spanned::new(Id::new("MyBase"), Span::new(15..21)),
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(24..33))),
                        attributes: vec![],
                    },
                },
                // type Derived : MyBase;
                TypeDefinition {
                    name: Spanned::new(Id::new("Derived"), Span::new(40..47)),
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("MyBase"), Span::new(50..56))),
                        attributes: vec![],
                    },
                },
            ],
            elements: vec![Element::Component {
                name: Spanned::new(Id::new("box"), Span::new(60..63)),
                display_name: None,
                type_spec: TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Derived"), Span::new(65..72))),
                    attributes: vec![],
                },
                content: ComponentContent::None,
            }],
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(
            result.is_ok(),
            "User-defined types used as base types should pass: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_self_reference_without_prior_definition_produces_e205() {
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Component, Span::new(0..9)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![TypeDefinition {
                name: Spanned::new(Id::new("MyType"), Span::new(15..21)),
                type_spec: TypeSpec {
                    type_name: Some(Spanned::new(Id::new("MyType"), Span::new(24..30))),
                    attributes: vec![Attribute {
                        name: Spanned::new("fill_color", Span::new(31..41)),
                        value: AttributeValue::String(Spanned::new(
                            "red".to_string(),
                            Span::new(43..48),
                        )),
                    }],
                },
            }],
            elements: vec![],
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(
            result.is_err(),
            "Self-reference without a prior definition should fail validation"
        );

        let err = result.unwrap_err();
        assert_eq!(err.len(), 1);
        assert_eq!(err[0].code(), Some(ErrorCode::E205));
    }

    #[test]
    fn test_self_reference_on_previously_defined_type_ok() {
        let diagram = FileAst {
            header: FileHeader::Diagram {
                kind: Spanned::new(DiagramKind::Component, Span::new(0..9)),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![
                // type MyType = Rectangle;
                TypeDefinition {
                    name: Spanned::new(Id::new("MyType"), Span::new(15..21)),
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(24..33))),
                        attributes: vec![],
                    },
                },
                // type MyType = MyType[fill_color="red"];
                TypeDefinition {
                    name: Spanned::new(Id::new("MyType"), Span::new(40..46)),
                    type_spec: TypeSpec {
                        type_name: Some(Spanned::new(Id::new("MyType"), Span::new(49..55))),
                        attributes: vec![Attribute {
                            name: Spanned::new("fill_color", Span::new(56..66)),
                            value: AttributeValue::String(Spanned::new(
                                "red".to_string(),
                                Span::new(68..73),
                            )),
                        }],
                    },
                },
            ],
            elements: vec![],
            imports: vec![],
        };

        let result = validate(&diagram);
        assert!(
            result.is_ok(),
            "Self-reference on a previously defined type should pass validation: {:?}",
            result.err()
        );
    }
}
