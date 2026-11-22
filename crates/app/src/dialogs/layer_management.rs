//! Graph > Layer Management... (T60), after Origin 2016's window of that name: the layers of the
//! graph in a grid at the left, tabs for Add, Arrange, Size/Position, Link, Axes and Display in the
//! middle and a preview of the page at the right. Every Apply changes a working copy of the graph
//! (the preview); Undo takes the last Apply back in the preview; OK writes the working copy to the
//! graph as one undo step; Cancel, Escape and closing the window drop it.

use std::collections::BTreeSet;

use eframe::egui::{self, Color32, Sense, Stroke};
use sheaf_core::graph::{Graph, Layer, ScaleLink};
use sheaf_core::layer_manage::{AlignEdge, ArrangeOptions, ArrangeOrder, SizeUnit};
use sheaf_core::{Document, Project, WindowId};

mod background;
use background::BackgroundDraft;

const HINT: &str = "Use the Apply buttons on each tab to update your preview graph. Use the OK button to apply the changes in preview graph to source graph.";
const NOT_YET: &str = "Not available in Sheaf yet";

/// What the application should do after a frame of the window.
pub enum Outcome {
    Open,
    Closed,
    /// OK: the working copy was written to the graph.
    Applied,
}

#[derive(Copy, Clone, PartialEq)]
enum Tab {
    Add,
    Arrange,
    SizePosition,
    Link,
    Axes,
    Display,
}

impl Tab {
    const ALL: [(Tab, &'static str); 6] = [
        (Tab::Add, "Add"),
        (Tab::Arrange, "Arrange"),
        (Tab::SizePosition, "Size/Position"),
        (Tab::Link, "Link"),
        (Tab::Axes, "Axes"),
        (Tab::Display, "Display"),
    ];
}

#[derive(Copy, Clone, PartialEq)]
enum SizeMode {
    Resize,
    Move,
    Swap,
    Align,
}

pub struct LayerManagementDialog {
    graph: WindowId,
    /// The preview: a copy of the graph that the Apply buttons change.
    work: Graph,
    /// The source layers at opening: OK must not overwrite an edit made behind this window.
    source_layers: Vec<Layer>,
    /// The layers before each Apply, newest last.
    history: Vec<Vec<Layer>>,
    selected: BTreeSet<usize>,
    tab: Tab,
    arrange: ArrangeOptions,
    common_width: bool,
    common_height: bool,
    mode: SizeMode,
    unit: SizeUnit,
    reference: usize,
    width: f64,
    height: f64,
    left: f64,
    top: f64,
    swap_a: Option<usize>,
    swap_b: Option<usize>,
    reorder: bool,
    edge: AlignEdge,
    link_to: Option<usize>,
    link_x: ScaleLink,
    link_y: ScaleLink,
    /// The layer whose link the Link tab shows (the first selected one when the tab was drawn).
    link_shown_for: Option<usize>,
    background: BackgroundDraft,
    problem: Option<String>,
}

impl LayerManagementDialog {
    /// The window for `graph`, with `layer` selected when it is given.
    pub fn new(project: &Project, graph: WindowId, layer: Option<usize>) -> Option<Self> {
        let work = project.graph(graph)?.clone();
        if work.layers.is_empty() {
            return None;
        }
        let selected = layer.filter(|&l| l < work.layers.len()).into_iter().collect();
        Some(Self {
            graph,
            source_layers: work.layers.clone(),
            work,
            history: Vec::new(),
            selected,
            tab: Tab::Arrange,
            arrange: ArrangeOptions::default(),
            common_width: false,
            common_height: false,
            mode: SizeMode::Resize,
            unit: SizeUnit::ReferencePercent,
            reference: 0,
            width: 100.0,
            height: 100.0,
            left: 0.0,
            top: 0.0,
            swap_a: None,
            swap_b: None,
            reorder: false,
            edge: AlignEdge::Left,
            link_to: None,
            link_x: ScaleLink::None,
            link_y: ScaleLink::None,
            link_shown_for: None,
            background: BackgroundDraft::default(),
            problem: None,
        })
    }

    fn selection(&self) -> Vec<usize> {
        self.selected.iter().copied().collect()
    }

    /// Runs one Apply on the preview; a refused one leaves the preview as it was.
    fn apply(&mut self, change: impl FnOnce(&mut Graph) -> Result<(), sheaf_core::layer_manage::LayerError>) {
        let before = self.work.layers.clone();
        match change(&mut self.work) {
            Ok(()) => {
                self.history.push(before);
                self.problem = None;
            }
            Err(e) => self.problem = Some(e.to_string()),
        }
    }

