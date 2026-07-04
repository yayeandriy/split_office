use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Instant;

use arrow::record_batch::RecordBatch;
use egui::Ui;
use tracing::{error, info};

use core::{fmt_large, FilterExpr, Viewport};
use grid::{GridAction, GridRenderer, GridState};
use profiler::DatasetProfile;
use query::stats::DatasetStats;
use query::{QueryEngine, QueryParams};
use storage::DatasetHandle;
use workflow::{
    build_filter, build_query_params, LinearWorkflowIds,
    NodePayload, WorkflowBuilder, WorkflowGraph,
};

use crate::label;
use crate::panels;
use crate::perf::PerfOverlay;
use crate::workflow_sidebar;

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

    // ── DAG workflow (spec §DAG-First Architecture) ────────────────────────
    //
    // All filter and sort state lives in the graph. The app never holds
    // raw `FilterExpr` or `Vec<SortSpec>` directly.
    workflow: WorkflowGraph,
    workflow_ids: Option<LinearWorkflowIds>,

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
        // Bootstrap an empty workflow graph (no dataset yet).
        // It will be rebuilt when a dataset is loaded.
        Self {
            rx,
            tx,
            handle: None,
            engine: None,
            dataset_stats: None,
            profile: None,
            workflow: WorkflowGraph::new(),
            workflow_ids: None,
            viewport: Viewport::default(),
            total_rows: 0,
            grid_state: GridState::new(),
            current_batch: None,
            inspected_col: None,
            perf: PerfOverlay::new(),
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

        // Build QueryParams from the workflow DAG.
        let params = if let Some(_ids) = &self.workflow_ids {
            let plan = self.workflow.execution_plan().unwrap_or_else(|_| {
                workflow::ExecutionPlan { steps: vec![] }
            });
            let wp = build_query_params(&self.workflow, &plan, self.viewport);
            QueryParams {
                filter: wp.filter,
                sort: wp.sort,
                viewport: wp.viewport,
            }
        } else {
            QueryParams::new(self.viewport)
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
        // Extract the current filter from the workflow graph.
        let filter = if let Some(_ids) = &self.workflow_ids {
            let plan = self.workflow.execution_plan().unwrap_or_else(|_| {
                workflow::ExecutionPlan { steps: vec![] }
            });
            build_filter(&self.workflow, &plan)
        } else {
            FilterExpr::None
        };
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
            // ── Profiler context owns all Polars IO (constitution §Bounded Contexts) ──
            let path = handle.parquet_path.to_str().unwrap_or("unknown").to_string();
            match profiler::profile_from_parquet(&path, &dataset_name) {
                Ok(profile) => {
                    let _ = tx.send(BgMessage::ProfileReady(profile));
                }
                Err(e) => {
                    let _ = tx.send(BgMessage::Error(e.to_string()));
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
                    self.current_batch = None;
                    self.dataset_stats = None;
                    self.profile = None;

                    // ── Bootstrap a fresh workflow graph for the new dataset ──────
                    //
                    // The graph starts as Dataset → Filter → Sort with all
                    // parameters at their defaults (no filter, no sort).
                    // The app layer only ever mutates node payloads via
                    // `workflow.update_payload(id, ...)` — never raw fields.
                    let (graph, ids) = WorkflowBuilder::new(handle.dataset.id)
                        .filter(FilterExpr::None)
                        .sort(vec![])
                        .build();
                    self.workflow = graph;
                    self.workflow_ids = Some(ids);

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

                    // Update the Sort node in the workflow graph.
                    if let Some(ids) = &self.workflow_ids {
                        let specs = self.grid_state.sort_specs.clone();
                        let sort_id = ids.sort;
                        let _ = self.workflow.update_payload(
                            sort_id,
                            NodePayload::Sort { specs },
                        );
                        // Mark sort node executing (fetch is in flight).
                        self.workflow.mark_executing(sort_id);
                    }

                    self.viewport.first_row = 0;
                    self.grid_state.scroll_y = 0.0;
                    self.inspected_col = Some(column);
                    self.fetch_page();
                }
                GridAction::SortCleared => {
                    self.grid_state.clear_sorts();
                    if let Some(ids) = &self.workflow_ids {
                        let sort_id = ids.sort;
                        let _ = self.workflow.update_payload(
                            sort_id,
                            NodePayload::Sort { specs: vec![] },
                        );
                    }
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

        let expr = if filters.is_empty() {
            FilterExpr::None
        } else {
            // AND together all column filters.
            let mut exprs: Vec<FilterExpr> = filters
                .into_iter()
                .map(|(col, text)| FilterExpr::Contains { column: col, pattern: text })
                .collect();
            let mut expr = exprs.remove(0);
            for next in exprs {
                expr = FilterExpr::And(Box::new(expr), Box::new(next));
            }
            expr
        };

        // Update the Filter node in the workflow graph.
        if let Some(ids) = &self.workflow_ids {
            let filter_id = ids.filter;
            let _ = self.workflow.update_payload(
                filter_id,
                NodePayload::Filter { expr: expr.clone() },
            );
            self.workflow.mark_executing(filter_id);
        }

        self.viewport.first_row = 0;
        self.grid_state.scroll_y = 0.0;
        self.refresh_count();
        self.fetch_page();
    }

    // ── UI helpers ────────────────────────────────────────────────────────────

    fn show_toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("▶  Open File").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Data Files", &["parquet", "csv"])
                    .pick_file()
                {
                    self.open_file(path);
                }
            }

            ui.separator();

            if !self.grid_state.sort_specs.is_empty() {
                if ui.button("↻  Clear Sorts").clicked() {
                    self.grid_state.clear_sorts();
                    // Clear the Sort node in the workflow graph.
                    if let Some(ids) = &self.workflow_ids {
                        let sort_id = ids.sort;
                        let _ = self.workflow.update_payload(
                            sort_id,
                            NodePayload::Sort { specs: vec![] },
                        );
                    }
                    self.viewport.first_row = 0;
                    self.grid_state.scroll_y = 0.0;
                    self.fetch_page();
                }
                ui.separator();
            }

            if self.grid_state.has_active_filters() {
                if ui.button("×  Clear Filters").clicked() {
                    self.grid_state.column_filters.clear();
                    // Clear the Filter node in the workflow graph.
                    if let Some(ids) = &self.workflow_ids {
                        let filter_id = ids.filter;
                        let _ = self.workflow.update_payload(
                            filter_id,
                            NodePayload::Filter { expr: FilterExpr::None },
                        );
                    }
                    self.apply_filter();
                }
                ui.separator();
            }

            if let Some(h) = &self.handle {
                label::text(ui, format!(
                    "{} · {} rows · {} cols",
                    h.dataset.name,
                    fmt_large(h.dataset.row_count),
                    h.dataset.schema.column_count()
                ));
            }

            if self.loading {
                ui.spinner();
            }
        });
    }

    fn show_status_bar(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            label::text(ui, &self.status_message);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.grid_state.has_active_filters() {
                    let count = self.grid_state.column_filters.len();
                    label::text(ui, format!(
                        "Filtering {} col{} · Showing {} rows",
                        count,
                        if count > 1 { "s" } else { "" },
                        fmt_large(self.total_rows)
                    ));
                    ui.separator();
                }
                if !self.grid_state.sort_specs.is_empty() {
                    let labels: Vec<String> = self
                        .grid_state
                        .sort_specs
                        .iter()
                        .map(|s| format!("{} {}", s.column, s.direction.arrow_label()))
                        .collect();
                    label::text(ui, format!("Sorted: {}", labels.join(", ")));
                }
            });
        });
    }
}

