//! Workflow modifier stack panel — Blender-style.
//!
//! Each modifier card can be expanded to reveal editable settings.
//! Column selectors use ComboBox dropdowns populated from the dataset schema.

use egui::Ui;
use std::collections::HashSet;

use crate::label;
use workflow::{ExecutionState, NodeId, NodeKind, NodePayload, WorkflowGraph, WorkflowNode};

// ── Action types ──────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct WorkflowActions {
    pub remove: Vec<NodeId>,
    pub move_up: Vec<NodeId>,
    pub move_down: Vec<NodeId>,
    pub toggle: Vec<NodeId>,
    pub duplicate: Vec<NodeId>,
    pub add_modifier: Option<NodeKind>,
    pub settings_changes: Vec<(NodeId, NodePayload)>,
    pub expand: Vec<NodeId>,
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

// ── Button helpers ────────────────────────────────────────────────────────────

/// A consistently-sized compact action button with rounded corners.
fn action_button(ui: &mut Ui, label: &str, hover: &str) -> bool {
    ui.button(egui::RichText::new(label).size(12.0))
        .on_hover_text(hover)
        .clicked()
}

fn action_button_small(ui: &mut Ui, label: &str, hover: &str) -> bool {
    ui.button(egui::RichText::new(label).size(11.0))
        .on_hover_text(hover)
        .clicked()
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Render the modifier stack panel into `ui`.
///
/// `columns` — available column names from the dataset schema (for dropdowns).
/// `expanded` — tracks which modifier cards are open for settings editing.
pub fn workflow_panel(
    ui: &mut Ui,
    graph: &WorkflowGraph,
    columns: &[String],
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

            render_modifier_card(ui, id, node, is_first, is_last, is_expanded, columns, &mut actions);
        }
    }

    // ── Add Modifier section ──────────────────────────────────────────────
    ui.add_space(6.0);
    render_add_modifier(ui, &mut actions);

    // Apply expand/collapse toggles.
    for id in &actions.expand {
        if expanded.contains(id) {
            expanded.remove(id);
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
    columns: &[String],
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
            // Scope all widget IDs to this stable node ID so they
            // don't shift when the card expands/collapses.
            ui.push_id(id, |ui| {
            if !node.enabled {
                ui.set_opacity(dim);
            }

            // ── Header row ────────────────────────────────────────────────
            ui.horizontal(|ui| {
                // Expand/collapse.
                let expand_icon = if is_expanded { "\u{25bc}" } else { "\u{25b6}" };
                if action_button_small(ui, expand_icon, "Expand / Collapse settings") {
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

                // Summary (only when collapsed).
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

                    // Enable/disable.
                    let toggle_label = if node.enabled { "\u{2713}" } else { "\u{2014}" };
                    if action_button_small(ui, toggle_label, "Enable / Disable") {
                        actions.toggle.push(id);
                    }

                    if !is_first && action_button_small(ui, "\u{25b2}", "Move up") {
                        actions.move_up.push(id);
                    }

                    if !is_last && action_button_small(ui, "\u{25bc}", "Move down") {
                        actions.move_down.push(id);
                    }

                    if action_button_small(ui, "\u{2913}", "Duplicate") {
                        actions.duplicate.push(id);
                    }

                    if node.kind != NodeKind::Dataset
                        && action_button_small(ui, "\u{00d7}", "Remove modifier")
                    {
                        actions.remove.push(id);
                    }
                });
            });
            }); // close ui.push_id scope

            // ── Expanded settings ─────────────────────────────────────────
            if is_expanded {
                ui.separator();
                render_modifier_settings(ui, id, node, columns, actions);
            }

            ui.allocate_space(egui::Vec2::new(0.0, 2.0));
        });

    ui.add_space(4.0);
}

// ── Modifier settings ─────────────────────────────────────────────────────────