    fn undo(&mut self) {
        if let Some(layers) = self.history.pop() {
            self.work.layers = layers;
            self.background.invalidate();
            self.problem = None;
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Outcome::Closed;
        }
        // The graph was deleted, or its layers changed behind the window.
        match doc.project().graph(self.graph) {
            Some(g) if self.history.is_empty() && g.layers.len() != self.work.layers.len() => {
                self.work = g.clone();
                self.source_layers = g.layers.clone();
                self.background.invalidate();
                self.selected.retain(|&i| i < self.work.layers.len());
            }
            Some(_) => {}
            None => return Outcome::Closed,
        }
        let mut open = true;
        let mut result = Outcome::Open;
        egui::Window::new("Layer Management")
            .id(egui::Id::new("layer_management"))
            .open(&mut open)
            .collapsible(false)
            .default_width(820.0)
            .default_pos(egui::pos2(40.0, 30.0))
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(190.0);
                        self.layer_grid(ui);
                    });
                    ui.vertical(|ui| {
                        ui.set_width(380.0);
                        ui.horizontal(|ui| {
                            for (tab, name) in Tab::ALL {
                                ui.selectable_value(&mut self.tab, tab, name);
                            }
                        });
                        ui.separator();
                        match self.tab {
                            Tab::Arrange => self.arrange_tab(ui, doc.project()),
                            Tab::SizePosition => self.size_tab(ui),
                            Tab::Link => self.link_tab(ui),
                            Tab::Display => self.display_tab(ui),
                            Tab::Add | Tab::Axes => unavailable(ui),
                        }
                    });
                    ui.vertical(|ui| self.preview(ui));
                });
                if let Some(e) = &self.problem {
                    ui.colored_label(ui.visuals().error_fg_color, e);
                }
                ui.separator();
                ui.add(egui::Label::new(HINT).wrap());
                ui.horizontal(|ui| {
                    if ui.add_enabled(!self.history.is_empty(), egui::Button::new("Undo")).clicked() {
                        self.undo();
                    }
                    if ui.button("OK").clicked() {
                        result = self.commit(doc);
                    }
                    if ui.button("Cancel").clicked() {
                        result = Outcome::Closed;
                    }
                });
            });
        if open { result } else { Outcome::Closed }
    }

    /// OK: writes the preview's layers to the graph as one undo step (nothing when no Apply was made).
    fn commit(&mut self, doc: &mut Document) -> Outcome {
        if self.history.is_empty() || self.work.layers == self.source_layers {
            return Outcome::Closed;
        }
        if doc.project().graph(self.graph).is_some_and(|g| g.layers != self.source_layers) {
            self.problem =
                Some("The graph layers changed while this window was open. Cancel and reopen Layer Management.".into());
            return Outcome::Open;
        }
        let layers = self.work.layers.clone();
        let id = self.graph;
        let done = doc.edit("Layer Management", |p| match p.graph_mut(id) {
            Some(g) => {
                g.layers = layers;
                Ok(())
            }
            None => Err("the graph is gone"),
        });
        match done {
            Ok(()) => Outcome::Applied,
            Err(e) => {
                self.problem = Some(e.to_string());
                Outcome::Open
            }
        }
    }

    fn layer_grid(&mut self, ui: &mut egui::Ui) {
        ui.strong("Layer Selection");
        egui::Grid::new("lm_layers").num_columns(3).striped(true).show(ui, |ui| {
            ui.strong("#");
            ui.strong("Name");
            ui.strong("Link to");
            ui.end_row();
            for i in 0..self.work.layers.len() {
                ui.label((i + 1).to_string());
                let on = self.selected.contains(&i);
                if ui.selectable_label(on, format!("Layer{}", i + 1)).clicked() {
                    if on {
                        self.selected.remove(&i);
                    } else {
                        self.selected.insert(i);
                    }
                }
                ui.label(self.work.link_parent(i).map_or(String::new(), |p| (p + 1).to_string()));
                ui.end_row();
            }
        });
    }

    fn arrange_tab(&mut self, ui: &mut egui::Ui, project: &Project) {
        let o = &mut self.arrange;
        ui.group(|ui| {
            ui.strong("Arrange Selected Layers");
            egui::Grid::new("lm_arrange").num_columns(2).show(ui, |ui| {
                let label = ui.label("Arrange Order");
                egui::ComboBox::from_id_salt("lm_order")
                    .selected_text(match o.order {
                        ArrangeOrder::LayerIndex => "Layer Index Order",
                        ArrangeOrder::CurrentPosition => "Current Position Order",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut o.order, ArrangeOrder::LayerIndex, "Layer Index Order");
                        ui.selectable_value(&mut o.order, ArrangeOrder::CurrentPosition, "Current Position Order");
                    })
                    .response
                    .labelled_by(label.id);
                ui.end_row();
                let label = ui.label("Number of Rows");
                ui.add(egui::DragValue::new(&mut o.rows).range(1..=64)).labelled_by(label.id);
                ui.end_row();
                let label = ui.label("Number of Columns");
                ui.add(egui::DragValue::new(&mut o.cols).range(1..=64)).labelled_by(label.id);
                ui.end_row();
            });
            ui.add_enabled(false, egui::Checkbox::new(&mut false, "Add Extra Layer(s) for Grid"));
            ui.add_enabled(
                !(self.common_width || self.common_height),
                egui::Checkbox::new(&mut o.keep_aspect, "Keep Layer Aspect Ratio"),
            );
            ui.checkbox(&mut o.group_linked, "Treat Linked Layers as a group");
            ui.checkbox(&mut o.link, "Link Layers");
            ui.checkbox(&mut self.common_width, "Set Layer Width by Common Scale");
            ui.checkbox(&mut self.common_height, "Set Layer Height by Common Scale");
            ui.checkbox(&mut o.hide_inner, "Show Axes Frame");
            ui.label("Overlapping axes/ticks are hidden");
        });
        ui.group(|ui| {
            ui.strong("Spacing (in % of Page Dimension)");
            egui::Grid::new("lm_spacing").num_columns(2).show(ui, |ui| {
                number(ui, "Horizontal Gap", &mut o.h_gap, 50.0);
                number(ui, "Vertical Gap", &mut o.v_gap, 50.0);
                number(ui, "Left Margin", &mut o.margin_left, 90.0);
                number(ui, "Right Margin", &mut o.margin_right, 90.0);
                number(ui, "Top Margin", &mut o.margin_top, 90.0);
                number(ui, "Bottom Margin", &mut o.margin_bottom, 90.0);
            });
        });
        if ui.button("Apply").clicked() {
            self.apply_arrangement(project);
        }
    }

    fn apply_arrangement(&mut self, project: &Project) {
        let mut opts = self.arrange.clone();
        if self.common_width || self.common_height {
            // Link edits already applied to the preview can change the effective axis ranges.
            let mut draft = project.clone();
            *draft.graph_mut(self.graph).expect("the managed graph exists") = self.work.clone();
            let geometry = match sheaf_plot::layout_geometry(&draft, self.graph) {
                Ok(geometry) => geometry,
                Err(e) => {
                    self.problem = Some(e.to_string());
                    return;
                }
            };
            if self.common_width {
                opts.common_x_spans = Some(geometry.iter().map(|layer| layer.x.max - layer.x.min).collect());
            }
            if self.common_height {
                opts.common_y_spans = Some(geometry.iter().map(|layer| layer.y.max - layer.y.min).collect());
            }
        }
        let sel = self.selection();
        self.apply(|g| g.arrange_layers(&sel, &opts));
    }

    fn size_tab(&mut self, ui: &mut egui::Ui) {
        let n = self.work.layers.len();
        self.reference = self.reference.min(n - 1);
        ui.group(|ui| {
            ui.strong("Option");
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.mode, SizeMode::Resize, "Resize");
                ui.radio_value(&mut self.mode, SizeMode::Move, "Move");
                ui.radio_value(&mut self.mode, SizeMode::Swap, "Swap");
                ui.radio_value(&mut self.mode, SizeMode::Align, "Align");
            });
        });
        match self.mode {
            SizeMode::Resize | SizeMode::Move => {
                egui::Grid::new("lm_size").num_columns(2).show(ui, |ui| {
                    let label = ui.label("Units");
                    egui::ComboBox::from_id_salt("lm_unit")
                        .selected_text(self.unit.name())
                        .show_ui(ui, |ui| {
                            for u in SizeUnit::ALL {
                                ui.selectable_value(&mut self.unit, u, u.name());
                            }
                        })
                        .response
                        .labelled_by(label.id);
                    ui.end_row();
                    layer_combo(ui, "Reference Layer", "lm_ref", &mut self.reference, n);
                    ui.end_row();
                    if self.mode == SizeMode::Resize {
                        length(ui, "Width", &mut self.width);
                        length(ui, "Height", &mut self.height);
                    } else {
                        length(ui, "Left", &mut self.left);
                        length(ui, "Top", &mut self.top);
                    }
                });
            }
            SizeMode::Swap => {
                egui::Grid::new("lm_swap").num_columns(2).show(ui, |ui| {
                    optional_layer_combo(ui, "First Layer", "lm_swap_a", &mut self.swap_a, n);
                    optional_layer_combo(ui, "Second Layer", "lm_swap_b", &mut self.swap_b, n);
                });
                ui.checkbox(&mut self.reorder, "Reorder");
            }
            SizeMode::Align => {
                egui::Grid::new("lm_align").num_columns(2).show(ui, |ui| {
                    layer_combo(ui, "Reference Layer", "lm_ref", &mut self.reference, n);
                    ui.end_row();
                    let label = ui.label("Direction");
                    egui::ComboBox::from_id_salt("lm_edge")
                        .selected_text(self.edge.name())
                        .show_ui(ui, |ui| {
                            for e in AlignEdge::ALL {
                                ui.selectable_value(&mut self.edge, e, e.name());
                            }
                        })
                        .response
                        .labelled_by(label.id);
                    ui.end_row();
                });
            }
        }
        let ready = self.mode != SizeMode::Swap || (self.swap_a.is_some() && self.swap_b.is_some());
        if ui.add_enabled(ready, egui::Button::new("Apply")).clicked() {
            let sel = self.selection();
            let (unit, r) = (self.unit, self.reference);
            let (w, h, l, t) = (self.width, self.height, self.left, self.top);
            let (a, b, reorder, edge) = (self.swap_a, self.swap_b, self.reorder, self.edge);
            match self.mode {
                SizeMode::Resize => self.apply(|g| g.resize_layers(&sel, r, unit, w, h)),
                SizeMode::Move => self.apply(|g| g.move_layers(&sel, r, unit, l, t)),
                SizeMode::Align => self.apply(|g| g.align_layers(&sel, r, edge)),
                SizeMode::Swap => {
                    if let (Some(a), Some(b)) = (a, b) {
                        self.apply(|g| g.swap_layers(a, b, reorder));
                    }
                }
            }
        }
    }

    fn link_tab(&mut self, ui: &mut egui::Ui) {
        // Only layers before the first selected layer that are not linked themselves can be parents.
        let first = self.selected.iter().next().copied().unwrap_or(0);
        // Like Origin's tab, show the link the (first) selected layer has now.
        let shown = self.selected.iter().next().copied();
        if shown != self.link_shown_for {
            self.link_shown_for = shown;
            let link = shown.and_then(|i| self.work.layers.get(i)).and_then(|l| l.link);
            self.link_to = link.map(|k| k.parent);
            self.link_x = link.map_or(ScaleLink::None, |k| k.x);
            self.link_y = link.map_or(ScaleLink::None, |k| k.y);
        }
        let parents: Vec<usize> =
            (0..first.min(self.work.layers.len())).filter(|&p| self.work.layers[p].link.is_none()).collect();
        if self.link_to.is_some_and(|p| !parents.contains(&p)) {
            self.link_to = None;
        }
        egui::Grid::new("lm_link").num_columns(2).show(ui, |ui| {
            let label = ui.label("Link To");
            egui::ComboBox::from_id_salt("lm_link_to")
                .selected_text(self.link_to.map_or("None".to_string(), |p| (p + 1).to_string()))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.link_to, None, "None");
                    for &p in &parents {
                        ui.selectable_value(&mut self.link_to, Some(p), (p + 1).to_string());
                    }
                })
                .response
                .labelled_by(label.id);
            ui.end_row();
            scale_combo(ui, "X Axis Link", "lm_link_x", &mut self.link_x);
            scale_combo(ui, "Y Axis Link", "lm_link_y", &mut self.link_y);
            ui.add_enabled_ui(false, |ui| {
                for name in ["X1", "X2", "Y1", "Y2"] {
                    ui.label(name);
                    ui.add(egui::DragValue::new(&mut 0.0_f64));
                    ui.end_row();
                }
                ui.label("Units");
                egui::ComboBox::from_id_salt("lm_link_unit").selected_text("% of Linked Layer").show_ui(ui, |_| {});
                ui.end_row();
            });
        });
        ui.add_enabled(
            false,
            egui::Checkbox::new(&mut false, "When dragged to reposition, also reposition parent layer"),
        );
        if ui.button("Apply").clicked() {
            let (sel, parent, x, y) = (self.selection(), self.link_to, self.link_x, self.link_y);
            self.apply(|g| g.link_layers(&sel, parent, x, y));
        }
    }

    fn display_tab(&mut self, ui: &mut egui::Ui) {
        let selection = self.selection();
        if let Some(color) = self.background.show(ui, &self.work, &selection) {
            self.apply(|g| {
                for &i in &selection {
                    g.layers[i].background = color;
                }
                Ok(())
            });
        }
    }

    /// The page with numbered layer frames, their backgrounds and a selection tint.
    fn preview(&self, ui: &mut egui::Ui) {
        ui.strong("Preview");
        let (pw, ph) = (self.work.page.width_pt.max(1.0), self.work.page.height_pt.max(1.0));
        let width = 220.0_f32;
        let size = egui::vec2(width, (width as f64 * ph / pw) as f32);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let painter = ui.painter_at(rect);
        let page = self.work.page.background;
        painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(page.r, page.g, page.b, page.a));
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, Color32::GRAY), egui::StrokeKind::Inside);
        for i in 0..self.work.layers.len() {
            let f = self.work.page_frame(i);
            let r = egui::Rect::from_min_size(
                rect.min + egui::vec2(f.left as f32 * size.x, f.top as f32 * size.y),
                egui::vec2(f.width as f32 * size.x, f.height as f32 * size.y),
            );
            if let Some(c) = self.work.layers[i].background {
                painter.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a));
            }
            let on = self.selected.contains(&i);
            if on {
                painter.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(70, 130, 220, 70));
            }
            let color = if on { Color32::from_rgb(40, 90, 190) } else { Color32::from_rgb(90, 90, 90) };
            painter.rect_stroke(r, 0.0, Stroke::new(1.0_f32, color), egui::StrokeKind::Inside);
            painter.text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                (i + 1).to_string(),
                egui::FontId::proportional(14.0),
                color,
            );
        }
    }
}

