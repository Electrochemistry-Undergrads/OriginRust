//! Graph > Add Error Bars: Y errors for one plot, generated as Origin's Error Bars window does
//! (Percent of Data, Standard Deviation of Data with a Scaling Factor, Square Root of Data, T241)
//! into a new Y Error column next to the plot's Y column, or taken from an existing column
//! (graphing spec 14.13a).

use eframe::egui;
use sheaf_core::graph::Plot;
use sheaf_core::{ColumnId, Designation, Document, Project, WindowId};

/// Where the Y errors come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// The given percent of each Y value (its magnitude).
    Percent,
    /// The standard deviation of the Y values times the scaling factor, the same for every row.
    StdDev,
    /// The square root of each Y value.
    SquareRoot,
    /// A numeric column that holds the errors (Sheaf's chooser).
    Column,
}

impl Source {
    /// The formula of the generated column (Set Column Values shows it) and its comment.
    pub fn formula(self, y: &str, percent: f64, scale: f64) -> Option<(String, String)> {
        match self {
            Source::Percent => {
                Some((format!("abs(col({y}))*{percent}/100"), format!("Error bars: Percent of Data {percent}%")))
            }
            Source::StdDev => Some((
                format!("sd(col({y}))*{scale}"),
                format!("Error bars: Standard Deviation of Data, Scaling Factor {scale}"),
            )),
            Source::SquareRoot => Some((format!("sqrt(col({y}))"), "Error bars: Square Root of Data".to_string())),
            Source::Column => None,
        }
    }
}

pub enum Outcome {
    Open,
    Closed,
    Added,
}

struct Target {
    label: String,
    plot: Plot,
}

pub struct ErrorBarsDialog {
    graph: WindowId,
    layer: usize,
    targets: Vec<Target>,
    target: usize,
    column: Option<ColumnId>,
    source: Source,
    percent: f64,
    scale: f64,
}

impl ErrorBarsDialog {
    pub fn new(project: &Project, graph: WindowId, layer: usize, selected: Option<usize>) -> Option<Self> {
        let plots = &project.graph(graph)?.layers.get(layer)?.plots;
        if plots.is_empty() {
            return None;
        }
        let targets = plots
            .iter()
            .enumerate()
            .map(|(i, plot)| Target {
                label: format!("{}: {}", i + 1, column_name(project, plot.y)),
                plot: plot.clone(),
            })
            .collect();
        let target = selected.filter(|i| *i < plots.len()).unwrap_or(0);
        let mut dialog =
            Self { graph, layer, targets, target, column: None, source: Source::Percent, percent: 5.0, scale: 1.0 };
        dialog.choose_default(project);
        Some(dialog)
    }

    /// Chooses the source of the errors (tests use it; the window has radio buttons).
    #[cfg(test)]
    pub fn set_source(&mut self, source: Source, percent: f64, scale: f64) {
        (self.source, self.percent, self.scale) = (source, percent, scale);
    }

    fn choose_default(&mut self, project: &Project) {
        let plot = &self.targets[self.target].plot;
        self.column = plot.y_error.or_else(|| {
            let (_, sheet, _) = project.locate(plot.y)?;
            sheet.y_error_for(sheet.column_index(plot.y)?).map(|c| c.id)
        });
        if !self.column.is_some_and(|c| project.column(c).is_some_and(|c| c.data.is_numeric())) {
            self.column = None;
        }
        // A plot that has or sits next to an error column starts on it; otherwise on Origin's first
        // choice, Percent of Data.
        self.source = if self.column.is_some() { Source::Column } else { Source::Percent };
    }

    /// Whether the numbers of the chosen source can be used.
    fn source_ready(&self, chosen_column: bool) -> bool {
        match self.source {
            Source::Percent => self.percent.is_finite() && self.percent > 0.0,
            Source::StdDev => self.scale.is_finite() && self.scale > 0.0,
            Source::SquareRoot => true,
            Source::Column => chosen_column,
        }
    }

