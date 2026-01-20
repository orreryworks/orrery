//! Elaboration phase for the Filament AST
//!
//! This module transforms the desugared AST from parser types into fully elaborated
//! types ready for layout and rendering. It performs type resolution, validates
//! semantic correctness, and builds the final representation.

use std::{collections::HashMap, rc::Rc, str::FromStr};

use log::{debug, info, trace};

use filament_core::{color::Color, draw, geometry::Insets, identifier::Id, semantic};

use crate::{
    builtin_types, elaborate_utils,
    error::{Diagnostic, ErrorCode, Result},
    parser_types,
    span::{Span, Spanned},
};

/// Configuration for the elaboration phase.
///
/// This struct holds the default layout engine settings that are used
/// when no explicit layout_engine attribute is specified in the diagram.
#[derive(Debug, Clone, Default)]
pub struct ElaborateConfig {
    /// Default layout engine for component diagrams
    pub component_layout: semantic::LayoutEngine,
    /// Default layout engine for sequence diagrams
    pub sequence_layout: semantic::LayoutEngine,
}

impl ElaborateConfig {
    /// Create a new ElaborateConfig with specified layout engines
    pub fn new(
        component_layout: semantic::LayoutEngine,
        sequence_layout: semantic::LayoutEngine,
    ) -> Self {
        Self {
            component_layout,
            sequence_layout,
        }
    }
}

pub struct Builder<'a> {
    cfg: ElaborateConfig,
    type_definitions: HashMap<Id, elaborate_utils::TypeDefinition>,
    _phantom: std::marker::PhantomData<&'a str>,
}

impl<'a> Builder<'a> {
    pub fn new(cfg: ElaborateConfig, _source: &'a str) -> Self {
        let type_definitions = builtin_types::defaults();
        let type_definition_map = type_definitions
            .into_iter()
            .map(|def| (def.id(), def))
            .collect();

        Self {
            cfg,
            type_definitions: type_definition_map,
            _phantom: std::marker::PhantomData,
        }
    }

    // ============================================================================
    // Main Entry Methods
    // ============================================================================

    pub fn build(mut self, diag: &Spanned<parser_types::Element<'a>>) -> Result<semantic::Diagram> {
        debug!("Building elaborated diagram");
        match diag.inner() {
            parser_types::Element::Diagram(diag) => {
                info!("Processing diagram of kind: {}", diag.kind);
                trace!("Type definitions: {:?}", diag.type_definitions);
                trace!("Elements count: {}", diag.elements.len());

                // Update type definitions
                debug!("Updating type definitions");
                self.update_type_direct_definitions(&diag.type_definitions)?;

                let kind = *diag.kind;

                // Build block from elements
                debug!("Building block from elements");
                let block = self.build_block_from_elements(&diag.elements, kind)?;

                // Convert block to scope
                let scope = match block {
                    semantic::Block::None => {
                        debug!("Empty block, using default scope");
                        semantic::Scope::default()
                    }
                    semantic::Block::Scope(scope) => {
                        debug!(
                            elements_len = scope.elements().len();
                            "Using scope from block",
                        );
                        scope
                    }
                    semantic::Block::Diagram(_) => {
                        return Err(Diagnostic::error("nested diagram not allowed")
                            .with_code(ErrorCode::E305)
                            .with_label(diag.kind.span(), "nested diagram")
                            .with_help("diagrams cannot be nested inside other diagrams"));
                    }
                };

                let (layout_engine, background_color, lifeline_definition) =
                    self.extract_diagram_attributes(kind, &diag.attributes)?;

                info!(kind:?; "Diagram elaboration completed successfully");
                Ok(semantic::Diagram::new(
                    kind,
                    scope,
                    layout_engine,
                    background_color,
                    lifeline_definition,
                ))
            }
            _ => Err(Diagnostic::error("invalid element, expected diagram")
                .with_code(ErrorCode::E306)
                .with_label(diag.span(), "expected diagram")),
        }
    }

    // ============================================================================
    // Attribute Value Extraction Helpers
    // ============================================================================
    // These associated functions provide a way to extract
    // and validate attribute values with consistent error messages.

