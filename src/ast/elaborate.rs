use super::parser_types;
use crate::ast::span::Spanned;
use crate::{
    color::Color,
    error::ElaborationDiagnosticError,
    shape::{Oval, Rectangle, Shape},
};
use log::{debug, info, trace};
use std::{collections::HashMap, fmt, rc::Rc};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeId(String);

#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: TypeId,
    pub value: String, // TODO: Can I convert it to str?
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelationType {
    Forward,       // ->
    Backward,      // <-
    Bidirectional, // <->
    Plain,         // -
}

impl RelationType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "->" => RelationType::Forward,
            "<-" => RelationType::Backward,
            "<->" => RelationType::Bidirectional,
            "-" => RelationType::Plain,
            _ => RelationType::Forward, // Default to forward if unknown
        }
    }

    fn to_string(&self) -> &'static str {
        match self {
            RelationType::Forward => "->",
            RelationType::Backward => "<-",
            RelationType::Bidirectional => "<->",
            RelationType::Plain => "-",
        }
    }
}
impl fmt::Display for RelationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: TypeId,
    pub name: String,
    pub block: Block,
    pub type_definition: Rc<TypeDefinition>,
}

#[derive(Debug, Clone)]
pub struct Relation {
    pub source: TypeId,
    pub target: TypeId,
    pub relation_type: RelationType,
    pub color: Color,
    pub width: usize,
}

#[derive(Debug, Clone)]
pub enum Element {
    Node(Node),
    Relation(Relation),
}

#[derive(Debug, Default, Clone)]
pub struct Scope {
    pub elements: Vec<Element>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum DiagramKind {
    Component,
    Sequence,
}

#[derive(Clone)]
pub struct TypeDefinition {
    pub id: TypeId,
    pub fill_color: Option<Color>,
    pub line_color: Color,
    pub line_width: usize,
    pub rounded: usize,
    pub font_size: usize,
    pub shape_type: Rc<dyn Shape>,
}

#[derive(Debug, Clone)]
pub struct Diagram {
    pub kind: DiagramKind,
    pub scope: Scope,
}

#[derive(Debug, Clone)]
pub enum Block {
    None,
    Scope(Scope),
    Diagram(Diagram),
}

impl Block {
    /// Returns true if this block contains any elements
    pub fn has_nested_blocks(&self) -> bool {
        match self {
            Block::None => false,
            Block::Scope(scope) => !scope.elements.is_empty(),
            Block::Diagram(diagram) => !diagram.scope.elements.is_empty(),
        }
    }
}

pub struct Builder<'a> {
    type_definitions: Vec<Rc<TypeDefinition>>,
    type_definition_map: HashMap<TypeId, Rc<TypeDefinition>>,
    source: &'a str, // Store the original source code for error reporting
}

impl<'a> Builder<'a> {
    pub fn new(source: &'a str) -> Self {
        let type_definitions = TypeDefinition::defaults();
        let type_definition_map = type_definitions
            .iter()
            .map(|def| (def.id.clone(), Rc::clone(def)))
            .collect();

        Self {
            type_definitions,
            type_definition_map,
            source,
        }
    }

    pub fn build(
        mut self,
        diag: Spanned<parser_types::Element<'a>>,
    ) -> Result<Diagram, ElaborationDiagnosticError> {
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
                    Block::None => {
                        debug!("Empty block, using default scope");
                        Scope::default()
                    }
                    Block::Scope(scope) => {
                        debug!(
                            elements_len = scope.elements.len();
                            "Using scope from block",
                        );
                        scope
                    }
                    Block::Diagram(_) => {
                        return Err(ElaborationDiagnosticError::from_spanned(
                            "Nested diagram not allowed".to_string(),
                            &diag.kind,
                            self.source,
                            "invalid diagram structure",
                            Some("Diagrams cannot be nested inside other diagrams".to_string()),
                        ));
                    }
                };

