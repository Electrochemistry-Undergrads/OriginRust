//! Window > Duplicate (Batch Plotting) (T99): Origin 2016's "Select Worksheet" (the other sheets, Match
//! Plot Columns by) and "Select Column - [Book]Sheet" (the sheet's other Y columns, Use Associated X)
//! windows. The copies are `Project::batch_plot_sheets` / `batch_plot_columns`; rules in
//! `docs/spec/graphing.md`, "Batch plotting".

use eframe::egui::{self, RichText};
use sheaf_core::batch_plot::MatchBy;
use sheaf_core::{ColumnId, Designation, Project, WindowId};

pub enum BatchOutcome {
    Open,
    Closed,
    Sheets { graph: WindowId, sheets: Vec<(WindowId, usize)>, by: MatchBy },
    Columns { graph: WindowId, columns: Vec<ColumnId>, associated_x: bool },
}

pub struct BatchPlotDialog {
    pub graph: WindowId,
    /// With New Columns (else With New Sheets/Books).
    pub columns: bool,
    title: String,
    /// The choices: a sheet (book, index) or a column, with its label and whether it is ticked.
    pub sheet_items: Vec<((WindowId, usize), String, bool)>,
    pub column_items: Vec<(ColumnId, String, bool)>,
    by: MatchBy,
    associated_x: bool,
}

impl BatchPlotDialog {
    /// The window for `graph`; `None` when the graph plots no worksheet column.
    pub fn new(project: &Project, graph: WindowId, columns: bool) -> Option<Self> {
        let (sb, ss) = project.graph_source_sheet(graph)?;
        let source_book = project.workbook(sb)?;
        let source = source_book.sheets.get(ss)?;
        let plotted: Vec<ColumnId> = project.graph(graph)?.layers.iter().flat_map(|l| &l.plots).map(|p| p.y).collect();
        let sheet_items = project
            .workbooks()
            .flat_map(|b| b.sheets.iter().enumerate().map(move |(i, s)| ((b.id, i), format!("[{}]{}", b.name, s.name))))
            .filter(|((b, i), _)| !(*b == sb && *i == ss))
            .map(|(k, label)| (k, label, false))
            .collect();
        let column_items = source
            .columns
            .iter()
            .filter(|c| c.designation == Designation::Y && !plotted.contains(&c.id))
            .map(|c| (c.id, c.short_name.clone(), false))
            .collect();
        let title = if columns {
            format!("Select Column - [{}]{}", source_book.name, source.name)
        } else {
            "Select Worksheet".to_string()
        };
        Some(BatchPlotDialog {
            graph,
            columns,
            title,
            sheet_items,
            column_items,
            by: MatchBy::ColumnIndex,
            associated_x: true,
        })
    }

    pub fn show(&mut self, ctx: &egui::Context) -> BatchOutcome {
        let mut outcome = BatchOutcome::Open;
        let mut open = true;
        egui::Window::new(&self.title)
            .id(egui::Id::new("batch_plot"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                    if self.columns {
                        for (_, label, on) in &mut self.column_items {
                            ui.checkbox(on, label.as_str());
                        }
                        if self.column_items.is_empty() {
                            ui.weak("The sheet has no other Y column.");
                        }
                    } else {
                        for (_, label, on) in &mut self.sheet_items {
                            ui.checkbox(on, label.as_str());
                        }
                        if self.sheet_items.is_empty() {
                            ui.weak("The project has no other sheet.");
                        }
                    }
                });
                if self.columns {
                    ui.checkbox(&mut self.associated_x, "Use Associated X");
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Match Plot Columns by");
                        egui::ComboBox::from_id_salt("batch_match").selected_text(self.by.name()).show_ui(ui, |ui| {
                            for m in MatchBy::ALL {
                                ui.selectable_value(&mut self.by, m, m.name());
                            }
                        });
                    });
                }
                ui.label(RichText::new("Each copy is named \"<graph> - Copy\".").weak());
                let any = if self.columns {
                    self.column_items.iter().any(|c| c.2)
                } else {
                    self.sheet_items.iter().any(|s| s.2)
                };
                ui.horizontal(|ui| {
                    if ui.add_enabled(any, egui::Button::new("OK")).clicked() {
                        outcome = if self.columns {
                            BatchOutcome::Columns {
                                graph: self.graph,
                                columns: self.column_items.iter().filter(|c| c.2).map(|c| c.0).collect(),
                                associated_x: self.associated_x,
                            }
                        } else {
                            BatchOutcome::Sheets {
                                graph: self.graph,
                                sheets: self.sheet_items.iter().filter(|s| s.2).map(|s| s.0).collect(),
                                by: self.by,
                            }
                        };
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = BatchOutcome::Closed;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    outcome = BatchOutcome::Closed;
                }
            });
        if !open { BatchOutcome::Closed } else { outcome }
    }
}
