use tracing_subscriber::EnvFilter;

/// Load the app icon from embedded PNG bytes.
fn load_icon() -> Option<egui::IconData> {
    let png_bytes = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(png_bytes).ok()?.into_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result<()> {
    // Init logging.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("split_office=info".parse().unwrap()))
        .init();

    let icon = load_icon();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Split Office — Research Prototype")
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([800.0, 600.0])
            .with_icon(icon.unwrap_or_default()),
        ..Default::default()
    };

    eframe::run_native(
        "Split Office",
        options,
        Box::new(|cc| {
            // ── SF Pro font family ─────────────────────────────────────────
            if let Some(font_defs) = app::sf_pro_fonts() {
                cc.egui_ctx.set_fonts(font_defs);
            }

            // ── Dark visuals ───────────────────────────────────────────────
            let mut visuals = egui::Visuals::dark();
            visuals.override_text_color = Some(egui::Color32::from_rgb(220, 220, 230));
            visuals.window_fill = egui::Color32::from_rgb(14, 14, 20);
            visuals.panel_fill = egui::Color32::from_rgb(14, 14, 20);
            cc.egui_ctx.set_visuals(visuals);
            Ok(Box::new(app::SplitOfficeApp::new(cc)))
        }),
    )
}
