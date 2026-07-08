use arrow::record_batch::RecordBatch;
use egui::{
    Color32, CursorIcon, FontId, Key, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind,
    Ui, Vec2,
};
use tracing::debug;

use core::Dataset;

use crate::cell::format_cell;
use crate::state::{GridAction, GridState};

// ── Color palette ────────────────────────────────────────────────────────────
//
// All colours must be declared here. No inline Color32::from_rgb literals
// anywhere else in this file (constitution §Limited Visual Vocabulary).
const BG_HEADER_DARK: Color32 = Color32::from_rgb(22, 22, 30);
const BG_FILTER_ROW_DARK: Color32 = Color32::from_rgb(17, 17, 24);
const BG_ROW_ODD_DARK: Color32 = Color32::from_rgb(18, 18, 26);
const BG_ROW_EVEN_DARK: Color32 = Color32::from_rgb(24, 24, 33);
const BG_ROW_SELECTED_DARK: Color32 = Color32::from_rgb(40, 70, 130);
const BG_ROW_HOVER_ODD_DARK: Color32 = Color32::from_rgb(28, 28, 36);
const BG_ROW_HOVER_EVEN_DARK: Color32 = Color32::from_rgb(34, 34, 43);
#[allow(dead_code)]
const BG_HOVER: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 12);
const FG_HEADER_DARK: Color32 = Color32::from_rgb(200, 200, 220);
const FG_TYPE_LABEL_DARK: Color32 = Color32::from_rgb(100, 100, 130);
const FG_CELL_TEXT_DARK: Color32 = Color32::from_rgb(220, 220, 230);
const FG_CELL_NULL_DARK: Color32 = Color32::from_rgb(90, 90, 110);
const FG_CELL_NUM_DARK: Color32 = Color32::from_rgb(130, 200, 255);
#[allow(dead_code)]
const FG_SORT_ARROW_DARK: Color32 = Color32::from_rgb(80, 160, 255);
const BORDER_COL_DARK: Color32 = Color32::from_rgb(40, 40, 55);
const RESIZE_HANDLE_DARK: Color32 = Color32::from_rgb(60, 80, 140);
const FILTER_BG_DARK: Color32 = Color32::from_rgb(30, 30, 42);
const FILTER_BG_ACTIVE_DARK: Color32 = Color32::from_rgb(40, 50, 70);
const FILTER_FOCUSED_DARK: Color32 = Color32::from_rgb(50, 60, 90);
const FILTER_TEXT_DARK: Color32 = Color32::from_rgb(200, 200, 220);
const FILTER_PLACEHOLDER_DARK: Color32 = Color32::from_rgb(70, 70, 100);
const SCROLLBAR_BG_DARK: Color32 = Color32::from_rgb(15, 15, 22);
const SCROLLBAR_THUMB_DARK: Color32 = Color32::from_rgb(60, 80, 140);

// Light theme variants
const BG_HEADER_LIGHT: Color32 = Color32::from_rgb(235, 235, 242);
const BG_FILTER_ROW_LIGHT: Color32 = Color32::from_rgb(245, 245, 250);
const BG_ROW_ODD_LIGHT: Color32 = Color32::from_rgb(255, 255, 255);
const BG_ROW_EVEN_LIGHT: Color32 = Color32::from_rgb(248, 248, 252);
const BG_ROW_SELECTED_LIGHT: Color32 = Color32::from_rgb(200, 220, 255);
const BG_ROW_HOVER_ODD_LIGHT: Color32 = Color32::from_rgb(240, 245, 255);
const BG_ROW_HOVER_EVEN_LIGHT: Color32 = Color32::from_rgb(235, 240, 250);
const FG_HEADER_LIGHT: Color32 = Color32::from_rgb(40, 40, 60);
const FG_TYPE_LABEL_LIGHT: Color32 = Color32::from_rgb(130, 130, 160);
const FG_CELL_TEXT_LIGHT: Color32 = Color32::from_rgb(30, 30, 50);
const FG_CELL_NULL_LIGHT: Color32 = Color32::from_rgb(160, 160, 180);
const FG_CELL_NUM_LIGHT: Color32 = Color32::from_rgb(20, 80, 180);
#[allow(dead_code)]
const FG_SORT_ARROW_LIGHT: Color32 = Color32::from_rgb(60, 140, 240);
const BORDER_COL_LIGHT: Color32 = Color32::from_rgb(210, 210, 225);
const RESIZE_HANDLE_LIGHT: Color32 = Color32::from_rgb(120, 150, 220);
const FILTER_BG_LIGHT: Color32 = Color32::from_rgb(238, 238, 245);
const FILTER_BG_ACTIVE_LIGHT: Color32 = Color32::from_rgb(220, 225, 240);
const FILTER_FOCUSED_LIGHT: Color32 = Color32::from_rgb(200, 210, 240);
const FILTER_TEXT_LIGHT: Color32 = Color32::from_rgb(30, 30, 50);
const FILTER_PLACEHOLDER_LIGHT: Color32 = Color32::from_rgb(160, 160, 180);
const SCROLLBAR_BG_LIGHT: Color32 = Color32::from_rgb(235, 235, 242);
const SCROLLBAR_THUMB_LIGHT: Color32 = Color32::from_rgb(150, 160, 200);

