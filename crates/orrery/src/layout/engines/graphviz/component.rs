//! Graphviz layout engine for component diagrams.
//!
//! This module translates a [`ComponentGraph`] into Graphviz `dot` input,
//! invokes the external `dot` process via [`DotBridge`],
//! and converts the resulting positions and edge splines back into Orrery's
//! [`Layout`] representation.

use std::{collections::HashMap, rc::Rc};

use orrery_core::{
    draw::{self, Drawable},
    geometry::{Insets, Size},
    identifier::Id,
    semantic,
};

use crate::{
    error::RenderError,
    layout::{
        component::{Component, Layout, adjust_positioned_contents_offset},
        engines::{ComponentEngine, EmbeddedLayouts, graphviz::dot_bridge::DotBridge},
        layer::{ContentStack, PositionedContent},
    },
    structure::{ComponentGraph, ContainmentScope},
};

/// Graphviz-based layout engine for component diagrams.
///
/// Computes component positions by invoking the Graphviz `dot` command
/// on a translation of each [`ContainmentScope`] in the
/// [`ComponentGraph`]. Relation directionality influences hierarchical
/// ranking via Graphviz's `constraint` attribute.
///
/// # Examples
///
/// ```ignore
/// # use orrery::layout::engines::graphviz::Component as GraphvizComponent;
/// let engine = GraphvizComponent::new();
/// let layout = engine.calculate(&graph, &embedded_layouts)?;
/// ```
pub struct Engine {
    /// Padding inside container components.
    container_padding: Insets,
}

impl Engine {
    /// Creates a new engine with default container padding.
    pub fn new() -> Self {
        Self {
            container_padding: Insets::uniform(20.0),
        }
    }

    /// Sets the padding inside container components.
    pub fn set_container_padding(&mut self, padding: Insets) -> &mut Self {
        self.container_padding = padding;
        self
    }

    /// Calculates a component layout by delegating to Graphviz.
    ///
    /// Iterates containment scopes in post-order: inner scopes are laid out
    /// first so their sizes are available when sizing their parent containers.
    ///
    /// # Arguments
    ///
    /// * `graph` - The component diagram graph to lay out.
    /// * `embedded_layouts` - Pre-calculated layouts for embedded diagrams,
    ///   indexed by node [`Id`].
    ///
    /// # Returns
    ///
    /// A [`ContentStack`] of component layouts with positions filled in.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Layout`] if Graphviz invocation fails, output
    /// cannot be parsed, or an embedded layout is missing.
    fn calculate_layout<'a>(
        &self,
        graph: &'a ComponentGraph<'a, '_>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> Result<ContentStack<Layout<'a>>, RenderError> {
        let mut content_stack = ContentStack::<Layout<'a>>::new();
        let mut positioned_content_sizes = HashMap::<Id, Size>::new();

        for containment_scope in graph.containment_scopes() {
            let positioned_content = self.layout_containment_scope(
                graph,
                containment_scope,
                &positioned_content_sizes,
                embedded_layouts,
            )?;

            if let Some(container) = containment_scope.container() {
                // If this layer is a container, we need to adjust its size based on its contents
                let size = positioned_content.layout_size();
                positioned_content_sizes.insert(container, size);
            }
            content_stack.push(positioned_content);
        }

        adjust_positioned_contents_offset(&mut content_stack, graph)?;

        Ok(content_stack)
    }

    /// Lays out a single containment scope.
    ///
    /// First computes sized shapes for every node in the scope via
    /// [`calculate_component_shapes`](Self::calculate_component_shapes), then
    /// feeds the resulting sizes into [`DotBridge`] to obtain Graphviz-computed
    /// node positions and edge spline paths. Finally, assembles the positioned
    /// components and relations into a [`Layout`] wrapped in
    /// [`PositionedContent`].
    ///
    /// # Arguments
    ///
    /// * `graph` - The full [`ComponentGraph`] being laid out.
    /// * `containment_scope` - The specific [`ContainmentScope`] to process.
    /// * `positioned_content_sizes` - Sizes of already-laid-out inner scopes.
    /// * `embedded_layouts` - Pre-calculated layouts for embedded diagrams.
    ///
    /// # Returns
    ///
    /// A [`PositionedContent`] wrapping the computed [`Layout`] for this scope.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Layout`] if shape construction, Graphviz
    /// invocation, or result assembly fails.
    fn layout_containment_scope<'a>(
        &self,
        graph: &'a ComponentGraph<'a, '_>,
        containment_scope: &ContainmentScope<'a, 'a>,
        positioned_content_sizes: &HashMap<Id, Size>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> Result<PositionedContent<Layout<'a>>, RenderError> {
        if containment_scope.nodes_count() == 0 {
            return Ok(PositionedContent::new(Layout::new(vec![], vec![])));
        }
        let mut component_shapes = self.calculate_component_shapes(
            graph,
            containment_scope,
            positioned_content_sizes,
            embedded_layouts,
        )?;

