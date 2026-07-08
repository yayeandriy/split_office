# Copilot Instructions — Split Office

## Project overview

Local-first analytical workbench in Rust. Split-pane UI (egui/eframe) where datasets, documents, and workflows coexist as first-class objects. Not a spreadsheet — workspace-centric.

Stack: egui 0.35 · eframe (wgpu) · Apache Arrow 58 · DuckDB 1.3 · Polars 0.46 · serde.

## Workspace structure

All library crates live under `crates/` (flat). The main binary is `app/`. Benchmarks in `bench/`. Full structure and crate descriptions: see `AGENTS.md`.

Key crates for completions:
- `crates/core` — `Dataset`, `Viewport`, `FilterExpr`, `SortSpec`, `ColumnType`
- `crates/grid` — `GridRenderer`, `GridState`, `GridAction`
- `crates/split_view` — `LayoutManager`, `LayoutNode`, `ViewId`, `TabAction`
- `crates/workflow` — `WorkflowGraph`, `WorkflowNode`, `NodePayload`
- `crates/query` — `QueryEngine`, `QueryParams`

## Coding conventions

- `Color32` values in `grid/src/renderer.rs` are **always** top-level `const` (`BG_HEADER_DARK`, etc.). Never suggest inline `Color32::from_rgb(…)` inside paint functions — use the `gc!(dark, TOKEN)` macro instead.
- Domain crates (`workflow`, `transform`, `lineage`, `expression`) must not use `egui` or `eframe`.
- Use `anyhow::Result` — no `unwrap()` in production paths.
- Heavy computation (DuckDB queries, Polars profiling) goes on background threads; results sent via `std::sync::mpsc`.

## README rule

When suggesting changes that add/remove a crate, add a directory, or introduce a major feature, also suggest updating the corresponding section of `README.md`. See `AGENTS.md → README Maintenance` for the mapping.
