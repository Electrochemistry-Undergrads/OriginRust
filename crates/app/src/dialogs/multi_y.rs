//! Plot > Multi-Y > Multiple Y Axes... (T61, graphing spec 21.6): one layer per Y axis with a choice of how many
//! axes stand left and right, a middle axis, plots per layer, colours, titles, a top X axis and the gap. OK makes
//! the graph as one undo step.

use eframe::egui;
use sheaf_core::multi_y::{MultiYOptions, MultiYPlotType};
use sheaf_core::{ColumnId, Document, WindowId};

pub enum Outcome {
    Open,
    Closed,
    Created(WindowId),
}

pub struct MultiYDialog {
    columns: Vec<ColumnId>,
    options: MultiYOptions,
    left_auto: bool,
    left: usize,
    right_auto: bool,
    right: usize,
    per_layer_text: String,
    gap_auto: bool,
    gap: f64,
    error: Option<String>,
}

impl MultiYDialog {
    pub fn new(columns: Vec<ColumnId>) -> Self {
        MultiYDialog {
            columns,
            options: MultiYOptions::default(),
            left_auto: true,
            left: 1,
            right_auto: true,
            right: 1,
            per_layer_text: String::new(),
            gap_auto: true,
            gap: 10.0,
            error: None,
        }
    }

    /// The options as the controls show them, or why the numbers of plots cannot be read.
    fn options(&self) -> Result<MultiYOptions, String> {
        let mut o = self.options.clone();
        o.left = (!self.left_auto).then_some(self.left);
        o.right = (!self.right_auto).then_some(self.right);
        o.gap_pct = (!self.gap_auto).then_some(self.gap);
        o.per_layer.clear();
        for word in self.per_layer_text.split_whitespace() {
            o.per_layer.push(word.parse::<usize>().map_err(|_| format!("'{word}' is not a number of plots"))?);
        }
        Ok(o)
    }

    /// The blue summary lines of Origin's dialog: the axes and plots in total and the plots in each layer.
    fn summary(&self) -> (String, String) {
        let n = self.columns.len();
        match self.options().and_then(|o| o.plots_per_layer(n).map_err(|e| e.to_string())) {
            Ok(per) => (
                format!("{} Y axes and {n} plots in total", per.len()),
                format!(
                    "Actual number of plots in each layer : {}",
                    per.iter().map(usize::to_string).collect::<Vec<_>>().join(" ")
                ),
            ),
            Err(e) => (e, String::new()),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        let mut open = true;
        let mut outcome = Outcome::Open;
        egui::Window::new("Multiple Y Axes")
            .id(egui::Id::new("multi_y_dialog"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("multi_y_grid").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                    let label = ui.label("Plot Type");
                    egui::ComboBox::from_id_salt("multi_y_type")
                        .selected_text(self.options.plot_type.label())
                        .show_ui(ui, |ui| {
                            for t in MultiYPlotType::ALL {
                                ui.selectable_value(&mut self.options.plot_type, t, t.label());
                            }
                        })
                        .response
                        .labelled_by(label.id);
                    ui.end_row();
                    ui.label("Number of Left Y Axes");
                    ui.horizontal(|ui| {
                        ui.add_enabled(!self.left_auto, egui::DragValue::new(&mut self.left).range(0..=30));
                        ui.checkbox(&mut self.left_auto, "Auto##left");
                    });
                    ui.end_row();
                    ui.label("Number of Right Y Axes");
                    ui.horizontal(|ui| {
                        ui.add_enabled(!self.right_auto, egui::DragValue::new(&mut self.right).range(0..=30));
                        ui.checkbox(&mut self.right_auto, "Auto##right");
                    });
                    ui.end_row();
                    ui.label("");
                    ui.checkbox(&mut self.options.middle, "Add Middle Y Axis");
                    ui.end_row();
                    let label = ui.label("Number of Plots in Each Layer");
                    ui.add(egui::TextEdit::singleline(&mut self.per_layer_text).hint_text("Example: 1 2 3"))
                        .labelled_by(label.id);
                    ui.end_row();
                });
                let (total, per) = self.summary();
                ui.colored_label(egui::Color32::from_rgb(0, 70, 190), total);
                ui.colored_label(egui::Color32::from_rgb(0, 70, 190), per);
                ui.separator();
                ui.checkbox(&mut self.options.axis_color, "Link Axis Color to Plot");
                ui.checkbox(&mut self.options.y_titles, "Show Y Title");
                ui.checkbox(&mut self.options.top_x, "Show Top X Axis");
                ui.horizontal(|ui| {
                    ui.label("Y Axes Gap (%)");
                    ui.add_enabled(!self.gap_auto, egui::DragValue::new(&mut self.gap).range(0.0..=100.0));
                    ui.checkbox(&mut self.gap_auto, "Auto##gap");
                });
                if let Some(e) = &self.error {
                    ui.colored_label(egui::Color32::from_rgb(180, 0, 0), e);
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        match self.options() {
                            Err(e) => self.error = Some(e),
                            Ok(options) => {
                                let columns = self.columns.clone();
                                match doc.edit("Plot columns", |p| p.plot_multi_y(&options, &columns)) {
                                    Ok(graph) => outcome = Outcome::Created(graph),
                                    Err(e) => self.error = Some(e.to_string()),
                                }
                            }
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
