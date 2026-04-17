//! Desugaring pass over the Orrery AST.
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

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    mem,
    rc::Rc,
};

use indexmap::IndexMap;
use orrery_core::identifier::Id;

use crate::{
    builtin_types,
    parser_types::{
        Attribute, AttributeValue, ComponentContent, DiagramSource, Element, FileAst, FileHeader,
        Fragment, FragmentSection, Import, Note, TypeDefinition, TypeSpec,
    },
    span::Spanned,
};

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
    /// Fold a complete [`FileAst`].
    fn fold_file_ast(&mut self, file_ast: FileAst<'a>) -> FileAst<'a> {
        FileAst {
            header: self.fold_header(file_ast.header),
            import_decls: file_ast.import_decls,
            imports: self.fold_imports(file_ast.imports),
            type_definitions: self.fold_type_definitions(file_ast.type_definitions),
            elements: self.fold_elements(file_ast.elements),
        }
    }

    /// Fold a [`FileHeader`] by dispatching.
    fn fold_header(&mut self, header: FileHeader<'a>) -> FileHeader<'a> {
        match header {
            FileHeader::Diagram { kind, attributes } => FileHeader::Diagram {
                kind,
                attributes: self.fold_attributes(attributes),
            },
            library @ FileHeader::Library { .. } => library,
        }
    }

    /// Fold a [`FileAst`] behind an `Rc<RefCell<…>>`.
    ///
    /// When the `Rc` is uniquely owned the inner value is unwrapped without
    /// cloning; shared instances fall back to `clone()`.
    fn fold_rc_file_ast(&mut self, rc: Rc<RefCell<FileAst<'a>>>) -> Rc<RefCell<FileAst<'a>>> {
        let inner = match Rc::try_unwrap(rc) {
            Ok(cell) => cell.into_inner(),
            Err(rc) => rc.borrow().clone(),
        };
        Rc::new(RefCell::new(self.fold_file_ast(inner)))
    }

    /// Fold resolved [`Import`]s.
    fn fold_imports(&mut self, imports: Vec<Import<'a>>) -> Vec<Import<'a>> {
        imports
            .into_iter()
            .map(|import| Import {
                namespace: import.namespace,
                file_ast: self.fold_rc_file_ast(import.file_ast),
            })
            .collect()
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
                content,
            } => self.fold_component(name, display_name, type_spec, content),
            Element::Relation {
                source,
                target,
                relation_type,
                type_spec,
                label,
            } => self.fold_relation(source, target, relation_type, type_spec, label),
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
        content: ComponentContent<'a>,
    ) -> Element<'a> {
        Element::Component {
            name: self.fold_component_name(name),
            display_name: display_name.map(|dn| self.fold_display_name(dn)),
            type_spec: self.fold_component_type_spec(type_spec),
            content: self.fold_component_content(content),
        }
    }

    /// Folds a [`ComponentContent`] node.
    fn fold_component_content(&mut self, content: ComponentContent<'a>) -> ComponentContent<'a> {
        match content {
            ComponentContent::None => ComponentContent::None,
            ComponentContent::Scope(elements) => {
                ComponentContent::Scope(self.fold_elements(elements))
            }
            ComponentContent::Diagram(source) => {
                ComponentContent::Diagram(self.fold_diagram_source(source))
            }
        }
    }

    /// Folds a [`DiagramSource`] node.
    fn fold_diagram_source(&mut self, source: DiagramSource<'a>) -> DiagramSource<'a> {
        match source {
            DiagramSource::Inline(rc) => DiagramSource::Inline(self.fold_rc_file_ast(rc)),
            DiagramSource::Ref(id) => DiagramSource::Ref(id),
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

/// Desugaring pass for the Orrery AST.
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
pub struct Desugar<'a> {
    /// Tracks the current position in the component hierarchy for identifier
    /// qualification.
    path_stack: PathStack,
    /// Set of built-in type [`Id`]s (e.g., `Rectangle`, `Stroke`) that must not
    /// receive a namespace prefix during import qualification.
    builtin_types: HashSet<Id>,
    /// Lookup map from namespace [`Id`] → `Rc<RefCell<FileAst>>` for diagram imports.
    /// Built during `fold_file_ast` and used to resolve `DiagramSource::Ref`.
    embed_refs: HashMap<Id, Rc<RefCell<FileAst<'a>>>>,
}

impl<'a> Desugar<'a> {
    /// Creates a new [`Desugar`] folder instance.
    ///
    /// Initializes the built-in type set from [`builtin_types::defaults`] and
    /// starts with an empty `embed_refs` map and root-level path stack.
    fn new() -> Self {
        let type_ids = builtin_types::defaults()
            .into_iter()
            .map(|type_def| type_def.id())
            .collect();
        Self {
            path_stack: PathStack::new(),
            builtin_types: type_ids,
            embed_refs: HashMap::new(),
        }
    }

    /// Qualifies type references in a [`TypeSpec`] with a namespace prefix.
    ///
    /// Rewrites `type_name` to `namespace::type_name` unless the type is a
    /// built-in. Recurses into nested [`TypeSpec`]s found in attribute values.
    fn qualify_type_spec(&self, type_spec: &mut TypeSpec, namespace: Id) {
        // Skip built-ins (e.g., Rectangle, Stroke) — they are globally scoped
        // and must never receive a namespace prefix.
        if let Some(spanned_name) = type_spec.type_name.as_mut()
            && !self.builtin_types.contains(spanned_name)
        {
            *spanned_name = spanned_name.map(|name| namespace.create_nested(*name));
        }
        // Attribute values can themselves be TypeSpecs (e.g., `stroke=DashedLine`),
        // so we recurse into them to qualify nested type references.
        type_spec
            .attributes
            .iter_mut()
            .filter_map(|attr| attr.value.as_type_spec_mut().ok())
            .for_each(|inner| self.qualify_type_spec(inner, namespace));
    }

    /// Qualifies a [`TypeDefinition`] with a namespace prefix.
    ///
    /// Prefixes the definition's own name and delegates to
    /// [`qualify_type_spec`](Self::qualify_type_spec) for the body.
    fn qualify_type_definition(&self, type_def: &mut TypeDefinition, namespace: Id) {
        type_def.name = type_def.name.map(|name| namespace.create_nested(*name));
        self.qualify_type_spec(&mut type_def.type_spec, namespace);
    }

    /// Extracts [`TypeDefinition`]s from resolved imports, qualifying each with
    /// its namespace prefix when present.
    ///
    /// Consumes the `type_definitions` vec from each import's [`FileAst`] so
    /// that library ASTs are left empty after extraction.
    fn extract_type_definitions_from_imports(
        &self,
        imports: Vec<Import<'a>>,
    ) -> impl DoubleEndedIterator<Item = TypeDefinition<'a>> {
        imports.into_iter().flat_map(|import| {
            // `mem::take` drains the library AST's type_definitions in place,
            // so the shared Rc<RefCell<FileAst>> is left empty — safe because
            // library imports are consumed.
            let mut type_defs = mem::take(&mut import.file_ast.borrow_mut().type_definitions);
            // Glob imports (namespace = None) keep their original names.
            if let Some(ns) = import.namespace {
                for type_def in &mut type_defs {
                    self.qualify_type_definition(type_def, ns);
                }
            }
            type_defs
        })
    }

    /// Merges imported and local [`TypeDefinition`]s, deduplicating by name.
    ///
    /// Chains imported definitions before local ones and collects into an
    /// [`IndexMap`] keyed by type name. On duplicate keys `IndexMap` overwrites
    /// the value but preserves the original insertion order, giving "first
    /// position, last value" semantics:
    ///
    /// - **Position**: the earliest import determines where a name appears.
    /// - **Value**: the last definition (local > later import > earlier import)
    ///   wins, matching the "last writer wins" rule from the spec.
    fn merge_with_import_type_definitions(
        &self,
        type_defs: Vec<TypeDefinition<'a>>,
        imports: Vec<Import<'a>>,
    ) -> Vec<TypeDefinition<'a>> {
        let dedup: IndexMap<_, _> = self
            .extract_type_definitions_from_imports(imports)
            .chain(type_defs)
            .map(|type_def| (*type_def.name.inner(), type_def))
            .collect();
        dedup.into_values().collect()
    }
}

impl<'a> Folder<'a> for Desugar<'a> {
    /// Desugars a complete [`FileAst`] by flattening library imports into the
    /// root type-definition list and consuming diagram imports into `embed_refs`.
    fn fold_file_ast(&mut self, mut file_ast: FileAst<'a>) -> FileAst<'a> {
        // Save the parent's embed refs so nested fold_file_ast calls (from
        // inline embeds processed during fold_elements) don't clobber them.
        let saved_embed_refs = mem::take(&mut self.embed_refs);

        let imports = self.fold_imports(file_ast.imports);

        // Library imports export type definitions — extract and merge them into
        // the root scope. Diagram imports are consumed into the embed_refs
        // lookup and not forwarded downstream.
        let mut lib_imports = Vec::new();
        for import in imports {
            if import.file_ast.borrow().header.is_library() {
                lib_imports.push(import);
            } else {
                // Consume diagram import: move its Rc into embed_refs if
                // namespaced; drop it otherwise (glob imports don't create
                // embed references).
                if let Some(ns) = import.namespace {
                    self.embed_refs.insert(ns, import.file_ast);
                }
            }
        }

        // Merge must happen before fold_type_definitions so that imported types
        // are visible during any subsequent elaboration.
        let type_defs = self.merge_with_import_type_definitions(
            mem::take(&mut file_ast.type_definitions),
            lib_imports,
        );

        let header = self.fold_header(file_ast.header);
        let type_definitions = self.fold_type_definitions(type_defs);
        let elements = self.fold_elements(file_ast.elements);

        self.embed_refs = saved_embed_refs;

        FileAst {
            header,
            import_decls: file_ast.import_decls,
            imports: vec![],
            type_definitions,
            elements,
        }
    }

    /// Resolves a [`DiagramSource`] reference against the `embed_refs` lookup table.
    fn fold_diagram_source(&mut self, source: DiagramSource<'a>) -> DiagramSource<'a> {
        match source {
            DiagramSource::Inline(rc) => DiagramSource::Inline(self.fold_rc_file_ast(rc)),
            DiagramSource::Ref(id) => {
                if let Some(rc) = self.embed_refs.get(id.inner()) {
                    DiagramSource::Inline(Rc::clone(rc))
                } else {
                    // Leave unresolved — validate will report E204
                    DiagramSource::Ref(id)
                }
            }
        }
    }

    /// Folds a component with path-stack tracking for identifier resolution.
    ///
    /// Pushes the component's name onto the [`PathStack`] before folding its
    /// content (so nested identifiers are qualified under this component's
    /// namespace), then pops it afterward to restore the parent scope.
    fn fold_component(
        &mut self,
        name: Spanned<Id>,
        display_name: Option<Spanned<String>>,
        type_spec: TypeSpec<'a>,
        content: ComponentContent<'a>,
    ) -> Element<'a> {
        // Enter this component's namespace
        self.path_stack.push(*name.inner());

        // Process content (nested elements will be qualified with this component's path)
        let content = self.fold_component_content(content);

        // Exit this component's namespace
        self.path_stack.pop();

        Element::Component {
            name: self.fold_component_name(name),
            display_name: display_name.map(|dn| self.fold_display_name(dn)),
            type_spec: self.fold_component_type_spec(type_spec),
            content,
        }
    }

    /// Override fold_identifier to qualify identifier with current path
    fn fold_identifier(&mut self, identifier: Spanned<Id>) -> Spanned<Id> {
        let original_span = identifier.span();
        let qualified = self.path_stack.qualify(*identifier.inner());
        Spanned::new(qualified, original_span)
    }

    /// Folds a list of elements, desugaring [`ActivateBlock`](Element::ActivateBlock) in-place.
    ///
    /// Each `ActivateBlock { component, elements, type_spec }` is expanded into:
    /// 1. An [`Element::Activate`] statement for `component`.
    /// 2. The recursively folded inner `elements`.
    /// 3. An [`Element::Deactivate`] statement for `component`.
    ///
    /// All other element variants are delegated to [`fold_element`](Folder::fold_element).
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
                content,
            } => self.fold_component(name, display_name, type_spec, content),
            Element::Relation {
                source,
                target,
                relation_type,
                type_spec,
                label,
            } => self.fold_relation(source, target, relation_type, type_spec, label),
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
/// All desugaring happens in a single pass using the [`Desugar`] folder:
/// 1. `ActivateBlock` elements → explicit `activate`/`deactivate` statements
/// 2. Fragment keyword sugar syntax → base `Fragment` elements
/// 3. Component identifiers → fully qualified paths (e.g., "child" → "parent::child")
///
/// # Arguments
///
/// * `ast` - The root [`FileAst`] of the parsed file.
///
/// # Returns
///
/// A desugared [`FileAst`] tree.
pub fn desugar<'a>(ast: FileAst<'a>) -> FileAst<'a> {
    let mut folder = Desugar::new();
    folder.fold_file_ast(ast)
}

#[cfg(test)]
mod tests {
    use orrery_core::semantic::DiagramKind;

    use super::*;
    use crate::span::Span;

    // Test-only IdentityFolder for verifying identity transformations
    struct IdentityFolder;

    impl<'a> Folder<'a> for IdentityFolder {
        // Use default methods: identity behavior for all nodes
    }

    /// Helper to create a spanned value for testing
    fn spanned<T>(value: T) -> Spanned<T> {
        Spanned::new(value, Span::new(0..1))
    }

    /// Helper: create a library `FileAst` with the given type definitions.
    fn make_library_ast<'a>(type_defs: Vec<TypeDefinition<'a>>) -> FileAst<'a> {
        FileAst {
            header: FileHeader::Library {
                span: Span::new(0..1),
            },
            import_decls: vec![],
            type_definitions: type_defs,
            elements: vec![],
            imports: vec![],
        }
    }

    /// Helper: create a diagram `FileAst` with the given elements and type defs.
    fn make_diagram_ast<'a>(
        type_defs: Vec<TypeDefinition<'a>>,
        elements: Vec<Element<'a>>,
    ) -> FileAst<'a> {
        FileAst {
            header: FileHeader::Diagram {
                kind: spanned(DiagramKind::Component),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: type_defs,
            elements,
            imports: vec![],
        }
    }

    /// Helper: create a sequence diagram `FileAst` with the given elements.
    fn make_sequence_ast<'a>(elements: Vec<Element<'a>>) -> FileAst<'a> {
        FileAst {
            header: FileHeader::Diagram {
                kind: spanned(DiagramKind::Sequence),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements,
            imports: vec![],
        }
    }

    /// Helper: create an `Import` from the given namespace and file AST.
    fn make_import<'a>(namespace: Option<Id>, file_ast: FileAst<'a>) -> Import<'a> {
        Import {
            namespace,
            file_ast: Rc::new(RefCell::new(file_ast)),
        }
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
        // Create a simple diagram FileAst and fold it directly
        let file_ast = make_diagram_ast(vec![], vec![]);

        let mut folder = IdentityFolder;
        let result = folder.fold_file_ast(file_ast);

        match &result.header {
            FileHeader::Diagram { kind, attributes } => {
                assert_eq!(**kind, DiagramKind::Component);
                assert!(attributes.is_empty());
            }
            _ => panic!("Expected diagram header"),
        }
        assert!(result.type_definitions.is_empty());
        assert!(result.elements.is_empty());
    }

    #[test]
    fn test_identity_folder_preserves_attributes() {
        // Create a diagram FileAst with attributes and fold it directly
        let file_ast = FileAst {
            header: FileHeader::Diagram {
                kind: spanned(DiagramKind::Component),
                attributes: vec![
                    Attribute {
                        name: spanned("background_color"),
                        value: AttributeValue::String(spanned("#ffffff".to_string())),
                    },
                    Attribute {
                        name: spanned("layout_engine"),
                        value: AttributeValue::String(spanned("basic".to_string())),
                    },
                ],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![],
            imports: vec![],
        };

        let mut folder = IdentityFolder;
        let result = folder.fold_file_ast(file_ast);

        match &result.header {
            FileHeader::Diagram { attributes, .. } => {
                assert_eq!(attributes.len(), 2);
                assert_eq!(*attributes[0].name.inner(), "background_color");
                match &attributes[0].value {
                    AttributeValue::String(s) => assert_eq!(s.inner(), "#ffffff"),
                    _ => panic!("Expected string attribute"),
                }
            }
            _ => panic!("Expected Diagram header"),
        }
    }

    #[test]
    fn test_identity_folder_preserves_type_definitions() {
        // Create a diagram FileAst with type definitions and fold it directly
        let file_ast = make_diagram_ast(
            vec![TypeDefinition {
                name: spanned(Id::new("Database")),
                type_spec: TypeSpec {
                    type_name: Some(spanned(Id::new("Rectangle"))),
                    attributes: vec![Attribute {
                        name: spanned("fill_color"),
                        value: AttributeValue::String(spanned("lightblue".to_string())),
                    }],
                },
            }],
            vec![],
        );

        let mut folder = IdentityFolder;
        let result = folder.fold_file_ast(file_ast);

        assert_eq!(result.type_definitions.len(), 1);
        assert_eq!(*result.type_definitions[0].name.inner(), "Database");
        assert_eq!(
            *result.type_definitions[0]
                .type_spec
                .type_name
                .as_ref()
                .unwrap()
                .inner(),
            "Rectangle"
        );
        assert_eq!(result.type_definitions[0].type_spec.attributes.len(), 1);
    }

    #[test]
    fn test_identity_folder_preserves_components() {
        // Create a diagram FileAst with a component element and fold it directly
        let file_ast = make_diagram_ast(
            vec![],
            vec![Element::Component {
                name: spanned(Id::new("frontend")),
                display_name: Some(spanned("Frontend App".to_string())),
                type_spec: TypeSpec {
                    type_name: Some(spanned(Id::new("Rectangle"))),
                    attributes: vec![Attribute {
                        name: spanned("fill_color"),
                        value: AttributeValue::String(spanned("blue".to_string())),
                    }],
                },
                content: ComponentContent::None,
            }],
        );

        let mut folder = IdentityFolder;
        let result = folder.fold_file_ast(file_ast);

        assert_eq!(result.elements.len(), 1);
        match &result.elements[0] {
            Element::Component {
                name,
                display_name,
                type_spec,
                content,
            } => {
                assert_eq!(*name.inner(), "frontend");
                assert_eq!(display_name.as_ref().unwrap().inner(), "Frontend App");
                assert_eq!(*type_spec.type_name.as_ref().unwrap().inner(), "Rectangle");
                assert_eq!(type_spec.attributes.len(), 1);
                assert!(matches!(content, ComponentContent::None));
            }
            _ => panic!("Expected component element"),
        }
    }

    #[test]
    fn test_identity_folder_preserves_activate_block() {
        // Create a sequence diagram FileAst with an activate block and fold it directly
        let file_ast = make_sequence_ast(vec![Element::ActivateBlock {
            component: spanned(Id::new("user")),
            type_spec: TypeSpec::default(),
            elements: vec![Element::Relation {
                source: spanned(Id::new("user")),
                target: spanned(Id::new("server")),
                relation_type: spanned("->"),
                type_spec: TypeSpec::default(),
                label: Some(spanned("Request".to_string())),
            }],
        }]);

        let mut folder = IdentityFolder;
        let result = folder.fold_file_ast(file_ast);

        assert_eq!(result.elements.len(), 1);
        match &result.elements[0] {
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

    #[test]
    fn test_desugar_rewrites_activate_blocks() {
        // Create a sequence diagram FileAst with an activate block and desugar it
        let file_ast = make_sequence_ast(vec![Element::ActivateBlock {
            component: spanned(Id::new("user")),
            type_spec: TypeSpec::default(),
            elements: vec![Element::Relation {
                source: spanned(Id::new("user")),
                target: spanned(Id::new("server")),
                relation_type: spanned("->"),
                type_spec: TypeSpec::default(),
                label: Some(spanned("Request".to_string())),
            }],
        }]);

        let mut folder = Desugar::new();
        let result = folder.fold_file_ast(file_ast);

        assert_eq!(
            result.elements.len(),
            3,
            "Expected Activate, inner, Deactivate"
        );
        match &result.elements[0] {
            Element::Activate { component, .. } => {
                assert_eq!(*component.inner(), "user");
            }
            _ => panic!("Expected Activate element"),
        }
        match &result.elements[1] {
            Element::Relation { label, .. } => {
                assert_eq!(label.as_ref().unwrap().inner(), "Request");
            }
            _ => panic!("Expected inner Relation element"),
        }
        match &result.elements[2] {
            Element::Deactivate { component } => {
                assert_eq!(*component.inner(), "user");
            }
            _ => panic!("Expected Deactivate element"),
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
            content: ComponentContent::Scope(vec![
                Element::Component {
                    name: spanned(Id::new("child1")),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(spanned(Id::new("Oval"))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
                },
                Element::Component {
                    name: spanned(Id::new("child2")),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(spanned(Id::new("Rectangle"))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
                },
                Element::Relation {
                    source: spanned(Id::new("child1")),
                    target: spanned(Id::new("child2")),
                    relation_type: spanned("->"),
                    type_spec: TypeSpec::default(),
                    label: None,
                },
            ]),
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(parent_component);

        // Extract the relation from the result
        if let Element::Component { content, .. } = result {
            if let ComponentContent::Scope(elements) = content {
                let relation = elements.iter().find_map(|e| match e {
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
                panic!("Expected Scope content");
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
            content: ComponentContent::Scope(vec![Element::Component {
                name: spanned(Id::new("level2")),
                display_name: None,
                type_spec: TypeSpec {
                    type_name: Some(spanned(Id::new("Rectangle"))),
                    attributes: vec![],
                },
                content: ComponentContent::Scope(vec![
                    Element::Component {
                        name: spanned(Id::new("level3")),
                        display_name: None,
                        type_spec: TypeSpec {
                            type_name: Some(spanned(Id::new("Oval"))),
                            attributes: vec![],
                        },
                        content: ComponentContent::None,
                    },
                    Element::Relation {
                        source: spanned(Id::new("level3")),
                        target: spanned(Id::new("sibling")),
                        relation_type: spanned("->"),
                        type_spec: TypeSpec::default(),
                        label: None,
                    },
                ]),
            }]),
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(level1);

        // Navigate to the deeply nested relation
        if let Element::Component { content, .. } = result {
            if let ComponentContent::Scope(level1_elements) = content {
                if let Some(Element::Component { content, .. }) = level1_elements.first() {
                    if let ComponentContent::Scope(level2_elements) = content {
                        let relation = level2_elements.iter().find_map(|e| match e {
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
                        panic!("Expected Scope content at level2");
                    }
                } else {
                    panic!("Expected nested component");
                }
            } else {
                panic!("Expected Scope content at level1");
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
            content: ComponentContent::Scope(vec![
                Element::Component {
                    name: spanned(Id::new("child")),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(spanned(Id::new("Oval"))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
                },
                Element::Activate {
                    component: spanned(Id::new("child")),
                    type_spec: TypeSpec::default(),
                },
            ]),
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(parent_component);

        if let Element::Component { content, .. } = result {
            if let ComponentContent::Scope(elements) = content {
                let activate = elements.iter().find_map(|e| match e {
                    Element::Activate { component, .. } => Some(component),
                    _ => None,
                });

                if let Some(component) = activate {
                    assert_eq!(component.inner(), "parent::child");
                } else {
                    panic!("Expected to find Activate element");
                }
            } else {
                panic!("Expected Scope content");
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
            content: ComponentContent::Scope(vec![
                Element::Component {
                    name: spanned(Id::new("child")),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(spanned(Id::new("Oval"))),
                        attributes: vec![],
                    },
                    content: ComponentContent::None,
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
            ]),
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(parent_component);

        if let Element::Component { content, .. } = result {
            if let ComponentContent::Scope(elements) = content {
                let note = elements.iter().find_map(|e| match e {
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
                panic!("Expected Scope content");
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
            content: ComponentContent::Scope(vec![Element::Relation {
                source: Spanned::new(Id::new("child"), original_span),
                target: spanned(Id::new("other")),
                relation_type: spanned("->"),
                type_spec: TypeSpec::default(),
                label: None,
            }]),
        };

        let mut folder = Desugar::new();
        let result = folder.fold_element(parent_component);

        if let Element::Component { content, .. } = result {
            if let ComponentContent::Scope(elements) = content {
                if let Some(Element::Relation { source, .. }) = elements.first() {
                    // The identifier should be qualified, but the span should be preserved
                    assert_eq!(source.inner(), "parent::child");
                    assert_eq!(source.span(), original_span);
                } else {
                    panic!("Expected Relation element");
                }
            } else {
                panic!("Expected Scope content");
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

    #[test]
    fn test_import_basic_namespace_qualification() {
        let lib_ast = make_library_ast(vec![
            TypeDefinition {
                name: spanned(Id::new("Service")),
                type_spec: TypeSpec {
                    type_name: Some(spanned(Id::new("Rectangle"))),
                    attributes: vec![Attribute {
                        name: spanned("fill_color"),
                        value: AttributeValue::String(spanned("blue".to_string())),
                    }],
                },
            },
            TypeDefinition {
                name: spanned(Id::new("Database")),
                type_spec: TypeSpec {
                    type_name: Some(spanned(Id::new("Oval"))),
                    attributes: vec![Attribute {
                        name: spanned("fill_color"),
                        value: AttributeValue::String(spanned("green".to_string())),
                    }],
                },
            },
        ]);

        let mut main_ast = make_diagram_ast(vec![], vec![]);
        main_ast.imports = vec![make_import(Some(Id::new("styles")), lib_ast)];

        let mut folder = Desugar::new();
        let result = folder.fold_file_ast(main_ast);

        let names: Vec<String> = result
            .type_definitions
            .iter()
            .map(|td| td.name.inner().to_string())
            .collect();
        assert_eq!(names, ["styles::Service", "styles::Database"]);
    }

    #[test]
    fn test_import_base_type_qualification() {
        // Verify non-built-in base types qualified.
        let lib_ast = make_library_ast(vec![
            TypeDefinition {
                name: spanned(Id::new("DashedLine")),
                type_spec: TypeSpec {
                    type_name: Some(spanned(Id::new("Stroke"))),
                    attributes: vec![Attribute {
                        name: spanned("style"),
                        value: AttributeValue::String(spanned("dashed".to_string())),
                    }],
                },
            },
            TypeDefinition {
                name: spanned(Id::new("Service")),
                type_spec: TypeSpec {
                    type_name: Some(spanned(Id::new("DashedLine"))),
                    attributes: vec![Attribute {
                        name: spanned("fill_color"),
                        value: AttributeValue::String(spanned("blue".to_string())),
                    }],
                },
            },
        ]);

        let mut main_ast = make_diagram_ast(vec![], vec![]);
        main_ast.imports = vec![make_import(Some(Id::new("styles")), lib_ast)];

        let mut folder = Desugar::new();
        let result = folder.fold_file_ast(main_ast);

        let service = result
            .type_definitions
            .iter()
            .find(|td| *td.name.inner() == "styles::Service")
            .expect("styles::Service should exist");

        let type_name = service
            .type_spec
            .type_name
            .as_ref()
            .expect("type_name should be present");
        assert_eq!(**type_name, "styles::DashedLine");
    }

    #[test]
    fn test_import_builtin_type_preserved() {
        // Verify built-in types are NOT qualified.
        let lib_ast = make_library_ast(vec![TypeDefinition {
            name: spanned(Id::new("Service")),
            type_spec: TypeSpec {
                type_name: Some(spanned(Id::new("Rectangle"))),
                attributes: vec![Attribute {
                    name: spanned("fill_color"),
                    value: AttributeValue::String(spanned("blue".to_string())),
                }],
            },
        }]);

        let mut main_ast = make_diagram_ast(vec![], vec![]);
        main_ast.imports = vec![make_import(Some(Id::new("styles")), lib_ast)];

        let mut folder = Desugar::new();
        let result = folder.fold_file_ast(main_ast);

        let service = result
            .type_definitions
            .iter()
            .find(|td| *td.name.inner() == "styles::Service")
            .expect("styles::Service should exist");

        let type_name = service
            .type_spec
            .type_name
            .as_ref()
            .expect("type_name should be present");
        // Rectangle is built-in — it must NOT be qualified.
        assert_eq!(**type_name, "Rectangle");
    }

    #[test]
    fn test_import_nested_attribute_type_ref_qualification() {
        let lib_ast = make_library_ast(vec![
            TypeDefinition {
                name: spanned(Id::new("DashedLine")),
                type_spec: TypeSpec {
                    type_name: Some(spanned(Id::new("Stroke"))),
                    attributes: vec![Attribute {
                        name: spanned("style"),
                        value: AttributeValue::String(spanned("dashed".to_string())),
                    }],
                },
            },
            TypeDefinition {
                name: spanned(Id::new("Service")),
                type_spec: TypeSpec {
                    type_name: Some(spanned(Id::new("Rectangle"))),
                    attributes: vec![Attribute {
                        name: spanned("stroke"),
                        value: AttributeValue::TypeSpec(TypeSpec {
                            type_name: Some(spanned(Id::new("DashedLine"))),
                            attributes: vec![],
                        }),
                    }],
                },
            },
        ]);

        let mut main_ast = make_diagram_ast(vec![], vec![]);
        main_ast.imports = vec![make_import(Some(Id::new("styles")), lib_ast)];

        let mut folder = Desugar::new();
        let result = folder.fold_file_ast(main_ast);

        let service = result
            .type_definitions
            .iter()
            .find(|td| *td.name.inner() == "styles::Service")
            .expect("styles::Service should exist");

        // Rectangle is built-in, must stay as-is.
        assert_eq!(**service.type_spec.type_name.as_ref().unwrap(), "Rectangle");

        // The nested stroke attribute's TypeSpec type_name should be qualified.
        let stroke_attr = &service.type_spec.attributes[0];
        assert_eq!(*stroke_attr.name, "stroke");
        let inner = &stroke_attr
            .value
            .as_type_spec()
            .expect("attribute should be TypeSpec");
        let inner_name = inner
            .type_name
            .as_ref()
            .expect("inner type_name should be present");
        assert_eq!(**inner_name, "styles::DashedLine");
    }

    #[test]
    fn test_import_type_def_dedup_last_import_wins() {
        let lib_a = make_library_ast(vec![TypeDefinition {
            name: spanned(Id::new("Service")),
            type_spec: TypeSpec {
                type_name: Some(spanned(Id::new("Rectangle"))),
                attributes: vec![Attribute {
                    name: spanned("fill_color"),
                    value: AttributeValue::String(spanned("blue".to_string())),
                }],
            },
        }]);

        let lib_b = make_library_ast(vec![TypeDefinition {
            name: spanned(Id::new("Service")),
            type_spec: TypeSpec {
                type_name: Some(spanned(Id::new("Rectangle"))),
                attributes: vec![Attribute {
                    name: spanned("fill_color"),
                    value: AttributeValue::String(spanned("red".to_string())),
                }],
            },
        }]);

        let mut main_ast = make_diagram_ast(vec![], vec![]);
        main_ast.imports = vec![make_import(None, lib_a), make_import(None, lib_b)];

        let mut folder = Desugar::new();
        let result = folder.fold_file_ast(main_ast);

        // Only one type definition should remain (dedup).
        assert_eq!(result.type_definitions.len(), 1);

        // Last writer wins → fill_color = "red".
        assert_eq!(
            result.type_definitions[0].type_spec.attributes[0]
                .value
                .as_str()
                .expect("attribute should be String"),
            "red"
        )
    }

    #[test]
    fn test_import_local_type_def_overrides_imported() {
        let lib_ast = make_library_ast(vec![TypeDefinition {
            name: spanned(Id::new("Service")),
            type_spec: TypeSpec {
                type_name: Some(spanned(Id::new("Rectangle"))),
                attributes: vec![Attribute {
                    name: spanned("fill_color"),
                    value: AttributeValue::String(spanned("blue".to_string())),
                }],
            },
        }]);

        let mut main_ast = make_diagram_ast(
            vec![TypeDefinition {
                name: spanned(Id::new("Service")),
                type_spec: TypeSpec {
                    type_name: Some(spanned(Id::new("Rectangle"))),
                    attributes: vec![Attribute {
                        name: spanned("fill_color"),
                        value: AttributeValue::String(spanned("custom".to_string())),
                    }],
                },
            }],
            vec![],
        );
        main_ast.imports = vec![make_import(None, lib_ast)];

        let mut folder = Desugar::new();
        let result = folder.fold_file_ast(main_ast);

        assert_eq!(result.type_definitions.len(), 1);

        // Local wins → fill_color = "custom".
        assert_eq!(
            result.type_definitions[0].type_spec.attributes[0]
                .value
                .as_str()
                .expect("attribute should be String"),
            "custom"
        )
    }

    #[test]
    fn test_import_transitive_chained_namespaces() {
        // base library: type Color = Stroke[style="solid"]
        let base_ast = make_library_ast(vec![TypeDefinition {
            name: spanned(Id::new("Color")),
            type_spec: TypeSpec {
                type_name: Some(spanned(Id::new("Stroke"))),
                attributes: vec![Attribute {
                    name: spanned("style"),
                    value: AttributeValue::String(spanned("solid".to_string())),
                }],
            },
        }]);

        // ext library: imports base with namespace "base"
        let mut ext_ast = make_library_ast(vec![]);
        ext_ast.imports = vec![make_import(Some(Id::new("base")), base_ast)];

        // main diagram: imports ext with namespace "ext"
        let mut main_ast = make_diagram_ast(vec![], vec![]);
        main_ast.imports = vec![make_import(Some(Id::new("ext")), ext_ast)];

        let mut folder = Desugar::new();
        let result = folder.fold_file_ast(main_ast);

        // Should contain ext::base::Color
        let color = result
            .type_definitions
            .iter()
            .find(|td| *td.name.inner() == "ext::base::Color")
            .expect("ext::base::Color should exist");

        // Stroke is built-in, stays unqualified.
        let type_name = color
            .type_spec
            .type_name
            .as_ref()
            .expect("type_name should be present");
        assert_eq!(**type_name, "Stroke");
    }

    #[test]
    fn test_import_diagram_preserved_library_consumed() {
        // Verify library imports consumed, diagram imports consumed into embed refs.
        let lib_ast = make_library_ast(vec![TypeDefinition {
            name: spanned(Id::new("Card")),
            type_spec: TypeSpec {
                type_name: Some(spanned(Id::new("Rectangle"))),
                attributes: vec![],
            },
        }]);

        let diag_ast = make_diagram_ast(
            vec![TypeDefinition {
                name: spanned(Id::new("FlowBox")),
                type_spec: TypeSpec {
                    type_name: Some(spanned(Id::new("Rectangle"))),
                    attributes: vec![],
                },
            }],
            vec![],
        );

        let mut main_ast = make_diagram_ast(vec![], vec![]);
        main_ast.imports = vec![
            make_import(Some(Id::new("styles")), lib_ast),
            make_import(Some(Id::new("flow")), diag_ast),
        ];

        let mut folder = Desugar::new();
        let result = folder.fold_file_ast(main_ast);

        // Both library and diagram imports are consumed — none in output.
        assert!(result.imports.is_empty());

        // Library's types are merged into type_definitions.
        let has_card = result
            .type_definitions
            .iter()
            .any(|td| *td.name.inner() == "styles::Card");
        assert!(has_card, "styles::Card should be in type_definitions");

        // Diagram's types are NOT extracted into type_definitions.
        let has_flowbox = result
            .type_definitions
            .iter()
            .any(|td| *td.name.inner() == "FlowBox" || *td.name.inner() == "flow::FlowBox");
        assert!(
            !has_flowbox,
            "Diagram types should NOT be in type_definitions"
        );
    }

    #[test]
    fn test_import_no_namespace_flat() {
        // Verify glob import (no namespace) – types imported flat.
        let lib_ast = make_library_ast(vec![TypeDefinition {
            name: spanned(Id::new("Service")),
            type_spec: TypeSpec {
                type_name: Some(spanned(Id::new("Rectangle"))),
                attributes: vec![Attribute {
                    name: spanned("fill_color"),
                    value: AttributeValue::String(spanned("blue".to_string())),
                }],
            },
        }]);

        let mut main_ast = make_diagram_ast(vec![], vec![]);
        main_ast.imports = vec![make_import(None, lib_ast)];

        let mut folder = Desugar::new();
        let result = folder.fold_file_ast(main_ast);

        assert_eq!(result.type_definitions.len(), 1);

        // With namespace: None, the type name stays bare "Service" (no prefix).
        assert_eq!(*result.type_definitions[0].name.inner(), "Service");

        // Built-in type_name stays as-is.
        let type_name = result.type_definitions[0]
            .type_spec
            .type_name
            .as_ref()
            .expect("type_name should be present");
        assert_eq!(**type_name, "Rectangle");
    }

    #[test]
    fn test_embed_ref_resolved_to_inline() {
        let imported_ast = make_diagram_ast(vec![], vec![]);
        let imported_rc = Rc::new(RefCell::new(imported_ast));

        // Create the root file with a diagram import and a component that embeds it by ref
        let root_ast = FileAst {
            header: FileHeader::Diagram {
                kind: spanned(DiagramKind::Component),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![Element::Component {
                name: spanned(Id::new("auth_box")),
                display_name: None,
                type_spec: TypeSpec {
                    type_name: Some(spanned(Id::new("Rectangle"))),
                    attributes: vec![],
                },
                content: ComponentContent::Diagram(DiagramSource::Ref(spanned(Id::new(
                    "auth_flow",
                )))),
            }],
            imports: vec![Import {
                namespace: Some(Id::new("auth_flow")),
                file_ast: Rc::clone(&imported_rc),
            }],
        };

        let result = desugar(root_ast);

        // The component should now have DiagramSource::Inline
        match &result.elements[0] {
            Element::Component { content, .. } => match content {
                ComponentContent::Diagram(DiagramSource::Inline(rc)) => {
                    // Verify it's a diagram (not a library)
                    assert!(matches!(rc.borrow().header, FileHeader::Diagram { .. }));
                }
                other => panic!("Expected DiagramSource::Inline, got: {:?}", other),
            },
            other => panic!("Expected Component element, got: {:?}", other),
        }
    }

    #[test]
    fn test_embed_ref_unresolved_stays_as_ref() {
        let root_ast = FileAst {
            header: FileHeader::Diagram {
                kind: spanned(DiagramKind::Component),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![Element::Component {
                name: spanned(Id::new("auth_box")),
                display_name: None,
                type_spec: TypeSpec {
                    type_name: Some(spanned(Id::new("Rectangle"))),
                    attributes: vec![],
                },
                content: ComponentContent::Diagram(DiagramSource::Ref(spanned(Id::new(
                    "nonexistent",
                )))),
            }],
            imports: vec![],
        };

        let result = desugar(root_ast);

        match &result.elements[0] {
            Element::Component { content, .. } => {
                assert!(
                    matches!(content, ComponentContent::Diagram(DiagramSource::Ref(_))),
                    "Expected unresolved DiagramSource::Ref, got: {:?}",
                    content
                );
            }
            other => panic!("Expected Component element, got: {:?}", other),
        }
    }

    #[test]
    fn test_embed_ref_does_not_match_glob_import() {
        let imported_ast = make_diagram_ast(vec![], vec![]);
        let imported_rc = Rc::new(RefCell::new(imported_ast));

        let root_ast = FileAst {
            header: FileHeader::Diagram {
                kind: spanned(DiagramKind::Component),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![Element::Component {
                name: spanned(Id::new("box")),
                display_name: None,
                type_spec: TypeSpec {
                    type_name: Some(spanned(Id::new("Rectangle"))),
                    attributes: vec![],
                },
                content: ComponentContent::Diagram(DiagramSource::Ref(spanned(Id::new("styles")))),
            }],
            imports: vec![Import {
                namespace: None, // glob import — no namespace
                file_ast: imported_rc,
            }],
        };

        let result = desugar(root_ast);

        match &result.elements[0] {
            Element::Component { content, .. } => {
                assert!(
                    matches!(content, ComponentContent::Diagram(DiagramSource::Ref(_))),
                    "Glob import should not resolve embed ref, got: {:?}",
                    content
                );
            }
            other => panic!("Expected Component element, got: {:?}", other),
        }
    }

    #[test]
    fn test_embed_ref_survives_inline_embed_in_same_file() {
        // Regression: an inline embed's fold_file_ast must not clobber the
        // parent file's embed_refs map. If it did, the ref embed that follows
        // would fail to resolve.
        let imported_ast = make_diagram_ast(vec![], vec![]);
        let imported_rc = Rc::new(RefCell::new(imported_ast));

        let inline_ast = make_sequence_ast(vec![]);

        let root_ast = FileAst {
            header: FileHeader::Diagram {
                kind: spanned(DiagramKind::Component),
                attributes: vec![],
            },
            import_decls: vec![],
            type_definitions: vec![],
            elements: vec![
                // First: inline embed — its fold_file_ast must not clear embed_refs
                Element::Component {
                    name: spanned(Id::new("box1")),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(spanned(Id::new("Rectangle"))),
                        attributes: vec![],
                    },
                    content: ComponentContent::Diagram(DiagramSource::Inline(Rc::new(
                        RefCell::new(inline_ast),
                    ))),
                },
                // Second: ref embed — must still resolve
                Element::Component {
                    name: spanned(Id::new("box2")),
                    display_name: None,
                    type_spec: TypeSpec {
                        type_name: Some(spanned(Id::new("Rectangle"))),
                        attributes: vec![],
                    },
                    content: ComponentContent::Diagram(DiagramSource::Ref(spanned(Id::new(
                        "imported",
                    )))),
                },
            ],
            imports: vec![Import {
                namespace: Some(Id::new("imported")),
                file_ast: Rc::clone(&imported_rc),
            }],
        };

        let result = desugar(root_ast);

        // box2's ref should have been resolved to Inline
        match &result.elements[1] {
            Element::Component { name, content, .. } => {
                assert_eq!(*name.inner(), "box2");
                assert!(
                    matches!(content, ComponentContent::Diagram(DiagramSource::Inline(_))),
                    "Embed ref after inline embed should resolve, got: {:?}",
                    content
                );
            }
            other => panic!("Expected Component element, got: {:?}", other),
        }
    }

    #[test]
    fn test_import_type_def_dedup_preserves_dependency_order() {
        // When two glob imports re-export the same type, the surviving
        // definition must appear at the *first* position where that name was
        // seen. Otherwise a type that depends on it (defined between the two
        // occurrences) ends up before its dependency.
        //
        // Scenario:
        //   a.orr  (library): type Service = Rectangle;
        //   b.orr  (library): import "a"::*;  type Critical = Service;
        //   diag.orr: import "b"::*;  import "a"::*;
        //
        // After desugaring b, its type_definitions = [Service, Critical].
        // In diag the chain is: [Service(via b), Critical(via b), Service(from a)]
        //
        // Expected result: [Service, Critical]  (Service first — Critical depends on it)

        // a.orr: type Service = Rectangle
        let a_ast = make_library_ast(vec![TypeDefinition {
            name: spanned(Id::new("Service")),
            type_spec: TypeSpec {
                type_name: Some(spanned(Id::new("Rectangle"))),
                attributes: vec![],
            },
        }]);

        // b.orr: import "a"::*;  type Critical = Service;
        // After b's own desugaring its type_definitions are [Service, Critical].
        let b_ast = make_library_ast(vec![
            TypeDefinition {
                name: spanned(Id::new("Service")),
                type_spec: TypeSpec {
                    type_name: Some(spanned(Id::new("Rectangle"))),
                    attributes: vec![],
                },
            },
            TypeDefinition {
                name: spanned(Id::new("Critical")),
                type_spec: TypeSpec {
                    type_name: Some(spanned(Id::new("Service"))),
                    attributes: vec![],
                },
            },
        ]);

        // diag.orr: import "b"::*;  import "a"::*;
        let mut main_ast = make_diagram_ast(vec![], vec![]);
        main_ast.imports = vec![
            make_import(None, b_ast), // glob import of b
            make_import(None, a_ast), // glob import of a
        ];

        let mut folder = Desugar::new();
        let result = folder.fold_file_ast(main_ast);

        // Should have exactly two types after dedup.
        assert_eq!(result.type_definitions.len(), 2);

        // Service must come first (Critical depends on it).
        assert_eq!(*result.type_definitions[0].name.inner(), "Service");
        assert_eq!(*result.type_definitions[1].name.inner(), "Critical");
    }
}