                // Determine the diagram kind based on the kind string
                let kind = match *diag.kind.inner() {
                    // FIXME: Why kind has &&str?!
                    "sequence" => DiagramKind::Sequence,
                    "component" => DiagramKind::Component,
                    _ => {
                        return Err(ElaborationDiagnosticError::from_spanned(
                            format!("Invalid diagram kind: '{}'", diag.kind),
                            &diag.kind,
                            self.source,
                            "unsupported diagram type",
                            Some(
                                "Supported diagram types are: 'component', 'sequence'".to_string(),
                            ),
                        ));
                    }
                };

                info!(
                    "Diagram elaboration completed successfully with kind: {:?}",
                    kind
                );
                Ok(Diagram { kind, scope })
            }
            _ => Err(ElaborationDiagnosticError::from_spanned(
                "Invalid element, expected Diagram".to_string(),
                &diag,
                self.source,
                "invalid element",
                None,
            )),
        }
    }

    fn insert_type_definition(
        &mut self,
        type_def: Spanned<TypeDefinition>,
    ) -> Result<Rc<TypeDefinition>, ElaborationDiagnosticError> {
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
                self.source,
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
            let base_type_name = TypeId::from_name(&type_def.base_type);
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
                        self.source,
                        "undefined type",
                        Some(format!(
                            "Type '{type_name}' must be a built-in type or defined with a 'type' statement before it can be used as a base type",
                        ))
                    )
                })?;

            // Try to create the type definition
            match TypeDefinition::from_base(
                TypeId::from_name(&type_def.name),
                base,
                &type_def.attributes,
                self.source,
            ) {
                Ok(new_type_def) => {
                    self.insert_type_definition(type_def.map(|_| new_type_def))?;
                }
                Err(err) => {
                    // Wrap the error with location information for attribute errors
                    return Err(ElaborationDiagnosticError::from_spanned(
                        format!("Invalid type definition: {err}"),
                        &type_def.name,
                        self.source,
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
    ) -> Result<Diagram, ElaborationDiagnosticError> {
        match diag.inner() {
            parser_types::Element::Diagram(diag) => {
                let block = self.build_block_from_elements(&diag.elements, None)?;
                let scope = match block {
                    Block::None => Scope::default(),
                    Block::Scope(scope) => scope,
                    Block::Diagram(_) => {
                        return Err(ElaborationDiagnosticError::from_spanned(
                            "Nested diagram not allowed".to_string(),
                            &diag.kind,
                            self.source,
                            "invalid nesting",
                            Some("Diagrams cannot be nested inside other diagrams".to_string()),
                        ));
                    }
                };

                // Determine the diagram kind
                let kind = match *diag.kind {
                    "sequence" => DiagramKind::Sequence,
                    "component" => DiagramKind::Component,
                    _ => {
                        return Err(ElaborationDiagnosticError::from_spanned(
                            format!("Invalid diagram kind: '{}'", diag.kind),
                            &diag.kind,
                            self.source,
                            "unsupported diagram type",
                            Some(
                                "Supported diagram types are: 'component', 'sequence'".to_string(),
                            ),
                        ));
                    }
                };

                Ok(Diagram { kind, scope })
            }
            _ => Err(ElaborationDiagnosticError::from_spanned(
                "Invalid element, expected Diagram".to_string(),
                diag,
                self.source,
                "invalid element",
                None,
            )),
        }
    }

    fn build_block_from_elements(
        &mut self,
        parser_elements: &[Spanned<parser_types::Element>],
        parent_id: Option<&TypeId>,
    ) -> Result<Block, ElaborationDiagnosticError> {
        if parser_elements.is_empty() {
            Ok(Block::None)
        } else if let parser_types::Element::Diagram { .. } = parser_elements[0].inner() {
            // This case happens when a diagram is the first element in a block
            Ok(Block::Diagram(
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
                        self.source,
                        "invalid nesting",
                        Some(
                            "A diagram declaration must be the only element in its scope"
                                .to_string(),
                        ),
                    ));
                }
            }

            // If no diagrams were found mixed with other elements, build the scope
            Ok(Block::Scope(
                self.build_scope_from_elements(parser_elements, parent_id)?,
            ))
        }
    }

    fn build_scope_from_elements(
        &mut self,
        parser_elements: &[Spanned<parser_types::Element>],
        parent_id: Option<&TypeId>,
    ) -> Result<Scope, ElaborationDiagnosticError> {
        let mut elements = Vec::new();
        for parser_elm in parser_elements {
            match parser_elm.inner() {
                parser_types::Element::Component {
                    name,
                    type_name,
                    attributes,
                    nested_elements,
                } => {
                    let node_id = match parent_id {
                        Some(parent) => parent.create_nested(name),
                        None => TypeId::from_name(name),
                    };

                    // Try to get the type definition for this element
                    let type_def = match self.build_element_type_definition(type_name, attributes) {
                        Ok(def) => def,
                        Err(_) => {
                            return Err(ElaborationDiagnosticError::from_spanned(
                                format!("Unknown type '{type_name}' for component '{name}'"),
                                name, // Use the component name's span as the error location
                                self.source,
                                "undefined type",
                                Some(format!(
                                    "Type '{type_name}' must be a built-in type or defined with a 'type' statement before it can be used as a base type"
                                )),
                            ));
                        }
                    };

                    // Process nested elements with the new ID as parent
                    let block = self.build_block_from_elements(nested_elements, Some(&node_id))?;

                    let node = Node {
                        id: node_id,
                        name: name.to_string(),
                        block,
                        type_definition: type_def,
                    };

                    elements.push(Element::Node(node));
                }
                parser_types::Element::Relation {
                    source,
                    target,
                    relation_type,
                    attributes,
                    ..
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
                                            self.source,
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
                                            self.source,
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
                    let source_id = match parent_id {
                        Some(parent) => parent.create_nested(source),
                        None => TypeId::from_name(source),
                    };

                    let target_id = match parent_id {
                        Some(parent) => parent.create_nested(target),
                        None => TypeId::from_name(target),
                    };

                    elements.push(Element::Relation(Relation {
                        source: source_id,
                        target: target_id,
                        relation_type: RelationType::from_str(relation_type),
                        color,
                        width,
                    }))
                }
                _ => {
                    // This should never happen since we already filtered out invalid elements
                    return Err(ElaborationDiagnosticError::from_spanned(
                        "Invalid element type".to_string(),
                        parser_elm,
                        self.source,
                        "invalid element type",
                        None,
                    ));
                }
            }
        }
        Ok(Scope { elements })
    }

    fn build_element_type_definition(
        &mut self,
        type_name: &Spanned<&str>,
        attributes: &[Spanned<parser_types::Attribute>],
    ) -> Result<Rc<TypeDefinition>, ElaborationDiagnosticError> {
        // Look up the base type
        let type_id = TypeId::from_name(type_name);
        let base = match self.type_definition_map.get(&type_id) {
            Some(base) => base,
            None => {
                return Err(ElaborationDiagnosticError::from_spanned(
                    format!("Unknown type '{type_name}' for component '{type_name}'"),
                    type_name, // Use the component name's span as the error location
                    self.source,
                    "undefined type",
                    Some(format!(
                        "Type '{type_name}' must be a built-in type or defined with a 'type' statement before it can be used as a base type"
                    )),
                ));
            }
        };

        // If there are no attributes, just return the base type
        if attributes.is_empty() {
            return Ok(Rc::clone(base));
        }

        // Otherwise, create a new anonymous type based on the base type
        let id = TypeId::from_anonymous(self.type_definition_map.len());
        match TypeDefinition::from_base(id, base, attributes, self.source) {
            Ok(new_type) => self.insert_type_definition(type_name.map(|_| new_type)),
            Err(err) => Err(ElaborationDiagnosticError::from_spanned(
                format!("Error creating type based on '{type_name}': {err}"),
                type_name,
                self.source,
                "undefined type",
                Some(format!(
                    "Type '{type_name}' must be a built-in type or defined with a 'type' statement before it can be used as a base type"
                )),
            )),
        }
    }
}

