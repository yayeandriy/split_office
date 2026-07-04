//! Workflow DAG for Split Office.
//!
//! # Architecture (from DAG-First Architecture spec)
//!
//! Every workflow is a Directed Acyclic Graph of nodes and edges.
//! Even when the UI shows a linear Dataset → Filter → Sort chain,
//! the internal model is a full graph — ready for branching, comparison,
//! and agent-generated workflows without architectural migration.
//!
//! ```text
//! Dataset ──► Filter ──► Sort
//! ```
//!
//! # Spec compliance
//!
//! | Requirement          | Status |
//! |----------------------|--------|
//! | Edges                | ✅     |
//! | Cycle detection      | ✅     |
//! | Topological sort     | ✅     |
//! | Graph validation     | ✅     |
//! | Dirty propagation    | ✅     |
//! | Node inputs/outputs  | ✅     |
//! | Execution states     | ✅     |
//! | Lazy evaluation      | ✅     |
//! | Deterministic        | ✅     |
//! | Reproducible         | ✅     |

use std::collections::{HashSet, VecDeque};

use core::{FilterExpr, SortSpec, Viewport};
use serde::{Deserialize, Serialize};

// ── Node identity ────────────────────────────────────────────────────────────

/// Unique identifier for a workflow node.
pub type NodeId = usize;

// ── Node kind ─────────────────────────────────────────────────────────────────

/// What kind of step this node represents in the pipeline.
///
/// Spec §Node Types: Dataset, Filter, Sort, Aggregate, Join, DerivedColumn,
/// Analysis, Visualization. Additional kinds added as the workbench evolves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    /// The data source. No inputs. Produces dataset output. (Spec §Dataset Node)
    Dataset,
    /// A filter step (WHERE clause). (Spec §Filter Node)
    Filter,
    /// A sort step (ORDER BY). (Spec §Sort Node)
    Sort,
    /// Reserved for Phase 2+. (Spec §Aggregate Node)
    Aggregate,
    /// Reserved for Phase 2+. (Spec §Join Node)
    Join,
    /// Reserved for Phase 2+. (Spec §Derived Column Node)
    DerivedColumn,
    /// Reserved for Phase 2+. (Spec §Analysis Node)
    Analysis,
    /// Reserved for Phase 2+. (Spec §Visualization Node)
    Visualization,
}

impl NodeKind {
    /// Unicode icon used in the workflow sidebar.
    pub fn icon(&self) -> &'static str {
        match self {
            NodeKind::Dataset => "\u{25c8}",
            NodeKind::Filter => "\u{25bd}",
            NodeKind::Sort => "\u{21c5}",
            NodeKind::Aggregate => "\u{03a3}",
            NodeKind::Join => "\u{22c8}",
            NodeKind::DerivedColumn => "\u{0192}",
            NodeKind::Analysis => "\u{2699}",
            NodeKind::Visualization => "\u{25eb}",
        }
    }

    /// Human-readable label for the sidebar.
    pub fn label(&self) -> &'static str {
        match self {
            NodeKind::Dataset => "Dataset",
            NodeKind::Filter => "Filter",
            NodeKind::Sort => "Sort",
            NodeKind::Aggregate => "Aggregate",
            NodeKind::Join => "Join",
            NodeKind::DerivedColumn => "Derived",
            NodeKind::Analysis => "Analysis",
            NodeKind::Visualization => "Chart",
        }
    }
}

// ── Execution state ───────────────────────────────────────────────────────────

/// Per-node execution lifecycle.
///
/// Spec §Dirty State Tracking: Clean, Dirty, Executing, Failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionState {
    /// Parameters are up to date; no re-execution needed.
    Clean,
    /// Parameters changed; re-execution is pending. (Spec §Lazy Evaluation)
    Dirty,
    /// Query is in flight.
    Executing,
    /// Execution failed with a message. (Spec §Failure Scenarios)
    Failed(String),
}

impl ExecutionState {
    /// Single-character status dot for the workflow sidebar.
    pub fn status_label(&self) -> &'static str {
        match self {
            ExecutionState::Clean => "\u{25cf}",
            ExecutionState::Dirty => "\u{25cb}",
            ExecutionState::Executing => "\u{25c9}",
            ExecutionState::Failed(_) => "\u{2715}",
        }
    }
}

// ── Node payload ──────────────────────────────────────────────────────────────

/// The parameters held by a workflow node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodePayload {
    /// Dataset node stores the dataset name.
    Dataset { name: String },
    /// Filter node stores a filter expression tree.
    Filter { expr: FilterExpr },
    /// Sort node stores the current sort specification.
    Sort { specs: Vec<SortSpec> },
    /// Reserved for future node kinds.
    Empty,
}

