//! Recursive egui renderer for the layout tree.
//!
//! `render_layout` is the single entry point.  It walks the `LayoutNode` tree
//! and draws each leaf by dispatching to the appropriate view render callback.
//!
//! # Design decisions
//!
//! - No `egui::Panel` per split: we receive a single `egui::Rect` (the
//!   "content area") and subdivide it manually using `ui.new_child`.
//!   This avoids double-nested panel bugs and keeps all split logic in one place.
//! - The caller supplies view callbacks via `ViewContext`, keeping the renderer
//!   crate free of any dependency on `app`-level types.
//! - Only the *active* tab is rendered (spec §Rendering Rules).
//! - Drag-handle interaction updates the ratio immediately; the manager stores it.
//! - Tab actions (close, new, switch type, reorder) are collected into a `Vec`
//!   and returned to the caller for processing.
//!
//! # Spec compliance
//!
//! - Split Creation < 16 ms: O(depth) tree walk, no allocations in the hot path.
//! - Tab Switch < 16 ms: single `active_tab` index change, one repaint.
//! - No frame drops: hidden tabs are completely skipped.

use egui::{
    Color32, CursorIcon, Layout, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2,
};

use crate::layout::{LeafView, LayoutNode, SplitNode, TabContainer, ViewId};
use crate::manager::LayoutManager;
use crate::registry::ViewType;

// ── Public API ────────────────────────────────────────────────────────────────

/// Actions emitted by the tab bar that the caller must apply to the layout.
#[derive(Debug, Clone)]
pub enum TabAction {
    /// Close the tab at `tab_index` inside the tab group identified by `group_id`.
    CloseTab { group_id: ViewId, tab_index: usize },
    /// Create a new tab in the group, inheriting the group's current type.
    NewTab { group_id: ViewId },
    /// Create a new tab with a specific view type.
    NewTabWithType { group_id: ViewId, view_type: ViewType },
    /// Switch a single tab to a different view type.
    SwitchType { group_id: ViewId, tab_index: usize, new_type: ViewType },
    /// Reorder tabs within the same group.
    Reorder { group_id: ViewId, from_index: usize, to_index: usize },
}

/// Callbacks supplied by `app` for rendering each view type.
///
/// Each variant receives `(&mut Ui, &LeafView)`.  The renderer does not know
/// anything about datasets or documents beyond the opaque `LeafView.object_ref`.
///
/// Ambient panels (Explorer, Inspector) are now floating windows — they are
/// no longer part of the split-view layout.
pub struct ViewContext<'a> {
    pub render_spreadsheet: &'a mut dyn FnMut(&mut Ui, &LeafView),
    pub render_document:    &'a mut dyn FnMut(&mut Ui, &LeafView),
}

/// Top-level entry point.  Call this once per frame from `eframe::App::ui`.
///
/// Occupies `ui.available_rect_before_wrap()` in full.
/// Collected `TabAction`s are appended to `tab_actions`.
pub fn render_layout(
    ui: &mut Ui,
    mgr: &mut LayoutManager,
    ctx: &mut ViewContext<'_>,
    tab_actions: &mut Vec<TabAction>,
) {
    let rect = ui.available_rect_before_wrap();
    render_node(ui, &mut mgr.root, &mut mgr.focus.focused, rect, ctx, tab_actions);
}

// ── Constants ─────────────────────────────────────────────────────────────────

const DIVIDER_THICKNESS: f32 = 4.0;
const TAB_HEIGHT: f32 = 28.0;
const TAB_CLOSE_SIZE: f32 = 14.0;
const TAB_PLUS_SIZE: f32 = 18.0;
const FOCUS_BORDER_WIDTH: f32 = 1.5;

// ── Theme-aware colours ───────────────────────────────────────────────────────

fn is_dark(ui: &Ui) -> bool {
    ui.visuals().dark_mode
}

fn focus_border_color() -> Color32 {
    Color32::from_rgb(80, 140, 255)
}

