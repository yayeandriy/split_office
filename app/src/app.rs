use std::collections::{HashMap, HashSet};
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
use split_view::{
    LayoutManager, LayoutState, ViewId,
    renderer::{TabAction, ViewContext},
};

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
    /// Object explorer visibility.
    #[serde(default = "default_true")]
    show_explorer: bool,
    /// UI theme: "dark" or "light".
    #[serde(default = "default_dark")]
    theme: String,
    /// Serialised layout state for the split view framework.
    #[serde(default)]
    layout_json: Option<String>,
}

fn default_true() -> bool { true }
fn default_dark() -> String { "dark".into() }

impl Default for UiPersist {
    fn default() -> Self {
        Self {
            last_file: None,
            show_schema_panel: true,
            show_workflow_panel: true,
            show_inspector_panel: true,
            perf_visible: false,
            inspected_col: None,
            show_explorer: true,
            theme: "dark".into(),
            layout_json: None,
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

    /// Per-tab grid state: each spreadsheet tab remembers its own scroll
    /// position and selection independently (only underlying data is shared).
    tab_grid_states: HashMap<ViewId, GridState>,

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

    // ── Split View Framework (spec §Split View Framework) ────────────────
    layout_mgr: LayoutManager,
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

        // Restore or build the split-view layout.
        let layout_mgr = persist
            .layout_json
            .as_deref()
            .and_then(|json| LayoutState::from_json(json).ok())
            .map(|s| s.restore())
            .unwrap_or_else(LayoutManager::default_layout);

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
            tab_grid_states: HashMap::new(),
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
            layout_mgr,
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

    // ── Tab action handling ──────────────────────────────────────────────────

    fn handle_tab_actions(&mut self, actions: Vec<TabAction>) {
        for action in actions {
            match action {
                TabAction::CloseTab { group_id, tab_index } => {
                    if self.layout_mgr.close_tab_in_group(group_id, tab_index) {
                        self.layout_mgr.focus.validate(&self.layout_mgr.root);
                    }
                }
                TabAction::NewTab { group_id } => {
                    self.layout_mgr.new_tab_in_group(group_id);
                }
                TabAction::NewTabWithType { group_id, view_type } => {
                    self.layout_mgr.new_tab_with_type(group_id, view_type);
                }
                TabAction::SwitchType { group_id, tab_index, new_type } => {
                    self.layout_mgr.switch_tab_type(group_id, tab_index, new_type);
                }
                TabAction::Reorder { group_id, from_index, to_index } => {
                    self.layout_mgr.reorder_tabs(group_id, from_index, to_index);
                }
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
        // Persist the split-view layout tree.
        self.persist.layout_json = LayoutState::capture(&self.layout_mgr).to_json().ok();
        if let Ok(json) = serde_json::to_string(&self.persist) {
            _storage.set_string(STORAGE_KEY, json);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // ── Apply theme ──────────────────────────────────────────────────
        let visuals = if self.persist.theme == "light" {
            light_visuals()
        } else {
            dark_visuals()
        };
        ui.ctx().set_visuals(visuals);

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
                        // ── Split View commands ────────────────────────────
                        ui.label("Split View");
                        if ui.button("⊟  Split Horizontal").clicked() {
                            self.layout_mgr.split_horizontal("Sheet", None, 0.5);
                            ui.close();
                        }
                        if ui.button("⊠  Split Vertical").clicked() {
                            self.layout_mgr.split_vertical("Sheet", None, 0.5);
                            ui.close();
                        }
                        if ui.button("⊞  Open Tab").clicked() {
                            self.layout_mgr.open_tab("Sheet", None);
                            ui.close();
                        }
                        if ui.button("✕  Close View").clicked() {
                            if let Some(fid) = self.layout_mgr.focus.focused {
                                self.layout_mgr.close_view(fid);
                            }
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("⇥  Focus Next").clicked() {
                            self.layout_mgr.focus_next();
                            ui.close();
                        }
                        if ui.button("⇤  Focus Previous").clicked() {
                            self.layout_mgr.focus_prev();
                            ui.close();
                        }
                        ui.separator();
                        // ── Side panels ────────────────────────────────────
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
                        ui.separator();
                        let is_dark = self.persist.theme == "dark";
                        if ui.selectable_label(is_dark, "Dark Theme").clicked() {
                            self.persist.theme = "dark".into();
                            ui.close();
                        }
                        if ui.selectable_label(!is_dark, "Light Theme").clicked() {
                            self.persist.theme = "light".into();
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

        // ── Floating panels ─────────────────────────────────────────────────
        //
        // All panels are now floating windows (like the perf overlay),
        // leaving the central area exclusively for the Doc | Spreadsheet split view.

        let ctx = ui.ctx();

        // Object Explorer
        let mut show_explorer = self.persist.show_explorer;
        egui::Window::new("🔍 Object Explorer")
            .default_pos([20.0, 100.0])
            .default_size([240.0, 350.0])
            .resizable(true)
            .open(&mut show_explorer)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    crate::explorer::object_explorer(ui, &self.workspace);
                });
            });
        self.persist.show_explorer = show_explorer;

        // Schema Panel
        let mut show_schema = self.persist.show_schema_panel;
        egui::Window::new("📋 Schema")
            .default_pos([20.0, 470.0])
            .default_size([260.0, 400.0])
            .resizable(true)
            .open(&mut show_schema)
            .show(ctx, |ui| {
                if let Some(handle) = &self.handle {
                    panels::schema_panel(ui, &handle.dataset, self.profile.as_ref());
                } else {
                    label::muted(ui, "No dataset loaded");
                }
            });
        self.persist.show_schema_panel = show_schema;

        // Workflow Panel
        let mut show_workflow = self.persist.show_workflow_panel;
        egui::Window::new("⚙ Workflow")
            .default_pos([20.0, 890.0])
            .default_size([280.0, 350.0])
            .resizable(true)
            .open(&mut show_workflow)
            .show(ctx, |ui| {
                if self.handle.is_some() {
                    let columns: Vec<String> = self.handle.as_ref()
                        .map(|h| h.dataset.schema.column_names().iter().map(|s| s.to_string()).collect())
                        .unwrap_or_default();

                    let wf_actions = workflow_sidebar::workflow_panel(
                        ui,
                        &self.workflow,
                        &columns,
                        &mut self.expanded_modifiers,
                    );

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

                    if has_actions {
                        self.fetch_page();
                        self.refresh_count();
                    }
                } else {
                    label::muted(ui, "No workflow");
                }
            });
        self.persist.show_workflow_panel = show_workflow;

        // Column Inspector
        let mut show_inspector = self.persist.show_inspector_panel;
        egui::Window::new("📊 Column Inspector")
            .default_pos([1200.0, 100.0])
            .default_size([260.0, 450.0])
            .resizable(true)
            .open(&mut show_inspector)
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
        self.persist.show_inspector_panel = show_inspector;


        // ── Central panel: split-view framework ──────────────────────────
        //
        // The LayoutManager owns the layout tree.  We pass view-render
        // callbacks via ViewContext so the renderer crate stays decoupled
        // from app-level types.
        egui::CentralPanel::default().show(ui, |ui: &mut egui::Ui| {
            // Capture references needed inside closures.
            let handle = self.handle.clone();
            let current_batch = self.current_batch.clone();
            let loading = self.loading;
            let mut grid_state = self.grid_state.clone();
            let total_rows = self.total_rows;
            let document = self.document.clone();
            let tab_grid_states = &mut self.tab_grid_states;

            let mut pending_grid_actions: Vec<GridAction> = Vec::new();
            let mut new_visible_rows: Option<usize> = None;

            let mut ctx = ViewContext {
                render_spreadsheet: &mut |ui: &mut egui::Ui, leaf| {
                    // ── Per-tab state: load this tab's scroll/selection ───
                    let vid = leaf.view_id;
                    if let Some(tab_gs) = tab_grid_states.get(&vid) {
                        grid_state.scroll_y = tab_gs.scroll_y;
                        grid_state.scroll_x = tab_gs.scroll_x;
                        grid_state.selection = tab_gs.selection.clone();
                    }

                    if let Some(h) = &handle {
                        if let Some(batch) = current_batch.clone() {
                            let height = ui.available_height();
                            new_visible_rows = Some(grid_state.rows_in_viewport(height));
                            let actions = GridRenderer::show(
                                ui, &h.dataset, &batch, &mut grid_state, total_rows,
                            );
                            pending_grid_actions.extend(actions);
                        } else if loading {
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

                    // ── Save per-tab state ──────────────────────────────
                    tab_grid_states.insert(vid, GridState {
                        scroll_y: grid_state.scroll_y,
                        scroll_x: grid_state.scroll_x,
                        selection: grid_state.selection.clone(),
                        ..Default::default()
                    });
                },
                render_document: &mut |ui: &mut egui::Ui, _leaf| {
                    document_view::render_document(ui, &document);
                },
            };

            let mut tab_actions: Vec<TabAction> = Vec::new();
            split_view::render_layout(ui, &mut self.layout_mgr, &mut ctx, &mut tab_actions);

            // Apply grid actions collected from inside the closure.
            if let Some(vr) = new_visible_rows {
                self.viewport.visible_rows = vr;
            }
            self.grid_state = grid_state;
            self.handle_grid_actions(pending_grid_actions);

            // Apply tab actions.
            self.handle_tab_actions(tab_actions);
        });

        // Performance overlay (always on top).
        self.perf.show(ui.ctx());

        // Drive continuous repainting for 60 FPS.
        ui.ctx().request_repaint();
    }
}

// ── Theme helpers ────────────────────────────────────────────────────────────

fn dark_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.override_text_color = Some(egui::Color32::from_rgb(220, 220, 230));
    v.window_fill = egui::Color32::from_rgb(14, 14, 20);
    v.panel_fill = egui::Color32::from_rgb(14, 14, 20);
    v
}

fn light_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::light();
    v.override_text_color = Some(egui::Color32::from_rgb(30, 30, 40));
    v.window_fill = egui::Color32::from_rgb(248, 248, 252);
    v.panel_fill = egui::Color32::from_rgb(245, 245, 250);
    v
}