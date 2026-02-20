//! Type infrastructure for the elaboration phase.
//!
//! This module contains the type definition and attribute extraction utilities
//! used during the elaboration phase of diagram parsing. These types support
//! the transformation from parser AST to semantic model.

use std::{rc::Rc, str::FromStr};

use orrery_core::{color::Color, draw, geometry::Insets, identifier::Id};

use crate::{
    error::{Diagnostic, ErrorCode, Result as DiagnosticResult},
    parser_types,
};

/// Unified drawing definition for types: either a shape or an arrow.
#[derive(Debug, Clone)]
pub enum DrawDefinition {
    Shape(Rc<Box<dyn draw::ShapeDefinition>>),
    Arrow(Rc<draw::ArrowDefinition>),
    Fragment(Rc<draw::FragmentDefinition>),
    Note(Rc<draw::NoteDefinition>),
    ActivationBox(Rc<draw::ActivationBoxDefinition>),
    Stroke(Rc<draw::StrokeDefinition>),
    Text(Rc<draw::TextDefinition>),
}

/// A concrete, elaborated type with text styling and drawing definition.
#[derive(Debug, Clone)]
pub struct TypeDefinition {
    id: Id,
    draw_definition: DrawDefinition,
}

impl TypeDefinition {
    fn new(id: Id, draw_definition: DrawDefinition) -> Self {
        Self {
            id,
            draw_definition,
        }
    }

    /// Construct a concrete shape type definition from a shape definition.
    pub fn new_shape(id: Id, shape_definition: Rc<Box<dyn draw::ShapeDefinition>>) -> Self {
        Self::new(id, DrawDefinition::Shape(shape_definition))
    }

    /// Construct a concrete arrow type definition from an arrow definition.
    pub fn new_arrow(id: Id, arrow_definition: Rc<draw::ArrowDefinition>) -> Self {
        Self::new(id, DrawDefinition::Arrow(arrow_definition))
    }

    /// Construct a concrete fragment type definition from a fragment definition.
    pub fn new_fragment(id: Id, fragment_definition: Rc<draw::FragmentDefinition>) -> Self {
        Self::new(id, DrawDefinition::Fragment(fragment_definition))
    }

    /// Construct a concrete note type definition from a note definition.
    pub fn new_note(id: Id, note_definition: Rc<draw::NoteDefinition>) -> Self {
        Self::new(id, DrawDefinition::Note(note_definition))
    }

    /// Construct a concrete activation box type definition from a activation box definition.
    pub fn new_activation_box(
        id: Id,
        activation_box_definition: Rc<draw::ActivationBoxDefinition>,
    ) -> Self {
        Self::new(id, DrawDefinition::ActivationBox(activation_box_definition))
    }

    /// Construct a concrete stroke type definition from a stroke definition.
    pub fn new_stroke(id: Id, stroke_definition: draw::StrokeDefinition) -> Self {
        Self::new(id, DrawDefinition::Stroke(Rc::new(stroke_definition)))
    }

    /// Construct a concrete text type definition from a text definition.
    pub fn new_text(id: Id, text_definition: draw::TextDefinition) -> Self {
        Self::new(id, DrawDefinition::Text(Rc::new(text_definition)))
    }

    /// Get the identifier for this type definition.
    pub fn id(&self) -> Id {
        self.id
    }

    /// Get a reference to the underlying DrawDefinition.
    pub fn draw_definition(&self) -> &DrawDefinition {
        &self.draw_definition
    }

    /// Borrow the shape definition if this type is a shape; otherwise returns an error.
    pub fn shape_definition(&self) -> Result<&Rc<Box<dyn draw::ShapeDefinition>>, String> {
        match &self.draw_definition {
            DrawDefinition::Shape(shape) => Ok(shape),
            _ => Err(format!("Type '{}' is not a shape type", self.id)),
        }
    }

