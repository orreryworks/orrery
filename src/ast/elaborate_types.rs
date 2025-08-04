use crate::{
    ast::parser_types, color::Color, draw, error::ElaborationDiagnosticError, geometry::Insets,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    rc::Rc,
    str::FromStr,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeId(String);

#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: TypeId,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: TypeId,
    pub name: String,
    pub display_name: Option<String>,
    pub block: Block,
    pub type_definition: Rc<TypeDefinition>,
}

impl Node {
    /// Returns the display text for this node
    /// Uses display_name if present, otherwise falls back to the identifier name
    pub fn display_text(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.name)
    }
}

#[derive(Debug, Clone)]
pub struct Relation {
    pub source: TypeId,
    pub target: TypeId,
    pub arrow_direction: draw::ArrowDirection,
    label: Option<String>,
    type_definition: Rc<TypeDefinition>,
}

impl Relation {
    pub fn new(
        source: TypeId,
        target: TypeId,
        arrow_direction: draw::ArrowDirection,
        label: Option<String>,
        type_definition: Rc<TypeDefinition>,
    ) -> Self {
        Self {
            source,
            target,
            arrow_direction,
            label,
            type_definition,
        }
    }

    pub fn text(&self) -> Option<draw::Text> {
        self.label.as_ref().map(|label| {
            draw::Text::new(
                Rc::clone(&self.type_definition.text_definition),
                label.clone(),
            )
        })
    }

    pub fn clone_arrow_definition(&self) -> Rc<draw::ArrowDefinition> {
        Rc::clone(
            self.type_definition
                .arrow_definition()
                .expect("Type definition must have an arrow definition"),
        )
    }
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DiagramKind {
    Component,
    Sequence,
}

#[derive(Debug)]
pub enum DrawDefinition {
    Shape(Rc<dyn draw::ShapeDefinition>),
    Arrow(Rc<draw::ArrowDefinition>),
}

#[derive(Debug)]
pub struct TypeDefinition {
    pub id: TypeId,
    pub text_definition: Rc<draw::TextDefinition>,
    pub draw_definition: DrawDefinition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutEngine {
    Basic,
    Force,
    Sugiyama,
}

impl FromStr for LayoutEngine {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "basic" => Ok(Self::Basic),
            "force" => Ok(Self::Force),
            "sugiyama" => Ok(Self::Sugiyama),
            _ => Err("Unsupported layout engine"),
        }
    }
}

impl From<LayoutEngine> for &'static str {
    fn from(val: LayoutEngine) -> Self {
        match val {
            LayoutEngine::Basic => "basic",
            LayoutEngine::Force => "force",
            LayoutEngine::Sugiyama => "sugiyama",
        }
    }
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::Basic
    }
}

impl Display for LayoutEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s: &'static str = (*self).into();
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone)]
pub struct Diagram {
    pub kind: DiagramKind,
    pub scope: Scope,
    pub layout_engine: LayoutEngine,
    pub background_color: Option<Color>,
}

#[derive(Debug, Clone)]
pub enum Block {
    None,
    Scope(Scope),
    Diagram(Diagram),
}

impl TypeId {
    /// Creates a `TypeId` from a component name as defined in the diagram
    pub fn from_name(name: &str) -> Self {
        Self(name.to_string())
    }

    /// Creates an internal `TypeId` used for generated types
    /// (e.g., for anonymous type definitions)
    pub fn from_anonymous(idx: usize) -> Self {
        Self(format!("__{idx}"))
    }

    /// Creates a nested ID by combining parent ID and child ID with '::' separator
    pub fn create_nested(&self, child_id: &str) -> Self {
        Self(format!("{}::{}", self.0, child_id))
    }
}

impl fmt::Display for TypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TypeDefinition {
    fn new(
        id: TypeId,
        text_definition: draw::TextDefinition,
        draw_definition: DrawDefinition,
    ) -> Self {
        Self {
            id,
            text_definition: Rc::new(text_definition),
            draw_definition,
        }
    }

