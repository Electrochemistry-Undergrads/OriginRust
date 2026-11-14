//! The graph dialogs in Origin's layout (ADR 0007 step 9): the **axis dialog** ("X Axis - Layer 1")
//! with a list at the left (Horizontal, Vertical) and the tabs Scale, Tick Labels, Title, Grids,
//! Line and Ticks, Special Ticks, Breaks; and **Plot Details** with a tree at the left (graph,
//! layers, plots) and the tabs of what is chosen. Both end in the buttons OK, Cancel, Apply.
//!
//! Every change is applied to the project at once, so the figure behind the window always shows the
//! current settings, and all changes made while the window is open form one undo step. Because the
//! changes are live, the buttons mean:
//!
//! * **Apply** keeps what has been done so far as the new starting point of Cancel, and the next
//!   changes form a new undo step. It is grey while nothing differs from the starting point.
//! * **OK** closes the window and keeps the changes.
//! * **Cancel** (and the close button of the window) takes the graph back to its state at the
//!   opening of the window, or at the last Apply: it undoes the window's step, so no undo entry
//!   is left behind.

use eframe::egui::{self, Align, Color32, DragValue, Layout, RichText};
use sheaf_core::graph::*;
use sheaf_core::{Color, ColumnId, Document, Project, WindowId};

use super::graph_axis::{self, AxisTab};
use super::graph_details::{self, DetailTab, Node};

/// The undo label of the dialog's edits.
const LABEL: &str = "Change graph settings";

