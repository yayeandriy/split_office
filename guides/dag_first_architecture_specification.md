# DAG-First Architecture Specification
## Future-Proof Workflow Engine for the Analytical Workbench

Version: 1.0

---

# Executive Summary

Do NOT model workflows internally as a linear pipeline.

Avoid:

Dataset
→ Filter
→ Aggregate
→ Sort

Instead model workflows as a Directed Acyclic Graph (DAG).

Even if the initial UI only exposes a linear sequence, the internal execution model should be a graph from day one.

This decision dramatically reduces future architectural migrations.

---

# Why DAG?

Linear pipelines eventually break down when users need:

- branching
- comparison
- experimentation
- reusable workflow fragments
- scenario analysis
- statistical pipelines
- agent-generated workflows

A DAG naturally supports all of these.

---

# Future Scenarios

## Scenario 1 — Compare Filters

Dataset

├─ Filter Country = US
│
└─ Filter Country = Germany

↓

Compare Revenue

Impossible with a strict pipeline.

Natural with a DAG.

---

## Scenario 2 — A/B Analysis

Dataset

├─ Version A
│
└─ Version B

↓

Metric Comparison

---

## Scenario 3 — Reusable Workflow Blocks

Dataset

↓

Cleaning Block

↓

Shared by multiple downstream analyses.

---

## Scenario 4 — Agent Generated Workflows

Agent creates:

Dataset

├─ Statistical Profile
├─ Correlation Analysis
├─ Forecast Pipeline
└─ Quality Checks

All running from a common source node.

---

# Graph Model

Every workflow consists of nodes and edges.

Nodes:

- dataset
- transformation
- view
- analysis
- output

Edges:

- dependency relationships

---

# Node Types

## Dataset Node

Source data.

No inputs.

Produces dataset output.

---

## Filter Node

Input:

Dataset

Output:

Filtered dataset

---

## Sort Node

Input:

Dataset

Output:

Sorted dataset

---

## Aggregate Node

Input:

Dataset

Output:

Aggregated dataset

---

## Join Node

Inputs:

Dataset A
Dataset B

Output:

Joined dataset

---

## Derived Column Node

Input:

Dataset

Output:

Dataset

---

## Analysis Node

Input:

Dataset

Output:

Metadata

Examples:

- profiling
- correlation
- quality analysis

---

## Visualization Node

Input:

Dataset

Output:

Renderable result

Examples:

- histogram
- scatter plot
- distribution summary

---

# Core Structures

Suggested abstraction:

WorkflowGraph

NodeId

Edge

ExecutionPlan

NodeMetadata

---

# Node Model

Every node should expose:

- unique id
- node type
- inputs
- outputs
- validation state
- execution state

---

# Edge Model

Every edge represents:

Dependency

Producer → Consumer

---

# Execution Planning

Execution should never traverse the graph arbitrarily.

Build an execution plan.

Steps:

1. Validate graph
2. Detect cycles
3. Topologically sort nodes
4. Execute in dependency order

---

# DAG Requirements

## Acyclic

Cycles forbidden.

Example:

A → B → C → A

Must fail validation.

---

## Deterministic

Same graph.

Same inputs.

Same outputs.

---

## Reproducible

Graph execution must be reproducible across sessions.

---

# Lazy Evaluation

Nodes should not execute immediately.

Instead:

Graph Change

↓

Mark downstream nodes dirty

↓

Recompute only when needed

---

# Incremental Recompute

Example:

Dataset
↓
Filter
↓
Aggregate
↓
Visualization

User changes Filter.

Only recompute:

Filter
Aggregate
Visualization

Do not recompute unrelated branches.

---

# Dirty State Tracking

Every node stores:

- clean
- dirty
- executing
- failed

---

# Caching Strategy

Cache node outputs.

Benefits:

- fast navigation
- branch switching
- agent experimentation

---

# Branching

A DAG naturally supports:

Dataset

├─ Revenue Analysis
├─ Forecast Pipeline
├─ Quality Assessment
└─ Correlation Study

without duplication.

---

# Workflow UI — Phase 2

Initial UI may remain linear.

Example:

Dataset
↓
Filter
↓
Aggregate
↓
Sort

Internally:

Still represented as DAG.

---

# Workflow UI — Future

Graph View

Nodes

Connections

Execution State

Lineage

Performance Metrics

---

# Lineage Integration

Every node contributes lineage.

Example:

Profit

↓

Revenue
Cost

Lineage graph becomes a subset of workflow graph.

---

# Agent Integration

Agents should never execute arbitrary actions.

Agents build DAG nodes.

Example:

Create Filter Node

Create Aggregate Node

Create Analysis Node

Connect Nodes

Submit Workflow

---

# Parallel Execution

Future optimization.

Independent branches:

Dataset

├─ Analysis A
└─ Analysis B

can execute concurrently.

---

# Benchmark Requirements

Measure:

- graph build time
- validation time
- execution planning time
- incremental recompute time
- cache hit rate

---

# Failure Scenarios

Detect:

- cycles
- disconnected nodes
- invalid references
- schema mismatches
- missing datasets

Graph remains inspectable after failure.

---

# Acceptance Criteria

The DAG implementation is complete when:

- nodes and edges exist
- cycle detection works
- topological execution works
- lazy execution works
- incremental recomputation works
- caching works
- lineage integrates
- future graph UI can be built without architectural changes

---

# Long-Term Vision

The DAG becomes the universal execution model for:

- transformations
- profiling
- statistics
- forecasting
- notebooks
- visual workflows
- automation
- agents

Everything becomes a graph.

The UI may evolve repeatedly.

The execution model should not need to.
