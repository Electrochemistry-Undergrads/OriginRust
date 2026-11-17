//! Graph > Merge Graph Windows... (T60): copies the layers of several graphs into the cells of one
//! new graph, over [`Project::merge_graphs`].

use eframe::egui;
use sheaf_core::merge::{MERGE_PAGE_IN, MergeOptions, MergeTextScale, auto_grid};
use sheaf_core::{Document, Project, WindowId};

mod folders;
use folders::FolderScope;

pub enum Outcome {
    Open,
    Closed,
    Created(WindowId),
}

#[derive(Copy, Clone, PartialEq)]
enum Source {
    AllInProject,
    Specified,
    ActiveFolder,
    Recursive,
}

impl Source {
    fn name(self) -> &'static str {
        match self {
            Self::AllInProject => "All in Project",
            Self::Specified => "Specified",
            Self::ActiveFolder => "All in Active Folder",
            Self::Recursive => "All in Active Folder (Recursive)",
        }
    }

    fn is_folder(self) -> bool {
        matches!(self, Self::ActiveFolder | Self::Recursive)
    }
}

#[derive(Copy, Clone, PartialEq)]
enum Scale {
    Auto,
    LayerFrame,
    Fixed,
}

#[derive(Copy, Clone, PartialEq)]
enum Unit {
    Inch,
    Cm,
    Mm,
    Pixel,
    Point,
}

impl Unit {
    const ALL: [Unit; 5] = [Unit::Inch, Unit::Cm, Unit::Mm, Unit::Pixel, Unit::Point];

    fn name(self) -> &'static str {
        match self {
            Unit::Inch => "inch",
            Unit::Cm => "cm",
            Unit::Mm => "mm",
            Unit::Pixel => "pixel",
            Unit::Point => "point",
        }
    }

    /// Units per inch; pixels are counted at 96 per inch.
    fn per_inch(self) -> f64 {
        match self {
            Unit::Inch => 1.0,
            Unit::Cm => 2.54,
            Unit::Mm => 25.4,
            Unit::Pixel => 96.0,
            Unit::Point => 72.0,
        }
    }
}

pub struct MergeGraphsDialog {
    source: Source,
    folder: FolderScope,
    /// The graphs in merge order, each with its tick (used by "Specified").
    graphs: Vec<(WindowId, bool)>,
    keep_sources: bool,
    rearrange: bool,
    rows: Option<usize>,
    cols: Option<usize>,
    keep_aspect: bool,
    link: bool,
    share_x: bool,
    hide_inner: bool,
    h_gap: f64,
    v_gap: f64,
    /// Left, right, top, bottom.
    margins: [f64; 4],
    portrait: bool,
    unit: Unit,
    /// Width and height of the page as shown, in inches.
    page_in: [f64; 2],
    scale: Scale,
    factor: f64,
    problem: Option<String>,
}

impl MergeGraphsDialog {
    pub fn new(project: &Project, folder: &[usize]) -> Self {
        Self {
            source: Source::AllInProject,
            folder: FolderScope::new(project, folder),
            graphs: project.graphs().map(|g| (g.id, true)).collect(),
            keep_sources: true,
            rearrange: true,
            rows: None,
            cols: None,
            keep_aspect: false,
            link: false,
            share_x: false,
            hide_inner: false,
            h_gap: 5.0,
            v_gap: 5.0,
            margins: [15.0, 10.0, 10.0, 15.0],
            portrait: false,
            unit: Unit::Inch,
            page_in: [MERGE_PAGE_IN.0, MERGE_PAGE_IN.1],
            scale: Scale::Auto,
            factor: 1.0,
            problem: None,
        }
    }

    /// The graphs that will be merged, in order.
    fn chosen(&self, project: &Project) -> Vec<WindowId> {
        if self.source.is_folder() {
            self.folder.graphs(project, self.source == Source::Recursive)
        } else {
            self.graphs.iter().filter(|(_, on)| self.source == Source::AllInProject || *on).map(|(id, _)| *id).collect()
        }
    }

    fn options(&self, project: &Project) -> MergeOptions {
        let sources = self.chosen(project);
        let (rows, cols) = auto_grid(sources.len(), self.rows, self.cols);
        let page = (self.page_in[0], self.page_in[1]);
        let default_page = !self.portrait && page == MERGE_PAGE_IN;
        MergeOptions {
            sources,
            rows: Some(rows),
            cols: Some(cols),
            h_gap: self.h_gap,
            v_gap: self.v_gap,
            margin_left: self.margins[0],
            margin_right: self.margins[1],
            margin_top: self.margins[2],
            margin_bottom: self.margins[3],
            keep_sources: self.keep_sources,
            rearrange: self.rearrange,
            portrait: self.portrait,
            // the numbers as shown: Orientation has already swapped them
            page_in: (!default_page).then_some(page),
            keep_aspect: self.keep_aspect,
            // greyed with the rest of Arrange Settings when the layout is not rearranged
            link: self.rearrange && self.link,
            share_x: self.rearrange && self.link && self.share_x,
            hide_inner: self.hide_inner,
            text_scale: match self.scale {
                Scale::Auto | Scale::LayerFrame => MergeTextScale::Auto,
                Scale::Fixed => MergeTextScale::Fixed(self.factor),
            },
        }
    }