fn render_modifier_settings(
    ui: &mut Ui,
    id: NodeId,
    node: &WorkflowNode,
    columns: &[String],
    actions: &mut WorkflowActions,
) {
    match &node.payload {
        NodePayload::Dataset { name: _ } => {
            label::muted(ui, "Dataset source \u{2014} no settings to edit.");
        }
        NodePayload::Filter { expr } => {
            render_filter_settings(ui, id, expr, columns, actions);
        }
        NodePayload::Sort { specs } => {
            render_sort_settings(ui, id, specs, columns, actions);
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
    columns: &[String],
    actions: &mut WorkflowActions,
) {
    let (mut col, mut pattern) = match expr {
        core::FilterExpr::Contains { column, pattern } => (column.clone(), pattern.clone()),
        core::FilterExpr::None => (String::new(), String::new()),
        _ => (String::new(), String::new()),
    };

    let mut changed = false;

    // Column selector — ComboBox dropdown.
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Column:").size(12.0));
        let prev = col.clone();
        egui::ComboBox::from_id_salt(egui::Id::new(("filter_col", id)))
            .width(140.0)
            .selected_text(&col)
            .show_ui(ui, |ui| {
                for c in columns {
                    ui.selectable_value(&mut col, c.clone(), c);
                }
                // Allow free-form text entry.
                let mut free = col.clone();
                if ui.text_edit_singleline(&mut free).changed() {
                    col = free;
                    changed = true;
                }
            });
        if col != prev {
            changed = true;
        }
    });

    // Pattern input.
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Contains:").size(12.0));
        let mut pat_buf = pattern.clone();
        if ui.add_sized([120.0, 18.0], egui::TextEdit::singleline(&mut pat_buf)).changed() {
            pattern = pat_buf;
            changed = true;
        }
    });

    if ui.button("Clear filter").clicked() {
        actions.settings_changes.push((id, NodePayload::Filter {
            expr: core::FilterExpr::None,
        }));
        return;
    }

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
    columns: &[String],
    actions: &mut WorkflowActions,
) {
    if specs.is_empty() {
        let mut col = String::new();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Column:").size(12.0));
            egui::ComboBox::from_id_salt(egui::Id::new(("sort_col", id)))
                .width(140.0)
                .selected_text(if col.is_empty() { "\u{2014}" } else { &col })
                .show_ui(ui, |ui| {
                    for c in columns {
                        if ui.selectable_label(false, c).clicked() {
                            col = c.clone();
                            actions.settings_changes.push((id, NodePayload::Sort {
                                specs: vec![core::SortSpec::desc(&col)],
                            }));
                        }
                    }
                });
        });
        return;
    }

    let spec = &specs[0];
    let mut column = spec.column.clone();
    let mut is_desc = spec.direction == core::SortDirection::Descending;

    // Column selector.
    let prev_col = column.clone();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Column:").size(12.0));
        egui::ComboBox::from_id_salt(egui::Id::new(("sort_col2", id)))
            .width(140.0)
            .selected_text(&column)
            .show_ui(ui, |ui| {
                for c in columns {
                    ui.selectable_value(&mut column, c.clone(), c);
                }
            });
    });
    if column != prev_col {
        let new_spec = if is_desc {
            core::SortSpec::desc(&column)
        } else {
            core::SortSpec::asc(&column)
        };
        actions.settings_changes.push((id, NodePayload::Sort {
            specs: vec![new_spec],
        }));
    }

    // Direction toggle.
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Direction:").size(12.0));
        if ui.selectable_label(!is_desc, "\u{2191} Ascending").clicked() {
            is_desc = false;
            actions.settings_changes.push((id, NodePayload::Sort {
                specs: vec![core::SortSpec::asc(&column)],
            }));
        }
        if ui.selectable_label(is_desc, "\u{2193} Descending").clicked() {
            is_desc = true;
            actions.settings_changes.push((id, NodePayload::Sort {
                specs: vec![core::SortSpec::desc(&column)],
            }));
        }
    });

    if ui.button("Clear sort").clicked() {
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

                if action_button(ui, "Filter", "Add a filter step") {
                    actions.add_modifier = Some(NodeKind::Filter);
                }
                if action_button(ui, "Sort", "Add a sort step") {
                    actions.add_modifier = Some(NodeKind::Sort);
                }
                if action_button(ui, "Aggregate", "Add an aggregation step") {
                    actions.add_modifier = Some(NodeKind::Aggregate);
                }
                if action_button(ui, "Derived", "Add a derived column") {
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
