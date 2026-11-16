//! Multiple Comparisons Plot... (T64): the options of the plot made from the report sheet of a one-way ANOVA with a
//! means comparison or of a two-sample t-test. OK makes the graph as one undo step.

use eframe::egui;
use sheaf_core::compplot::{BracketLabels, ComparisonErrorBar, ComparisonPlotOptions, ComparisonPlotType};
use sheaf_core::{Document, WindowId};

pub enum Outcome {
    Open,
    Closed,
    Created(WindowId),
}

pub struct ComparisonPlotDialog {
    book: WindowId,
    sheet: usize,
    options: ComparisonPlotOptions,
    error: Option<String>,
}

impl ComparisonPlotDialog {
    pub fn new(book: WindowId, sheet: usize) -> Self {
        ComparisonPlotDialog { book, sheet, options: ComparisonPlotOptions::default(), error: None }
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        let mut open = true;
        let mut outcome = Outcome::Open;
        egui::Window::new("Multiple Comparisons Plot")
            .id(egui::Id::new("comparison_plot_dialog"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("comparison_plot_grid").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                    let label = ui.label("Plot Type");
                    egui::ComboBox::from_id_salt("comparison_plot_type")
                        .selected_text(self.options.plot_type.label())
                        .show_ui(ui, |ui| {
                            for t in ComparisonPlotType::ALL {
                                ui.selectable_value(&mut self.options.plot_type, t, t.label());
                            }
                        })
                        .response
                        .labelled_by(label.id);
                    ui.end_row();
                    let label = ui.label("Error Bar");
                    ui.add_enabled_ui(self.options.plot_type.has_error_bars(), |ui| {
                        egui::ComboBox::from_id_salt("comparison_plot_error")
                            .selected_text(self.options.error_bar.label())
                            .show_ui(ui, |ui| {
                                for e in ComparisonErrorBar::ALL {
                                    ui.selectable_value(&mut self.options.error_bar, e, e.label());
                                }
                            })
                            .response
                            .labelled_by(label.id);
                    });
                    ui.end_row();
                    let label = ui.label("Label");
                    egui::ComboBox::from_id_salt("comparison_plot_labels")
                        .selected_text(self.options.labels.label())
                        .show_ui(ui, |ui| {
                            for l in [BracketLabels::Asterisks, BracketLabels::PValues] {
                                ui.selectable_value(&mut self.options.labels, l, l.label());
                            }
                        })
                        .response
                        .labelled_by(label.id);
                    ui.end_row();
                    ui.label("");
                    ui.checkbox(&mut self.options.significant_only, "Significant only");
                    ui.end_row();
                    ui.label("Threshold");
                    ui.add_enabled(
                        self.options.significant_only,
                        egui::DragValue::new(&mut self.options.threshold).range(0.0001..=1.0).speed(0.001),
                    );
                    ui.end_row();
                });
                if let Some(e) = &self.error {
                    ui.colored_label(egui::Color32::from_rgb(180, 0, 0), e);
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        let (book, sheet, options) = (self.book, self.sheet, self.options.clone());
                        match doc.edit("Multiple comparisons plot", |p| p.comparison_plot(book, sheet, &options)) {
                            Ok(graph) => outcome = Outcome::Created(graph),
                            Err(e) => self.error = Some(e),
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = Outcome::Closed;
                    }
                });
            });
        if !open { Outcome::Closed } else { outcome }
    }
}