macro_rules! gc {
    ($dark:expr, BG_HEADER) => { if $dark { BG_HEADER_DARK } else { BG_HEADER_LIGHT } };
    ($dark:expr, BG_FILTER_ROW) => { if $dark { BG_FILTER_ROW_DARK } else { BG_FILTER_ROW_LIGHT } };
    ($dark:expr, BG_ROW_ODD) => { if $dark { BG_ROW_ODD_DARK } else { BG_ROW_ODD_LIGHT } };
    ($dark:expr, BG_ROW_EVEN) => { if $dark { BG_ROW_EVEN_DARK } else { BG_ROW_EVEN_LIGHT } };
    ($dark:expr, BG_ROW_SELECTED) => { if $dark { BG_ROW_SELECTED_DARK } else { BG_ROW_SELECTED_LIGHT } };
    ($dark:expr, BG_ROW_HOVER_ODD) => { if $dark { BG_ROW_HOVER_ODD_DARK } else { BG_ROW_HOVER_ODD_LIGHT } };
    ($dark:expr, BG_ROW_HOVER_EVEN) => { if $dark { BG_ROW_HOVER_EVEN_DARK } else { BG_ROW_HOVER_EVEN_LIGHT } };
    ($dark:expr, FG_HEADER) => { if $dark { FG_HEADER_DARK } else { FG_HEADER_LIGHT } };
    ($dark:expr, FG_TYPE_LABEL) => { if $dark { FG_TYPE_LABEL_DARK } else { FG_TYPE_LABEL_LIGHT } };
    ($dark:expr, FG_CELL_TEXT) => { if $dark { FG_CELL_TEXT_DARK } else { FG_CELL_TEXT_LIGHT } };
    ($dark:expr, FG_CELL_NULL) => { if $dark { FG_CELL_NULL_DARK } else { FG_CELL_NULL_LIGHT } };
    ($dark:expr, FG_CELL_NUM) => { if $dark { FG_CELL_NUM_DARK } else { FG_CELL_NUM_LIGHT } };
    ($dark:expr, FG_SORT_ARROW) => { if $dark { FG_SORT_ARROW_DARK } else { FG_SORT_ARROW_LIGHT } };
    ($dark:expr, BORDER_COL) => { if $dark { BORDER_COL_DARK } else { BORDER_COL_LIGHT } };
    ($dark:expr, RESIZE_HANDLE) => { if $dark { RESIZE_HANDLE_DARK } else { RESIZE_HANDLE_LIGHT } };
    ($dark:expr, FILTER_BG) => { if $dark { FILTER_BG_DARK } else { FILTER_BG_LIGHT } };
    ($dark:expr, FILTER_BG_ACTIVE) => { if $dark { FILTER_BG_ACTIVE_DARK } else { FILTER_BG_ACTIVE_LIGHT } };
    ($dark:expr, FILTER_FOCUSED) => { if $dark { FILTER_FOCUSED_DARK } else { FILTER_FOCUSED_LIGHT } };
    ($dark:expr, FILTER_TEXT) => { if $dark { FILTER_TEXT_DARK } else { FILTER_TEXT_LIGHT } };
    ($dark:expr, FILTER_PLACEHOLDER) => { if $dark { FILTER_PLACEHOLDER_DARK } else { FILTER_PLACEHOLDER_LIGHT } };
    ($dark:expr, SCROLLBAR_BG) => { if $dark { SCROLLBAR_BG_DARK } else { SCROLLBAR_BG_LIGHT } };
    ($dark:expr, SCROLLBAR_THUMB) => { if $dark { SCROLLBAR_THUMB_DARK } else { SCROLLBAR_THUMB_LIGHT } };
}