    pub fn new_shape(
        id: TypeId,
        text_definition: draw::TextDefinition,
        shape_definition: Box<dyn draw::ShapeDefinition>,
    ) -> Self {
        Self::new(
            id,
            text_definition,
            DrawDefinition::Shape(Rc::from(shape_definition)),
        )
    }

    pub fn new_arrow(
        id: TypeId,
        text_definition: draw::TextDefinition,
        arrow_definition: draw::ArrowDefinition,
    ) -> Self {
        Self::new(
            id,
            text_definition,
            DrawDefinition::Arrow(Rc::from(arrow_definition)),
        )
    }

    pub fn shape_definition(&self) -> Result<&Rc<dyn draw::ShapeDefinition>, String> {
        match &self.draw_definition {
            DrawDefinition::Shape(shape) => Ok(shape),
            DrawDefinition::Arrow(_) => Err(format!(
                "Type '{}' is an arrow type, not a shape type",
                self.id
            )),
        }
    }

    pub fn arrow_definition(&self) -> Result<&Rc<draw::ArrowDefinition>, String> {
        match &self.draw_definition {
            DrawDefinition::Arrow(arrow) => Ok(arrow),
            DrawDefinition::Shape(_) => Err(format!(
                "Type '{}' is a shape type, not an arrow type",
                self.id
            )),
        }
    }
}

/// Extractor for text-related attributes that can be applied to TextDefinition
pub struct TextAttributeExtractor;

impl TextAttributeExtractor {
    /// Extract and apply text-related attributes to a TextDefinition from a group of nested attributes.
    ///
    /// Returns `Ok(())` if all attributes were processed successfully,
    /// `Err(...)` if any attribute has an invalid value or is not a valid text attribute.
    fn extract_text_attributes(
        text_def: &mut draw::TextDefinition,
        attrs: &[parser_types::Attribute],
    ) -> Result<(), ElaborationDiagnosticError> {
        for attr in attrs {
            Self::extract_single_attribute(text_def, attr)?;
        }
        Ok(())
    }

    /// Extract and apply a single text-related attribute to a TextDefinition.
    ///
    /// Returns `Ok(())` if the attribute was processed successfully,
    /// `Err(...)` if the attribute has an invalid value or is not a valid text attribute.
    fn extract_single_attribute(
        text_def: &mut draw::TextDefinition,
        attr: &parser_types::Attribute,
    ) -> Result<(), ElaborationDiagnosticError> {
        let name = attr.name.inner();
        let value = &attr.value;

        match *name {
            "font_size" => {
                let val = value.as_u16().map_err(|_| {
                    ElaborationDiagnosticError::from_span(
                        format!("Invalid font_size value '{value}'"),
                        attr.span(),
                        "invalid number",
                        Some("Font size must be a positive integer".to_string()),
                    )
                })?;
                text_def.set_font_size(val);
                Ok(())
            }
            "font_family" => {
                text_def.set_font_family(value.as_str().map_err(|err| {
                    ElaborationDiagnosticError::from_span(
                        err.to_string(),
                        attr.span(),
                        "invalid font family",
                        Some("Font family must be a string value".to_string()),
                    )
                })?);
                Ok(())
            }
            "background_color" => {
                let val = Color::new(value.as_str().map_err(|err| {
                    ElaborationDiagnosticError::from_span(
                        err.to_string(),
                        attr.span(),
                        "invalid color value",
                        Some("Color values must be strings".to_string()),
                    )
                })?)
                .map_err(|err| {
                    ElaborationDiagnosticError::from_span(
                        format!("Invalid background_color: {err}"),
                        attr.span(),
                        "invalid color",
                        Some("Use a CSS color".to_string()),
                    )
                })?;
                text_def.set_background_color(Some(val));
                Ok(())
            }
            "padding" => {
                let val = value.as_float().map_err(|err| {
                    ElaborationDiagnosticError::from_span(
                        format!("Invalid padding value: {err}"),
                        attr.span(),
                        "invalid number",
                        Some("Text padding must be a positive number".to_string()),
                    )
                })?;
                text_def.set_padding(Insets::uniform(val));
                Ok(())
            }
            name => Err(ElaborationDiagnosticError::from_span(
                format!("Unknown text attribute '{name}'"),
                attr.span(),
                "unknown text attribute",
                Some(
                    "Valid text attributes are: font_size, font_family, background_color, padding"
                        .to_string(),
                ),
            )),
        }
    }
}

