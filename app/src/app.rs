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
    /// Left sidebar (Navigator) visibility.
    #[serde(default = "default_true")]
    show_left_panel: bool,
    /// Right sidebar (Inspector) visibility.
    #[serde(default = "default_true")]
    show_right_panel: bool,
    /// Active navigator tab: 0 = Explorer, 1 = Schema, 2 = Workflow.
    #[serde(default)]
    left_nav_tab: u8,
    /// Performance overlay visibility.
    #[serde(default)]
    perf_visible: bool,
    /// Last inspected column name.
    #[serde(default)]
    inspected_col: Option<String>,
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
            show_left_panel: true,
            show_right_panel: true,
            left_nav_tab: 0,
            perf_visible: false,
            inspected_col: None,
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
    /// Per-tab viewport: each tab fetches its own data window.
    tab_viewports: HashMap<ViewId, Viewport>,
    /// Per-tab data batch: each tab renders from its own fetched rows.
    tab_batches: HashMap<ViewId, RecordBatch>,

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
            tab_viewports: HashMap::new(),
            tab_batches: HashMap::new(),
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
                GridAction::ScrollChanged { first_row, view_id } => {
                    // Store per-tab viewport.
                    let vid = ViewId(view_id);
                    let vp = self.tab_viewports.entry(vid).or_insert(Viewport::default());
                    vp.first_row = first_row;
                    vp.visible_rows = self.viewport.visible_rows;
                    *vp = vp.clamped(self.total_rows);

                    // Compute union viewport covering all active tabs.
                    let mut min_row = first_row;
                    let mut max_row = first_row + vp.visible_rows;
                    for v in self.tab_viewports.values() {
                        min_row = min_row.min(v.first_row);
                        max_row = max_row.max(v.first_row + v.visible_rows);
                    }
                    self.viewport.first_row = min_row;
                    self.viewport.visible_rows = (max_row - min_row).max(50);
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

        // Status bar — only remaining panel row.
        // Toolbar + View menu are now injected directly into the tab bar (single header row).
        egui::Panel::bottom("status")
            .show_separator_line(false)
            .show(ui, |ui: &mut egui::Ui| {
                self.show_status_bar(ui);
            });

        // ── Compact header row ────────────────────────────────────────────────
        //
        // Single row: ◧ navigator | ≡ unified-menu (opens dropdown) | [↻ Sorts] [× Filters] | ◨ inspector
        // Dataset info is NOT shown here — the tab bar and status bar provide enough context.
        {
            let dark          = self.persist.theme == "dark";
            let bg_fill       = if dark { egui::Color32::from_rgb(28, 28, 30) } else { egui::Color32::WHITE };
            let accent        = if dark { egui::Color32::from_rgb(10, 132, 255) } else { egui::Color32::from_rgb(0, 122, 255) };
            let icon_col      = if dark { egui::Color32::from_rgb(142, 142, 147) } else { egui::Color32::from_rgb(110, 110, 118) };
            let chip_bg       = if dark { egui::Color32::from_rgb(44, 44, 48) } else { egui::Color32::from_rgb(228, 228, 234) };
            let chip_bg_hover = if dark { egui::Color32::from_rgb(58, 58, 62) } else { egui::Color32::from_rgb(210, 210, 218) };
            let has_sorts   = !self.grid_state.sort_specs.is_empty();
            let has_filters = self.grid_state.has_active_filters();

            egui::Panel::top("toolbar")
                .show_separator_line(false)
                // vertical padding gives the toolbar a comfortable breathing room
                .frame(egui::Frame::new().fill(bg_fill).inner_margin(egui::Margin::same(split_view::BAR_PADDING as i8)))
                .show(ui, |ui: &mut egui::Ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);

                        // ◧ Navigator sidebar toggle
                        if ui.add(
                            egui::Button::new(egui::RichText::new("◧").size(14.0).color(
                                if self.persist.show_left_panel { accent } else { icon_col }
                            ))
                            .fill(egui::Color32::TRANSPARENT)
                            .stroke(egui::Stroke::NONE)
                            .min_size(egui::Vec2::new(28.0, 24.0)),
                        ).clicked() {
                            self.persist.show_left_panel = !self.persist.show_left_panel;
                        }

                        // ≡ Unified menu — true circle using painter + egui::Popup::menu.
                        // menu_button sizes itself from text metrics → always non-square.
                        // allocate_exact_size forces a 28×28 square, we paint a circle, and
                        // egui::Popup::menu handles toggle + positioning automatically.
                        {
                            let menu_id  = egui::Id::new("main_menu");
                            let is_open  = egui::Popup::is_id_open(ui.ctx(), menu_id);

                            let btn_size = egui::Vec2::splat(28.0);
                            let (btn_rect, btn_resp) = ui.allocate_exact_size(btn_size, egui::Sense::click());

                            let circle_fill = if btn_resp.hovered() || btn_resp.is_pointer_button_down_on() || is_open {
                                chip_bg_hover
                            } else {
                                chip_bg
                            };
                            ui.painter().circle_filled(btn_rect.center(), btn_rect.height() / 2.0, circle_fill);
                            crate::label::paint_icon_centered(
                                ui, btn_rect, "≡",
                                egui::FontId::proportional(15.0),
                                icon_col,
                            );

                            // Popup::menu toggles open/close on click automatically.
                            egui::Popup::menu(&btn_resp)
                                .id(menu_id)
                                .show(|ui| {
                                    ui.set_min_width(160.0);
                                    let close = |ui: &mut egui::Ui| egui::Popup::close_id(ui.ctx(), menu_id);

                                    if ui.button("Open File…").clicked() {
                                        if let Some(path) = rfd::FileDialog::new()
                                            .add_filter("Data Files", &["parquet", "csv"])
                                            .pick_file()
                                        {
                                            self.open_file(path);
                                        }
                                        close(ui);
                                    }
                                    ui.separator();
                                    ui.label("Split View");
                                    if ui.button("⊟  Horizontal").clicked() { self.layout_mgr.split_horizontal("Sheet", None, 0.5); close(ui); }
                                    if ui.button("⊠  Vertical").clicked()   { self.layout_mgr.split_vertical("Sheet", None, 0.5);   close(ui); }
                                    if ui.button("⊞  New Tab").clicked()     { self.layout_mgr.open_tab("Sheet", None);              close(ui); }
                                    if ui.button("✕  Close View").clicked() {
                                        if let Some(fid) = self.layout_mgr.focus.focused { self.layout_mgr.close_view(fid); }
                                        close(ui);
                                    }
                                    ui.separator();
                                    if ui.button("⇥  Next").clicked() { self.layout_mgr.focus_next(); close(ui); }
                                    if ui.button("⇤  Prev").clicked() { self.layout_mgr.focus_prev(); close(ui); }
                                    ui.separator();
                                    ui.label("Sidebars");
                                    if ui.selectable_label(self.persist.show_left_panel,  "Navigator").clicked() { self.persist.show_left_panel  = !self.persist.show_left_panel;  close(ui); }
                                    if ui.selectable_label(self.persist.show_right_panel, "Inspector").clicked() { self.persist.show_right_panel = !self.persist.show_right_panel; close(ui); }
                                    ui.separator();
                                    if ui.selectable_label(self.perf.visible, "Perf Overlay").clicked() { self.perf.visible = !self.perf.visible; close(ui); }
                                    ui.separator();
                                    let is_dark = self.persist.theme == "dark";
                                    if ui.selectable_label(is_dark,  "Dark").clicked()  { self.persist.theme = "dark".into();  close(ui); }
                                    if ui.selectable_label(!is_dark, "Light").clicked() { self.persist.theme = "light".into(); close(ui); }
                                });
                        }

                        ui.add_space(4.0);

                        // ↻ Sorts chip — shown only when sorts are active
                        if has_sorts {
                            if ui.add(
                                egui::Button::new(egui::RichText::new("↻ Sorts").size(11.0).color(icon_col))
                                    .fill(chip_bg).stroke(egui::Stroke::NONE),
                            ).on_hover_text("Clear all sorts").clicked() {
                                self.grid_state.clear_sorts();
                                if let Some(ids) = &self.workflow_ids {
                                    let sort_id = ids.sort;
                                    let _ = self.workflow.update_payload(sort_id, NodePayload::Sort { specs: vec![] });
                                }
                                self.viewport.first_row = 0;
                                self.grid_state.scroll_y = 0.0;
                                self.fetch_page();
                            }
                            ui.add_space(2.0);
                        }

                        // × Filters chip — shown only when filters are active
                        if has_filters {
                            if ui.add(
                                egui::Button::new(egui::RichText::new("× Filters").size(11.0).color(icon_col))
                                    .fill(chip_bg).stroke(egui::Stroke::NONE),
                            ).on_hover_text("Clear all filters").clicked() {
                                self.grid_state.column_filters.clear();
                                if let Some(ids) = &self.workflow_ids {
                                    let filter_id = ids.filter;
                                    let _ = self.workflow.update_payload(
                                        filter_id,
                                        NodePayload::Filter { expr: FilterExpr::None },
                                    );
                                }
                                self.apply_filter();
                            }
                        }

                        // ◨ Inspector sidebar toggle — right-aligned
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(8.0);
                            if ui.add(
                                egui::Button::new(egui::RichText::new("◨").size(14.0).color(
                                    if self.persist.show_right_panel { accent } else { icon_col }
                                ))
                                .fill(egui::Color32::TRANSPARENT)
                                .stroke(egui::Stroke::NONE)
                                .min_size(egui::Vec2::new(28.0, 24.0)),
                            ).clicked() {
                                self.persist.show_right_panel = !self.persist.show_right_panel;
                            }
                        });
                    });
                });
        }

        // ── Fixed side panels ────────────────────────────────────────────────
        //
        // Left: Navigator (Explorer / Schema / Workflow tabs).
        // Right: Column Inspector.

        let dark = self.persist.theme == "dark";

        let window_bg = if dark { egui::Color32::from_rgb(28, 28, 30) } else { egui::Color32::WHITE };
        let card_fill  = if dark { egui::Color32::from_rgb(36, 36, 40) } else { egui::Color32::WHITE };
        // The outer panel frame is window-coloured so the rounded card inside
        // looks like it floats with a gap on all sides.
        let sidebar_frame = egui::Frame::new()
            .fill(window_bg)
            .inner_margin(egui::Margin::same(0));
        let card_radius = egui::epaint::CornerRadius::same(10);
        // Top is flush with the panel so the tab row sits at the same
        // vertical level as the split-view tab bar.  Sides and bottom
        // keep the 8 px breathing gap.
        let card_gap = egui::Margin { top: 0, left: 8, right: 8, bottom: 8 };
        let card_pad = egui::Margin::same(split_view::BAR_PADDING as i8);

        // ── Left: Navigator ─────────────────────────────────────────────────
        if self.persist.show_left_panel {
            egui::Panel::left("navigator_panel")
                .resizable(true)
                .default_size(400.0)
                .min_size(200.0)
                .frame(sidebar_frame)
                .show_separator_line(false)
                .show(ui, |ui: &mut egui::Ui| {
                    egui::Frame::new()
                        .fill(card_fill)
                        .corner_radius(card_radius)
                        .outer_margin(card_gap)
                        .inner_margin(card_pad)
                        .show(ui, |ui| {
                            ui.set_min_height(ui.available_height());

                            // ── Nav tab row — same component as split-view tabs ──
                            ui.horizontal(|ui| {
                                let tabs: &[(&str, u8)] = &[
                                    ("Explorer", 0),
                                    ("Schema",   1),
                                    ("Workflow", 2),
                                ];
                                for (tab_label, idx) in tabs {
                                    let selected = self.persist.left_nav_tab == *idx;
                                    if split_view::tab_button(ui, tab_label, selected) {
                                        self.persist.left_nav_tab = *idx;
                                    }
                                }
                            });
                            ui.add_space(4.0);
                            ui.separator();
                            ui.add_space(4.0);

                            // ── Content ───────────────────────────────────────
                            match self.persist.left_nav_tab {
                                0 => {
                                    egui::ScrollArea::vertical().show(ui, |ui| {
                                        crate::explorer::object_explorer(ui, &self.workspace);
                                    });
                                }
                                1 => {
                                    if let Some(handle) = &self.handle {
                                        panels::schema_panel(ui, &handle.dataset, self.profile.as_ref());
                                    } else {
                                        label::muted(ui, "No dataset loaded.");
                                    }
                                }
                                _ => {
                                    if self.handle.is_some() {
                                        let columns: Vec<String> = self.handle.as_ref()
                                            .map(|h| h.dataset.schema.column_names()
                                                .iter().map(|s| s.to_string()).collect())
                                            .unwrap_or_default();

                                        egui::ScrollArea::vertical().show(ui, |ui| {
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
                                                let pos = self.workflow.nodes().position(|n| n.id == *node_id);
                                                if let Some(i) = pos { let _ = self.workflow.move_modifier_up(i); }
                                            }
                                            for node_id in &wf_actions.move_down {
                                                let pos = self.workflow.nodes().position(|n| n.id == *node_id);
                                                if let Some(i) = pos { let _ = self.workflow.move_modifier_down(i); }
                                            }
                                            for node_id in &wf_actions.toggle {
                                                let _ = self.workflow.toggle_node(*node_id);
                                            }
                                            for node_id in &wf_actions.duplicate {
                                                if let Some(node) = self.workflow.node(*node_id) {
                                                    let kind = node.kind;
                                                    let payload = node.payload.clone();
                                                    let pos = self.workflow.nodes().position(|n| n.id == *node_id);
                                                    if let Some(i) = pos {
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
                                                    workflow::NodeKind::Sort => workflow::NodePayload::Sort { specs: vec![] },
                                                    _ => workflow::NodePayload::Empty,
                                                };
                                                let _ = self.workflow.add_node(kind, payload);
                                            }
                                            if has_actions {
                                                self.fetch_page();
                                                self.refresh_count();
                                            }
                                        });
                                    } else {
                                        label::muted(ui, "No workflow.");
                                    }
                                }
                            }
                        });
                });
        }

        // ── Right: Inspector ────────────────────────────────────────────────
        if self.persist.show_right_panel {
            egui::Panel::right("inspector_panel")
                .resizable(true)
                .default_size(400.0)
                .min_size(200.0)
                .frame(sidebar_frame)
                .show_separator_line(false)
                .show(ui, |ui: &mut egui::Ui| {
                    egui::Frame::new()
                        .fill(card_fill)
                        .corner_radius(card_radius)
                        .outer_margin(card_gap)
                        .inner_margin(card_pad)
                        .show(ui, |ui| {
                            ui.set_min_height(ui.available_height());
                            label::muted(ui, "Inspector");
                            ui.add_space(4.0);
                            ui.separator();
                            ui.add_space(4.0);

                            let col_stats = self.inspected_col.as_ref().and_then(|name| {
                                self.dataset_stats.as_ref()?.columns.iter().find(|c| &c.name == name)
                            });
                            let col_profile = self.inspected_col.as_ref().and_then(|name| {
                                self.profile.as_ref()?.column(name)
                            });
                            panels::column_inspector(ui, col_stats, col_profile);
                        });
                });
        }


        // ── Central panel: split-view framework ──────────────────────────────────
        let panel_fill = if self.persist.theme == "dark" {
            egui::Color32::from_rgb(28, 28, 30)
        } else {
            egui::Color32::WHITE
        };
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(panel_fill))
            .show(ui, |ui: &mut egui::Ui| {
                let handle = self.handle.clone();
                let current_batch = self.current_batch.clone();
                let loading = self.loading;
                let total_rows = self.total_rows;
                let batch_start_row = self.viewport.first_row;
                let document = self.document.clone();
                let tab_grid_states = &mut self.tab_grid_states;
                let tab_viewports = &mut self.tab_viewports;

                let mut pending_grid_actions: Vec<GridAction> = Vec::new();
                let mut new_visible_rows: Option<usize> = None;
                let mut tab_actions: Vec<TabAction> = Vec::new();

                let mut ctx = ViewContext {
                    render_spreadsheet: &mut |ui: &mut egui::Ui, leaf| {
                        let vid = leaf.view_id;
                        let mut tab_gs = tab_grid_states
                            .entry(vid)
                            .or_insert_with(GridState::new)
                            .clone();

                        if let Some(h) = &handle {
                            if let Some(batch) = current_batch.clone() {
                                let height = ui.available_height();
                                let vis = tab_gs.rows_in_viewport(height);
                                new_visible_rows = Some(vis);

                                let vp = tab_viewports.entry(vid).or_insert_with(Viewport::default);
                                vp.first_row = tab_gs.first_row();
                                vp.visible_rows = vis;

                                let actions = GridRenderer::show(
                                    ui, &h.dataset, &batch, &mut tab_gs, total_rows,
                                    vid.0, batch_start_row,
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
                                    label::text(ui, "Drop a Parquet or CSV file here");
                                    label::muted(ui, "or click Open in the toolbar");
                                });
                            });
                        }

                        if let Some(stored) = tab_grid_states.get_mut(&vid) {
                            *stored = tab_gs;
                        }
                    },
                    render_document: &mut |ui: &mut egui::Ui, leaf| {
                        document_view::render_document(ui, &document, leaf.view_id.0);
                    },
                    tab_left:  None,
                    tab_right: None,
                };

                split_view::render_layout(ui, &mut self.layout_mgr, &mut ctx, &mut tab_actions);

                if let Some(vr) = new_visible_rows { self.viewport.visible_rows = vr; }
                self.handle_grid_actions(pending_grid_actions);
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
    v.override_text_color = Some(egui::Color32::from_rgb(242, 242, 247));
    v.window_fill = egui::Color32::from_rgb(28, 28, 30);
    v.panel_fill = egui::Color32::from_rgb(28, 28, 30);
    v.faint_bg_color = egui::Color32::from_rgb(36, 36, 38);
    v.extreme_bg_color = egui::Color32::from_rgb(18, 18, 20);
    let sep = egui::Color32::from_rgb(54, 54, 56);
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(0.5, sep);
    v.widgets.inactive.bg_stroke = egui::Stroke::new(0.5, sep);
    v.widgets.hovered.bg_stroke = egui::Stroke::new(0.5, egui::Color32::from_rgb(80, 80, 88));
    v.widgets.active.bg_stroke = egui::Stroke::new(0.5, egui::Color32::from_rgb(100, 100, 110));
    v.selection.bg_fill = egui::Color32::from_rgba_premultiplied(10, 132, 255, 60);
    let r = egui::epaint::CornerRadius::same(5);
    v.widgets.inactive.corner_radius = r;
    v.widgets.hovered.corner_radius = r;
    v.widgets.active.corner_radius = r;
    v.widgets.open.corner_radius = r;
    v
}

fn light_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::light();
    v.override_text_color = Some(egui::Color32::from_rgb(0, 0, 0));
    v.window_fill = egui::Color32::WHITE;
    v.panel_fill = egui::Color32::WHITE;
    v.faint_bg_color = egui::Color32::from_rgb(242, 242, 247);
    v.extreme_bg_color = egui::Color32::from_rgb(242, 242, 247);
    let sep = egui::Color32::from_rgb(196, 196, 200);
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(0.5, sep);
    v.widgets.inactive.bg_stroke = egui::Stroke::new(0.5, sep);
    v.widgets.hovered.bg_stroke = egui::Stroke::new(0.5, egui::Color32::from_rgb(160, 160, 168));
    v.widgets.active.bg_stroke = egui::Stroke::new(0.5, egui::Color32::from_rgb(130, 130, 140));
    v.selection.bg_fill = egui::Color32::from_rgba_premultiplied(0, 122, 255, 55);
    let r = egui::epaint::CornerRadius::same(5);
    v.widgets.inactive.corner_radius = r;
    v.widgets.hovered.corner_radius = r;
    v.widgets.active.corner_radius = r;
    v.widgets.open.corner_radius = r;
    v
}