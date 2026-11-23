//! Graph > Merge Graph Windows... (T60): copies the layers of several graphs into the cells of one
//! new graph, over [`Project::merge_graphs`].

use crate::workspace::{Mode, Workspace};
use eframe::egui;
use sheaf_core::column::default_short_name;
use sheaf_core::merge::{
    MERGE_PAGE_IN, MergeError, MergeLabelPosition, MergeOptions, MergePanelLabel, MergeTextScale, auto_grid,
    source_panel_roots,
};
use sheaf_core::{Document, Project, WindowId};
use sheaf_plot::{SystemFonts, TextMeasurer};

pub(crate) mod folders;
mod preview;
use folders::FolderScope;

pub enum Outcome {
    Open,
    Closed,
    Created(WindowId),
}

#[derive(Copy, Clone, PartialEq)]
enum Source {
    ActivePage,
    AllInProject,
    Specified,
    ActiveFolder,
    Recursive,
    OpenFolder,
    IncludeEmbedded,
}

impl Source {
    const ALL: [Self; 7] = [
        Self::ActivePage,
        Self::ActiveFolder,
        Self::Recursive,
        Self::OpenFolder,
        Self::IncludeEmbedded,
        Self::AllInProject,
        Self::Specified,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::ActivePage => "Active Page",
            Self::AllInProject => "All in Project",
            Self::Specified => "Specified",
            Self::ActiveFolder => "All in Active Folder",
            Self::Recursive => "All in Active Folder (Recursive)",
            Self::OpenFolder => "All in Active Folder (Open)",
            Self::IncludeEmbedded => "All in Active Folder (Include Embedded)",
        }
    }

    fn is_folder(self) -> bool {
        matches!(self, Self::ActiveFolder | Self::Recursive | Self::OpenFolder | Self::IncludeEmbedded)
    }

    fn allows_single_source(self) -> bool {
        matches!(self, Self::ActivePage | Self::OpenFolder | Self::IncludeEmbedded)
    }
}

#[derive(Copy, Clone, PartialEq)]
enum Scale {
    Auto,
    LayerFrame,
    Fixed,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LabelText {
    None,
    Uppercase,
    Lowercase,
    UppercaseRoman,
    LowercaseRoman,
    Custom,
}

impl LabelText {
    const ALL: [Self; 6] =
        [Self::None, Self::Uppercase, Self::Lowercase, Self::UppercaseRoman, Self::LowercaseRoman, Self::Custom];

    fn name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Uppercase => "A",
            Self::Lowercase => "a",
            Self::UppercaseRoman => "I, II, III",
            Self::LowercaseRoman => "i, ii, iii",
            Self::Custom => "Custom",
        }
    }

    fn text(self, index: usize, custom: &str) -> Option<String> {
        match self {
            Self::None => None,
            Self::Uppercase => Some(default_short_name(index)),
            Self::Lowercase => Some(default_short_name(index).to_ascii_lowercase()),
            Self::UppercaseRoman => Some(roman_number(index + 1)),
            Self::LowercaseRoman => Some(roman_number(index + 1).to_ascii_lowercase()),
            Self::Custom => Some(custom_panel_label(custom, index)),
        }
    }
}

