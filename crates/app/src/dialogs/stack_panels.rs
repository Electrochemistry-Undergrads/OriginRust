//! Plot > Multi-Panel > Stack, over the existing panel builder (graphing spec 15.7a).

use eframe::egui;
use sheaf_core::graph::{LegendPosition, TickDirection};
use sheaf_core::panels::{MAX_PANELS, PanelLayout, PanelOrder};
use sheaf_core::template::Template;
use sheaf_core::{ColumnId, Document, Project, WindowId};
use std::ops::Range;

pub enum Outcome {
    Open,
    Closed,
    Created(WindowId),
}

pub struct StackPanelsDialog {
    book: WindowId,
    columns: Vec<ColumnId>,
    rows: Option<(usize, usize)>,
    input: Vec<String>,
    template: Template,
    portrait: bool,
    order: PanelOrder,
    legend: bool,
    link: bool,
    link_x: bool,
    counts: String,
    gap: f64,
    margins: [f64; 4],
    problem: Option<String>,
}

impl StackPanelsDialog {
    pub fn new(
        project: &Project,
        book: WindowId,
        columns: Vec<ColumnId>,
        rows: Option<(usize, usize)>,
    ) -> Option<Self> {
        project.workbook(book)?;
        if columns.is_empty() {
            return None;
        }
        let input = columns
            .iter()
            .filter_map(|id| project.locate(*id))
            .map(|(book, sheet, c)| format!("[{}]{}!{} ({})", book.name, sheet.name, c.short_name, c.axis_title()))
            .collect();
        Some(Self {
            book,
            columns,
            rows,
            input,
            template: Template::Line,
            portrait: true,
            order: PanelOrder::BottomUp,
            legend: true,
            link: true,
            link_x: true,
            counts: "1".into(),
            gap: 0.0,
            margins: [15.0, 10.0, 10.0, 10.0],
            problem: None,
        })
    }