// ── Font size tokens ──────────────────────────────────────────────────────────
//
// All font sizes must be declared here (constitution §Limited Visual Vocabulary).
/// Primary font for column names and cell values.
const FONT_SIZE_CELL: f32 = 12.0;
/// Smaller font for column type labels shown below header names.
const FONT_SIZE_TYPE_LABEL: f32 = 9.0;
/// Font for filter row input text.
const FONT_SIZE_FILTER: f32 = 11.0;


/// The grid renderer.
///
/// Stateless — all state lives in `GridState`.
/// Call `show()` every frame; collect returned `GridAction`s to drive queries.
pub struct GridRenderer;

impl GridRenderer {
    /// Paint the grid inside `ui`.
    ///
    /// # Parameters
    /// - `ui`         — egui UI context.
    /// - `dataset`    — metadata (schema, row_count).
    /// - `batch`      — current Arrow page from DuckDB.
    /// - `state`      — mutable grid state (scroll, widths, selection).
    /// - `total_rows` — total rows matching current filter (for scrollbar).
    ///
    /// # Returns
    /// A list of actions the caller must process (sort, scroll, filter changes).
    pub fn show(
        ui: &mut Ui,
        dataset: &Dataset,
        batch: &RecordBatch,
        state: &mut GridState,
        total_rows: usize,
    ) -> Vec<GridAction> {
        let dark = ui.visuals().dark_mode;
        let mut actions = Vec::new();

        let available = ui.available_rect_before_wrap();

        // ── 1. Total content width (sum of all column widths) ──────────────
        let col_count = dataset.schema.column_count();
        let total_content_width: f32 = (0..col_count).map(|i| state.col_width(i)).sum::<f32>();

        // ── 2. Clamp horizontal scroll ─────────────────────────────────────
        let max_scroll_x = (total_content_width - available.width()).max(0.0);
        state.scroll_x = state.scroll_x.clamp(0.0, max_scroll_x);

        // ── 3. Scrollbar dims ──────────────────────────────────────────────
        let v_scrollbar_width: f32 = 12.0;
        let h_scrollbar_height: f32 = 12.0;

        let grid_rect = Rect::from_min_size(
            available.min,
            Vec2::new(
                available.width() - v_scrollbar_width,
                available.height() - h_scrollbar_height,
            ),
        );

        let total_header_h = GridState::total_header_height();
        let viewport_height = grid_rect.height() - total_header_h;
        let visible_rows = state.rows_in_viewport(grid_rect.height());
        let first_row = state.first_row();

        // ── 4. Allocate full grid rect for interaction ─────────────────────
        let (grid_response, painter) = ui.allocate_painter(grid_rect.size(), Sense::click_and_drag());

        // ── 5. Background ──────────────────────────────────────────────────
        painter.rect_filled(grid_rect, 0.0, gc!(dark, BG_ROW_ODD));

        // ── 6. Paint header + handle header clicks (sort) ─────────────────
        Self::paint_header(
            ui,
            &painter,
            dataset,
            state,
            grid_rect,
            &grid_response,
            &mut actions,
        );

        // ── 7. Paint filter row ────────────────────────────────────────────
        Self::paint_filter_row(
            ui,
            &painter,
            dataset,
            state,
            grid_rect,
            &grid_response,
            &mut actions,
        );

        // ── 8. Paint data rows ─────────────────────────────────────────────
        Self::paint_rows(
            ui,
            &painter,
            dataset,
            batch,
            state,
            grid_rect,
            first_row,
            visible_rows,
            &grid_response,
            &mut actions,
        );

        // ── 9. Column resize handles ────────────────────────────────────────
        Self::handle_column_resize(ui, &painter, dataset, state, grid_rect);

        // ── 10. Vertical scrollbar ──────────────────────────────────────────
        let v_scroll_rect = Rect::from_min_size(
            Pos2::new(grid_rect.right(), grid_rect.top()),
            Vec2::new(v_scrollbar_width, grid_rect.height()),
        );
        if let Some(new_first) =
            Self::vertical_scrollbar(ui, v_scroll_rect, total_rows, first_row, visible_rows)
        {
            state.scroll_y = new_first as f32 * GridState::ROW_HEIGHT;
            actions.push(GridAction::ScrollChanged {
                first_row: new_first,
            });
        }

        // ── 11. Horizontal scrollbar ───────────────────────────────────────
        let h_scroll_rect = Rect::from_min_size(
            Pos2::new(grid_rect.left(), grid_rect.bottom()),
            Vec2::new(grid_rect.width(), h_scrollbar_height),
        );
        Self::horizontal_scrollbar(
            ui,
            h_scroll_rect,
            total_content_width,
            grid_rect.width(),
            &mut state.scroll_x,
        );

        // ── 12. Mouse wheel scroll ─────────────────────────────────────────
        if grid_response.hovered() {
            let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
            if scroll_delta.y.abs() > 0.5 {
                let old_first = first_row;
                state.scroll_y = (state.scroll_y - scroll_delta.y).max(0.0);
                let new_first = state.first_row();
                tracing::debug!(
                    scroll_delta_y = scroll_delta.y,
                    old_first,
                    new_first,
                    scroll_y = state.scroll_y,
                    hovered = grid_response.hovered(),
                    "mouse wheel scroll"
                );
                if new_first != old_first {
                    actions.push(GridAction::ScrollChanged {
                        first_row: new_first,
                    });
                }
                ui.ctx().input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
            }
            if scroll_delta.x.abs() > 0.5 {
                state.scroll_x = (state.scroll_x - scroll_delta.x).clamp(0.0, max_scroll_x);
                ui.ctx().input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
            }
        }

        // ── 13. Keyboard navigation ────────────────────────────────────────
        if grid_response.has_focus() || grid_response.hovered() {
            let pressed_down = ui.input(|i| i.key_pressed(Key::ArrowDown));
            let pressed_up = ui.input(|i| i.key_pressed(Key::ArrowUp));
            let pressed_pgdn = ui.input(|i| i.key_pressed(Key::PageDown));
            let pressed_pgup = ui.input(|i| i.key_pressed(Key::PageUp));

            if pressed_down {
                state.scroll_y = (state.scroll_y + GridState::ROW_HEIGHT).min(
                    (total_rows as f32 - 1.0) * GridState::ROW_HEIGHT,
                );
                actions.push(GridAction::ScrollChanged { first_row: state.first_row() });
            }
            if pressed_up {
                state.scroll_y = (state.scroll_y - GridState::ROW_HEIGHT).max(0.0);
                actions.push(GridAction::ScrollChanged { first_row: state.first_row() });
            }
            if pressed_pgdn {
                state.scroll_y = (state.scroll_y + viewport_height)
                    .min((total_rows as f32 - 1.0) * GridState::ROW_HEIGHT);
                actions.push(GridAction::ScrollChanged { first_row: state.first_row() });
            }
            if pressed_pgup {
                state.scroll_y = (state.scroll_y - viewport_height).max(0.0);
                actions.push(GridAction::ScrollChanged { first_row: state.first_row() });
            }
        }

        actions
    }

