//! The Object Manager: the layers and plots of the active graph as a tree with a tick box on each
//! node. A tick box on a plot shows or hides that plot in the graph (`Plot::visible`), one on a
//! layer shows or hides all of its plots. Clicking a plot selects it, and the selection is the
//! one of the Plot Details window while that is open on the graph; a double click opens Plot
//! Details at that plot.

use super::PanelData;
use crate::actions::Action;
use crate::dialogs::graph_props::PropsTab;
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use sheaf_core::graph::{Graph, Plot};
use sheaf_core::{Project, WindowId};

const ROW_H: f32 = 18.0;
const INDENT: f32 = 14.0;

#[derive(Default)]
pub struct ObjectsState {
    /// The selected node: a layer and, inside it, a plot; `None` for the page.
    pub selected: Option<(usize, Option<usize>)>,
}

pub fn show(ui: &mut egui::Ui, st: &mut ObjectsState, data: &PanelData, actions: &mut Vec<Action>) {
    let Some((graph_id, graph)) = data.active.and_then(|id| data.project.graph(id).map(|g| (id, g))) else {
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Activate a graph window to see its layers and plots").color(Color32::GRAY));
        return;
    };
    // While Plot Details is open on this graph its selection is the selection.
    if let Some(p) = data.props_plot {
        st.selected = Some((0, Some(p)));
    }
    egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        // The graph itself.
        ui.horizontal(|ui| {
            ui.set_height(ROW_H);
            let (r, _) = ui.allocate_exact_size(Vec2::new(16.0, ROW_H), Sense::hover());
            paint_folder(ui.painter(), Rect::from_center_size(r.center(), Vec2::splat(12.0)));
            let mut r = ui.selectable_label(st.selected.is_none(), graph.name.as_str());
            if !graph.page.style.comments.is_empty() {
                r = r.on_hover_text(graph.page.style.comments.as_str());
            }
            if r.clicked() {
                st.selected = None;
            }
            if r.double_clicked() {
                actions.push(Action::GraphProperties { graph: graph_id, layer: 0, tab: PropsTab::Page, plot: None });
            }
        });
        for (li, layer) in graph.layers.iter().enumerate() {
            layer_row(ui, st, data.project, graph_id, li, &layer.plots, actions);
            for (pi, plot) in layer.plots.iter().enumerate() {
                plot_row(ui, st, data.project, graph_id, li, pi, plot, actions);
            }
        }
    });
}

