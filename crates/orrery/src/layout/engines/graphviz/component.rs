//! Graphviz layout engine for component diagrams.
//!
//! Translates [`ComponentGraph`] containment scopes into DOT graphs,
//! invokes Graphviz `dot` for positioning, and maps the results back
//! into Orrery's [`Layout`] representation.
//!
//! # Data Flow
//!
//! ```text
//! ComponentGraph
//!     ↓ build_graphviz_graph
//! dot_structures::Graph
//!     ↓ run_graphviz (dot -Tdot -y)
//! dot_structures::Graph (with pos attributes)
//!     ↓ extract_positions_from_graph
//! HashMap<Id, Point>
//!     ↓ calculate_layout
//! ContentStack<Layout>
//! ```

use std::{collections::HashMap, rc::Rc};

use dot_structures::{
    Attribute, Edge, EdgeTy, Graph as DotGraph, Id as DotId, Node as DotNode, NodeId, Stmt, Vertex,
};
use graphviz_rust::{
    cmd::{CommandArg, Format},
    printer::PrinterContext,
};
use log::{debug, trace};

use orrery_core::{
    draw::{self, ArrowDirection, Drawable},
    geometry::{Insets, Point, Size},
    identifier::Id,
    semantic,
};

use crate::{
    error::RenderError,
    layout::{
        component::{Component, Layout, LayoutRelation, adjust_positioned_contents_offset},
        engines::{ComponentEngine, EmbeddedLayouts},
        layer::{ContentStack, PositionedContent},
    },
    structure::{ComponentGraph, ContainmentScope},
};

