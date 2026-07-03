use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Instant;

use arrow::record_batch::RecordBatch;
use egui::{Color32, Context, RichText, Ui};
use tracing::{error, info};

use core::{FilterExpr, SortSpec, Viewport};
use grid::{GridAction, GridRenderer, GridState};
use profiler::DatasetProfile;
use query::stats::DatasetStats;
use query::{QueryEngine, QueryParams};
use storage::DatasetHandle;

use crate::panels;
use crate::perf::PerfOverlay;

// ── Background messages ───────────────────────────────────────────────────────

enum BgMessage {
    DatasetLoaded(Box<DatasetHandle>),
    PageReady {
        batch: RecordBatch,
        query_ms: f64,
    },
    RowCount(usize),
    StatsReady(DatasetStats),
    ProfileReady(DatasetProfile),
    Error(String),
}

// ── Application state ─────────────────────────────────────────────────────────

pub struct SplitOfficeApp {
    rx: Receiver<BgMessage>,
    tx: Sender<BgMessage>,

    handle: Option<DatasetHandle>,
    engine: Option<Arc<QueryEngine>>,
    dataset_stats: Option<DatasetStats>,
    profile: Option<DatasetProfile>,

    filter_expr: FilterExpr,
    sort: Vec<SortSpec>,
    viewport: Viewport,
    total_rows: usize,

    grid_state: GridState,
    current_batch: Option<RecordBatch>,

    inspected_col: Option<String>,

    perf: PerfOverlay,
    last_frame: Instant,
    loading: bool,
    status_message: String,
}