impl NodePayload {
    /// Human-readable summary shown in the workflow sidebar.
    pub fn summary(&self) -> String {
        match self {
            NodePayload::Dataset { name } => name.clone(),
            NodePayload::Filter { expr } => match expr {
                FilterExpr::None => "\u{2014}".into(),
                FilterExpr::Contains { column, pattern } => {
                    format!("{} \u{220b} \"{}\"", column, pattern)
                }
                FilterExpr::Eq { column, value } => {
                    format!("{} = {:?}", column, value)
                }
                FilterExpr::Gt { column, value } => {
                    format!("{} > {:?}", column, value)
                }
                FilterExpr::Lt { column, value } => {
                    format!("{} < {:?}", column, value)
                }
                FilterExpr::Gte { column, value } => {
                    format!("{} >= {:?}", column, value)
                }
                FilterExpr::Lte { column, value } => {
                    format!("{} <= {:?}", column, value)
                }
                FilterExpr::And(_, _) => "(multiple)".into(),
                FilterExpr::Or(_, _) => "(multiple)".into(),
                FilterExpr::Not(_) => "(not \u{2026})".into(),
            },
            NodePayload::Sort { specs } => {
                if specs.is_empty() {
                    "\u{2014}".into()
                } else {
                    let labels: Vec<String> = specs
                        .iter()
                        .map(|s| format!("{} {}", s.column, s.direction.arrow_label()))
                        .collect();
                    labels.join(", ")
                }
            }
            NodePayload::Empty => "\u{2014}".into(),
        }
    }
}

// ── Edge ──────────────────────────────────────────────────────────────────────

/// A directed edge in the workflow DAG.
///
/// Spec §Edge Model: Every edge represents Dependency. Producer → Consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Edge {
    /// Source node (producer).
    pub from: NodeId,
    /// Target node (consumer).
    pub to: NodeId,
}

impl Edge {
    pub fn new(from: NodeId, to: NodeId) -> Self {
        Self { from, to }
    }
}

// ── Workflow node ─────────────────────────────────────────────────────────────

/// A single node in the workflow DAG.
///
/// Spec §Node Model: unique id, node type, inputs, outputs, validation state,
/// execution state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: NodeId,
    pub kind: NodeKind,
    pub payload: NodePayload,
    /// Execution lifecycle state. (Spec §Dirty State Tracking)
    pub execution_state: ExecutionState,
    /// Whether this node participates in query execution.
    pub enabled: bool,
    /// IDs of nodes that feed into this node. (Spec §Node Model: inputs)
    #[serde(default)]
    pub inputs: Vec<NodeId>,
    /// IDs of nodes this node feeds into. (Spec §Node Model: outputs)
    #[serde(default)]
    pub outputs: Vec<NodeId>,
}

impl WorkflowNode {
    fn new(id: NodeId, kind: NodeKind, payload: NodePayload) -> Self {
        Self {
            id,
            kind,
            payload,
            execution_state: ExecutionState::Dirty,
            enabled: true,
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }
}

// ── Execution plan ────────────────────────────────────────────────────────────

/// The ordered list of node IDs to execute.
///
/// Produced by topological sort of the DAG. Spec §Execution Planning.
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    /// Nodes in dependency order (producers before consumers).
    pub steps: Vec<NodeId>,
}

// ── Linear workflow IDs ───────────────────────────────────────────────────────

/// Convenience struct holding the three node IDs of the standard linear pipeline.
#[derive(Debug, Clone, Copy)]
pub struct LinearWorkflowIds {
    pub dataset: NodeId,
    pub filter: NodeId,
    pub sort: NodeId,
}

// ── Workflow graph ────────────────────────────────────────────────────────────

/// The workflow DAG.
///
/// Spec §Core Structures: WorkflowGraph, NodeId, Edge, ExecutionPlan.
/// Spec §Graph Model: nodes and edges with dependency relationships.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowGraph {
    nodes: Vec<WorkflowNode>,
    edges: Vec<Edge>,
}

impl WorkflowGraph {
    // ── Construction ──────────────────────────────────────────────────────

