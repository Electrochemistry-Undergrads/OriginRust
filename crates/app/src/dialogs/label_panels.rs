//! Multiple Panels by Label over the existing panel builder (graphing 15.7b).

use eframe::egui;
use sheaf_core::panels::{MAX_PANELS, PanelLayout};
use sheaf_core::template::Template;
use sheaf_core::{ColumnId, Document, Project, WindowId};
use std::collections::BTreeSet;

pub enum Outcome {
    Open,
    Closed,
    Created(WindowId),
}

#[derive(Clone, PartialEq)]
enum Identifier {
    LongName,
    Units,
    Comments,
    Parameter(String),
}

impl Identifier {
    fn name(&self) -> String {
        match self {
            Self::LongName => "Long Name".into(),
            Self::Units => "Units".into(),
            Self::Comments => "Comments".into(),
            Self::Parameter(name) => format!("Parameter: {name}"),
        }
    }

    fn value(&self, column: &sheaf_core::Column) -> String {
        match self {
            Self::LongName => column.long_name.clone(),
            Self::Units => column.units.clone(),
            Self::Comments => column.comments.clone(),
            Self::Parameter(name) => column.params.get(name).cloned().unwrap_or_default(),
        }
    }
}

type Group = (String, Vec<ColumnId>);

pub struct LabelPanelsDialog {
    book: WindowId,
    columns: Vec<ColumnId>,
    input_rows: Option<(usize, usize)>,
    identifiers: Vec<Identifier>,
    identifier: Identifier,
    template: Template,
    rows: usize,
    cols: usize,
    auto_rows: bool,
    auto_cols: bool,
    gaps: [f64; 2],
    margins: [f64; 4],
    problem: Option<String>,
}

impl LabelPanelsDialog {
    pub fn new(
        project: &Project,
        book: WindowId,
        columns: Vec<ColumnId>,
        input_rows: Option<(usize, usize)>,
    ) -> Option<Self> {
        project.workbook(book)?;
        if columns.is_empty() {
            return None;
        }
        let parameters: BTreeSet<_> =
            columns.iter().filter_map(|id| project.column(*id)).flat_map(|c| c.params.keys().cloned()).collect();
        let mut identifiers = vec![Identifier::LongName, Identifier::Units, Identifier::Comments];
        identifiers.extend(parameters.into_iter().map(Identifier::Parameter));
        Some(Self {
            book,
            columns,
            input_rows,
            identifiers,
            identifier: Identifier::LongName,
            template: Template::Line,
            rows: 1,
            cols: 1,
            auto_rows: true,
            auto_cols: true,
            gaps: [5.0, 5.0],
            margins: [12.0, 10.0, 10.0, 12.0],
            problem: None,
        })
    }

    fn groups(&self, project: &Project) -> Result<Vec<Group>, String> {
        let mut groups: Vec<Group> = Vec::new();
        for id in &self.columns {
            let column = project
                .column(*id)
                .filter(|c| c.data.is_numeric())
                .ok_or("The input columns changed. Close this window and select them again.")?;
            let value = self.identifier.value(column).trim().to_owned();
            if let Some((_, columns)) = groups.iter_mut().find(|(name, _)| name.eq_ignore_ascii_case(&value)) {
                columns.push(*id);
            } else {
                if groups.len() == MAX_PANELS {
                    return Err(format!("Select data for at most {MAX_PANELS} groups."));
                }
                groups.push((value, vec![*id]));
            }
        }
        if groups.is_empty() || groups.len() > MAX_PANELS {
            return Err(format!("Select data for 1 to {MAX_PANELS} groups."));
        }
        Ok(groups)
    }

