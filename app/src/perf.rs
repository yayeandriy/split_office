use core::fmt_large;

/// Performance metrics tracked every frame.
#[derive(Default, Clone)]
pub struct PerfOverlay {
    /// Smoothed frames per second.
    pub fps: f64,
    /// Last frame time in milliseconds.
    pub frame_ms: f64,
    /// Current memory usage in MiB (RSS).
    pub memory_mib: f64,
    /// Number of rows visible in the current viewport.
    pub visible_rows: usize,
    /// Last DuckDB query latency in milliseconds.
    pub query_latency_ms: f64,
    /// Number of total rows in the active dataset (after filter).
    pub total_rows: usize,
    /// Whether the overlay is visible.
    pub visible: bool,
    // Internal smoothing state.
    frame_count: u64,
    fps_accum: f64,
}

impl PerfOverlay {
    pub fn new() -> Self {
        Self {
            visible: true,
            ..Default::default()
        }
    }

    pub fn update(&mut self, frame_time_secs: f64) {
        self.frame_ms = frame_time_secs * 1000.0;
        self.frame_count += 1;
        self.fps_accum += 1.0 / frame_time_secs;

        // Smooth FPS over 30 frames.
        if self.frame_count.is_multiple_of(30) {
            self.fps = self.fps_accum / 30.0;
            self.fps_accum = 0.0;
        }

        // Memory (RSS) via /proc/self/status on Linux, or approximate on macOS.
        self.memory_mib = Self::current_memory_mib();
    }

    fn current_memory_mib() -> f64 {
        // macOS: use `mach_task_basic_info` (simplified approximation via allocated).
        // For Phase 0, we read from /proc/self/status on Linux or skip on macOS.
        #[cfg(target_os = "linux")]
        {
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        let kb: u64 = line
                            .split_whitespace()
                            .nth(1)
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0);
                        return kb as f64 / 1024.0;
                    }
                }
            }
        }
        0.0
    }

    /// Render the overlay as a small floating panel in the top-right.
    ///
    /// Note: This overlay uses `ui.label()` directly — it is a developer debug
    /// tool, not a user-facing surface, and is therefore exempt from the
    /// `label::*` typography mandate.
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        let screen_rect = ctx.content_rect();
        let panel_rect = egui::Rect::from_min_size(
            egui::Pos2::new(screen_rect.right() - 220.0, screen_rect.top() + 40.0),
            egui::Vec2::new(210.0, 150.0),
        );

        egui::Area::new(egui::Id::new("perf_overlay"))
            .fixed_pos(panel_rect.min)
            .order(egui::Order::Foreground)
            .movable(true)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgba_premultiplied(10, 10, 18, 210))
                    .corner_radius(6.0)
                    .inner_margin(10.0)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 70)))
                    .show(ui, |ui| {
                        // Title bar with close button.
                        ui.horizontal(|ui| {
                            ui.label("Performance");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("×").clicked() {
                                    self.visible = false;
                                }
                            });
                        });
                        ui.separator();
                        ui.set_width(190.0);
                        let label = |ui: &mut egui::Ui, key: &str, val: &str| {
                            ui.horizontal(|ui| {
                                ui.label(key);
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(val);
                                });
                            });
                        };

                        label(ui, "FPS", &format!("{:.0}", self.fps));
                        label(ui, "Frame", &format!("{:.2} ms", self.frame_ms));
                        label(ui, "Memory", &format!("{:.1} MiB", self.memory_mib));
                        label(ui, "Visible rows", &format!("{}", self.visible_rows));
                        label(ui, "Total rows", &fmt_large(self.total_rows));
                        label(ui, "Query", &format!("{:.1} ms", self.query_latency_ms));
                    });
            });
    }
}