    // ── Header ────────────────────────────────────────────────────────────

    fn paint_header(
        ui: &Ui,
        painter: &Painter,
        dataset: &Dataset,
        state: &GridState,
        grid_rect: Rect,
        grid_response: &Response,
        actions: &mut Vec<GridAction>,
    ) {
        let dark = ui.visuals().dark_mode;
        let header_rect = Rect::from_min_size(
            grid_rect.min,
            Vec2::new(grid_rect.width(), GridState::HEADER_HEIGHT),
        );
        painter.rect_filled(header_rect, 0.0, gc!(dark, BG_HEADER));
        painter.rect_stroke(
            header_rect,
            0.0,
            Stroke::new(1.0, gc!(dark, BORDER_COL)),
            StrokeKind::Inside,
        );

        // Detect header clicks via the grid response.
        let shift_held = ui.input(|i| i.modifiers.shift);
        let header_clicked = grid_response.clicked()
            && grid_response
                .interact_pointer_pos()
                .is_some_and(|pos| header_rect.contains(pos));

        let mut x = grid_rect.left() - state.scroll_x;
        let font = FontId::proportional(FONT_SIZE_CELL);

        for (idx, col) in dataset.schema.columns.iter().enumerate() {
            let w = state.col_width(idx);
            let col_rect = Rect::from_min_size(
                Pos2::new(x, grid_rect.top()),
                Vec2::new(w, GridState::HEADER_HEIGHT),
            );

            // Clip to grid area.
            if col_rect.right() < grid_rect.left() || col_rect.left() > grid_rect.right() {
                x += w;
                continue;
            }

            // Header click → sort.
            if header_clicked {
                if let Some(pos) = grid_response.interact_pointer_pos() {
                    if col_rect.contains(pos) {
                        actions.push(GridAction::SortRequested {
                            column: col.name.clone(),
                            shift_held,
                        });
                    }
                }
            }

            // Hover effect on header.
            let hovered = grid_response.hover_pos().is_some_and(|pos| col_rect.contains(pos));
            if hovered {
                painter.rect_filled(col_rect, 0.0, Color32::from_rgba_premultiplied(255, 255, 255, 8));
            }

            // Column label with sort indicator.
            let sort_indicator = match (state.sort_priority(&col.name), state.sort_direction(&col.name)) {
                (Some(prio), Some(dir)) => {
                    let arrow = dir.arrow_label();
                    if state.sort_specs.len() > 1 {
                        format!(" {}{}", prio, arrow)
                    } else {
                        format!(" {}", arrow)
                    }
                }
                _ => String::new(),
            };

            let label = format!("{}{}", col.name, sort_indicator);
            let type_label = format!(" {}", col.dtype.label());

            let text_pos = Pos2::new(
                (x + 6.0).max(grid_rect.left() + 2.0),
                grid_rect.top() + 8.0,
            );

            // Clip text to column bounds.
            painter.with_clip_rect(col_rect.intersect(grid_rect)).text(
                text_pos,
                egui::Align2::LEFT_TOP,
                &label,
                font.clone(),
                gc!(dark, FG_HEADER),
            );

            // Type label in dimmer color.
            let type_pos = Pos2::new(text_pos.x, text_pos.y + 12.0);
            painter
                .with_clip_rect(col_rect.intersect(grid_rect))
                .text(
                    type_pos,
                    egui::Align2::LEFT_TOP,
                    &type_label,
                    FontId::proportional(FONT_SIZE_TYPE_LABEL),
                    gc!(dark, FG_TYPE_LABEL),
                );

            // Column border.
            painter.line_segment(
                [
                    Pos2::new(col_rect.right().min(grid_rect.right()), grid_rect.top()),
                    Pos2::new(col_rect.right().min(grid_rect.right()), header_rect.bottom()),
                ],
                Stroke::new(1.0, gc!(dark, BORDER_COL)),
            );

            x += w;
        }
    }