/// Points per inch — Graphviz uses inches for node dimensions.
const POINTS_PER_INCH: f32 = 72.0;

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
            // Calculate component shapes - they contain all sizing information
            let mut component_shapes = self.calculate_component_shapes(
                graph,
                containment_scope,
                &positioned_content_sizes,
                embedded_layouts,
            )?;

            // Extract sizes from shapes for Graphviz node sizing
            let component_sizes: HashMap<Id, Size> = component_shapes
                .iter()
                .map(|(idx, shape_with_text)| (*idx, shape_with_text.size()))
                .collect();

            // Calculate positions using Graphviz
            let positions = self.positions(graph, containment_scope, &component_sizes)?;

            // Build the final component list using the pre-configured shapes
            let mut components: Vec<Component> = Vec::new();
            for node in graph.scope_nodes(containment_scope) {
                let position = *positions.get(&node.id()).ok_or_else(|| {
                    RenderError::Layout(format!("position not found for node `{node}`"))
                })?;
                let shape_with_text = component_shapes.remove(&node.id()).ok_or_else(|| {
                    RenderError::Layout(format!("shape not found for node `{node}`"))
                })?;

                // If this node contains an embedded diagram, adjust position to normalize
                // the embedded layout's coordinate system to start at origin
                let final_position = if let semantic::Block::Diagram(_) = node.block()
                    && let Some(layout) = embedded_layouts.get(&node.id())
                {
                    position.add_point(layout.normalize_offset())
                } else {
                    position
                };

                components.push(Component::new(node, shape_with_text, final_position));
            }

            // Map node IDs to their component indices
            let component_indices: HashMap<_, _> = components
                .iter()
                .enumerate()
                .map(|(idx, component)| (component.node_id(), idx))
                .collect();

            // Build the list of relations between components
            let relations: Vec<LayoutRelation> = graph
                .scope_relations(containment_scope)
                .filter_map(|relation| {
                    // Only include relations between visible components
                    // (not including relations within inner blocks)
                    if let (Some(&source_index), Some(&target_index)) = (
                        component_indices.get(&relation.source()),
                        component_indices.get(&relation.target()),
                    ) {
                        Some(LayoutRelation::from_ast(
                            relation,
                            source_index,
                            target_index,
                        ))
                    } else {
                        None
                    }
                })
                .collect();

            let positioned_content = PositionedContent::new(Layout::new(components, relations));

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
                        RenderError::Layout(format!("embedded layout not found for node `{node}`"))
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
                            RenderError::Layout(format!("scope size not found for node `{node}`"))
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

    /// Computes node positions via Graphviz `dot -Tdot -y`.
    ///
    /// Positions are in points (72 per inch). The `-y` flag makes
    /// Graphviz output Y increasing downward, matching Orrery's
    /// coordinate system.
    fn positions(
        &self,
        graph: &ComponentGraph<'_, '_>,
        containment_scope: &ContainmentScope,
        component_sizes: &HashMap<Id, Size>,
    ) -> Result<HashMap<Id, Point>, RenderError> {
        if containment_scope.nodes_count() == 0 {
            return Ok(HashMap::new());
        }

        // Build the Graphviz graph structure
        let gv_graph = self.build_graphviz_graph(graph, containment_scope, component_sizes)?;

        // Execute Graphviz layout
        let laid_out_graph = run_graphviz(gv_graph)?;

        // Extract node positions from the annotated output
        let positions = extract_positions_from_graph(&laid_out_graph)?;

        if positions.len() != containment_scope.nodes_count() {
            return Err(RenderError::Layout(format!(
                "graphviz produced {} positions, expected {}",
                positions.len(),
                containment_scope.nodes_count(),
            )));
        }

        Ok(positions)
    }

    /// Translates a containment scope into a [`DotGraph`].
    ///
    /// Nodes are sized in inches (`fixedsize=true`) and edges carry
    /// `dir`/`constraint` attributes based on [`ArrowDirection`].
    fn build_graphviz_graph(
        &self,
        graph: &ComponentGraph<'_, '_>,
        containment_scope: &ContainmentScope,
        component_sizes: &HashMap<Id, Size>,
    ) -> Result<DotGraph, RenderError> {
        let mut stmts: Vec<Stmt> = Vec::new();

        // Graph-level attributes
        stmts.push(Stmt::Attribute(dot_attr("rankdir", "TB")));
        stmts.push(Stmt::Attribute(dot_attr("nodesep", "0.5")));
        stmts.push(Stmt::Attribute(dot_attr("ranksep", "0.75")));

        // Add nodes with size attributes
        for node in graph.scope_nodes(containment_scope) {
            // Convert size from points/pixels to inches for Graphviz
            let size = component_sizes.get(&node.id()).ok_or_else(|| {
                RenderError::Layout(format!("component size not found for node `{node}`"))
            })?;
            let width_inches = size.width() / POINTS_PER_INCH;
            let height_inches = size.height() / POINTS_PER_INCH;

            let width_str = format!("{width_inches:.4}");
            let height_str = format!("{height_inches:.4}");
            let gv_node = DotNode::new(
                NodeId(DotId::Plain(node.id().to_string()), None),
                vec![
                    dot_attr("shape", "box"),
                    dot_attr("fixedsize", "true"),
                    Attribute(DotId::Plain("width".into()), DotId::Plain(width_str)),
                    Attribute(DotId::Plain("height".into()), DotId::Plain(height_str)),
                ],
            );
            stmts.push(Stmt::Node(gv_node));
        }

        // Add edges for relations within this scope
        for relation in graph.scope_relations(containment_scope) {
            let attributes = match relation.arrow_direction() {
                ArrowDirection::Forward => vec![],
                ArrowDirection::Backward => vec![dot_attr("dir", "back")],
                ArrowDirection::Bidirectional => {
                    vec![dot_attr("dir", "both"), dot_attr("constraint", "false")]
                }
                ArrowDirection::Plain => {
                    vec![dot_attr("dir", "none"), dot_attr("constraint", "false")]
                }
            };
            let edge = Edge {
                ty: EdgeTy::Pair(
                    Vertex::N(NodeId(DotId::Plain(relation.source().to_string()), None)),
                    Vertex::N(NodeId(DotId::Plain(relation.target().to_string()), None)),
                ),
                attributes,
            };
            stmts.push(Stmt::Edge(edge));
        }

        Ok(DotGraph::DiGraph {
            id: DotId::Plain("scope".into()),
            strict: true,
            stmts,
        })
    }
}

/// Creates a DOT [`Attribute`] from a key-value string pair.
fn dot_attr(key: &str, value: &str) -> Attribute {
    Attribute(DotId::Plain(key.into()), DotId::Plain(value.into()))
}

