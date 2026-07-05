//! Workspace Object Explorer — VS Code-style sidebar.
//!
//! # Spec: Object Explorer (§Deliverable 2)
//!
//! Shows workspace objects grouped by type:
//! 📊 Datasets  📄 Documents  ⚙ Workflows

use egui::Ui;
use workspace_core::Workspace;

/// Render the workspace object explorer.
pub fn object_explorer(ui: &mut Ui, workspace: &Workspace) {
    let dark = ui.visuals().dark_mode;
    let text_color = if dark { egui::Color32::from_rgb(200, 200, 220) } else { egui::Color32::from_rgb(30, 30, 50) };
    let muted_color = if dark { egui::Color32::from_rgb(100, 100, 130) } else { egui::Color32::from_rgb(130, 130, 160) };

    ui.label(egui::RichText::new(&workspace.name).size(13.0).strong());
    ui.separator();
    ui.add_space(4.0);

    let types = [
        ("📊 Datasets", workspace.objects_of_type(cdm::ObjectType::Dataset)),
        ("📄 Documents", workspace.objects_of_type(cdm::ObjectType::Document)),
        ("⚙ Workflows", workspace.objects_of_type(cdm::ObjectType::Workflow)),
    ];

    for (label, objects) in &types {
        ui.add_space(2.0);
        ui.label(egui::RichText::new(*label).size(12.0).strong());

        if objects.is_empty() {
            ui.label(
                egui::RichText::new("  (empty)")
                    .size(11.0)
                    .color(muted_color),
            );
        } else {
            for obj in objects {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(obj.icon()).size(12.0));
                    ui.label(
                        egui::RichText::new(obj.name())
                            .size(12.0)
                            .color(text_color),
                    );
                });
            }
        }
    }

    ui.add_space(8.0);
    ui.separator();
    ui.label(
        egui::RichText::new(format!("{} objects", workspace.object_count()))
            .size(10.0)
            .color(muted_color),
    );
}
