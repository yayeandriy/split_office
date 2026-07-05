use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Instant;

use arrow::record_batch::RecordBatch;
use egui::Ui;
use serde::{Deserialize, Serialize};
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
use cdm::{IdGenerator, ObjectId};
use document_core::{Block, Document, HeadingBlock, ParagraphBlock, ReferenceBlock, Section, TableBlock};
use workspace_core::{Workspace, WorkspaceObject};

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

/// UI state that survives app restarts.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UiPersist {
    /// Last successfully opened file path.
    last_file: Option<PathBuf>,
    /// Panel widths in logical pixels.
    schema_panel_width: f32,
    workflow_panel_width: f32,
    inspector_panel_width: f32,
    /// Whether each panel is visible.
    #[serde(default = "default_true")]
    show_schema_panel: bool,
    #[serde(default = "default_true")]
    show_workflow_panel: bool,
    #[serde(default = "default_true")]
    show_inspector_panel: bool,
    /// Performance overlay visibility.
    #[serde(default)]
    perf_visible: bool,
    /// Last inspected column name.
    #[serde(default)]
    inspected_col: Option<String>,
    /// Document view visibility.
    #[serde(default)]
    show_document: bool,
    /// Object explorer visibility.
    #[serde(default = "default_true")]
    show_explorer: bool,
}

fn default_true() -> bool { true }

impl Default for UiPersist {
    fn default() -> Self {
        Self {
            last_file: None,
            schema_panel_width: 220.0,
            workflow_panel_width: 180.0,
            inspector_panel_width: 230.0,
            show_schema_panel: true,
            show_workflow_panel: true,
            show_inspector_panel: true,
            perf_visible: false,
            inspected_col: None,
            show_document: false,
            show_explorer: true,
        }
    }
}

const STORAGE_KEY: &str = "split_office_ui_state";

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

    // ── Persisted UI state ───────────────────────────────────────────────
    persist: UiPersist,
    /// Which modifier cards are currently expanded for settings editing.
    expanded_modifiers: HashSet<workflow::NodeId>,

    // ── CDM & Document ──────────────────────────────────────────────────
    /// ID generator for CDM objects.
    id_gen: cdm::IdGenerator,
    /// The workspace document (CDM consumer).
    document: document_core::Document,

    workspace: Workspace,
}

impl SplitOfficeApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = channel();

        // ── Load persisted UI state ──────────────────────────────────────
        let persist = cc
            .storage
            .and_then(|storage| storage.get_string(STORAGE_KEY))
            .and_then(|json| serde_json::from_str::<UiPersist>(&json).ok())
            .unwrap_or_default();

        // Bootstrap an empty workflow graph (no dataset yet).
        // It will be rebuilt when a dataset is loaded.
        let mut app = Self {
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
            inspected_col: persist.inspected_col.clone(),
            perf: PerfOverlay::new(),
            last_frame: Instant::now(),
            loading: false,
            status_message: "Drop a Parquet or CSV file to open it.".to_string(),
            persist,
            expanded_modifiers: HashSet::new(),
            id_gen: IdGenerator::new(),
            document: Self::create_sample_document(),
            workspace: Workspace::new("My Workspace"),

        };

        // ── Apply persisted perf overlay visibility ─────────────────────
        app.perf.visible = app.persist.perf_visible;

        // ── Auto-restore last opened file ───────────────────────────────
        let restore_path = app.persist.last_file.clone();
        if let Some(path) = restore_path {
            if path.exists() {
                app.open_file(path);
            }
        }

        app
    }

    // ── Document factory ─────────────────────────────────────────────────

    fn create_sample_document() -> Document {
        let mut gen = IdGenerator::new();
        let doc_id = gen.next();

        let mut doc = Document::new(doc_id, "Analysis Report");

        // Section 1: Executive Summary
        let sec1_id = gen.next();
        let mut section1 = Section::new(sec1_id, "Executive Summary");
        section1.add_block(Block::Heading(HeadingBlock::new(gen.next(), 1, "Overview")));
        section1.add_block(Block::Paragraph(ParagraphBlock::new(
            gen.next(),
            "This report provides an analysis of the production dataset. Key metrics and trends are summarized below.",
        )));
        section1.add_block(Block::Reference(ReferenceBlock {
            id: gen.next(),
            target: ObjectId::new(1),
            label: "Production Dataset".into(),
        }));
        doc.sections = vec![section1];

        // Section 2: Methodology
        let sec2_id = gen.next();
        let mut section2 = Section::new(sec2_id, "Methodology");
        section2.add_block(Block::Heading(HeadingBlock::new(gen.next(), 2, "Data Processing")));
        section2.add_block(Block::Paragraph(ParagraphBlock::new(
            gen.next(),
            "Data was loaded from Parquet format and processed through a filter → sort pipeline. Statistical profiling was performed to identify patterns.",
        )));
        section2.add_block(Block::Table(TableBlock {
            id: gen.next(),
            source: ObjectId::new(1),
            caption: "Column Statistics".into(),
        }));
        doc.sections.push(section2);

        doc
    }

    // ── File loading ──────────────────────────────────────────────────────────

    fn open_file(&mut self, path: PathBuf) {
        self.loading = true;
        self.status_message = format!("Loading {}…", path.display());
        // Remember the path for next session (written on success in drain_messages).
        self.persist.last_file = Some(path.clone());
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

                    // ── Register in workspace ─────────────────────────────
                    let obj_id = self.workspace.next_id();
                    self.workspace.add_object(WorkspaceObject::Dataset(
                        workspace_core::DatasetEntry {
                            id: obj_id,
                            name: handle.dataset.name.clone(),
                            path: Some(handle.parquet_path.to_string_lossy().into_owned()),
                            row_count: handle.dataset.row_count,
                            col_count: handle.dataset.schema.column_count(),
                        },
                    ));

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


    // ── UI helpers ────────────────────────────────────────────────────────────

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
}


