//! Side-panel UI components: schema inspector, column stats, dataset overview.

use egui::{Color32, RichText, Ui};

use core::Dataset;
use profiler::{ColumnProfile, DatasetProfile, SemanticType};
use query::stats::ColumnStats;

/// Map a SemanticType to a Unicode icon that renders in SF Pro / standard fonts.
/// Uses Geometric Shapes (U+25A0–U+25FF) and Misc Symbols (U+2700–U+27BF).
fn semantic_icon(st: &SemanticType) -> &'static str {
    match st {
        SemanticType::Identifier => "◆",  // U+25C6 diamond
        SemanticType::Measure => "●",      // U+25CF filled circle
        SemanticType::Temporal => "◷",     // U+25F7 clock-like
        SemanticType::Category => "▥",     // U+25A5 square with vertical fill
        SemanticType::Geographic => "◎",   // U+25CE bullseye
        SemanticType::Boolean => "✓",      // U+2713 checkmark
        SemanticType::Text => "≡",         // U+2261 identical to
        SemanticType::Unknown => "?",
    }
}

/// Left panel: dataset schema overview with semantic types from profiler.
pub fn schema_panel(ui: &mut Ui, dataset: &Dataset, profile: Option<&DatasetProfile>) {
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

    // Quality score if available.
    if let Some(p) = profile {
        let score = p.quality.score;
        let (color, label) = if score > 0.8 {
            (Color32::from_rgb(100, 220, 100), "Excellent")
        } else if score > 0.6 {
            (Color32::from_rgb(220, 200, 80), "Good")
        } else {
            (Color32::from_rgb(220, 120, 80), "Needs attention")
        };
        ui.label(
            RichText::new(format!("Quality: {:.0}% · {}", score * 100.0, label))
                .size(11.0)
                .color(color),
        );
    }

    ui.separator();
    ui.label(RichText::new("Schema").size(11.0).strong());
    ui.add_space(4.0);

    egui::ScrollArea::vertical()
        .id_salt("schema_scroll")
        .show(ui, |ui| {
            for (i, col) in dataset.schema.columns.iter().enumerate() {
                let col_profile = profile.and_then(|p| p.column(&col.name));

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{:>3}", i + 1))
                            .size(10.0)
                            .color(Color32::from_rgb(80, 80, 110)),
                    );

                    // Semantic type icon if available.
                    if let Some(cp) = col_profile {
                        ui.label(
                            RichText::new(semantic_icon(&cp.semantic_type))
                                .size(11.0),
                        );
                    }

                    ui.label(RichText::new(&col.name).size(11.0));

                    // Null bar if available.
                    if let Some(cp) = col_profile {
                        let bar = cp.null_bar();
                        if cp.null_pct > 0.0 {
                            ui.label(
                                RichText::new(&bar)
                                    .size(9.0)
                                    .monospace()
                                    .color(Color32::from_rgb(100, 100, 130)),
                            );
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let type_str = if let Some(cp) = col_profile {
                            format!("{} · {}", col.dtype.label(), cp.semantic_type.label())
                        } else {
                            col.dtype.label().to_string()
                        };
                        ui.label(
                            RichText::new(type_str)
                                .size(10.0)
                                .monospace()
                                .color(Color32::from_rgb(100, 160, 220)),
                        );
                    });
                });

                // Distribution sparkline.
                if let Some(cp) = col_profile {
                    if let Some(ref dist) = cp.distribution {
                        let spark = dist.sparkline();
                        if !spark.is_empty() {
                            ui.label(
                                RichText::new(&spark)
                                    .size(9.0)
                                    .monospace()
                                    .color(Color32::from_rgb(80, 140, 200)),
                            );
                        }
                    }
                }

                ui.separator();
            }
        });
}

