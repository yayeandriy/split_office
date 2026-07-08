# Split Office — Agent Context

This file is the canonical source of project context and conventions for all AI agents. See also: `CLAUDE.md`, `.github/copilot-instructions.md`, `.windsurfrules`, `.cursor/rules/`.

---

## What This Project Is

**Split Office** is a local-first analytical workbench built in Rust with egui. It is not a spreadsheet — it is a workspace-centric knowledge system where datasets, documents, workflows, and analyses coexist as first-class objects in composable split panes.

**Current phase:** Research prototype (Phase 0). Core features: virtualized grid with per-tab scroll isolation, Blender-style workflow modifier stack, DuckDB-backed query engine, column profiler (Polars), document tab renderer, VS Code-style split-pane layout.

---

## Workspace Layout

```
app/          ← Main binary — eframe App, wires all crates
bench/        ← criterion benchmarks
crates/       ← All library crates (flat; inter-crate paths are ../sibling)
  core/           Shared primitives: Dataset, Viewport, FilterExpr, SortSpec
  cdm/            Canonical Data Model IDs (IdGenerator, ObjectId)
  storage/        File loading: Parquet + CSV → DatasetHandle
  query/          DuckDB engine: fetch_page, count_rows, stats
  expression/     Expression parser / evaluator
  workflow/       DAG workflow graph: Dataset → Filter → Sort nodes
  transform/      Data transformation primitives
  lineage/        Data lineage tracking
  profiler/       Column profiling: distributions, cardinality, nulls, correlations
  project/        Project bundles (dataset + workflow + transform)
  document_core/  Document CDM: Section, Block, Heading, Paragraph, Table, Reference
  document_view/  Document tab renderer
  workspace_core/ Workspace object registry
  grid/           Virtualized spreadsheet grid (custom scrollbars, Arrow batches)
  split_view/     Split-pane / tab layout framework
docs/         ← Architecture specs and constitution
assets/       ← App icons
scripts/      ← Git hooks (pre-commit bumps minor version)
test_data/    ← Sample .parquet / .csv files
research/     ← Research notes
```

Adding a new crate: place it in `crates/`, add `"crates/<name>"` to `[workspace] members` in root `Cargo.toml`, and update the **Project Structure** section in `README.md`.

---

## Architecture Rules

These come from `docs/analytical_workbench_constitution.md` — follow them on every change.

1. **Bounded contexts** — `query` owns DuckDB. `profiler` owns Polars. Domain crates (`workflow`, `transform`, `lineage`, `expression`) must not import `egui`, `eframe`, or any IO crate.
2. **No business logic in UI** — `app/` orchestrates; domain crates compute.
3. **No global mutable state** — prefer owned values and message-passing over shared `Arc<Mutex<T>>` in hot paths.
4. **Reproducibility** — every transformation is a DAG node; no hidden mutations.
5. **Non-blocking UI** — heavy work (queries, profiling, IO) goes to background threads; results arrive via `mpsc::channel`.
6. **Virtualization everywhere** — never render rows that aren't visible.

---

## Coding Conventions

- **Colour constants** — in `grid/src/renderer.rs`, all `Color32` values are declared as top-level `const` with light/dark variants. Never add inline `Color32::from_rgb(…)` literals inside paint functions.
- **Font size constants** — same file; all `f32` font sizes are `const FONT_SIZE_*` at the top.
- **Error handling** — use `anyhow::Result` in library crates. Never `unwrap()` in production paths.
- **Scroll routing** — each grid reads `ui.available_rect_before_wrap()` (captured before any allocation) to decide whether a pointer event belongs to it. Do not use `ui.clip_rect()` for hit tests across panes.
- **Per-tab state** — each spreadsheet tab has its own `GridState` in `tab_grid_states: HashMap<ViewId, GridState>`. Batch rendering uses `batch_offset = tab_first_row − union_viewport_first_row` to read the correct slice.
- **Version bumps** — the pre-commit hook (`scripts/pre-commit`) bumps the workspace minor version automatically. Skip with `SKIP_BUMP=1 git commit` for docs-only commits.

---

## README Maintenance (Required)

**Update `README.md` in the same commit** whenever any of the following change:

| Change | Section to update |
|---|---|
| New / removed / renamed crate | Project Structure + Dependency Graph |
| New top-level directory | Project Structure tree |
| Tech stack dep added / removed | Tech Stack table |
| Major feature lands | Key Concepts or Roadmap |
| Phase milestone completed | Roadmap table |
| Build / run instructions change | Build & Run section |
| New dev convention | Development → Conventions |

Do **not** rewrite the Vision or Architecture Principles sections without explicit instruction.

---

## Build & Run

```bash
cargo build          # dev (opt-level 1)
cargo run            # launch the app
cargo build --release
cargo bench -p benchmark
```

Drop a `.parquet` or `.csv` file onto the window to open it.