fn tab_bar_bg(ui: &Ui) -> Color32 {
    if is_dark(ui) {
        Color32::from_gray(30)
    } else {
        Color32::from_rgb(225, 225, 232)
    }
}

fn tab_active_bg(ui: &Ui) -> Color32 {
    if is_dark(ui) {
        Color32::from_gray(55)
    } else {
        Color32::from_rgb(245, 245, 252)
    }
}

fn tab_active_text(ui: &Ui) -> Color32 {
    if is_dark(ui) {
        Color32::from_gray(230)
    } else {
        Color32::from_rgb(20, 20, 35)
    }
}

fn tab_inactive_text(ui: &Ui) -> Color32 {
    if is_dark(ui) {
        Color32::from_gray(160)
    } else {
        Color32::from_rgb(130, 130, 150)
    }
}

fn divider_color(ui: &Ui) -> Color32 {
    if is_dark(ui) {
        Color32::from_gray(50)
    } else {
        Color32::from_rgb(200, 200, 212)
    }
}

fn divider_hover_color(ui: &Ui) -> Color32 {
    if is_dark(ui) {
        Color32::from_gray(90)
    } else {
        Color32::from_rgb(160, 160, 180)
    }
}

fn close_btn_color(ui: &Ui, hovered: bool) -> Color32 {
    if hovered {
        if is_dark(ui) {
            Color32::from_rgb(200, 60, 60)
        } else {
            Color32::from_rgb(220, 70, 70)
        }
    } else if is_dark(ui) {
        Color32::from_gray(130)
    } else {
        Color32::from_gray(140)
    }
}

// ── Recursive renderer ────────────────────────────────────────────────────────

fn render_node(
    ui: &mut Ui,
    node: &mut LayoutNode,
    focused: &mut Option<ViewId>,
    rect: Rect,
    ctx: &mut ViewContext<'_>,
    tab_actions: &mut Vec<TabAction>,
) {
    match node {
        LayoutNode::Leaf(leaf) => render_leaf(ui, leaf, focused, rect, ctx),
        LayoutNode::HSplit(split) => render_hsplit(ui, split, focused, rect, ctx, tab_actions),
        LayoutNode::VSplit(split) => render_vsplit(ui, split, focused, rect, ctx, tab_actions),
        LayoutNode::Tabs(tabs) => render_tabs(ui, tabs, focused, rect, ctx, tab_actions),
    }
}

// ── Helpers: child UI ─────────────────────────────────────────────────────────

fn child_ui_at(ui: &mut Ui, rect: Rect) -> Ui {
    ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::top_down(egui::Align::LEFT)),
    )
}

// ── Leaf ──────────────────────────────────────────────────────────────────────

fn render_leaf(
    ui: &mut Ui,
    leaf: &mut LeafView,
    focused: &mut Option<ViewId>,
    rect: Rect,
    ctx: &mut ViewContext<'_>,
) {
    let is_focused = *focused == Some(leaf.view_id);

    let focus_response = ui.allocate_rect(rect, Sense::click());
    if focus_response.clicked() {
        *focused = Some(leaf.view_id);
    }

    if is_focused {
        ui.painter().rect_stroke(
            rect.shrink(1.0),
            0.0,
            Stroke::new(FOCUS_BORDER_WIDTH, focus_border_color()),
            StrokeKind::Outside,
        );
    }

    let inner_rect = if is_focused {
        rect.shrink(FOCUS_BORDER_WIDTH + 0.5)
    } else {
        rect
    };
    let mut child = child_ui_at(ui, inner_rect);
    dispatch_view(&mut child, leaf, ctx);
}

fn dispatch_view(ui: &mut Ui, leaf: &LeafView, ctx: &mut ViewContext<'_>) {
    match leaf.view_type {
        ViewType::Spreadsheet => (ctx.render_spreadsheet)(ui, leaf),
        ViewType::Document    => (ctx.render_document)(ui, leaf),
    }
}

// ── HSplit (top / bottom) ─────────────────────────────────────────────────────