    fn target_valid(&self, project: &Project) -> bool {
        project
            .graph(self.graph)
            .and_then(|g| g.layers.get(self.layer))
            .and_then(|l| l.plots.get(self.target))
            .is_some_and(|p| {
                *p == self.targets[self.target].plot && project.column(p.y).is_some_and(|c| c.data.is_numeric())
            })
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        if doc.project().graph(self.graph).is_none() || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Outcome::Closed;
        }
        let mut open = true;
        let mut outcome = Outcome::Open;
        egui::Window::new("Error Bars")
            .id(egui::Id::new("add_error_bars"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                let before = self.target;
                ui.horizontal(|ui| {
                    let label = ui.label("Add Error Bars to");
                    egui::ComboBox::from_id_salt("error_bar_target")
                        .selected_text(&self.targets[self.target].label)
                        .show_ui(ui, |ui| {
                            for (i, target) in self.targets.iter().enumerate() {
                                ui.selectable_value(&mut self.target, i, &target.label);
                            }
                        })
                        .response
                        .labelled_by(label.id);
                });
                if before != self.target {
                    self.choose_default(doc.project());
                }
                ui.separator();
                ui.label("Source of Errors");
                // T241: Origin's three generated sources, then Sheaf's existing-column chooser.
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.source, Source::Percent, "Percent of Data (%)");
                    ui.add_enabled(
                        self.source == Source::Percent,
                        egui::DragValue::new(&mut self.percent).range(0.0..=1e6).speed(0.5),
                    );
                });
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.source, Source::StdDev, "Standard Deviation of Data");
                    ui.label("Scaling Factor");
                    ui.add_enabled(
                        self.source == Source::StdDev,
                        egui::DragValue::new(&mut self.scale).range(0.0..=1e6).speed(0.1),
                    );
                });
                ui.radio_value(&mut self.source, Source::SquareRoot, "Square Root of Data");
                ui.radio_value(&mut self.source, Source::Column, "Existing Column");
                let sources = numeric_columns(doc.project());
                let chosen = self.column.and_then(|c| sources.iter().find(|(id, _)| *id == c));
                ui.add_enabled_ui(self.source == Source::Column, |ui| {
                    ui.horizontal(|ui| {
                        let label = ui.label("Y Error Column");
                        egui::ComboBox::from_id_salt("error_bar_column")
                            .selected_text(chosen.map_or("Select a column", |(_, name)| name.as_str()))
                            .show_ui(ui, |ui| {
                                for (id, name) in &sources {
                                    ui.selectable_value(&mut self.column, Some(*id), name);
                                }
                            })
                            .response
                            .labelled_by(label.id);
                    });
                    if sources.is_empty() {
                        ui.label("No numeric columns are available.");
                    }
                });
                if self.source != Source::Column {
                    ui.weak("A new Y Error column next to the plot's Y column holds the errors.");
                }
                let valid = self.target_valid(doc.project());
                if !valid {
                    ui.colored_label(ui.visuals().error_fg_color, "The plot changed. Reopen Add Error Bars.");
                }
                if self.targets[self.target].plot.y_error.is_some() {
                    ui.label("OK replaces the Y error column of this plot.");
                }
                ui.horizontal(|ui| {
                    if ui.add_enabled(valid && self.source_ready(chosen.is_some()), egui::Button::new("OK")).clicked() {
                        self.apply(doc);
                        outcome = Outcome::Added;
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = Outcome::Closed;
                    }
                });
            });
        if !open {
            return Outcome::Closed;
        }
        outcome
    }

    fn apply(&self, doc: &mut Document) {
        if self.source != Source::Column {
            self.generate(doc);
            return;
        }
        // The dialog can stay open while a column or plot changes elsewhere. Recheck before
        // touching the model rather than assigning an old column or plot index.
        if !self.target_valid(doc.project())
            || !self.column.is_some_and(|c| doc.project().column(c).is_some_and(|c| c.data.is_numeric()))
        {
            return;
        }
        doc.apply("Add error bars", |project| {
            let plot = &mut project.graph_mut(self.graph).unwrap().layers[self.layer].plots[self.target];
            plot.y_error = self.column;
        });
    }
}

impl ErrorBarsDialog {
    /// T241: a new Y Error column right of the plot's Y column, filled by the source's formula (kept
    /// on the column for Set Column Values, run once) with a comment naming the source, and set as
    /// the plot's Y errors; one undo step.
    fn generate(&self, doc: &mut Document) {
        if !self.target_valid(doc.project()) || !self.source_ready(false) {
            return;
        }
        let y = self.targets[self.target].plot.y;
        let Some((book, sheet, at, short)) = doc.project().locate(y).and_then(|(b, s, c)| {
            let sheet = b.sheets.iter().position(|w| std::ptr::eq(w, s))?;
            Some((b.id, sheet, s.column_index(y)?, c.short_name.clone()))
        }) else {
            return;
        };
        let Some((formula, comment)) = self.source.formula(&short, self.percent, self.scale) else { return };
        let (graph, layer, target) = (self.graph, self.layer, self.target);
        doc.apply("Add error bars", |p| {
            let Ok(ids) = p.insert_columns(book, sheet, at + 1, 1) else { return };
            let id = ids[0];
            if let Some(c) = p.column_mut(id) {
                c.designation = Designation::YError;
                c.comments = comment;
            }
            // The formula only reads the Y column of the same sheet; it cannot fail on a sound sheet.
            let _ = p.set_column_values(id, &formula, None);
            if let Some(plot) =
                p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)).and_then(|l| l.plots.get_mut(target))
            {
                plot.y_error = Some(id);
            }
        });
    }
}

fn column_name(project: &Project, column: ColumnId) -> String {
    let Some((book, sheet, c)) = project.locate(column) else { return "Missing column".into() };
    let long = if c.long_name.is_empty() { String::new() } else { format!(" ({})", c.long_name) };
    format!("[{}]{}!{}{long}", book.name, sheet.name, c.short_name)
}

fn numeric_columns(project: &Project) -> Vec<(ColumnId, String)> {
    let mut sources = Vec::new();
    for book in project.workbooks() {
        for sheet in &book.sheets {
            for c in &sheet.columns {
                if c.data.is_numeric() {
                    let long = if c.long_name.is_empty() { String::new() } else { format!(" ({})", c.long_name) };
                    sources.push((c.id, format!("[{}]{}!{}{long}", book.name, sheet.name, c.short_name)));
                }
            }
        }
    }
    sources
}
