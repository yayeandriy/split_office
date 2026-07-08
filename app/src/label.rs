//! Canonical text rendering for Split Office.
//!
//! # Typography constitution
//!
//! Every piece of UI text MUST use one of these functions — never raw
//! `ui.label()` or `ui.heading()`. This guarantees consistent font style,
//! size, and colour across the entire application.
//!
//! ## Hierarchy (from UI Design Constitution Addendum)
//!
//! | Function        | Role                               |
//! |-----------------|------------------------------------|
//! | `text`          | Body — primary content             |
//! | `muted`         | Secondary — placeholders, captions |
//! | `mono`          | Data / code values                 |
//! | `section`       | Section heading                    |
//!
//! Do NOT create visual hierarchy by changing font size or weight ad-hoc.
//! Use spacing and structure (separators, indentation) first.

use egui::{Color32, FontId, Rect, RichText, Ui};

// ── Colour tokens ─────────────────────────────────────────────────────────────
// All colours must be defined here — never inline at call sites.
// Theme-aware: picks dark/light variant based on ui.visuals().dark_mode.

fn color_text(dark: bool) -> Color32 {
    if dark { Color32::from_rgb(242, 242, 247) } else { Color32::from_rgb(0, 0, 0) }
}

fn color_muted(dark: bool) -> Color32 {
    if dark { Color32::from_rgb(142, 142, 147) } else { Color32::from_rgb(110, 110, 118) }
}

fn color_mono(dark: bool) -> Color32 {
    if dark { Color32::from_rgb(94, 158, 255) } else { Color32::from_rgb(0, 102, 204) }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Render primary body text.
#[inline]
pub fn text(ui: &mut Ui, s: impl Into<String>) {
    let dark = ui.visuals().dark_mode;
    ui.label(RichText::new(s).color(color_text(dark)));
}

/// Render secondary / muted text (placeholders, captions, key labels).
#[inline]
pub fn muted(ui: &mut Ui, s: impl Into<String>) {
    let dark = ui.visuals().dark_mode;
    ui.label(RichText::new(s).color(color_muted(dark)));
}

/// Render a data value in monospace style (numbers, identifiers, raw values).
#[inline]
pub fn mono(ui: &mut Ui, s: impl Into<String>) {
    let dark = ui.visuals().dark_mode;
    ui.label(RichText::new(s).color(color_mono(dark)).monospace());
}

/// Render a section heading followed by a separator.
///
/// Same size as body text — hierarchy comes from the separator, not font size.
#[inline]
pub fn section(ui: &mut Ui, s: impl Into<String>) {
    let dark = ui.visuals().dark_mode;
    ui.label(RichText::new(s).color(color_text(dark)));
    ui.separator();
}

/// Paint a single glyph optically centred inside `rect`.
///
/// `painter.text(…, Align2::CENTER_CENTER, …)` centres on the font's full
/// line-height box (ascent + descent). For glyphs with no descenders (≡, ◨, ◧,
/// ⊟, …) this pulls the visual glyph above the true midpoint.
///
/// This function uses `Galley::mesh_bounds` — the tight rect around the actual
/// rendered pixels — so the glyph is always visually centred regardless of
/// descender space.
pub fn paint_icon_centered(ui: &Ui, rect: Rect, icon: &str, font_id: FontId, color: Color32) {
    let galley = ui.ctx().fonts_mut(|f| {
        f.layout_no_wrap(icon.to_owned(), font_id.clone(), color)
    });
    // mesh_bounds is the tight pixel bounding box of the rendered glyphs.
    // Paint the galley at a position that aligns mesh_bounds.center() with rect.center().
    let paint_pos = if galley.mesh_bounds != egui::Rect::NOTHING {
        rect.center() - galley.mesh_bounds.center().to_vec2()
    } else {
        // Fallback: use the full galley rect (good enough for ASCII).
        rect.center() - galley.rect.size() * 0.5
    };
    ui.painter().galley(paint_pos, galley, color);
}