// ── eframe::App impl ─────────────────────────────────────────────────────────

impl eframe::App for SplitOfficeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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
        ui.input(|i| {
            if let Some(file) = i.raw.dropped_files.first() {
                if let Some(path) = &file.path {
                    self.open_file(path.clone());
                }
            }
        });

        // Menu bar.
        egui::Panel::top("menu_bar")
            .show(ui, |ui: &mut egui::Ui| {
                ui.horizontal(|ui: &mut egui::Ui| {
                    ui.menu_button("View", |ui: &mut egui::Ui| {
                        if ui
                            .selectable_label(self.perf.visible, "Performance Overlay")
                            .clicked()
                        {
                            self.perf.visible = !self.perf.visible;
                            ui.close();
                        }
                    });
                });
            });

        // Top toolbar.
        egui::Panel::top("toolbar")
            .show(ui, |ui: &mut egui::Ui| {
                ui.add_space(4.0);
                self.show_toolbar(ui);
            });

        // Status bar.
        egui::Panel::bottom("status")
            .show(ui, |ui: &mut egui::Ui| {
                ui.add_space(2.0);
                self.show_status_bar(ui);
            });

        // Left panel: schema.
        egui::Panel::left("schema_panel")
            .resizable(true)
            .show(ui, |ui: &mut egui::Ui| {
                if let Some(handle) = &self.handle {
                    panels::schema_panel(ui, &handle.dataset, self.profile.as_ref());
                } else {
                    label::muted(ui, "No dataset loaded");
                }
            });

        // Second left panel: workflow DAG sidebar.
        egui::Panel::left("workflow_panel")
            .resizable(true)
            .show(ui, |ui: &mut egui::Ui| {
                if self.handle.is_some() {
                    let to_remove = workflow_sidebar::workflow_panel(ui, &self.workflow);
                    for node_id in to_remove {
                        let _ = self.workflow.remove_node(node_id);
                        self.fetch_page();
                        self.refresh_count();
                    }
                } else {
                    label::muted(ui, "No workflow");
                }
            });

        // Right panel: column inspector.
        egui::Panel::right("inspector_panel")
            .resizable(true)
            .show(ui, |ui: &mut egui::Ui| {
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
        egui::CentralPanel::default().show(ui, |ui: &mut egui::Ui| {
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
                            label::text(ui, "Loading dataset…");
                        });
                    });
                }
            } else {
                // Drop-zone landing.
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(60.0);
                        label::text(ui, "Split Office");
                        label::muted(ui, "Research Prototype");
                        ui.add_space(20.0);
                        label::text(ui, "▶  Drop a Parquet or CSV file here");
                        label::muted(ui, "or click Open File above");
                    });
                });
            }
        });

        // Performance overlay (always on top).
        self.perf.show(ui.ctx());

        // Drive continuous repainting for 60 FPS.
        ui.ctx().request_repaint();
    }
}


