//! Side-panel UI components: schema inspector, column stats.

use egui::{Color32, RichText, Ui};

use core::Dataset;
use query::stats::ColumnStats;

/// Left panel: dataset schema overview.
pub fn schema_panel(ui: &mut Ui, dataset: &Dataset) {
    ui.heading(RichText::new(&dataset.name).size(14.0).strong());
    ui.label(
        RichText::new(format!(
            "{} rows · {} columns",
            format_large(dataset.row_count),
            dataset.schema.column_count()
        ))
        .size(11.0)
        .color(Color32::from_rgb(130, 130, 160)),
    );

    ui.separator();
    ui.label(RichText::new("Schema").size(11.0).strong());
    ui.add_space(4.0);

    egui::ScrollArea::vertical()
        .id_salt("schema_scroll")
        .show(ui, |ui| {
            for (i, col) in dataset.schema.columns.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{:>3}", i + 1))
                            .size(10.0)
                            .color(Color32::from_rgb(80, 80, 110)),
                    );
                    ui.label(RichText::new(&col.name).size(11.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(col.dtype.label())
                                .size(10.0)
                                .monospace()
                                .color(Color32::from_rgb(100, 160, 220)),
                        );
                    });
                });
            }
        });
}

/// Right panel: column inspector showing statistics for selected column.
pub fn column_inspector(ui: &mut Ui, col_stats: Option<&ColumnStats>) {
    ui.heading(RichText::new("Column Inspector").size(13.0).strong());
    ui.separator();

    match col_stats {
        None => {
            ui.label(
                RichText::new("Click a column header to inspect.")
                    .size(11.0)
                    .color(Color32::from_rgb(100, 100, 130)),
            );
        }
        Some(stats) => {
            stat_row(ui, "Column", &stats.name);
            stat_row(ui, "Nulls", &stats.null_count.to_string());
            if let Some(u) = stats.unique_count {
                stat_row(ui, "Unique", &u.to_string());
            }
            if let Some(ref min) = stats.min {
                stat_row(ui, "Min", min);
            }
            if let Some(ref max) = stats.max {
                stat_row(ui, "Max", max);
            }
            if let Some(mean) = stats.mean {
                stat_row(ui, "Mean", &format!("{mean:.4}"));
            }
        }
    }
}

fn stat_row(ui: &mut Ui, key: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(key)
                .size(11.0)
                .color(Color32::from_rgb(120, 120, 160)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).size(11.0).monospace());
        });
    });
}

fn format_large(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
