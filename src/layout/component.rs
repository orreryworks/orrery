use crate::{
    ast, graph,
    layout::{
        geometry::{self, LayoutSizing, Size},
        layer,
    },
    shape,
};
use log::{debug, error};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// Represents a diagram component with a reference to its AST node and positioning information
/// TODO: Do I need Clone?!
/// Find a better name and location for this struct.
#[derive(Debug, Clone)]
pub struct Component<'a> {
    node: &'a ast::Node, // TODO: Can I get rid of this?
    shape: shape::Shape,
    text: shape::Text,
    position: geometry::Point,
}

impl Component<'_> {
    /// Creates a new component with the specified properties.
    pub fn new<'a>(
        node: &'a ast::Node,
        shape: shape::Shape,
        position: geometry::Point,
    ) -> Component<'a> {
        // TODO: Can we construct the shape here?
        let text = shape::Text::new(
            Rc::clone(&node.type_definition.text_definition),
            node.display_text().to_string(),
        );
        Component {
            node,
            shape,
            text,
            position,
        }
    }

    /// Returns a reference to the component's shape.
    pub fn shape(&self) -> &shape::Shape {
        &self.shape
    }

    /// Returns a reference to the component's text styling and content.
    pub fn text(&self) -> &shape::Text {
        &self.text
    }

    /// Returns the center position of the component.
    ///
    /// The position represents the center point of the component in the layout
    /// coordinate system.
    pub fn position(&self) -> geometry::Point {
        self.position
    }

    /// Calculates the bounds of this component
    ///
    /// The position is treated as the center of the component,
    /// and the bounds extend half the width/height in each direction.
    pub fn bounds(&self) -> geometry::Bounds {
        self.shape.bounds(self.position)
    }

    /// Returns the unique identifier of the AST node this component represents.
    // TODO: Can I get rid of this method?
    pub fn node_id(&self) -> &ast::TypeId {
        &self.node.id
    }

    /// Checks whether this component contains nested diagram blocks.
    ///
    /// Returns `true` if the component's AST node contains nested blocks that
    /// represent sub-diagrams or container structures, `false` otherwise.
    // TODO: Remove this method.
    pub fn has_nested_blocks(&self) -> bool {
        self.node.block.has_nested_blocks()
    }
}

/// Represents a relation (connection) in a component layout with positional information.
///
/// LayoutRelation wraps an AST relation with additional layout-specific data,
/// including the indices of the source and target components within the layout.
/// This allows the layout system to efficiently reference components when
/// positioning and rendering relations.
#[derive(Debug, Clone)]
pub struct LayoutRelation<'a> {
    relation: &'a ast::Relation,
    source_index: usize,
    target_index: usize,
    text: Option<shape::Text>, // Optional text label for the relation
}

impl<'a> LayoutRelation<'a> {
    /// Creates a new LayoutRelation with the given relation and component indices.
    ///
    /// # Arguments
    /// * `relation` - Reference to the AST relation being laid out
    /// * `source_index` - Index of the source component in the layout
    /// * `target_index` - Index of the target component in the layout
    pub fn new(relation: &'a ast::Relation, source_index: usize, target_index: usize) -> Self {
        let text = relation.label.as_ref().map(|label| {
            // HACK: move it to the ast::Relation.
            let mut text_def = shape::TextDefinition::new();
            text_def.set_font_size(14);
            let text_def = Rc::new(RefCell::new(text_def));

            shape::Text::new(text_def, label.clone())
        });
        Self {
            relation,
            source_index,
            target_index,
            text,
        }
    }

    /// Returns a reference to the underlying AST relation.
    ///
    /// This provides access to the relation's properties such as type,
    /// attributes, and labels for rendering purposes.
    pub fn relation(&self) -> &ast::Relation {
        self.relation
    }

    pub fn text(&self) -> Option<&shape::Text> {
        self.text.as_ref()
    }
}

/// Represents a complete layout of components and their relationships.
///
/// A `Layout` contains all the positioned components and their connecting relations
/// for a diagram. It provides methods to access related components and calculate
/// overall layout dimensions.
#[derive(Debug, Clone)]
pub struct Layout<'a> {
    pub components: Vec<Component<'a>>,
    pub relations: Vec<LayoutRelation<'a>>,
}

impl<'a> Layout<'a> {
    /// Returns a reference to the source component of the given relation.
    pub fn source(&self, lr: &LayoutRelation<'a>) -> &Component<'a> {
        &self.components[lr.source_index]
    }

    /// Returns a reference to the target component of the given relation.
    pub fn target(&self, lr: &LayoutRelation<'a>) -> &Component<'a> {
        &self.components[lr.target_index]
    }
}

impl<'a> LayoutSizing for Layout<'a> {
    fn layout_size(&self) -> Size {
        // For component layouts, get the bounding box of all components
        if self.components.is_empty() {
            return Size::default();
        }

        // Calculate bounds from all components
        let bounds = self
            .components
            .iter()
            .skip(1)
            .fold(self.components[0].bounds(), |acc, comp| {
                acc.merge(&comp.bounds())
            });

        bounds.to_size()
    }
}

/// Adjusts the offset of positioned contents in a content stack based on containment relationships.
///
/// This function handles the proper positioning of nested elements within their containers.
///
/// # Arguments
/// * `content_stack` - Mutable reference to the content stack containing all layout layers
/// * `graph` - Reference to the containment graph that defines parent-child relationships
///
/// # Behavior
/// The function processes containment scopes in reverse order to ensure proper nesting.
/// For each nested element, it:
/// 1. Finds the container component in the source layer
/// 2. Calculates the target offset based on the container's bounds and shape properties
/// 3. Updates the destination layer's offset to position the nested content correctly
///
/// # Panics
/// Panics if a component referenced in the containment graph is not found in its
/// corresponding layout layer.
// TODO: Once added enough abstractions, make this a method on ContentStack.
pub fn adjust_positioned_contents_offset<'a>(
    content_stack: &mut layer::ContentStack<Layout<'a>>,
    graph: &'a graph::Graph<'a>,
) {
    let container_indices: HashMap<_, _> = graph
        .containment_scopes()
        .iter()
        .enumerate()
        .filter_map(|(idx, scope)| scope.container().map(|container| (container, idx)))
        .collect();

    for (source_idx, source_scope) in graph.containment_scopes().iter().enumerate().rev() {
        for (node_idx, destination_idx) in source_scope.node_indices().filter_map(|node_idx| {
            container_indices
                .get(&node_idx)
                .map(|&destination_idx| (node_idx, destination_idx))
        }) {
            if source_idx == destination_idx {
                // If the source and destination are the same, skip
                error!(index = source_idx; "Source and destination indices are the same");
                continue;
            }
            let source = content_stack.get_unchecked(source_idx);
            let node = graph.node_from_idx(node_idx);

            // Find the component in the source layer that matches the node
            let source_component = source
                .content()
                .components
                .iter()
                .find(|component| component.node.id == node.id)
                .expect("Component must exist in source layer");
            let target_offset = source
                .offset()
                .add(source_component.bounds().min_point())
                .add(source_component.shape.shape_to_container_min_point()); // TODO: This does not account for text.
            debug!(
                node_id:? = node.id,
                source_offset:? = source.offset();
                "Adjusting positioned content offset [source]",
            );
            let target = content_stack.get_mut_unchecked(destination_idx);
            debug!(
                node_id:? = node.id,
                original_offset:? = target.offset(),
                new_offset:? = target_offset;
                "Adjusting positioned content offset [target]",
            );
            target.set_offset(target_offset);
        }
    }
}