impl SplitOfficeApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = channel();
        Self {
            rx,
            tx,
            handle: None,
            engine: None,
            dataset_stats: None,
            profile: None,
            filter_expr: FilterExpr::None,
            sort: Vec::new(),
            viewport: Viewport::default(),
            total_rows: 0,
            grid_state: GridState::new(),
            current_batch: None,
            inspected_col: None,
            perf: PerfOverlay::default(),
            last_frame: Instant::now(),
            loading: false,
            status_message: "Drop a Parquet or CSV file to open it.".to_string(),
        }
    }

    // ── File loading ──────────────────────────────────────────────────────────

    fn open_file(&mut self, path: PathBuf) {
        self.loading = true;
        self.status_message = format!("Loading {}…", path.display());
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let result = if ext.eq_ignore_ascii_case("csv") {
                storage::load_csv(&path)
            } else {
                storage::load_parquet(&path)
            };
            match result {
                Ok(h) => { let _ = tx.send(BgMessage::DatasetLoaded(Box::new(h))); }
                Err(e) => { let _ = tx.send(BgMessage::Error(e.to_string())); }
            }
        });
    }

    // ── Query dispatch ────────────────────────────────────────────────────────

    fn fetch_page(&self) {
        let engine = match &self.engine {
            Some(e) => Arc::clone(e),
            None => return,
        };
        let params = QueryParams {
            filter: self.filter_expr.clone(),
            sort: self.sort.clone(),
            viewport: self.viewport,
        };
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let t0 = Instant::now();
            match engine.fetch_page(&params) {
                Ok(batch) => {
                    let query_ms = t0.elapsed().as_secs_f64() * 1000.0;
                    let _ = tx.send(BgMessage::PageReady { batch, query_ms });
                }
                Err(e) => { let _ = tx.send(BgMessage::Error(e.to_string())); }
            }
        });
    }

    fn refresh_count(&self) {
        let engine = match &self.engine {
            Some(e) => Arc::clone(e),
            None => return,
        };
        let filter = self.filter_expr.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || match engine.count_rows(&filter) {
            Ok(n) => { let _ = tx.send(BgMessage::RowCount(n)); }
            Err(e) => { let _ = tx.send(BgMessage::Error(e.to_string())); }
        });
    }

    fn fetch_stats(&self) {
        let handle = match &self.handle { Some(h) => h.clone(), None => return };
        let tx = self.tx.clone();
        std::thread::spawn(move || match query::stats::compute_stats(&handle) {
            Ok(s) => { let _ = tx.send(BgMessage::StatsReady(s)); }
            Err(e) => { let _ = tx.send(BgMessage::Error(e.to_string())); }
        });
    }

    fn start_profiling(&self) {
        let handle = match &self.handle { Some(h) => h.clone(), None => return };
        let tx = self.tx.clone();
        let dataset_name = handle.dataset.name.clone();
        std::thread::spawn(move || {
            let path = handle
                .parquet_path
                .to_str()
                .unwrap_or("unknown");
            match polars::prelude::LazyFrame::scan_parquet(
                path,
                polars::prelude::ScanArgsParquet::default(),
            ) {
                Ok(lf) => match lf.collect() {
                    Ok(df) => match profiler::profile_dataframe(&df, &dataset_name) {
                        Ok(profile) => {
                            let _ = tx.send(BgMessage::ProfileReady(profile));
                        }
                        Err(e) => {
                            let _ = tx.send(BgMessage::Error(format!("Profiling: {e}")));
                        }
                    },
                    Err(e) => {
                        let _ = tx.send(BgMessage::Error(format!("Polars collect: {e}")));
                    }
                },
                Err(e) => {
                    let _ = tx.send(BgMessage::Error(format!("Polars scan: {e}")));
                }
            }
        });
    }

    // ── Message drain ─────────────────────────────────────────────────────────

    fn drain_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                BgMessage::DatasetLoaded(handle) => {
                    info!(name = %handle.dataset.name, rows = handle.dataset.row_count, "dataset loaded");
                    self.loading = false;
                    self.total_rows = handle.dataset.row_count;
                    self.viewport = Viewport::new(0, 50);
                    self.grid_state = GridState::new();
                    self.sort.clear();
                    self.filter_expr = FilterExpr::None;
                    self.current_batch = None;
                    self.dataset_stats = None;
                    self.profile = None;

                    match QueryEngine::open(&handle) {
                        Ok(engine) => {
                            self.status_message = format!(
                                "Opened: {} · {} rows · {} cols",
                                handle.dataset.name,
                                fmt_large(handle.dataset.row_count),
                                handle.dataset.schema.column_count()
                            );
                            self.engine = Some(Arc::new(engine));
                            self.handle = Some(*handle);
                            self.fetch_page();
                            self.fetch_stats();
                            self.start_profiling();
                        }
                        Err(e) => {
                            self.status_message = format!("Engine error: {e}");
                            error!("{e}");
                        }
                    }
                }

                BgMessage::PageReady { batch, query_ms } => {
                    self.current_batch = Some(batch);
                    self.perf.query_latency_ms = query_ms;
                }

                BgMessage::RowCount(n) => {
                    self.total_rows = n;
                }

                BgMessage::StatsReady(stats) => {
                    self.dataset_stats = Some(stats);
                }

                BgMessage::ProfileReady(profile) => {
                    info!(cols = profile.column_count, "profile ready");
                    self.profile = Some(profile);
                }

                BgMessage::Error(e) => {
                    self.loading = false;
                    self.status_message = format!("Error: {e}");
                    error!("{e}");
                }
            }
        }
    }

    // ── Grid action handling ──────────────────────────────────────────────────

    fn handle_grid_actions(&mut self, actions: Vec<GridAction>) {
        for action in actions {
            match action {
                GridAction::SortRequested { column, shift_held } => {
                    self.grid_state.toggle_sort(&column, shift_held);
                    self.sort = self.grid_state.sort_specs.clone();
                    self.viewport.first_row = 0;
                    self.grid_state.scroll_y = 0.0;
                    self.inspected_col = Some(column);
                    self.fetch_page();
                }
                GridAction::SortCleared => {
                    self.grid_state.clear_sorts();
                    self.sort.clear();
                    self.viewport.first_row = 0;
                    self.grid_state.scroll_y = 0.0;
                    self.fetch_page();
                }
                GridAction::ScrollChanged { first_row } => {
                    self.viewport.first_row = first_row;
                    self.viewport = self.viewport.clamped(self.total_rows);
                    self.fetch_page();
                }
                GridAction::FilterColumnChanged { column: _, text: _ } => {
                    self.apply_filter();
                }
                GridAction::SelectionChanged(_) => {}
            }
        }
    }

    fn apply_filter(&mut self) {
        let filters: Vec<(String, String)> = self
            .grid_state
            .column_filters
            .iter()
            .map(|(col, text)| (col.clone(), text.clone()))
            .collect();

        if filters.is_empty() {
            self.filter_expr = FilterExpr::None;
        } else {
            // AND together all column filters, each doing a Contains on its column.
            let mut exprs: Vec<FilterExpr> = filters
                .into_iter()
                .map(|(col, text)| FilterExpr::Contains {
                    column: col,
                    pattern: text,
                })
                .collect();

            let mut expr = exprs.remove(0);
            for next in exprs {
                expr = FilterExpr::And(Box::new(expr), Box::new(next));
            }
            self.filter_expr = expr;
        }

        self.viewport.first_row = 0;
        self.grid_state.scroll_y = 0.0;
        self.refresh_count();
        self.fetch_page();
    }

    // ── UI helpers ────────────────────────────────────────────────────────────

    fn show_toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("📂  Open File").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Data Files", &["parquet", "csv"])
                    .pick_file()
                {
                    self.open_file(path);
                }
            }

            ui.separator();

            if !self.grid_state.sort_specs.is_empty() {
                if ui.button("🔄  Clear Sorts").clicked() {
                    self.grid_state.clear_sorts();
                    self.sort.clear();
                    self.viewport.first_row = 0;
                    self.grid_state.scroll_y = 0.0;
                    self.fetch_page();
                }
                ui.separator();
            }

            if self.grid_state.has_active_filters() {
                if ui.button("✕  Clear Filters").clicked() {
                    self.grid_state.column_filters.clear();
                    self.apply_filter();
                }
                ui.separator();
            }

            if let Some(h) = &self.handle {
                ui.label(
                    RichText::new(format!(
                        "{} · {} rows · {} cols",
                        h.dataset.name,
                        fmt_large(h.dataset.row_count),
                        h.dataset.schema.column_count()
                    ))
                    .size(12.0)
                    .color(Color32::from_rgb(160, 160, 190)),
                );
            }

            if self.loading {
                ui.spinner();
            }
        });
    }

    fn show_status_bar(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(&self.status_message)
                    .size(11.0)
                    .color(Color32::from_rgb(140, 140, 170)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.grid_state.has_active_filters() {
                    let count = self.grid_state.column_filters.len();
                    ui.label(
                        RichText::new(format!("Filtering {} col{} · Showing {} rows", count, if count > 1 { "s" } else { "" }, fmt_large(self.total_rows)))
                            .size(11.0)
                            .color(Color32::from_rgb(100, 180, 100)),
                    );
                    ui.separator();
                }
                if !self.grid_state.sort_specs.is_empty() {
                    let labels: Vec<String> = self
                        .grid_state
                        .sort_specs
                        .iter()
                        .map(|s| format!("{} {}", s.column, s.direction.arrow_label()))
                        .collect();
                    ui.label(
                        RichText::new(format!("Sorted: {}", labels.join(", ")))
                            .size(11.0)
                            .color(Color32::from_rgb(100, 160, 220)),
                    );
                }
            });
        });
    }
}

