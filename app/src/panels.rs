//! Side-panel UI components: schema inspector, column stats, dataset overview.

use egui::Ui;

use crate::label;
use core::{fmt_large, Dataset};
use profiler::{ColumnProfile, DatasetProfile, SemanticType};
use query::stats::ColumnStats;

/// Map a SemanticType to a Unicode icon that renders in SF Pro / standard fonts.
fn semantic_icon(st: &SemanticType) -> &'static str {
    match st {
        SemanticType::Identifier => "◆",
        SemanticType::Measure    => "●",
        SemanticType::Temporal   => "◷",
        SemanticType::Category   => "▥",
        SemanticType::Geographic => "◎",
        SemanticType::Boolean    => "✓",
        SemanticType::Text       => "≡",
        SemanticType::Unknown    => "?",
    }
}

/// Left panel: dataset schema overview with semantic types and quality summary.
///
/// Quality issues and relationships are rendered inside the same scroll area
/// so they never overflow the panel (constitution §Predictability).
pub fn schema_panel(ui: &mut Ui, dataset: &Dataset, profile: Option<&DatasetProfile>) {
    label::text(ui, &dataset.name);
    label::muted(ui, format!(
        "{} rows · {} columns",
        fmt_large(dataset.row_count),
        dataset.schema.column_count()
    ));

    if let Some(p) = profile {
        let score = p.quality.score;
        let label_str = if score > 0.8 {
            "Excellent"
        } else if score > 0.6 {
            "Good"
        } else {
            "Needs attention"
        };
        label::muted(ui, format!("Quality: {:.0}% · {}", score * 100.0, label_str));
    }

    ui.separator();
    label::section(ui, "Schema");

    egui::ScrollArea::vertical()
        .id_salt("schema_scroll")
        .show(ui, |ui| {
            // ── Column list ───────────────────────────────────────────────
            for (i, col) in dataset.schema.columns.iter().enumerate() {
                let col_profile = profile.and_then(|p| p.column(&col.name));

                ui.horizontal(|ui| {
                    label::muted(ui, format!("{:>3}", i + 1));

                    if let Some(cp) = col_profile {
                        label::muted(ui, semantic_icon(&cp.semantic_type));
                    }

                    label::text(ui, &col.name);

                    if let Some(cp) = col_profile {
                        if cp.null_pct > 0.0 {
                            label::muted(ui, cp.null_bar());
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let type_str = if let Some(cp) = col_profile {
                            format!("{} · {}", col.dtype.label(), cp.semantic_type.label())
                        } else {
                            col.dtype.label().to_string()
                        };
                        label::muted(ui, type_str);
                    });
                });

                if let Some(cp) = col_profile {
                    if let Some(ref dist) = cp.distribution {
                        let spark = dist.sparkline();
                        if !spark.is_empty() {
                            label::mono(ui, &spark);
                        }
                    }
                }

                ui.separator();
            }

            // ── Quality issues ────────────────────────────────────────────
            // Rendered inside the same scroll area (constitution §Predictability).
            if let Some(p) = profile {
                let q = &p.quality;
                if !q.issues.is_empty() {
                    ui.separator();
                    label::section(ui, "Quality");
                    for issue in &q.issues {
                        label::muted(ui, format!("• {}", issue));
                    }
                }

                // ── Relationships ─────────────────────────────────────────
                if !p.relationships.is_empty() {
                    ui.separator();
                    label::section(ui, "Relationships");
                    for rel in &p.relationships {
                        let kind_str = match rel.kind {
                            profiler::RelationshipKind::ForeignKey => "FK",
                            profiler::RelationshipKind::Hierarchy  => "Hierarchy",
                            profiler::RelationshipKind::Repeated   => "Repeated",
                        };
                        label::muted(ui, format!(
                            "{} → {} ({}, {:.0}%)",
                            rel.from_column, rel.to_column, kind_str,
                            rel.confidence * 100.0
                        ));
                    }
                }
            }
        });
}

