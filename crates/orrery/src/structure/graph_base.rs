//! Low-level graph data structures and primitives.
//!
//! This module provides the foundational graph implementation used by both
//! component and sequence diagrams. It offers a lightweight, custom graph
//! structure that is optimized for Orrery's specific needs without requiring
//! external dependencies.
//!
//! # Architecture
//!
//! The module provides:
//! - [`EdgeIndex`]: Type-safe edge indices with lifetime tracking
//! - [`Edge`]: Edge structure storing source, target, and associated data
//! - [`GraphInternal`]: Core graph implementation with nodes and edges
//!
//! Capabilities:
//! - Node and edge storage via `HashMap` and `Vec`
//! - Tracking of both incoming and outgoing edges per node
//! - Root detection (nodes with no incoming edges)
//! - Type-safe node and edge access with lifetime guarantees
//!
//! This is an internal module; its types are not exposed publicly but are used
//! by the higher-level `ComponentGraph` and `SequenceGraph` structures.

use std::{collections::HashMap, marker::PhantomData};

use orrery_core::identifier::Id;

// =============================================================================
// Low-level primitive types and internal data structures
// =============================================================================

/// Type-safe index for edges in the graph.
///
/// Uses phantom data to track lifetime relationships, ensuring that edge indices
/// cannot outlive the graph they belong to. The lifetime parameter prevents
/// use-after-free bugs when graphs are modified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct EdgeIndex<'idx>(usize, PhantomData<&'idx ()>);

impl<'idx> EdgeIndex<'idx> {
    /// Creates a new edge index with the given numeric index.
    fn new(index: usize) -> Self {
        EdgeIndex(index, PhantomData)
    }
}

/// A directed edge in the graph.
///
/// Stores the source and target node IDs along with an associated value
/// of generic type `E`.
#[derive(Debug)]
struct Edge<E>
where
    E: Copy + std::fmt::Debug,
{
    #[allow(dead_code)]
    source: Id,
    target: Id,
    value: E,
}

impl<E> Edge<E>
where
    E: Copy + std::fmt::Debug,
{
    /// Creates a new edge with the given source, target, and value.
    fn new(source: Id, target: Id, value: E) -> Self {
        Edge {
            source,
            target,
            value,
        }
    }
}

// =============================================================================
// Core internal graph structure
// =============================================================================

/// Core graph data structure.
///
/// This generic graph implementation provides:
/// - Node storage by ID with generic node data type `N`
/// - Edge storage with generic edge data type `E`
/// - Tracking of incoming and outgoing edges for each node
/// - Efficient lookups and traversals
///
/// The graph is directed and allows self-loops and multiple edges between nodes.
///
/// Type parameters:
/// - `'idx`: Lifetime for edge indices
/// - `N`: Node data type (must be Copy and Debug)
/// - `E`: Edge data type (must be Copy and Debug)
#[derive(Debug)]
pub(super) struct GraphInternal<'idx, N, E>
where
    N: Copy + std::fmt::Debug,
    E: Copy + std::fmt::Debug,
{
    nodes: HashMap<Id, N>,
    edges: Vec<Edge<E>>,
    income_edges: HashMap<Id, Vec<EdgeIndex<'idx>>>,
    outgoing_edges: HashMap<Id, Vec<EdgeIndex<'idx>>>,
}