// ── eframe::App impl ─────────────────────────────────────────────────────────

impl eframe::App for SplitOfficeApp {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        // Sync runtime state back into the persisted struct before serializing.
        self.persist.perf_visible = self.perf.visible;
        self.persist.inspected_col = self.inspected_col.clone();
        if let Ok(json) = serde_json::to_string(&self.persist) {
            _storage.set_string(STORAGE_KEY, json);
        }
    }

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
                        ui.label("Panels");
                        if ui.selectable_label(self.persist.show_explorer, "Object Explorer").clicked() {
                            self.persist.show_explorer = !self.persist.show_explorer;
                            ui.close();
                        }
                        if ui.selectable_label(self.persist.show_schema_panel, "Schema Panel").clicked() {
                            self.persist.show_schema_panel = !self.persist.show_schema_panel;
                            ui.close();
                        }
                        if ui.selectable_label(self.persist.show_workflow_panel, "Workflow Panel").clicked() {
                            self.persist.show_workflow_panel = !self.persist.show_workflow_panel;
                            ui.close();
                        }
                        if ui.selectable_label(self.persist.show_inspector_panel, "Column Inspector").clicked() {
                            self.persist.show_inspector_panel = !self.persist.show_inspector_panel;
                            ui.close();
                        }
                        ui.separator();
                        if ui.selectable_label(self.perf.visible, "Performance Overlay").clicked() {
                            self.perf.visible = !self.perf.visible;
                            ui.close();
                        }
                        if ui.selectable_label(self.persist.show_document, "Document View").clicked() {
                            self.persist.show_document = !self.persist.show_document;
                            ui.close();
                        }
                    });
                });
            });

        // Top toolbar.
        egui::Panel::top("toolbar")
            .show(ui, |ui: &mut egui::Ui| {
                self.show_toolbar(ui);
            });

        // Status bar.
        egui::Panel::bottom("status")
            .show(ui, |ui: &mut egui::Ui| {
                self.show_status_bar(ui);
            });

        // Object Explorer — always rendered.
        if self.persist.show_explorer {
            egui::Panel::left("explorer_panel")
                .resizable(true)
                .show(ui, |ui: &mut egui::Ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        crate::explorer::object_explorer(ui, &self.workspace);
                    });
                });
        }

        // Left panel: schema — always rendered so eframe can persist its size.
        let schema_resp = egui::Panel::left("schema_panel")
            .resizable(true)
            .show(ui, |ui: &mut egui::Ui| {
                if self.persist.show_schema_panel {
                    if let Some(handle) = &self.handle {
                        panels::schema_panel(ui, &handle.dataset, self.profile.as_ref());
                    } else {
                        label::muted(ui, "No dataset loaded");
                    }
                }
            });
        self.persist.schema_panel_width = schema_resp.response.rect.width();

        // Second left panel: workflow DAG sidebar — always rendered.
        let wf_resp = egui::Panel::left("workflow_panel")
            .resizable(true)
            .show(ui, |ui: &mut egui::Ui| {
                if self.persist.show_workflow_panel {
                    if self.handle.is_some() {
                        // Collect column names for dropdown selects.
                        let columns: Vec<String> = self.handle.as_ref()
                            .map(|h| h.dataset.schema.column_names().iter().map(|s| s.to_string()).collect())
                            .unwrap_or_default();

                        let wf_actions = workflow_sidebar::workflow_panel(
                            ui,
                            &self.workflow,
                            &columns,
                            &mut self.expanded_modifiers,
                        );

                        // ── Process modifier stack actions ────────────────
                        let has_actions = !wf_actions.is_empty();

                        for node_id in &wf_actions.remove {
                            let _ = self.workflow.remove_node(*node_id);
                        }
                        for node_id in &wf_actions.move_up {
                            let idx = self.workflow.nodes().position(|n| n.id == *node_id);
                            if let Some(i) = idx {
                                let _ = self.workflow.move_modifier_up(i);
                            }
                        }
                        for node_id in &wf_actions.move_down {
                            let idx = self.workflow.nodes().position(|n| n.id == *node_id);
                            if let Some(i) = idx {
                                let _ = self.workflow.move_modifier_down(i);
                            }
                        }
                        for node_id in &wf_actions.toggle {
                            let _ = self.workflow.toggle_node(*node_id);
                        }
                        for node_id in &wf_actions.duplicate {
                            if let Some(node) = self.workflow.node(*node_id) {
                                let kind = node.kind;
                                let payload = node.payload.clone();
                                let idx = self.workflow.nodes().position(|n| n.id == *node_id);
                                if let Some(i) = idx {
                                    let _ = self.workflow.insert_node_at(i + 1, kind, payload);
                                }
                            }
                        }
                        // Settings changes from editable modifier cards.
                        for (node_id, new_payload) in wf_actions.settings_changes {
                            let _ = self.workflow.update_payload(node_id, new_payload);
                        }
                        if let Some(kind) = wf_actions.add_modifier {
                            let payload = match kind {
                                workflow::NodeKind::Filter => workflow::NodePayload::Filter {
                                    expr: core::FilterExpr::None,
                                },
                                workflow::NodeKind::Sort => workflow::NodePayload::Sort {
                                    specs: vec![],
                                },
                                workflow::NodeKind::Aggregate => workflow::NodePayload::Empty,
                                workflow::NodeKind::DerivedColumn => workflow::NodePayload::Empty,
                                _ => workflow::NodePayload::Empty,
                            };
                            let _ = self.workflow.add_node(kind, payload);
                        }

                        // Rebuild query if any actions occurred.
                        if has_actions {
                            self.fetch_page();
                            self.refresh_count();
                        }
                    } else {
                        label::muted(ui, "No workflow");
                    }
                }
            });
        self.persist.workflow_panel_width = wf_resp.response.rect.width();

        // Right panel: column inspector — always rendered.
        let insp_resp = egui::Panel::right("inspector_panel")
            .resizable(true)
            .show(ui, |ui: &mut egui::Ui| {
                if self.persist.show_inspector_panel {
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
                }
            });
        self.persist.inspector_panel_width = insp_resp.response.rect.width();


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

        // ── Document View (bottom panel) ─────────────────────────────────
        if self.persist.show_document {
            egui::Panel::bottom("document_panel")
                .resizable(true)
                .show(ui, |ui: &mut egui::Ui| {
                    ui.vertical_centered(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("📄 Document").size(13.0).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("×").on_hover_text("Close document view").clicked() {
                                    self.persist.show_document = false;
                                }
                            });
                        });
                    });
                    ui.separator();
                    document_view::render_document(ui, &self.document);
                });
        }

        // Performance overlay (always on top).
        self.perf.show(ui.ctx());

        // ── Persist panel widths into egui Memory (auto-restored by eframe) ──
        let ctx = ui.ctx();
        ctx.data_mut(|d| {
            d.insert_persisted(egui::Id::new("panel_schema_width"), self.persist.schema_panel_width);
            d.insert_persisted(egui::Id::new("panel_workflow_width"), self.persist.workflow_panel_width);
            d.insert_persisted(egui::Id::new("panel_inspector_width"), self.persist.inspector_panel_width);
        });

        // Drive continuous repainting for 60 FPS.
        ctx.request_repaint();
    }
}