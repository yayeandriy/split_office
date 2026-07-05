//! Document View — renders a structured document from the CDM.
//!
//! # Spec: Document View Phase 1
//!
//! Renders a block-based document alongside the spreadsheet.
//! Supports headings, paragraphs, and reference blocks.

use document_core::{Block, Document, HeadingBlock, ParagraphBlock, ReferenceBlock, TableBlock};

/// Render a full document into the egui UI.
pub fn render_document(ui: &mut egui::Ui, doc: &Document) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── Document title ────────────────────────────────────────────────
        ui.heading(&doc.name);
        ui.separator();

        for section in &doc.sections {
            render_section(ui, section);
        }
    });
}

fn render_section(ui: &mut egui::Ui, section: &document_core::Section) {
    if !section.title.is_empty() {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(&section.title)
                .size(15.0)
                .strong(),
        );
        ui.add_space(4.0);
    }

    for block in &section.blocks {
        render_block(ui, block);
    }
}

fn render_block(ui: &mut egui::Ui, block: &Block) {
    match block {
        Block::Heading(h) => render_heading(ui, h),
        Block::Paragraph(p) => render_paragraph(ui, p),
        Block::Reference(r) => render_reference(ui, r),
        Block::Table(t) => render_table(ui, t),
    }
}

fn render_heading(ui: &mut egui::Ui, heading: &HeadingBlock) {
    let size = match heading.level {
        1 => 16.0,
        2 => 14.0,
        _ => 13.0,
    };
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(&heading.text)
            .size(size)
            .strong()
            .color(egui::Color32::from_rgb(220, 220, 230)),
    );
    ui.add_space(2.0);
}

fn render_paragraph(ui: &mut egui::Ui, para: &ParagraphBlock) {
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new(&para.text)
            .size(13.0)
            .color(egui::Color32::from_rgb(180, 180, 200)),
    );
}

fn render_reference(ui: &mut egui::Ui, ref_block: &ReferenceBlock) {
    let bg = egui::Color32::from_rgb(25, 25, 38);
    let border = egui::Color32::from_rgb(50, 60, 90);

    egui::Frame::new()
        .fill(bg)
        .stroke(egui::Stroke::new(1.0, border))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("📎").size(12.0));
                ui.label(
                    egui::RichText::new(&ref_block.label)
                        .size(12.0)
                        .color(egui::Color32::from_rgb(130, 200, 255)),
                );
                ui.label(
                    egui::RichText::new(format!("({})", ref_block.target))
                        .size(10.0)
                        .color(egui::Color32::from_rgb(100, 100, 130)),
                );
            });
        });

    ui.add_space(4.0);
}

fn render_table(ui: &mut egui::Ui, table: &TableBlock) {
    let bg = egui::Color32::from_rgb(22, 22, 32);
    let border = egui::Color32::from_rgb(40, 40, 55);

    egui::Frame::new()
        .fill(bg)
        .stroke(egui::Stroke::new(1.0, border))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(format!("📊 {}", table.caption))
                    .size(12.0),
            );
            ui.label(
                egui::RichText::new(format!("Source: {}", table.source))
                    .size(10.0)
                    .color(egui::Color32::from_rgb(100, 100, 130)),
            );
        });

    ui.add_space(4.0);
}