impl TypeId {
    /// Creates a TypeId from a component name as defined in the diagram
    fn from_name(name: &str) -> Self {
        TypeId(name.to_string())
    }

    /// Creates an internal TypeId used for generated types
    /// (e.g., for anonymous type definitions)
    fn from_anonymous(idx: usize) -> Self {
        TypeId(format!("__{idx}"))
    }

    /// Creates a nested ID by combining parent ID and child ID with '::' separator
    fn create_nested(&self, child_id: &str) -> Self {
        TypeId(format!("{}::{}", self.0, child_id))
    }
}

impl fmt::Display for TypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Implement Debug manually for TypeDefinition since we can't derive it due to the dyn ShapeType
impl std::fmt::Debug for TypeDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypeDefinition")
            .field("id", &self.id)
            .field("fill_color", &self.fill_color)
            .field("line_color", &self.line_color)
            .field("line_width", &self.line_width)
            .field("rounded", &self.rounded)
            .field("font_size", &self.font_size)
            .field("shape_type", &self.shape_type.name())
            .finish()
    }
}

impl TypeDefinition {
    fn from_base(
        id: TypeId,
        base: &Self,
        attributes: &[Spanned<parser_types::Attribute>],
        src: &str, // TODO: Implement source location tracking
    ) -> Result<Self, ElaborationDiagnosticError> {
        let mut type_def = base.clone();
        type_def.id = id;
        // Process attributes with descriptive errors
        for attr in Attribute::new_from_parser(attributes) {
            let name = attr.name.0.as_str();
            let value = attr.value.as_str();

            match name {
                "fill_color" => {
                    type_def.fill_color = Some(Color::new(value).map_err(|err| {
                        ElaborationDiagnosticError::from_spanned(
                            format!("Invalid fill_color '{value}': {err}"),
                            &attr,
                            src,
                            "invalid color",
                            Some("Use a CSS color".to_string()),
                        )
                    })?)
                }
                "line_color" => {
                    type_def.line_color = Color::new(value).map_err(|err| {
                        ElaborationDiagnosticError::from_spanned(
                            format!("Invalid line_color '{value}': {err}"),
                            &attr,
                            src,
                            "invalid color",
                            Some("Use a CSS color".to_string()),
                        )
                    })?
                }
                "line_width" => {
                    type_def.line_width =
                        value
                            .parse::<usize>()
                            .or(Err(ElaborationDiagnosticError::from_spanned(
                                format!("Invalid line_width '{value}'"),
                                &attr,
                                src,
                                "invalid positive integer",
                                Some("Use a positive integer".to_string()),
                            )))?
                }
                "rounded" => {
                    type_def.rounded =
                        value
                            .parse::<usize>()
                            .or(Err(ElaborationDiagnosticError::from_spanned(
                                format!("Invalid rounded '{value}'"),
                                &attr,
                                src,
                                "invalid positive integer",
                                Some("Use a positive integer".to_string()),
                            )))?
                }
                "font_size" => {
                    type_def.font_size =
                        value
                            .parse::<usize>()
                            .or(Err(ElaborationDiagnosticError::from_spanned(
                                format!("Invalid font_size '{value}'"),
                                &attr,
                                src,
                                "invalid positive integer",
                                Some("Use a positive integer".to_string()),
                            )))?
                }
                _ => {
                    // TODO: For unknown attributes, just add them to the list
                    // We could warn about them, but we'll just keep them for now
                }
            }
        }

        Ok(type_def)
    }

    fn defaults() -> Vec<Rc<TypeDefinition>> {
        let black = Color::default();
        vec![
            Rc::new(Self {
                id: TypeId::from_name("Rectangle"),
                fill_color: None,
                line_color: black.clone(),
                line_width: 2,
                rounded: 0,
                font_size: 15,
                shape_type: Rc::new(Rectangle) as Rc<dyn Shape>,
            }),
            Rc::new(Self {
                id: TypeId::from_name("Oval"),
                fill_color: None,
                line_color: black,
                line_width: 2,
                rounded: 0,
                font_size: 15,
                shape_type: Rc::new(Oval) as Rc<dyn Shape>,
            }),
        ]
    }
}

impl Attribute {
    fn new(name: &str, value: &str) -> Self {
        Self {
            name: TypeId::from_name(name),
            value: value.to_string(),
        }
    }

    fn new_from_parser(parser_attrs: &[Spanned<parser_types::Attribute>]) -> Vec<Spanned<Self>> {
        parser_attrs
            .iter()
            .map(|attr| attr.map(|attr| Self::new(&attr.name, &attr.value)))
            .collect()
    }
}