/// Executes Graphviz `dot` and returns the annotated graph.
///
/// Passes `-Tdot` to get DOT output with layout attributes injected,
/// and `-y` to invert the Y-axis to match screen coordinates.
///
/// # Errors
///
/// Returns [`RenderError::Layout`] if:
/// - The `dot` command cannot be spawned.
/// - The process exits with a non-zero status.
/// - The output cannot be re-parsed into a DOT graph.
fn run_graphviz(gv_graph: DotGraph) -> Result<DotGraph, RenderError> {
    // Execute Graphviz with DOT output and -y to invert the Y-axis
    let output = graphviz_rust::exec(
        gv_graph,
        &mut PrinterContext::default(),
        vec![
            CommandArg::Format(Format::Dot),
            CommandArg::Custom("-y".into()),
        ],
    )
    .map_err(|err| {
        RenderError::Layout(format!(
            "cannot execute `dot` command, is Graphviz installed? {err}"
        ))
    })?;

    let dot_output = String::from_utf8(output)
        .map_err(|err| RenderError::Layout(format!("graphviz output is not valid UTF-8: {err}")))?;

    debug!(dot_output:%; "Graphviz DOT output");

    // Re-parse the DOT output into a structured graph
    graphviz_rust::parse(&dot_output)
        .map_err(|err| RenderError::Layout(format!("cannot parse graphviz DOT output: {err}")))
}

/// Extracts node positions from a Graphviz-annotated DOT graph.
///
/// Reads the `pos` attribute (`"x,y"` in points) from each node statement.
/// Positions are used directly — the `-y` flag already aligns output with
/// Orrery's Y-down coordinate system.
///
/// # Errors
///
/// Returns [`RenderError::Layout`] if any node lacks a `pos` attribute
/// or the value cannot be parsed.
fn extract_positions_from_graph(graph: &DotGraph) -> Result<HashMap<Id, Point>, RenderError> {
    let stmts = match graph {
        DotGraph::Graph { stmts, .. } | DotGraph::DiGraph { stmts, .. } => stmts,
    };

    let mut positions = HashMap::new();

    for node in stmts.iter().filter_map(|stmt| match stmt {
        Stmt::Node(node) => Some(node),
        _ => None,
    }) {
        let node_name = dot_id_to_str(&node.id.0);

        let pos_str = find_attribute(&node.attributes, "pos").ok_or_else(|| {
            RenderError::Layout(format!(
                "node `{node_name}` missing `pos` attribute after graphviz layout"
            ))
        })?;

        let pos = parse_pos_str(pos_str).ok_or_else(|| {
            RenderError::Layout(format!(
                "cannot parse `pos` value `{pos_str}` for node `{node_name}`"
            ))
        })?;

        positions.insert(Id::new(node_name), pos);

        trace!(
            node_name,
            pos:?;
            "Extracted Graphviz position",
        );
    }

    Ok(positions)
}

/// Extracts the raw string from a [`DotId`].
///
/// The `Escaped` variant includes surrounding double quotes which are
/// stripped to yield the inner content.
fn dot_id_to_str(id: &DotId) -> &str {
    match id {
        DotId::Escaped(s) => s.trim_matches('"'),
        DotId::Plain(s) | DotId::Html(s) | DotId::Anonymous(s) => s,
    }
}

/// Finds an attribute value by key in a list of DOT attributes.
fn find_attribute<'a>(attributes: &'a [Attribute], key: &str) -> Option<&'a str> {
    attributes.iter().find_map(|Attribute(k, v)| {
        if dot_id_to_str(k) == key {
            Some(dot_id_to_str(v))
        } else {
            None
        }
    })
}

/// Parses a Graphviz `pos` value (`"x,y"`) into a [`Point`].
///
/// Strips an optional trailing `!` (pinned position indicator).
fn parse_pos_str(pos_str: &str) -> Option<Point> {
    let cleaned = pos_str.trim().trim_end_matches('!');
    let (x_str, y_str) = cleaned.split_once(',')?;
    let x: f32 = x_str.trim().parse().ok()?;
    let y: f32 = y_str.trim().parse().ok()?;
    Some(Point::new(x, y))
}