/// Right panel: column inspector with full profiler data.
pub fn column_inspector(
    ui: &mut Ui,
    col_stats: Option<&ColumnStats>,
    col_profile: Option<&ColumnProfile>,
) {
    ui.heading(RichText::new("Column Inspector").size(13.0).strong());
    ui.separator();

    match (col_stats, col_profile) {
        (None, None) => {
            ui.label(
                RichText::new("Click a column header to inspect.")
                    .size(11.0)
                    .color(Color32::from_rgb(100, 100, 130)),
            );
        }
        (stats, profile) => {
            let _name = stats
                .map(|s| s.name.as_str())
                .or_else(|| profile.map(|p| p.name.as_str()))
                .unwrap_or("?");
            let semantic = profile.map(|p| &p.semantic_type);

            egui::ScrollArea::vertical()
                .id_salt("inspector_scroll")
                .show(ui, |ui| {
                    if let Some(st) = semantic {
                        stat_row(ui, "Type", &format!("{} {}", semantic_icon(st), st.label()));
                    }

                    if let Some(p) = profile {
                        stat_row(ui, "Physical", &p.physical_type);
                    }

                    ui.separator();

                    // Counts.
                    stat_row(ui, "Count", &profile.map_or("?".into(), |p| format_large(p.count)));
                    if let Some(p) = profile {
                        stat_row(ui, "Nulls", &format!("{} ({:.1}%)", p.null_count, p.null_pct * 100.0));
                        stat_label(ui, "Completeness", &p.null_bar());
                        stat_row(ui, "Unique", &format!("{} ({:.1}%)", p.unique_count, p.unique_pct * 100.0));
                    } else if let Some(s) = stats {
                        stat_row(ui, "Nulls", &s.null_count.to_string());
                        if let Some(u) = s.unique_count {
                            stat_row(ui, "Unique", &u.to_string());
                        }
                    }

                    ui.separator();

                    // Statistics.
                    if let Some(p) = profile {
                        if let Some(min) = p.min {
                            stat_row(ui, "Min", &format!("{min:.4}"));
                        }
                        if let Some(max) = p.max {
                            stat_row(ui, "Max", &format!("{max:.4}"));
                        }
                        if let Some(mean) = p.mean {
                            stat_row(ui, "Mean", &format!("{mean:.4}"));
                        }
                        if let Some(median) = p.median {
                            stat_row(ui, "Median", &format!("{median:.4}"));
                        }
                        if let Some(std) = p.stddev {
                            stat_row(ui, "StdDev", &format!("{std:.4}"));
                        }
                    } else if let Some(s) = stats {
                        if let Some(ref min) = s.min {
                            stat_row(ui, "Min", min);
                        }
                        if let Some(ref max) = s.max {
                            stat_row(ui, "Max", max);
                        }
                        if let Some(mean) = s.mean {
                            stat_row(ui, "Mean", &format!("{mean:.4}"));
                        }
                    }

                    // Quantiles.
                    if let Some(p) = profile {
                        if p.p25.is_some() {
                            ui.separator();
                            stat_row(ui, "P1", &fmt_opt(p.p1));
                            stat_row(ui, "P5", &fmt_opt(p.p5));
                            stat_row(ui, "P25", &fmt_opt(p.p25));
                            stat_row(ui, "P50", &fmt_opt(p.p50));
                            stat_row(ui, "P75", &fmt_opt(p.p75));
                            stat_row(ui, "P95", &fmt_opt(p.p95));
                            stat_row(ui, "P99", &fmt_opt(p.p99));
                        }
                    }

                    // Distribution sparkline.
                    if let Some(p) = profile {
                        if let Some(ref dist) = p.distribution {
                            ui.separator();
                            stat_row(ui, "Distribution", &dist.sparkline());
                        }
                    }

                    // Outliers.
                    if let Some(p) = profile {
                        if let Some(count) = p.outlier_count {
                            if count > 0 {
                                ui.separator();
                                stat_row(ui, "Outliers", &format!("{} ({:.1}%)",
                                    count,
                                    count as f64 / p.count as f64 * 100.0));
                                if let Some(lo) = p.outlier_lower {
                                    stat_row(ui, "Lower fence", &format!("{lo:.4}"));
                                }
                                if let Some(hi) = p.outlier_upper {
                                    stat_row(ui, "Upper fence", &format!("{hi:.4}"));
                                }
                            }
                        }
                    }

                    // Correlations.
                    if let Some(p) = profile {
                        if !p.top_correlations.is_empty() {
                            ui.separator();
                            ui.label(RichText::new("Top Correlations").size(11.0).strong().color(Color32::from_rgb(120, 160, 220)));
                            for (col, coeff) in &p.top_correlations {
                                stat_row(ui, col, &format!("{coeff:+.3}"));
                            }
                        }
                    }
                });
        }
    }
}

/// Show a brief issues summary from the dataset profile.
pub fn quality_panel(ui: &mut Ui, profile: &DatasetProfile) {
    let q = &profile.quality;
    if !q.issues.is_empty() {
        ui.separator();
        ui.label(
            RichText::new(format!("{} issues found", q.issues.len()))
                .size(11.0)
                .color(Color32::from_rgb(220, 160, 80)),
        );
        for issue in &q.issues {
            ui.label(
                RichText::new(format!("• {}", issue))
                    .size(10.0)
                    .color(Color32::from_rgb(180, 140, 100)),
            );
        }
    }

    // Relationships.
    if !profile.relationships.is_empty() {
        ui.separator();
        ui.label(RichText::new("Relationships").size(11.0).strong().color(Color32::from_rgb(120, 160, 220)));
        for rel in &profile.relationships {
            let kind_str = match rel.kind {
                profiler::RelationshipKind::ForeignKey => "FK",
                profiler::RelationshipKind::Hierarchy => "Hierarchy",
                profiler::RelationshipKind::Repeated => "Repeated",
            };
            ui.label(
                RichText::new(format!(
                    "{} → {} ({}, {:.0}%)",
                    rel.from_column,
                    rel.to_column,
                    kind_str,
                    rel.confidence * 100.0
                ))
                .size(10.0)
                .color(Color32::from_rgb(140, 150, 180)),
            );
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

fn stat_label(ui: &mut Ui, key: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(key)
                .size(11.0)
                .color(Color32::from_rgb(120, 120, 160)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).size(10.0).monospace());
        });
    });
}

fn fmt_opt(v: Option<f64>) -> String {
    v.map(|x| format!("{x:.4}")).unwrap_or_else(|| "—".to_string())
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
