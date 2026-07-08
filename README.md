# Split Office

> **Workspace-Centric Knowledge System** — a local-first analytical workbench where datasets, documents, workflows, and analyses coexist as interconnected objects in a single split-pane environment.

---

## What it is

Split Office is not a spreadsheet and not a document editor.

It is a *VS Code for data work*: a workspace that treats datasets, analyses, documents, and transformations as first-class objects — all visible side-by-side in resizable, composable split panes.

**Current phase:** Research prototype — split-pane grid viewer with per-tab scroll isolation, Blender-style workflow modifier stack, column profiling, and a document tab renderer.

---

## Screenshots

> Drop a Parquet or CSV to open → filter / sort / profile → split vertically or horizontally → each pane scrolls independently.

---

## Tech Stack

| Layer | Technology |
|---|---|
| UI framework | [egui](https://github.com/emilk/egui) + [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) (wgpu backend) |
| In-memory data | [Apache Arrow](https://arrow.apache.org/docs/rust/) |
| Query engine | [DuckDB](https://duckdb.org/) (bundled) |
| Profiling & stats | [Polars](https://pola.rs/) |
| Serialization | serde / serde_json |
| Logging | tracing / tracing-subscriber |

---

## Project Structure

```
split_office/
  app/                  ← Main binary (eframe app, wires everything together)
  bench/                ← Benchmarks (criterion)
  crates/               ← All library crates (flat; sibling paths stay valid)
    core/               ← Shared primitives: Dataset, Viewport, FilterExpr, SortSpec, …
    cdm/                ← Canonical Data Model IDs (IdGenerator, ObjectId)
    storage/            ← File loading: Parquet + CSV → DatasetHandle
    query/              ← DuckDB query engine: fetch_page, count_rows, stats
    expression/         ← Expression parser / evaluator (filter expressions)
    workflow/           ← DAG workflow graph: Dataset → Filter → Sort → … nodes
    transform/          ← Data transformation primitives
    lineage/            ← Data lineage tracking
    profiler/           ← Column profiling: distributions, cardinality, nulls, correlations
    project/            ← Project management (dataset + workflow + transform bundles)
    document_core/      ← Document CDM: Section, Block, Heading, Paragraph, Table, Reference
    document_view/      ← Document tab renderer (egui ScrollArea)
    workspace_core/     ← Workspace object registry (datasets, documents, analyses)
    grid/               ← Virtualized spreadsheet grid renderer (custom scrollbars, Arrow batch)
    split_view/         ← Split-pane / tab layout framework (VS Code-style)
  docs/                 ← Architecture specs and constitution documents
  assets/               ← App icons
  scripts/              ← Git hooks (pre-commit / pre-push version bump)
  test_data/            ← Sample Parquet / CSV files
  research/             ← Research notes and references
```

### Crate Dependency Graph

```
core ◄─── storage ◄─── query
  ▲           ▲            
  │       profiler        
  │                       
  ├── expression ◄── transform ◄── project
  ├── workflow ──────────────────► project
  ├── lineage                     
  └── grid                       

cdm ◄─── document_core ◄─── document_view
  ▲              
  └── workspace_core ◄─── split_view
```

`app` depends on: `core`, `storage`, `query`, `grid`, `profiler`, `workflow`, `document_core`, `document_view`, `cdm`, `workspace_core`, `split_view`.

---

## Build & Run

```bash
# Prerequisites: Rust stable (1.75+), macOS or Linux
cargo build --release
cargo run --bin split_office
```

Drop a `.parquet` or `.csv` file onto the window, or use **Open File** in the toolbar.

**Dev build** (faster iteration — `opt-level = 1`):

```bash
cargo run
```

**Benchmarks:**

```bash
cargo bench -p benchmark
```

---

## Key Concepts

### Split-Pane Layout (`split_view`)

Every view is a `LayoutNode` — a recursive tree of `Leaf`, `HSplit`, `VSplit`, and `Tabs`. The renderer walks the tree and dispatches to typed view callbacks (`render_spreadsheet`, `render_document`). View actions (close, new tab, reorder) are collected and applied after each frame.

Scroll events are routed by pointer position against each pane's `available_rect_before_wrap()` — each pane only consumes `smooth_scroll_delta` when the pointer is inside its own rect.

### Grid (`grid`)

Fully custom virtualized renderer — no `egui::ScrollArea`. It maintains per-tab `GridState` (scroll position, column widths, sort specs, filter text) and fetches only the visible row window from DuckDB via `viewport`. Each split tab gets its own `GridState` and its own slice of the shared Arrow batch (`batch_offset = tab_first_row − union_viewport_first_row`).

### Workflow DAG (`workflow`)

Filter → Sort pipeline represented as a directed graph of `WorkflowNode`s with typed `NodePayload`s. The sidebar renders a Blender-style modifier stack: drag to reorder, toggle to disable, expand to configure. The app translates the execution plan into `QueryParams` on every fetch.

### Profiler (`profiler`)

Runs on a background thread via Polars. Produces per-column `ColumnProfile`: cardinality, null fraction, inferred semantic type, top-N frequency, numeric distribution, and correlation matrix.

---

## Architecture Principles (from the Constitution)

- **Dataset First** — Dataset → Column → Transformation → View, not Workbook → Sheet → Cell.
- **Local First** — all execution is local; no cloud dependency.
- **Bounded Contexts** — `query` owns DuckDB; `profiler` owns Polars; domain code must not depend on egui.
- **Reproducibility** — every operation is inspectable via the workflow DAG; no hidden transformations.
- **Virtualization Everywhere** — never render invisible data.
- **Non-Blocking UI** — long-running work (queries, profiling) runs on background threads; UI stays at 60 FPS.

Full constitution: [`docs/analytical_workbench_constitution.md`](docs/analytical_workbench_constitution.md)

---

## Roadmap

| Phase | Status | Description |
|---|---|---|
| 0 | ✅ Research prototype | Split-pane grid, workflow sidebar, profiling, document tab |
| 1 | 🔜 Data Intelligence Foundation | Semantic type inference, inline sparklines, column relationships |
| 2 | Planned | Transformation Engine — expression language, derived columns |
| 3 | Planned | Workspace Foundation — persistent project files |
| 4 | Planned | Structured Documents — bidirectional dataset ↔ report references |
| 5 | Planned | Agent Workspace — agent operates on workspace objects |

Active TODO: [`TODO`](TODO)

---

## Development

### Conventions

- `cargo clippy` before committing.
- Version is bumped automatically by the pre-commit hook (`scripts/pre-commit`). To skip: `SKIP_BUMP=1 git commit`.
- All colours and font sizes in `grid/src/renderer.rs` are declared as top-level constants — no inline `Color32::from_rgb` literals in paint code.
- Domain crates (`workflow`, `transform`, `lineage`, `expression`) must not import `egui`.

### Adding a new crate

1. `cargo new --lib crates/<name>`
2. Add `"crates/<name>"` to `[workspace] members` in root `Cargo.toml`.
3. Add the crate to this README's **Project Structure** and **Dependency Graph** sections.

### Adding a new view type

1. Add a variant to `split_view/src/registry.rs` `ViewType`.
2. Add a render callback to `ViewContext` in `split_view/src/renderer.rs`.
3. Wire the callback in `app/src/app.rs`.
