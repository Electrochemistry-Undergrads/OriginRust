//! Edit > Find in Project (F3, T95): Origin 2016's "Find" window with Sheaf's search
//! (`crate::project_find`). Fields as Origin's: Find What, Case Sensitive, Match Whole Word, Allow
//! Wildcards, the types to include (Folder, Graph, Book, Sheet, Column, with Folder and Sheet Comments and the column's
//! Units and F(x) behind Select...), Data in Cells (1st Matched in Column) and Text Objects on Graphs.
//! Each hit shows its number, range, element, attribute and folder. Clicking a hit goes to it.

use crate::project_find::{FindHit, FindScope, FindSpec, FindTarget, find_in_project};
use eframe::egui::{self, RichText};
use sheaf_core::Project;

pub enum FindProjectOutcome {
    Open,
    Closed,
    Go(FindTarget),
}

pub struct FindProjectDialog {
    pub spec: FindSpec,
    pub hits: Option<Vec<FindHit>>,
    select: bool,
}

impl FindProjectDialog {
    /// A window with the settings of the last search (Origin keeps them).
    pub fn new(spec: FindSpec) -> Self {
        FindProjectDialog { spec, hits: None, select: false }
    }

    pub fn search(&mut self, project: &Project, active_folder: &[usize]) {
        self.hits = Some(find_in_project(project, &self.spec, active_folder));
    }

    pub fn show(&mut self, ctx: &egui::Context, project: &Project, active_folder: &[usize]) -> FindProjectOutcome {
        let mut folder = &project.root;
        let mut names = vec![folder.name.as_str()];
        for &index in active_folder {
            let Some(child) = folder.folders.get(index) else { break };
            folder = child;
            names.push(folder.name.as_str());
        }
        let active_folder = &active_folder[..names.len() - 1];
        let mut outcome = FindProjectOutcome::Open;
        let mut open = true;
        egui::Window::new("Find")
            .id(egui::Id::new("find_in_project"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                let mut go = false;
                ui.horizontal(|ui| {
                    ui.label("Current Path");
                    ui.label(format!("/{}/", names.join("/")));
                });
                ui.horizontal(|ui| {
                    ui.label("Find What");
                    let r = ui.add(egui::TextEdit::singleline(&mut self.spec.text).desired_width(260.0));
                    go |= r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    go |= ui.add_enabled(!self.spec.text.is_empty(), egui::Button::new("Find")).clicked();
                    if ui.button("Cancel").clicked() {
                        outcome = FindProjectOutcome::Closed;
                    }
                });
                ui.horizontal(|ui| {
                    let label = ui.label("Within");
                    egui::ComboBox::from_id_salt("find_scope")
                        .selected_text(self.spec.within.name())
                        .show_ui(ui, |ui| {
                            for scope in FindScope::ALL {
                                ui.selectable_value(&mut self.spec.within, scope, scope.name());
                            }
                        })
                        .response
                        .labelled_by(label.id);
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.spec.case_sensitive, "Case Sensitive");
                    ui.checkbox(&mut self.spec.whole_word, "Match Whole Word");
                    ui.checkbox(&mut self.spec.wildcards, "Allow Wildcards (* and ?)");
                });
                ui.group(|ui| {
                    ui.label("Types to be Included");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.spec.folders, "Folder");
                        ui.checkbox(&mut self.spec.graphs, "Graph");
                        ui.checkbox(&mut self.spec.books, "Book");
                        ui.checkbox(&mut self.spec.sheets, "Sheet");
                        ui.checkbox(&mut self.spec.columns, "Column");
                        if ui.button("Select...").clicked() {
                            self.select = !self.select;
                        }
                    });
                    if self.select {
                        ui.checkbox(&mut self.spec.folder_comments, "Folder Comments");
                        ui.horizontal(|ui| {
                            ui.label("Sheet also:");
                            ui.checkbox(&mut self.spec.sheet_comments, "Comments");
                        });
                        ui.horizontal(|ui| {
                            ui.label("Column also:");
                            ui.checkbox(&mut self.spec.column_units, "Units");
                            ui.checkbox(&mut self.spec.column_formula, "F(x)");
                        });
                    }
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.spec.data_cells, "Data in Cells (1st Matched in Column)");
                        ui.checkbox(&mut self.spec.graph_text, "Text Objects on Graphs");
                    });
                });
                if go && !self.spec.text.is_empty() {
                    self.search(project, active_folder);
                }
                if let Some(hits) = &self.hits {
                    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                        egui::Grid::new("find_project_hits").striped(true).num_columns(5).show(ui, |ui| {
                            for head in ["No.", "Range", "Element", "Attribute", "Folder"] {
                                ui.label(RichText::new(head).strong());
                            }
                            ui.end_row();
                            for (i, h) in hits.iter().enumerate() {
                                ui.label((i + 1).to_string());
                                if ui.selectable_label(false, &h.range).clicked() {
                                    outcome = FindProjectOutcome::Go(h.target.clone());
                                }
                                ui.label(h.element);
                                ui.label(h.attribute);
                                ui.label(&h.folder);
                                ui.end_row();
                            }
                        });
                    });
                    ui.label(format!("Found: {}", hits.len()));
                }
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    outcome = FindProjectOutcome::Closed;
                }
            });
        if !open { FindProjectOutcome::Closed } else { outcome }
    }
}
