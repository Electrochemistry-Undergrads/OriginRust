//! A Notes window (T154): one text box over the whole window. The text being typed is kept here and
//! written to the project as one undo step ("Edit notes") when the box loses the keyboard.
//!
//! Linked reports (T439): Preview shows the text as a report, its links resolved and its figure lines as figures;
//! Export Report... writes it as HTML or Markdown with the figures' graphs as PNG files beside it.

use crate::actions::Action;
use eframe::egui;
use sheaf_core::{Document, WindowId};

/// What a Notes window keeps between frames.
#[derive(Default)]
pub struct NotesUi {
    /// While the text is edited: the project's text when the editing began, and the text typed.
    editing: Option<(String, String)>,
    /// Showing the resolved report instead of the text box (T439).
    pub preview: bool,
    /// The pictures of the Preview's figures (T459): per graph, the project revision and width they were drawn for.
    figures: std::collections::HashMap<WindowId, (u64, u32, egui::TextureHandle)>,
}

/// The widest a figure is drawn in the Preview, in points.
const FIGURE_WIDTH: f32 = 480.0;

/// Graph `graph` drawn `width` points wide, kept until the project or the width changes.
fn figure_texture(
    ui: &egui::Ui,
    doc: &Document,
    graph: WindowId,
    width: f32,
    cache: &mut std::collections::HashMap<WindowId, (u64, u32, egui::TextureHandle)>,
) -> Option<egui::TextureHandle> {
    let key = width.round() as u32;
    if let Some((rev, w, t)) = cache.get(&graph)
        && *rev == doc.revision()
        && *w == key
    {
        return Some(t.clone());
    }
    let page = doc.project().graph(graph)?.page.width_pt;
    let scale = f64::from(width) * f64::from(ui.ctx().pixels_per_point()) / page.max(1.0);
    let scene = sheaf_plot::graph_scene(doc.project(), graph).ok()?;
    let raster = sheaf_plot::render_scene_raster(&scene, scale).ok()?;
    let image = egui::ColorImage::from_rgba_unmultiplied([raster.width as usize, raster.height as usize], &raster.rgba);
    let texture = ui.ctx().load_texture(format!("report-figure-{}", graph.0), image, egui::TextureOptions::LINEAR);
    cache.insert(graph, (doc.revision(), key, texture.clone()));
    Some(texture)
}

/// Writes the report of Notes window `id` to `path`: Markdown for a `.md` or `.markdown` name, else HTML. Each
/// figure's graph goes beside it as `<name>_<graph>.png` (150 dpi). Returns how many pictures were written.
pub fn export_report(project: &sheaf_core::Project, id: WindowId, path: &std::path::Path) -> Result<usize, String> {
    use sheaf_core::report::{ReportPart, to_html, to_markdown};
    let notes = project.notes_window(id).ok_or("the Notes window is gone")?;
    let parts = project.report_parts(&notes.text);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("report").to_string();
    let dir = path.parent().map(std::path::Path::to_path_buf).unwrap_or_default();
    let picture = |name: &str| format!("{stem}_{name}.png");
    let mut written = 0;
    for p in &parts {
        if let ReportPart::Figure { name, graph: Some(g), .. } = p {
            let png = sheaf_plot::render_png(project, *g, 150.0).map_err(|e| e.to_string())?;
            std::fs::write(dir.join(picture(name)), png).map_err(|e| e.to_string())?;
            written += 1;
        }
    }
    let markdown = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"));
    let text = if markdown { to_markdown(&parts, &picture) } else { to_html(&notes.title(), &parts, &picture) };
    std::fs::write(path, text).map_err(|e| e.to_string())?;
    Ok(written)
}

/// The report, resolved, as it will read.
fn preview(ui: &mut egui::Ui, doc: &Document, id: WindowId, state: &mut NotesUi) {
    use sheaf_core::report::ReportPart;
    let Some(notes) = doc.project().notes_window(id) else { return };
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        for part in doc.project().report_parts(&notes.text) {
            match part {
                ReportPart::Text(t) => {
                    for line in t.lines() {
                        if let Some(h) = line.strip_prefix("# ") {
                            ui.heading(h);
                        } else if let Some(h) = line.strip_prefix("## ").or_else(|| line.strip_prefix("### ")) {
                            ui.strong(h);
                        } else {
                            ui.label(line);
                        }
                    }
                }
                ReportPart::Figure { name, graph, caption } => {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        // The graph itself (T459), as wide as the window allows up to FIGURE_WIDTH.
                        let width = ui.available_width().clamp(120.0, FIGURE_WIDTH);
                        match graph.and_then(|g| figure_texture(ui, doc, g, width, &mut state.figures)) {
                            Some(t) => {
                                let [w, h] = t.size();
                                let size = egui::vec2(width, width * h as f32 / w.max(1) as f32);
                                ui.add(egui::Image::new((t.id(), size)).alt_text(format!("Figure: {name}")));
                            }
                            None => {
                                ui.label(format!("Figure: no graph is called {name}"));
                            }
                        }
                        ui.label(egui::RichText::new(caption).italics());
                    });
                }
            }
        }
    });
}

pub fn show(ui: &mut egui::Ui, doc: &Document, id: WindowId, state: &mut NotesUi, actions: &mut Vec<Action>) {
    let Some(notes) = doc.project().notes_window(id) else { return };
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.preview, false, "Edit");
        ui.selectable_value(&mut state.preview, true, "Preview");
        if ui.button("Export Report...").clicked() {
            actions.push(Action::ExportReport(id));
        }
    });
    if state.preview {
        preview(ui, doc, id, state);
        return;
    }
    // An undo or redo that changed the text meanwhile wins over what was typed before it.
    if state.editing.as_ref().is_some_and(|(base, _)| *base != notes.text) {
        state.editing = None;
    }
    let (base, text) = state.editing.get_or_insert_with(|| (notes.text.clone(), notes.text.clone()));
    let edit = egui::TextEdit::multiline(text)
        .id_salt(("notes_window", id.0))
        .desired_width(f32::INFINITY)
        .font(egui::TextStyle::Monospace);
    let r = egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| ui.add_sized(ui.available_size(), edit))
        .inner;
    if !r.has_focus() {
        if *text != *base {
            actions.push(Action::SetNotesText { id, text: text.clone() });
        }
        state.editing = None;
    }
}
