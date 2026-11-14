//! The three fitting tools that are not NLFit: Analysis > Fitting > Multiple Linear Regression,
//! Compare Models and Rank Models. Each is a small window that picks columns (or functions) and
//! runs the tool on OK; a problem is shown in the window, which stays open.

use super::frame::{DialogButton, button_row};
use eframe::egui;
use sheaf_core::analysis::RowRange;
use sheaf_core::fitting::{
    CompareOutcome, FitModel, FitRequest, FitRequestError, MultiRegOutcome, MultiRegRequest, RankOutcome,
    builtin_functions,
};
use sheaf_core::{ColumnId, Designation, Document, Project, WindowId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitMoreKind {
    Multiple,
    Compare,
    Rank,
}

impl FitMoreKind {
    pub fn title(self) -> &'static str {
        match self {
            FitMoreKind::Multiple => "Multiple Linear Regression",
            FitMoreKind::Compare => "Compare Models",
            FitMoreKind::Rank => "Rank Models",
        }
    }

    /// The name used with `--show fit:NAME`.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "multiple" | "mr" | "regression" => FitMoreKind::Multiple,
            "compare" => FitMoreKind::Compare,
            "rank" => FitMoreKind::Rank,
            _ => return None,
        })
    }
}

pub enum FitMoreOutcome {
    Open,
    Closed,
    Multiple(Box<MultiRegOutcome>),
    Compare(Box<CompareOutcome>),
    Rank(Box<RankOutcome>),
}

/// A model that Compare Models offers: the closed-form fits and every built-in function.
fn model_choices() -> Vec<(String, FitModel)> {
    let mut out = vec![
        ("Linear".to_string(), FitModel::Linear),
        ("Polynomial (order 2)".to_string(), FitModel::Polynomial { order: 2 }),
        ("Polynomial (order 3)".to_string(), FitModel::Polynomial { order: 3 }),
    ];
    out.extend(builtin_functions().into_iter().map(|f| (f.name.clone(), FitModel::BuiltIn(f.name))));
    out
}

pub struct FitMoreDialog {
    book: WindowId,
    kind: FitMoreKind,
    y: ColumnId,
    /// The Y columns of the sheet (id, label).
    y_choices: Vec<(ColumnId, String)>,
    /// Multiple regression: every other numeric column of the sheet, and whether it is an
    /// independent variable.
    x_choices: Vec<(ColumnId, String, bool)>,
    fix_intercept: bool,
    intercept: f64,
    /// Compare: the two models (indexes into `models`).
    models: Vec<(String, FitModel)>,
    model_a: usize,
    model_b: usize,
    /// Rank: the functions with their category, and which are chosen.
    functions: Vec<(String, String, bool)>,
    category: String,
    confidence: f64,
    problem: Option<String>,
}

impl FitMoreDialog {
    /// `y`: the column to fit; `None` takes the first numeric Y column of the active sheet.
    pub fn new(project: &Project, book: WindowId, y: Option<ColumnId>, kind: FitMoreKind) -> Option<Self> {
        let b = project.workbook(book)?;
        let sheet = match y {
            Some(id) => b.sheets.iter().find(|s| s.column(id).is_some())?,
            None => b.sheets.get(b.active_sheet.min(b.sheets.len().checked_sub(1)?))?,
        };
        let label = |c: &sheaf_core::Column| format!("{}: {}", c.short_name, c.axis_title());
        let y_choices: Vec<(ColumnId, String)> = sheet
            .columns
            .iter()
            .filter(|c| c.designation == Designation::Y && c.data.is_numeric() && !c.fit_result)
            .map(|c| (c.id, label(c)))
            .collect();
        let y = match y {
            Some(id) if y_choices.iter().any(|(c, _)| *c == id) => id,
            _ => y_choices.first()?.0,
        };
        let x_choices: Vec<(ColumnId, String, bool)> = sheet
            .columns
            .iter()
            .filter(|c| c.data.is_numeric() && c.id != y && matches!(c.designation, Designation::X | Designation::Y))
            .filter(|c| !c.fit_result)
            .map(|c| (c.id, label(c), false))
            .collect();
        let mut x_choices = x_choices;
        // Start with the sheet's X column (when there is one) as the first variable.
        if let Some(first) =
            x_choices.iter_mut().find(|c| sheet.column(c.0).is_some_and(|col| col.designation == Designation::X))
        {
            first.2 = true;
        } else if let Some(first) = x_choices.first_mut() {
            first.2 = true;
        }
        let models = model_choices();
        let model_b = models.iter().position(|m| m.0 == "Polynomial (order 2)").unwrap_or(1);
        let functions: Vec<(String, String, bool)> =
            builtin_functions().into_iter().map(|f| (f.name, f.category, false)).collect();
        let category = functions.first().map(|f| f.1.clone()).unwrap_or_default();
        Some(FitMoreDialog {
            book,
            kind,
            y,
            y_choices,
            x_choices,
            fix_intercept: false,
            intercept: 0.0,
            models,
            model_a: 0,
            model_b,
            functions,
            category,
            confidence: 95.0,
            problem: None,
        })
    }

