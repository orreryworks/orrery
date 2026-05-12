//! Bridge between Orrery's semantic model and Graphviz DOT format.
//!
//! This module converts a [`ContainmentScope`] into a DOT graph, invokes the
//! external `dot` command, and maps the annotated output back into Orrery
//! geometry types.
//!
//! # Overview
//!
//! * [`DotBridge`] — builds a DOT graph from a [`ContainmentScope`], executes
//!   Graphviz, and returns a [`DotOutput`].
//! * [`DotOutput`] — holds the computed node positions and per-relation edge
//!   paths.

use std::{collections::HashMap, fmt, io::ErrorKind};

use dot_structures::{
    Attribute, Edge, EdgeTy, Graph as DotGraph, Id as DotId, Node as DotNode, NodeId, Stmt, Vertex,
};
use graphviz_rust::{
    cmd::{CommandArg, Format},
    printer::PrinterContext,
};
use log::{debug, trace};

use orrery_core::{
    draw::{ArrowDirection, ArrowPath},
    geometry::{Point, Size},
    identifier::Id,
    semantic::{Node, Relation},
};

use crate::{
    error::RenderError,
    structure::{ComponentGraph, ContainmentScope},
};

/// Points per inch - Graphviz uses inches for node dimensions.
const POINTS_PER_INCH: f32 = 72.0;

/// Computed node positions and per-relation edge paths produced by Graphviz.
pub struct DotOutput<'a> {
    positions: HashMap<Id, Point>,
    edge_paths: Vec<(&'a Relation, ArrowPath)>,
}

impl<'a> DotOutput<'a> {
    /// Returns the computed position for the given node.
    pub fn position(&self, id: Id) -> Option<Point> {
        self.positions.get(&id).copied()
    }

    /// Consumes this output and returns the per-relation edge paths.
    ///
    /// # Returns
    ///
    /// Returns a `Vec` of `(&Relation, ArrowPath)` tuples.
    pub fn into_edge_paths(self) -> Vec<(&'a Relation, ArrowPath)> {
        self.edge_paths
    }
}

/// Translates a [`ContainmentScope`] to DOT, runs Graphviz, and maps the
/// result back to Orrery types.
///
/// `DotBridge` owns a DOT graph and the mapping between the sequential edge
/// indices emitted as DOT `id` attributes and the original [`Relation`]
pub struct DotBridge<'a> {
    /// Sequential index → `&Relation` — populated during DOT graph
    /// construction, consumed when parsing the Graphviz output.
    edge_map: HashMap<usize, &'a Relation>,
    dot_graph: DotGraph,
}

impl<'a> DotBridge<'a> {
    /// Creates a new `DotBridge` from a containment scope.
    ///
    /// Translates a containment scope into a [`DotGraph`].
    ///
    /// # Arguments
    ///
    /// * `graph` - the [`ComponentGraph`] that owns the nodes and relations.
    /// * `containment_scope` - the subset of the graph to lay out.
    /// * `component_sizes` - pre-measured sizes of component in the scope.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Layout`] if a component size is missing.
    pub fn new(
        graph: &'a ComponentGraph<'a, '_>,
        containment_scope: &ContainmentScope<'a, '_>,
        component_sizes: &HashMap<Id, Size>,
    ) -> Result<Self, RenderError> {
        let mut stmts: Vec<Stmt> = Vec::new();
        let mut edge_map = HashMap::new();

        stmts.push(Stmt::Attribute(dot_attr("rankdir", "TB")));
        stmts.push(Stmt::Attribute(dot_attr("nodesep", "0.5")));
        stmts.push(Stmt::Attribute(dot_attr("ranksep", "0.75")));

        for node in graph.scope_nodes(containment_scope) {
            let size = component_sizes.get(&node.id()).ok_or_else(|| {
                RenderError::Layout(format!("component size not found for `{node}`"))
            })?;
            stmts.push(node_stmt(node, *size));
        }

        for (idx, relation) in graph.scope_relations(containment_scope).enumerate() {
            edge_map.insert(idx, relation);
            stmts.push(edge_stmt(idx, relation));
        }

        let dot_graph = DotGraph::DiGraph {
            id: DotId::Plain("scope".into()),
            strict: false,
            stmts,
        };

        Ok(Self {
            edge_map,
            dot_graph,
        })
    }

