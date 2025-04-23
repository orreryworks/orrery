use super::parser;
use crate::{
    color::Color,
    error::FilamentError,
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
    pub attributes: Vec<Attribute>,
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

pub struct Builder {
    type_definitions: Vec<Rc<TypeDefinition>>,
    type_definition_map: HashMap<TypeId, Rc<TypeDefinition>>,
}

impl Builder {
    pub fn new() -> Self {
        let type_definitions = TypeDefinition::defaults();
        let type_definition_map = type_definitions
            .iter()
            .map(|def| (def.id.clone(), Rc::clone(def)))
            .collect();

        Self {
            type_definitions,
            type_definition_map,
        }
    }

    pub fn build(mut self, diag: &parser::Element) -> Result<Diagram, FilamentError> {
        debug!("Building elaborated diagram");
        match diag {
            parser::Element::Diagram(diag) => {
                info!("Processing diagram of kind: {}", diag.kind);
                trace!("Type definitions: {:?}", diag.type_definitions);
                trace!("Elements count: {}", diag.elements.len());

                // Update type definitions
                debug!("Updating type definitions");
                self.update_type_direct_definitions(diag)?;

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
                        return Err(FilamentError::Elaboration(
                            "Nested diagram not allowed".to_string(),
                        ))
                    }
                };

                // Determine the diagram kind based on the kind string
                let kind = match diag.kind {
                    "sequence" => DiagramKind::Sequence,
                    "component" => DiagramKind::Component,
                    _ => {
                        return Err(FilamentError::Elaboration(
                            "Invalid diagram kind".to_string(),
                        ))
                    }
                };

                info!(
                    "Diagram elaboration completed successfully with kind: {:?}",
                    kind
                );
                Ok(Diagram { kind, scope })
            }
            _ => Err(FilamentError::Elaboration(
                "Invalid element, expected Diagram".to_string(),
            )),
        }
    }

    fn insert_type_definition(
        &mut self,
        type_def: TypeDefinition,
    ) -> Result<Rc<TypeDefinition>, FilamentError> {
        let id = type_def.id.clone();
        let type_def = Rc::new(type_def);
        self.type_definitions.push(Rc::clone(&type_def));
        if self
            .type_definition_map
            .insert(id, Rc::clone(&type_def))
            .is_none()
        {
            Ok(type_def)
        } else {
            Err(FilamentError::Elaboration(format!(
                "Type definition '{}' already exists",
                type_def.id
            )))
        }
    }

    fn update_type_direct_definitions(
        &mut self,
        diag: &parser::Diagram,
    ) -> Result<(), FilamentError> {
        for type_def in &diag.type_definitions {
            let base = self
                .type_definition_map
                .get(&TypeId::from_name(type_def.base_type))
                .ok_or_else(|| {
                    FilamentError::Elaboration(format!(
                        "Base type '{}' not found",
                        &type_def.base_type
                    ))
                })?;
            self.insert_type_definition(TypeDefinition::from_base(
                TypeId::from_name(type_def.name),
                base,
                &type_def.attributes,
            )?)?;
        }
        Ok(())
    }

    fn build_diagram_from_parser(
        &mut self,
        diag: &parser::Element,
    ) -> Result<Diagram, FilamentError> {
        match diag {
            parser::Element::Diagram(diag) => {
                let block = self.build_block_from_elements(&diag.elements, None)?;
                let scope = match block {
                    Block::None => Scope::default(),
                    Block::Scope(scope) => scope,
                    Block::Diagram(_) => {
                        return Err(FilamentError::Elaboration(
                            "Nested diagram not allowed".to_string(),
                        ));
                    }
                };
                Ok(Diagram {
                    kind: DiagramKind::Component,
                    scope,
                })
            }
            _ => Err(FilamentError::Elaboration(
                "Invalid element, expected Diagram".to_string(),
            )),
        }
    }

    fn build_block_from_elements(
        &mut self,
        parser_elements: &[parser::Element],
        parent_id: Option<&TypeId>,
    ) -> Result<Block, FilamentError> {
        if parser_elements.is_empty() {
            Ok(Block::None)
        } else if let parser::Element::Diagram { .. } = parser_elements[0] {
            Ok(Block::Diagram(
                self.build_diagram_from_parser(&parser_elements[0])?,
            ))
        } else {
            for parser_elm in parser_elements {
                if let parser::Element::Diagram { .. } = parser_elm {
                    return Err(FilamentError::Elaboration(
                        "Diagram cannot share scope with other elements".to_string(),
                    ));
                }
            }
            Ok(Block::Scope(
                self.build_scope_from_elements(parser_elements, parent_id)?,
            ))
        }
    }

    fn build_scope_from_elements(
        &mut self,
        parser_elements: &[parser::Element],
        parent_id: Option<&TypeId>,
    ) -> Result<Scope, FilamentError> {
        let mut elements = Vec::new();
        for parser_elm in parser_elements {
            match parser_elm {
                parser::Element::Component {
                    name,
                    type_name,
                    attributes,
                    nested_elements,
                } => {
                    let node_id = match parent_id {
                        Some(parent) => parent.create_nested(name),
                        None => TypeId::from_name(name),
                    };

                    // Process nested elements with the new ID as parent
                    let block = self.build_block_from_elements(nested_elements, Some(&node_id))?;

                    let node = Node {
                        id: node_id,
                        name: name.to_string(),
                        block,
                        type_definition: self
                            .build_element_type_definition(type_name, attributes)?,
                    };

                    elements.push(Element::Node(node));
                }
                parser::Element::Relation {
                    source,
                    target,
                    relation_type,
                    attributes,
                    ..
                } => {
                    // Extract color and width from attributes if they exist
                    let mut color = Color::default();
                    let mut width = 1;

                    for attr in attributes {
                        match attr.name {
                            "color" => color = Color::new(attr.value)?,
                            "width" => {
                                if let Ok(w) = attr.value.parse::<usize>() {
                                    width = w;
                                }
                            }
                            _ => {}
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
                    return Err(FilamentError::Elaboration("Invalid element".to_string()));
                }
            }
        }
        Ok(Scope { elements })
    }

    fn build_element_type_definition(
        &mut self,
        type_name: &str,
        attributes: &[parser::Attribute],
    ) -> Result<Rc<TypeDefinition>, FilamentError> {
        let base = self
            .type_definition_map
            .get(&TypeId::from_name(type_name))
            .ok_or_else(|| {
                FilamentError::Elaboration(format!("Base type '{}' not found", type_name,))
            })?;
        if attributes.is_empty() {
            return Ok(Rc::clone(base));
        }
        let id = TypeId::from_anonymous(self.type_definition_map.len());
        self.insert_type_definition(TypeDefinition::from_base(id, base, attributes)?)
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
            .field("attributes", &self.attributes)
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
        attributes: &[parser::Attribute],
    ) -> Result<Self, FilamentError> {
        let mut type_def = base.clone();
        type_def.id = id;
        let mut attributes = Attribute::new_from_parser(attributes);
        for attr in &attributes {
            match attr.name.0.as_str() {
                "fill_color" => type_def.fill_color = Some(Color::new(attr.value.as_str())?),
                "line_color" => type_def.line_color = Color::new(attr.value.as_str())?,
                "line_width" => {
                    type_def.line_width = attr.value.parse().map_err(|e| {
                        FilamentError::Elaboration(format!("Invalid line_width: {}", e))
                    })?
                }
                "rounded" => {
                    type_def.rounded = attr.value.parse().map_err(|e| {
                        FilamentError::Elaboration(format!("Invalid rounded: {}", e))
                    })?
                }
                "font_size" => {
                    type_def.font_size = attr.value.parse().map_err(|e| {
                        FilamentError::Elaboration(format!("Invalid font_size: {}", e))
                    })?
                }
                _ => {}
            }
        }
        type_def.attributes.append(&mut attributes);

        Ok(type_def)
    }

    fn defaults() -> Vec<Rc<TypeDefinition>> {
        let black = Color::default();
        vec![
            Rc::new(Self {
                id: TypeId::from_name("Rectangle"),
                attributes: vec![],
                fill_color: None,
                line_color: black.clone(),
                line_width: 2,
                rounded: 0,
                font_size: 15,
                shape_type: Rc::new(Rectangle) as Rc<dyn Shape>,
            }),
            Rc::new(Self {
                id: TypeId::from_name("Oval"),
                attributes: vec![],
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

    fn new_from_parser(parser_attrs: &[parser::Attribute]) -> Vec<Self> {
        parser_attrs
            .iter()
            .map(|attr| Self::new(attr.name, attr.value))
            .collect()
    }
}
