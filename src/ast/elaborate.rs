use super::{elaborate_types as types, parser_types};
use crate::ast::span::Spanned;
use crate::{color::Color, error::ElaborationDiagnosticError};
use log::{debug, info, trace};
use std::{collections::HashMap, rc::Rc};

pub struct Builder<'a> {
    type_definitions: Vec<Rc<types::TypeDefinition>>,
    type_definition_map: HashMap<types::TypeId, Rc<types::TypeDefinition>>,
    _phantom: std::marker::PhantomData<&'a str>, // Use PhantomData to maintain the lifetime parameter
}

impl<'a> Builder<'a> {
    pub fn new(_source: &'a str) -> Self {
        // We keep the source parameter for backward compatibility but don't store it anymore
        let type_definitions = types::TypeDefinition::defaults();
        let type_definition_map = type_definitions
            .iter()
            .map(|def| (def.id.clone(), Rc::clone(def)))
            .collect();

        Self {
            type_definitions,
            type_definition_map,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn build(
        mut self,
        diag: &Spanned<parser_types::Element<'a>>,
    ) -> Result<types::Diagram, ElaborationDiagnosticError> {
        debug!("Building elaborated diagram");
        match diag.inner() {
            parser_types::Element::Diagram(diag) => {
                info!("Processing diagram of kind: {}", diag.kind);
                trace!("Type definitions: {:?}", diag.type_definitions);
                trace!("Elements count: {}", diag.elements.len());

                // Update type definitions
                debug!("Updating type definitions");
                self.update_type_direct_definitions(&diag.type_definitions)?;

                // Build block from elements
                debug!("Building block from elements");
                let block = self.build_block_from_elements(&diag.elements, None)?;

                // Convert block to scope
                let scope = match block {
                    types::Block::None => {
                        debug!("Empty block, using default scope");
                        types::Scope::default()
                    }
                    types::Block::Scope(scope) => {
                        debug!(
                            elements_len = scope.elements.len();
                            "Using scope from block",
                        );
                        scope
                    }
                    types::Block::Diagram(_) => {
                        return Err(ElaborationDiagnosticError::from_spanned(
                            "Nested diagram not allowed".to_string(),
                            &diag.kind,
                            "invalid diagram structure",
                            Some("Diagrams cannot be nested inside other diagrams".to_string()),
                        ));
                    }
                };

                // Determine the diagram kind based on the kind string
                let kind = match *diag.kind.inner() {
                    // FIXME: Why kind has &&str?!
                    "sequence" => types::DiagramKind::Sequence,
                    "component" => types::DiagramKind::Component,
                    _ => {
                        return Err(ElaborationDiagnosticError::from_spanned(
                            format!("Invalid diagram kind: '{}'", diag.kind),
                            &diag.kind,
                            "unsupported diagram type",
                            Some(
                                "Supported diagram types are: 'component', 'sequence'".to_string(),
                            ),
                        ));
                    }
                };

                info!(kind:?; "Diagram elaboration completed successfully");
                Ok(types::Diagram { kind, scope })
            }
            _ => Err(ElaborationDiagnosticError::from_spanned(
                "Invalid element, expected Diagram".to_string(),
                diag,
                "invalid element",
                None,
            )),
        }
    }

    fn insert_type_definition(
        &mut self,
        type_def: Spanned<types::TypeDefinition>,
    ) -> Result<Rc<types::TypeDefinition>, ElaborationDiagnosticError> {
        let span = type_def.clone_spanned();
        let type_def = type_def.into_inner();
        let id = type_def.id.clone();
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
            Err(ElaborationDiagnosticError::from_spanned(
                format!("Type definition '{}' already exists", type_def.id),
                &span,
                "duplicate type definition",
                None,
            ))
        }
    }

    fn update_type_direct_definitions(
        &mut self,
        type_defs: &Spanned<Vec<Spanned<parser_types::TypeDefinition<'a>>>>,
    ) -> Result<(), ElaborationDiagnosticError> {
        for type_def in type_defs.inner() {
            let base_type_name = types::TypeId::from_name(&type_def.base_type);
            let base = self
                .type_definition_map
                .get(&base_type_name)
                .ok_or_else(|| {
                    // Create a rich diagnostic error with source location information
                    let type_name = &type_def.base_type;
                    let message = format!("Base type '{type_name}' not found");

                    ElaborationDiagnosticError::from_spanned(
                        message,
                        &type_def.base_type,
                        "undefined type",
                        Some(format!(
                            "Type '{type_name}' must be a built-in type or defined with a 'type' statement before it can be used as a base type",
                        ))
                    )
                })?;

            // Try to create the type definition
            match types::TypeDefinition::from_base(
                types::TypeId::from_name(&type_def.name),
                base,
                &type_def.attributes,
            ) {
                Ok(new_type_def) => {
                    self.insert_type_definition(type_def.map(|_| new_type_def))?;
                }
                Err(err) => {
                    // Wrap the error with location information for attribute errors
                    return Err(ElaborationDiagnosticError::from_spanned(
                        format!("Invalid type definition: {err}"),
                        &type_def.name,
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
        diag: &Spanned<parser_types::Element>,
    ) -> Result<types::Diagram, ElaborationDiagnosticError> {
        match diag.inner() {
            parser_types::Element::Diagram(diag) => {
                let block = self.build_block_from_elements(&diag.elements, None)?;
                let scope = match block {
                    types::Block::None => types::Scope::default(),
                    types::Block::Scope(scope) => scope,
                    types::Block::Diagram(_) => {
                        return Err(ElaborationDiagnosticError::from_spanned(
                            "Nested diagram not allowed".to_string(),
                            &diag.kind,
                            "invalid nesting",
                            Some("Diagrams cannot be nested inside other diagrams".to_string()),
                        ));
                    }
                };

                // Determine the diagram kind
                let kind = match *diag.kind {
                    "sequence" => types::DiagramKind::Sequence,
                    "component" => types::DiagramKind::Component,
                    _ => {
                        return Err(ElaborationDiagnosticError::from_spanned(
                            format!("Invalid diagram kind: '{}'", diag.kind),
                            &diag.kind,
                            "unsupported diagram type",
                            Some(
                                "Supported diagram types are: 'component', 'sequence'".to_string(),
                            ),
                        ));
                    }
                };

                Ok(types::Diagram { kind, scope })
            }
            _ => Err(ElaborationDiagnosticError::from_spanned(
                "Invalid element, expected Diagram".to_string(),
                diag,
                "invalid element",
                None,
            )),
        }
    }

    fn build_block_from_elements(
        &mut self,
        parser_elements: &[Spanned<parser_types::Element>],
        parent_id: Option<&types::TypeId>,
    ) -> Result<types::Block, ElaborationDiagnosticError> {
        if parser_elements.is_empty() {
            Ok(types::Block::None)
        } else if let parser_types::Element::Diagram { .. } = parser_elements[0].inner() {
            // This case happens when a diagram is the first element in a block
            Ok(types::Block::Diagram(
                self.build_diagram_from_parser(&parser_elements[0])?,
            ))
        } else {
            // Check to make sure no diagrams are mixed with other elements
            for parser_elm in parser_elements {
                if let parser_types::Element::Diagram(diag) = parser_elm.inner() {
                    // If we found a diagram mixed with other elements, provide a rich error
                    return Err(ElaborationDiagnosticError::from_spanned(
                        "Diagram cannot share scope with other elements".to_string(),
                        &diag.kind, // Use the diagram kind span as the error location
                        "invalid nesting",
                        Some(
                            "A diagram declaration must be the only element in its scope"
                                .to_string(),
                        ),
                    ));
                }
            }

            // If no diagrams were found mixed with other elements, build the scope
            Ok(types::Block::Scope(
                self.build_scope_from_elements(parser_elements, parent_id)?,
            ))
        }
    }

    fn build_scope_from_elements(
        &mut self,
        parser_elements: &[Spanned<parser_types::Element>],
        parent_id: Option<&types::TypeId>,
    ) -> Result<types::Scope, ElaborationDiagnosticError> {
        let mut elements = Vec::new();
        for parser_elm in parser_elements {
            match parser_elm.inner() {
                parser_types::Element::Component {
                    name,
                    type_name,
                    attributes,
                    nested_elements,
                } => {
                    let node_id = parent_id.map_or_else(
                        || types::TypeId::from_name(name),
                        |parent| parent.create_nested(name),
                    );

                    let type_def = self.build_element_type_definition(type_name, attributes)
                        .map_err(|_| ElaborationDiagnosticError::from_spanned(
                            format!("Unknown type '{type_name}' for component '{name}'"),
                            name, // Use the component name's span as the error location
                            "undefined type",
                            Some(format!(
                                "Type '{type_name}' must be a built-in type or defined with a 'type' statement before it can be used as a base type"
                            )),
                        )
                    )?;

                    // Process nested elements with the new ID as parent
                    let block = self.build_block_from_elements(nested_elements, Some(&node_id))?;

                    let node = types::Node {
                        id: node_id,
                        name: name.to_string(),
                        block,
                        type_definition: type_def,
                    };

                    elements.push(types::Element::Node(node));
                }
                parser_types::Element::Relation {
                    source,
                    target,
                    relation_type,
                    attributes,
                    label,
                } => {
                    // Extract color and width from attributes if they exist
                    let mut color = Color::default();
                    let mut width = 1;

                    // Process attributes with better error handling
                    for attr in attributes.inner() {
                        match *attr.name {
                            "color" => {
                                color = match Color::new(&attr.value) {
                                    Ok(color) => color,
                                    Err(err) => {
                                        return Err(ElaborationDiagnosticError::from_spanned(
                                            format!("Invalid color value '{}': {err}", attr.value,),
                                            &attr.value,
                                            "invalid color",
                                            Some(
                                                "Color must be a valid CSS color value".to_string(),
                                            ),
                                        ));
                                    }
                                }
                            }
                            "width" => {
                                width = match attr.value.parse::<usize>() {
                                    Ok(width) => width,
                                    Err(_) => {
                                        return Err(ElaborationDiagnosticError::from_spanned(
                                            format!(
                                                "Invalid width value '{}': expected a positive integer",
                                                attr.value
                                            ),
                                            &attr.value,
                                            "invalid width",
                                            Some("Width must be a positive integer".to_string()),
                                        ));
                                    }
                                };
                            }
                            _ => {
                                // TODO: We could warn about unknown attributes here
                            }
                        }
                    }

                    // Create source and target IDs based on parent context if present
                    let source_id = parent_id.map_or_else(
                        || types::TypeId::from_name(source),
                        |parent| parent.create_nested(source),
                    );

                    let target_id = parent_id.map_or_else(
                        || types::TypeId::from_name(target),
                        |parent| parent.create_nested(target),
                    );

                    elements.push(types::Element::Relation(types::Relation {
                        source: source_id,
                        target: target_id,
                        relation_type: types::RelationType::from_str(relation_type),
                        color,
                        width,
                        label: label.as_ref().map(|l| l.to_string()),
                    }));
                }
                parser_types::Element::Diagram(_) => {
                    // This should never happen since we already filtered out invalid elements
                    return Err(ElaborationDiagnosticError::from_spanned(
                        "Invalid element type".to_string(),
                        parser_elm,
                        "invalid element type",
                        None,
                    ));
                }
            }
        }
        Ok(types::Scope { elements })
    }

    fn build_element_type_definition(
        &mut self,
        type_name: &Spanned<&str>,
        attributes: &[Spanned<parser_types::Attribute>],
    ) -> Result<Rc<types::TypeDefinition>, ElaborationDiagnosticError> {
        // Look up the base type
        let type_id = types::TypeId::from_name(type_name);
        let Some(base) = self.type_definition_map.get(&type_id) else {
            return Err(ElaborationDiagnosticError::from_spanned(
                format!("Unknown type '{type_name}' for component '{type_name}'"),
                type_name, // Use the component name's span as the error location
                "undefined type",
                Some(format!(
                    "Type '{type_name}' must be a built-in type or defined with a 'type' statement before it can be used as a base type"
                )),
            ));
        };

        // If there are no attributes, just return the base type
        if attributes.is_empty() {
            return Ok(Rc::clone(base));
        }

        // Otherwise, create a new anonymous type based on the base type
        let id = types::TypeId::from_anonymous(self.type_definition_map.len());
        match types::TypeDefinition::from_base(id, base, attributes) {
            Ok(new_type) => self.insert_type_definition(type_name.map(|_| new_type)),
            Err(err) => Err(ElaborationDiagnosticError::from_spanned(
                format!("Error creating type based on '{type_name}': {err}"),
                type_name,
                "undefined type",
                Some(format!(
                    "Type '{type_name}' must be a built-in type or defined with a 'type' statement before it can be used as a base type"
                )),
            )),
        }
    }
}
