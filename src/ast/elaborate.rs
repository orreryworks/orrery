use super::{elaborate_types as types, parser_types};
use crate::{
    ast::span::{Span, Spanned},
    color::Color,
    config::AppConfig,
    draw,
    error::ElaborationDiagnosticError,
    identifier::Id,
};
use log::{debug, info, trace};
use std::{collections::HashMap, rc::Rc, str::FromStr};

/// Type alias for Result with ElaborationDiagnosticError as the error type
type EResult<T> = Result<T, ElaborationDiagnosticError>;

pub struct Builder<'a> {
    cfg: &'a AppConfig,
    default_arrow_type: Rc<types::TypeDefinition>,
    type_definitions: Vec<Rc<types::TypeDefinition>>,
    type_definition_map: HashMap<Id, Rc<types::TypeDefinition>>,
    _phantom: std::marker::PhantomData<&'a str>, // Use PhantomData to maintain the lifetime parameter
}

impl<'a> Builder<'a> {
    pub fn new(cfg: &'a AppConfig, _source: &'a str) -> Self {
        let default_arrow_type = types::TypeDefinition::default_arrow_definition();
        // We keep the source parameter for backward compatibility but don't store it anymore
        let type_definitions = types::TypeDefinition::defaults(&default_arrow_type);
        let type_definition_map = type_definitions
            .iter()
            .map(|def| (def.id(), Rc::clone(def)))
            .collect();

        Self {
            cfg,
            default_arrow_type,
            type_definitions,
            type_definition_map,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn build(mut self, diag: &Spanned<parser_types::Element<'a>>) -> EResult<types::Diagram> {
        debug!("Building elaborated diagram");
        match diag.inner() {
            parser_types::Element::Diagram(diag) => {
                info!("Processing diagram of kind: {}", diag.kind);
                trace!("Type definitions: {:?}", diag.type_definitions);
                trace!("Elements count: {}", diag.elements.len());

                // Update type definitions
                debug!("Updating type definitions");
                self.update_type_direct_definitions(&diag.type_definitions)?;

                // Determine diagram kind
                let kind = self.determine_diagram_kind(&diag.kind)?;

                // Build block from elements
                debug!("Building block from elements");
                let block = self.build_block_from_elements(&diag.elements, None, kind)?;

                // Convert block to scope
                let scope = match block {
                    types::Block::None => {
                        debug!("Empty block, using default scope");
                        types::Scope::default()
                    }
                    types::Block::Scope(scope) => {
                        debug!(
                            elements_len = scope.elements().len();
                            "Using scope from block",
                        );
                        scope
                    }
                    types::Block::Diagram(_) => {
                        return Err(ElaborationDiagnosticError::from_span(
                            "Nested diagram not allowed".to_string(),
                            diag.kind.span(),
                            "invalid diagram structure",
                            Some("Diagrams cannot be nested inside other diagrams".to_string()),
                        ));
                    }
                };

                let (layout_engine, background_color) =
                    self.extract_diagram_attributes(kind, &diag.attributes)?;

                info!(kind:?; "Diagram elaboration completed successfully");
                Ok(types::Diagram::new(
                    kind,
                    scope,
                    layout_engine,
                    background_color,
                ))
            }
            _ => Err(ElaborationDiagnosticError::from_span(
                "Invalid element, expected Diagram".to_string(),
                diag.span(),
                "invalid element",
                None,
            )),
        }
    }

    // TODO: Change error type so it would not accept a span.
    fn insert_type_definition(
        &mut self,
        type_def: types::TypeDefinition,
        span: Span,
    ) -> EResult<Rc<types::TypeDefinition>> {
        let id = type_def.id();
        let type_def = Rc::new(type_def);
        self.type_definitions.push(Rc::clone(&type_def));

        // Check if the type already exists
        if self
            .type_definition_map
            .insert(id, Rc::clone(&type_def))
            .is_none()
        {
            Ok(type_def)
        } else {
            // We could use a span here if we tracked where the duplicate was defined
            // For now, we use a simple error since we don't store that information
            Err(ElaborationDiagnosticError::from_span(
                format!("Type definition '{}' already exists", type_def.id()),
                span,
                "duplicate type definition",
                None,
            ))
        }
    }

    fn update_type_direct_definitions(
        &mut self,
        type_definitions: &Vec<parser_types::TypeDefinition<'a>>,
    ) -> EResult<()> {
        for type_def in type_definitions {
            let base_type_name = Id::new(&type_def.base_type);
            let base = self
                .type_definition_map
                .get(&base_type_name)
                .ok_or_else(|| {
                    // Create a rich diagnostic error with source location information
                    let type_name = &type_def.base_type;
                    self.create_undefined_type_error(
                        type_name,
                        &format!("Base type '{type_name}' not found"),
                    )
                })?;

            // Try to create the type definition
            match types::TypeDefinition::from_base(
                Id::new(&type_def.name),
                base,
                &type_def.attributes,
            ) {
                Ok(new_type_def) => {
                    self.insert_type_definition(new_type_def, type_def.span())?;
                }
                Err(err) => {
                    // Wrap the error with location information for attribute errors
                    return Err(ElaborationDiagnosticError::from_span(
                        format!("Invalid type definition: {err}"),
                        type_def.span(),
                        "type definition error",
                        Some("Check attribute types and values for errors".to_string()),
                    ));
                }
            }
        }
        Ok(())
    }

    fn build_diagram_from_parser(
        &mut self,
        diag: &parser_types::Element,
    ) -> EResult<types::Diagram> {
        match diag {
            parser_types::Element::Diagram(diag) => {
                // Determine diagram kind for embedded diagram
                let embedded_kind = self.determine_diagram_kind(&diag.kind)?;
                // Create a block from the diagram elements
                let block = self.build_block_from_elements(&diag.elements, None, embedded_kind)?;
                let scope = match block {
                    types::Block::None => types::Scope::default(),
                    types::Block::Scope(scope) => scope,
                    types::Block::Diagram(_) => {
                        return Err(ElaborationDiagnosticError::from_span(
                            "Nested diagram not allowed".to_string(),
                            diag.kind.span(),
                            "invalid nesting",
                            Some("Diagrams cannot be nested inside other diagrams".to_string()),
                        ));
                    }
                };

                let kind = self.determine_diagram_kind(&diag.kind)?;
                let (layout_engine, background_color) =
                    self.extract_diagram_attributes(kind, &diag.attributes)?;

                Ok(types::Diagram::new(
                    kind,
                    scope,
                    layout_engine,
                    background_color,
                ))
            }
            _ => Err(ElaborationDiagnosticError::from_span(
                "Invalid element, expected Diagram".to_string(),
                diag.span(),
                "invalid element",
                None,
            )),
        }
    }

    fn build_diagram_from_embedded_diagram(
        &mut self,
        element: &parser_types::Element,
    ) -> EResult<types::Diagram> {
        if let parser_types::Element::Diagram(diag) = element {
            // Determine diagram kind for embedded diagram
            let embedded_kind = self.determine_diagram_kind(&diag.kind)?;
            // Create a block from the diagram elements
            let block = self.build_block_from_elements(&diag.elements, None, embedded_kind)?;
            let scope = match block {
                types::Block::None => types::Scope::default(),
                types::Block::Scope(scope) => scope,
                types::Block::Diagram(_) => {
                    return Err(ElaborationDiagnosticError::from_span(
                        "Nested diagram not allowed".to_string(),
                        diag.kind.span(),
                        "invalid nesting",
                        Some("Diagrams cannot be nested inside other diagrams".to_string()),
                    ));
                }
            };

            let kind = self.determine_diagram_kind(&diag.kind)?;
            let (layout_engine, background_color) =
                self.extract_diagram_attributes(kind, &diag.attributes)?;

            Ok(types::Diagram::new(
                kind,
                scope,
                layout_engine,
                background_color,
            ))
        } else {
            Err(ElaborationDiagnosticError::from_span(
                "Expected diagram element".to_string(),
                element.span(),
                "invalid element",
                None,
            ))
        }
    }

    fn build_block_from_elements(
        &mut self,
        parser_elements: &[parser_types::Element],
        parent_id: Option<Id>,
        diagram_kind: types::DiagramKind,
    ) -> EResult<types::Block> {
        if parser_elements.is_empty() {
            Ok(types::Block::None)
        } else if let parser_types::Element::Diagram { .. } = &parser_elements[0] {
            // This case happens when a diagram is the first element in a block
            Ok(types::Block::Diagram(
                self.build_diagram_from_parser(&parser_elements[0])?,
            ))
        } else {
            // Check to make sure no diagrams are mixed with other elements
            for parser_elm in parser_elements {
                if let parser_types::Element::Diagram(diag) = parser_elm {
                    // If we found a diagram mixed with other elements, provide a rich error
                    return Err(ElaborationDiagnosticError::from_span(
                        "Diagram cannot share scope with other elements".to_string(),
                        diag.kind.span(), // Use the diagram kind span as the error location
                        "invalid nesting",
                        Some(
                            "A diagram declaration must be the only element in its scope"
                                .to_string(),
                        ),
                    ));
                }
            }

            // If no diagrams were found mixed with other elements, build the scope
            Ok(types::Block::Scope(self.build_scope_from_elements(
                parser_elements,
                parent_id,
                diagram_kind,
            )?))
        }
    }

    fn build_scope_from_elements(
        &mut self,
        parser_elements: &[parser_types::Element],
        parent_id: Option<Id>,
        diagram_kind: types::DiagramKind,
    ) -> EResult<types::Scope> {
        let mut elements = Vec::new();

        for parser_elm in parser_elements {
            let element = match parser_elm {
                parser_types::Element::Component {
                    name,
                    display_name,
                    type_name,
                    attributes,
                    nested_elements,
                } => self.build_component_element(
                    name,
                    display_name,
                    type_name,
                    attributes,
                    nested_elements,
                    parent_id,
                    parser_elm,
                    diagram_kind,
                )?,
                parser_types::Element::Relation {
                    source,
                    target,
                    relation_type,
                    type_spec,
                    label,
                } => self.build_relation_element(
                    source,
                    target,
                    relation_type,
                    type_spec,
                    label,
                    parent_id,
                )?,
                parser_types::Element::Diagram(_) => {
                    // This should never happen since we already filtered out invalid elements
                    return Err(ElaborationDiagnosticError::from_span(
                        "Invalid element type".to_string(),
                        parser_elm.span(),
                        "invalid element type",
                        None,
                    ));
                }
                parser_types::Element::ActivateBlock { .. } => {
                    unreachable!(
                        "ActivateBlock should have been desugared into explicit activate/deactivate statements before elaboration"
                    );
                }
                parser_types::Element::Activate { component } => {
                    self.build_activate_element(component, parent_id, diagram_kind)?
                }
                parser_types::Element::Deactivate { component } => {
                    self.build_deactivate_element(component, parent_id, diagram_kind)?
                }
                parser_types::Element::Fragment(fragment) => {
                    self.build_fragment_element(fragment, parent_id, diagram_kind)?
                }
            };
            elements.push(element);
        }
        Ok(types::Scope::new(elements))
    }

    /// Builds a component element from parser data
    fn build_component_element(
        &mut self,
        name: &Spanned<&str>,
        display_name: &Option<Spanned<String>>,
        type_name: &Spanned<&str>,
        attributes: &[parser_types::Attribute],
        nested_elements: &[parser_types::Element],
        parent_id: Option<Id>,
        parser_elm: &parser_types::Element,
        diagram_kind: types::DiagramKind,
    ) -> EResult<types::Element> {
        let node_id = self.create_type_id(parent_id, name.inner());

        let type_def = self
            .build_type_definition(type_name, attributes)
            .map_err(|_| {
                self.create_undefined_type_error(
                    name,
                    &format!("Unknown type '{type_name}' for component '{name}'"),
                )
            })?;

        // Check if this shape supports content before processing nested elements
        if !nested_elements.is_empty()
            && !type_def
                .shape_definition_rc()
                .is_ok_and(|s| s.supports_content())
        {
            return Err(ElaborationDiagnosticError::from_span(
                format!("Shape type '{type_name}' does not support nested content"),
                parser_elm.span(),
                "content not supported",
                Some(format!(
                    "The '{type_name}' shape is content-free and cannot contain nested elements or embedded diagrams"
                )),
            ));
        }

        // Check if there's a nested diagram element
        let block = if nested_elements.len() == 1
            && matches!(&nested_elements[0], parser_types::Element::Diagram(_))
        {
            // Handle a single diagram element specially
            let elaborated_diagram =
                self.build_diagram_from_embedded_diagram(&nested_elements[0])?;
            types::Block::Diagram(elaborated_diagram)
        } else {
            // Process regular nested elements
            self.build_block_from_elements(nested_elements, Some(node_id), diagram_kind)?
        };

        let node = types::Node::new(
            node_id,
            name.to_string(),
            display_name.as_ref().map(|n| n.to_string()),
            block,
            type_def,
        );

        Ok(types::Element::Node(node))
    }

    /// Builds a relation element from parser data
    fn build_relation_element(
        &mut self,
        source: &Spanned<String>,
        target: &Spanned<String>,
        relation_type: &Spanned<&str>,
        type_spec: &Option<parser_types::RelationTypeSpec>,
        label: &Option<Spanned<String>>,
        parent_id: Option<Id>,
    ) -> EResult<types::Element> {
        // Extract relation type definition from type_spec
        let relation_type_def = self.build_relation_type_definition_from_spec(type_spec)?;

        // Create source and target IDs based on parent context if present
        let source_id = self.create_type_id(parent_id, source.inner());
        let target_id = self.create_type_id(parent_id, target.inner());

        let arrow_direction = draw::ArrowDirection::from_str(relation_type).map_err(|_| {
            ElaborationDiagnosticError::from_span(
                format!("Invalid arrow direction '{relation_type}'"),
                relation_type.span(),
                "invalid direction",
                Some("Arrow direction must be '->', '<-', '<->', or '-'".to_string()),
            )
        })?;

        Ok(types::Element::Relation(types::Relation::new(
            source_id,
            target_id,
            arrow_direction,
            label.as_ref().map(|l| l.to_string()),
            relation_type_def,
        )))
    }

    /// Builds an activate element from parser data
    fn build_activate_element(
        &mut self,
        component: &Spanned<String>,
        parent_id: Option<Id>,
        diagram_kind: types::DiagramKind,
    ) -> EResult<types::Element> {
        // Only allow activate in sequence diagrams
        if diagram_kind != types::DiagramKind::Sequence {
            return Err(ElaborationDiagnosticError::from_span(
                "Activate statements are only supported in sequence diagrams".to_string(),
                component.span(),
                "activate not allowed here",
                Some(
                    "Activate statements are used for temporal grouping in sequence diagrams"
                        .to_string(),
                ),
            ));
        }

        // Create Id for the component being activated
        let component_id = self.create_type_id(parent_id, component.inner());

        Ok(types::Element::Activate(component_id))
    }

    /// Builds a deactivate element from parser data
    fn build_deactivate_element(
        &mut self,
        component: &Spanned<String>,
        parent_id: Option<Id>,
        diagram_kind: types::DiagramKind,
    ) -> EResult<types::Element> {
        // Only allow deactivate in sequence diagrams
        if diagram_kind != types::DiagramKind::Sequence {
            return Err(ElaborationDiagnosticError::from_span(
                "Deactivate statements are only supported in sequence diagrams".to_string(),
                component.span(),
                "deactivate not allowed here",
                Some(
                    "Deactivate statements are used for temporal grouping in sequence diagrams"
                        .to_string(),
                ),
            ));
        }

        // Create Id for the component being deactivated
        let component_id = self.create_type_id(parent_id, component.inner());

        Ok(types::Element::Deactivate(component_id))
    }

    /// Builds a fragment element from parser data
    fn build_fragment_element(
        &mut self,
        fragment: &parser_types::Fragment,
        parent_id: Option<Id>,
        diagram_kind: types::DiagramKind,
    ) -> EResult<types::Element> {
        // Only allow fragments in sequence diagrams
        if diagram_kind != types::DiagramKind::Sequence {
            return Err(ElaborationDiagnosticError::from_span(
                "Fragment blocks are only supported in sequence diagrams".to_string(),
                fragment.span(),
                "fragment not allowed here",
                Some("Fragment blocks are used for grouping in sequence diagrams".to_string()),
            ));
        }

        let type_name = "Fragment"; // TODO: Hardcoded for now.

        // Build the type definition for this fragment
        let type_def = self
            .build_type_definition(
                &Spanned::new(type_name, fragment.operation.span()),
                &fragment.attributes,
            )
            .map_err(|_| {
                ElaborationDiagnosticError::from_span(
                    format!(
                        "Unknown fragment type '{type_name}' for operation '{}'",
                        fragment.operation.inner()
                    ),
                    fragment.operation.span(),
                    "unknown fragment type",
                    Some("Fragment types must be defined in the type system".to_string()),
                )
            })?;

        let mut sections = Vec::new();
        for parser_section in &fragment.sections {
            let scope =
                self.build_scope_from_elements(&parser_section.elements, parent_id, diagram_kind)?;
            let elements_vec = scope.elements().to_vec();

            sections.push(types::FragmentSection::new(
                parser_section.title.as_ref().map(|t| t.inner().to_string()),
                elements_vec,
            ));
        }

        Ok(types::Element::Fragment(types::Fragment::new(
            fragment.operation.inner().to_string(),
            sections,
            type_def,
        )))
    }

    /// Build a relation type definition from a relation type specification
    fn build_relation_type_definition_from_spec(
        &mut self,
        type_spec: &Option<parser_types::RelationTypeSpec>,
    ) -> EResult<Rc<types::TypeDefinition>> {
        match type_spec {
            Some(spec) => {
                match (&spec.type_name, &spec.attributes) {
                    // Direct attributes without type name: [color="red", width="3"]
                    (None, attrs) => {
                        let arrow_def = self.create_arrow_definition_from_attributes(attrs)?;
                        Ok(Rc::new(arrow_def))
                    }
                    // Type reference with additional attributes: [RedArrow; width="5"]
                    (Some(type_name), attributes) => {
                        self.build_type_definition(type_name, attributes)
                    }
                }
            }
            None => Ok(Rc::clone(&self.default_arrow_type)),
        }
    }

    fn build_type_definition(
        &mut self,
        type_name: &Spanned<&str>,
        attributes: &[parser_types::Attribute],
    ) -> EResult<Rc<types::TypeDefinition>> {
        // Look up the base type
        let type_id = Id::new(type_name);
        let Some(base) = self.type_definition_map.get(&type_id) else {
            return Err(
                self.create_undefined_type_error(type_name, &format!("Unknown type '{type_name}'"))
            );
        };

        // If there are no attributes, just return the base type
        if attributes.is_empty() {
            return Ok(Rc::clone(base));
        }

        // Otherwise, create a new anonymous type based on the base type
        let id = Id::from_anonymous(self.type_definition_map.len());
        match types::TypeDefinition::from_base(id, base, attributes) {
            Ok(new_type) => self.insert_type_definition(new_type, type_name.span()),
            Err(err) => Err(self.create_undefined_type_error(
                type_name,
                &format!("Error creating type based on '{type_name}': {err}"),
            )),
        }
    }

    /// Determines the diagram kind based on the input string.
    fn determine_diagram_kind(&self, kind_str: &Spanned<&str>) -> EResult<types::DiagramKind> {
        match *kind_str.inner() {
            "sequence" => Ok(types::DiagramKind::Sequence),
            "component" => Ok(types::DiagramKind::Component),
            _ => Err(ElaborationDiagnosticError::from_span(
                format!("Invalid diagram kind: '{kind_str}'"),
                kind_str.span(),
                "unsupported diagram type",
                Some("Supported diagram types are: 'component', 'sequence'".to_string()),
            )),
        }
    }

    /// Creates an Id from a string name, considering the parent context if available
    ///
    /// This function is used for both component names (simple identifiers) and relation
    /// source/target names (which may be nested identifiers like "frontend::app" created
    /// by joining parts with "::").
    fn create_type_id(&self, parent_id: Option<Id>, name: &str) -> Id {
        parent_id.map_or_else(|| Id::new(name), |parent| parent.create_nested(name))
    }

    /// Creates a standardized error for undefined type situations
    fn create_undefined_type_error(
        &self,
        span: &Spanned<&str>,
        message: &str,
    ) -> ElaborationDiagnosticError {
        ElaborationDiagnosticError::from_span(
            message.to_string(),
            span.span(),
            "undefined type",
            Some(format!(
                "Type '{}' must be a built-in type or defined with a 'type' statement before it can be used as a base type",
                span.inner()
            )),
        )
    }

    /// Parses relation attributes and creates an ArrowDefinition
    fn create_arrow_definition_from_attributes(
        &self,
        attributes: &Vec<parser_types::Attribute<'_>>,
    ) -> EResult<types::TypeDefinition> {
        let id = Id::from_anonymous(self.type_definition_map.len());
        types::TypeDefinition::from_base(id, &self.default_arrow_type, attributes)
    }

    /// Extract diagram attributes (layout engine and background color)
    fn extract_diagram_attributes(
        &self,
        kind: types::DiagramKind,
        attrs: &Vec<parser_types::Attribute<'_>>,
    ) -> EResult<(types::LayoutEngine, Option<Color>)> {
        // Set the default layout engine based on the diagram kind and config
        let mut layout_engine = match kind {
            types::DiagramKind::Component => self.cfg.layout.component,
            types::DiagramKind::Sequence => self.cfg.layout.sequence,
        };

        let mut background_color = None;

        // Single pass through the attributes to extract both values
        for attr in attrs {
            match *attr.name {
                "layout_engine" => {
                    layout_engine = Self::determine_layout_engine(attr)?;
                }
                "background_color" => {
                    let color = Self::extract_background_color(attr)?;
                    background_color = Some(color);
                }
                _ => {
                    return Err(ElaborationDiagnosticError::from_span(
                        format!("Unsupported diagram attribute '{}'", attr.name),
                        attr.span(),
                        "unsupported attribute",
                        None,
                    ));
                }
            }
        }

        Ok((layout_engine, background_color))
    }

    /// Extract background color from an attribute
    fn extract_background_color(color_attr: &parser_types::Attribute<'_>) -> EResult<Color> {
        let color_str = color_attr.value.as_str().map_err(|err| {
            ElaborationDiagnosticError::from_span(
                err.to_string(),
                color_attr.value.span(),
                "invalid color value",
                Some("Color values must be strings".to_string()),
            )
        })?;
        Color::new(color_str).map_err(|err| {
            ElaborationDiagnosticError::from_span(
                format!("Invalid background_color: {err}"),
                color_attr.value.span(),
                "invalid color",
                Some("Use a valid CSS color".to_string()),
            )
        })
    }

    /// Determines the layout engine from an attribute
    fn determine_layout_engine(
        engine_attr: &parser_types::Attribute<'_>,
    ) -> EResult<types::LayoutEngine> {
        let engine_str = engine_attr.value.as_str().map_err(|err| {
            ElaborationDiagnosticError::from_span(
                err.to_string(),
                engine_attr.value.span(),
                "invalid layout engine",
                Some("Layout engine must be a string".to_string()),
            )
        })?;
        types::LayoutEngine::from_str(engine_str).map_err(|_| {
            ElaborationDiagnosticError::from_span(
                format!("Invalid layout_engine value: '{engine_str}'"),
                engine_attr.value.span(),
                "unsupported layout engine",
                Some("Supported layout engines are: 'basic', 'force', 'sugiyama'".to_string()),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "ActivateBlock should have been desugared")]
    fn test_activate_block_panics_in_elaboration() {
        // Build a parser_types diagram directly with an ActivateBlock element
        let elements = vec![parser_types::Element::ActivateBlock {
            component: Spanned::new("user".to_string(), Span::new(0..4)),
            elements: vec![],
        }];

        let diagram = parser_types::Diagram {
            kind: Spanned::new("component", Span::new(0..9)),
            attributes: vec![],
            type_definitions: vec![],
            elements,
        };

        let spanned_element =
            Spanned::new(parser_types::Element::Diagram(diagram), Span::new(0..100));

        let config = AppConfig::default();
        let builder = Builder::new(&config, "test");
        // This should panic due to unreachable!() on ActivateBlock during elaboration
        let _ = builder.build(&spanned_element);
    }

    #[test]
    fn test_explicit_activation_scoping_behavior() {
        // Test that sequence diagrams don't create namespace scopes within activate blocks
        let elements = vec![
            parser_types::Element::Activate {
                component: Spanned::new("user".to_string(), Span::new(0..4)),
            },
            parser_types::Element::Relation {
                source: Spanned::new("user".to_string(), Span::new(0..4)),
                target: Spanned::new("server".to_string(), Span::new(0..6)),
                relation_type: Spanned::new("->", Span::new(0..2)),
                type_spec: None,
                label: Some(Spanned::new("Request".to_string(), Span::new(0..7))),
            },
            parser_types::Element::Relation {
                source: Spanned::new("server".to_string(), Span::new(0..6)),
                target: Spanned::new("database".to_string(), Span::new(0..8)),
                relation_type: Spanned::new("->", Span::new(0..2)),
                type_spec: None,
                label: Some(Spanned::new("Query".to_string(), Span::new(0..5))),
            },
            parser_types::Element::Deactivate {
                component: Spanned::new("user".to_string(), Span::new(0..4)),
            },
        ];

        let diagram = parser_types::Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements,
        };

        let spanned_element =
            Spanned::new(parser_types::Element::Diagram(diagram), Span::new(0..100));

        let config = AppConfig::default();
        let builder = Builder::new(&config, "test");
        let result = builder.build(&spanned_element);

        assert!(
            result.is_ok(),
            "Sequence diagram with activate block should work"
        );

        let diagram = result.unwrap();
        // After desugaring, relations remain unscoped; ensure names were not prefixed
        for element in diagram.scope().elements() {
            if let types::Element::Relation(relation) = element {
                // Relations should maintain original naming, not be scoped under "user"
                let source_str = relation.source().to_string();
                let target_str = relation.target().to_string();
                assert!(
                    !source_str.starts_with("user::user::"),
                    "Source should not be double-scoped: {}",
                    source_str
                );
                assert!(
                    !target_str.starts_with("user::server::"),
                    "Target should not be double-scoped: {}",
                    target_str
                );
            }
        }
    }

    #[test]
    fn test_nested_explicit_activations_same_component() {
        // Test that nested activate blocks work and same component can be activated multiple times
        let elements = vec![
            parser_types::Element::Activate {
                component: Spanned::new("user".to_string(), Span::new(0..4)),
            },
            parser_types::Element::Relation {
                source: Spanned::new("user".to_string(), Span::new(0..4)),
                target: Spanned::new("server".to_string(), Span::new(0..6)),
                relation_type: Spanned::new("->", Span::new(0..2)),
                type_spec: None,
                label: Some(Spanned::new(
                    "Initial request".to_string(),
                    Span::new(0..16),
                )),
            },
            parser_types::Element::Activate {
                component: Spanned::new("user".to_string(), Span::new(0..4)),
            },
            parser_types::Element::Relation {
                source: Spanned::new("user".to_string(), Span::new(0..4)),
                target: Spanned::new("database".to_string(), Span::new(0..8)),
                relation_type: Spanned::new("->", Span::new(0..2)),
                type_spec: None,
                label: Some(Spanned::new("Direct query".to_string(), Span::new(0..12))),
            },
            parser_types::Element::Deactivate {
                component: Spanned::new("user".to_string(), Span::new(0..4)),
            },
            parser_types::Element::Activate {
                component: Spanned::new("server".to_string(), Span::new(0..6)),
            },
            parser_types::Element::Relation {
                source: Spanned::new("server".to_string(), Span::new(0..6)),
                target: Spanned::new("cache".to_string(), Span::new(0..5)),
                relation_type: Spanned::new("->", Span::new(0..2)),
                type_spec: None,
                label: Some(Spanned::new("Cache lookup".to_string(), Span::new(0..12))),
            },
            parser_types::Element::Deactivate {
                component: Spanned::new("server".to_string(), Span::new(0..6)),
            },
            parser_types::Element::Deactivate {
                component: Spanned::new("user".to_string(), Span::new(0..4)),
            },
        ];

        let diagram = parser_types::Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements,
        };

        let spanned_element =
            Spanned::new(parser_types::Element::Diagram(diagram), Span::new(0..100));

        let config = AppConfig::default();
        let builder = Builder::new(&config, "test");
        let result = builder.build(&spanned_element);

        assert!(
            result.is_ok(),
            "Nested activate blocks should work: {:?}",
            result.err()
        );

        let diagram = result.unwrap();
        let elems = diagram.scope().elements();

        let activations: Vec<_> = elems
            .iter()
            .filter_map(|e| {
                if let types::Element::Activate(id) = e {
                    Some(id.to_string())
                } else {
                    None
                }
            })
            .collect();
        let deactivations: Vec<_> = elems
            .iter()
            .filter_map(|e| {
                if let types::Element::Deactivate(id) = e {
                    Some(id.to_string())
                } else {
                    None
                }
            })
            .collect();
        let relations: Vec<_> = elems
            .iter()
            .filter_map(|e| {
                if let types::Element::Relation(r) = e {
                    Some((r.source().to_string(), r.target().to_string()))
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(
            relations.len(),
            3,
            "Should have 3 relations after desugaring"
        );
        assert_eq!(
            activations.len(),
            3,
            "Should have 3 activation starts after desugaring"
        );
        assert_eq!(
            deactivations.len(),
            3,
            "Should have 3 activation ends after desugaring"
        );

        assert_eq!(
            activations[0], "user",
            "First activation should be for 'user'"
        );
        assert_eq!(
            deactivations.last().unwrap(),
            "user",
            "Last deactivation should be for 'user'"
        );
    }

    #[test]
    fn test_explicit_activate_in_sequence_diagram() {
        use crate::config::AppConfig;
        let config = AppConfig::default();
        let builder = Builder::new(&config, "test");

        // Create a simple sequence diagram with explicit activate
        let elements = vec![
            // Define a component
            parser_types::Element::Component {
                name: Spanned::new("user", Span::new(0..4)),
                display_name: None,
                type_name: Spanned::new("Rectangle", Span::new(5..14)),
                attributes: vec![],
                nested_elements: vec![],
            },
            // Activate the component
            parser_types::Element::Activate {
                component: Spanned::new("user".to_string(), Span::new(0..4)),
            },
            // Deactivate the component
            parser_types::Element::Deactivate {
                component: Spanned::new("user".to_string(), Span::new(0..4)),
            },
        ];

        let diagram = parser_types::Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements,
        };

        let spanned_element =
            Spanned::new(parser_types::Element::Diagram(diagram), Span::new(0..100));

        let result = builder.build(&spanned_element);
        assert!(
            result.is_ok(),
            "Should successfully build sequence diagram with explicit activate/deactivate"
        );

        let elaborate_diagram = result.unwrap();
        let elements = elaborate_diagram.scope().elements();

        // Check that we have the expected elements
        assert_eq!(
            elements.len(),
            3,
            "Should have 3 elements: component, activate, deactivate"
        );

        // Verify the activate element
        if let types::Element::Activate(id) = &elements[1] {
            assert_eq!(
                id.to_string(),
                "user",
                "Activate should reference 'user' component"
            );
        } else {
            panic!("Second element should be Activate");
        }

        // Verify the deactivate element
        if let types::Element::Deactivate(id) = &elements[2] {
            assert_eq!(
                id.to_string(),
                "user",
                "Deactivate should reference 'user' component"
            );
        } else {
            panic!("Third element should be Deactivate");
        }
    }

    #[test]
    fn test_explicit_activate_not_allowed_in_component_diagram() {
        use crate::config::AppConfig;
        let config = AppConfig::default();
        let builder = Builder::new(&config, "test");

        // Create a component diagram with explicit activate (should fail)
        let elements = vec![
            // Define a component
            parser_types::Element::Component {
                name: Spanned::new("user", Span::new(0..4)),
                display_name: None,
                type_name: Spanned::new("Rectangle", Span::new(5..14)),
                attributes: vec![],
                nested_elements: vec![],
            },
            // Try to activate the component (should fail)
            parser_types::Element::Activate {
                component: Spanned::new("user".to_string(), Span::new(0..4)),
            },
        ];

        let diagram = parser_types::Diagram {
            kind: Spanned::new("component", Span::new(0..9)),
            attributes: vec![],
            type_definitions: vec![],
            elements,
        };

        let spanned_element =
            Spanned::new(parser_types::Element::Diagram(diagram), Span::new(0..100));

        let result = builder.build(&spanned_element);
        assert!(
            result.is_err(),
            "Should fail to build component diagram with explicit activate"
        );

        if let Err(err) = result {
            let error_message = format!("{}", err);
            assert!(
                error_message
                    .contains("Activate statements are only supported in sequence diagrams"),
                "Error should mention that activate is not allowed in component diagrams"
            );
        }
    }

    #[test]
    fn test_explicit_activation_timing_and_nesting() {
        // Test that activate blocks have proper timing based on contained messages
        // and correct nesting levels for nested activate blocks
        let elements = vec![
            // components
            parser_types::Element::Component {
                name: Spanned::new("user", Span::new(0..4)),
                display_name: None,
                type_name: Spanned::new("Rectangle", Span::new(0..9)),
                attributes: vec![],
                nested_elements: vec![],
            },
            parser_types::Element::Component {
                name: Spanned::new("server", Span::new(0..6)),
                display_name: None,
                type_name: Spanned::new("Rectangle", Span::new(0..9)),
                attributes: vec![],
                nested_elements: vec![],
            },
            parser_types::Element::Component {
                name: Spanned::new("database", Span::new(0..8)),
                display_name: None,
                type_name: Spanned::new("Rectangle", Span::new(0..9)),
                attributes: vec![],
                nested_elements: vec![],
            },
            // activations and relations
            parser_types::Element::Activate {
                component: Spanned::new("user".to_string(), Span::new(0..4)),
            },
            parser_types::Element::Relation {
                source: Spanned::new("user".to_string(), Span::new(0..4)),
                target: Spanned::new("server".to_string(), Span::new(0..6)),
                relation_type: Spanned::new("->", Span::new(0..2)),
                type_spec: None,
                label: Some(Spanned::new("First request".to_string(), Span::new(0..13))),
            },
            parser_types::Element::Activate {
                component: Spanned::new("server".to_string(), Span::new(0..6)),
            },
            parser_types::Element::Relation {
                source: Spanned::new("server".to_string(), Span::new(0..6)),
                target: Spanned::new("database".to_string(), Span::new(0..8)),
                relation_type: Spanned::new("->", Span::new(0..2)),
                type_spec: None,
                label: Some(Spanned::new("Nested query".to_string(), Span::new(0..12))),
            },
            parser_types::Element::Relation {
                source: Spanned::new("database".to_string(), Span::new(0..8)),
                target: Spanned::new("server".to_string(), Span::new(0..6)),
                relation_type: Spanned::new("->", Span::new(0..2)),
                type_spec: None,
                label: Some(Spanned::new(
                    "Nested response".to_string(),
                    Span::new(0..15),
                )),
            },
            parser_types::Element::Deactivate {
                component: Spanned::new("server".to_string(), Span::new(0..6)),
            },
            parser_types::Element::Relation {
                source: Spanned::new("server".to_string(), Span::new(0..6)),
                target: Spanned::new("user".to_string(), Span::new(0..4)),
                relation_type: Spanned::new("->", Span::new(0..2)),
                type_spec: None,
                label: Some(Spanned::new("First response".to_string(), Span::new(0..14))),
            },
            parser_types::Element::Activate {
                component: Spanned::new("user".to_string(), Span::new(0..4)),
            },
            parser_types::Element::Relation {
                source: Spanned::new("user".to_string(), Span::new(0..4)),
                target: Spanned::new("server".to_string(), Span::new(0..6)),
                relation_type: Spanned::new("->", Span::new(0..2)),
                type_spec: None,
                label: Some(Spanned::new("Second request".to_string(), Span::new(0..14))),
            },
            parser_types::Element::Deactivate {
                component: Spanned::new("user".to_string(), Span::new(0..4)),
            },
            parser_types::Element::Deactivate {
                component: Spanned::new("user".to_string(), Span::new(0..4)),
            },
        ];

        let diagram = parser_types::Diagram {
            kind: Spanned::new("sequence", Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements,
        };

        let spanned_element =
            Spanned::new(parser_types::Element::Diagram(diagram), Span::new(0..100));

        let config = AppConfig::default();
        let builder = Builder::new(&config, "test");
        let result = builder.build(&spanned_element);

        assert!(
            result.is_ok(),
            "Complex nested activate blocks should work: {:?}",
            result.err()
        );

        let diagram = result.unwrap();

        // After desugaring, ensure we have multiple relations and activation statements
        let elems = diagram.scope().elements();
        let relations = elems
            .iter()
            .filter(|e| matches!(e, types::Element::Relation(_)))
            .count();
        let activates = elems
            .iter()
            .filter(|e| matches!(e, types::Element::Activate(_)))
            .count();
        let deactivates = elems
            .iter()
            .filter(|e| matches!(e, types::Element::Deactivate(_)))
            .count();

        assert!(
            relations >= 5,
            "Should have at least 5 relations after desugaring, found {}",
            relations
        );
        assert!(
            activates >= 3,
            "Should have at least 3 activates after desugaring, found {}",
            activates
        );
        assert!(
            deactivates >= 3,
            "Should have at least 3 deactivates after desugaring, found {}",
            deactivates
        );
    }
}
