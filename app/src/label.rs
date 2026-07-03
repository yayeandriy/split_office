//! Canonical text rendering for Split Office.
//!
//! Every piece of UI text MUST use these functions — never raw `ui.label()`
//! or `ui.heading()`. This guarantees consistent font size, style, and spacing.

use egui::Ui;

/// Render body text. This is the ONLY way to put text on screen.
#[inline]
pub fn text(ui: &mut Ui, s: impl AsRef<str>) {
    ui.label(s.as_ref());
}

/// Render a section header. Same size as body text, just prefixed.
#[inline]
pub fn section(ui: &mut Ui, s: impl AsRef<str>) {
    ui.label(s.as_ref());
    ui.separator();
}
