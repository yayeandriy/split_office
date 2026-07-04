//! Workflow sidebar panel.
//!
//! Renders the workflow as a vertical list of nodes with their execution
//! state indicators. Phase 1 shows the linear Dataset → Filter → Sort chain.
//!
//! # Visual structure
//!
//! ```text
//! ◈ Dataset       sales.parquet        ●
//! ▽ Filter        country ∋ "US"       ○
//! ⇅ Sort          revenue ↓            ●
//! ```
//!
//! Columns:
//! - Icon + kind label (left)
//! - Parameter summary  (muted, truncated)
//! - Execution state dot (right, aligned)
//! - Remove button [×] (right)

use egui::Ui;

use crate::label;
use workflow::{ExecutionState, WorkflowGraph, NodeId};

// ── Public entry point ────────────────────────────────────────────────────────

/// Render the workflow panel into `ui`.
///
/// Returns a list of nodes the user wants to remove (empty if none).
pub fn workflow_panel(ui: &mut Ui, graph: &WorkflowGraph) -> Vec<NodeId> {
    label::section(ui, "Workflow");

    let mut to_remove: Vec<NodeId> = Vec::new();

    // Render nodes in topological order so the visual sequence matches
    // execution order. Fall back to insertion order if planning fails
    // (e.g. graph is empty).
    let ordered: Vec<NodeId> = graph
        .execution_plan()
        .map(|p| p.steps)
        .unwrap_or_else(|_| graph.nodes().map(|n| n.id).collect());

    if ordered.is_empty() {
        label::muted(ui, "No workflow steps yet.");
        return to_remove;
    }

    for id in ordered {
        let node = match graph.node(id) {
            Some(n) => n,
            None => continue,
        };

        let remove = render_node_row(ui, id, node);
        if remove {
            to_remove.push(id);
        }
    }

    to_remove
}

// ── Row rendering ─────────────────────────────────────────────────────────────

/// Render a single node row. Returns `true` if the user clicked [×].
fn render_node_row(
    ui: &mut Ui,
    _id: NodeId,
    node: &workflow::WorkflowNode,
) -> bool {
    let mut remove = false;

    // Dim the row if the node is disabled.
    let alpha = if node.enabled { 1.0 } else { 0.4 };
    let tint = egui::Color32::from_white_alpha((alpha * 255.0) as u8);
    let _visuals_override = tint; // reserved for future per-row tinting

    ui.horizontal(|ui| {
        // Kind icon (fixed width).
        ui.add_sized(
            [16.0, 14.0],
            egui::Label::new(
                egui::RichText::new(node.kind.icon())
                    .color(state_color(&node.execution_state))
                    .monospace(),
            ),
        );

        // Kind label.
        label::text(ui, node.kind.label());

        // Summary (muted, takes remaining space before the right-side buttons).
        let summary = node.payload.summary();
        if !summary.is_empty() && summary != "—" {
            label::muted(ui, &summary);
        }

        // Right-aligned execution state + remove button.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Remove button (not shown for Dataset nodes — they can't be removed).
            if node.kind != workflow::NodeKind::Dataset
                && ui.small_button("×").on_hover_text("Remove this step").clicked()
            {
                remove = true;
            }

            // Execution state indicator.
            let dot = node.execution_state.status_label();
            let color = state_color(&node.execution_state);
            ui.label(egui::RichText::new(dot).color(color));
        });
    });

    ui.separator();

    remove
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn state_color(state: &ExecutionState) -> egui::Color32 {
    match state {
        ExecutionState::Clean     => egui::Color32::from_rgb(80, 200, 120),   // green
        ExecutionState::Dirty     => egui::Color32::from_rgb(120, 120, 145),  // muted grey
        ExecutionState::Executing => egui::Color32::from_rgb(100, 160, 255),  // blue
        ExecutionState::Failed(_) => egui::Color32::from_rgb(220, 80,  80),   // red
    }
}