impl TypeDefinition {
    pub fn from_base(
        id: TypeId,
        base: &Self,
        attributes: &[parser_types::Attribute],
    ) -> Result<Self, ElaborationDiagnosticError> {
        let mut text_def = (*base.text_definition).clone();

        match &base.draw_definition {
            DrawDefinition::Shape(shape_def) => {
                let mut new_shape_def = shape_def.clone_box();

                // Process shape attributes
                for attr in attributes {
                    let name = attr.name.inner();
                    let value = &attr.value;

                    match *name {
                        "fill_color" => {
                            let val = Color::new(value.as_str().map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "invalid color value",
                                    Some("Color values must be strings".to_string()),
                                )
                            })?)
                            .map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    format!("Invalid fill_color '{value}': {err}"),
                                    attr.span(),
                                    "invalid color",
                                    Some("Use a CSS color".to_string()),
                                )
                            })?;
                            new_shape_def.set_fill_color(Some(val)).map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "unsupported attribute",
                                    None,
                                )
                            })?;
                        }
                        "line_color" => {
                            let val = Color::new(value.as_str().map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "invalid color value",
                                    Some("Color values must be strings".to_string()),
                                )
                            })?)
                            .map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    format!("Invalid line_color: {err}"),
                                    attr.span(),
                                    "invalid color",
                                    Some("Use a CSS color".to_string()),
                                )
                            })?;
                            new_shape_def.set_line_color(val).map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "unsupported attribute",
                                    None,
                                )
                            })?;
                        }
                        "line_width" => {
                            let val = value.as_usize().map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    format!("Invalid line_width value: {err}"),
                                    attr.span(),
                                    "invalid number",
                                    Some("Width must be a positive number".to_string()),
                                )
                            })?;
                            new_shape_def.set_line_width(val).map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "unsupported attribute",
                                    None,
                                )
                            })?;
                        }
                        "rounded" => {
                            let val = value.as_usize().map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    format!("Invalid rounded value: {err}"),
                                    attr.span(),
                                    "invalid number",
                                    Some("Rounded must be a positive number".to_string()),
                                )
                            })?;
                            new_shape_def.set_rounded(val).map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "unsupported attribute",
                                    None,
                                )
                            })?;
                        }
                        "text" => {
                            // Handle nested text attributes
                            let nested_attrs = value.as_attributes().map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "invalid text attribute value",
                                    Some("Text attribute must contain nested attributes like [font_size=12, padding=6.5]".to_string()),
                                )
                            })?;

                            // Process all nested text attributes
                            TextAttributeExtractor::extract_text_attributes(
                                &mut text_def,
                                nested_attrs,
                            )?;
                        }
                        name => {
                            return Err(ElaborationDiagnosticError::from_span(
                                format!("Unknown shape attribute '{name}'"),
                                attr.span(),
                                "unknown attribute",
                                Some(
                                    "Valid shape attributes are: fill_color, line_color, line_width, rounded, text=[...]"
                                        .to_string(),
                                ),
                            ));
                        }
                    }
                }

                Ok(Self::new_shape(id, text_def, new_shape_def))
            }
            DrawDefinition::Arrow(arrow_def) => {
                let mut new_arrow_def = (**arrow_def).clone();

                // Process arrow attributes
                for attr in attributes {
                    let name = attr.name.inner();
                    let value = &attr.value;

                    match *name {
                        "color" => {
                            let val = Color::new(value.as_str().map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "invalid color value",
                                    Some("Color values must be strings".to_string()),
                                )
                            })?)
                            .map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    format!("Invalid color: {err}"),
                                    attr.span(),
                                    "invalid color",
                                    Some("Use a CSS color".to_string()),
                                )
                            })?;
                            new_arrow_def.set_color(val);
                        }
                        "width" => {
                            let val = value.as_usize().map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    format!("Invalid width value: {err}"),
                                    attr.span(),
                                    "invalid number",
                                    Some("Width must be a positive number".to_string()),
                                )
                            })?;
                            new_arrow_def.set_width(val);
                        }
                        "style" => {
                            let val =
                                draw::ArrowStyle::from_str(value.as_str().map_err(|err| {
                                    ElaborationDiagnosticError::from_span(
                                        err.to_string(),
                                        attr.span(),
                                        "invalid style value",
                                        Some("Style values must be strings".to_string()),
                                    )
                                })?)
                                .map_err(|_| {
                                    ElaborationDiagnosticError::from_span(
                                    "Invalid arrow style".to_string(),
                                    attr.span(),
                                    "invalid style",
                                    Some(
                                        "Arrow style must be 'straight', 'curved', or 'orthogonal'"
                                            .to_string(),
                                    ),
                                )
                                })?;
                            new_arrow_def.set_style(val);
                        }
                        "text" => {
                            // Handle nested text attributes
                            let nested_attrs = value.as_attributes().map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "invalid text attribute value",
                                    Some("Text attribute must contain nested attributes like [font_size=12, padding=6.5]".to_string()),
                                )
                            })?;

                            // Process all nested text attributes
                            TextAttributeExtractor::extract_text_attributes(
                                &mut text_def,
                                nested_attrs,
                            )?;
                        }
                        name => {
                            return Err(ElaborationDiagnosticError::from_span(
                                format!("Unknown arrow attribute '{name}'"),
                                attr.span(),
                                "unknown attribute",
                                Some(
                                    "Valid arrow attributes are: color, width, style, text=[...]"
                                        .to_string(),
                                ),
                            ));
                        }
                    }
                }

                Ok(Self::new_arrow(id, text_def, new_arrow_def))
            }
        }
    }

    pub fn default_arrow_definition() -> Rc<Self> {
        Rc::from(Self::new_arrow(
            TypeId::from_name("Arrow"),
            draw::TextDefinition::new(),
            draw::ArrowDefinition::new(),
        ))
    }

    pub fn defaults(default_arrow_definition: &Rc<Self>) -> Vec<Rc<Self>> {
        vec![
            Rc::new(Self::new_shape(
                TypeId::from_name("Rectangle"),
                draw::TextDefinition::new(),
                Box::new(draw::RectangleDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                TypeId::from_name("Oval"),
                draw::TextDefinition::new(),
                Box::new(draw::OvalDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                TypeId::from_name("Component"),
                draw::TextDefinition::new(),
                Box::new(draw::ComponentDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                TypeId::from_name("Boundary"),
                draw::TextDefinition::new(),
                Box::new(draw::BoundaryDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                TypeId::from_name("Actor"),
                draw::TextDefinition::new(),
                Box::new(draw::ActorDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                TypeId::from_name("Entity"),
                draw::TextDefinition::new(),
                Box::new(draw::EntityDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                TypeId::from_name("Control"),
                draw::TextDefinition::new(),
                Box::new(draw::ControlDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                TypeId::from_name("Interface"),
                draw::TextDefinition::new(),
                Box::new(draw::InterfaceDefinition::new()),
            )),
            Rc::clone(default_arrow_definition),
        ]
    }
}

#[cfg(test)]
mod elaborate_tests {
    use super::*;
    use crate::ast::span::{Span, Spanned};

    fn create_test_attribute(
        name: &'static str,
        value: parser_types::AttributeValue<'static>,
    ) -> parser_types::Attribute<'static> {
        parser_types::Attribute {
            name: Spanned::new(name, Span::default()),
            value,
        }
    }

    fn create_string_value(s: &str) -> parser_types::AttributeValue<'static> {
        parser_types::AttributeValue::String(Spanned::new(s.to_string(), Span::default()))
    }

    fn create_float_value(f: f32) -> parser_types::AttributeValue<'static> {
        parser_types::AttributeValue::Float(Spanned::new(f, Span::default()))
    }

    #[test]
    fn test_text_attribute_extractor_all_attributes() {
        let mut text_def = draw::TextDefinition::new();

        let attributes = vec![
            create_test_attribute("font_size", create_float_value(16.0)),
            create_test_attribute("font_family", create_string_value("Arial")),
            create_test_attribute("background_color", create_string_value("white")),
            create_test_attribute("padding", create_float_value(8.0)),
        ];

        let result = TextAttributeExtractor::extract_text_attributes(&mut text_def, &attributes);
        assert!(result.is_ok());
    }

    #[test]
    fn test_text_attribute_extractor_empty_attributes() {
        let mut text_def = draw::TextDefinition::new();
        let attributes = vec![];

        let result = TextAttributeExtractor::extract_text_attributes(&mut text_def, &attributes);
        assert!(result.is_ok());
    }

    #[test]
    fn test_text_attribute_extractor_invalid_attribute_name() {
        let mut text_def = draw::TextDefinition::new();
        let attributes = vec![
            create_test_attribute("font_size", create_float_value(16.0)),
            create_test_attribute("invalid_attribute", create_string_value("test")),
        ];

        let result = TextAttributeExtractor::extract_text_attributes(&mut text_def, &attributes);
        assert!(result.is_err());

        if let Err(error) = result {
            let error_message = error.to_string();
            assert!(error_message.contains("Unknown text attribute 'invalid_attribute'"));
        }
    }

    #[test]
    fn test_text_attribute_extractor_invalid_value_types() {
        // Test font_size with string value (should be float)
        let mut text_def = draw::TextDefinition::new();
        let attributes = vec![create_test_attribute(
            "font_size",
            create_string_value("not_a_number"),
        )];
        let result = TextAttributeExtractor::extract_text_attributes(&mut text_def, &attributes);
        assert!(result.is_err());

        // Test font_family with float value (should be string)
        let mut text_def = draw::TextDefinition::new();
        let attributes = vec![create_test_attribute(
            "font_family",
            create_float_value(123.0),
        )];
        let result = TextAttributeExtractor::extract_text_attributes(&mut text_def, &attributes);
        assert!(result.is_err());
    }

    #[test]
    fn test_type_definition_with_text_attributes() {
        // Create a base rectangle type
        let base_id = TypeId::from_name("Rectangle");
        let base_text_def = draw::TextDefinition::new();
        let base_shape_def = Box::new(draw::RectangleDefinition::new());
        let base_type = TypeDefinition::new_shape(base_id, base_text_def, base_shape_def);

        // Create attributes including text group
        let text_attrs = vec![
            create_test_attribute("font_size", create_float_value(14.0)),
            create_test_attribute("font_family", create_string_value("Arial")),
        ];

        let attributes = vec![
            create_test_attribute("fill_color", create_string_value("blue")),
            create_test_attribute("text", parser_types::AttributeValue::Attributes(text_attrs)),
        ];

        // Create new type from base with text attributes
        let new_id = TypeId::from_name("StyledRectangle");
        let result = TypeDefinition::from_base(new_id, &base_type, &attributes);

        assert!(result.is_ok());
    }

    #[test]
    fn test_type_definition_text_not_nested_attributes() {
        // Create a base rectangle type
        let base_id = TypeId::from_name("Rectangle");
        let base_text_def = draw::TextDefinition::new();
        let base_shape_def = Box::new(draw::RectangleDefinition::new());
        let base_type = TypeDefinition::new_shape(base_id, base_text_def, base_shape_def);

        // Try to use text with string value instead of nested attributes
        let attributes = vec![
            create_test_attribute("fill_color", create_string_value("blue")),
            create_test_attribute(
                "text",
                create_string_value("this_should_be_nested_attributes"),
            ),
        ];

        // This should fail because text must contain nested attributes
        let new_id = TypeId::from_name("InvalidTextType");
        let result = TypeDefinition::from_base(new_id, &base_type, &attributes);

        assert!(result.is_err());
        if let Err(error) = result {
            let error_message = format!("{}", error);
            assert!(error_message.contains("Expected nested attributes"));
        }
    }
}