/// Where the older entry points (menus, toolbar, Object Manager, the graph window) want to go.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PropsTab {
    /// The axis dialog on the horizontal axis.
    XAxis,
    /// The axis dialog on the vertical axis.
    YAxis,
    /// Plot Details on a plot (when one is given) or on the layer (Layer Contents).
    Plots,
    /// Plot Details on the legend settings of the layer.
    Legend,
    /// Plot Details on the page of the graph.
    Page,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum View {
    Axis { vertical: bool, tab: AxisTab },
    Details { node: Node, tab: DetailTab },
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Button {
    Ok,
    Cancel,
    Apply,
}

pub struct GraphPropsDialog {
    pub graph: WindowId,
    /// The layer whose axes, plots and legend the dialog edits.
    pub layer: usize,
    view: View,
    /// Groups all edits since the opening (or the last Apply) into one undo step.
    token: u64,
    /// The graph as Cancel gives it back: at the opening, or at the last Apply.
    baseline: Option<Graph>,
    /// True when an edit was made with the current token.
    touched: bool,
    /// The plot of the dialog's layer that was chosen last.
    selected_plot: usize,
    /// The sheet chosen in "Add data to this graph" (index into the list of sheets with Y columns).
    add_from: usize,
}

/// A Y column that can be added to a graph, with the X and error columns its sheet gives it.
pub struct Addable {
    pub y: ColumnId,
    pub x: Option<ColumnId>,
    pub y_error: Option<ColumnId>,
    pub label: String,
}

/// One worksheet and its numeric Y columns.
pub struct Source {
    pub title: String,
    pub columns: Vec<Addable>,
}

fn sources(project: &Project) -> Vec<Source> {
    let mut out = Vec::new();
    for book in project.workbooks() {
        for sheet in &book.sheets {
            let columns: Vec<Addable> = sheet
                .y_columns()
                .into_iter()
                .filter(|i| sheet.columns[*i].data.is_numeric())
                .map(|i| {
                    let c = &sheet.columns[i];
                    let mut label = format!("{}: {}", c.short_name, c.axis_title());
                    if let Some(first) = c.comments.lines().next().filter(|l| !l.is_empty()) {
                        label.push_str(&format!(" [{first}]"));
                    }
                    Addable {
                        y: c.id,
                        x: sheet.x_for(i).map(|x| x.id),
                        y_error: sheet.y_error_for(i).map(|e| e.id),
                        label,
                    }
                })
                .collect();
            if !columns.is_empty() {
                out.push(Source { title: format!("{} / {}", book.title(), sheet.name), columns });
            }
        }
    }
    out
}

/// Every column of the project with a name for lists.
fn all_columns(project: &Project) -> Vec<(ColumnId, String)> {
    let mut out = Vec::new();
    for book in project.workbooks() {
        for sheet in &book.sheets {
            for c in &sheet.columns {
                out.push((c.id, format!("{} / {}: {}", book.title(), sheet.name, c.short_name)));
            }
        }
    }
    out
}

impl GraphPropsDialog {
    pub fn new(graph: WindowId, layer: usize, tab: PropsTab, token: u64) -> Self {
        let view = match tab {
            PropsTab::XAxis => View::Axis { vertical: false, tab: AxisTab::Scale },
            PropsTab::YAxis => View::Axis { vertical: true, tab: AxisTab::Scale },
            PropsTab::Plots => View::Details { node: Node::Layer(layer), tab: DetailTab::LayerContents },
            PropsTab::Legend => View::Details { node: Node::Layer(layer), tab: DetailTab::Legend },
            PropsTab::Page => View::Details { node: Node::Graph, tab: DetailTab::Page },
        };
        GraphPropsDialog { graph, layer, view, token, baseline: None, touched: false, selected_plot: 0, add_from: 0 }
    }

    /// Which of the entry points this dialog is at now.
    #[cfg(test)]
    pub fn tab(&self) -> PropsTab {
        match self.view {
            View::Axis { vertical: false, .. } => PropsTab::XAxis,
            View::Axis { vertical: true, .. } => PropsTab::YAxis,
            View::Details { node: Node::Graph, .. } => PropsTab::Page,
            View::Details { node: Node::Layer(_), tab: DetailTab::Legend } => PropsTab::Legend,
            View::Details { .. } => PropsTab::Plots,
        }
    }

    /// The plot of the dialog's layer that Plot Details has chosen last.
    pub fn selected_plot(&self) -> usize {
        self.selected_plot
    }

    /// Chooses a plot of the dialog's layer (the Object Manager and the graph window do this
    /// when a plot is clicked) and, in Plot Details, shows its node.
    pub fn select_plot(&mut self, plot: usize) {
        self.selected_plot = plot;
        if let View::Details { node, tab } = &mut self.view {
            *node = Node::Plot(self.layer, plot);
            // The tab is made valid for the plot while drawing.
            if matches!(*tab, DetailTab::LayerContents | DetailTab::Page) {
                *tab = DetailTab::Line;
            }
        }
    }

    fn fresh_token(&mut self) {
        // The application's own tokens count up from 1; this keeps clear of them.
        self.token = self.token.wrapping_add(1 << 40);
        self.touched = false;
    }

    /// Takes the graph back to the baseline: one undo step when the window's step is the last one,
    /// otherwise by writing the saved copy.
    fn cancel(&mut self, doc: &mut Document) {
        let Some(base) = self.baseline.take() else {
            return;
        };
        if !self.touched {
            return;
        }
        if doc.undo_label() == Some(LABEL) {
            doc.undo();
        }
        if doc.project().graph(self.graph).is_some_and(|g| *g != base) {
            let id = self.graph;
            doc.apply("Restore graph settings", move |p| {
                if let Some(g) = p.graph_mut(id) {
                    *g = base;
                }
            });
        }
    }

    /// Draws the window. Returns `false` when it was closed.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> bool {
        let Some(original) = doc.project().graph(self.graph) else {
            return false;
        };
        if original.layers.is_empty() {
            return false;
        }
        if self.baseline.is_none() {
            self.baseline = Some(original.clone());
        }
        let mut graph = original.clone();
        let li = self.layer.min(graph.layers.len() - 1);
        self.layer = li;
        let geometry = sheaf_plot::layout_geometry(doc.project(), self.graph).ok();
        let labels: Vec<Vec<String>> =
            graph.layers.iter().map(|l| l.plots.iter().map(|p| doc.project().plot_label(p)).collect()).collect();
        let sources = sources(doc.project());
        let columns = all_columns(doc.project());

        let (title, window_id, size) = match self.view {
            View::Axis { vertical, .. } => (
                format!("{} Axis - Layer {}", if vertical { "Y" } else { "X" }, li + 1),
                "graph_axis_dialog",
                [600.0_f32, 540.0],
            ),
            View::Details { node, .. } => (
                format!(
                    "Plot Details - {}",
                    match node {
                        Node::Graph => "Page Properties",
                        Node::Layer(_) => "Layer Properties",
                        Node::Plot(..) => "Plot Properties",
                    }
                ),
                "graph_plot_details",
                [900.0_f32, 560.0],
            ),
        };

        // Centred in the free area between the docked panels, and no wider than it.
        let avail = ctx.available_rect();
        let size = [size[0].min(avail.width() - 16.0).max(480.0), size[1].min(avail.height() - 16.0).max(360.0)];
        let pos = (avail.center() - egui::vec2(size[0], size[1]) / 2.0).max(avail.min);
        let mut open = true;
        let mut button: Option<Button> = None;
        let this = &mut *self;
        egui::Window::new(title)
            .id(egui::Id::new(window_id))
            .open(&mut open)
            .default_pos(pos)
            .default_size(size)
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                let body_h = (ui.available_height() - 46.0).max(240.0);
                ui.allocate_ui(egui::vec2(ui.available_width(), body_h), |ui| match &mut this.view {
                    View::Axis { vertical, tab } => {
                        let resolved =
                            geometry.as_ref().and_then(|g| g.get(li)).map(|g| if *vertical { &g.y } else { &g.x });
                        graph_axis::dialog_body(ui, vertical, tab, &mut graph, li, resolved, &columns);
                    }
                    View::Details { node, tab } => {
                        graph_details::dialog_body(
                            ui,
                            node,
                            tab,
                            &mut graph,
                            &labels,
                            &sources,
                            &columns,
                            &mut this.add_from,
                        );
                    }
                });
                ui.separator();
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let dirty = this.baseline.as_ref().is_some_and(|b| *b != graph);
                    // Right to left: Apply is drawn first so that the order reads OK, Cancel, Apply.
                    if ui.add_enabled(dirty, egui::Button::new("Apply")).clicked() {
                        button = Some(Button::Apply);
                    }
                    if ui.button("Cancel").clicked() {
                        button = Some(Button::Cancel);
                    }
                    if ui.button("OK").clicked() {
                        button = Some(Button::Ok);
                    }
                });
            });

        // Which plot and layer the dialog is on, for the Object Manager and the next opening.
        if let View::Details { node, .. } = self.view {
            match node {
                Node::Plot(l, p) => {
                    self.layer = l;
                    self.selected_plot = p;
                }
                Node::Layer(l) => self.layer = l,
                Node::Graph => {}
            }
        }

        let changed = graph != *doc.project().graph(self.graph).expect("checked above");
        if changed {
            let id = self.graph;
            let edited = graph.clone();
            doc.apply_merging(LABEL, self.token, move |p| {
                if let Some(g) = p.graph_mut(id) {
                    *g = edited;
                }
            });
            self.touched = true;
        }

        match button {
            Some(Button::Ok) => return false,
            Some(Button::Cancel) => {
                self.cancel(doc);
                return false;
            }
            Some(Button::Apply) => {
                // What is shown now becomes the starting point of Cancel; later edits are a new step.
                self.baseline = doc.project().graph(self.graph).cloned();
                self.fresh_token();
            }
            None => {}
        }
        if !open {
            self.cancel(doc);
            return false;
        }
        true
    }
}