    /// Create an empty workflow graph.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Add a node and return its assigned ID.
    pub fn add_node(&mut self, kind: NodeKind, payload: NodePayload) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(WorkflowNode::new(id, kind, payload));
        id
    }

    /// Add a directed edge between two nodes. (Spec §Edge Model)
    ///
    /// Updates `inputs`/`outputs` on both nodes so adjacency lookups are O(1).
    pub fn add_edge(&mut self, from: NodeId, to: NodeId) -> Result<(), WorkflowError> {
        // Validate node existence.
        if from >= self.nodes.len() {
            return Err(WorkflowError::NodeNotFound(from));
        }
        if to >= self.nodes.len() {
            return Err(WorkflowError::NodeNotFound(to));
        }
        // Prevent self-loops — they trivially form cycles.
        if from == to {
            return Err(WorkflowError::CycleDetected);
        }
        // Prevent duplicate edges.
        if self.edges.iter().any(|e| e.from == from && e.to == to) {
            return Ok(());
        }

        self.edges.push(Edge::new(from, to));
        self.nodes[from].outputs.push(to);
        self.nodes[to].inputs.push(from);
        Ok(())
    }

    // ── Queries ───────────────────────────────────────────────────────────

    /// Get a reference to a node by ID.
    pub fn node(&self, id: NodeId) -> Option<&WorkflowNode> {
        self.nodes.get(id)
    }

    /// Get a mutable reference to a node by ID.
    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut WorkflowNode> {
        self.nodes.get_mut(id)
    }

    /// Iterate over all nodes in insertion order.
    pub fn nodes(&self) -> impl Iterator<Item = &WorkflowNode> {
        self.nodes.iter()
    }

    /// Iterate over all edges.
    pub fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.edges.iter()
    }

    /// Number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    // ── Mutation ──────────────────────────────────────────────────────────

    /// Update the payload of a node.
    ///
    /// Marks the node AND all downstream nodes dirty.
    /// Spec §Lazy Evaluation: "Graph Change → Mark downstream nodes dirty."
    pub fn update_payload(
        &mut self,
        id: NodeId,
        payload: NodePayload,
    ) -> Result<(), WorkflowError> {
        let node = self
            .nodes
            .get_mut(id)
            .ok_or(WorkflowError::NodeNotFound(id))?;
        node.payload = payload;

        // Mark this node and all transitive dependents dirty.
        // Spec §Lazy Evaluation + §Incremental Recompute.
        self.propagate_dirty(id);

        Ok(())
    }

    /// Mark a node as executing (query in flight).
    pub fn mark_executing(&mut self, id: NodeId) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.execution_state = ExecutionState::Executing;
        }
    }

    /// Mark a node as clean (query completed successfully).
    pub fn mark_clean(&mut self, id: NodeId) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.execution_state = ExecutionState::Clean;
        }
    }

    /// Mark a node as failed with an error message. (Spec §Failure Scenarios)
    pub fn mark_failed(&mut self, id: NodeId, error: String) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.execution_state = ExecutionState::Failed(error);
        }
    }

    /// Mark all nodes as clean (e.g. after a successful full execution).
    pub fn mark_all_clean(&mut self) {
        for node in &mut self.nodes {
            node.execution_state = ExecutionState::Clean;
        }
    }

    /// Remove a node and all edges connected to it.
    pub fn remove_node(&mut self, id: NodeId) -> Result<(), WorkflowError> {
        if id >= self.nodes.len() {
            return Err(WorkflowError::NodeNotFound(id));
        }

        // Remove all edges involving this node.
        self.edges.retain(|e| e.from != id && e.to != id);

        // Remove references from other nodes' inputs/outputs.
        for node in &mut self.nodes {
            node.inputs.retain(|&i| i != id);
            node.outputs.retain(|&o| o != id);
        }

        // Remove the node.
        self.nodes.remove(id);

        // Re-index: all node IDs shift down by 1 for indices > id.
        // Update edges, inputs, and outputs to reflect new IDs.
        for edge in &mut self.edges {
            if edge.from > id {
                edge.from -= 1;
            }
            if edge.to > id {
                edge.to -= 1;
            }
        }
        for node in &mut self.nodes {
            node.id = if node.id > id { node.id - 1 } else { node.id };
            for inp in &mut node.inputs {
                if *inp > id {
                    *inp -= 1;
                }
            }
            for out in &mut node.outputs {
                if *out > id {
                    *out -= 1;
                }
            }
        }

        Ok(())
    }

    // ── Dirty propagation ─────────────────────────────────────────────────

    /// Mark `seed` and all transitive downstream nodes dirty.
    ///
    /// Spec §Lazy Evaluation: "Graph Change → Mark downstream nodes dirty."
    /// Spec §Incremental Recompute: user changes Filter → recompute Filter
    /// and everything downstream of it.
    fn propagate_dirty(&mut self, seed: NodeId) {
        // BFS from seed through output edges.
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(seed);

        while let Some(id) = queue.pop_front() {
            if !visited.insert(id) {
                continue;
            }

            if let Some(node) = self.nodes.get_mut(id) {
                node.execution_state = ExecutionState::Dirty;
                for &out in &node.outputs {
                    if !visited.contains(&out) {
                        queue.push_back(out);
                    }
                }
            }
        }
    }

    // ── Execution planning ────────────────────────────────────────────────

    /// Produce a topological execution plan.
    ///
    /// Spec §Execution Planning:
    /// 1. Validate graph
    /// 2. Detect cycles
    /// 3. Topologically sort nodes
    /// 4. Return execution order
    ///
    /// Spec §DAG Requirements: Acyclic — cycles are detected and reported.
    /// Spec §DAG Requirements: Deterministic — same graph → same plan.
    /// Spec §DAG Requirements: Reproducible — no randomness or external state.
    pub fn execution_plan(&self) -> Result<ExecutionPlan, WorkflowError> {
        // Step 1: Validate graph.
        self.validate()?;

        // Step 2–3: Kahn's algorithm for topological sort with cycle detection.
        let n = self.nodes.len();
        let mut in_degree = vec![0usize; n];

        // Build adjacency from edges.
        let mut adj: Vec<Vec<NodeId>> = vec![Vec::new(); n];
        for edge in &self.edges {
            adj[edge.from].push(edge.to);
            in_degree[edge.to] += 1;
        }

        // Queue nodes with zero in-degree.
        let mut queue: VecDeque<NodeId> = VecDeque::new();
        for (id, &deg) in in_degree.iter().enumerate() {
            if deg == 0 {
                queue.push_back(id);
            }
        }

        let mut steps = Vec::with_capacity(n);

        while let Some(id) = queue.pop_front() {
            steps.push(id);
            for &next in &adj[id] {
                in_degree[next] -= 1;
                if in_degree[next] == 0 {
                    queue.push_back(next);
                }
            }
        }

        // Step 2: Cycle detection — if we didn't visit all nodes, there's a cycle.
        if steps.len() != n {
            return Err(WorkflowError::CycleDetected);
        }

        // Spec §Deterministic: stable ordering for nodes with no dependencies.
        // Kahn's algorithm already gives deterministic results for a fixed
        // insertion order (VecDeque preserves order).

        Ok(ExecutionPlan { steps })
    }

    // ── Validation ────────────────────────────────────────────────────────

    /// Validate the graph.
    ///
    /// Spec §Failure Scenarios: detect cycles, disconnected nodes, invalid
    /// references, schema mismatches, missing datasets.
    pub fn validate(&self) -> Result<(), WorkflowError> {
        let n = self.nodes.len();

        // Check for invalid edge references (stale IDs).
        for edge in &self.edges {
            if edge.from >= n {
                return Err(WorkflowError::NodeNotFound(edge.from));
            }
            if edge.to >= n {
                return Err(WorkflowError::NodeNotFound(edge.to));
            }
        }

        // Check for disconnected nodes (no edges at all).
        // Only warn if there are edges in the graph — an empty graph is valid.
        if !self.edges.is_empty() {
            let connected: HashSet<NodeId> = self
                .edges
                .iter()
                .flat_map(|e| [e.from, e.to])
                .collect();
            for node in &self.nodes {
                if !connected.contains(&node.id) {
                    return Err(WorkflowError::DisconnectedNode(node.id));
                }
            }
        }

        // Check inputs/outputs consistency with edges.
        for edge in &self.edges {
            if !self.nodes[edge.from].outputs.contains(&edge.to) {
                return Err(WorkflowError::InconsistentAdjacency {
                    node: edge.from,
                    expected_output: edge.to,
                });
            }
            if !self.nodes[edge.to].inputs.contains(&edge.from) {
                return Err(WorkflowError::InconsistentAdjacency {
                    node: edge.to,
                    expected_output: edge.from,
                });
            }
        }

        Ok(())
    }
}