    fn prepare(&self, project: &Project) -> Result<(Vec<Vec<ColumnId>>, PanelLayout), String> {
        if self.columns.iter().any(|id| project.column(*id).is_none_or(|c| !c.data.is_numeric())) {
            return Err("The input columns changed. Close this window and select them again.".into());
        }
        let ranges = assignment(&self.counts, self.columns.len())?;
        let mut layout = PanelLayout::stack(ranges.len());
        layout.order = self.order;
        layout.link_x = self.link && self.link_x;
        layout.v_gap = self.gap;
        [layout.margin_left, layout.margin_right, layout.margin_top, layout.margin_bottom] = self.margins;
        layout.frames().map_err(|e| e.to_string())?;
        Ok((ranges.into_iter().map(|r| self.columns[r].to_vec()).collect(), layout))
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        if doc.project().workbook(self.book).is_none() || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Outcome::Closed;
        }
        let mut open = true;
        let mut result = Outcome::Open;
        egui::Window::new("Stack")
            .id(egui::Id::new("stack_panels"))
            .open(&mut open)
            .collapsible(false)
            .default_width(470.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(560.0).show(ui, |ui| {
                    ui.collapsing(format!("Input: {} Y column(s)", self.columns.len()), |ui| {
                        for name in &self.input {
                            ui.label(name);
                        }
                    });
                    if let Some((first, last)) = self.rows {
                        ui.label(format!("Rows {} to {last}", first + 1));
                    }
                    egui::Grid::new("stack_plot_type").num_columns(2).show(ui, |ui| {
                        let label = ui.label("Plot Type");
                        let name = match self.template {
                            Template::Scatter => "Scatter",
                            Template::LineSymbol => "Line+Symbol",
                            Template::Column => "Column",
                            _ => "Line",
                        };
                        egui::ComboBox::from_id_salt("stack_type")
                            .selected_text(name)
                            .show_ui(ui, |ui| {
                                for (kind, text) in [
                                    (Template::Line, "Line"),
                                    (Template::Scatter, "Scatter"),
                                    (Template::LineSymbol, "Line+Symbol"),
                                    (Template::Column, "Column"),
                                ] {
                                    ui.selectable_value(&mut self.template, kind, text);
                                }
                                for text in ["Use Template", "Custom"] {
                                    ui.add_enabled(false, egui::Button::selectable(false, text));
                                }
                            })
                            .response
                            .labelled_by(label.id);
                        ui.end_row();
                        ui.label("Graph Template");
                        ui.add_enabled(false, egui::Label::new("Built-in"));
                        ui.end_row();
                    });
                    egui::CollapsingHeader::new("Options").default_open(true).show(ui, |ui| {
                        egui::Grid::new("stack_options").num_columns(2).show(ui, |ui| {
                            let label = ui.label("Orientation");
                            egui::ComboBox::from_id_salt("stack_orientation")
                                .selected_text(if self.portrait { "Portrait" } else { "Landscape" })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.portrait, true, "Portrait");
                                    ui.selectable_value(&mut self.portrait, false, "Landscape");
                                })
                                .response
                                .labelled_by(label.id);
                            ui.end_row();
                            let label = ui.label("Stack Direction");
                            egui::ComboBox::from_id_salt("stack_direction")
                                .selected_text("Vertical")
                                .show_ui(ui, |ui| {
                                    ui.label("Vertical");
                                    ui.add_enabled(
                                        false,
                                        egui::Button::selectable(false, "Horizontal (X-Y Axes Exchanged)"),
                                    );
                                })
                                .response
                                .labelled_by(label.id);
                            ui.end_row();
                            let label = ui.label("Layer Order");
                            egui::ComboBox::from_id_salt("stack_order")
                                .selected_text(if self.order == PanelOrder::BottomUp {
                                    "Bottom to Top"
                                } else {
                                    "Top to Bottom"
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.order, PanelOrder::BottomUp, "Bottom to Top");
                                    ui.selectable_value(&mut self.order, PanelOrder::TopDown, "Top to Bottom");
                                })
                                .response
                                .labelled_by(label.id);
                            ui.end_row();
                            let label = ui.label("Legend");
                            egui::ComboBox::from_id_salt("stack_legend")
                                .selected_text(if self.legend { "Legend for Each Layer" } else { "No Legend" })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.legend, true, "Legend for Each Layer");
                                    ui.selectable_value(&mut self.legend, false, "No Legend");
                                    ui.add_enabled(false, egui::Button::selectable(false, "One Legend for Whole Page"));
                                })
                                .response
                                .labelled_by(label.id);
                            ui.end_row();
                            ui.label("");
                            ui.checkbox(&mut self.link, "Link Layers");
                            ui.end_row();
                            let label = ui.label("X Axis Link");
                            ui.add_enabled_ui(self.link, |ui| {
                                egui::ComboBox::from_id_salt("stack_link_x")
                                    .selected_text(if self.link_x { "Straight (1 to 1)" } else { "None" })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.link_x, true, "Straight (1 to 1)");
                                        ui.selectable_value(&mut self.link_x, false, "None");
                                    })
                                    .response
                                    .labelled_by(label.id);
                            });
                            ui.end_row();
                            ui.label("Unit");
                            ui.label(if self.link { "% of Linked Layer" } else { "% of Page" });
                            ui.end_row();
                        });
                    });
                    egui::CollapsingHeader::new("Plot Assignment").default_open(true).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Number of Layers");
                            ui.label(
                                assignment(&self.counts, self.columns.len())
                                    .map_or_else(|_| "—".into(), |r| r.len().to_string()),
                            );
                            ui.add_enabled(false, egui::Checkbox::new(&mut true, "Auto"));
                        });
                        ui.horizontal(|ui| {
                            let label = ui.label("Number of Plots in Each Layer");
                            ui.add(egui::TextEdit::singleline(&mut self.counts).desired_width(160.0))
                                .labelled_by(label.id);
                        });
                        ui.label("Use spaces between counts; the final count repeats.");
                        if let Ok(ranges) = assignment(&self.counts, self.columns.len()) {
                            let counts = ranges.iter().map(|r| r.len().to_string()).collect::<Vec<_>>().join(" ");
                            ui.label(format!("{} plots; layer counts: {counts}", self.columns.len()));
                        }
                    });
                    egui::CollapsingHeader::new("Spacing (in % of Page Dimension)").default_open(true).show(ui, |ui| {
                        egui::Grid::new("stack_spacing").num_columns(2).show(ui, |ui| {
                            number(ui, "Vertical Gap", &mut self.gap, 50.0);
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
                    ui.add_enabled(false, egui::Button::new("Preview"));
                    if ui.add_enabled(prepared.is_ok(), egui::Button::new("OK")).clicked() {
                        let (groups, layout) = prepared.unwrap();
                        match doc.edit("Stack panels", |p| {
                            let id = p.plot_panels(self.template, &groups, &layout)?;
                            let graph = p.graph_mut(id).unwrap();
                            for layer in &mut graph.layers {
                                layer.legend.visible = self.legend;
                                layer.legend.position = LegendPosition::TopLeft;
                                layer.legend.frame = true;
                                for axis in [&mut layer.x_axis, &mut layer.y_axis] {
                                    axis.major_tick = TickDirection::In;
                                    axis.minor_tick = TickDirection::In;
                                }
                                for plot in &mut layer.plots {
                                    plot.rows = self.rows;
                                }
                            }
                            if self.link && !self.link_x {
                                for i in 1..graph.layers.len() {
                                    graph.set_link(i, Some(0));
                                }
                            }
                            let page = &mut graph.page;
                            if (page.width_pt > page.height_pt) == self.portrait {
                                std::mem::swap(&mut page.width_pt, &mut page.height_pt);
                            }
                            Ok::<_, sheaf_core::project::ProjectError>(id)
                        }) {
                            Ok(graph) => result = Outcome::Created(graph),
                            Err(e) => self.problem = Some(e.to_string()),
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        result = Outcome::Closed;
                    }
                });
            });
        if !open { Outcome::Closed } else { result }
    }
}

