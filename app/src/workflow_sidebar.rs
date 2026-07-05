//! Workflow modifier stack panel — Blender-style.
//!
//! # Spec: Workflow UI — Blender-Style Modifier Stack
//!
//! Each modifier card can be expanded to reveal editable settings:
//! - Filter: expression text input
//! - Sort: column name + direction toggle
//! - Aggregate: group-by + aggregate function inputs
//! - Derived Column: name + expression inputs
//!
//! ```text
//! ┌─ ▽ Filter ────────── [▶] [✓] [▲][▼] [×] ─●─┐
//! │  country = "US"                              │
//! └──────────────────────────────────────────────┘
//! ┌─ ▽ Filter ────────── [▼] [✓] [▲][▼] [×] ─●─┐  ← expanded
//! │  Column: [country        ▾]                  │
//! │  Contains: [US___________]                   │
//! │  ─────────────────────────────────           │
//! │  Where: country ∋ "US"                       │
//! └──────────────────────────────────────────────┘
//! ```

use egui::Ui;
use std::collections::HashSet;

use crate::label;
use workflow::{ExecutionState, NodeId, NodeKind, NodePayload, WorkflowGraph, WorkflowNode};

// ── Action types ──────────────────────────────────────────────────────────────

/// Actions the modifier stack panel wants the app to perform.
#[derive(Debug, Default)]
pub struct WorkflowActions {
    pub remove: Vec<NodeId>,
    pub move_up: Vec<NodeId>,
    pub move_down: Vec<NodeId>,
    pub toggle: Vec<NodeId>,
    pub duplicate: Vec<NodeId>,
    pub add_modifier: Option<NodeKind>,
    /// Payload updates from settings edits: (node_id, new_payload).
    pub settings_changes: Vec<(NodeId, NodePayload)>,
    /// Nodes to expand (clicked expand button).
    pub expand: Vec<NodeId>,
    /// Nodes to collapse.
    pub collapse: Vec<NodeId>,
}

impl WorkflowActions {
    pub fn is_empty(&self) -> bool {
        self.remove.is_empty()
            && self.move_up.is_empty()
            && self.move_down.is_empty()
            && self.toggle.is_empty()
            && self.duplicate.is_empty()
            && self.add_modifier.is_none()
            && self.settings_changes.is_empty()
            && self.expand.is_empty()
            && self.collapse.is_empty()
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Render the modifier stack panel into `ui`.
///
/// `expanded` tracks which modifier is currently expanded for settings editing.
pub fn workflow_panel(
    ui: &mut Ui,
    graph: &WorkflowGraph,
    expanded: &mut HashSet<NodeId>,
) -> WorkflowActions {
    let mut actions = WorkflowActions::default();

    // ── Header ────────────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        label::section(ui, "Modifiers");
    });

    // ── Render nodes in execution order ───────────────────────────────────
    let ordered: Vec<NodeId> = graph
        .execution_plan()
        .map(|p| p.steps)
        .unwrap_or_else(|_| graph.nodes().map(|n| n.id).collect());

    if ordered.is_empty() {
        label::muted(ui, "No modifiers yet.");
    } else {
        for (idx, &id) in ordered.iter().enumerate() {
            let node = match graph.node(id) {
                Some(n) => n,
                None => continue,
            };

            let is_first = idx == 0;
            let is_last = idx == ordered.len() - 1;
            let is_expanded = expanded.contains(&id);

            render_modifier_card(ui, id, node, is_first, is_last, is_expanded, &mut actions);
        }
    }

    // ── Add Modifier section ──────────────────────────────────────────────
    ui.add_space(6.0);
    render_add_modifier(ui, &mut actions);

    // Apply expand/collapse toggles from user clicks.
    for id in &actions.expand {
        if expanded.contains(id) {
            expanded.remove(id);
            actions.collapse.push(*id);
        } else {
            expanded.insert(*id);
        }
    }

    actions
}

// ── Modifier card ─────────────────────────────────────────────────────────────