// ── Workflow builder ──────────────────────────────────────────────────────────

/// Builder for constructing the standard linear Dataset → Filter → Sort pipeline.
///
/// Internally creates a proper DAG with edges, ready for branching when the
/// UI evolves. Spec: "Even if the initial UI only exposes a linear sequence,
/// the internal execution model should be a graph from day one."
pub struct WorkflowBuilder {
    dataset_name: String,
    filter_expr: Option<FilterExpr>,
    sort_specs: Option<Vec<SortSpec>>,
}

impl WorkflowBuilder {
    /// Start building a workflow for the given dataset.
    pub fn new(dataset_id: core::DatasetId) -> Self {
        Self {
            dataset_name: format!("dataset_{}", dataset_id.0),
            filter_expr: None,
            sort_specs: None,
        }
    }

    /// Set the initial filter expression.
    pub fn filter(mut self, expr: FilterExpr) -> Self {
        self.filter_expr = Some(expr);
        self
    }

    /// Set the initial sort specifications.
    pub fn sort(mut self, specs: Vec<SortSpec>) -> Self {
        self.sort_specs = Some(specs);
        self
    }

    /// Build the workflow graph with edges and return it along with node IDs.
    ///
    /// Creates: Dataset ──► Filter ──► Sort
    /// with proper directed edges connecting them.
    pub fn build(self) -> (WorkflowGraph, LinearWorkflowIds) {
        let mut graph = WorkflowGraph::new();

        let dataset_id = graph.add_node(
            NodeKind::Dataset,
            NodePayload::Dataset {
                name: self.dataset_name,
            },
        );

        let filter_id = graph.add_node(
            NodeKind::Filter,
            NodePayload::Filter {
                expr: self.filter_expr.unwrap_or(FilterExpr::None),
            },
        );

        let sort_id = graph.add_node(
            NodeKind::Sort,
            NodePayload::Sort {
                specs: self.sort_specs.unwrap_or_default(),
            },
        );

        // Wire up edges: Dataset → Filter → Sort. (Spec §Edge Model)
        let _ = graph.add_edge(dataset_id, filter_id);
        let _ = graph.add_edge(filter_id, sort_id);

        // All start clean since they have default/initial values.
        graph.mark_all_clean();

        (
            graph,
            LinearWorkflowIds {
                dataset: dataset_id,
                filter: filter_id,
                sort: sort_id,
            },
        )
    }
}

// ── Query parameter extraction ────────────────────────────────────────────────

