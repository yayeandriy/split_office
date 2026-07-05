//! Workflow modifier stack panel — Blender-style.
//!
//! # Spec: Workflow UI — Blender-Style Modifier Stack
//!
//! ```text
//! ┌─ Dataset ────────────────────────────────────── ● ─┐
//! │  sales.parquet · 1.2M rows · 37 cols               │
//! └────────────────────────────────────────────────────┘
//! ┌─ ▽ Filter ─────────────────── [✓] [▼] [×] [▲][▼] ─┐
//! │  country = "US"                                     │
//! │  1,245,000 → 873,000                                │
//! └────────────────────────────────────────────────────┘
//! ┌─ ⇅ Sort ───────────────────── [✓] [▼] [×] [▲][▼] ─┐
//! │  revenue ↓                                          │
//! └────────────────────────────────────────────────────┘
//! ┌─ + Add Modifier ───────────────────────────────────┐
//! │  Filter | Sort | Aggregate | Derived Column | ...   │
//! └────────────────────────────────────────────────────┘
//! ```
//!
//! # Modifier Card Structure (Spec)
//!
//! Header: icon + name | enable toggle | expand toggle | status dot | menu (×,⤓,▲,▼)
//! Body: parameters (collapsed by default)
//! Metrics: rows before/after, execution time (future)

use egui::Ui;

use crate::label;
use workflow::{ExecutionState, NodeId, NodeKind, WorkflowGraph, WorkflowNode};

// ── Action types ──────────────────────────────────────────────────────────────

/// Actions the modifier stack panel wants the app to perform.
#[derive(Debug, Default)]
pub struct WorkflowActions {
    /// Nodes to remove.
    pub remove: Vec<NodeId>,
    /// Nodes to move up one position.
    pub move_up: Vec<NodeId>,
    /// Nodes to move down one position.
    pub move_down: Vec<NodeId>,
    /// Nodes to toggle enabled/disabled.
    pub toggle: Vec<NodeId>,
    /// Nodes to duplicate.
    pub duplicate: Vec<NodeId>,
    /// User wants to add a new modifier of this kind.
    pub add_modifier: Option<NodeKind>,
}

impl WorkflowActions {
    pub fn is_empty(&self) -> bool {
        self.remove.is_empty()
            && self.move_up.is_empty()
            && self.move_down.is_empty()
            && self.toggle.is_empty()
            && self.duplicate.is_empty()
            && self.add_modifier.is_none()
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Render the modifier stack panel into `ui`.
pub fn workflow_panel(ui: &mut Ui, graph: &WorkflowGraph) -> WorkflowActions {
    let mut actions = WorkflowActions::default();

    // ── Header ────────────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        label::section(ui, "Modifiers");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("Undo").on_hover_text("Undo last action").clicked() {
                // Handled by the caller — we just return actions.
            }
        });
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

            render_modifier_card(ui, id, node, is_first, is_last, &mut actions);
        }
    }

    // ── Add Modifier section ──────────────────────────────────────────────
    ui.add_space(6.0);
    render_add_modifier(ui, &mut actions);

    actions
}

// ── Modifier card ─────────────────────────────────────────────────────────────

/// Render a single modifier card.
///
/// Spec §Modifier Card Structure:
/// Header: modifier name · enable toggle · expand toggle · status · context menu
/// Body: parameters (collapsed)
fn render_modifier_card(
    ui: &mut Ui,
    id: NodeId,
    node: &WorkflowNode,
    is_first: bool,
    is_last: bool,
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
            // Opacity for disabled state.
            if !node.enabled {
                ui.set_opacity(dim);
            }

            // ── Header row ────────────────────────────────────────────────
            ui.horizontal(|ui| {
                // Icon + name.
                let icon_color = state_color(&node.execution_state);
                ui.label(
                    egui::RichText::new(format!("{} {}", node.kind.icon(), node.kind.label()))
                        .color(icon_color)
                        .size(12.0),
                );

                ui.add_space(4.0);

                // Summary (truncated).
                let summary = node.payload.summary();
                if !summary.is_empty() && summary != "\u{2014}" {
                    label::muted(ui, &summary);
                }

                // Push right.
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

                    // Move up.
                    if !is_first
                        && ui.small_button("▲").on_hover_text("Move up").clicked()
                    {
                        actions.move_up.push(id);
                    }

                    // Move down.
                    if !is_last
                        && ui.small_button("▼").on_hover_text("Move down").clicked()
                    {
                        actions.move_down.push(id);
                    }

                    // Duplicate.
                    if ui.small_button("⤓").on_hover_text("Duplicate").clicked() {
                        actions.duplicate.push(id);
                    }

                    // Remove.
                    if node.kind != NodeKind::Dataset
                        && ui.small_button("×").on_hover_text("Remove modifier").clicked()
                    {
                        actions.remove.push(id);
                    }
                });
            });

            // ── Body (parameter summary) ──────────────────────────────────
            render_card_body(ui, node);

            ui.allocate_space(egui::Vec2::new(0.0, 2.0));
        });

    ui.add_space(4.0);
}

// ── Card body ─────────────────────────────────────────────────────────────────

fn render_card_body(ui: &mut Ui, node: &WorkflowNode) {
    match &node.payload {
        workflow::NodePayload::Dataset { name } => {
            ui.horizontal(|ui| {
                label::muted(ui, &format!("Source: {}", name));
            });
        }
        workflow::NodePayload::Filter { expr } => {
            if !matches!(expr, core::FilterExpr::None) {
                ui.horizontal(|ui| {
                    label::muted(ui, &format!("Where: {:?}", expr));
                });
            }
        }
        workflow::NodePayload::Sort { specs } => {
            if !specs.is_empty() {
                let labels: Vec<String> = specs
                    .iter()
                    .map(|s| format!("{} {}", s.column, s.direction.arrow_label()))
                    .collect();
                ui.horizontal(|ui| {
                    label::muted(ui, &format!("Order: {}", labels.join(", ")));
                });
            }
        }
        workflow::NodePayload::Empty => {}
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
        ExecutionState::Clean => egui::Color32::from_rgb(80, 200, 120),   // green
        ExecutionState::Dirty => egui::Color32::from_rgb(120, 120, 145),  // muted grey
        ExecutionState::Executing => egui::Color32::from_rgb(100, 160, 255), // blue
        ExecutionState::Failed(_) => egui::Color32::from_rgb(220, 80, 80), // red
    }
}