/// Right panel: column inspector with full profiler data.
pub fn column_inspector(
    ui: &mut Ui,
    col_stats: Option<&ColumnStats>,
    col_profile: Option<&ColumnProfile>,
) {
    label::section(ui, "Column Inspector");

    match (col_stats, col_profile) {
        (None, None) => {
            label::muted(ui, "Click a column header to inspect.");
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

                    stat_row(ui, "Count", &profile.map_or("?".into(), |p| fmt_large(p.count)));
                    if let Some(p) = profile {
                        stat_row(ui, "Nulls", &format!("{} ({:.1}%)", p.null_count, p.null_pct * 100.0));
                        stat_row(ui, "Completeness", &p.null_bar());
                        stat_row(ui, "Unique", &format!("{} ({:.1}%)", p.unique_count, p.unique_pct * 100.0));
                    } else if let Some(s) = stats {
                        stat_row(ui, "Nulls", &s.null_count.to_string());
                        if let Some(u) = s.unique_count {
                            stat_row(ui, "Unique", &u.to_string());
                        }
                    }

                    ui.separator();

                    if let Some(p) = profile {
                        if let Some(min) = p.min    { stat_row(ui, "Min",    &format!("{min:.4}")); }
                        if let Some(max) = p.max    { stat_row(ui, "Max",    &format!("{max:.4}")); }
                        if let Some(mean) = p.mean  { stat_row(ui, "Mean",   &format!("{mean:.4}")); }
                        if let Some(med)  = p.median { stat_row(ui, "Median", &format!("{med:.4}")); }
                        if let Some(std)  = p.stddev { stat_row(ui, "StdDev", &format!("{std:.4}")); }
                    } else if let Some(s) = stats {
                        if let Some(ref min)  = s.min  { stat_row(ui, "Min",  min); }
                        if let Some(ref max)  = s.max  { stat_row(ui, "Max",  max); }
                        if let Some(mean)     = s.mean { stat_row(ui, "Mean", &format!("{mean:.4}")); }
                    }

                    if let Some(p) = profile {
                        if p.p25.is_some() {
                            ui.separator();
                            stat_row(ui, "P1",  &fmt_opt(p.p1));
                            stat_row(ui, "P5",  &fmt_opt(p.p5));
                            stat_row(ui, "P25", &fmt_opt(p.p25));
                            stat_row(ui, "P50", &fmt_opt(p.p50));
                            stat_row(ui, "P75", &fmt_opt(p.p75));
                            stat_row(ui, "P95", &fmt_opt(p.p95));
                            stat_row(ui, "P99", &fmt_opt(p.p99));
                        }
                    }

                    if let Some(p) = profile {
                        if let Some(ref dist) = p.distribution {
                            ui.separator();
                            stat_row(ui, "Distribution", &dist.sparkline());
                        }
                    }

                    if let Some(p) = profile {
                        if let Some(count) = p.outlier_count {
                            if count > 0 {
                                ui.separator();
                                stat_row(ui, "Outliers", &format!(
                                    "{} ({:.1}%)",
                                    count,
                                    count as f64 / p.count as f64 * 100.0
                                ));
                                if let Some(lo) = p.outlier_lower { stat_row(ui, "Lower fence", &format!("{lo:.4}")); }
                                if let Some(hi) = p.outlier_upper { stat_row(ui, "Upper fence", &format!("{hi:.4}")); }
                            }
                        }
                    }

                    if let Some(p) = profile {
                        if !p.top_correlations.is_empty() {
                            ui.separator();
                            label::section(ui, "Top Correlations");
                            for (col, coeff) in &p.top_correlations {
                                stat_row(ui, col, &format!("{coeff:+.3}"));
                            }
                        }
                    }
                });
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Key-value row: key is muted (secondary), value is mono (data).
///
/// Per §Single Primary Typography System: hierarchy through colour role,
/// not font size or weight changes.
fn stat_row(ui: &mut Ui, key: &str, value: &str) {
    ui.horizontal(|ui| {
        label::muted(ui, key);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            label::mono(ui, value);
        });
    });
}

fn fmt_opt(v: Option<f64>) -> String {
    v.map(|x| format!("{x:.4}")).unwrap_or_else(|| "—".to_string())
}