    fn prepare(&self, project: &Project) -> Result<(Vec<Group>, PanelLayout), String> {
        let groups = self.groups(project)?;
        let (rows, cols) = dimensions(groups.len(), self.rows, self.cols, self.auto_rows, self.auto_cols)?;
        let mut layout = PanelLayout::arrange(rows, cols);
        [layout.h_gap, layout.v_gap] = self.gaps;
        [layout.margin_left, layout.margin_right, layout.margin_top, layout.margin_bottom] = self.margins;
        layout.frames().map_err(|e| e.to_string())?;
        Ok((groups, layout))
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        if doc.project().workbook(self.book).is_none() || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Outcome::Closed;
        }
        let mut open = true;
        let mut outcome = Outcome::Open;
        egui::Window::new("Multiple Panels by Label")
            .id(egui::Id::new("label_panels"))
            .open(&mut open)
            .collapsible(false)
            .default_width(480.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(530.0).show(ui, |ui| {
                    ui.collapsing(format!("Input: {} Y column(s)", self.columns.len()), |ui| {
                        for id in &self.columns {
                            if let Some((book, sheet, column)) = doc.project().locate(*id) {
                                ui.label(format!("[{}]{}!{}", book.name, sheet.name, column.short_name));
                            }
                        }
                        if let Some((start, end)) = self.input_rows {
                            ui.label(format!("Rows {} to {end}", start + 1));
                        }
                    });
                    egui::Grid::new("label_panel_options").num_columns(2).show(ui, |ui| {
                        let label = ui.label("Group Identifier");
                        egui::ComboBox::from_id_salt("panel_identifier")
                            .selected_text(self.identifier.name())
                            .show_ui(ui, |ui| {
                                for identifier in &self.identifiers {
                                    ui.selectable_value(&mut self.identifier, identifier.clone(), identifier.name());
                                }
                            })
                            .response
                            .labelled_by(label.id);
                        ui.end_row();
                        let label = ui.label("Plot Type");
                        egui::ComboBox::from_id_salt("label_panel_type")
                            .selected_text(template_name(self.template))
                            .show_ui(ui, |ui| {
                                for template in
                                    [Template::Line, Template::Scatter, Template::LineSymbol, Template::Column]
                                {
                                    ui.selectable_value(&mut self.template, template, template_name(template));
                                }
                            })
                            .response
                            .labelled_by(label.id);
                        ui.end_row();
                        let group_count = self.groups(doc.project()).map_or(1, |g| g.len());
                        let computed =
                            dimensions(group_count, self.rows, self.cols, self.auto_rows, self.auto_cols).ok();
                        grid_size(
                            ui,
                            "Number of Rows",
                            "Auto Rows",
                            &mut self.rows,
                            &mut self.auto_rows,
                            computed.map(|d| d.0),
                        );
                        grid_size(
                            ui,
                            "Number of Columns",
                            "Auto Columns",
                            &mut self.cols,
                            &mut self.auto_cols,
                            computed.map(|d| d.1),
                        );
                    });
                    if let Ok(groups) = self.groups(doc.project()) {
                        ui.label(format!("{} groups in total", groups.len()));
                        ui.collapsing("Groups", |ui| {
                            for (name, columns) in groups {
                                ui.label(format!(
                                    "{}: {} plot(s)",
                                    if name.is_empty() { "(empty)" } else { &name },
                                    columns.len()
                                ));
                            }
                        });
                    }
                    egui::CollapsingHeader::new("Spacing (in % of Page Dimension)").default_open(true).show(ui, |ui| {
                        egui::Grid::new("label_panel_spacing").num_columns(2).show(ui, |ui| {
                            for (label, value) in ["Horizontal Gap", "Vertical Gap"].into_iter().zip(&mut self.gaps) {
                                number(ui, label, value, 50.0);
                            }
                            for (label, value) in ["Left Margin", "Right Margin", "Top Margin", "Bottom Margin"]
                                .into_iter()
                                .zip(&mut self.margins)
                            {
                                number(ui, label, value, 90.0);
                            }
                        });
                    });
                });
                let prepared = self.prepare(doc.project());
                if let Err(e) = &prepared {
                    ui.colored_label(ui.visuals().error_fg_color, e);
                }
                if let Some(e) = &self.problem {
                    ui.colored_label(ui.visuals().error_fg_color, e);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.add_enabled(false, egui::Button::new("Auto Preview"));
                    ui.add_enabled(false, egui::Button::new("Preview"));
                    if ui.add_enabled(prepared.is_ok(), egui::Button::new("OK")).clicked() {
                        let (groups, layout) = prepared.unwrap();
                        let columns: Vec<_> = groups.iter().map(|(_, columns)| columns.clone()).collect();
                        match doc.edit("Panels by label", |p| {
                            let id = p.plot_panels(self.template, &columns, &layout)?;
                            for (layer, (name, _)) in p.graph_mut(id).unwrap().layers.iter_mut().zip(&groups) {
                                layer.y_axis.title = Some(name.clone());
                                for plot in &mut layer.plots {
                                    plot.rows = self.input_rows;
                                }
                            }
                            Ok::<_, sheaf_core::project::ProjectError>(id)
                        }) {
                            Ok(graph) => outcome = Outcome::Created(graph),
                            Err(e) => self.problem = Some(e.to_string()),
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = Outcome::Closed;
                    }
                });
            });
        if open { outcome } else { Outcome::Closed }
    }
}