    /// Borrow the arrow definition if this type is an arrow; otherwise returns an error.
    pub fn arrow_definition(&self) -> Result<&Rc<draw::ArrowDefinition>, String> {
        match &self.draw_definition {
            DrawDefinition::Arrow(arrow) => Ok(arrow),
            _ => Err(format!("Type '{}' is not an arrow type", self.id)),
        }
    }

    /// Borrow the fragment definition if this type is a fragment; otherwise returns an error.
    pub fn fragment_definition(&self) -> Result<&Rc<draw::FragmentDefinition>, String> {
        match &self.draw_definition {
            DrawDefinition::Fragment(fragment) => Ok(fragment),
            _ => Err(format!("Type '{}' is not a fragment type", self.id)),
        }
    }

    /// Borrow the note definition if this type is a note; otherwise returns an error.
    pub fn note_definition(&self) -> Result<&Rc<draw::NoteDefinition>, String> {
        match &self.draw_definition {
            DrawDefinition::Note(note) => Ok(note),
            _ => Err(format!("Type '{}' is not a note type", self.id)),
        }
    }

    /// Borrow the activation box definition if this type is an activation box; otherwise returns an error.
    pub fn activation_box_definition(&self) -> Result<&Rc<draw::ActivationBoxDefinition>, String> {
        match &self.draw_definition {
            DrawDefinition::ActivationBox(activation_box) => Ok(activation_box),
            _ => Err(format!("Type '{}' is not an activation box type", self.id)),
        }
    }

    /// Get the stroke definition Rc if this type is a stroke; otherwise returns an error.
    pub fn stroke_definition(&self) -> Result<&Rc<draw::StrokeDefinition>, String> {
        match &self.draw_definition {
            DrawDefinition::Stroke(stroke) => Ok(stroke),
            _ => Err(format!("Type '{}' is not a stroke type", self.id)),
        }
    }