    // ── Filter row ────────────────────────────────────────────────────────

    fn paint_filter_row(
        ui: &mut Ui,
        painter: &Painter,
        dataset: &Dataset,
        state: &mut GridState,
        grid_rect: Rect,
        grid_response: &Response,
        actions: &mut Vec<GridAction>,
    ) {
        let dark = ui.visuals().dark_mode;
        let filter_y = grid_rect.top() + GridState::HEADER_HEIGHT;
        let filter_rect = Rect::from_min_size(
            Pos2::new(grid_rect.left(), filter_y),
            Vec2::new(grid_rect.width(), GridState::FILTER_ROW_HEIGHT),
        );

        // Background.
        painter.rect_filled(filter_rect, 0.0, gc!(dark, BG_FILTER_ROW));
        painter.rect_stroke(
            filter_rect,
            0.0,
            Stroke::new(1.0, gc!(dark, BORDER_COL)),
            StrokeKind::Inside,
        );

        // Detect clicks in the filter row via the grid response.
        let filter_clicked = grid_response.clicked()
            && grid_response
                .interact_pointer_pos()
                .is_some_and(|pos| filter_rect.contains(pos));

        let font = FontId::proportional(FONT_SIZE_FILTER);
        let mut x = grid_rect.left() - state.scroll_x;

        for (_idx, col) in dataset.schema.columns.iter().enumerate() {
            let idx = _idx;
            let w = state.col_width(idx);
            let col_filter_rect = Rect::from_min_size(
                Pos2::new(x + 2.0, filter_y + 2.0),
                Vec2::new(w - 4.0, GridState::FILTER_ROW_HEIGHT - 4.0),
            );

            // Skip off-screen columns.
            if col_filter_rect.right() < grid_rect.left() || col_filter_rect.left() > grid_rect.right() {
                // Column separator.
                painter.line_segment(
                    [
                        Pos2::new((x + w).min(grid_rect.right()), filter_y),
                        Pos2::new((x + w).min(grid_rect.right()), filter_y + GridState::FILTER_ROW_HEIGHT),
                    ],
                    Stroke::new(1.0, gc!(dark, BORDER_COL)),
                );
                x += w;
                continue;
            }

            let is_focused = state.focused_filter_col.as_deref() == Some(&col.name);
            let has_filter = state.column_filters.contains_key(&col.name);

            // Filter input background.
            let bg = if is_focused {
                gc!(dark, FILTER_FOCUSED)
            } else if has_filter {
                gc!(dark, FILTER_BG_ACTIVE)
            } else {
                gc!(dark, FILTER_BG)
            };
            painter.rect_filled(col_filter_rect, 2.0, bg);

            // Handle click on this filter cell → set focus.
            if filter_clicked {
                if let Some(pos) = grid_response.interact_pointer_pos() {
                    if col_filter_rect.contains(pos) {
                        state.focused_filter_col = Some(col.name.clone());
                    }
                }
            }

            // Show current filter text or placeholder.
            let current_text = state.column_filters.get(&col.name).cloned().unwrap_or_default();
            let display_text = if current_text.is_empty() {
                if is_focused { String::new() } else { "Filter…".to_string() }
            } else {
                current_text.clone()
            };

            let is_placeholder = current_text.is_empty() && !is_focused;
            let text_color = if is_placeholder { gc!(dark, FILTER_PLACEHOLDER) } else { gc!(dark, FILTER_TEXT) };

            painter.with_clip_rect(col_filter_rect).text(
                Pos2::new(col_filter_rect.left() + 4.0, col_filter_rect.center().y - 5.0),
                egui::Align2::LEFT_TOP,
                &display_text,
                font.clone(),
                text_color,
            );

            // Column separator.
            painter.line_segment(
                [
                    Pos2::new((x + w).min(grid_rect.right()), filter_y),
                    Pos2::new((x + w).min(grid_rect.right()), filter_y + GridState::FILTER_ROW_HEIGHT),
                ],
                Stroke::new(1.0, gc!(dark, BORDER_COL)),
            );

            x += w;
        }

        // ── Keyboard input for the focused filter column ──────────────────
        if let Some(ref col_name) = state.focused_filter_col.clone() {
            let current_text = state.column_filters.get(col_name).cloned().unwrap_or_default();
            let mut new_text = current_text.clone();

            // Click outside the filter row → blur.
            if grid_response.clicked() {
                if let Some(pos) = grid_response.interact_pointer_pos() {
                    if !filter_rect.contains(pos) {
                        state.focused_filter_col = None;
                        return;
                    }
                }
            }

            ui.input(|i| {
                // Escape → clear filter and blur.
                if i.key_pressed(egui::Key::Escape) {
                    new_text.clear();
                    state.focused_filter_col = None;
                    state.set_column_filter(col_name.clone(), String::new());
                    actions.push(GridAction::FilterColumnChanged {
                        column: col_name.clone(),
                        text: String::new(),
                    });
                    return;
                }
                // Enter → blur (keep filter).
                if i.key_pressed(egui::Key::Enter) {
                    state.focused_filter_col = None;
                    return;
                }
                // Tab → blur (keep filter).
                if i.key_pressed(egui::Key::Tab) {
                    state.focused_filter_col = None;
                    return;
                }

                for event in &i.events {
                    match event {
                        egui::Event::Text(text) => {
                            new_text.push_str(text);
                        }
                        egui::Event::Key {
                            key: egui::Key::Backspace,
                            pressed: true,
                            repeat: false,
                            ..
                        } => {
                            new_text.pop();
                        }
                        _ => {}
                    }
                }
            });

            if new_text != current_text {
                state.set_column_filter(col_name.clone(), new_text.clone());
                actions.push(GridAction::FilterColumnChanged {
                    column: col_name.clone(),
                    text: new_text,
                });
            }
        }
    }