fn number(ui: &mut egui::Ui, name: &str, value: &mut f64, max: f64) {
    let label = ui.label(name);
    ui.add(egui::DragValue::new(value).speed(0.1).range(0.0..=max).max_decimals(2)).labelled_by(label.id);
    ui.end_row();
}

fn assignment(text: &str, total: usize) -> Result<Vec<Range<usize>>, String> {
    let counts = text
        .split_whitespace()
        .map(|s| s.parse::<usize>().ok().filter(|n| *n > 0))
        .collect::<Option<Vec<_>>>()
        .filter(|c| !c.is_empty())
        .ok_or("Enter positive whole counts separated by spaces.")?;
    if total == 0 {
        return Err("Select at least one numeric Y column.".into());
    }
    let mut ranges = Vec::new();
    let mut start = 0;
    while start < total {
        if ranges.len() == MAX_PANELS {
            return Err(format!("A stack can have at most {MAX_PANELS} layers."));
        }
        let count = counts[ranges.len().min(counts.len() - 1)].min(total - start);
        ranges.push(start..start + count);
        start += count;
    }
    Ok(ranges)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn measured_origin_assignment_repeats_the_last_count_and_clips_the_final_layer() {
        let sizes = |text| assignment(text, 5).unwrap().into_iter().map(|r| r.len()).collect::<Vec<_>>();
        assert_eq!(sizes("1"), [1, 1, 1, 1, 1]);
        assert_eq!(sizes("2"), [2, 2, 1]);
        assert_eq!(sizes("1 2"), [1, 2, 2]);
    }
    #[test]
    fn invalid_counts_and_too_many_layers_cannot_create_a_stack() {
        for value in ["", "0", "-1", "1.5", "1 nope"] {
            assert!(assignment(value, 5).is_err());
        }
        assert!(assignment("1", 65).is_err());
        assert_eq!(assignment("2", 65).unwrap().len(), 33);
        assert!(assignment("1", 0).is_err());
    }
}