        // Extract sizes from shapes for Graphviz node sizing
        let component_sizes: HashMap<Id, Size> = component_shapes
            .iter()
            .map(|(idx, shape_with_text)| (*idx, shape_with_text.size()))
            .collect();

        // Run Graphviz to get node positions and edge paths
        let bridge = DotBridge::new(graph, containment_scope, &component_sizes)?;
        let layout_result = bridge.run()?;

        // Build the final component list using the pre-configured shapes
        let components: Vec<Component> = graph
            .scope_nodes(containment_scope)
            .map(|node| {
                let position = layout_result.position(node.id()).ok_or_else(|| {
                    RenderError::Layout(format!("position not found for `{node}`"))
                })?;
                let shape_with_text = component_shapes
                    .remove(&node.id())
                    .ok_or_else(|| RenderError::Layout(format!("shape not found for `{node}`")))?;

                Ok(Component::new(node, shape_with_text, position))
            })
            .collect::<Result<_, RenderError>>()?;

        // Build relations from the Graphviz edge paths
        let relations: Vec<draw::PositionedArrowWithText> = layout_result
            .into_edge_paths()
            .into_iter()
            .map(|(relation, path)| {
                let arrow_def = Rc::clone(relation.arrow_definition());
                let arrow = draw::Arrow::new(arrow_def, relation.arrow_direction());
                let arrow_with_text = draw::ArrowWithText::new(arrow, relation.text());
                draw::PositionedArrowWithText::new(arrow_with_text, path)
            })
            .collect();

        Ok(PositionedContent::new(Layout::new(components, relations)))
    }

    /// Calculates sized shapes for all components in a containment scope.
    ///
    /// Embedded diagram and inner scope sizes are resolved from previously
    /// computed layouts so that container nodes reserve the correct area.
    fn calculate_component_shapes<'a>(
        &self,
        graph: &'a ComponentGraph<'a, '_>,
        containment_scope: &ContainmentScope,
        positioned_content_sizes: &HashMap<Id, Size>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> Result<HashMap<Id, draw::ShapeWithText<'a>>, RenderError> {
        let mut component_shapes: HashMap<Id, draw::ShapeWithText<'a>> = HashMap::new();

        for node in graph.scope_nodes(containment_scope) {
            let mut shape = draw::Shape::new(Rc::clone(node.shape_definition()));
            shape.set_padding(self.container_padding);
            let text = draw::Text::new(node.shape_definition().text(), node.display_text());
            let mut shape_with_text = draw::ShapeWithText::new(shape, Some(text));

            match node.block() {
                semantic::Block::Diagram(_) => {
                    // Since we process in post-order (innermost to outermost),
                    // embedded diagram layouts should already be calculated and available
                    let layout = embedded_layouts.get(&node.id()).ok_or_else(|| {
                        RenderError::Layout(format!("embedded layout not found for `{node}`"))
                    })?;

                    let content_size = layout.calculate_size();
                    shape_with_text
                        .set_inner_content_size(content_size)
                        .map_err(|err| {
                            RenderError::Layout(format!(
                                "cannot set content size for diagram block `{node}`: {err}"
                            ))
                        })?;
                }
                semantic::Block::Scope(_) => {
                    let content_size =
                        *positioned_content_sizes.get(&node.id()).ok_or_else(|| {
                            RenderError::Layout(format!("scope size not found for `{node}`"))
                        })?;
                    shape_with_text
                        .set_inner_content_size(content_size)
                        .map_err(|err| {
                            RenderError::Layout(format!(
                                "cannot set content size for scope block `{node}`: {err}"
                            ))
                        })?;
                }
                semantic::Block::None => {
                    // No content to size, so don't call set_inner_content_size
                }
            };
            component_shapes.insert(node.id(), shape_with_text);
        }

        Ok(component_shapes)
    }
}

/// [`ComponentEngine`] implementation that delegates to Graphviz.
///
/// See [`Engine::calculate_layout`] for the underlying algorithm.
impl ComponentEngine for Engine {
    fn calculate<'a>(
        &self,
        graph: &'a ComponentGraph<'a, '_>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> Result<ContentStack<Layout<'a>>, RenderError> {
        self.calculate_layout(graph, embedded_layouts)
    }
}