impl<'idx, N, E> GraphInternal<'idx, N, E>
where
    N: Copy + std::fmt::Debug,
    E: Copy + std::fmt::Debug,
{
    /// Creates a new empty graph.
    pub(super) fn new() -> Self {
        GraphInternal {
            nodes: HashMap::new(),
            edges: Vec::new(),
            income_edges: HashMap::new(),
            outgoing_edges: HashMap::new(),
        }
    }

    /// Returns the node data for the given ID, if it exists.
    pub(super) fn node(&self, id: Id) -> Option<N> {
        self.nodes.get(&id).copied()
    }

    /// Returns the node data for the given ID without checking existence.
    ///
    /// # Panics
    /// Panics if the node ID does not exist in the graph.
    pub(super) fn node_unchecked(&self, id: Id) -> N {
        self.nodes[&id]
    }

    /// Returns an iterator over all node data in the graph.
    pub(super) fn nodes(&self) -> impl Iterator<Item = N> {
        self.nodes.values().copied()
    }

    /// Returns the total number of nodes in the graph.
    pub(super) fn nodes_count(&self) -> usize {
        self.nodes.len()
    }

    /// Checks if a node with the given ID exists in the graph.
    pub(super) fn contains_node(&self, id: Id) -> bool {
        self.nodes.contains_key(&id)
    }

    /// Returns the edge data for the given index without checking existence.
    ///
    /// # Panics
    /// Panics if the edge index does not exist in the graph.
    pub(super) fn edge_unchecked(&self, idx: EdgeIndex) -> E {
        self.edges[idx.0].value
    }

    /// Returns an iterator over all edge data in the graph.
    pub(super) fn edges(&self) -> impl Iterator<Item = E> {
        self.edges.iter().map(|edge| edge.value)
    }

    /// Returns an iterator over root nodes (nodes with no incoming edges).
    pub(super) fn roots(&self) -> impl Iterator<Item = N> {
        self.nodes.iter().filter_map(|(node_id, node)| {
            if !self.income_edges.contains_key(node_id) {
                Some(*node)
            } else {
                None
            }
        })
    }

    /// Returns an iterator over nodes that are targets of outgoing edges from the given source.
    ///
    /// This provides all nodes directly connected from the source node via outgoing edges.
    /// Returns an empty iterator if the source node has no outgoing edges.
    pub(super) fn outgoing_nodes(&self, source_id: Id) -> impl Iterator<Item = N> {
        self.outgoing_edges
            .get(&source_id)
            .into_iter()
            .flatten() // TODO: This should return an Error.
            .map(|idx| {
                let outgoing_node_id = self.edges[idx.0].target;
                self.node_unchecked(outgoing_node_id)
            })
    }

    /// Adds a node to the graph with the given ID and data.
    ///
    /// If a node with the same ID already exists, it will be replaced.
    pub(super) fn add_node(&mut self, id: Id, node: N) {
        self.nodes.insert(id, node);
    }

    /// Adds a directed edge to the graph between two nodes.
    ///
    /// Updates both the edge storage and the incoming/outgoing edge indices for
    /// efficient traversal. Both source and target nodes must exist in the graph.
    ///
    /// # Returns
    /// The index of the newly added edge.
    ///
    /// # Panics
    /// Panics in debug mode if either the source or target node does not exist in the graph.
    /// This panic is for internal developer testing and bug detection. In a release build,
    /// this check is optimized away.
    pub(super) fn add_edge(&mut self, source_id: Id, target_id: Id, edge: E) -> EdgeIndex<'idx> {
        #[cfg(debug_assertions)]
        {
            assert!(
                self.nodes.contains_key(&source_id),
                "Adding edge: Source node {source_id} does not exist for {edge:?}",
            );
            assert!(
                self.nodes.contains_key(&target_id),
                "Adding edge: Target node {target_id} does not exist for {edge:?}",
            );
        }

        self.edges.push(Edge::new(source_id, target_id, edge));

        let idx = EdgeIndex::new(self.edges.len() - 1);
        self.outgoing_edges.entry(source_id).or_default().push(idx);
        self.income_edges.entry(target_id).or_default().push(idx);
        idx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test node data structure with a simple numeric value
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct TestNode {
        value: u32,
    }

    /// Test edge data structure with a weight attribute
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct TestEdge {
        weight: i32,
    }

    #[test]
    fn test_edge_index_creation() {
        // Test that EdgeIndex properly wraps indices and implements equality
        let idx1 = EdgeIndex::new(5);
        let idx2 = EdgeIndex::new(5);
        let idx3 = EdgeIndex::new(10);

        assert_eq!(idx1, idx2);
        assert_ne!(idx1, idx3);
        assert_eq!(idx1.0, 5);
        assert_eq!(idx3.0, 10);
    }

    #[test]
    fn test_graph_new() {
        let graph: GraphInternal<TestNode, TestEdge> = GraphInternal::new();

        assert_eq!(graph.nodes_count(), 0);
        assert_eq!(graph.nodes().count(), 0);
        assert_eq!(graph.edges().count(), 0);
        assert_eq!(graph.roots().count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut graph: GraphInternal<TestNode, TestEdge> = GraphInternal::new();
        let id1 = Id::new("node1");
        let id2 = Id::new("node2");
        let node1 = TestNode { value: 10 };
        let node2 = TestNode { value: 20 };

        graph.add_node(id1, node1);
        graph.add_node(id2, node2);

        assert_eq!(graph.nodes_count(), 2);
        assert!(graph.contains_node(id1));
        assert!(graph.contains_node(id2));
        assert_eq!(graph.node(id1), Some(node1));
        assert_eq!(graph.node(id2), Some(node2));
    }

    #[test]
    fn test_node_unchecked() {
        let mut graph: GraphInternal<TestNode, TestEdge> = GraphInternal::new();
        let id = Id::new("node");
        let node = TestNode { value: 30 };

        graph.add_node(id, node);

        assert_eq!(graph.node_unchecked(id), node);
    }

    #[test]
    fn test_node_returns_none_for_missing() {
        let graph: GraphInternal<TestNode, TestEdge> = GraphInternal::new();
        let id = Id::new("missing");

        assert_eq!(graph.node(id), None);
    }

    #[test]
    fn test_nodes_iterator() {
        let mut graph: GraphInternal<TestNode, TestEdge> = GraphInternal::new();
        let id1 = Id::new("node1");
        let id2 = Id::new("node2");
        let id3 = Id::new("node3");
        let node1 = TestNode { value: 10 };
        let node2 = TestNode { value: 20 };
        let node3 = TestNode { value: 30 };

        graph.add_node(id1, node1);
        graph.add_node(id2, node2);
        graph.add_node(id3, node3);

        let nodes: Vec<TestNode> = graph.nodes().collect();
        assert_eq!(nodes.len(), 3);
        assert!(nodes.contains(&node1));
        assert!(nodes.contains(&node2));
        assert!(nodes.contains(&node3));
    }

    #[test]
    fn test_add_edge() {
        let mut graph: GraphInternal<TestNode, TestEdge> = GraphInternal::new();
        let id1 = Id::new("source");
        let id2 = Id::new("target");
        let node1 = TestNode { value: 10 };
        let node2 = TestNode { value: 20 };
        let edge = TestEdge { weight: 5 };

        graph.add_node(id1, node1);
        graph.add_node(id2, node2);
        let edge_idx = graph.add_edge(id1, id2, edge);

        assert_eq!(graph.edge_unchecked(edge_idx), edge);
        assert_eq!(graph.edges().count(), 1);
    }

    #[test]
    fn test_edges_iterator() {
        let mut graph: GraphInternal<TestNode, TestEdge> = GraphInternal::new();
        let id1 = Id::new("node1");
        let id2 = Id::new("node2");
        let id3 = Id::new("node3");
        let node1 = TestNode { value: 10 };
        let node2 = TestNode { value: 20 };
        let node3 = TestNode { value: 30 };
        let edge1 = TestEdge { weight: 1 };
        let edge2 = TestEdge { weight: 2 };
        let edge3 = TestEdge { weight: 3 };

        graph.add_node(id1, node1);
        graph.add_node(id2, node2);
        graph.add_node(id3, node3);
        graph.add_edge(id1, id2, edge1);
        graph.add_edge(id2, id3, edge2);
        graph.add_edge(id3, id1, edge3);

        let edges: Vec<TestEdge> = graph.edges().collect();
        assert_eq!(edges.len(), 3);
        assert!(edges.contains(&edge1));
        assert!(edges.contains(&edge2));
        assert!(edges.contains(&edge3));
    }

    #[test]
    fn test_roots() {
        let mut graph: GraphInternal<TestNode, TestEdge> = GraphInternal::new();
        let id1 = Id::new("root1");
        let id2 = Id::new("root2");
        let id3 = Id::new("child");
        let node1 = TestNode { value: 10 };
        let node2 = TestNode { value: 20 };
        let node3 = TestNode { value: 30 };
        let edge = TestEdge { weight: 1 };

        graph.add_node(id1, node1);
        graph.add_node(id2, node2);
        graph.add_node(id3, node3);
        graph.add_edge(id1, id3, edge);

        let roots: Vec<TestNode> = graph.roots().collect();
        assert_eq!(roots.len(), 2);
        assert!(roots.contains(&node1));
        assert!(roots.contains(&node2));
        assert!(!roots.contains(&node3)); // node3 has incoming edge
    }

    #[test]
    fn test_outgoing_nodes() {
        let mut graph: GraphInternal<TestNode, TestEdge> = GraphInternal::new();
        let id1 = Id::new("source");
        let id2 = Id::new("target1");
        let id3 = Id::new("target2");
        let id4 = Id::new("isolated");
        let node1 = TestNode { value: 10 };
        let node2 = TestNode { value: 20 };
        let node3 = TestNode { value: 30 };
        let node4 = TestNode { value: 40 };
        let edge1 = TestEdge { weight: 1 };
        let edge2 = TestEdge { weight: 2 };

        graph.add_node(id1, node1);
        graph.add_node(id2, node2);
        graph.add_node(id3, node3);
        graph.add_node(id4, node4);
        graph.add_edge(id1, id2, edge1);
        graph.add_edge(id1, id3, edge2);

        let outgoing: Vec<TestNode> = graph.outgoing_nodes(id1).collect();
        assert_eq!(outgoing.len(), 2);
        assert!(outgoing.contains(&node2));
        assert!(outgoing.contains(&node3));

        let outgoing_empty: Vec<TestNode> = graph.outgoing_nodes(id4).collect();
        assert_eq!(outgoing_empty.len(), 0);
    }

    #[test]
    fn test_self_loop() {
        let mut graph: GraphInternal<TestNode, TestEdge> = GraphInternal::new();
        let id = Id::new("self_loop");
        let node = TestNode { value: 10 };
        let edge = TestEdge { weight: 1 };

        graph.add_node(id, node);
        let edge_idx = graph.add_edge(id, id, edge);

        assert_eq!(graph.edge_unchecked(edge_idx), edge);
        assert_eq!(graph.edges().count(), 1);

        // Node with self-loop is not a root (has incoming edge from itself)
        assert_eq!(graph.roots().count(), 0);

        // Node with self-loop appears in its own outgoing nodes
        let outgoing: Vec<TestNode> = graph.outgoing_nodes(id).collect();
        assert_eq!(outgoing.len(), 1);
        assert!(outgoing.contains(&node));
    }

    #[test]
    fn test_multiple_edges_between_same_nodes() {
        let mut graph: GraphInternal<TestNode, TestEdge> = GraphInternal::new();
        let id1 = Id::new("source");
        let id2 = Id::new("target");
        let node1 = TestNode { value: 10 };
        let node2 = TestNode { value: 20 };
        let edge1 = TestEdge { weight: 1 };
        let edge2 = TestEdge { weight: 2 };
        let edge3 = TestEdge { weight: 3 };

        graph.add_node(id1, node1);
        graph.add_node(id2, node2);

        // Add multiple edges between the same nodes
        let idx1 = graph.add_edge(id1, id2, edge1);
        let idx2 = graph.add_edge(id1, id2, edge2);
        let idx3 = graph.add_edge(id2, id1, edge3); // Reverse direction

        assert_eq!(graph.edges().count(), 3);
        assert_eq!(graph.edge_unchecked(idx1), edge1);
        assert_eq!(graph.edge_unchecked(idx2), edge2);
        assert_eq!(graph.edge_unchecked(idx3), edge3);

        // Check outgoing nodes for id1 (should include duplicates)
        let outgoing1: Vec<TestNode> = graph.outgoing_nodes(id1).collect();
        assert_eq!(outgoing1.len(), 2); // Two edges to id2

        // Check outgoing nodes for id2
        let outgoing2: Vec<TestNode> = graph.outgoing_nodes(id2).collect();
        assert_eq!(outgoing2.len(), 1); // One edge to id1
    }

    #[test]
    fn test_complex_graph_structure() {
        let mut graph: GraphInternal<TestNode, TestEdge> = GraphInternal::new();

        // Create a diamond-shaped graph:
        //     top
        //    /   \
        //  left  right
        //    \   /
        //    bottom
        let top = Id::new("top");
        let left = Id::new("left");
        let right = Id::new("right");
        let bottom = Id::new("bottom");

        let node_top = TestNode { value: 1 };
        let node_left = TestNode { value: 2 };
        let node_right = TestNode { value: 3 };
        let node_bottom = TestNode { value: 4 };

        graph.add_node(top, node_top);
        graph.add_node(left, node_left);
        graph.add_node(right, node_right);
        graph.add_node(bottom, node_bottom);

        graph.add_edge(top, left, TestEdge { weight: 1 });
        graph.add_edge(top, right, TestEdge { weight: 2 });
        graph.add_edge(left, bottom, TestEdge { weight: 3 });
        graph.add_edge(right, bottom, TestEdge { weight: 4 });

        // Check that only 'top' is a root
        let roots: Vec<TestNode> = graph.roots().collect();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], node_top);

        // Check outgoing nodes
        let top_outgoing: Vec<TestNode> = graph.outgoing_nodes(top).collect();
        assert_eq!(top_outgoing.len(), 2);
        assert!(top_outgoing.contains(&node_left));
        assert!(top_outgoing.contains(&node_right));

        let bottom_outgoing: Vec<TestNode> = graph.outgoing_nodes(bottom).collect();
        assert_eq!(bottom_outgoing.len(), 0); // No outgoing edges

        // Verify total counts
        assert_eq!(graph.nodes_count(), 4);
        assert_eq!(graph.edges().count(), 4);
    }

    #[test]
    fn test_node_replacement() {
        let mut graph: GraphInternal<TestNode, TestEdge> = GraphInternal::new();
        let id = Id::new("node");
        let node1 = TestNode { value: 10 };
        let node2 = TestNode { value: 20 };

        graph.add_node(id, node1);
        assert_eq!(graph.node(id), Some(node1));

        // Replace the node with same ID
        graph.add_node(id, node2);
        assert_eq!(graph.node(id), Some(node2));
        assert_eq!(graph.nodes_count(), 1); // Still only one node
    }

    #[test]
    fn test_empty_graph_outgoing_nodes() {
        let graph: GraphInternal<TestNode, TestEdge> = GraphInternal::new();
        let id = Id::new("nonexistent");

        // Outgoing nodes for non-existent node should return empty iterator
        let outgoing: Vec<TestNode> = graph.outgoing_nodes(id).collect();
        assert_eq!(outgoing.len(), 0);
    }

    #[test]
    fn test_disconnected_nodes() {
        // Test graph with multiple disconnected components
        let mut graph: GraphInternal<TestNode, TestEdge> = GraphInternal::new();

        // Component 1: node1 -> node2
        let id1 = Id::new("node1");
        let id2 = Id::new("node2");
        let node1 = TestNode { value: 1 };
        let node2 = TestNode { value: 2 };

        // Component 2: node3 -> node4
        let id3 = Id::new("node3");
        let id4 = Id::new("node4");
        let node3 = TestNode { value: 3 };
        let node4 = TestNode { value: 4 };

        // Isolated node
        let id5 = Id::new("isolated");
        let node5 = TestNode { value: 5 };

        graph.add_node(id1, node1);
        graph.add_node(id2, node2);
        graph.add_node(id3, node3);
        graph.add_node(id4, node4);
        graph.add_node(id5, node5);

        graph.add_edge(id1, id2, TestEdge { weight: 1 });
        graph.add_edge(id3, id4, TestEdge { weight: 2 });

        // Should have 3 roots: node1, node3, and node5 (isolated)
        let roots: Vec<TestNode> = graph.roots().collect();
        assert_eq!(roots.len(), 3);
        assert!(roots.contains(&node1));
        assert!(roots.contains(&node3));
        assert!(roots.contains(&node5));

        // Verify graph structure
        assert_eq!(graph.nodes_count(), 5);
        assert_eq!(graph.edges().count(), 2);
    }
}