    fn set_orientation(&mut self, portrait: bool) {
        if portrait != self.portrait {
            self.portrait = portrait;
            self.page_in.swap(0, 1);
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Outcome::Closed;
        }
        // Graphs deleted or added meanwhile.
        let project = doc.project();
        self.graphs.retain(|(id, _)| project.graph(*id).is_some());
        for g in project.graphs() {
            if !self.graphs.iter().any(|(id, _)| *id == g.id) {
                self.graphs.push((g.id, true));
            }
        }
        let mut open = true;
        let mut result = Outcome::Open;
        egui::Window::new("Merge Graph Windows")
            .id(egui::Id::new("merge_graphs"))
            .open(&mut open)
            .collapsible(false)
            .default_width(460.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(560.0).show(ui, |ui| self.body(ui, doc.project()));
                if let Some(e) = &self.problem {
                    ui.colored_label(ui.visuals().error_fg_color, e);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        if self.source.is_folder() && !self.folder.valid(doc.project()) {
                            self.problem = Some("Source folder tree changed. Reopen Merge Graph Windows.".into());
                        } else {
                            let opts = self.options(doc.project());
                            match doc.edit("Merge Graph Windows", |p| p.merge_graphs(&opts)) {
                                Ok(id) => result = Outcome::Created(id),
                                Err(e) => self.problem = Some(e.to_string()),
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        result = Outcome::Closed;
                    }
                });
            });
        if !open { Outcome::Closed } else { result }
    }

    fn body(&mut self, ui: &mut egui::Ui, project: &Project) {
        let previous = self.source;
        egui::Grid::new("merge_top").num_columns(2).show(ui, |ui| {
            let label = ui.label("Merge");
            egui::ComboBox::from_id_salt("merge_source")
                .selected_text(self.source.name())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.source, Source::ActiveFolder, "All in Active Folder");
                    ui.selectable_value(&mut self.source, Source::Recursive, "All in Active Folder (Recursive)");
                    ui.selectable_value(&mut self.source, Source::AllInProject, "All in Project");
                    ui.selectable_value(&mut self.source, Source::Specified, "Specified");
                })
                .response
                .labelled_by(label.id);
            ui.end_row();
        });
        if previous != self.source {
            self.problem = None;
        }
        let chosen = self.chosen(project);
        ui.label("Graphs");
        let specified = self.source == Source::Specified;
        let mut swap = None;
        let count = self.graphs.len();
        if specified {
            for (i, (id, on)) in self.graphs.iter_mut().enumerate() {
                let name = project.graph(*id).map_or("?", |g| g.name.as_str()).to_string();
                ui.horizontal(|ui| {
                    ui.checkbox(on, name);
                    if ui.add_enabled(i > 0, egui::Button::new("Up")).clicked() {
                        swap = Some((i - 1, i));
                    }
                    if ui.add_enabled(i + 1 < count, egui::Button::new("Down")).clicked() {
                        swap = Some((i, i + 1));
                    }
                });
            }
        } else {
            for &id in &chosen {
                if let Some(graph) = project.graph(id) {
                    ui.label(&graph.name);
                }
            }
        }
        if let Some((a, b)) = swap {
            self.graphs.swap(a, b);
        }
        ui.checkbox(&mut self.keep_sources, "Keep Source Graphs");
        ui.checkbox(&mut self.rearrange, "Rearrange Layout");
        ui.add_enabled_ui(self.rearrange, |ui| {
            self.arrange(ui, chosen.len());
            self.spacing(ui);
        });
        self.page_setup(ui);
        self.scale_elements(ui);
        egui::CollapsingHeader::new("Add Label").show(ui, |ui| {
            ui.add_enabled_ui(false, |ui| {
                egui::Grid::new("merge_label").num_columns(2).show(ui, |ui| {
                    ui.label("Label Text");
                    egui::ComboBox::from_id_salt("merge_label_text").selected_text("None").show_ui(ui, |_| {});
                    ui.end_row();
                    ui.label("Label Position");
                    egui::ComboBox::from_id_salt("merge_label_pos")
                        .selected_text("Top Left Outside")
                        .show_ui(ui, |_| {});
                    ui.end_row();
                });
            });
        });
    }

    fn arrange(&mut self, ui: &mut egui::Ui, sources: usize) {
        egui::CollapsingHeader::new("Arrange Settings").default_open(true).show(ui, |ui| {
            // Until the user edits them the numbers follow the selection.
            let (rows, cols) = auto_grid(sources, self.rows, self.cols);
            let (mut r, mut c) = (rows, cols);
            egui::Grid::new("merge_arrange").num_columns(2).show(ui, |ui| {
                let label = ui.label("Number of Rows");
                if ui.add(egui::DragValue::new(&mut r).range(1..=64)).labelled_by(label.id).changed() {
                    self.rows = Some(r);
                }
                ui.end_row();
                let label = ui.label("Number of Columns");
                if ui.add(egui::DragValue::new(&mut c).range(1..=64)).labelled_by(label.id).changed() {
                    self.cols = Some(c);
                }
                ui.end_row();
            });
            ui.add_enabled(false, egui::Checkbox::new(&mut false, "Add Extra Layer(s) for Grid"));
            ui.checkbox(&mut self.keep_aspect, "Keep Layer Aspect Ratio");
            ui.add_enabled(false, egui::Checkbox::new(&mut true, "Treat Each Source Graph as a Unit"));
            ui.checkbox(&mut self.link, "Link Source Graphs");
            ui.horizontal(|ui| {
                ui.add_space(18.0);
                ui.add_enabled(self.link, egui::Checkbox::new(&mut self.share_x, "Share X scale (Sheaf)"));
            });
            ui.add_enabled(false, egui::Checkbox::new(&mut false, "Set Layer Width by Common Scale"));
            ui.add_enabled(false, egui::Checkbox::new(&mut false, "Set Layer Height by Common Scale"));
            ui.checkbox(&mut self.hide_inner, "Show Axes Frame");
            ui.label("Overlapping axes/ticks are hidden");
        });
    }

    fn spacing(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Spacing (in % of Page Dimension)").default_open(true).show(ui, |ui| {
            egui::Grid::new("merge_spacing").num_columns(2).show(ui, |ui| {
                number(ui, "Horizontal Gap", &mut self.h_gap, 50.0);
                number(ui, "Vertical Gap", &mut self.v_gap, 50.0);
                for (name, value) in
                    ["Left Margin", "Right Margin", "Top Margin", "Bottom Margin"].into_iter().zip(&mut self.margins)
                {
                    number(ui, name, value, 90.0);
                }
            });
        });
    }

    fn page_setup(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Page Setup").show(ui, |ui| {
            egui::Grid::new("merge_page").num_columns(2).show(ui, |ui| {
                let label = ui.label("Orientation");
                let mut portrait = self.portrait;
                egui::ComboBox::from_id_salt("merge_orientation")
                    .selected_text(if portrait { "Portrait" } else { "Landscape" })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut portrait, false, "Landscape");
                        ui.selectable_value(&mut portrait, true, "Portrait");
                    })
                    .response
                    .labelled_by(label.id);
                self.set_orientation(portrait);
                ui.end_row();
                ui.label("Original Size");
                ui.label(format!("{} inch x {} inch", MERGE_PAGE_IN.0, MERGE_PAGE_IN.1));
                ui.end_row();
                // The numbers are shown in the chosen unit and kept in inches.
                let k = self.unit.per_inch();
                for (name, i) in [("Width", 0), ("Height", 1)] {
                    let label = ui.label(name);
                    let mut shown = self.page_in[i] * k;
                    if ui
                        .add(egui::DragValue::new(&mut shown).speed(0.05).range(0.0..=1e5).max_decimals(3))
                        .labelled_by(label.id)
                        .changed()
                    {
                        self.page_in[i] = shown / k;
                    }
                    ui.end_row();
                }
                let label = ui.label("Unit");
                egui::ComboBox::from_id_salt("merge_unit")
                    .selected_text(self.unit.name())
                    .show_ui(ui, |ui| {
                        for u in Unit::ALL {
                            ui.selectable_value(&mut self.unit, u, u.name());
                        }
                    })
                    .response
                    .labelled_by(label.id);
                ui.end_row();
            });
        });
    }

    fn scale_elements(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Scale Elements").show(ui, |ui| {
            egui::Grid::new("merge_scale").num_columns(2).show(ui, |ui| {
                let label = ui.label("Scale Mode");
                egui::ComboBox::from_id_salt("merge_scale_mode")
                    .selected_text(match self.scale {
                        Scale::Auto => "Auto",
                        Scale::LayerFrame => "Scale with Layer Frame",
                        Scale::Fixed => "Fixed Factor",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.scale, Scale::Auto, "Auto");
                        ui.selectable_value(&mut self.scale, Scale::LayerFrame, "Scale with Layer Frame");
                        ui.selectable_value(&mut self.scale, Scale::Fixed, "Fixed Factor");
                    })
                    .response
                    .labelled_by(label.id);
                ui.end_row();
                let label = ui.label("Fixed Factor");
                ui.add_enabled(
                    self.scale == Scale::Fixed,
                    egui::DragValue::new(&mut self.factor).speed(0.01).range(0.05..=20.0).max_decimals(2),
                )
                .labelled_by(label.id);
                ui.end_row();
            });
        });
    }
}

fn number(ui: &mut egui::Ui, name: &str, value: &mut f64, max: f64) {
    let label = ui.label(name);
    ui.add(egui::DragValue::new(value).speed(0.1).range(0.0..=max).max_decimals(2)).labelled_by(label.id);
    ui.end_row();
}