    // ── Data rows ─────────────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    fn paint_rows(
        _ui: &Ui,
        painter: &Painter,
        dataset: &Dataset,
        batch: &RecordBatch,
        state: &mut GridState,
        grid_rect: Rect,
        first_row: usize,
        visible_rows: usize,
        grid_response: &Response,
        actions: &mut Vec<GridAction>,
    ) {
        let dark = _ui.visuals().dark_mode;
        let font = FontId::proportional(FONT_SIZE_CELL);
        let total_header_h = GridState::total_header_height();
        let data_rect = Rect::from_min_size(
            Pos2::new(grid_rect.left(), grid_rect.top() + total_header_h),
            Vec2::new(grid_rect.width(), grid_rect.height() - total_header_h),
        );

        // Hover detection.
        let hover_row: Option<usize> = grid_response.hover_pos().and_then(|pos| {
            if data_rect.contains(pos) {
                let rel_y = pos.y - data_rect.top();
                Some((rel_y / GridState::ROW_HEIGHT) as usize)
            } else {
                None
            }
        });

        // Click detection.
        let clicked_row: Option<usize> = if grid_response.clicked() {
            grid_response.interact_pointer_pos().and_then(|pos| {
                if data_rect.contains(pos) {
                    let rel_y = pos.y - data_rect.top();
                    Some((rel_y / GridState::ROW_HEIGHT) as usize)
                } else {
                    None
                }
            })
        } else {
            None
        };

        for local_row in 0..visible_rows {
            let _absolute_row = first_row + local_row;

            // Y position of this row.
            let row_y = data_rect.top() + local_row as f32 * GridState::ROW_HEIGHT;
            if row_y > data_rect.bottom() {
                break;
            }

            let is_selected = state.selection.contains(local_row);
            let is_hovered = hover_row == Some(local_row);

            let bg = if is_selected {
                gc!(dark, BG_ROW_SELECTED)
            } else if is_hovered {
                if local_row % 2 == 0 {
                    gc!(dark, BG_ROW_HOVER_EVEN)
                } else {
                    gc!(dark, BG_ROW_HOVER_ODD)
                }
            } else if local_row % 2 == 0 {
                gc!(dark, BG_ROW_EVEN)
            } else {
                gc!(dark, BG_ROW_ODD)
            };

            // Row background.
            let row_rect = Rect::from_min_size(
                Pos2::new(grid_rect.left(), row_y),
                Vec2::new(grid_rect.width(), GridState::ROW_HEIGHT),
            );
            painter.rect_filled(row_rect, 0.0, bg);

            // Row separator.
            painter.line_segment(
                [
                    Pos2::new(grid_rect.left(), row_y + GridState::ROW_HEIGHT),
                    Pos2::new(grid_rect.right(), row_y + GridState::ROW_HEIGHT),
                ],
                Stroke::new(1.0, gc!(dark, BORDER_COL)),
            );

            // Paint each cell.
            let mut x = grid_rect.left() - state.scroll_x;
            for col_idx in 0..dataset.schema.column_count() {
                let w = state.col_width(col_idx);
                let col_rect = Rect::from_min_size(
                    Pos2::new(x, row_y),
                    Vec2::new(w, GridState::ROW_HEIGHT),
                );

                // Skip off-screen columns.
                if col_rect.right() < grid_rect.left() || col_rect.left() > grid_rect.right() {
                    x += w;
                    continue;
                }

                // Cell value from Arrow batch.
                let text = if local_row < batch.num_rows() {
                    format_cell(batch, col_idx, local_row)
                } else {
                    String::new()
                };

                let is_null = text == "null";
                let is_num = dataset
                    .schema
                    .columns
                    .get(col_idx)
                    .map(|c| c.dtype.is_numeric())
                    .unwrap_or(false);

                let color = if is_null {
                    gc!(dark, FG_CELL_NULL)
                } else if is_num {
                    gc!(dark, FG_CELL_NUM)
                } else {
                    gc!(dark, FG_CELL_TEXT)
                };

                let align = if is_num {
                    egui::Align2::RIGHT_CENTER
                } else {
                    egui::Align2::LEFT_CENTER
                };

                let text_x = if is_num {
                    (col_rect.right() - 6.0).min(grid_rect.right() - 6.0)
                } else {
                    (col_rect.left() + 6.0).max(grid_rect.left() + 2.0)
                };
                let text_pos = Pos2::new(text_x, row_y + GridState::ROW_HEIGHT * 0.5);

                painter
                    .with_clip_rect(col_rect.intersect(data_rect))
                    .text(text_pos, align, &text, font.clone(), color);

                // Column separator.
                painter.line_segment(
                    [
                        Pos2::new(col_rect.right().min(grid_rect.right()), row_y),
                        Pos2::new(
                            col_rect.right().min(grid_rect.right()),
                            row_y + GridState::ROW_HEIGHT,
                        ),
                    ],
                    Stroke::new(1.0, gc!(dark, BORDER_COL)),
                );

                x += w;
            }

            // Handle row click → selection.
            if Some(local_row) == clicked_row {
                state.selection.select_single(local_row);
                actions.push(GridAction::SelectionChanged(state.selection.clone()));
            }
        }
    }

    // ── Column resize handles ──────────────────────────────────────────────

    fn handle_column_resize(
        ui: &mut Ui,
        painter: &Painter,
        dataset: &Dataset,
        state: &mut GridState,
        grid_rect: Rect,
    ) {
        let dark = ui.visuals().dark_mode;
        let mut x = grid_rect.left() - state.scroll_x;

        for col_idx in 0..dataset.schema.column_count() {
            let w = state.col_width(col_idx);
            let handle_x = x + w - GridState::RESIZE_HANDLE_WIDTH * 0.5;

            let handle_rect = Rect::from_min_size(
                Pos2::new(handle_x, grid_rect.top()),
                Vec2::new(GridState::RESIZE_HANDLE_WIDTH, GridState::total_header_height()),
            );

            if handle_rect.right() < grid_rect.left() || handle_rect.left() > grid_rect.right() {
                x += w;
                continue;
            }

            let handle_response = ui.allocate_rect(handle_rect, Sense::drag());

            if handle_response.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeColumn);
                painter.rect_filled(handle_rect, 0.0, gc!(dark, RESIZE_HANDLE));
            }

            if handle_response.drag_started() {
                state.resizing = Some((col_idx, state.col_width(col_idx)));
            }

            if handle_response.dragged() {
                let delta = handle_response.drag_delta().x;
                let current = state.col_width(col_idx);
                let new_w = (current + delta).clamp(
                    GridState::MIN_COL_WIDTH,
                    GridState::MAX_COL_WIDTH,
                );
                state.column_widths.insert(col_idx, new_w);
            }

            if handle_response.drag_stopped() {
                state.resizing = None;
            }

            x += w;
        }
    }

    // ── Vertical scrollbar ─────────────────────────────────────────────────

    fn vertical_scrollbar(
        ui: &mut Ui,
        rect: Rect,
        total_rows: usize,
        first_row: usize,
        visible_rows: usize,
    ) -> Option<usize> {
        let dark = ui.visuals().dark_mode;
        if total_rows == 0 {
            return None;
        }

        let (response, painter) = ui.allocate_painter(rect.size(), Sense::click_and_drag());
        let painter = painter;
        painter.rect_filled(rect, 0.0, gc!(dark, SCROLLBAR_BG));

        let thumb_ratio = (visible_rows as f32 / total_rows as f32).min(1.0);
        let thumb_h = (rect.height() * thumb_ratio).max(20.0);
        let scroll_range = rect.height() - thumb_h;
        let thumb_y = rect.top()
            + scroll_range * (first_row as f32 / (total_rows.saturating_sub(visible_rows)) as f32).min(1.0);

        let thumb_rect = Rect::from_min_size(
            Pos2::new(rect.left() + 2.0, thumb_y),
            Vec2::new(rect.width() - 4.0, thumb_h),
        );
        painter.rect_filled(thumb_rect, 4.0, gc!(dark, SCROLLBAR_THUMB));

        if response.dragged() {
            let delta = response.drag_delta().y;
            let new_thumb_y = (thumb_y + delta - rect.top()).clamp(0.0, scroll_range);
            let fraction = new_thumb_y / scroll_range;
            let new_first = (fraction * (total_rows.saturating_sub(visible_rows)) as f32) as usize;
            return Some(new_first);
        }

        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let fraction = ((pos.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
                let new_first = (fraction * total_rows as f32) as usize;
                return Some(new_first);
            }
        }

        None
    }

    // ── Horizontal scrollbar ───────────────────────────────────────────────

    fn horizontal_scrollbar(
        ui: &mut Ui,
        rect: Rect,
        content_width: f32,
        viewport_width: f32,
        scroll_x: &mut f32,
    ) {
        let dark = ui.visuals().dark_mode;
        if content_width <= viewport_width {
            return;
        }

        let (response, painter) = ui.allocate_painter(rect.size(), Sense::click_and_drag());
        painter.rect_filled(rect, 0.0, gc!(dark, SCROLLBAR_BG));

        let thumb_ratio = (viewport_width / content_width).min(1.0);
        let thumb_w = (rect.width() * thumb_ratio).max(20.0);
        let scroll_range = rect.width() - thumb_w;
        let max_scroll = (content_width - viewport_width).max(1.0);
        let thumb_x = rect.left() + scroll_range * (*scroll_x / max_scroll).min(1.0);

        let thumb_rect = Rect::from_min_size(
            Pos2::new(thumb_x, rect.top() + 2.0),
            Vec2::new(thumb_w, rect.height() - 4.0),
        );
        painter.rect_filled(thumb_rect, 4.0, gc!(dark, SCROLLBAR_THUMB));

        if response.dragged() {
            let delta = response.drag_delta().x;
            let new_thumb_x = (thumb_x + delta - rect.left()).clamp(0.0, scroll_range);
            let fraction = new_thumb_x / scroll_range;
            *scroll_x = (fraction * max_scroll).clamp(0.0, max_scroll);
        }
    }
}
