//! SF Pro font loader for egui.
//!
//! Replaces egui's built-in font with the system-installed Apple SF Pro family.
//! Falls back gracefully to egui's bundled font if the system fonts are not found.

use egui::{FontData, FontDefinitions, FontFamily};

/// Font file paths (macOS system install by Xcode / SF Pro download).
const SF_PRO_TEXT_REGULAR: &str = "/Library/Fonts/SF-Pro-Text-Regular.otf";
const SF_PRO_TEXT_MEDIUM: &str = "/Library/Fonts/SF-Pro-Text-Medium.otf";
const SF_PRO_TEXT_SEMIBOLD: &str = "/Library/Fonts/SF-Pro-Text-Semibold.otf";
const SF_PRO_TEXT_BOLD: &str = "/Library/Fonts/SF-Pro-Text-Bold.otf";
const SF_PRO_TEXT_ITALIC: &str = "/Library/Fonts/SF-Pro-Text-RegularItalic.otf";
const SF_PRO_DISPLAY_REGULAR: &str = "/Library/Fonts/SF-Pro-Display-Regular.otf";
const SF_PRO_DISPLAY_SEMIBOLD: &str = "/Library/Fonts/SF-Pro-Display-Semibold.otf";
const SF_PRO_DISPLAY_BOLD: &str = "/Library/Fonts/SF-Pro-Display-Bold.otf";
const SFNS_MONO: &str = "/System/Library/Fonts/SFNSMono.ttf";
const SFNS_MONO_ITALIC: &str = "/System/Library/Fonts/SFNSMonoItalic.ttf";

/// Build egui `FontDefinitions` using SF Pro.
///
/// If any font file is missing (non-macOS or SF Pro not installed),
/// returns `None` and egui will use its built-in font.
pub fn sf_pro_fonts() -> Option<FontDefinitions> {
    // Quick availability check — if the primary file is missing, bail out.
    if !std::path::Path::new(SF_PRO_TEXT_REGULAR).exists() {
        tracing::warn!("SF Pro fonts not found — using egui default font");
        return None;
    }

    let mut fonts = FontDefinitions::empty();

    // ── Load font files ───────────────────────────────────────────────────
    let pairs: &[(&str, &str)] = &[
        ("SFProText-Regular",   SF_PRO_TEXT_REGULAR),
        ("SFProText-Medium",    SF_PRO_TEXT_MEDIUM),
        ("SFProText-Semibold",  SF_PRO_TEXT_SEMIBOLD),
        ("SFProText-Bold",      SF_PRO_TEXT_BOLD),
        ("SFProText-Italic",    SF_PRO_TEXT_ITALIC),
        ("SFProDisplay-Regular",   SF_PRO_DISPLAY_REGULAR),
        ("SFProDisplay-Semibold",  SF_PRO_DISPLAY_SEMIBOLD),
        ("SFProDisplay-Bold",      SF_PRO_DISPLAY_BOLD),
        ("SFNSMono",            SFNS_MONO),
        ("SFNSMono-Italic",     SFNS_MONO_ITALIC),
    ];

    for (name, path) in pairs {
        match std::fs::read(path) {
            Ok(bytes) => {
                fonts
                    .font_data
                    .insert(name.to_string(), std::sync::Arc::new(FontData::from_owned(bytes)));
                tracing::debug!(font = name, "loaded SF font");
            }
            Err(e) => {
                tracing::warn!(font = name, path, error = %e, "could not load font — skipping");
            }
        }
    }

    // ── Wire font families ────────────────────────────────────────────────
    //
    // egui looks up fonts in the order listed in each family vec.
    // Priority: Text → Display → fallback to egui's built-in.

    // Proportional (used for all UI text).
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .extend([
            "SFProText-Regular".to_string(),
            "SFProText-Medium".to_string(),
            "SFProText-Semibold".to_string(),
            "SFProText-Bold".to_string(),
            "SFProDisplay-Regular".to_string(),
            "SFProDisplay-Semibold".to_string(),
            "SFProDisplay-Bold".to_string(),
        ]);

    // Monospace (code / numeric displays).
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .extend([
            "SFNSMono".to_string(),
            "SFNSMono-Italic".to_string(),
            // Fall through to proportional if mono not available.
            "SFProText-Regular".to_string(),
        ]);

    tracing::info!("SF Pro font family loaded");
    Some(fonts)
}