    /// Executes Graphviz and maps the annotated output back to Orrery types.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Layout`] if Graphviz execution fails, the
    /// output cannot be parsed, or expected positions/paths are missing.
    pub fn run(self) -> Result<DotOutput<'a>, RenderError> {
        let laid_out_graph = run_graphviz(self.dot_graph)?;

        let positions = extract_positions_from_graph(&laid_out_graph)?;
        let edge_paths = Self::resolve_edge_paths(&laid_out_graph, self.edge_map)?;

        Ok(DotOutput {
            positions,
            edge_paths,
        })
    }

    /// Resolves parsed edge paths back to their semantic relations.
    ///
    /// Pairs each extracted edge path with the [`Relation`] it belongs to,
    /// using the index mapping built during DOT graph construction.
    fn resolve_edge_paths(
        graph: &DotGraph,
        edge_map: HashMap<usize, &'a Relation>,
    ) -> Result<Vec<(&'a Relation, ArrowPath)>, RenderError> {
        let raw_edge_paths = extract_edge_paths_from_graph(graph)?;

        raw_edge_paths
            .into_iter()
            .map(|(idx, path)| {
                edge_map
                    .get(&idx)
                    .map(|edge| (*edge, path))
                    .ok_or(RenderError::Layout(format!(
                        "edge path not found for relation index {idx}"
                    )))
            })
            .collect()
    }
}

/// Creates a DOT `node` statement.
fn node_stmt(node: &Node, size: Size) -> Stmt {
    let width_inches = size.width() / POINTS_PER_INCH;
    let height_inches = size.height() / POINTS_PER_INCH;

    let width_str = format!("{width_inches:.4}");
    let height_str = format!("{height_inches:.4}");
    let gv_node = DotNode::new(
        NodeId(into_dot_id(node.id()), None),
        vec![
            dot_attr("shape", "box"),
            dot_attr("fixedsize", "true"),
            Attribute(DotId::Plain("width".into()), DotId::Plain(width_str)),
            Attribute(DotId::Plain("height".into()), DotId::Plain(height_str)),
        ],
    );
    Stmt::Node(gv_node)
}

/// Creates a DOT `edge` statement tagged with an `id`.
fn edge_stmt(idx: usize, relation: &Relation) -> Stmt {
    let mut attributes = match relation.arrow_direction() {
        ArrowDirection::Forward => vec![],
        ArrowDirection::Backward => vec![dot_attr("dir", "back")],
        ArrowDirection::Bidirectional => {
            vec![dot_attr("dir", "both"), dot_attr("constraint", "false")]
        }
        ArrowDirection::Plain => {
            vec![dot_attr("dir", "none"), dot_attr("constraint", "false")]
        }
    };
    attributes.push(dot_attr("id", &format!("e_{idx}")));

    let edge = Edge {
        ty: EdgeTy::Pair(
            Vertex::N(NodeId(into_dot_id(relation.source()), None)),
            Vertex::N(NodeId(into_dot_id(relation.target()), None)),
        ),
        attributes,
    };
    Stmt::Edge(edge)
}

/// Creates a DOT [`Attribute`] from a key-value string pair.
fn dot_attr(key: &str, value: &str) -> Attribute {
    Attribute(DotId::Plain(key.into()), DotId::Plain(value.into()))
}