// ----- shared widgets of the two dialogs ----------------------------------------------------

pub(super) fn grid<R>(ui: &mut egui::Ui, id: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Grid::new(id).num_columns(2).spacing([14.0, 6.0]).show(ui, add).inner
}

pub(super) fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(8.0);
    ui.label(RichText::new(title).strong());
}

/// A group box: a thin frame with its title above, as Origin draws them.
pub(super) fn group_box<R>(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.add_space(4.0);
    ui.label(RichText::new(title).strong());
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        add(ui)
    })
    .inner
}

/// A row of tabs. Each is (value, name, enabled); a disabled one is shown grey.
pub(super) fn tab_row<T: Copy + PartialEq>(ui: &mut egui::Ui, current: &mut T, tabs: &[(T, &str, bool)]) {
    ui.horizontal_wrapped(|ui| {
        for (value, name, enabled) in tabs {
            let button = egui::Button::selectable(*current == *value, *name);
            if ui.add_enabled(*enabled, button).clicked() {
                *current = *value;
            }
        }
    });
}

/// A number field that can be switched between "automatic" and a fixed value.
pub(super) fn optional_number(ui: &mut egui::Ui, value: &mut Option<f64>, auto_value: Option<f64>, speed: f64) {
    ui.horizontal(|ui| {
        let mut auto = value.is_none();
        if ui.checkbox(&mut auto, "Auto").changed() {
            *value = if auto { None } else { Some(auto_value.unwrap_or(0.0)) };
        }
        match value {
            Some(v) => {
                ui.add(DragValue::new(v).speed(speed).max_decimals(8));
            }
            None => {
                let shown = auto_value.map(|v| sheaf_core::format::format_general(v, 6)).unwrap_or_default();
                ui.weak(shown);
            }
        }
    });
}