    fn categories(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for f in &self.functions {
            if !out.contains(&f.1) {
                out.push(f.1.clone());
            }
        }
        out
    }

    #[cfg(test)]
    pub(crate) fn choose_x_for_test(&mut self, ids: &[ColumnId]) {
        for c in &mut self.x_choices {
            c.2 = ids.contains(&c.0);
        }
    }

    #[cfg(test)]
    pub(crate) fn choose_models_for_test(&mut self, a: &str, b: &str) {
        self.model_a = self.models.iter().position(|m| m.0 == a).expect("model a");
        self.model_b = self.models.iter().position(|m| m.0 == b).expect("model b");
    }

    #[cfg(test)]
    pub(crate) fn choose_functions_for_test(&mut self, names: &[&str]) {
        for f in &mut self.functions {
            f.2 = names.contains(&f.0.as_str());
        }
    }

    #[cfg(test)]
    pub(crate) fn problem_for_test(&self) -> Option<String> {
        self.problem.clone()
    }

    fn base_request(&self) -> FitRequest {
        let mut r = FitRequest::new(self.y, FitModel::Linear);
        r.confidence_level = self.confidence / 100.0;
        r.rows = RowRange::All;
        r
    }

    /// Runs the tool; `None` (with the problem set) when it cannot be run.
    fn run(&mut self, doc: &mut Document) -> Option<FitMoreOutcome> {
        let outcome = match self.kind {
            FitMoreKind::Multiple => {
                let xs: Vec<ColumnId> = self.x_choices.iter().filter(|c| c.2).map(|c| c.0).collect();
                let mut req = MultiRegRequest::new(self.y, xs);
                req.fix_intercept = self.fix_intercept.then_some(self.intercept);
                req.confidence_level = self.confidence / 100.0;
                doc.edit("Multiple Linear Regression", |p| p.fit_multiple_regression(&req))
                    .map(|o| FitMoreOutcome::Multiple(Box::new(o)))
            }
            FitMoreKind::Compare => {
                let (mut a, mut b) = (self.base_request(), self.base_request());
                a.model = self.models[self.model_a].1.clone();
                b.model = self.models[self.model_b].1.clone();
                doc.edit("Compare Models", |p| p.compare_models(&a, &b)).map(|o| FitMoreOutcome::Compare(Box::new(o)))
            }
            FitMoreKind::Rank => {
                let names: Vec<String> = self.functions.iter().filter(|f| f.2).map(|f| f.0.clone()).collect();
                let req = self.base_request();
                doc.edit("Rank Models", |p| p.rank_models(&req, &names)).map(|o| FitMoreOutcome::Rank(Box::new(o)))
            }
        };
        match outcome {
            Ok(o) => Some(o),
            Err(e) => {
                let e: FitRequestError = e;
                self.problem = Some(e.to_string());
                None
            }
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> FitMoreOutcome {
        if doc.project().column(self.y).is_none() || doc.project().workbook(self.book).is_none() {
            return FitMoreOutcome::Closed;
        }
        let mut open = true;
        let mut pressed = None;
        egui::Window::new(self.kind.title())
            .id(egui::Id::new("fit_more_dialog"))
            .open(&mut open)
            .collapsible(false)
            .default_width(380.0)
            .show(ctx, |ui| {
                egui::Grid::new("fit_more_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label(if self.kind == FitMoreKind::Multiple { "Dependent (Y)" } else { "Y data" });
                    let shown = self.y_choices.iter().find(|c| c.0 == self.y).map(|c| c.1.clone()).unwrap_or_default();
                    egui::ComboBox::from_id_salt("fit_more_y").selected_text(shown).show_ui(ui, |ui| {
                        for (id, text) in &self.y_choices {
                            ui.selectable_value(&mut self.y, *id, text);
                        }
                    });
                    ui.end_row();
                    ui.label("Confidence level (%)");
                    ui.add(egui::DragValue::new(&mut self.confidence).range(0.1..=99.99).speed(0.1));
                    ui.end_row();
                });
                ui.separator();
                match self.kind {
                    FitMoreKind::Multiple => {
                        ui.label("Independent variables (X)");
                        egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                            for (id, text, on) in self.x_choices.iter_mut() {
                                if *id != self.y {
                                    ui.checkbox(on, text.as_str());
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.fix_intercept, "Fix intercept at");
                            ui.add_enabled(self.fix_intercept, egui::DragValue::new(&mut self.intercept).speed(0.1));
                        });
                    }
                    FitMoreKind::Compare => {
                        egui::Grid::new("fit_more_models").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                            for (i, label) in ["Model 1", "Model 2"].into_iter().enumerate() {
                                ui.label(label);
                                let slot = if i == 0 { &mut self.model_a } else { &mut self.model_b };
                                egui::ComboBox::from_id_salt(("fit_more_model", i))
                                    .selected_text(self.models[*slot].0.clone())
                                    .show_ui(ui, |ui| {
                                        for (k, m) in self.models.iter().enumerate() {
                                            ui.selectable_value(slot, k, &m.0);
                                        }
                                    });
                                ui.end_row();
                            }
                        });
                        ui.weak("Both models are fitted with their default settings and no weights.");
                    }
                    FitMoreKind::Rank => {
                        let categories = self.categories();
                        ui.horizontal(|ui| {
                            ui.label("Category");
                            egui::ComboBox::from_id_salt("fit_more_category")
                                .selected_text(self.category.clone())
                                .show_ui(ui, |ui| {
                                    for c in &categories {
                                        ui.selectable_value(&mut self.category, c.clone(), c);
                                    }
                                });
                            let category = self.category.clone();
                            if ui.small_button("Select all").clicked() {
                                self.functions.iter_mut().filter(|f| f.1 == category).for_each(|f| f.2 = true);
                            }
                            if ui.small_button("Clear").clicked() {
                                self.functions.iter_mut().for_each(|f| f.2 = false);
                            }
                        });
                        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                            for f in self.functions.iter_mut().filter(|f| f.1 == self.category) {
                                ui.checkbox(&mut f.2, f.0.as_str());
                            }
                        });
                        let n = self.functions.iter().filter(|f| f.2).count();
                        ui.weak(format!("{n} function(s) chosen (in all categories). Ranked by AICc."));
                    }
                }
                if let Some(p) = &self.problem {
                    ui.add_space(4.0);
                    ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
                }
                ui.add_space(6.0);
                pressed = button_row(ui, false);
            });
        match pressed {
            Some(DialogButton::Ok) => {
                self.problem = None;
                if let Some(o) = self.run(doc) {
                    return o;
                }
                FitMoreOutcome::Open
            }
            Some(DialogButton::Cancel) => FitMoreOutcome::Closed,
            _ if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) => FitMoreOutcome::Closed,
            _ => FitMoreOutcome::Open,
        }
    }
}
