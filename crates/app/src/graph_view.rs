//! A graph window: shows the figure exactly as it will be exported.
//!
//! The figure is rasterised by `sheaf-plot` (the same code path as PNG
//! export) at the resolution of the window and shown as an image. It is
//! rendered again only when the project, the size or the zoom changes.

use crate::actions::Action;
use crate::dialogs::graph_props::PropsTab;
use eframe::egui::{self, Color32, Sense, TextureHandle, TextureOptions};
use sheaf_core::{Document, WindowId};
use sheaf_plot::LayerGeometry;

/// The cached image of one graph.
pub struct GraphTexture {
    revision: u64,
    /// Raster pixels per page point, quantised, so tiny size changes do not re-render.
    scale_key: u32,
    texture: TextureHandle,
    geometry: Vec<LayerGeometry>,
}

#[derive(Default)]
pub struct GraphUi {
    pub texture: Option<GraphTexture>,
    pub error: Option<String>,
}

pub fn show(ui: &mut egui::Ui, doc: &Document, graph_id: WindowId, state: &mut GraphUi, actions: &mut Vec<Action>) {
    let Some(graph) = doc.project().graph(graph_id) else {
        ui.label("This graph no longer exists.");
        return;
    };

    let mut readout = String::new();
    egui::TopBottomPanel::top(ui.id().with("graph_bar")).show_inside(ui, |ui| {
        ui.horizontal(|ui| {
            if ui.button("Axes...").clicked() {
                actions.push(Action::GraphProperties { graph: graph_id, tab: PropsTab::XAxis });
            }
            if ui.button("Plots...").clicked() {
                actions.push(Action::GraphProperties { graph: graph_id, tab: PropsTab::Plots });
            }
            if ui.button("Legend...").clicked() {
                actions.push(Action::GraphProperties { graph: graph_id, tab: PropsTab::Legend });
            }
            if ui.button("Page...").clicked() {
                actions.push(Action::GraphProperties { graph: graph_id, tab: PropsTab::Page });
            }
            ui.separator();
            if ui.button("Export...").clicked() {
                actions.push(Action::ExportGraph(graph_id));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.weak(format!("{:.2} x {:.2} in", graph.page.width_pt / 72.0, graph.page.height_pt / 72.0));
            });
        });
    });

    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(Color32::from_gray(225))).show_inside(ui, |ui| {
        let avail = ui.available_size();
        let margin = 12.0;
        let page_w = graph.page.width_pt as f32;
        let page_h = graph.page.height_pt as f32;
        if page_w <= 0.0 || page_h <= 0.0 || avail.x <= 2.0 * margin || avail.y <= 2.0 * margin {
            return;
        }
        // UI points per page point, so that the whole page fits in the window.
        let fit = ((avail.x - 2.0 * margin) / page_w).min((avail.y - 2.0 * margin) / page_h).max(0.05);
        let size = egui::vec2(page_w * fit, page_h * fit);
        let (rect, response) = ui.allocate_exact_size(avail, Sense::click());
        let image_rect = egui::Rect::from_center_size(rect.center(), size);

        // Raster pixels per page point.
        let pixel_scale = fit * ui.ctx().pixels_per_point();
        let scale_key = (pixel_scale * 50.0).round() as u32;
        let stale =
            state.texture.as_ref().is_none_or(|t| t.revision != doc.revision() || t.scale_key != scale_key);
        if stale {
            let scale = f64::from(scale_key) / 50.0;
            match sheaf_plot::render_raster(doc.project(), graph_id, scale)
                .and_then(|raster| Ok((raster, sheaf_plot::layout_geometry(doc.project(), graph_id)?)))
            {
                Ok((raster, geometry)) => {
                    let image = egui::ColorImage::from_rgba_unmultiplied(
                        [raster.width as usize, raster.height as usize],
                        &raster.rgba,
                    );
                    let texture =
                        ui.ctx().load_texture(format!("graph-{}", graph_id.0), image, TextureOptions::LINEAR);
                    state.texture = Some(GraphTexture { revision: doc.revision(), scale_key, texture, geometry });
                    state.error = None;
                }
                Err(e) => state.error = Some(e.to_string()),
            }
        }

        // A soft shadow and the page itself.
        ui.painter().rect_filled(image_rect.translate(egui::vec2(3.0, 3.0)), 0.0, Color32::from_black_alpha(40));
        if let Some(t) = &state.texture {
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            ui.painter().image(t.texture.id(), image_rect, uv, Color32::WHITE);
        }
        if let Some(e) = &state.error {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("This graph could not be drawn:\n{e}"),
                egui::FontId::proportional(14.0),
                Color32::DARK_RED,
            );
        }

        // Mouse: data read-out, and double-click to open the matching settings.
        let Some(t) = &state.texture else {
            return;
        };
        let to_page = |p: egui::Pos2| (f64::from((p.x - image_rect.left()) / fit), f64::from((p.y - image_rect.top()) / fit));
        if let Some(pos) = response.hover_pos() {
            let (px, py) = to_page(pos);
            for g in &t.geometry {
                let f = &g.frame;
                if px >= f.x && px <= f.x + f.w && py >= f.y && py <= f.y + f.h {
                    let (x, y) = g.page_to_data(px, py);
                    readout = format!("x = {}   y = {}", short(x), short(y));
                }
            }
        }
        if response.double_clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let (px, py) = to_page(pos);
            let tab = match t.geometry.first() {
                Some(g) if py > g.frame.y + g.frame.h => PropsTab::XAxis,
                Some(g) if px < g.frame.x => PropsTab::YAxis,
                Some(_) => PropsTab::Plots,
                None => PropsTab::Page,
            };
            actions.push(Action::GraphProperties { graph: graph_id, tab });
        }
        response.context_menu(|ui| {
            if ui.button("Axes...").clicked() {
                actions.push(Action::GraphProperties { graph: graph_id, tab: PropsTab::XAxis });
                ui.close();
            }
            if ui.button("Plots...").clicked() {
                actions.push(Action::GraphProperties { graph: graph_id, tab: PropsTab::Plots });
                ui.close();
            }
            if ui.button("Legend...").clicked() {
                actions.push(Action::GraphProperties { graph: graph_id, tab: PropsTab::Legend });
                ui.close();
            }
            ui.separator();
            if ui.button("Export...").clicked() {
                actions.push(Action::ExportGraph(graph_id));
                ui.close();
            }
        });
        if !readout.is_empty() {
            ui.painter().text(
                rect.left_bottom() + egui::vec2(8.0, -6.0),
                egui::Align2::LEFT_BOTTOM,
                &readout,
                egui::FontId::monospace(12.0),
                Color32::from_gray(60),
            );
        }
    });
}

fn short(v: f64) -> String {
    sheaf_core::format::format_general(v, 5)
}