fn roman_number(mut number: usize) -> String {
    const DIGITS: [(usize, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut text = String::new();
    for (value, symbol) in DIGITS {
        while number >= value {
            text.push_str(symbol);
            number -= value;
        }
    }
    text
}

/// Expands the measured enumeration codes and pads the index to the length of each hash run.
fn custom_panel_label(pattern: &str, index: usize) -> String {
    let number = index + 1;
    let mut formatted = String::with_capacity(pattern.len());
    let mut rest = pattern;
    while let Some(start) = rest.find(['#', '$', '%']) {
        formatted.push_str(&rest[..start]);
        rest = &rest[start..];
        if rest.starts_with('#') {
            let width = rest.bytes().take_while(|&b| b == b'#').count();
            formatted.push_str(&format!("{number:0width$}"));
            rest = &rest[width..];
        } else if let Some(suffix) = rest.strip_prefix("$(i)") {
            formatted.push_str(&number.to_string());
            rest = suffix;
        } else if let Some(suffix) = rest.strip_prefix("%(u$)") {
            formatted.push_str(&default_short_name(index));
            rest = suffix;
        } else if let Some(suffix) = rest.strip_prefix("%(v$)") {
            formatted.push_str(&default_short_name(index).to_ascii_lowercase());
            rest = suffix;
        } else if let Some(suffix) = rest.strip_prefix("%(p$)") {
            formatted.push('#');
            rest = suffix;
        } else {
            // All the token starts are ASCII. Preserve an unrecognized token as typed.
            formatted.push(rest.as_bytes()[0] as char);
            rest = &rest[1..];
        }
    }
    formatted.push_str(rest);
    formatted
}

fn label_position_name(position: MergeLabelPosition) -> &'static str {
    match position {
        MergeLabelPosition::TopLeftOutside => "Top Left Outside",
        MergeLabelPosition::TopLeftInside => "Top Left Inside",
        MergeLabelPosition::TopRightOutside => "Top Right Outside",
        MergeLabelPosition::TopRightInside => "Top Right Inside",
    }
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

#[derive(Clone, Default, PartialEq)]
struct MergeDraft {
    options: MergeOptions,
    common_width: bool,
    common_height: bool,
}

impl MergeDraft {
    fn merge(&self, project: &mut Project) -> Result<WindowId, MergeError> {
        if !self.options.rearrange || !(self.common_width || self.common_height) {
            return project.merge_graphs(&self.options);
        }
        let mut options = self.options.clone();
        let mut widths = Vec::with_capacity(options.sources.len());
        let mut heights = Vec::with_capacity(options.sources.len());
        // Resolve the same axes the source graphs display, including automatic limits.
        // This runs when rendering a preview or creating the graph, not on every dialog frame.
        for &id in &options.sources {
            let graph = project.graph(id).ok_or(MergeError::NotAGraph(id))?;
            if graph.layers.is_empty() {
                return Err(MergeError::Empty(graph.name.clone()));
            }
            let geometry = sheaf_plot::layout_geometry(project, id).map_err(|_| MergeError::CommonScale)?;
            for root in source_panel_roots(graph, options.group_sources) {
                let axes = &geometry[root];
                let width = axes.x.max - axes.x.min;
                let width = if options.share_x && !graph.has_children(root) {
                    widths.first().copied().unwrap_or(width)
                } else {
                    width
                };
                widths.push(width);
                heights.push(axes.y.max - axes.y.min);
            }
        }
        options.common_x_spans = self.common_width.then_some(widths);
        options.common_y_spans = self.common_height.then_some(heights);
        project.merge_graphs(&options)
    }
}

pub struct MergeGraphsDialog {
    source: Source,
    active_page: Option<WindowId>,
    folder: FolderScope,
    /// The graphs in merge order, each with its tick (used by "Specified").
    graphs: Vec<(WindowId, bool)>,
    keep_sources: bool,
    rearrange: bool,
    rows: Option<usize>,
    cols: Option<usize>,
    add_extra_layers: bool,
    keep_aspect: bool,
    group_sources: bool,
    common_width: bool,
    common_height: bool,
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
    label_text: LabelText,
    custom_label: String,
    label_position: MergeLabelPosition,
    problem: Option<String>,
    preview: preview::Preview,
}

impl MergeGraphsDialog {
    pub fn new(project: &Project, folder: &[usize], active_page: Option<WindowId>) -> Self {
        let active_page = active_page.filter(|&id| project.graph(id).is_some());
        let graphs: Vec<_> = project.graphs().map(|g| (g.id, true)).collect();
        Self {
            source: if graphs.len() == 1 && active_page.is_some() { Source::ActivePage } else { Source::AllInProject },
            active_page,
            folder: FolderScope::new(project, folder),
            graphs,
            keep_sources: true,
            rearrange: true,
            rows: None,
            cols: None,
            add_extra_layers: false,
            keep_aspect: false,
            group_sources: true,
            common_width: false,
            common_height: false,
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
            label_text: LabelText::None,
            custom_label: "Graph##".into(),
            label_position: MergeLabelPosition::default(),
            problem: None,
            preview: preview::Preview::default(),
        }
    }

    /// The graphs that will be merged, in order.
    fn chosen(&self, project: &Project, workspace: &Workspace) -> Vec<WindowId> {
        match self.source {
            Source::ActivePage => self.active_page.filter(|&id| project.graph(id).is_some()).into_iter().collect(),
            Source::AllInProject => self.graphs.iter().map(|(id, _)| *id).collect(),
            Source::Specified => self.graphs.iter().filter(|(_, on)| *on).map(|(id, _)| *id).collect(),
            Source::ActiveFolder | Source::Recursive => self.folder.graphs(project, self.source == Source::Recursive),
            Source::OpenFolder => self
                .folder
                .graphs(project, false)
                .into_iter()
                .filter(|&id| workspace.is_shown(id) && workspace.mode(id) != Some(Mode::Minimised))
                .collect(),
            Source::IncludeEmbedded => self.folder.graphs_with_embedded(project),
        }
    }

    fn draft(&self, project: &Project, workspace: &Workspace) -> MergeDraft {
        let sources = self.chosen(project, workspace);
        let page = (self.page_in[0], self.page_in[1]);
        let default_page = !self.portrait && page == MERGE_PAGE_IN;
        let font = MergePanelLabel::font();
        let panel_labels = (0..sources.len())
            .filter_map(|index| self.label_text.text(index, &self.custom_label))
            .map(|text| {
                let width_pt = SystemFonts::shared().width(&text, &font, font.size_pt);
                MergePanelLabel { text, width_pt }
            })
            .collect();
        let options = MergeOptions {
            sources,
            allow_single_source: self.source.allows_single_source(),
            rows: self.rows,
            cols: self.cols,
            add_extra_layers: self.add_extra_layers,
            group_sources: self.group_sources,
            common_x_spans: None,
            common_y_spans: None,
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
            panel_labels,
            label_position: self.label_position,
        };
        MergeDraft { common_width: self.common_width, common_height: self.common_height, options }
    }

    fn set_orientation(&mut self, portrait: bool) {
        if portrait != self.portrait {
            self.portrait = portrait;
            self.page_in.swap(0, 1);
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document, workspace: &Workspace) -> Outcome {
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
            .default_width(800.0)
            .show(ctx, |ui| {
                let preview_width = (ui.available_width() * 0.4).min(320.0);
                let controls_width = ui.available_width() - preview_width - ui.spacing().item_spacing.x;
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(controls_width, 560.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, true])
                                .max_height(560.0)
                                .show(ui, |ui| self.body(ui, doc.project(), workspace));
                        },
                    );
                    ui.allocate_ui_with_layout(
                        egui::vec2(preview_width, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            let draft = self.draft(doc.project(), workspace);
                            let valid_folder = !self.source.is_folder() || self.folder.valid(doc.project());
                            self.preview.show(ui, doc, draft, valid_folder);
                        },
                    );
                });
                if let Some(e) = &self.problem
                    && self.preview.error() != Some(e.as_str())
                {
                    ui.colored_label(ui.visuals().error_fg_color, e);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        if self.source.is_folder() && !self.folder.valid(doc.project()) {
                            self.problem = Some("Source folder tree changed. Reopen Merge Graph Windows.".into());
                        } else {
                            let draft = self.draft(doc.project(), workspace);
                            match doc.edit("Merge Graph Windows", |p| draft.merge(p)) {
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

    fn body(&mut self, ui: &mut egui::Ui, project: &Project, workspace: &Workspace) {
        let previous = self.source;
        egui::Grid::new("merge_top").num_columns(2).show(ui, |ui| {
            let label = ui.label("Merge");
            egui::ComboBox::from_id_salt("merge_source")
                .selected_text(self.source.name())
                .show_ui(ui, |ui| {
                    for source in Source::ALL {
                        ui.selectable_value(&mut self.source, source, source.name());
                    }
                })
                .response
                .labelled_by(label.id);
            ui.end_row();
        });
        if previous != self.source {
            self.problem = None;
        }
        let chosen = self.chosen(project, workspace);
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
            let panels = chosen
                .iter()
                .filter_map(|&id| project.graph(id))
                .map(|graph| source_panel_roots(graph, self.group_sources).count())
                .sum();
            self.arrange(ui, panels);
            self.spacing(ui);
        });
        self.page_setup(ui);
        self.scale_elements(ui);
        egui::CollapsingHeader::new("Add Label").show(ui, |ui| {
            egui::Grid::new("merge_label").num_columns(2).show(ui, |ui| {
                let label = ui.label("Label Text");
                egui::ComboBox::from_id_salt("merge_label_text")
                    .selected_text(self.label_text.name())
                    .show_ui(ui, |ui| {
                        for mode in LabelText::ALL {
                            ui.selectable_value(&mut self.label_text, mode, mode.name());
                        }
                    })
                    .response
                    .labelled_by(label.id);
                ui.end_row();
                if self.label_text == LabelText::Custom {
                    let label = ui.label("Custom");
                    ui.text_edit_singleline(&mut self.custom_label)
                        .labelled_by(label.id)
                        .on_hover_text(
                            "Use %(u$) or %(v$) for letters, # or $(i) for 1, 2, …, ## for 01, 02, …, and %(p$) for a literal #.",
                        );
                    ui.end_row();
                }
                ui.add_enabled_ui(self.label_text != LabelText::None, |ui| {
                    ui.label("Label Position");
                    egui::ComboBox::from_id_salt("merge_label_pos")
                        .selected_text(label_position_name(self.label_position))
                        .show_ui(ui, |ui| {
                            use MergeLabelPosition::*;
                            for position in [TopLeftOutside, TopLeftInside, TopRightOutside, TopRightInside] {
                                ui.selectable_value(&mut self.label_position, position, label_position_name(position));
                            }
                        });
                    ui.end_row();
                });
            });
        });
    }

    fn arrange(&mut self, ui: &mut egui::Ui, panels: usize) {
        egui::CollapsingHeader::new("Arrange Settings").default_open(true).show(ui, |ui| {
            // Until the user edits them the numbers follow the selection.
            let (rows, cols) = auto_grid(panels, self.rows, self.cols);
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
            ui.checkbox(&mut self.add_extra_layers, "Add Extra Layer(s) for Grid");
            ui.add_enabled(
                !(self.common_width || self.common_height),
                egui::Checkbox::new(&mut self.keep_aspect, "Keep Layer Aspect Ratio"),
            );
            ui.checkbox(&mut self.group_sources, "Treat Each Source Graph as a Unit");
            ui.checkbox(&mut self.link, "Link Source Graphs");
            ui.horizontal(|ui| {
                ui.add_space(18.0);
                ui.add_enabled(self.link, egui::Checkbox::new(&mut self.share_x, "Share X scale (Sheaf)"));
            });
            ui.checkbox(&mut self.common_width, "Set Layer Width by Common Scale");
            ui.checkbox(&mut self.common_height, "Set Layer Height by Common Scale");
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

#[cfg(test)]
mod label_tests {
    use super::{LabelText, custom_panel_label};

    #[test]
    fn roman_labels_are_one_based_and_use_subtractive_notation() {
        for (number, expected) in [(1, "I"), (4, "IV"), (9, "IX"), (49, "XLIX"), (994, "CMXCIV"), (2024, "MMXXIV")] {
            assert_eq!(LabelText::UppercaseRoman.text(number - 1, "").as_deref(), Some(expected));
            assert_eq!(
                LabelText::LowercaseRoman.text(number - 1, "").as_deref(),
                Some(expected.to_ascii_lowercase().as_str())
            );
        }
    }

    #[test]
    fn each_panel_number_expands_with_its_own_padding() {
        assert_eq!(custom_panel_label("Panel $(i) / ##", 2), "Panel 3 / 03");
        assert_eq!(custom_panel_label("Graph##", 122), "Graph123");
        assert_eq!(custom_panel_label("Section (a$)", 0), "Section (a$)");
    }

    #[test]
    fn enumeration_respects_literal_hashes_and_unrecognized_text() {
        assert_eq!(custom_panel_label("Panel %(u$) / %(v$) / ## / $(i) / %(p$)", 2), "Panel C / c / 03 / 3 / #");
        assert_eq!(custom_panel_label("%(p$)##", 0), "#01");
        assert_eq!(custom_panel_label("α (%(u$),%(v$)) %H %(r$) $", 26), "α (AA,aa) %H %(r$) $");
    }
}
