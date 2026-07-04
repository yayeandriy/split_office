# Phase 0 — Research Prototype

## Goal

Validate:

```text
Rust
+
egui
+
wgpu
+
Arrow
+
Polars
+
DuckDB
```

against:

```text
100k rows
1M rows
10M rows
```

while maintaining:

```text
smooth scrolling
instant filtering
instant sorting
low memory
```

---

# Success Criteria

## Dataset Size

Must load:

```text
100k rows
```

instantly.

Should load:

```text
1M rows
```

comfortably.

Should remain usable:

```text
10M rows
```

with virtualization.

---

## Interaction Targets

| Action | Target |
|----------|----------|
| Vertical scroll | 60 FPS |
| Horizontal scroll | 60 FPS |
| Sort column | < 200 ms |
| Filter column | < 200 ms |
| Open dataset | < 2 s |
| Resize column | Instant |
| Select rows | Instant |

---

# Prototype Scope

Build exactly:

```text
Dataset Loader
Grid Renderer
Query Engine
Column Inspector
```

Nothing more.

---

# Repository Layout

```text
workspace/
│
├── app/
│
├── core/
│
├── grid/
│
├── query/
│
├── storage/
│
└── benchmark/
```

---

# Crate Responsibilities

## app

egui shell.

Responsibilities:

```text
windows
panels
menus
routing
```

No business logic.

---

## storage

Handles:

```text
csv
parquet
arrow
```

Outputs:

```rust
DatasetHandle
```

---

## query

DuckDB integration.

Responsibilities:

```text
sorting
filtering
aggregation
```

---

## core

Dataset model.

---

## grid

Rendering.

Most experimental crate.

---

## benchmark

Performance tests.

Never skip this.

---

# Core Data Model

Avoid:

```rust
Workbook
Sheet
Cell
```

Use:

```rust
Dataset
Column
View
Selection
```

Example:

```rust
pub struct Dataset {
    id: DatasetId,
    schema: Schema,
    row_count: usize,
}
```

Column:

```rust
pub struct Column {
    name: String,
    dtype: DataType,
}
```

View:

```rust
pub struct View {
    dataset: DatasetId,
    filter: FilterExpr,
    sort: Vec<SortSpec>,
}
```

Notice:

- No cells
- No formulas
- No workbook

---

# Storage Layer

Start with:

## CSV

Only for import.

## Parquet

Primary format.

Everything eventually converts to:

```text
Parquet
```

---

# DuckDB Layer

Load dataset:

```sql
SELECT *
FROM sales
```

Filtering:

```sql
SELECT *
FROM sales
WHERE revenue > 1000
```

Sorting:

```sql
ORDER BY revenue DESC
```

Never implement custom sorting initially.

DuckDB already solved this.

---

# Polars Layer

Use Polars for:

```text
profiling
column statistics
sampling
transforms
```

Suggested split:

| Task | Tool |
|--------|--------|
| Query | DuckDB |
| Statistics | Polars |
| Transform | Polars |
| Storage | Arrow |
| UI | egui |

---

# Grid Architecture

This is the real project.

Everything else is solved.

## Never Render Rows

Render:

```text
visible rows only
```

Example:

Dataset:

```text
10,000,000 rows
```

Viewport:

```text
42 rows
```

Only those visible rows exist on screen.

---

## Viewport Model

```rust
pub struct Viewport {
    first_row: usize,
    visible_rows: usize,
}
```

Query:

```rust
dataset.slice(
    first_row,
    visible_rows
)
```

---

# Data Flow

```text
Parquet
↓
Arrow
↓
DuckDB
↓
Viewport Query
↓
Arrow Batch
↓
Grid Renderer
```

---

# Grid Rendering Strategy

Initially:

```rust
egui::Painter
```

Each visible cell:

```rust
Rect
Text
Background
```

No widgets.

No nested layouts.

No tables.

Manual painting.

```rust
for row in visible_rows {
    for column in visible_columns {
        paint_cell(...)
    }
}
```

---

# Column Widths

Store separately:

```rust
HashMap<ColumnId, f32>
```

Never derive from data every frame.

---

# Column Inspector

Shows:

```text
name
type
null count
unique count
```

Computed with Polars.

---

# Performance Dashboard

Always visible.

Display:

```text
FPS
frame time
memory
visible rows
query latency
```

---

# Benchmarks

## Dataset A

```text
100k rows
20 columns
```

## Dataset B

```text
1M rows
50 columns
```

## Dataset C

```text
10M rows
100 columns
```

Synthetic. Stored as Parquet.

---

# Metrics To Capture

```text
load time
sort time
filter time
scroll FPS
memory
```

Logged automatically.

---

# Deliverable

User can:

```text
Open Parquet
↓
Inspect Schema
↓
Scroll Millions Of Rows
↓
Sort
↓
Filter
↓
View Column Statistics
```

with smooth performance.

---

# Most Important Discovery

Answer this question:

```text
Can Arrow batches be streamed
from DuckDB into a custom egui grid
at 60 FPS with multi-million-row datasets?
```

If yes, the rest of the roadmap becomes engineering.

If no, the architecture must change before additional features are built.