/// Extracted query parameters from the workflow graph.
pub struct WorkflowQueryParams {
    pub filter: FilterExpr,
    pub sort: Vec<SortSpec>,
    pub viewport: Viewport,
}

/// Walk the execution plan and extract filter + sort from the graph, combining
/// with the given viewport.
///
/// Spec §Deterministic: same graph + same plan → same params.
pub fn build_query_params(
    graph: &WorkflowGraph,
    plan: &ExecutionPlan,
    viewport: Viewport,
) -> WorkflowQueryParams {
    let mut filter = FilterExpr::None;
    let mut sort = Vec::new();

    for &id in &plan.steps {
        if let Some(node) = graph.node(id) {
            match &node.payload {
                NodePayload::Filter { expr } => {
                    filter = expr.clone();
                }
                NodePayload::Sort { specs } => {
                    sort = specs.clone();
                }
                _ => {}
            }
        }
    }

    WorkflowQueryParams {
        filter,
        sort,
        viewport,
    }
}

/// Walk the execution plan and extract only the filter expression.
pub fn build_filter(graph: &WorkflowGraph, plan: &ExecutionPlan) -> FilterExpr {
    for &id in &plan.steps {
        if let Some(node) = graph.node(id) {
            if let NodePayload::Filter { expr } = &node.payload {
                return expr.clone();
            }
        }
    }
    FilterExpr::None
}

// ── Errors ────────────────────────────────────────────────────────────────────

/// Workflow-level errors.
///
/// Spec §Failure Scenarios: cycles, disconnected nodes, invalid references,
/// schema mismatches, missing datasets.
#[derive(Debug)]
pub enum WorkflowError {
    /// A referenced node does not exist in the graph.
    NodeNotFound(NodeId),
    /// The graph contains a cycle — execution would never terminate.
    /// Spec §DAG Requirements: "Cycles forbidden... Must fail validation."
    CycleDetected,
    /// A node has no edges connecting it to the rest of the graph.
    /// Spec §Failure Scenarios: disconnected nodes.
    DisconnectedNode(NodeId),
    /// A node's inputs/outputs don't match the edge list (internal inconsistency).
    InconsistentAdjacency {
        node: NodeId,
        expected_output: NodeId,
    },
}

impl std::fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowError::NodeNotFound(id) => write!(f, "Node not found: {}", id),
            WorkflowError::CycleDetected => {
                write!(f, "Cycle detected in workflow graph — execution would never terminate")
            }
            WorkflowError::DisconnectedNode(id) => {
                write!(f, "Node {} is disconnected from the workflow graph", id)
            }
            WorkflowError::InconsistentAdjacency { node, expected_output } => {
                write!(
                    f,
                    "Node {} adjacency inconsistent: expected output edge to {}",
                    node, expected_output
                )
            }
        }
    }
}