fn render_modifier_card(
    ui: &mut Ui,
    id: NodeId,
    node: &WorkflowNode,
    is_first: bool,
    is_last: bool,
    is_expanded: bool,
    actions: &mut WorkflowActions,
) {
    let card_bg = egui::Color32::from_rgb(22, 22, 32);
    let card_border = egui::Color32::from_rgb(40, 40, 55);
    let dim = if node.enabled { 1.0 } else { 0.45 };

    egui::Frame::new()
        .fill(card_bg)
        .stroke(egui::Stroke::new(1.0, card_border))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            if !node.enabled {
                ui.set_opacity(dim);
            }

            // ── Header row ────────────────────────────────────────────────
            ui.horizontal(|ui| {
                // Expand/collapse toggle.
                let expand_icon = if is_expanded { "▼" } else { "▶" };
                if ui.small_button(expand_icon).on_hover_text("Expand / Collapse settings").clicked() {
                    actions.expand.push(id);
                }

                // Icon + name.
                let icon_color = state_color(&node.execution_state);
                ui.label(
                    egui::RichText::new(format!("{} {}", node.kind.icon(), node.kind.label()))
                        .color(icon_color)
                        .size(12.0),
                );

                ui.add_space(4.0);

                // Summary (truncated) — only when collapsed.
                if !is_expanded {
                    let summary = node.payload.summary();
                    if !summary.is_empty() && summary != "\u{2014}" {
                        label::muted(ui, &summary);
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Status dot.
                    let dot = node.execution_state.status_label();
                    ui.label(
                        egui::RichText::new(dot)
                            .color(state_color(&node.execution_state))
                            .size(10.0),
                    );

                    ui.add_space(4.0);

                    // Enable/disable toggle.
                    let toggle_label = if node.enabled { "✓" } else { "—" };
                    if ui.small_button(toggle_label).on_hover_text("Enable / Disable").clicked() {
                        actions.toggle.push(id);
                    }

                    if !is_first
                        && ui.small_button("▲").on_hover_text("Move up").clicked()
                    {
                        actions.move_up.push(id);
                    }

                    if !is_last
                        && ui.small_button("▼").on_hover_text("Move down").clicked()
                    {
                        actions.move_down.push(id);
                    }

                    if ui.small_button("⤓").on_hover_text("Duplicate").clicked() {
                        actions.duplicate.push(id);
                    }

                    if node.kind != NodeKind::Dataset
                        && ui.small_button("×").on_hover_text("Remove modifier").clicked()
                    {
                        actions.remove.push(id);
                    }
                });
            });

            // ── Expanded settings ─────────────────────────────────────────
            if is_expanded {
                ui.separator();
                render_modifier_settings(ui, id, node, actions);
            }

            ui.allocate_space(egui::Vec2::new(0.0, 2.0));
        });

    ui.add_space(4.0);
}

// ── Modifier settings (editable parameters) ───────────────────────────────────

fn render_modifier_settings(
    ui: &mut Ui,
    id: NodeId,
    node: &WorkflowNode,
    actions: &mut WorkflowActions,
) {
    match &node.payload {
        NodePayload::Dataset { name: _ } => {
            label::muted(ui, "Dataset source — no settings to edit.");
        }
        NodePayload::Filter { expr } => {
            render_filter_settings(ui, id, expr, actions);
        }
        NodePayload::Sort { specs } => {
            render_sort_settings(ui, id, specs, actions);
        }
        NodePayload::Empty => {
            label::muted(ui, "No settings available for this modifier type.");
        }
    }
}

// ── Filter settings ──────────────────────────────────────────────────────────

fn render_filter_settings(
    ui: &mut Ui,
    id: NodeId,
    expr: &core::FilterExpr,
    actions: &mut WorkflowActions,
) {
    // Extract current contains filter fields for editing.
    let (mut col, mut pattern) = match expr {
        core::FilterExpr::Contains { column, pattern } => (column.clone(), pattern.clone()),
        core::FilterExpr::None => (String::new(), String::new()),
        _ => (String::new(), String::new()),
    };

    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label("Column:");
        let mut col_buf = col.clone();
        if ui.text_edit_singleline(&mut col_buf).changed() {
            col = col_buf;
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Contains:");
        let mut pat_buf = pattern.clone();
        if ui.text_edit_singleline(&mut pat_buf).changed() {
            pattern = pat_buf;
            changed = true;
        }
    });

    // Quick filters.
    ui.horizontal(|ui| {
        if ui.small_button("Clear filter").clicked() {
            actions.settings_changes.push((id, NodePayload::Filter {
                expr: core::FilterExpr::None,
            }));
            return;
        }
    });

    if changed {
        if col.is_empty() && pattern.is_empty() {
            actions.settings_changes.push((id, NodePayload::Filter {
                expr: core::FilterExpr::None,
            }));
        } else {
            actions.settings_changes.push((id, NodePayload::Filter {
                expr: core::FilterExpr::Contains { column: col, pattern },
            }));
        }
    }
}