/// Creates a quoted DOT node identifier.
///
/// Wraps the identifier in double quotes so that namespacing characters (e.g. `::`) are
/// treated as literal parts of the name rather than DOT syntax. This works around
/// `graphviz_rust`'s printer emitting all [`DotId`] variants verbatim without quoting.
///
/// # Arguments
///
/// * `id` - The node identifier to quote.
fn into_dot_id(id: impl fmt::Display) -> DotId {
    DotId::Plain(format!("\"{id}\""))
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

/// Finds an attribute value by key in a slice of DOT [`Attribute`]s.
fn find_attribute<'a>(attributes: &'a [Attribute], key: &str) -> Option<&'a str> {
    attributes.iter().find_map(|Attribute(k, v)| {
        if dot_id_to_str(k) == key {
            Some(dot_id_to_str(v))
        } else {
            None
        }
    })
}

/// Executes Graphviz `dot` and returns the annotated graph.
///
/// Passes `-Tdot` to get DOT output with layout attributes injected,
/// and `-y` to invert the Y-axis to match screen coordinates.
///
/// # Errors
///
/// Returns [`RenderError::Layout`] if:
/// - The `dot` binary is not found on `PATH`.
/// - The `dot` process exits with a non-zero status (includes the DOT input in the message).
/// - The output is not valid UTF-8.
/// - The output cannot be re-parsed into a [`DotGraph`].
fn run_graphviz(gv_graph: DotGraph) -> Result<DotGraph, RenderError> {
    let mut ctx = PrinterContext::default();
    let dot_input = graphviz_rust::print(gv_graph, &mut ctx);
    debug!(dot_input:%; "Graphviz DOT input");

    let output = graphviz_rust::exec_dot(
        dot_input.clone(),
        vec![
            CommandArg::Format(Format::Dot),
            CommandArg::Custom("-y".into()),
        ],
    )
    .map_err(|err| match err.kind() {
        ErrorKind::NotFound => RenderError::Layout(
            "`dot` command not found, is Graphviz installed? \
             see https://graphviz.org/download/"
                .into(),
        ),
        _ => RenderError::Layout(format!(
            "`dot` command failed: {err}\n\nDOT input:\n{dot_input}"
        )),
    })?;

    let dot_output = String::from_utf8(output)
        .map_err(|err| RenderError::Layout(format!("invalid UTF-8 in `dot` output: {err}")))?;

    debug!(dot_output:%; "Graphviz DOT output");

    graphviz_rust::parse(&dot_output)
        .map_err(|err| RenderError::Layout(format!("cannot parse `dot` output: {err}")))
}

/// Extracts node centre-point positions from a Graphviz-annotated DOT graph.
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
                "node `{node_name}` missing `pos` attribute in graphviz output"
            ))
        })?;

        let pos = parse_pos_str(pos_str).ok_or_else(|| {
            RenderError::Layout(format!(
                "cannot parse `pos` value `{pos_str}` for `{node_name}`"
            ))
        })?;

        positions.insert(Id::new(node_name), pos);

        trace!(node_name, pos:?; "Extracted Graphviz position");
    }

    Ok(positions)
}