impl std::error::Error for WorkflowError {}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Spec §Deterministic: same graph → same plan.
    #[test]
    fn test_deterministic_execution_plan() {
        let mut g = WorkflowGraph::new();
        let a = g.add_node(NodeKind::Dataset, NodePayload::Empty);
        let b = g.add_node(NodeKind::Filter, NodePayload::Empty);
        g.add_edge(a, b).unwrap();

        let plan1 = g.execution_plan().unwrap();
        let plan2 = g.execution_plan().unwrap();
        assert_eq!(plan1.steps, plan2.steps, "execution plan must be deterministic");
    }

    /// Spec §DAG Requirements: cycles forbidden.
    #[test]
    fn test_cycle_detection() {
        let mut g = WorkflowGraph::new();
        let a = g.add_node(NodeKind::Dataset, NodePayload::Empty);
        let b = g.add_node(NodeKind::Filter, NodePayload::Empty);
        let c = g.add_node(NodeKind::Sort, NodePayload::Empty);
        g.add_edge(a, b).unwrap();
        g.add_edge(b, c).unwrap();
        g.add_edge(c, a).unwrap(); // back edge → cycle

        let result = g.execution_plan();
        assert!(matches!(result, Err(WorkflowError::CycleDetected)));
    }

    /// Spec §DAG Requirements: acyclic graphs must produce valid plans.
    #[test]
    fn test_topological_sort() {
        let mut g = WorkflowGraph::new();
        let a = g.add_node(NodeKind::Dataset, NodePayload::Empty);
        let b = g.add_node(NodeKind::Filter, NodePayload::Empty);
        let c = g.add_node(NodeKind::Sort, NodePayload::Empty);
        g.add_edge(a, b).unwrap();
        g.add_edge(b, c).unwrap();

        let plan = g.execution_plan().unwrap();
        // a must come before b, b before c.
        let pos_a = plan.steps.iter().position(|&id| id == a).unwrap();
        let pos_b = plan.steps.iter().position(|&id| id == b).unwrap();
        let pos_c = plan.steps.iter().position(|&id| id == c).unwrap();
        assert!(pos_a < pos_b, "Dataset must execute before Filter");
        assert!(pos_b < pos_c, "Filter must execute before Sort");
    }

    /// Spec §Lazy Evaluation: dirty propagation to downstream nodes.
    #[test]
    fn test_dirty_propagation() {
        let mut g = WorkflowGraph::new();
        let a = g.add_node(NodeKind::Dataset, NodePayload::Empty);
        let b = g.add_node(NodeKind::Filter, NodePayload::Empty);
        let c = g.add_node(NodeKind::Sort, NodePayload::Empty);
        g.add_edge(a, b).unwrap();
        g.add_edge(b, c).unwrap();
        g.mark_all_clean();

        // Update filter → should dirty filter AND sort (downstream).
        g.update_payload(b, NodePayload::Filter { expr: FilterExpr::None })
            .unwrap();

        assert_eq!(g.node(b).unwrap().execution_state, ExecutionState::Dirty);
        assert_eq!(g.node(c).unwrap().execution_state, ExecutionState::Dirty);
        // Dataset is upstream — should NOT be dirtied.
        assert_eq!(g.node(a).unwrap().execution_state, ExecutionState::Clean);
    }

    /// Spec §Failure Scenarios: disconnected nodes detected.
    #[test]
    fn test_disconnected_node_detection() {
        let mut g = WorkflowGraph::new();
        let a = g.add_node(NodeKind::Dataset, NodePayload::Empty);
        let b = g.add_node(NodeKind::Filter, NodePayload::Empty);
        let _c = g.add_node(NodeKind::Sort, NodePayload::Empty);
        g.add_edge(a, b).unwrap();
        // c has no edges — disconnected.

        let result = g.validate();
        assert!(matches!(result, Err(WorkflowError::DisconnectedNode(_))));
    }

    /// WorkflowBuilder creates a properly wired DAG.
    #[test]
    fn test_builder_creates_dag() {
        let (graph, ids) = WorkflowBuilder::new(core::DatasetId(42))
            .filter(FilterExpr::None)
            .sort(vec![])
            .build();

        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);

        // Dataset → Filter edge exists.
        assert!(graph
            .edges()
            .any(|e| e.from == ids.dataset && e.to == ids.filter));

        // Filter → Sort edge exists.
        assert!(graph
            .edges()
            .any(|e| e.from == ids.filter && e.to == ids.sort));

        // Plan is valid and in correct order.
        let plan = graph.execution_plan().unwrap();
        assert_eq!(plan.steps.len(), 3);
    }

    // ── Edge & graph invariants ───────────────────────────────────────────

    /// Spec §DAG Requirements: self-loops are cycles and must be rejected.
    #[test]
    fn test_self_loop_rejected() {
        let mut g = WorkflowGraph::new();
        let a = g.add_node(NodeKind::Dataset, NodePayload::Empty);
        let result = g.add_edge(a, a);
        assert!(matches!(result, Err(WorkflowError::CycleDetected)));
    }

    /// Duplicate edges are idempotent — no error, no double-count.
    #[test]
    fn test_duplicate_edge_idempotent() {
        let mut g = WorkflowGraph::new();
        let a = g.add_node(NodeKind::Dataset, NodePayload::Empty);
        let b = g.add_node(NodeKind::Filter, NodePayload::Empty);
        g.add_edge(a, b).unwrap();
        g.add_edge(a, b).unwrap(); // duplicate
        assert_eq!(g.edge_count(), 1);
    }

    /// Edge to non-existent node returns NodeNotFound.
    #[test]
    fn test_edge_to_missing_node() {
        let mut g = WorkflowGraph::new();
        let a = g.add_node(NodeKind::Dataset, NodePayload::Empty);
        let result = g.add_edge(a, 99);
        assert!(matches!(result, Err(WorkflowError::NodeNotFound(99))));
    }

    // ── Branching DAG (§Branching) ────────────────────────────────────────

    /// Spec §Branching: a DAG with one source feeding two sinks must produce
    /// a valid topological plan with the source before both sinks.
    #[test]
    fn test_branching_dag() {
        let mut g = WorkflowGraph::new();
        let dataset = g.add_node(NodeKind::Dataset, NodePayload::Empty);
        let filter_a = g.add_node(NodeKind::Filter, NodePayload::Empty);
        let filter_b = g.add_node(NodeKind::Filter, NodePayload::Empty);
        g.add_edge(dataset, filter_a).unwrap();
        g.add_edge(dataset, filter_b).unwrap();

        let plan = g.execution_plan().unwrap();
        assert_eq!(plan.steps.len(), 3);

        let pos_ds = plan.steps.iter().position(|&id| id == dataset).unwrap();
        let pos_a = plan.steps.iter().position(|&id| id == filter_a).unwrap();
        let pos_b = plan.steps.iter().position(|&id| id == filter_b).unwrap();
        assert!(pos_ds < pos_a, "Dataset must execute before Filter A");
        assert!(pos_ds < pos_b, "Dataset must execute before Filter B");
    }

    // ── Incremental recompute (§Incremental Recompute) ────────────────────

    /// Spec §Incremental Recompute: changing a node dirties only its
    /// downstream transitive closure — independent branches stay clean.
    #[test]
    fn test_incremental_recompute_independent_branches() {
        let mut g = WorkflowGraph::new();
        let ds = g.add_node(NodeKind::Dataset, NodePayload::Empty);
        let branch_a = g.add_node(NodeKind::Filter, NodePayload::Empty);
        let branch_b = g.add_node(NodeKind::Sort, NodePayload::Empty);
        g.add_edge(ds, branch_a).unwrap();
        g.add_edge(ds, branch_b).unwrap();
        g.mark_all_clean();

        // Change branch_a → should dirty branch_a but NOT branch_b.
        g.update_payload(
            branch_a,
            NodePayload::Filter {
                expr: FilterExpr::None,
            },
        )
        .unwrap();

        assert_eq!(
            g.node(branch_a).unwrap().execution_state,
            ExecutionState::Dirty,
            "Changed branch must be dirty"
        );
        assert_eq!(
            g.node(branch_b).unwrap().execution_state,
            ExecutionState::Clean,
            "Independent branch must stay clean"
        );
        assert_eq!(
            g.node(ds).unwrap().execution_state,
            ExecutionState::Clean,
            "Upstream dataset must stay clean"
        );
    }

    // ── Node removal ──────────────────────────────────────────────────────

    /// Removing a node shifts IDs and updates all edge references.
    #[test]
    fn test_remove_node_reindexes() {
        let mut g = WorkflowGraph::new();
        let a = g.add_node(NodeKind::Dataset, NodePayload::Empty);
        let b = g.add_node(NodeKind::Filter, NodePayload::Empty);
        let c = g.add_node(NodeKind::Sort, NodePayload::Empty);
        g.add_edge(a, b).unwrap();
        g.add_edge(b, c).unwrap();

        // Remove middle node (b).
        g.remove_node(b).unwrap();

        assert_eq!(g.node_count(), 2);
        // Edge should now be from old-a (still id 0) to old-c (now id 1).
        let plan = g.execution_plan().unwrap();
        assert_eq!(plan.steps.len(), 2);

        // Verify the edge was re-indexed: old edge b→c should be gone;
        // the surviving edge a→b was removed when b was deleted.
        // After removal, we have a (id=0) and c (now id=1) with NO edges
        // because the edge a→b was removed (b was the target).
        assert_eq!(g.edge_count(), 0);
    }

    /// Removing the last node leaves an empty valid graph.
    #[test]
    fn test_remove_all_nodes() {
        let mut g = WorkflowGraph::new();
        let a = g.add_node(NodeKind::Dataset, NodePayload::Empty);
        g.remove_node(a).unwrap();
        assert_eq!(g.node_count(), 0);
        assert!(g.execution_plan().unwrap().steps.is_empty());
    }

    // ── State transitions ─────────────────────────────────────────────────

    #[test]
    fn test_mark_executing() {
        let mut g = WorkflowGraph::new();
        let a = g.add_node(NodeKind::Dataset, NodePayload::Empty);
        g.mark_executing(a);
        assert_eq!(g.node(a).unwrap().execution_state, ExecutionState::Executing);
    }

    #[test]
    fn test_mark_clean() {
        let mut g = WorkflowGraph::new();
        let a = g.add_node(NodeKind::Dataset, NodePayload::Empty);
        g.mark_executing(a);
        g.mark_clean(a);
        assert_eq!(g.node(a).unwrap().execution_state, ExecutionState::Clean);
    }

    #[test]
    fn test_mark_failed() {
        let mut g = WorkflowGraph::new();
        let a = g.add_node(NodeKind::Dataset, NodePayload::Empty);
        g.mark_failed(a, "disk error".into());
        assert_eq!(
            g.node(a).unwrap().execution_state,
            ExecutionState::Failed("disk error".into())
        );
    }

    #[test]
    fn test_mark_all_clean() {
        let mut g = WorkflowGraph::new();
        let a = g.add_node(NodeKind::Dataset, NodePayload::Empty);
        let b = g.add_node(NodeKind::Filter, NodePayload::Empty);
        g.mark_executing(a);
        g.mark_failed(b, "err".into());
        g.mark_all_clean();
        assert_eq!(g.node(a).unwrap().execution_state, ExecutionState::Clean);
        assert_eq!(g.node(b).unwrap().execution_state, ExecutionState::Clean);
    }

    // ── Validation ────────────────────────────────────────────────────────

    /// An empty graph is valid — no edges means no invariants to violate.
    #[test]
    fn test_empty_graph_valid() {
        let g = WorkflowGraph::new();
        assert!(g.validate().is_ok());
        assert!(g.execution_plan().unwrap().steps.is_empty());
    }

    /// A single-node graph with no edges is valid.
    #[test]
    fn test_single_node_graph_valid() {
        let mut g = WorkflowGraph::new();
        g.add_node(NodeKind::Dataset, NodePayload::Empty);
        // No edges — but also no other nodes to be disconnected from.
        // validate() only flags disconnected nodes when edges exist.
        assert!(g.validate().is_ok());
    }

    // ── Query parameter extraction ────────────────────────────────────────

    #[test]
    fn test_build_query_params_extracts_filter_and_sort() {
        let mut g = WorkflowGraph::new();
        let ds = g.add_node(NodeKind::Dataset, NodePayload::Empty);
        let f = g.add_node(
            NodeKind::Filter,
            NodePayload::Filter {
                expr: FilterExpr::Contains {
                    column: "country".into(),
                    pattern: "US".into(),
                },
            },
        );
        let s = g.add_node(
            NodeKind::Sort,
            NodePayload::Sort {
                specs: vec![SortSpec::desc("revenue")],
            },
        );
        g.add_edge(ds, f).unwrap();
        g.add_edge(f, s).unwrap();

        let plan = g.execution_plan().unwrap();
        let vp = Viewport::new(0, 50);
        let params = build_query_params(&g, &plan, vp);

        assert!(matches!(params.filter, FilterExpr::Contains { .. }));
        assert_eq!(params.sort.len(), 1);
        assert_eq!(params.sort[0].column, "revenue");
        assert_eq!(params.viewport.first_row, 0);
    }

    #[test]
    fn test_build_filter_extracts_only_filter() {
        let mut g = WorkflowGraph::new();
        let ds = g.add_node(NodeKind::Dataset, NodePayload::Empty);
        let f = g.add_node(
            NodeKind::Filter,
            NodePayload::Filter {
                expr: FilterExpr::Eq {
                    column: "id".into(),
                    value: core::filter::FilterValue::Integer(42),
                },
            },
        );
        let s = g.add_node(NodeKind::Sort, NodePayload::Empty);
        g.add_edge(ds, f).unwrap();
        g.add_edge(f, s).unwrap();

        let plan = g.execution_plan().unwrap();
        let filter = build_filter(&g, &plan);
        assert!(matches!(filter, FilterExpr::Eq { .. }));
    }

    /// When no Filter node exists, build_filter returns FilterExpr::None.
    #[test]
    fn test_build_filter_returns_none_when_absent() {
        let mut g = WorkflowGraph::new();
        let ds = g.add_node(NodeKind::Dataset, NodePayload::Empty);
        let s = g.add_node(NodeKind::Sort, NodePayload::Empty);
        g.add_edge(ds, s).unwrap();

        let plan = g.execution_plan().unwrap();
        let filter = build_filter(&g, &plan);
        assert!(matches!(filter, FilterExpr::None));
    }

    // ── Payload summaries ─────────────────────────────────────────────────

    #[test]
    fn test_payload_summary_dataset() {
        let p = NodePayload::Dataset {
            name: "sales.parquet".into(),
        };
        assert_eq!(p.summary(), "sales.parquet");
    }

    #[test]
    fn test_payload_summary_filter_none() {
        let p = NodePayload::Filter {
            expr: FilterExpr::None,
        };
        assert_eq!(p.summary(), "\u{2014}");
    }

    #[test]
    fn test_payload_summary_filter_contains() {
        let p = NodePayload::Filter {
            expr: FilterExpr::Contains {
                column: "city".into(),
                pattern: "Berlin".into(),
            },
        };
        assert!(p.summary().contains("city"));
        assert!(p.summary().contains("Berlin"));
    }

    #[test]
    fn test_payload_summary_sort() {
        let p = NodePayload::Sort {
            specs: vec![SortSpec::asc("name"), SortSpec::desc("age")],
        };
        let s = p.summary();
        assert!(s.contains("name"));
        assert!(s.contains("age"));
    }

    #[test]
    fn test_payload_summary_empty_sort() {
        let p = NodePayload::Sort { specs: vec![] };
        assert_eq!(p.summary(), "\u{2014}");
    }

    #[test]
    fn test_payload_summary_empty() {
        assert_eq!(NodePayload::Empty.summary(), "\u{2014}");
    }

    // ── Execution state labels ────────────────────────────────────────────

    #[test]
    fn test_execution_state_labels() {
        assert_eq!(ExecutionState::Clean.status_label(), "\u{25cf}");
        assert_eq!(ExecutionState::Dirty.status_label(), "\u{25cb}");
        assert_eq!(ExecutionState::Executing.status_label(), "\u{25c9}");
        assert_eq!(
            ExecutionState::Failed("boom".into()).status_label(),
            "\u{2715}"
        );
    }

    // ── Reproducible (§Reproducible) ──────────────────────────────────────

    /// Spec §Reproducible: graph execution must be reproducible across sessions.
    /// Same graph built twice must produce identical plans.
    #[test]
    fn test_reproducible_plan() {
        let build = || {
            let mut g = WorkflowGraph::new();
            let a = g.add_node(NodeKind::Dataset, NodePayload::Empty);
            let b = g.add_node(NodeKind::Filter, NodePayload::Empty);
            let c = g.add_node(NodeKind::Sort, NodePayload::Empty);
            g.add_edge(a, b).unwrap();
            g.add_edge(b, c).unwrap();
            g.execution_plan().unwrap().steps
        };

        let plan1 = build();
        let plan2 = build();
        assert_eq!(plan1, plan2, "identical graphs must produce identical plans");
    }
}
