# Split Office — Claude Project Context

> Full context and conventions are in `AGENTS.md`. This file summarises the most important points for Claude Code.

---

## Project

Local-first analytical workbench in Rust/egui. Workspace-centric: datasets, documents, workflows, and analyses are first-class objects in composable split panes. **Not** a spreadsheet.

Stack: egui + eframe (wgpu) · Apache Arrow · DuckDB · Polars · serde.

## Layout

- `app/` — main binary
- `crates/` — all 15 library crates (flat; inter-crate deps are `../sibling`)
- `bench/` · `docs/` · `assets/` · `scripts/` · `test_data/` · `research/`

See `AGENTS.md` for the full crate-by-crate breakdown.

## Hard Rules

1. **Domain crates** (`workflow`, `transform`, `lineage`, `expression`) must not import `egui` or `eframe`.
2. All `Color32` values in `grid/src/renderer.rs` are top-level `const` — never add inline `Color32::from_rgb` in paint functions.
3. No `unwrap()` in production paths — use `anyhow::Result`.
4. Heavy work runs on background threads; results arrive via `mpsc::channel`. Never block the egui frame loop.
5. Scroll hit-tests use `ui.available_rect_before_wrap()` captured per-pane, **not** `ui.clip_rect()`.

## README Rule

Update `README.md` in the same commit whenever you add/remove a crate, add a top-level directory, land a major feature, or complete a roadmap phase. See `AGENTS.md → README Maintenance` for the full table.

## Version Bumps

The pre-commit hook auto-bumps the workspace minor version. For docs-only commits: `SKIP_BUMP=1 git commit`.
