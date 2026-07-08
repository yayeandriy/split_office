
# Workflow UI Specification
## Blender-Style Modifier Stack for Analytical Workflows

Version: 1.0

## Executive Summary

The workflow experience should be modeled after:

- Blender Modifiers
- Houdini Operators
- Fusion 360 Feature Stack

Not after:

- Excel formulas
- wizard-based analytics
- dashboard builders

The workflow stack becomes the primary interface for analytical transformations.

Users modify datasets through a stack of transformations.

Internally: DAG

Externally: Linear Modifier Stack

---

# Product Philosophy

Dataset = Source Object

Transformation = Modifier

View = Result

Mental model:

Dataset
↓
Modifier Stack
↓
Result

Users should think:

"I am modifying a dataset"

rather than

"I am editing cells"

---

# Main Layout

Three-panel layout:

Workflow Stack
|
Dataset View
|
Inspector

The workflow stack is always visible.

---

# Workflow Panel

Contains:

- Dataset Source
- Modifier Stack
- Add Modifier
- Execution Status
- Workflow Controls

---

# Dataset Entry

Displays:

- dataset name
- row count
- column count
- status

Example:

sales.parquet

1,245,000 rows

37 columns

---

# Modifier Cards

Each transformation is represented as a card.

Examples:

- Filter
- Sort
- Join
- Aggregate
- Derived Column
- Pivot
- Unpivot

---

# Modifier Card Structure

Header

Body

Metrics

Status

---

# Header

Contains:

- modifier name
- enable toggle
- expand toggle
- status indicator
- context menu

---

# Required Actions

- enable
- disable
- duplicate
- delete
- move up
- move down
- inspect output

---

# Modifier Body

Displays modifier parameters.

Example:

Filter

Revenue > 1000

Country = Germany

---

# Metrics Area

Displays:

Rows Before

Rows After

Execution Time

Memory Cost

Example:

1,245,000 → 873,000

14 ms

---

# Status States

- valid
- warning
- executing
- failed
- disabled
- dirty

---

# Expanded Modifier View

Shows:

- inputs
- outputs
- lineage
- validation
- statistics
- execution information

---

# Add Modifier Flow

User clicks Add Modifier.

Searchable catalog appears.

Categories:

- Filtering
- Sorting
- Aggregation
- Columns
- Reshaping
- Joins
- Statistics
- Future Agent Actions

---

# Reordering

Support:

- drag and drop
- move up
- move down

Reordering marks downstream modifiers dirty.

Only dependent nodes recompute.

---

# Enable / Disable

Disabled modifiers:

- remain in workflow
- preserve parameters
- preserve lineage

---

# Output Inspection

Critical requirement.

Selecting a modifier displays the output at that stage.

Workflow:

Dataset
↓
Filter
↓
Aggregate
↓
Sort

Selecting Aggregate displays Aggregate output.

Not final output.

---

# Diff View

Every modifier displays impact.

Rows:

1,245,000

↓

873,000

Removed:

372,000

Columns:

Added:
Profit
Margin

Removed:
None

---

# Lineage View

Every modifier exposes dependencies.

Example:

Profit

↓

Revenue
Cost

---

# Validation System

Continuous validation.

Examples:

- missing column
- invalid expression
- broken join
- type mismatch
- missing dataset

---

# Error Handling

Errors must be:

- actionable
- persistent
- visible

Never hidden.

---

# Undo / Redo

All modifier operations support:

- undo
- redo

Scenarios:

- create
- delete
- edit
- reorder
- enable
- disable

---

# Workflow Persistence

Persist:

- modifier order
- parameters
- states
- expanded cards
- selections
- inspector state

---

# Keyboard Support

Required:

- add modifier
- delete modifier
- duplicate modifier
- move up
- move down
- expand
- collapse
- search
- navigate stack

---

# Large Workflow Support

Target:

100+ modifiers

Requirements:

- virtualization
- incremental rendering
- efficient updates

---

# Performance Requirements

Modifier Creation:
< 50 ms

Modifier Edit:
< 50 ms

Navigation:
instant

Scroll:
60 FPS

---

# Accessibility

- keyboard navigation
- focus indicators
- predictable shortcuts
- consistent interaction model

---

# Testing Requirements

## Unit Tests

- creation
- deletion
- reordering
- validation
- lineage generation

---

## Integration Tests

- workflow execution
- persistence
- incremental recomputation
- undo/redo

---

## UI Tests

- rendering
- expansion
- selection
- drag-and-drop
- keyboard navigation

---

## Snapshot Tests

- modifier card
- expanded state
- disabled state
- error state
- large workflow

---

## Performance Tests

10 modifiers

50 modifiers

100 modifiers

250 modifiers

Measure:

- FPS
- memory
- update latency

---

# User Stories

Story 1

Create filter modifier and inspect output.

Story 2

Create derived column and inspect lineage.

Story 3

Disable modifier and compare results.

Story 4

Reorder modifiers and verify recomputation.

Story 5

Save and reload workflow.

Story 6

Inspect intermediate outputs.

Story 7

Recover from invalid modifier.

---

# Acceptance Criteria

Users can:

- create workflows
- edit workflows
- reorder workflows
- disable workflows
- inspect outputs
- inspect lineage
- undo changes
- redo changes
- save projects
- reload projects
- handle errors
- work with 100+ modifiers

while maintaining responsive UI.

---

# Long-Term Evolution

Phase 2:

Linear Modifier Stack

Future:

Graph View

Both operate on the same DAG.

The modifier stack remains the primary editing experience.

The graph view becomes an advanced visualization and debugging tool.
