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

use egui::{Color32, RichText, Ui};

// ── Colour tokens ─────────────────────────────────────────────────────────────
// All colours must be defined here — never inline at call sites.

/// Primary body text colour.
const COLOR_TEXT: Color32 = Color32::from_rgb(220, 220, 230);

/// Muted / secondary text colour (placeholders, captions, keys in key-value pairs).
const COLOR_MUTED: Color32 = Color32::from_rgb(120, 120, 145);

/// Monospace / data value colour (numeric results, identifiers, raw values).
const COLOR_MONO: Color32 = Color32::from_rgb(160, 210, 255);

// ── Public API ────────────────────────────────────────────────────────────────

/// Render primary body text.
#[inline]
pub fn text(ui: &mut Ui, s: impl Into<String>) {
    ui.label(RichText::new(s).color(COLOR_TEXT));
}

/// Render secondary / muted text (placeholders, captions, key labels).
#[inline]
pub fn muted(ui: &mut Ui, s: impl Into<String>) {
    ui.label(RichText::new(s).color(COLOR_MUTED));
}

/// Render a data value in monospace style (numbers, identifiers, raw values).
#[inline]
pub fn mono(ui: &mut Ui, s: impl Into<String>) {
    ui.label(RichText::new(s).color(COLOR_MONO).monospace());
}

/// Render a section heading followed by a separator.
///
/// Same size as body text — hierarchy comes from the separator, not font size.
#[inline]
pub fn section(ui: &mut Ui, s: impl Into<String>) {
    ui.label(RichText::new(s).color(COLOR_TEXT));
    ui.separator();
}