    /// Extract a TypeSpec from an attribute value with contextual error.
    ///
    /// # Arguments
    /// * `attr` - The attribute containing the value
    /// * `key` - Display name for error messages (e.g., "stroke", "text")
    fn extract_type_spec<'b>(
        attr: &'b parser_types::Attribute<'b>,
        key: &str,
    ) -> Result<&'b parser_types::TypeSpec<'b>> {
        attr.value.as_type_spec().map_err(|err| {
            Diagnostic::error(err.to_string())
                .with_code(ErrorCode::E302)
                .with_label(attr.span(), format!("invalid {key} attribute value"))
                .with_help(format!(
                    "{key} attribute must be a type reference or inline attributes"
                ))
        })
    }

    /// Extract a string from an attribute value with contextual error.
    ///
    /// # Arguments
    /// * `attr` - The attribute containing the value
    /// * `key` - Display name for error messages (e.g., "style", "layout_engine")
    fn extract_string<'b>(attr: &'b parser_types::Attribute<'b>, key: &str) -> Result<&'b str> {
        attr.value.as_str().map_err(|err| {
            Diagnostic::error(err.to_string())
                .with_code(ErrorCode::E302)
                .with_label(attr.span(), format!("invalid {key} value"))
                .with_help(format!("{key} values must be strings"))
        })
    }

    /// Extract and parse a color from an attribute value with contextual error.
    /// This performs both string extraction and color parsing in one step.
    ///
    /// # Arguments
    /// * `attr` - The attribute containing the value
    /// * `key` - Display name for error messages (e.g., "fill_color", "background_color")
    fn extract_color(attr: &parser_types::Attribute<'_>, key: &str) -> Result<Color> {
        let color_str = attr.value.as_str().map_err(|err| {
            Diagnostic::error(err.to_string())
                .with_code(ErrorCode::E302)
                .with_label(attr.span(), "invalid color value")
                .with_help("color values must be strings")
        })?;

        Color::new(color_str).map_err(|err| {
            Diagnostic::error(format!("invalid {key} `{color_str}`: {err}"))
                .with_code(ErrorCode::E302)
                .with_label(attr.span(), "invalid color")
                .with_help("use a valid CSS color")
        })
    }

    /// Extract a positive float from an attribute value with contextual error.
    ///
    /// # Arguments
    /// * `attr` - The attribute containing the value
    /// * `key` - Display name for error messages (e.g., "width", "padding")
    fn extract_positive_float(attr: &parser_types::Attribute<'_>, key: &str) -> Result<f32> {
        attr.value.as_float().map_err(|err| {
            Diagnostic::error(err.to_string())
                .with_code(ErrorCode::E302)
                .with_label(attr.span(), format!("invalid {key} value"))
                .with_help(format!("{key} must be a positive number"))
        })
    }

    /// Extract a usize from an attribute value with contextual error.
    ///
    /// # Arguments
    /// * `attr` - The attribute containing the value
    /// * `key` - Display name for error messages (e.g., "rounded")
    /// * `hint` - Additional hint for the error message (e.g., "must be a positive number")
    fn extract_usize(attr: &parser_types::Attribute<'_>, key: &str, hint: &str) -> Result<usize> {
        attr.value.as_usize().map_err(|err| {
            Diagnostic::error(err.to_string())
                .with_code(ErrorCode::E302)
                .with_label(attr.span(), format!("invalid {key} value"))
                .with_help(format!("{key} {hint}"))
        })
    }

    // ============================================================================
    // Type Definition Methods
    // ============================================================================

    // TODO: Change error type so it would not accept a span.
    fn insert_type_definition(
        &mut self,
        type_def: elaborate_utils::TypeDefinition,
        span: Span,
    ) -> Result<elaborate_utils::TypeDefinition> {
        let id = type_def.id();

        // Check if the type already exists
        if self.type_definitions.insert(id, type_def.clone()).is_none() {
            Ok(type_def)
        } else {
            // We could use a span here if we tracked where the duplicate was defined
            // For now, we use a simple error since we don't store that information
            Err(
                Diagnostic::error(format!("type definition `{id}` already exists"))
                    .with_code(ErrorCode::E301)
                    .with_label(span, "duplicate type definition"),
            )
        }
    }

    fn update_type_direct_definitions(
        &mut self,
        type_definitions: &Vec<parser_types::TypeDefinition>,
    ) -> Result<()> {
        for type_def in type_definitions {
            let base_type_name = type_def
                .type_spec
                .type_name
                .as_ref()
                .expect("TypeDefinition should always have a type_name in TypeSpec");

            let base = self
                .type_definitions
                .get(base_type_name.inner())
                .ok_or_else(|| {
                    // Create a rich diagnostic error with source location information
                    self.create_undefined_type_error(
                        base_type_name,
                        &format!("Base type '{}' not found", base_type_name.inner()),
                    )
                })?;

            // Try to create the type definition
            let new_type_def = self.build_type_from_base(
                *type_def.name.inner(),
                base,
                &type_def.type_spec.attributes,
            )?;
            self.insert_type_definition(new_type_def, type_def.span())?;
        }
        Ok(())
    }

    fn build_diagram_from_parser(
        &mut self,
        diag: &parser_types::Element<'a>,
    ) -> Result<semantic::Diagram> {
        match diag {
            parser_types::Element::Diagram(diag) => {
                let kind = *diag.kind;
                // Create a block from the diagram elements
                let block = self.build_block_from_elements(&diag.elements, kind)?;
                let scope = match block {
                    semantic::Block::None => semantic::Scope::default(),
                    semantic::Block::Scope(scope) => scope,
                    semantic::Block::Diagram(_) => {
                        return Err(Diagnostic::error("nested diagram not allowed")
                            .with_code(ErrorCode::E305)
                            .with_label(diag.kind.span(), "nested diagram")
                            .with_help("diagrams cannot be nested inside other diagrams"));
                    }
                };

                let (layout_engine, background_color, lifeline_definition) =
                    self.extract_diagram_attributes(kind, &diag.attributes)?;

                Ok(semantic::Diagram::new(
                    kind,
                    scope,
                    layout_engine,
                    background_color,
                    lifeline_definition,
                ))
            }
            _ => Err(Diagnostic::error("invalid element, expected diagram")
                .with_code(ErrorCode::E306)
                .with_label(diag.span(), "expected diagram")),
        }
    }

    fn build_diagram_from_embedded_diagram(
        &mut self,
        element: &parser_types::Element<'a>,
    ) -> Result<semantic::Diagram> {
        if let parser_types::Element::Diagram(diag) = element {
            let kind = *diag.kind;
            // Create a block from the diagram elements
            let block = self.build_block_from_elements(&diag.elements, kind)?;
            let scope = match block {
                semantic::Block::None => semantic::Scope::default(),
                semantic::Block::Scope(scope) => scope,
                semantic::Block::Diagram(_) => {
                    return Err(Diagnostic::error("nested diagram not allowed")
                        .with_code(ErrorCode::E305)
                        .with_label(diag.kind.span(), "nested diagram")
                        .with_help("diagrams cannot be nested inside other diagrams"));
                }
            };

            let (layout_engine, background_color, lifeline_definition) =
                self.extract_diagram_attributes(kind, &diag.attributes)?;

            Ok(semantic::Diagram::new(
                kind,
                scope,
                layout_engine,
                background_color,
                lifeline_definition,
            ))
        } else {
            Err(Diagnostic::error("expected diagram element")
                .with_code(ErrorCode::E306)
                .with_label(element.span(), "expected diagram"))
        }
    }

    fn build_block_from_elements(
        &mut self,
        parser_elements: &[parser_types::Element<'a>],
        diagram_kind: semantic::DiagramKind,
    ) -> Result<semantic::Block> {
        if parser_elements.is_empty() {
            Ok(semantic::Block::None)
        } else if let parser_types::Element::Diagram { .. } = &parser_elements[0] {
            // This case happens when a diagram is the first element in a block
            Ok(semantic::Block::Diagram(
                self.build_diagram_from_parser(&parser_elements[0])?,
            ))
        } else {
            // Check to make sure no diagrams are mixed with other elements
            for parser_elm in parser_elements {
                if let parser_types::Element::Diagram(diag) = parser_elm {
                    // If we found a diagram mixed with other elements, provide a rich error
                    return Err(Diagnostic::error(
                        "diagram cannot share scope with other elements",
                    )
                    .with_code(ErrorCode::E309)
                    .with_label(diag.kind.span(), "must be sole element")
                    .with_help("a diagram declaration must be the only element in its scope"));
                }
            }

            // If no diagrams were found mixed with other elements, build the scope
            Ok(semantic::Block::Scope(self.build_scope_from_elements(
                parser_elements,
                diagram_kind,
            )?))
        }
    }

    fn build_scope_from_elements(
        &mut self,
        parser_elements: &[parser_types::Element<'a>],
        diagram_kind: semantic::DiagramKind,
    ) -> Result<semantic::Scope> {
        let mut elements = Vec::new();

        for parser_elm in parser_elements {
            let element = match parser_elm {
                parser_types::Element::Component {
                    name,
                    display_name,
                    type_spec,
                    nested_elements,
                } => self.build_component_element(
                    name,
                    display_name,
                    type_spec,
                    nested_elements,
                    parser_elm,
                    diagram_kind,
                )?,
                parser_types::Element::Relation {
                    source,
                    target,
                    relation_type,
                    type_spec,
                    label,
                } => {
                    self.build_relation_element(source, target, relation_type, type_spec, label)?
                }
                parser_types::Element::Diagram(_) => {
                    // This should never happen since we already filtered out invalid elements
                    return Err(Diagnostic::error("invalid element type")
                        .with_code(ErrorCode::E306)
                        .with_label(parser_elm.span(), "invalid element"));
                }
                parser_types::Element::ActivateBlock { .. } => {
                    unreachable!(
                        "ActivateBlock should have been desugared into explicit activate/deactivate statements before elaboration"
                    );
                }
                parser_types::Element::Activate {
                    component,
                    type_spec,
                } => self.build_activate_element(component, type_spec, diagram_kind)?,
                parser_types::Element::Deactivate { component } => {
                    self.build_deactivate_element(component, diagram_kind)?
                }
                parser_types::Element::Fragment(fragment) => {
                    self.build_fragment_element(fragment, diagram_kind)?
                }
                parser_types::Element::AltElseBlock { .. }
                | parser_types::Element::OptBlock { .. }
                | parser_types::Element::LoopBlock { .. }
                | parser_types::Element::ParBlock { .. }
                | parser_types::Element::BreakBlock { .. }
                | parser_types::Element::CriticalBlock { .. } => {
                    unreachable!(
                        "Fragment sugar syntax should have been desugared into Fragment elements before elaboration"
                    );
                }
                parser_types::Element::Note(note) => self.build_note_element(note, diagram_kind)?,
            };
            elements.push(element);
        }
        Ok(semantic::Scope::new(elements))
    }

    /// Builds a component element from parser data
    fn build_component_element(
        &mut self,
        name: &Spanned<Id>,
        display_name: &Option<Spanned<String>>,
        type_spec: &parser_types::TypeSpec,
        nested_elements: &[parser_types::Element<'a>],
        parser_elm: &parser_types::Element,
        diagram_kind: semantic::DiagramKind,
    ) -> Result<semantic::Element> {
        let type_def = self.build_type_definition(type_spec)?;

        let shape_def = type_def.shape_definition().map_err(|err| {
            Diagnostic::error(err)
                .with_code(ErrorCode::E307)
                .with_label(type_spec.span(), "invalid shape type")
        })?;

        if !nested_elements.is_empty() && !shape_def.supports_content() {
            let type_name = type_spec
                .type_name
                .as_ref()
                .map_or(type_def.id(), |name| *name.inner());
            return Err(Diagnostic::error(format!(
                "shape type `{type_name}` does not support nested content"
            ))
            .with_code(ErrorCode::E308)
            .with_label(parser_elm.span(), "content not supported")
            .with_help(format!(
                "shape `{type_name}` is content-free and cannot contain nested elements or embedded diagrams"
            )));
        }

        // Check if there's a nested diagram element
        let block = if nested_elements.len() == 1
            && matches!(&nested_elements[0], parser_types::Element::Diagram(_))
        {
            // Handle a single diagram element specially
            let elaborated_diagram =
                self.build_diagram_from_embedded_diagram(&nested_elements[0])?;
            semantic::Block::Diagram(elaborated_diagram)
        } else {
            // Process regular nested elements
            self.build_block_from_elements(nested_elements, diagram_kind)?
        };

        let node = semantic::Node::new(
            *name.inner(),
            name.to_string(),
            display_name.as_ref().map(|n| n.to_string()),
            block,
            Rc::clone(shape_def),
        );

        Ok(semantic::Element::Node(node))
    }

    /// Builds a relation element from parser data
    fn build_relation_element(
        &mut self,
        source: &Spanned<Id>,
        target: &Spanned<Id>,
        relation_type: &Spanned<&str>,
        type_spec: &parser_types::TypeSpec<'a>,
        label: &Option<Spanned<String>>,
    ) -> Result<semantic::Element> {
        // Extract relation type definition from type_spec
        let relation_type_def = self.build_type_definition(type_spec)?;

        let arrow_def = relation_type_def.arrow_definition().map_err(|err| {
            Diagnostic::error(err)
                .with_code(ErrorCode::E307)
                .with_label(type_spec.span(), "invalid arrow type")
        })?;

        let arrow_direction = draw::ArrowDirection::from_str(relation_type).map_err(|_| {
            Diagnostic::error(format!("Invalid arrow direction '{relation_type}'"))
                .with_code(ErrorCode::E302)
                .with_label(relation_type.span(), "invalid direction")
                .with_help("Arrow direction must be '->', '<-', '<->', or '-'")
        })?;

        Ok(semantic::Element::Relation(semantic::Relation::new(
            *source.inner(),
            *target.inner(),
            arrow_direction,
            label.as_ref().map(|l| l.to_string()),
            Rc::clone(arrow_def),
        )))
    }

    /// Builds an activate element from parser data
    fn build_activate_element(
        &mut self,
        component: &Spanned<Id>,
        type_spec: &parser_types::TypeSpec<'a>,
        diagram_kind: semantic::DiagramKind,
    ) -> Result<semantic::Element> {
        // Only allow activate in sequence diagrams
        if diagram_kind != semantic::DiagramKind::Sequence {
            return Err(Diagnostic::error(
                "Activate statements are only supported in sequence diagrams",
            )
            .with_code(ErrorCode::E304)
            .with_label(component.span(), "activate not allowed here")
            .with_help("Activate statements are used for temporal grouping in sequence diagrams"));
        }

        let activate_type_def = self.build_type_definition(type_spec)?;

        let activation_box_def = activate_type_def
            .activation_box_definition()
            .map_err(|err| {
                Diagnostic::error(err)
                    .with_code(ErrorCode::E307)
                    .with_label(type_spec.span(), "invalid activation box type")
            })?;

        Ok(semantic::Element::Activate(semantic::Activate::new(
            *component.inner(),
            Rc::clone(activation_box_def),
        )))
    }

    /// Builds a deactivate element from parser data
    fn build_deactivate_element(
        &mut self,
        component: &Spanned<Id>,
        diagram_kind: semantic::DiagramKind,
    ) -> Result<semantic::Element> {
        // Only allow deactivate in sequence diagrams
        if diagram_kind != semantic::DiagramKind::Sequence {
            return Err(Diagnostic::error(
                "Deactivate statements are only supported in sequence diagrams",
            )
            .with_code(ErrorCode::E304)
            .with_label(component.span(), "deactivate not allowed here")
            .with_help(
                "Deactivate statements are used for temporal grouping in sequence diagrams",
            ));
        }

        Ok(semantic::Element::Deactivate(*component.inner()))
    }

    /// Builds a fragment element from parser data
    fn build_fragment_element(
        &mut self,
        fragment: &parser_types::Fragment<'a>,
        diagram_kind: semantic::DiagramKind,
    ) -> Result<semantic::Element> {
        // Only allow fragments in sequence diagrams
        if diagram_kind != semantic::DiagramKind::Sequence {
            return Err(Diagnostic::error(
                "Fragment blocks are only supported in sequence diagrams",
            )
            .with_code(ErrorCode::E304)
            .with_label(fragment.span(), "fragment not allowed here")
            .with_help("Fragment blocks are used for grouping in sequence diagrams"));
        }

        // Build the type definition for this fragment
        let type_def = self
            .build_type_definition(&fragment.type_spec)
            .map_err(|_| {
                Diagnostic::error(format!(
                    "invalid fragment type for operation `{}`",
                    fragment.operation.inner()
                ))
                .with_code(ErrorCode::E300)
                .with_label(fragment.operation.span(), "invalid fragment type")
                .with_help("Fragment types must be defined in the type system")
            })?;

        let fragment_def = type_def.fragment_definition().map_err(|err| {
            Diagnostic::error(err)
                .with_code(ErrorCode::E307)
                .with_label(fragment.type_spec.span(), "invalid fragment type")
        })?;

        let mut sections = Vec::new();
        for parser_section in &fragment.sections {
            let scope = self.build_scope_from_elements(&parser_section.elements, diagram_kind)?;
            let elements_vec = scope.elements().to_vec();

            sections.push(semantic::FragmentSection::new(
                parser_section.title.as_ref().map(|t| t.inner().to_string()),
                elements_vec,
            ));
        }

        Ok(semantic::Element::Fragment(semantic::Fragment::new(
            fragment.operation.inner().to_string(),
            sections,
            Rc::clone(fragment_def),
        )))
    }

    fn build_type_definition(
        &mut self,
        type_spec: &parser_types::TypeSpec,
    ) -> Result<elaborate_utils::TypeDefinition> {
        let type_name = type_spec.type_name.as_ref().ok_or_else(|| {
            Diagnostic::error("base type type_spec must have a type name")
                .with_code(ErrorCode::E306)
                .with_label(type_spec.span(), "missing type name")
        })?;
        // Look up the base type
        let Some(base) = self.type_definitions.get(type_name.inner()) else {
            return Err(
                self.create_undefined_type_error(type_name, &format!("Unknown type '{type_name}'"))
            );
        };

        let attributes = &type_spec.attributes;
        // If there are no attributes, just return the base type
        if attributes.is_empty() {
            return Ok(base.clone());
        }

        // Otherwise, create a new anonymous type based on the base type
        let id = Id::from_anonymous(self.type_definitions.len());
        let new_type = self.build_type_from_base(id, base, attributes)?;
        self.insert_type_definition(new_type, type_name.span())
    }

    /// Resolve a text type reference and apply inline attribute overrides.
    ///
    /// # Arguments
    /// * `type_spec` - The type specification with optional type name and attributes
    /// * `current_text` - The current text definition reference from the host shape
    fn resolve_text_type_reference(
        &self,
        type_spec: &parser_types::TypeSpec,
        current_text_rc: &Rc<draw::TextDefinition>,
    ) -> Result<Rc<draw::TextDefinition>> {
        // Step 1: Determine which Rc to use (current or resolved)
        let mut text_rc = if let Some(type_name) = &type_spec.type_name {
            let base_type = self
                .type_definitions
                .get(type_name.inner())
                .ok_or_else(|| {
                    Diagnostic::error(format!("undefined text type `{}`", type_name.inner()))
                        .with_code(ErrorCode::E300)
                        .with_label(type_spec.span(), "undefined type")
                        .with_help("Type must be defined with 'type' statement before use")
                })?;

            let base_text_rc = base_type.text_definition_from_draw().map_err(|err| {
                Diagnostic::error(format!(
                    "type `{}` is not a text type: {}",
                    type_name.inner(),
                    err
                ))
                .with_code(ErrorCode::E307)
                .with_label(type_spec.span(), "invalid type reference")
                .with_help("Only Text types can be used for text attributes")
            })?;

            Rc::clone(base_text_rc)
        } else {
            Rc::clone(current_text_rc)
        };

        // Step 2: If attributes exist, make mutable and apply them
        if !type_spec.attributes.is_empty() {
            let text_def_mut = Rc::make_mut(&mut text_rc);
            elaborate_utils::TextAttributeExtractor::extract_text_attributes(
                text_def_mut,
                &type_spec.attributes,
            )?;
        }

        Ok(text_rc)
    }

    /// Resolve a stroke type reference and apply inline attribute overrides.
    ///
    /// # Arguments
    /// * `type_spec` - The type specification with optional type name and attributes
    /// * `current_stroke` - The current stroke definition reference from the host shape
    fn resolve_stroke_type_reference(
        &self,
        type_spec: &parser_types::TypeSpec,
        current_stroke_rc: &Rc<draw::StrokeDefinition>,
    ) -> Result<Rc<draw::StrokeDefinition>> {
        // Step 1: Determine which Rc to use (current or resolved)
        let mut stroke_rc = if let Some(type_name) = &type_spec.type_name {
            let base_type = self
                .type_definitions
                .get(type_name.inner())
                .ok_or_else(|| {
                    Diagnostic::error(format!("undefined stroke type `{}`", type_name.inner()))
                        .with_code(ErrorCode::E300)
                        .with_label(type_spec.span(), "undefined type")
                        .with_help("Type must be defined with 'type' statement before use")
                })?;

            let base_stroke_rc = base_type.stroke_definition().map_err(|err| {
                Diagnostic::error(format!(
                    "type `{}` is not a stroke type: {}",
                    type_name.inner(),
                    err
                ))
                .with_code(ErrorCode::E307)
                .with_label(type_spec.span(), "invalid type reference")
                .with_help("Only Stroke types can be used for stroke attributes")
            })?;

            Rc::clone(base_stroke_rc)
        } else {
            Rc::clone(current_stroke_rc)
        };

        // Step 2: If attributes exist, make mutable and apply them
        if !type_spec.attributes.is_empty() {
            let stroke_def_mut = Rc::make_mut(&mut stroke_rc);
            elaborate_utils::StrokeAttributeExtractor::extract_stroke_attributes(
                stroke_def_mut,
                &type_spec.attributes,
            )?;
        }

        Ok(stroke_rc)
    }

    /// Build a new type definition from a base type with additional attributes.
    /// This method handles type composition and attribute inheritance with integrated
    /// type reference resolution for text and stroke attributes.
    fn build_type_from_base(
        &self,
        id: Id,
        base: &elaborate_utils::TypeDefinition,
        attributes: &[parser_types::Attribute],
    ) -> Result<elaborate_utils::TypeDefinition> {
        match base.draw_definition() {
            elaborate_utils::DrawDefinition::Shape(shape_def) => {
                let mut new_shape_def = Rc::clone(shape_def);
                let shape_def_mut = Rc::make_mut(&mut new_shape_def);

                for attr in attributes {
                    let name = attr.name.inner();

                    match *name {
                        "fill_color" => {
                            let color = Self::extract_color(attr, "fill_color")?;
                            shape_def_mut.set_fill_color(Some(color)).map_err(|err| {
                                Diagnostic::error(err.to_string())
                                    .with_code(ErrorCode::E304)
                                    .with_label(attr.span(), "unsupported attribute")
                            })?;
                        }
                        "stroke" => {
                            let type_spec = Self::extract_type_spec(attr, "stroke")?;
                            let stroke_rc = self
                                .resolve_stroke_type_reference(type_spec, shape_def_mut.stroke())?;
                            shape_def_mut.set_stroke(stroke_rc);
                        }
                        "rounded" => {
                            let val =
                                Self::extract_usize(attr, "rounded", "must be a positive number")?;
                            shape_def_mut.set_rounded(val).map_err(|err| {
                                Diagnostic::error(err.to_string())
                                    .with_code(ErrorCode::E304)
                                    .with_label(attr.span(), "unsupported attribute")
                            })?;
                        }
                        "text" => {
                            let type_spec = Self::extract_type_spec(attr, "text")?;
                            let text_rc =
                                self.resolve_text_type_reference(type_spec, shape_def_mut.text())?;
                            shape_def_mut.set_text(text_rc);
                        }
                        name => {
                            return Err(Diagnostic::error(format!(
                                "unknown shape attribute `{name}`"
                            ))
                            .with_code(ErrorCode::E303)
                            .with_label(attr.span(), "unknown attribute")
                            .with_help(
                                "Valid shape attributes are: fill_color, stroke=[...], rounded, text=[...]",
                            ));
                        }
                    }
                }

                Ok(elaborate_utils::TypeDefinition::new_shape(
                    id,
                    new_shape_def,
                ))
            }
            elaborate_utils::DrawDefinition::Arrow(arrow_def) => {
                let mut new_arrow_def = Rc::clone(arrow_def);
                let arrow_def_mut = Rc::make_mut(&mut new_arrow_def);

                for attr in attributes {
                    let name = attr.name.inner();

                    match *name {
                        "stroke" => {
                            let type_spec = Self::extract_type_spec(attr, "stroke")?;
                            let stroke_rc = self
                                .resolve_stroke_type_reference(type_spec, arrow_def_mut.stroke())?;
                            arrow_def_mut.set_stroke(stroke_rc);
                        }
                        "style" => {
                            let style_str = Self::extract_string(attr, "style")?;
                            let val = draw::ArrowStyle::from_str(style_str).map_err(|_| {
                                Diagnostic::error("invalid arrow style")
                                    .with_code(ErrorCode::E302)
                                    .with_label(attr.span(), "invalid style")
                                    .with_help(
                                        "Arrow style must be 'straight', 'curved', or 'orthogonal'",
                                    )
                            })?;
                            arrow_def_mut.set_style(val);
                        }
                        "text" => {
                            let type_spec = Self::extract_type_spec(attr, "text")?;
                            let text_rc =
                                self.resolve_text_type_reference(type_spec, arrow_def_mut.text())?;
                            arrow_def_mut.set_text(text_rc);
                        }
                        name => {
                            return Err(Diagnostic::error(format!(
                                "unknown arrow attribute `{name}`"
                            ))
                            .with_code(ErrorCode::E303)
                            .with_label(attr.span(), "unknown attribute")
                            .with_help(
                                "Valid arrow attributes are: stroke=[...], style, text=[...]",
                            ));
                        }
                    }
                }

                Ok(elaborate_utils::TypeDefinition::new_arrow(
                    id,
                    new_arrow_def,
                ))
            }
            elaborate_utils::DrawDefinition::Fragment(fragment_def) => {
                let mut new_fragment_def = Rc::clone(fragment_def);
                let fragment_def_mut = Rc::make_mut(&mut new_fragment_def);

                for attr in attributes {
                    let name = attr.name.inner();

                    match *name {
                        "border_stroke" => {
                            let type_spec = Self::extract_type_spec(attr, "border_stroke")?;
                            let stroke_rc = self.resolve_stroke_type_reference(
                                type_spec,
                                fragment_def_mut.border_stroke(),
                            )?;
                            fragment_def_mut.set_border_stroke(stroke_rc);
                        }
                        "background_color" => {
                            let color = Self::extract_color(attr, "background_color")?;
                            fragment_def_mut.set_background_color(Some(color));
                        }
                        "separator_stroke" => {
                            let type_spec = Self::extract_type_spec(attr, "separator_stroke")?;
                            let stroke_rc = self.resolve_stroke_type_reference(
                                type_spec,
                                fragment_def_mut.separator_stroke(),
                            )?;
                            fragment_def_mut.set_separator_stroke(stroke_rc);
                        }
                        "content_padding" => {
                            let val = Self::extract_positive_float(attr, "content_padding")?;
                            fragment_def_mut.set_content_padding(Insets::uniform(val));
                        }
                        "operation_label_text" => {
                            let type_spec = Self::extract_type_spec(attr, "operation_label_text")?;
                            let text_rc = self.resolve_text_type_reference(
                                type_spec,
                                fragment_def_mut.operation_label_text(),
                            )?;
                            fragment_def_mut.set_operation_label_text(text_rc);
                        }
                        "section_title_text" => {
                            let type_spec = Self::extract_type_spec(attr, "section_title_text")?;
                            let text_rc = self.resolve_text_type_reference(
                                type_spec,
                                fragment_def_mut.section_title_text(),
                            )?;
                            fragment_def_mut.set_section_title_text(text_rc);
                        }
                        name => {
                            return Err(Diagnostic::error(format!(
                                "unknown fragment attribute `{name}`"
                            ))
                            .with_code(ErrorCode::E303)
                            .with_label(attr.span(), "unknown attribute")
                            .with_help("Valid fragment attributes are: border_stroke=[...], separator_stroke=[...], background_color, content_padding, operation_label_text=[...], section_title_text=[...]"));
                        }
                    }
                }

                Ok(elaborate_utils::TypeDefinition::new_fragment(
                    id,
                    new_fragment_def,
                ))
            }
            elaborate_utils::DrawDefinition::Note(note_def) => {
                let mut new_note_def = Rc::clone(note_def);
                let note_def_mut = Rc::make_mut(&mut new_note_def);

                for attr in attributes {
                    let name = attr.name.inner();

                    match *name {
                        "background_color" => {
                            let color = Self::extract_color(attr, "background_color")?;
                            note_def_mut.set_background_color(Some(color));
                        }
                        "stroke" => {
                            let type_spec = Self::extract_type_spec(attr, "stroke")?;
                            let stroke_rc = self
                                .resolve_stroke_type_reference(type_spec, note_def_mut.stroke())?;
                            note_def_mut.set_stroke(stroke_rc);
                        }
                        "text" => {
                            let type_spec = Self::extract_type_spec(attr, "text")?;
                            let text_rc =
                                self.resolve_text_type_reference(type_spec, note_def_mut.text())?;
                            note_def_mut.set_text(text_rc);
                        }
                        "on" | "align" => {
                            // Skip positioning attributes - these are handled by build_note_element
                            // and are not part of the note's styling definition
                        }
                        name => {
                            return Err(Diagnostic::error(format!(
                                "unknown note attribute `{name}`"
                            ))
                            .with_code(ErrorCode::E303)
                            .with_label(attr.span(), "unknown attribute")
                            .with_help(
                                "Valid note attributes are: background_color, stroke=[...], text=[...]",
                            ));
                        }
                    }
                }

                Ok(elaborate_utils::TypeDefinition::new_note(id, new_note_def))
            }
            elaborate_utils::DrawDefinition::ActivationBox(activation_box_def) => {
                let mut new_activation_box_def = Rc::clone(activation_box_def);
                let activation_box_def_mut = Rc::make_mut(&mut new_activation_box_def);

                for attr in attributes {
                    let name = attr.name.inner();

                    match *name {
                        "width" => {
                            let val = Self::extract_positive_float(attr, "width")?;
                            activation_box_def_mut.set_width(val);
                        }
                        "nesting_offset" => {
                            let val = Self::extract_positive_float(attr, "nesting_offset")?;
                            activation_box_def_mut.set_nesting_offset(val);
                        }
                        "fill_color" => {
                            let color = Self::extract_color(attr, "fill_color")?;
                            activation_box_def_mut.set_fill_color(color);
                        }
                        "stroke" => {
                            let type_spec = Self::extract_type_spec(attr, "stroke")?;
                            let stroke_rc = self.resolve_stroke_type_reference(
                                type_spec,
                                activation_box_def_mut.stroke(),
                            )?;
                            activation_box_def_mut.set_stroke(stroke_rc);
                        }
                        name => {
                            return Err(Diagnostic::error(format!(
                                "unknown activation box attribute `{name}`"
                            ))
                            .with_code(ErrorCode::E303)
                            .with_label(attr.span(), "unknown attribute")
                            .with_help("Valid activation box attributes are: width, nesting_offset, fill_color, stroke=[...]"));
                        }
                    }
                }

                Ok(elaborate_utils::TypeDefinition::new_activation_box(
                    id,
                    new_activation_box_def,
                ))
            }
            elaborate_utils::DrawDefinition::Stroke(stroke_def) => {
                let mut new_stroke = (**stroke_def).clone();
                elaborate_utils::StrokeAttributeExtractor::extract_stroke_attributes(
                    &mut new_stroke,
                    attributes,
                )?;
                Ok(elaborate_utils::TypeDefinition::new_stroke(id, new_stroke))
            }
            elaborate_utils::DrawDefinition::Text(text_def) => {
                let mut new_text_def = (**text_def).clone();
                elaborate_utils::TextAttributeExtractor::extract_text_attributes(
                    &mut new_text_def,
                    attributes,
                )?;
                Ok(elaborate_utils::TypeDefinition::new_text(id, new_text_def))
            }
        }
    }

    /// Creates a standardized error for undefined type situations
    fn create_undefined_type_error(&self, span: &Spanned<Id>, message: &str) -> Diagnostic {
        Diagnostic::error(message)
            .with_code(ErrorCode::E300)
            .with_label(span.span(), "undefined type")
            .with_help(format!(
                "type `{}` must be a built-in type or defined with a 'type' statement before it can be used as a base type",
                span.inner()
            ))
    }

    /// Extract diagram attributes (layout engine, background color, and lifeline definition)
    fn extract_diagram_attributes(
        &self,
        kind: semantic::DiagramKind,
        attrs: &Vec<parser_types::Attribute<'_>>,
    ) -> Result<(
        semantic::LayoutEngine,
        Option<Color>,
        Option<Rc<draw::LifelineDefinition>>,
    )> {
        // Set the default layout engine based on the diagram kind and config
        let mut layout_engine = match kind {
            semantic::DiagramKind::Component => self.cfg.component_layout,
            semantic::DiagramKind::Sequence => self.cfg.sequence_layout,
        };

        let mut background_color = None;
        let mut lifeline_definition = None;

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
                "lifeline" => {
                    // Only valid for sequence diagrams
                    if kind != semantic::DiagramKind::Sequence {
                        return Err(Diagnostic::error(
                            "`lifeline` attribute is only valid for sequence diagrams",
                        )
                        .with_code(ErrorCode::E304)
                        .with_label(attr.span(), "invalid attribute"));
                    }
                    let definition = self.extract_lifeline_definition(attr)?;
                    lifeline_definition = Some(Rc::new(definition));
                }
                _ => {
                    return Err(Diagnostic::error(format!(
                        "unsupported diagram attribute `{}`",
                        attr.name
                    ))
                    .with_code(ErrorCode::E303)
                    .with_label(attr.span(), "unsupported attribute"));
                }
            }
        }

        Ok((layout_engine, background_color, lifeline_definition))
    }

    /// Extract background color from an attribute
    fn extract_background_color(color_attr: &parser_types::Attribute<'_>) -> Result<Color> {
        Self::extract_color(color_attr, "background_color")
    }

    /// Extract lifeline definition from an attribute
    fn extract_lifeline_definition(
        &self,
        lifeline_attr: &parser_types::Attribute<'_>,
    ) -> Result<draw::LifelineDefinition> {
        let type_spec = Self::extract_type_spec(lifeline_attr, "lifeline")?;

        // Start with default lifeline stroke
        let default_stroke_rc = Rc::new(draw::StrokeDefinition::dashed(Color::default(), 1.0));

        // Look for stroke attribute
        let stroke_rc =
            if let Some(stroke_attr) = type_spec.attributes.iter().find(|a| *a.name == "stroke") {
                let stroke_type_spec = Self::extract_type_spec(stroke_attr, "stroke")?;

                self.resolve_stroke_type_reference(stroke_type_spec, &default_stroke_rc)?
            } else if !type_spec.attributes.is_empty() {
                return Err(Diagnostic::error(format!(
                    "unknown lifeline attribute `{}`",
                    type_spec.attributes[0].name
                ))
                .with_code(ErrorCode::E303)
                .with_label(type_spec.attributes[0].span(), "unknown attribute")
                .with_help("Valid lifeline attributes are: stroke=[...]"));
            } else {
                default_stroke_rc
            };

        Ok(draw::LifelineDefinition::new(stroke_rc))
    }

    /// Determines the layout engine from an attribute
    fn determine_layout_engine(
        engine_attr: &parser_types::Attribute<'_>,
    ) -> Result<semantic::LayoutEngine> {
        let engine_str = Self::extract_string(engine_attr, "layout_engine")?;
        semantic::LayoutEngine::from_str(engine_str).map_err(|_| {
            Diagnostic::error(format!("invalid `layout_engine` value: `{engine_str}`"))
                .with_code(ErrorCode::E302)
                .with_label(engine_attr.value.span(), "unsupported layout engine")
                .with_help("Supported layout engines are: 'basic', 'sugiyama'")
        })
    }

    /// Build a note element from parser types.
    ///
    /// Converts a parsed note element into an elaborated note with:
    /// - Type definition for styling
    /// - Element IDs for attachment (from 'on' attribute)
    /// - Alignment with diagram-specific defaults (from 'align' attribute)
    /// - Text content
    ///
    /// # Arguments
    ///
    /// * `note` - Parsed note element from the parser
    /// * `diagram_kind` - Diagram type (determines default alignment)
    ///
    /// # Returns
    ///
    /// Returns an `Element::Note`
    fn build_note_element(
        &mut self,
        note: &parser_types::Note,
        diagram_kind: semantic::DiagramKind,
    ) -> Result<semantic::Element> {
        let type_def = self.build_type_definition(&note.type_spec)?;

        // Extract 'on' and 'align' attributes
        let (on, align) = self.extract_note_attributes(&note.type_spec.attributes, diagram_kind)?;

        let content = note.content.inner().to_string();

        // Extract NoteDefinition from TypeDefinition
        let note_def_ref = type_def.note_definition().map_err(|err| {
            Diagnostic::error(err)
                .with_code(ErrorCode::E307)
                .with_label(note.content.span(), "invalid note type")
        })?;
        let note_def = Rc::clone(note_def_ref);

        Ok(semantic::Element::Note(semantic::Note::new(
            on, align, content, note_def,
        )))
    }

    /// Extract 'on' and 'align' attributes from note attributes.
    ///
    /// This method extracts:
    /// - `on`: List of element identifiers converted to IDs
    /// - `align`: Alignment string parsed to NoteAlign enum
    ///
    /// # Arguments
    ///
    /// * `attributes` - Note attributes from the parser
    /// * `diagram_kind` - Diagram type (determines default alignment if not specified)
    ///
    /// # Returns
    ///
    /// Returns `(Vec<Id>, NoteAlign)` tuple with:
    /// - Element IDs (empty vec for margin notes)
    /// - Alignment
    fn extract_note_attributes(
        &mut self,
        attributes: &[parser_types::Attribute],
        diagram_kind: semantic::DiagramKind,
    ) -> Result<(Vec<Id>, semantic::NoteAlign)> {
        let mut on: Option<Vec<Id>> = None;
        let mut align: Option<semantic::NoteAlign> = None;

        for attr in attributes {
            match *attr.name.inner() {
                "on" => {
                    let ids = attr.value.as_identifiers().map_err(|_| {
                        Diagnostic::error("`on` attribute must be a list of element identifiers")
                            .with_code(ErrorCode::E302)
                            .with_label(attr.value.span(), "invalid on value")
                            .with_help("Use syntax: on=[element1, element2]")
                    })?;

                    on = Some(ids.iter().map(|id| *id.inner()).collect());
                }
                "align" => {
                    let align_str = Self::extract_string(attr, "align")?;

                    let alignment = align_str.parse::<semantic::NoteAlign>().map_err(|_| {
                        Diagnostic::error(format!("invalid alignment value: `{}`", align_str))
                            .with_code(ErrorCode::E302)
                            .with_label(attr.value.span(), "invalid alignment")
                            .with_help("Valid values: over, left, right, top, bottom")
                    })?;

                    align = Some(alignment);
                }
                _ => {} // Ignore other attributes (handled by build_type_definition)
            }
        }

        // Apply defaults if not specified
        let on = on.unwrap_or_default();
        let align = align.unwrap_or(match diagram_kind {
            semantic::DiagramKind::Sequence => semantic::NoteAlign::Over,
            semantic::DiagramKind::Component => semantic::NoteAlign::Bottom,
        });

        Ok((on, align))
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
            component: Spanned::new(Id::new("user"), Span::new(0..4)),
            type_spec: parser_types::TypeSpec::default(),
            elements: vec![],
        }];

        let diagram = parser_types::Diagram {
            kind: Spanned::new(semantic::DiagramKind::Component, Span::new(0..9)),
            attributes: vec![],
            type_definitions: vec![],
            elements,
        };

        let spanned_element =
            Spanned::new(parser_types::Element::Diagram(diagram), Span::new(0..100));

        let config = ElaborateConfig::default();
        let builder = Builder::new(config, "test");
        // This should panic due to unreachable!() on ActivateBlock during elaboration
        let _ = builder.build(&spanned_element);
    }

    #[test]
    fn test_explicit_activation_scoping_behavior() {
        // Test that sequence diagrams don't create namespace scopes within activate blocks
        let elements = vec![
            parser_types::Element::Activate {
                component: Spanned::new(Id::new("user"), Span::new(0..4)),
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Activate"), Span::new(0..8))),
                    attributes: vec![],
                },
            },
            parser_types::Element::Relation {
                source: Spanned::new(Id::new("user"), Span::new(0..4)),
                target: Spanned::new(Id::new("server"), Span::new(0..6)),
                relation_type: Spanned::new("->", Span::new(0..2)),
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Arrow"), Span::new(0..5))),
                    attributes: vec![],
                },
                label: Some(Spanned::new("Request".to_string(), Span::new(0..7))),
            },
            parser_types::Element::Relation {
                source: Spanned::new(Id::new("server"), Span::new(0..6)),
                target: Spanned::new(Id::new("database"), Span::new(0..8)),
                relation_type: Spanned::new("->", Span::new(0..2)),
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Arrow"), Span::new(0..5))),
                    attributes: vec![],
                },
                label: Some(Spanned::new("Query".to_string(), Span::new(0..5))),
            },
            parser_types::Element::Deactivate {
                component: Spanned::new(Id::new("user"), Span::new(0..4)),
            },
        ];

        let diagram = parser_types::Diagram {
            kind: Spanned::new(semantic::DiagramKind::Sequence, Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements,
        };

        let spanned_element =
            Spanned::new(parser_types::Element::Diagram(diagram), Span::new(0..100));

        let config = ElaborateConfig::default();
        let builder = Builder::new(config, "test");
        let result = builder.build(&spanned_element);

        assert!(
            result.is_ok(),
            "Sequence diagram with activate block should work"
        );

        let diagram = result.unwrap();
        // After desugaring, relations remain unscoped; ensure names were not prefixed
        for element in diagram.scope().elements() {
            if let semantic::Element::Relation(relation) = element {
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
                component: Spanned::new(Id::new("user"), Span::new(0..4)),
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Activate"), Span::new(0..8))),
                    attributes: vec![],
                },
            },
            parser_types::Element::Relation {
                source: Spanned::new(Id::new("user"), Span::new(0..4)),
                target: Spanned::new(Id::new("server"), Span::new(0..6)),
                relation_type: Spanned::new("->", Span::new(0..2)),
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Arrow"), Span::new(0..5))),
                    attributes: vec![],
                },
                label: Some(Spanned::new(
                    "Initial request".to_string(),
                    Span::new(0..16),
                )),
            },
            parser_types::Element::Activate {
                component: Spanned::new(Id::new("user"), Span::new(0..4)),
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Activate"), Span::new(0..8))),
                    attributes: vec![],
                },
            },
            parser_types::Element::Relation {
                source: Spanned::new(Id::new("user"), Span::new(0..4)),
                target: Spanned::new(Id::new("database"), Span::new(0..8)),
                relation_type: Spanned::new("->", Span::new(0..2)),
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Arrow"), Span::new(0..5))),
                    attributes: vec![],
                },
                label: Some(Spanned::new("Direct query".to_string(), Span::new(0..12))),
            },
            parser_types::Element::Deactivate {
                component: Spanned::new(Id::new("user"), Span::new(0..4)),
            },
            parser_types::Element::Activate {
                component: Spanned::new(Id::new("server"), Span::new(0..6)),
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Activate"), Span::new(0..8))),
                    attributes: vec![],
                },
            },
            parser_types::Element::Relation {
                source: Spanned::new(Id::new("server"), Span::new(0..6)),
                target: Spanned::new(Id::new("cache"), Span::new(0..5)),
                relation_type: Spanned::new("->", Span::new(0..2)),
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Arrow"), Span::new(0..5))),
                    attributes: vec![],
                },
                label: Some(Spanned::new("Cache lookup".to_string(), Span::new(0..12))),
            },
            parser_types::Element::Deactivate {
                component: Spanned::new(Id::new("server"), Span::new(0..6)),
            },
            parser_types::Element::Deactivate {
                component: Spanned::new(Id::new("user"), Span::new(0..4)),
            },
        ];

        let diagram = parser_types::Diagram {
            kind: Spanned::new(semantic::DiagramKind::Sequence, Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements,
        };

        let spanned_element =
            Spanned::new(parser_types::Element::Diagram(diagram), Span::new(0..100));

        let config = ElaborateConfig::default();
        let builder = Builder::new(config, "test");
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
                if let semantic::Element::Activate(activate) = e {
                    Some(activate.component().to_string())
                } else {
                    None
                }
            })
            .collect();
        let deactivations: Vec<_> = elems
            .iter()
            .filter_map(|e| {
                if let semantic::Element::Deactivate(id) = e {
                    Some(id.to_string())
                } else {
                    None
                }
            })
            .collect();
        let relations: Vec<_> = elems
            .iter()
            .filter_map(|e| {
                if let semantic::Element::Relation(r) = e {
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
        let config = ElaborateConfig::default();
        let builder = Builder::new(config, "test");

        // Create a simple sequence diagram with explicit activate
        let elements = vec![
            // Define a component
            parser_types::Element::Component {
                name: Spanned::new(Id::new("user"), Span::new(0..4)),
                display_name: None,
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(5..14))),
                    attributes: vec![],
                },
                nested_elements: vec![],
            },
            // Activate the component
            parser_types::Element::Activate {
                component: Spanned::new(Id::new("user"), Span::new(0..4)),
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Activate"), Span::new(0..8))),
                    attributes: vec![],
                },
            },
            // Deactivate the component
            parser_types::Element::Deactivate {
                component: Spanned::new(Id::new("user"), Span::new(0..4)),
            },
        ];

        let diagram = parser_types::Diagram {
            kind: Spanned::new(semantic::DiagramKind::Sequence, Span::new(0..8)),
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
        if let semantic::Element::Activate(activate) = &elements[1] {
            assert_eq!(
                activate.component().to_string(),
                "user",
                "Activate should reference 'user' component"
            );
        } else {
            panic!("Second element should be Activate");
        }

        // Verify the deactivate element
        if let semantic::Element::Deactivate(id) = &elements[2] {
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
        let config = ElaborateConfig::default();
        let builder = Builder::new(config, "test");

        // Create a component diagram with explicit activate (should fail)
        let elements = vec![
            // Define a component
            parser_types::Element::Component {
                name: Spanned::new(Id::new("user"), Span::new(0..4)),
                display_name: None,
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(5..14))),
                    attributes: vec![],
                },
                nested_elements: vec![],
            },
            // Try to activate the component (should fail)
            parser_types::Element::Activate {
                component: Spanned::new(Id::new("user"), Span::new(0..4)),
                type_spec: parser_types::TypeSpec::default(),
            },
        ];

        let diagram = parser_types::Diagram {
            kind: Spanned::new(semantic::DiagramKind::Component, Span::new(0..9)),
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
                name: Spanned::new(Id::new("user"), Span::new(0..4)),
                display_name: None,
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(0..9))),
                    attributes: vec![],
                },
                nested_elements: vec![],
            },
            parser_types::Element::Component {
                name: Spanned::new(Id::new("server"), Span::new(0..6)),
                display_name: None,
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(0..9))),
                    attributes: vec![],
                },
                nested_elements: vec![],
            },
            parser_types::Element::Component {
                name: Spanned::new(Id::new("database"), Span::new(0..8)),
                display_name: None,
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Rectangle"), Span::new(0..9))),
                    attributes: vec![],
                },
                nested_elements: vec![],
            },
            // activations and relations
            parser_types::Element::Activate {
                component: Spanned::new(Id::new("user"), Span::new(0..4)),
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Activate"), Span::new(0..8))),
                    attributes: vec![],
                },
            },
            parser_types::Element::Relation {
                source: Spanned::new(Id::new("user"), Span::new(0..4)),
                target: Spanned::new(Id::new("server"), Span::new(0..6)),
                relation_type: Spanned::new("->", Span::new(0..2)),
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Arrow"), Span::new(0..5))),
                    attributes: vec![],
                },
                label: Some(Spanned::new("First request".to_string(), Span::new(0..13))),
            },
            parser_types::Element::Activate {
                component: Spanned::new(Id::new("server"), Span::new(0..6)),
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Activate"), Span::new(0..8))),
                    attributes: vec![],
                },
            },
            parser_types::Element::Relation {
                source: Spanned::new(Id::new("server"), Span::new(0..6)),
                target: Spanned::new(Id::new("database"), Span::new(0..8)),
                relation_type: Spanned::new("->", Span::new(0..2)),
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Arrow"), Span::new(0..5))),
                    attributes: vec![],
                },
                label: Some(Spanned::new("Nested query".to_string(), Span::new(0..12))),
            },
            parser_types::Element::Relation {
                source: Spanned::new(Id::new("database"), Span::new(0..8)),
                target: Spanned::new(Id::new("server"), Span::new(0..6)),
                relation_type: Spanned::new("->", Span::new(0..2)),
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Arrow"), Span::new(0..5))),
                    attributes: vec![],
                },
                label: Some(Spanned::new(
                    "Nested response".to_string(),
                    Span::new(0..15),
                )),
            },
            parser_types::Element::Deactivate {
                component: Spanned::new(Id::new("server"), Span::new(0..6)),
            },
            parser_types::Element::Relation {
                source: Spanned::new(Id::new("server"), Span::new(0..6)),
                target: Spanned::new(Id::new("user"), Span::new(0..4)),
                relation_type: Spanned::new("->", Span::new(0..2)),
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Arrow"), Span::new(0..5))),
                    attributes: vec![],
                },
                label: Some(Spanned::new("First response".to_string(), Span::new(0..14))),
            },
            parser_types::Element::Activate {
                component: Spanned::new(Id::new("user"), Span::new(0..4)),
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Activate"), Span::new(0..8))),
                    attributes: vec![],
                },
            },
            parser_types::Element::Relation {
                source: Spanned::new(Id::new("user"), Span::new(0..4)),
                target: Spanned::new(Id::new("server"), Span::new(0..6)),
                relation_type: Spanned::new("->", Span::new(0..2)),
                type_spec: parser_types::TypeSpec {
                    type_name: Some(Spanned::new(Id::new("Arrow"), Span::new(0..5))),
                    attributes: vec![],
                },
                label: Some(Spanned::new("Second request".to_string(), Span::new(0..14))),
            },
            parser_types::Element::Deactivate {
                component: Spanned::new(Id::new("user"), Span::new(0..4)),
            },
            parser_types::Element::Deactivate {
                component: Spanned::new(Id::new("user"), Span::new(0..4)),
            },
        ];

        let diagram = parser_types::Diagram {
            kind: Spanned::new(semantic::DiagramKind::Sequence, Span::new(0..8)),
            attributes: vec![],
            type_definitions: vec![],
            elements,
        };

        let spanned_element =
            Spanned::new(parser_types::Element::Diagram(diagram), Span::new(0..100));

        let config = ElaborateConfig::default();
        let builder = Builder::new(config, "test");
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
            .filter(|e| matches!(e, semantic::Element::Relation(_)))
            .count();
        let activates = elems
            .iter()
            .filter(|e| matches!(e, semantic::Element::Activate(_)))
            .count();
        let deactivates = elems
            .iter()
            .filter(|e| matches!(e, semantic::Element::Deactivate(_)))
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

    #[test]
    fn test_note_with_default_alignment_sequence() {
        let cfg = ElaborateConfig::default();
        let mut builder = Builder::new(cfg, "");

        let note = parser_types::Note {
            type_spec: parser_types::TypeSpec {
                type_name: Some(Spanned::new(Id::new("Note"), Span::new(0..4))),
                attributes: vec![],
            },
            content: Spanned::new("Test note".to_string(), Span::new(0..9)),
        };

        let diagram_kind = semantic::DiagramKind::Sequence;
        let result = builder.build_note_element(&note, diagram_kind);

        assert!(result.is_ok());
        let element = result.unwrap();
        if let semantic::Element::Note(note_elem) = element {
            assert_eq!(note_elem.on().len(), 0); // Margin note
            assert_eq!(note_elem.align(), semantic::NoteAlign::Over); // Sequence default
            assert_eq!(note_elem.content(), "Test note");
        } else {
            panic!("Expected Note element");
        }
    }

    #[test]
    fn test_note_with_default_alignment_component() {
        let cfg = ElaborateConfig::default();
        let mut builder = Builder::new(cfg, "");

        let note = parser_types::Note {
            type_spec: parser_types::TypeSpec {
                type_name: Some(Spanned::new(Id::new("Note"), Span::new(0..4))),
                attributes: vec![],
            },
            content: Spanned::new("Test note".to_string(), Span::new(0..9)),
        };

        let diagram_kind = semantic::DiagramKind::Component;
        let result = builder.build_note_element(&note, diagram_kind);

        assert!(result.is_ok());
        let element = result.unwrap();
        if let semantic::Element::Note(note_elem) = element {
            assert_eq!(note_elem.on().len(), 0); // Margin note
            assert_eq!(note_elem.align(), semantic::NoteAlign::Bottom); // Component default
            assert_eq!(note_elem.content(), "Test note");
        } else {
            panic!("Expected Note element");
        }
    }

    #[test]
    fn test_note_with_styling_attributes() {
        let cfg = ElaborateConfig::default();
        let mut builder = Builder::new(cfg, "");

        let attributes = vec![
            parser_types::Attribute {
                name: Spanned::new("background_color", Span::new(0..16)),
                value: parser_types::AttributeValue::String(Spanned::new(
                    "lightyellow".to_string(),
                    Span::new(0..11),
                )),
            },
            parser_types::Attribute {
                name: Spanned::new("stroke", Span::new(0..6)),
                value: parser_types::AttributeValue::TypeSpec(parser_types::TypeSpec {
                    type_name: None,
                    attributes: vec![
                        parser_types::Attribute {
                            name: Spanned::new("color", Span::new(0..5)),
                            value: parser_types::AttributeValue::String(Spanned::new(
                                "blue".to_string(),
                                Span::new(0..4),
                            )),
                        },
                        parser_types::Attribute {
                            name: Spanned::new("width", Span::new(0..5)),
                            value: parser_types::AttributeValue::Float(Spanned::new(
                                2.0,
                                Span::new(0..3),
                            )),
                        },
                    ],
                }),
            },
            parser_types::Attribute {
                name: Spanned::new("text", Span::new(0..4)),
                value: parser_types::AttributeValue::TypeSpec(parser_types::TypeSpec {
                    type_name: None,
                    attributes: vec![parser_types::Attribute {
                        name: Spanned::new("font_size", Span::new(0..9)),
                        value: parser_types::AttributeValue::Float(Spanned::new(
                            14.0,
                            Span::new(0..2),
                        )),
                    }],
                }),
            },
        ];

        let note = parser_types::Note {
            type_spec: parser_types::TypeSpec {
                type_name: Some(Spanned::new(Id::new("Note"), Span::new(0..4))),
                attributes,
            },
            content: Spanned::new("Styled note".to_string(), Span::new(0..11)),
        };

        let diagram_kind = semantic::DiagramKind::Sequence;
        let result = builder.build_note_element(&note, diagram_kind);

        assert!(result.is_ok());
        let element = result.unwrap();
        if let semantic::Element::Note(note_elem) = element {
            assert_eq!(note_elem.content(), "Styled note");
            assert_eq!(note_elem.align(), semantic::NoteAlign::Over); // Default for sequence
            assert_eq!(note_elem.on().len(), 0); // Margin note
        } else {
            panic!("Expected Note element");
        }
    }

    // ============================================================================
    // Extraction Helper Tests
    // ============================================================================

    #[test]
    fn test_extract_type_spec_success() {
        use crate::parser_types::{Attribute, AttributeValue, TypeSpec};

        let type_spec = TypeSpec {
            type_name: Some(Spanned::new(Id::new("BoldText"), Span::new(0..8))),
            attributes: vec![],
        };
        let attr = Attribute {
            name: Spanned::new("text", Span::new(0..4)),
            value: AttributeValue::TypeSpec(type_spec),
        };

        let result = Builder::extract_type_spec(&attr, "text");
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_type_spec_error() {
        use crate::parser_types::{Attribute, AttributeValue};

        let attr = Attribute {
            name: Spanned::new("text", Span::new(0..4)),
            value: AttributeValue::String(Spanned::new(
                "not a type spec".to_string(),
                Span::new(5..20),
            )),
        };

        let result = Builder::extract_type_spec(&attr, "text");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Expected type spec"));
    }

    #[test]
    fn test_extract_string_success() {
        use crate::parser_types::{Attribute, AttributeValue};

        let attr = Attribute {
            name: Spanned::new("style", Span::new(0..5)),
            value: AttributeValue::String(Spanned::new("curved".to_string(), Span::new(6..14))),
        };

        let result = Builder::extract_string(&attr, "style");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "curved");
    }

    #[test]
    fn test_extract_string_error() {
        use crate::parser_types::{Attribute, AttributeValue};

        let attr = Attribute {
            name: Spanned::new("style", Span::new(0..5)),
            value: AttributeValue::Float(Spanned::new(42.0, Span::new(6..8))),
        };

        let result = Builder::extract_string(&attr, "style");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Expected string value"));
    }

    #[test]
    fn test_extract_color_success() {
        use crate::parser_types::{Attribute, AttributeValue};

        let attr = Attribute {
            name: Spanned::new("fill_color", Span::new(0..10)),
            value: AttributeValue::String(Spanned::new("red".to_string(), Span::new(11..16))),
        };

        let result = Builder::extract_color(&attr, "fill_color");
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_color_invalid_string() {
        use crate::parser_types::{Attribute, AttributeValue};

        let attr = Attribute {
            name: Spanned::new("fill_color", Span::new(0..10)),
            value: AttributeValue::Float(Spanned::new(42.0, Span::new(11..13))),
        };

        let result = Builder::extract_color(&attr, "fill_color");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Expected string value"));
    }

    #[test]
    fn test_extract_color_invalid_color() {
        use crate::parser_types::{Attribute, AttributeValue};

        let attr = Attribute {
            name: Spanned::new("fill_color", Span::new(0..10)),
            value: AttributeValue::String(Spanned::new(
                "not-a-color-xyz".to_string(),
                Span::new(11..28),
            )),
        };

        let result = Builder::extract_color(&attr, "fill_color");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("invalid fill_color"));
    }

    #[test]
    fn test_extract_positive_float_success() {
        use crate::parser_types::{Attribute, AttributeValue};

        let attr = Attribute {
            name: Spanned::new("width", Span::new(0..5)),
            value: AttributeValue::Float(Spanned::new(42.5, Span::new(6..10))),
        };

        let result = Builder::extract_positive_float(&attr, "width");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42.5);
    }

    #[test]
    fn test_extract_positive_float_error() {
        use crate::parser_types::{Attribute, AttributeValue};

        let attr = Attribute {
            name: Spanned::new("width", Span::new(0..5)),
            value: AttributeValue::String(Spanned::new(
                "not a number".to_string(),
                Span::new(6..20),
            )),
        };

        let result = Builder::extract_positive_float(&attr, "width");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Expected"));
    }

    #[test]
    fn test_extract_usize_success() {
        use crate::parser_types::{Attribute, AttributeValue};

        let attr = Attribute {
            name: Spanned::new("rounded", Span::new(0..7)),
            value: AttributeValue::Float(Spanned::new(10.0, Span::new(8..10))),
        };

        let result = Builder::extract_usize(&attr, "rounded", "must be a positive number");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 10);
    }

    #[test]
    fn test_extract_usize_error() {
        use crate::parser_types::{Attribute, AttributeValue};

        let attr = Attribute {
            name: Spanned::new("rounded", Span::new(0..7)),
            value: AttributeValue::String(Spanned::new(
                "not a number".to_string(),
                Span::new(8..22),
            )),
        };

        let result = Builder::extract_usize(&attr, "rounded", "must be a positive number");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Expected"));
    }

    #[test]
    fn test_fragment_with_both_text_attributes() {
        use crate::parser_types::{Attribute, AttributeValue, TypeSpec};

        let cfg = ElaborateConfig::default();
        let mut builder = Builder::new(cfg, "");

        // Create a fragment type with both operation_label_text and section_title_text attributes
        let type_spec = TypeSpec {
            type_name: Some(Spanned::new(Id::new("Fragment"), Span::new(0..8))),
            attributes: vec![
                Attribute {
                    name: Spanned::new("operation_label_text", Span::new(0..4)),
                    value: AttributeValue::TypeSpec(TypeSpec {
                        type_name: None,
                        attributes: vec![Attribute {
                            name: Spanned::new("font_size", Span::new(0..9)),
                            value: AttributeValue::Float(Spanned::new(14.0, Span::new(0..2))),
                        }],
                    }),
                },
                Attribute {
                    name: Spanned::new("section_title_text", Span::new(0..18)),
                    value: AttributeValue::TypeSpec(TypeSpec {
                        type_name: None,
                        attributes: vec![Attribute {
                            name: Spanned::new("font_size", Span::new(0..9)),
                            value: AttributeValue::Float(Spanned::new(12.0, Span::new(0..2))),
                        }],
                    }),
                },
            ],
        };

        let result = builder.build_type_definition(&type_spec);
        assert!(
            result.is_ok(),
            "Failed to build type definition with both operation_label_text and section_title_text: {:?}",
            result.err()
        );

        let type_def = result.unwrap();
        // Verify it's a Fragment type definition
        match type_def.draw_definition() {
            elaborate_utils::DrawDefinition::Fragment(_) => {
                // Success - fragment type was created with both operation_label_text and section_title_text attributes
            }
            _ => panic!("Expected Fragment draw definition"),
        }
    }
}