fn render_hsplit(
    ui: &mut Ui,
    split: &mut SplitNode,
    focused: &mut Option<ViewId>,
    rect: Rect,
    ctx: &mut ViewContext<'_>,
    tab_actions: &mut Vec<TabAction>,
) {
    let total_h = rect.height();
    let divider_y = rect.min.y + total_h * split.ratio;

    let first_rect = Rect::from_min_max(rect.min, Pos2::new(rect.max.x, divider_y));
    let divider_rect = Rect::from_min_max(
        Pos2::new(rect.min.x, divider_y),
        Pos2::new(rect.max.x, divider_y + DIVIDER_THICKNESS),
    );
    let second_rect = Rect::from_min_max(
        Pos2::new(rect.min.x, divider_y + DIVIDER_THICKNESS),
        rect.max,
    );

    render_node(ui, &mut split.first, focused, first_rect, ctx, tab_actions);
    render_divider(ui, split, divider_rect, DividerAxis::Horizontal, rect);
    render_node(ui, &mut split.second, focused, second_rect, ctx, tab_actions);
}

// ── VSplit (left / right) ─────────────────────────────────────────────────────

fn render_vsplit(
    ui: &mut Ui,
    split: &mut SplitNode,
    focused: &mut Option<ViewId>,
    rect: Rect,
    ctx: &mut ViewContext<'_>,
    tab_actions: &mut Vec<TabAction>,
) {
    let total_w = rect.width();
    let divider_x = rect.min.x + total_w * split.ratio;

    let first_rect = Rect::from_min_max(rect.min, Pos2::new(divider_x, rect.max.y));
    let divider_rect = Rect::from_min_max(
        Pos2::new(divider_x, rect.min.y),
        Pos2::new(divider_x + DIVIDER_THICKNESS, rect.max.y),
    );
    let second_rect = Rect::from_min_max(
        Pos2::new(divider_x + DIVIDER_THICKNESS, rect.min.y),
        rect.max,
    );

    render_node(ui, &mut split.first, focused, first_rect, ctx, tab_actions);
    render_divider(ui, split, divider_rect, DividerAxis::Vertical, rect);
    render_node(ui, &mut split.second, focused, second_rect, ctx, tab_actions);
}

// ── Divider ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum DividerAxis {
    Horizontal,
    Vertical,
}

fn render_divider(
    ui: &mut Ui,
    split: &mut SplitNode,
    divider_rect: Rect,
    axis: DividerAxis,
    parent_rect: Rect,
) {
    let response = ui.allocate_rect(divider_rect, Sense::click_and_drag());

    let color = if response.hovered() || response.dragged() {
        divider_hover_color(ui)
    } else {
        divider_color(ui)
    };
    ui.painter().rect_filled(divider_rect, 0.0, color);

    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(match axis {
            DividerAxis::Horizontal => CursorIcon::ResizeVertical,
            DividerAxis::Vertical   => CursorIcon::ResizeHorizontal,
        });
    }

    if response.dragged() {
        let delta = response.drag_delta();
        let new_ratio = match axis {
            DividerAxis::Horizontal => {
                let parent_h = parent_rect.height().max(1.0);
                (response.rect.min.y + delta.y - parent_rect.min.y) / parent_h
            }
            DividerAxis::Vertical => {
                let parent_w = parent_rect.width().max(1.0);
                (response.rect.min.x + delta.x - parent_rect.min.x) / parent_w
            }
        };
        split.set_ratio(new_ratio);
    }
}

// ── Tabs ──────────────────────────────────────────────────────────────────────

/// Identity of a tab group — the ViewId of its first tab.
fn group_id(tabs: &TabContainer) -> ViewId {
    tabs.tabs.first().map(|t| t.view_id).unwrap_or(ViewId::new(0))
}