impl ComponentEngine for Engine {
    fn calculate<'a>(
        &self,
        graph: &'a ComponentGraph<'a, '_>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> Result<ContentStack<Layout<'a>>, RenderError> {
        self.calculate_layout(graph, embedded_layouts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pos_value_basic() {
        assert_eq!(parse_pos_str("72,108"), Some(Point::new(72.0, 108.0)));
    }

    #[test]
    fn test_parse_pos_value_with_pin() {
        assert_eq!(parse_pos_str("50.5,75.3!"), Some(Point::new(50.5, 75.3)));
    }

    #[test]
    fn test_parse_pos_value_invalid() {
        assert_eq!(parse_pos_str(""), None);
        assert_eq!(parse_pos_str("abc,def"), None);
        assert_eq!(parse_pos_str("100"), None);
    }

    #[test]
    fn test_extract_positions_from_graph() {
        // Build a graph that simulates Graphviz DOT output with pos attributes
        let graph = DotGraph::DiGraph {
            id: DotId::Plain("scope".into()),
            strict: true,
            stmts: vec![
                Stmt::Node(DotNode::new(
                    NodeId(DotId::Plain("component_a".into()), None),
                    vec![
                        dot_attr("pos", "72,144"),
                        dot_attr("width", "1.0"),
                        dot_attr("height", "0.75"),
                    ],
                )),
                Stmt::Node(DotNode::new(
                    NodeId(DotId::Plain("component_b".into()), None),
                    vec![
                        dot_attr("pos", "72,36"),
                        dot_attr("width", "1.0"),
                        dot_attr("height", "0.75"),
                    ],
                )),
            ],
        };

        let positions = extract_positions_from_graph(&graph).expect("positions extraction failed");

        let id_a = Id::new("component_a");
        let id_b = Id::new("component_b");

        assert_eq!(positions.len(), 2);
        assert!(positions.contains_key(&id_a));
        assert!(positions.contains_key(&id_b));

        let pos_a = positions[&id_a];
        assert!((pos_a.x() - 72.0).abs() < 0.01);
        assert!((pos_a.y() - 144.0).abs() < 0.01);

        let pos_b = positions[&id_b];
        assert!((pos_b.x() - 72.0).abs() < 0.01);
        assert!((pos_b.y() - 36.0).abs() < 0.01);
    }

    #[test]
    fn test_extract_positions_empty_graph() {
        let graph = DotGraph::DiGraph {
            id: DotId::Plain("empty".into()),
            strict: true,
            stmts: vec![],
        };
        let positions = extract_positions_from_graph(&graph).expect("positions extraction failed");
        assert!(positions.is_empty());
    }

    #[test]
    fn test_run_graphviz_and_extract_positions() {
        // This test requires graphviz to be installed
        let gv_graph = DotGraph::DiGraph {
            id: DotId::Plain("test".into()),
            strict: true,
            stmts: vec![
                Stmt::Attribute(dot_attr("rankdir", "TB")),
                Stmt::Node(DotNode::new(
                    NodeId(DotId::Plain("n0".into()), None),
                    vec![
                        dot_attr("shape", "box"),
                        dot_attr("fixedsize", "true"),
                        dot_attr("width", "1.0"),
                        dot_attr("height", "0.75"),
                    ],
                )),
                Stmt::Node(DotNode::new(
                    NodeId(DotId::Plain("n1".into()), None),
                    vec![
                        dot_attr("shape", "box"),
                        dot_attr("fixedsize", "true"),
                        dot_attr("width", "1.0"),
                        dot_attr("height", "0.75"),
                    ],
                )),
                Stmt::Edge(Edge {
                    ty: EdgeTy::Pair(
                        Vertex::N(NodeId(DotId::Plain("n0".into()), None)),
                        Vertex::N(NodeId(DotId::Plain("n1".into()), None)),
                    ),
                    attributes: vec![],
                }),
            ],
        };

        let laid_out_graph = run_graphviz(gv_graph).expect("graphviz execution failed");
        let positions =
            extract_positions_from_graph(&laid_out_graph).expect("positions extraction failed");

        let id0 = Id::new("n0");
        let id1 = Id::new("n1");

        assert_eq!(positions.len(), 2);
        assert!(positions.contains_key(&id0));
        assert!(positions.contains_key(&id1));

        // With rankdir=TB and -y flag: n0 (source) is at top (smaller Y)
        let pos0 = positions[&id0];
        let pos1 = positions[&id1];
        assert!(pos0.y() < pos1.y(), "n0 should be above n1 (smaller Y)");
    }
}