// ── eframe::App impl ─────────────────────────────────────────────────────────

impl eframe::App for SplitOfficeApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Perf tracking.
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f64().max(0.0001);
        self.last_frame = now;
        self.perf.update(dt);
        self.perf.visible_rows = self.viewport.visible_rows;
        self.perf.total_rows = self.total_rows;

        // Drain background messages.
        self.drain_messages();

        // File drop.
        ctx.input(|i| {
            if let Some(file) = i.raw.dropped_files.first() {
                if let Some(path) = &file.path {
                    self.open_file(path.clone());
                }
            }
        });

        // Top toolbar.
        egui::TopBottomPanel::top("toolbar")
            .min_height(36.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                self.show_toolbar(ui);
            });

        // Status bar.
        egui::TopBottomPanel::bottom("status")
            .min_height(24.0)
            .show(ctx, |ui| {
                ui.add_space(2.0);
                self.show_status_bar(ui);
            });

        // Left panel: schema.
        egui::SidePanel::left("schema_panel")
            .resizable(true)
            .min_width(160.0)
            .default_width(220.0)
            .show(ctx, |ui| {
                if let Some(handle) = &self.handle {
                    panels::schema_panel(ui, &handle.dataset, self.profile.as_ref());
                    if let Some(ref profile) = self.profile {
                        panels::quality_panel(ui, profile);
                    }
                } else {
                    ui.label(
                        RichText::new("No dataset loaded")
                            .color(Color32::from_rgb(100, 100, 130))
                            .size(12.0),
                    );
                }
            });

        // Right panel: column inspector.
        egui::SidePanel::right("inspector_panel")
            .resizable(true)
            .min_width(180.0)
            .default_width(230.0)
            .show(ctx, |ui| {
                let col_stats = self.inspected_col.as_ref().and_then(|name| {
                    self.dataset_stats
                        .as_ref()?
                        .columns
                        .iter()
                        .find(|c| &c.name == name)
                });
                let col_profile = self.inspected_col.as_ref().and_then(|name| {
                    self.profile.as_ref()?.column(name)
                });
                panels::column_inspector(ui, col_stats, col_profile);
            });

        // Central panel: grid.
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(handle) = &self.handle {
                if let Some(batch) = self.current_batch.clone() {
                    let h = ui.available_height();
                    self.viewport.visible_rows = self.grid_state.rows_in_viewport(h);

                    let actions = GridRenderer::show(
                        ui,
                        &handle.dataset,
                        &batch,
                        &mut self.grid_state,
                        self.total_rows,
                    );
                    self.handle_grid_actions(actions);
                } else if self.loading {
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(40.0);
                            ui.spinner();
                            ui.add_space(8.0);
                            ui.label(RichText::new("Loading dataset…").size(14.0));
                        });
                    });
                }
            } else {
                // Drop-zone landing.
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(60.0);
                        ui.label(
                            RichText::new("Split Office")
                                .size(36.0)
                                .strong()
                                .color(Color32::from_rgb(180, 180, 220)),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Research Prototype · Phase 0")
                                .size(13.0)
                                .color(Color32::from_rgb(100, 100, 140)),
                        );
                        ui.add_space(50.0);
                        ui.label(
                            RichText::new("📂  Drop a Parquet or CSV file here")
                                .size(18.0)
                                .color(Color32::from_rgb(140, 140, 180)),
                        );
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new("or click Open File above")
                                .size(12.0)
                                .color(Color32::from_rgb(90, 90, 130)),
                        );
                    });
                });
            }
        });

        // Performance overlay (always on top).
        self.perf.show(ctx);

        // Drive continuous repainting for 60 FPS.
        ctx.request_repaint();
    }
}

fn fmt_large(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