// ── Sort settings ─────────────────────────────────────────────────────────────

fn render_sort_settings(
    ui: &mut Ui,
    id: NodeId,
    specs: &[core::SortSpec],
    actions: &mut WorkflowActions,
) {
    if specs.is_empty() {
        ui.horizontal(|ui| {
            ui.label("Column:");
            let mut col_buf = String::new();
            if ui.text_edit_singleline(&mut col_buf).changed() && !col_buf.is_empty() {
                actions.settings_changes.push((id, NodePayload::Sort {
                    specs: vec![core::SortSpec::desc(col_buf)],
                }));
            }
        });
        return;
    }

    let spec = &specs[0]; // Edit first sort spec for now.
    let mut column = spec.column.clone();
    let mut is_desc = spec.direction == core::SortDirection::Descending;

    ui.horizontal(|ui| {
        ui.label("Column:");
        if ui.text_edit_singleline(&mut column).changed() {
            let new_spec = if is_desc {
                core::SortSpec::desc(&column)
            } else {
                core::SortSpec::asc(&column)
            };
            actions.settings_changes.push((id, NodePayload::Sort {
                specs: vec![new_spec],
            }));
        }
    });

    ui.horizontal(|ui| {
        ui.label("Direction:");
        if ui.selectable_label(!is_desc, "↑ Ascending").clicked() {
            is_desc = false;
            actions.settings_changes.push((id, NodePayload::Sort {
                specs: vec![core::SortSpec::asc(&column)],
            }));
        }
        if ui.selectable_label(is_desc, "↓ Descending").clicked() {
            is_desc = true;
            actions.settings_changes.push((id, NodePayload::Sort {
                specs: vec![core::SortSpec::desc(&column)],
            }));
        }
    });

    if ui.small_button("Clear sort").clicked() {
        actions.settings_changes.push((id, NodePayload::Sort {
            specs: vec![],
        }));
    }
}

// ── Add Modifier ──────────────────────────────────────────────────────────────

fn render_add_modifier(ui: &mut Ui, actions: &mut WorkflowActions) {
    let add_bg = egui::Color32::from_rgb(25, 25, 38);
    let add_border = egui::Color32::from_rgb(50, 60, 90);

    egui::Frame::new()
        .fill(add_bg)
        .stroke(egui::Stroke::new(1.0, add_border))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("+ Add Modifier").size(12.0));

                ui.add_space(8.0);

                if ui.small_button("Filter").on_hover_text("Add a filter step").clicked() {
                    actions.add_modifier = Some(NodeKind::Filter);
                }
                if ui.small_button("Sort").on_hover_text("Add a sort step").clicked() {
                    actions.add_modifier = Some(NodeKind::Sort);
                }
                if ui.small_button("Aggregate").on_hover_text("Add an aggregation step").clicked() {
                    actions.add_modifier = Some(NodeKind::Aggregate);
                }
                if ui.small_button("Derived").on_hover_text("Add a derived column").clicked() {
                    actions.add_modifier = Some(NodeKind::DerivedColumn);
                }
            });
        });
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn state_color(state: &ExecutionState) -> egui::Color32 {
    match state {
        ExecutionState::Clean => egui::Color32::from_rgb(80, 200, 120),
        ExecutionState::Dirty => egui::Color32::from_rgb(120, 120, 145),
        ExecutionState::Executing => egui::Color32::from_rgb(100, 160, 255),
        ExecutionState::Failed(_) => egui::Color32::from_rgb(220, 80, 80),
    }
}