fn layer_row(
    ui: &mut egui::Ui,
    st: &mut ObjectsState,
    _project: &Project,
    graph: WindowId,
    layer: usize,
    plots: &[Plot],
    actions: &mut Vec<Action>,
) {
    let name = format!("Layer{}", layer + 1);
    let shown = plots.is_empty() || plots.iter().any(|p| p.visible);
    ui.horizontal(|ui| {
        ui.set_height(ROW_H);
        ui.add_space(INDENT);
        if tick(ui, shown, &format!("Show layer: {name}")) && !plots.is_empty() {
            actions.push(Action::SetLayerVisible { graph, layer, visible: !shown });
        }
        let selected = st.selected == Some((layer, None));
        let r = ui.selectable_label(selected, name.as_str());
        if r.clicked() {
            st.selected = Some((layer, None));
            actions.push(Action::ActivateLayer { graph, layer });
        }
        if r.double_clicked() {
            actions.push(Action::GraphProperties { graph, layer, tab: PropsTab::Plots, plot: None });
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn plot_row(
    ui: &mut egui::Ui,
    st: &mut ObjectsState,
    project: &Project,
    graph: WindowId,
    layer: usize,
    index: usize,
    plot: &Plot,
    actions: &mut Vec<Action>,
) {
    let label = project.plot_label(plot);
    ui.horizontal(|ui| {
        ui.set_height(ROW_H);
        ui.add_space(INDENT * 2.0);
        if tick(ui, plot.visible, &format!("Show plot: {label}")) {
            actions.push(Action::SetPlotVisible { graph, layer, plot: index, visible: !plot.visible });
        }
        sample(ui, plot);
        let selected = st.selected == Some((layer, Some(index)));
        let r = ui.selectable_label(selected, label.as_str());
        if r.clicked() {
            st.selected = Some((layer, Some(index)));
            actions.push(Action::SelectPlot { graph, layer, plot: index });
        }
        if r.double_clicked() {
            actions.push(Action::GraphProperties { graph, layer, tab: PropsTab::Plots, plot: Some(index) });
        }
    });
}

/// A tick box; true when it was clicked.
fn tick(ui: &mut egui::Ui, checked: bool, label: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(16.0, ROW_H), Sense::click());
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, checked, label));
    let b = Rect::from_center_size(rect.center(), Vec2::splat(12.0));
    let p = ui.painter();
    p.rect_filled(b, 0.0, Color32::WHITE);
    let edge = if response.hovered() { Color32::from_rgb(0, 84, 153) } else { Color32::from_rgb(51, 102, 160) };
    p.rect_stroke(b, 0.0, Stroke::new(1.0_f32, edge), egui::StrokeKind::Inside);
    if checked {
        let c = b.center();
        let s = Stroke::new(1.6_f32, Color32::from_rgb(30, 90, 170));
        p.line_segment([c + Vec2::new(-3.0, 0.0), c + Vec2::new(-1.0, 2.5)], s);
        p.line_segment([c + Vec2::new(-1.0, 2.5), c + Vec2::new(3.5, -3.0)], s);
    }
    response.on_hover_text(label).clicked()
}

/// The little picture of the plot's line or symbols, in its colour.
fn sample(ui: &mut egui::Ui, plot: &Plot) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(30.0, ROW_H), Sense::hover());
    let c = plot.line.color;
    let color = Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a);
    let y = rect.center().y;
    let p = ui.painter();
    if plot.kind.has_line() {
        p.line_segment(
            [Pos2::new(rect.left() + 1.0, y), Pos2::new(rect.right() - 1.0, y)],
            Stroke::new(1.5_f32, color),
        );
    }
    if plot.kind.has_symbols() || !plot.kind.has_line() {
        let s = plot.symbol.color;
        let sc = Color32::from_rgba_unmultiplied(s.r, s.g, s.b, s.a);
        p.rect_filled(Rect::from_center_size(rect.center(), Vec2::splat(6.0)), 0.0, sc);
    }
}

fn paint_folder(painter: &egui::Painter, r: Rect) {
    let edge = Stroke::new(1.0_f32, Color32::from_rgb(160, 130, 40));
    let fill = Color32::from_rgb(250, 215, 110);
    let tab = Rect::from_min_size(r.min, Vec2::new(r.width() * 0.45, 3.0));
    painter.rect_filled(tab, 0.0, fill);
    painter.rect_stroke(tab, 0.0, edge, egui::StrokeKind::Inside);
    let body = Rect::from_min_max(Pos2::new(r.min.x, r.min.y + 2.0), r.max);
    painter.rect_filled(body, 0.0, fill);
    painter.rect_stroke(body, 0.0, edge, egui::StrokeKind::Inside);
}

/// Shows or hides one plot, as one undo step.
pub fn set_plot_visible(project: &mut Project, graph: WindowId, layer: usize, plot: usize, visible: bool) {
    if let Some(p) = project.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)).and_then(|l| l.plots.get_mut(plot))
    {
        p.visible = visible;
    }
}

/// Shows or hides every plot of a layer.
pub fn set_layer_visible(project: &mut Project, graph: WindowId, layer: usize, visible: bool) {
    if let Some(l) = project.graph_mut(graph).and_then(|g: &mut Graph| g.layers.get_mut(layer)) {
        l.plots.iter_mut().for_each(|p| p.visible = visible);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::ColumnData;
    use sheaf_core::template::Template;

    #[test]
    fn plots_and_layers_can_be_hidden_and_shown() {
        let mut p = Project::new();
        let book = p.add_workbook("w");
        let x = p.add_column(book, 0, sheaf_core::Designation::X, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
        let y = p.add_column(book, 0, sheaf_core::Designation::Y, ColumnData::numeric(vec![3.0, 4.0])).unwrap();
        let _ = x;
        let g = p.add_graph(Template::default());
        p.add_plot(g, 0, y).unwrap();
        set_plot_visible(&mut p, g, 0, 0, false);
        assert!(!p.graph(g).unwrap().layers[0].plots[0].visible);
        set_layer_visible(&mut p, g, 0, true);
        assert!(p.graph(g).unwrap().layers[0].plots[0].visible);
        set_plot_visible(&mut p, g, 3, 9, false); // no such plot: nothing happens
    }
}