/// Extracts edge B-spline paths from a Graphviz-annotated DOT graph.
///
/// Returns a map from the sequential edge index to the corresponding [`ArrowPath`].
fn extract_edge_paths_from_graph(
    graph: &DotGraph,
) -> Result<HashMap<usize, ArrowPath>, RenderError> {
    let stmts = match graph {
        DotGraph::Graph { stmts, .. } | DotGraph::DiGraph { stmts, .. } => stmts,
    };

    let mut edge_paths = HashMap::new();

    for edge in stmts.iter().filter_map(|stmt| match stmt {
        Stmt::Edge(edge) => Some(edge),
        _ => None,
    }) {
        let id_str = find_attribute(&edge.attributes, "id").ok_or_else(|| {
            RenderError::Layout("edge missing `id` attribute in graphviz output".into())
        })?;

        let index: usize = id_str
            .strip_prefix("e_")
            .and_then(|n| n.parse().ok())
            .ok_or_else(|| {
                RenderError::Layout(format!("cannot parse edge id `{id_str}` as index"))
            })?;

        let pos_str = find_attribute(&edge.attributes, "pos").ok_or_else(|| {
            RenderError::Layout(format!(
                "edge `{id_str}` missing `pos` attribute in graphviz output"
            ))
        })?;

        let path = parse_edge_pos(pos_str)?;

        edge_paths.insert(index, path);
    }

    Ok(edge_paths)
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

/// Parses a Graphviz edge `pos` attribute into an [`ArrowPath`].
///
/// The [Graphviz `splineType`](https://graphviz.org/docs/attr-types/splineType/)
/// format is:
///
/// ```text
/// (endp)? (startp)? point (triple)+
/// ```
///
/// Where `endp` = `e,x,y`, `startp` = `s,x,y`, and `triple` = `point point point`.
/// This means spline points are always `1 + 3k` (at least 4). A straight line is
/// simply 4 collinear points forming a degenerate cubic Bézier.
///
/// `e,` marks the arrowhead endpoint, `s,` marks the arrow tail, and the
/// remaining tokens are cubic B-spline control points. Backslash-newline
/// continuations that Graphviz inserts for long values are collapsed before
/// parsing.
fn parse_edge_pos(pos_str: &str) -> Result<ArrowPath, RenderError> {
    // Graphviz uses backslash-newline for line continuation in long pos values.
    let mut cleaned = String::with_capacity(pos_str.len());
    let mut chars = pos_str.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            while chars.peek().is_some_and(|c| c.is_whitespace()) {
                chars.next();
            }
        } else {
            cleaned.push(ch);
        }
    }
    let tokens: Vec<&str> = cleaned.split_whitespace().collect();

    let mut endpoint: Option<Point> = None;
    let mut startpoint: Option<Point> = None;
    let mut spline_points: Vec<Point> = Vec::with_capacity(4);

    for token in &tokens {
        if let Some(rest) = token.strip_prefix("e,") {
            endpoint = Some(parse_pos_str(rest).ok_or_else(|| {
                RenderError::Layout(format!("cannot parse edge endpoint `e,{rest}`"))
            })?);
        } else if let Some(rest) = token.strip_prefix("s,") {
            startpoint = Some(parse_pos_str(rest).ok_or_else(|| {
                RenderError::Layout(format!("cannot parse edge startpoint `s,{rest}`"))
            })?);
        } else {
            let point = parse_pos_str(token).ok_or_else(|| {
                RenderError::Layout(format!("cannot parse edge spline point `{token}`"))
            })?;
            spline_points.push(point);
        }
    }

    if spline_points.len() < 2 {
        return Err(RenderError::Layout(
            "edge `pos` contains fewer than 2 B-spline points".into(),
        ));
    }

    let source = startpoint.unwrap_or(spline_points[0]);
    let destination = endpoint.unwrap_or(*spline_points.last().unwrap());

    let control_points = spline_points[1..spline_points.len() - 1].to_vec();

    Ok(ArrowPath::new(source, destination, control_points))
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
    fn test_parse_edge_pos_single_segment_with_endpoint() {
        let pos = "e,24.177,107.86 24.132,54.475 22.42,67.218 22.055,82.718 23.036,96.637";
        let path = parse_edge_pos(pos).unwrap();

        assert!((path.source().x() - 24.132).abs() < 0.001);
        assert!((path.source().y() - 54.475).abs() < 0.001);
        assert!((path.destination().x() - 24.177).abs() < 0.001);
        assert!((path.destination().y() - 107.86).abs() < 0.001);
        assert_eq!(path.control_points().len(), 2);
    }

    #[test]
    fn test_parse_edge_pos_with_startpoint() {
        let pos = "s,47.823,107.86 48.964,96.637 49.945,82.718 49.58,67.218 47.868,54.475";
        let path = parse_edge_pos(pos).unwrap();

        assert!((path.source().x() - 47.823).abs() < 0.001);
        assert!((path.destination().x() - 47.868).abs() < 0.001);
        assert_eq!(path.control_points().len(), 2);
    }

    #[test]
    fn test_parse_edge_pos_multi_segment_self_loop() {
        let pos = "e,72.305,46.538 72.305,7.4619 91.078,4.4561 108,10.969 108,27 108,39.65 97.464,46.373 83.797,47.17";
        let path = parse_edge_pos(pos).unwrap();

        assert_eq!(path.control_points().len(), 5);
    }

    #[test]
    fn test_parse_edge_pos_no_prefix() {
        let pos = "10,20 30,40 50,60 70,80";
        let path = parse_edge_pos(pos).unwrap();

        assert_eq!(path.source(), Point::new(10.0, 20.0));
        assert_eq!(path.destination(), Point::new(70.0, 80.0));
        assert_eq!(path.control_points().len(), 2);
    }

    #[test]
    fn test_parse_edge_pos_single_point() {
        let pos = "50,100";
        assert!(parse_edge_pos(pos).is_err());
    }

    #[test]
    fn test_parse_edge_pos_both_start_and_end() {
        let pos = "s,10,20 e,90,80 15,25 30,40 60,60 85,75";
        let path = parse_edge_pos(pos).unwrap();

        // Source = s point, destination = e point
        assert_eq!(path.source(), Point::new(10.0, 20.0));
        assert_eq!(path.destination(), Point::new(90.0, 80.0));
        // 4 spline points → 2 intermediate control points
        assert_eq!(path.control_points().len(), 2);
        assert_eq!(path.control_points()[0], Point::new(30.0, 40.0));
        assert_eq!(path.control_points()[1], Point::new(60.0, 60.0));
    }

    #[test]
    fn test_parse_edge_pos_backslash_continuation() {
        // Simulates Graphviz line continuation: backslash + newline splits a coordinate
        let pos = "e,100,200 10,20 30,40 50,\\\n60 80,90";
        let path = parse_edge_pos(pos).unwrap();

        assert_eq!(path.source(), Point::new(10.0, 20.0));
        assert_eq!(path.destination(), Point::new(100.0, 200.0));
        // "50,\\\n60" should rejoin to "50,60"
        assert_eq!(path.control_points().len(), 2);
        assert_eq!(path.control_points()[0], Point::new(30.0, 40.0));
        assert_eq!(path.control_points()[1], Point::new(50.0, 60.0));
    }

    #[test]
    fn test_parse_edge_pos_empty() {
        assert!(parse_edge_pos("").is_err());
    }

    #[test]
    fn test_extract_edge_paths_from_graph() {
        let graph = DotGraph::DiGraph {
            id: DotId::Plain("scope".into()),
            strict: false,
            stmts: vec![
                Stmt::Edge(Edge {
                    ty: EdgeTy::Pair(
                        Vertex::N(NodeId(DotId::Plain("a".into()), None)),
                        Vertex::N(NodeId(DotId::Plain("b".into()), None)),
                    ),
                    attributes: vec![
                        dot_attr("id", "e_0"),
                        dot_attr("pos", "e,36,107.86 36,54.475 36,67.218 36,82.718 36,96.637"),
                    ],
                }),
                Stmt::Edge(Edge {
                    ty: EdgeTy::Pair(
                        Vertex::N(NodeId(DotId::Plain("b".into()), None)),
                        Vertex::N(NodeId(DotId::Plain("a".into()), None)),
                    ),
                    attributes: vec![
                        dot_attr("id", "e_1"),
                        dot_attr(
                            "pos",
                            "e,59.737,54.475 59.645,107.86 63.116,95.168 63.894,79.679 61.979,65.732",
                        ),
                    ],
                }),
            ],
        };

        let paths = extract_edge_paths_from_graph(&graph).unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[&0].control_points().len(), 2);
        assert_eq!(paths[&1].control_points().len(), 2);
    }
}