fn unavailable(ui: &mut egui::Ui) {
    ui.label(NOT_YET);
    ui.add_enabled_ui(false, |ui| {
        ui.checkbox(&mut false, "Options");
    });
}

fn number(ui: &mut egui::Ui, name: &str, value: &mut f64, max: f64) {
    let label = ui.label(name);
    ui.add(egui::DragValue::new(value).speed(0.1).range(0.0..=max).max_decimals(2)).labelled_by(label.id);
    ui.end_row();
}

fn length(ui: &mut egui::Ui, name: &str, value: &mut f64) {
    let label = ui.label(name);
    ui.add(egui::DragValue::new(value).speed(0.1).range(-1e6..=1e6).max_decimals(3)).labelled_by(label.id);
    ui.end_row();
}

fn layer_combo(ui: &mut egui::Ui, name: &str, id: &str, value: &mut usize, n: usize) {
    let label = ui.label(name);
    egui::ComboBox::from_id_salt(id)
        .selected_text((*value + 1).to_string())
        .show_ui(ui, |ui| {
            for i in 0..n {
                ui.selectable_value(value, i, (i + 1).to_string());
            }
        })
        .response
        .labelled_by(label.id);
}

fn optional_layer_combo(ui: &mut egui::Ui, name: &str, id: &str, value: &mut Option<usize>, n: usize) {
    let label = ui.label(name);
    egui::ComboBox::from_id_salt(id)
        .selected_text(value.map_or("None".to_string(), |i| (i + 1).to_string()))
        .show_ui(ui, |ui| {
            ui.selectable_value(value, None, "None");
            for i in 0..n {
                ui.selectable_value(value, Some(i), (i + 1).to_string());
            }
        })
        .response
        .labelled_by(label.id);
    ui.end_row();
}

/// Sheaf links an axis scale one to one or not at all; Origin's Align and Custom are not built.
fn scale_combo(ui: &mut egui::Ui, name: &str, id: &str, value: &mut ScaleLink) {
    let label = ui.label(name);
    egui::ComboBox::from_id_salt(id)
        .selected_text(match value {
            ScaleLink::None => "None",
            ScaleLink::Straight => "Straight (1 to 1)",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(value, ScaleLink::None, "None");
            ui.selectable_value(value, ScaleLink::Straight, "Straight (1 to 1)");
            ui.add_enabled(false, egui::Button::selectable(false, "Align"));
            ui.add_enabled(false, egui::Button::selectable(false, "Custom"));
        })
        .response
        .labelled_by(label.id);
    ui.end_row();
}