fn render_tabs(
    ui: &mut Ui,
    tabs: &mut TabContainer,
    focused: &mut Option<ViewId>,
    rect: Rect,
    ctx: &mut ViewContext<'_>,
    tab_actions: &mut Vec<TabAction>,
) {
    let gid = group_id(tabs);

    let tab_bar_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width(), TAB_HEIGHT));
    let content_rect = Rect::from_min_max(
        Pos2::new(rect.min.x, rect.min.y + TAB_HEIGHT),
        rect.max,
    );

    // ── Persistent popup state (survives across frames) ────────────────
    let plus_state_id = egui::Id::new(format!("plus_state_{:?}", gid));
    let ctx_state_id = egui::Id::new(format!("ctx_state_{:?}", gid));
    let plus_just_opened_id = egui::Id::new(format!("plus_just_{:?}", gid));
    // Stable, deterministic Area widget ID for context menu.
    let ctx_area_id = egui::Id::new(format!("ctx_area_{:?}", gid));

    let mut plus_open = ui.ctx().data_mut(|d| d.get_temp::<bool>(plus_state_id).unwrap_or(false));
    let mut ctx_open_data: Option<(usize, Pos2)> =
        ui.ctx().data_mut(|d| d.get_temp::<(usize, [f32; 2])>(ctx_state_id).map(|(i, p)| (i, Pos2::new(p[0], p[1]))));

    let save_plus = |open: bool, ui: &Ui| {
        ui.ctx().data_mut(|d| d.insert_temp(plus_state_id, open));
    };
    let save_ctx = |data: Option<(usize, Pos2)>, ui: &Ui| {
        ui.ctx().data_mut(|d| {
            if let Some((i, p)) = data {
                d.insert_temp(ctx_state_id, (i, [p.x, p.y]));
            } else {
                d.remove::<(usize, [f32; 2])>(ctx_state_id);
            }
        });
    };
    let save_plus_just_opened = |val: bool, ui: &Ui| {
        ui.ctx().data_mut(|d| d.insert_temp(plus_just_opened_id, val));
    };

    // Draw tab bar background.
    ui.painter().rect_filled(tab_bar_rect, 0.0, tab_bar_bg(ui));

    // ── Tab bar layout ──────────────────────────────────────────────────
    let mut tab_ui = child_ui_at(ui, tab_bar_rect);

    tab_ui.horizontal(|ui: &mut Ui| {
        ui.add_space(4.0);

        for (i, tab) in tabs.tabs.iter().enumerate() {
            let is_active = i == tabs.active_tab;

            let label = format!("{} {}", tab.view_type.icon(), tab.title);
            let text_color = if is_active {
                tab_active_text(ui)
            } else {
                tab_inactive_text(ui)
            };

            let galley = ui.painter().layout_no_wrap(
                label.clone(),
                egui::FontId::proportional(12.0),
                text_color,
            );
            let text_w = galley.size().x;
            let close_w = if tabs.tabs.len() > 1 { TAB_CLOSE_SIZE + 4.0 } else { 4.0 };
            let tab_w = text_w + close_w + 12.0;

            let (_, tab_rect) = ui.allocate_space(Vec2::new(tab_w, TAB_HEIGHT));

            if is_active {
                ui.painter().rect_filled(tab_rect, 2.0, tab_active_bg(ui));
            }

            // ── Unified click detection for the whole tab ──────────────
            let tab_resp = ui.allocate_rect(tab_rect, Sense::click());

            if tab_resp.clicked_by(egui::PointerButton::Secondary) {
                let pos = tab_resp.interact_pointer_pos().unwrap_or(tab_rect.center());
                ctx_open_data = Some((i, pos));
                save_ctx(Some((i, pos)), ui);
                continue;
            }

            // Check if click was in the close zone (right edge)
            let in_close_zone = tabs.tabs.len() > 1 && tab_resp.clicked() && {
                tab_resp.interact_pointer_pos()
                    .map_or(false, |p| p.x >= tab_rect.max.x - TAB_CLOSE_SIZE - 4.0)
            };

            if in_close_zone {
                tab_actions.push(TabAction::CloseTab { group_id: gid, tab_index: i });
                continue;
            }

            if tab_resp.clicked() {
                tabs.active_tab = i;
                *focused = Some(tab.view_id);
            }

            // Draw text
            let text_pos = Pos2::new(
                tab_rect.min.x + 6.0,
                tab_rect.center().y - galley.size().y / 2.0,
            );
            ui.painter().galley(text_pos, galley, text_color);

            // Close × decoration
            if tabs.tabs.len() > 1 {
                let cx = tab_rect.max.x - TAB_CLOSE_SIZE / 2.0 - 2.0;
                let cy = tab_rect.center().y;
                let close_color = if tab_resp.hovered() && tab_resp.hover_pos().map_or(false, |p| p.x >= tab_rect.max.x - TAB_CLOSE_SIZE - 4.0) {
                    close_btn_color(ui, true)
                } else {
                    close_btn_color(ui, false)
                };
                ui.painter().text(
                    Pos2::new(cx, cy),
                    egui::Align2::CENTER_CENTER,
                    "×",
                    egui::FontId::proportional(13.0),
                    close_color,
                );
            }

            ui.add_space(2.0);
        }

        // ── + New Tab button ──────────────────────────────────────────
        let (_, plus_alloc) = ui.allocate_space(Vec2::new(TAB_PLUS_SIZE + 4.0, TAB_HEIGHT));
        let plus_btn_rect = Rect::from_center_size(
            Pos2::new(plus_alloc.min.x + TAB_PLUS_SIZE / 2.0, plus_alloc.center().y),
            Vec2::new(TAB_PLUS_SIZE, TAB_PLUS_SIZE),
        );
        let plus_resp = ui.interact(plus_btn_rect, ui.next_auto_id(), Sense::click());
        let plus_color = if plus_resp.hovered() || plus_open {
            tab_active_text(ui)
        } else {
            tab_inactive_text(ui)
        };
        ui.painter().text(
            plus_btn_rect.center(),
            egui::Align2::CENTER_CENTER,
            "+",
            egui::FontId::proportional(16.0),
            plus_color,
        );
        if plus_resp.clicked() {
            if plus_open {
                plus_open = false;
                save_plus(false, ui);
            } else {
                plus_open = true;
                save_plus(true, ui);
                save_plus_just_opened(true, ui);
                ctx_open_data = None;
                save_ctx(None, ui);
            }
        }

        // ── Inline ghost tabs (shown when + is active) ─────────────────
        if plus_open {
            ui.push_id("ghost_tabs", |ui| {
            let just_opened = ui.ctx().data_mut(|d| d.get_temp::<bool>(plus_just_opened_id).unwrap_or(false));
            let ghost_text = tab_inactive_text(ui);
            let ghost_alpha = 0.55;

            let draw_ghost = |ui: &mut Ui, icon: &str, label: &str| -> bool {
                let text = format!("{} {}", icon, label);
                let galley = ui.painter().layout_no_wrap(
                    text.clone(),
                    egui::FontId::proportional(12.0),
                    ghost_text,
                );
                let w = galley.size().x + 14.0;
                let (_, rect) = ui.allocate_space(Vec2::new(w, TAB_HEIGHT));
                let ghost_bg = egui::Color32::from_rgba_premultiplied(
                    ghost_text.r(), ghost_text.g(), ghost_text.b(),
                    (ghost_alpha * 40.0) as u8,
                );
                ui.painter().rect_filled(rect, 2.0, ghost_bg);
                let text_pos = Pos2::new(rect.min.x + 7.0, rect.center().y - galley.size().y / 2.0);
                ui.painter().galley(text_pos, galley, ghost_text);
                let resp = ui.interact(rect, ui.next_auto_id(), Sense::click());
                resp.clicked()
            };

            let sheet_clicked = draw_ghost(ui, "⊞", "Sheet");
            let doc_clicked = draw_ghost(ui, "≡", "Doc");

            // Cancel ×
            let cancel_x_w = TAB_CLOSE_SIZE + 4.0;
            let (_, cancel_alloc) = ui.allocate_space(Vec2::new(cancel_x_w, TAB_HEIGHT));
            let cancel_rect = Rect::from_center_size(
                Pos2::new(cancel_alloc.min.x + TAB_CLOSE_SIZE / 2.0, cancel_alloc.center().y),
                Vec2::new(TAB_CLOSE_SIZE, TAB_CLOSE_SIZE),
            );
            let cancel_resp = ui.interact(cancel_rect, ui.next_auto_id(), Sense::click());
            let cancel_col = close_btn_color(ui, cancel_resp.hovered());
            ui.painter().text(
                cancel_rect.center(),
                egui::Align2::CENTER_CENTER,
                "×",
                egui::FontId::proportional(13.0),
                cancel_col,
            );

            let cancel_clicked = cancel_resp.clicked();

            // Outside click (skip on the frame it just opened)
            let outside_click = !just_opened && ui.ctx().input(|i| {
                i.pointer.primary_clicked() &&
                i.pointer.interact_pos().map_or(true, |p| {
                    p.y < tab_bar_rect.min.y || p.y > tab_bar_rect.max.y
                })
            });

            if sheet_clicked {
                tab_actions.push(TabAction::NewTabWithType {
                    group_id: gid,
                    view_type: ViewType::Spreadsheet,
                });
                plus_open = false;
                save_plus(false, ui);
            } else if doc_clicked {
                tab_actions.push(TabAction::NewTabWithType {
                    group_id: gid,
                    view_type: ViewType::Document,
                });
                plus_open = false;
                save_plus(false, ui);
            } else if cancel_clicked || outside_click {
                plus_open = false;
                save_plus(false, ui);
            }

            save_plus_just_opened(false, ui);
            }); // close push_id("ghost_tabs")
        }
    });

    // ── Context menu ────────────────────────────────────────────────────
    if let Some((tab_i, pos)) = ctx_open_data {
        let mut clicked_item = false;
        egui::Area::new(ctx_area_id)
            .fixed_pos(pos)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                ui.push_id("ctx_menu", |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(190.0);
                    let vt = tabs.tabs[tab_i].view_type;

                    if ui.button("✕  Close").clicked() {
                        tab_actions.push(TabAction::CloseTab { group_id: gid, tab_index: tab_i });
                        clicked_item = true;
                    }
                    if ui.button("✕  Close Others").clicked() {
                        for i in (0..tabs.tabs.len()).rev() {
                            if i != tab_i {
                                tab_actions.push(TabAction::CloseTab { group_id: gid, tab_index: i });
                            }
                        }
                        clicked_item = true;
                    }
                    ui.separator();
                    ui.label("Switch to");
                    let other_type = match vt {
                        ViewType::Spreadsheet => ViewType::Document,
                        ViewType::Document => ViewType::Spreadsheet,
                    };
                    if ui.button(format!("→  {}", other_type.label())).clicked() {
                        tab_actions.push(TabAction::SwitchType {
                            group_id: gid,
                            tab_index: tab_i,
                            new_type: other_type,
                        });
                        clicked_item = true;
                    }
                    ui.separator();
                    ui.label("New tab");
                    if ui.button("⊞  New Sheet").clicked() {
                        tab_actions.push(TabAction::NewTabWithType {
                            group_id: gid,
                            view_type: ViewType::Spreadsheet,
                        });
                        clicked_item = true;
                    }
                    if ui.button("📄  New Doc").clicked() {
                        tab_actions.push(TabAction::NewTabWithType {
                            group_id: gid,
                            view_type: ViewType::Document,
                        });
                        clicked_item = true;
                    }
                });
                });
            });
        let clicked_outside = ui.ctx().input(|i| {
            i.pointer.primary_clicked() &&
            i.pointer.interact_pos().map_or(true, |p| {
                p.x < pos.x || p.x > pos.x + 200.0 ||
                p.y < pos.y || p.y > pos.y + 220.0
            })
        });
        if clicked_item || clicked_outside {
            save_ctx(None, ui);
        }
    }

    // ── Render active tab content ───────────────────────────────────────
    if let Some(active_leaf) = tabs.tabs.get_mut(tabs.active_tab) {
        render_leaf(ui, active_leaf, focused, content_rect, ctx);
    }
}