    /// Get the text definition Rc if this type is a text; otherwise returns an error.
    pub fn text_definition_from_draw(&self) -> Result<&Rc<draw::TextDefinition>, String> {
        match &self.draw_definition {
            DrawDefinition::Text(text) => Ok(text),
            _ => Err(format!("Type '{}' is not a text type", self.id)),
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
    pub fn extract_text_attributes(
        text_def: &mut draw::TextDefinition,
        attrs: &[parser_types::Attribute],
    ) -> DiagnosticResult<()> {
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
    ) -> DiagnosticResult<()> {
        let name = attr.name.inner();
        let value = &attr.value;

        match *name {
            "font_size" => {
                let val = value.as_u16().map_err(|_| {
                        Diagnostic::error(format!("invalid font_size value `{value}`"))
                            .with_code(ErrorCode::E302)
                            .with_label(attr.span(), "invalid number")
                            .with_help("font size must be a positive integer")
                    })?;
                text_def.set_font_size(val);
                Ok(())
            }
            "font_family" => {
                text_def.set_font_family(value.as_str().map_err(|err| {
                    Diagnostic::error(err.to_string())
                        .with_code(ErrorCode::E302)
                        .with_label(attr.span(), "invalid font family")
                        .with_help("font family must be a string value")
                })?);
                Ok(())
            }
            "background_color" => {
                let val = Color::new(value.as_str().map_err(|err| {
                    Diagnostic::error(err.to_string())
                        .with_code(ErrorCode::E302)
                        .with_label(attr.span(), "invalid color value")
                        .with_help("color values must be strings")
                })?)
                .map_err(|err| {
                    Diagnostic::error(format!("invalid background_color: {err}"))
                        .with_code(ErrorCode::E302)
                        .with_label(attr.span(), "invalid color")
                        .with_help("use a CSS color")
                })?;
                text_def.set_background_color(Some(val));
                Ok(())
            }
            "padding" => {
                let val = value.as_float().map_err(|err| {
                    Diagnostic::error(format!("invalid padding value: {err}"))
                        .with_code(ErrorCode::E302)
                        .with_label(attr.span(), "invalid number")
                        .with_help("text padding must be a positive number")
                })?;
                text_def.set_padding(Insets::uniform(val));
                Ok(())
            }
            "color" => {
                let val = Color::new(value.as_str().map_err(|err| {
                    Diagnostic::error(err.to_string())
                        .with_code(ErrorCode::E302)
                        .with_label(attr.span(), "invalid color value")
                        .with_help("color values must be strings")
                })?)
                .map_err(|err| {
                    Diagnostic::error(format!("invalid color: {err}"))
                        .with_code(ErrorCode::E302)
                        .with_label(attr.span(), "invalid color")
                        .with_help("use a CSS color")
                })?;
                text_def.set_color(Some(val));
                Ok(())
            }
            name => Err(Diagnostic::error(format!("unknown text attribute `{name}`"))
                .with_code(ErrorCode::E303)
                .with_label(attr.span(), "unknown attribute")
                .with_help(
                    "valid text attributes are: font_size, font_family, background_color, padding, color",
                )),
        }
    }
}

/// Helper for extracting stroke attributes from nested attribute lists.
pub struct StrokeAttributeExtractor;

impl StrokeAttributeExtractor {
    /// Extract and apply stroke-related attributes to a StrokeDefinition from a group of nested attributes.
    pub fn extract_stroke_attributes(
        stroke_def: &mut draw::StrokeDefinition,
        attrs: &[parser_types::Attribute],
    ) -> DiagnosticResult<()> {
        for attr in attrs {
            Self::extract_single_attribute(stroke_def, attr)?;
        }
        Ok(())
    }

    /// Extract and apply a single stroke-related attribute to a StrokeDefinition.
    fn extract_single_attribute(
        stroke_def: &mut draw::StrokeDefinition,
        attr: &parser_types::Attribute,
    ) -> DiagnosticResult<()> {
        let name = *attr.name.inner();
        let value = &attr.value;

        match name {
            "color" => {
                let color_str = value.as_str().map_err(|err| {
                    Diagnostic::error(err.to_string())
                        .with_code(ErrorCode::E302)
                        .with_label(attr.span(), "invalid color value")
                        .with_help("color values must be strings")
                })?;
                let val = Color::new(color_str).map_err(|err| {
                    Diagnostic::error(format!("invalid stroke color: {err}"))
                        .with_code(ErrorCode::E302)
                        .with_label(attr.span(), "invalid color")
                        .with_help("use a CSS color")
                })?;
                stroke_def.set_color(val);
                Ok(())
            }
            "width" => {
                let val = value.as_float().map_err(|err| {
                    Diagnostic::error(format!("invalid stroke width value: {err}"))
                        .with_code(ErrorCode::E302)
                        .with_label(attr.span(), "invalid number")
                        .with_help("width must be a positive number")
                })?;
                stroke_def.set_width(val);
                Ok(())
            }
            "style" => {
                let style_str = value.as_str().map_err(|err| {
                    Diagnostic::error(err.to_string())
                        .with_code(ErrorCode::E302)
                        .with_label(attr.span(), "invalid stroke style value")
                        .with_help("stroke style must be a string")
                })?;

                let style = draw::StrokeStyle::from_str(style_str).map_err(|err| {
                    Diagnostic::error(err)
                        .with_code(ErrorCode::E302)
                        .with_label(attr.span(), "invalid stroke style")
                        .with_help("use a valid style name or dasharray pattern")
                })?;

                stroke_def.set_style(style);
                Ok(())
            }
            "cap" => {
                let cap_str = value.as_str().map_err(|err| {
                    Diagnostic::error(err.to_string())
                        .with_code(ErrorCode::E302)
                        .with_label(attr.span(), "invalid stroke cap value")
                        .with_help("stroke cap must be a string")
                })?;
                let cap = draw::StrokeCap::from_str(cap_str).map_err(|err| {
                    Diagnostic::error(err)
                        .with_code(ErrorCode::E302)
                        .with_label(attr.span(), "invalid stroke cap")
                        .with_help("valid values are: butt, round, square")
                })?;
                stroke_def.set_cap(cap);
                Ok(())
            }
            "join" => {
                let join_str = value.as_str().map_err(|err| {
                    Diagnostic::error(err.to_string())
                        .with_code(ErrorCode::E302)
                        .with_label(attr.span(), "invalid stroke join value")
                        .with_help("stroke join must be a string")
                })?;
                let join = draw::StrokeJoin::from_str(join_str).map_err(|err| {
                    Diagnostic::error(err)
                        .with_code(ErrorCode::E302)
                        .with_label(attr.span(), "invalid stroke join")
                        .with_help("valid values are: miter, round, bevel")
                })?;
                stroke_def.set_join(join);
                Ok(())
            }
            name => Err(
                Diagnostic::error(format!("unknown stroke attribute `{name}`"))
                    .with_code(ErrorCode::E303)
                    .with_label(attr.span(), "unknown attribute")
                    .with_help("valid stroke attributes are: color, width, style, cap, join"),
            ),
        }
    }
}

#[cfg(test)]
mod elaborate_tests {
    use super::*;
    use crate::span::{Span, Spanned};

    #[test]
    fn test_new_stroke_type() {
        let stroke = draw::StrokeDefinition::default();
        let type_def = TypeDefinition::new_stroke(Id::new("TestStroke"), stroke);
        assert_eq!(type_def.id(), "TestStroke");
        assert!(type_def.stroke_definition().is_ok());
        assert!(type_def.text_definition_from_draw().is_err());
    }

    #[test]
    fn test_new_text_type() {
        let text = draw::TextDefinition::default();
        let type_def = TypeDefinition::new_text(Id::new("TestText"), text);
        assert_eq!(type_def.id(), "TestText");
        assert!(type_def.text_definition_from_draw().is_ok());
        assert!(type_def.stroke_definition().is_err());
    }

    #[test]
    fn test_shape_type_has_text_definition() {
        let type_def = TypeDefinition::new_shape(
            Id::new("Rect"),
            Rc::new(Box::new(draw::RectangleDefinition::new())),
        );
        // Verify shape has embedded text
        assert!(type_def.shape_definition().is_ok());
        let shape_def = type_def.shape_definition().unwrap();
        let _text = shape_def.text(); // Should not panic
        assert!(type_def.stroke_definition().is_err());
    }

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
            create_test_attribute("font_family", create_string_value("Helvetica")),
            create_test_attribute("background_color", create_string_value("red")),
            create_test_attribute("padding", create_float_value(5.0)),
            create_test_attribute("color", create_string_value("blue")),
        ];
        let result = TextAttributeExtractor::extract_text_attributes(&mut text_def, &attributes);
        assert!(result.is_ok());
    }

