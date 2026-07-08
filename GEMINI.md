# Split Office — Gemini Project Context

> Full context and conventions are in `AGENTS.md`. This file provides the key points for Gemini CLI.

---

## What This Is

Local-first analytical workbench in Rust. Composable split-pane workspace where datasets, documents, and workflows coexist as first-class objects. Built with egui (wgpu backend), Apache Arrow, DuckDB, and Polars.

## Directory Layout

```
app/       main binary
bench/     benchmarks
crates/    all 15 library crates (flat)
docs/      architecture specs
```

Key crates: `core` · `grid` · `split_view` · `workflow` · `query` · `profiler` · `document_core`. See `AGENTS.md` for descriptions of each.

## Constraints

1. Domain crates (`workflow`, `transform`, `lineage`, `expression`) must not depend on `egui`.
2. All colours in `grid/src/renderer.rs` are top-level `const` — never add inline `Color32::from_rgb(…)`.
3. No `unwrap()` in production code — use `anyhow::Result`.
4. Blocking work (DuckDB queries, Polars) runs on background threads.
5. Per-pane scroll isolation: hit-test uses `ui.available_rect_before_wrap()`, not `ui.clip_rect()`.

## README Rule

Update `README.md` in the same commit whenever you add/remove a crate, add a directory, or implement a significant feature. Refer to `AGENTS.md → README Maintenance` for which section to update.