pub fn color_button(ui: &mut egui::Ui, color: &mut Color) {
    let mut c = Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a);
    if ui.color_edit_button_srgba(&mut c).changed() {
        let [r, g, b, a] = c.to_srgba_unmultiplied();
        *color = Color::rgba(r, g, b, a);
    }
}

pub(super) fn font_ui(ui: &mut egui::Ui, font: &mut Font) {
    ui.horizontal(|ui| {
        ui.add(DragValue::new(&mut font.size_pt).range(4.0..=96.0).speed(0.25).suffix(" pt"));
        ui.checkbox(&mut font.bold, "Bold");
        ui.checkbox(&mut font.italic, "Italic");
        color_button(ui, &mut font.color);
    });
}

pub(super) fn tick_direction_ui(ui: &mut egui::Ui, id: &str, dir: &mut TickDirection) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(match dir {
            TickDirection::In => "In",
            TickDirection::Out => "Out",
            TickDirection::Both => "In and out",
            TickDirection::None => "None",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(dir, TickDirection::In, "In");
            ui.selectable_value(dir, TickDirection::Out, "Out");
            ui.selectable_value(dir, TickDirection::Both, "In and out");
            ui.selectable_value(dir, TickDirection::None, "None");
        });
}

pub(super) fn dash_name(d: Dash) -> &'static str {
    match d {
        Dash::Solid => "Solid",
        Dash::Dash => "Dash",
        Dash::Dot => "Dot",
        Dash::DashDot => "Dash dot",
        Dash::DashDotDot => "Dash dot dot",
        Dash::ShortDash => "Short dash",
        Dash::ShortDot => "Short dot",
        Dash::ShortDashDot => "Short dash dot",
    }
}

pub(super) fn dash_ui(ui: &mut egui::Ui, id: &str, dash: &mut Dash) {
    egui::ComboBox::from_id_salt(id).selected_text(dash_name(*dash)).show_ui(ui, |ui| {
        for d in Dash::ALL {
            ui.selectable_value(dash, d, dash_name(d));
        }
    });
}

pub(super) fn connect_name(c: Connect) -> &'static str {
    match c {
        Connect::Straight => "Straight",
        Connect::Spline => "Spline",
        Connect::BSpline => "B-spline",
        Connect::Bezier => "Bezier",
        Connect::StepHorizontal => "Step, horizontal first",
        Connect::StepVertical => "Step, vertical first",
    }
}

pub(super) fn symbol_name(s: SymbolShape) -> &'static str {
    match s {
        SymbolShape::Square => "Square",
        SymbolShape::Circle => "Circle",
        SymbolShape::TriangleUp => "Triangle up",
        SymbolShape::TriangleDown => "Triangle down",
        SymbolShape::Diamond => "Diamond",
        SymbolShape::Plus => "Plus",
        SymbolShape::Cross => "Cross",
        SymbolShape::Star => "Star",
    }
}