fn template_name(template: Template) -> &'static str {
    match template {
        Template::Scatter => "Scatter",
        Template::LineSymbol => "Line+symbol",
        Template::Column => "Column",
        _ => "Line",
    }
}

fn grid_size(
    ui: &mut egui::Ui,
    name: &str,
    auto_name: &str,
    value: &mut usize,
    auto: &mut bool,
    computed: Option<usize>,
) {
    let label = ui.label(name);
    ui.horizontal(|ui| {
        if *auto {
            *value = computed.unwrap_or(*value);
            let mut displayed = computed.unwrap_or(*value);
            ui.add_enabled(false, egui::DragValue::new(&mut displayed)).labelled_by(label.id);
        } else {
            ui.add(egui::DragValue::new(value).range(1..=MAX_PANELS)).labelled_by(label.id);
        }
        ui.checkbox(auto, auto_name);
    });
    ui.end_row();
}

fn number(ui: &mut egui::Ui, name: &str, value: &mut f64, max: f64) {
    let label = ui.label(name);
    ui.add(egui::DragValue::new(value).speed(0.1).range(0.0..=max).max_decimals(2)).labelled_by(label.id);
    ui.end_row();
}

fn dimensions(
    groups: usize,
    rows: usize,
    cols: usize,
    auto_rows: bool,
    auto_cols: bool,
) -> Result<(usize, usize), String> {
    let (rows, cols) = match (auto_rows, auto_cols) {
        (true, true) => {
            let rows = (groups as f64).sqrt().ceil() as usize;
            (rows, groups.div_ceil(rows.max(1)))
        }
        (true, false) => (groups.div_ceil(cols.max(1)), cols),
        (false, true) => (rows, groups.div_ceil(rows.max(1))),
        (false, false) => (rows, cols),
    };
    if rows == 0 || cols == 0 || rows.checked_mul(cols).is_none_or(|n| n < groups || n > MAX_PANELS) {
        Err(format!("The grid must fit all groups in at most {MAX_PANELS} cells."))
    } else {
        Ok((rows, cols))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_automatic_grids_and_explicit_grid_capacity_are_preserved() {
        for (groups, expected) in [(1, (1, 1)), (2, (2, 1)), (3, (2, 2)), (5, (3, 2))] {
            assert_eq!(dimensions(groups, 1, 1, true, true).unwrap(), expected);
        }
        assert_eq!(dimensions(5, 2, 3, false, false).unwrap(), (2, 3));
        assert_eq!(dimensions(5, 1, 3, true, false).unwrap(), (2, 3));
        assert_eq!(dimensions(5, 2, 1, false, true).unwrap(), (2, 3));
        for (groups, rows, cols) in [(3, 1, 2), (1, 0, 1), (1, 9, 8), (65, 8, 8)] {
            assert!(dimensions(groups, rows, cols, false, false).is_err());
        }
    }
}