    #[test]
    fn test_text_attribute_extractor_color_attribute() {
        let mut text_def = draw::TextDefinition::new();
        let attributes = vec![create_test_attribute("color", create_string_value("red"))];
        let result = TextAttributeExtractor::extract_text_attributes(&mut text_def, &attributes);
        assert!(result.is_ok());

        // Test with invalid color value (should be string)
        let mut text_def = draw::TextDefinition::new();
        let attributes = vec![create_test_attribute("color", create_float_value(255.0))];
        let result = TextAttributeExtractor::extract_text_attributes(&mut text_def, &attributes);
        assert!(result.is_err());
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

        if let Err(err) = result {
            let error_message = err.to_string();
            assert!(error_message.contains("unknown text attribute `invalid_attribute`"));
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
    fn test_stroke_attribute_extractor_all_attributes() {
        let attrs = vec![
            create_test_attribute("color", create_string_value("blue")),
            create_test_attribute("width", create_float_value(2.5)),
            create_test_attribute("style", create_string_value("dashed")),
            create_test_attribute("cap", create_string_value("round")),
            create_test_attribute("join", create_string_value("bevel")),
        ];

        let mut stroke_def = draw::StrokeDefinition::default();
        let result = StrokeAttributeExtractor::extract_stroke_attributes(&mut stroke_def, &attrs);

        assert!(result.is_ok());
        assert_eq!(stroke_def.color().to_string(), "blue");
        assert_eq!(stroke_def.width(), 2.5);
        assert_eq!(*stroke_def.style(), draw::StrokeStyle::Dashed);
        assert_eq!(stroke_def.cap(), draw::StrokeCap::Round);
        assert_eq!(stroke_def.join(), draw::StrokeJoin::Bevel);
    }

    #[test]
    fn test_stroke_attribute_extractor_color_only() {
        let attrs = vec![create_test_attribute("color", create_string_value("red"))];

        let mut stroke_def = draw::StrokeDefinition::default();
        let result = StrokeAttributeExtractor::extract_stroke_attributes(&mut stroke_def, &attrs);

        assert!(result.is_ok());
        assert_eq!(stroke_def.color().to_string(), "red");
    }

    #[test]
    fn test_stroke_attribute_extractor_invalid_attribute_name() {
        let attrs = vec![create_test_attribute(
            "invalid_attr",
            create_string_value("value"),
        )];

        let mut stroke_def = draw::StrokeDefinition::default();
        let result = StrokeAttributeExtractor::extract_stroke_attributes(&mut stroke_def, &attrs);

        assert!(result.is_err());
        if let Err(err) = result {
            let error_message = format!("{err}");
            assert!(error_message.contains("unknown stroke attribute"));
            assert!(error_message.contains("invalid_attr"));
        }
    }

    #[test]
    fn test_stroke_attribute_extractor_invalid_color() {
        let attrs = vec![create_test_attribute(
            "color",
            create_string_value("not-a-valid-color-12345"),
        )];

        let mut stroke_def = draw::StrokeDefinition::default();
        let result = StrokeAttributeExtractor::extract_stroke_attributes(&mut stroke_def, &attrs);

        assert!(result.is_err());
        if let Err(err) = result {
            let error_message = format!("{err}");
            assert!(error_message.contains("invalid stroke color"));
        }
    }

    #[test]
    fn test_stroke_attribute_extractor_invalid_cap() {
        let attrs = vec![create_test_attribute("cap", create_string_value("invalid"))];

        let mut stroke_def = draw::StrokeDefinition::default();
        let result = StrokeAttributeExtractor::extract_stroke_attributes(&mut stroke_def, &attrs);

        assert!(result.is_err());
        if let Err(err) = result {
            let error_message = format!("{err}");
            assert!(error_message.contains("invalid stroke cap"));
        }
    }

    #[test]
    fn test_stroke_attribute_extractor_invalid_join() {
        let attrs = vec![create_test_attribute(
            "join",
            create_string_value("invalid"),
        )];

        let mut stroke_def = draw::StrokeDefinition::default();
        let result = StrokeAttributeExtractor::extract_stroke_attributes(&mut stroke_def, &attrs);

        assert!(result.is_err());
        if let Err(err) = result {
            let error_message = format!("{err}");
            assert!(error_message.contains("invalid stroke join"));
        }
    }

    #[test]
    fn test_stroke_attribute_extractor_all_predefined_styles() {
        let styles = vec![
            ("solid", draw::StrokeStyle::Solid),
            ("dashed", draw::StrokeStyle::Dashed),
            ("dotted", draw::StrokeStyle::Dotted),
            ("dash-dot", draw::StrokeStyle::DashDot),
            ("dash-dot-dot", draw::StrokeStyle::DashDotDot),
        ];

        for (style_str, expected_style) in styles {
            let attrs = vec![create_test_attribute(
                "style",
                create_string_value(style_str),
            )];
            let mut stroke_def = draw::StrokeDefinition::default();
            let result =
                StrokeAttributeExtractor::extract_stroke_attributes(&mut stroke_def, &attrs);

            assert!(result.is_ok());
            assert_eq!(*stroke_def.style(), expected_style);
        }
    }
}
