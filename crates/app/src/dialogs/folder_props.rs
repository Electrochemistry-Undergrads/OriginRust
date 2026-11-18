//! The Project Explorer folder's Properties... (T100): Origin 2016's "Folder Properties - <name>" window
//! with Name, Comments and Page Info (Type Folder, Contains N Windows, Location). Rules in
//! `docs/spec/workbook-model.md`, "Folders".

use crate::panels::explorer;
use eframe::egui;
use sheaf_core::Project;
use sheaf_core::project::Folder;

pub enum FolderOutcome {
    Open,
    Closed,
    Set { path: Vec<usize>, name: String, comments: String },
}

pub struct FolderPropsDialog {
    pub path: Vec<usize>,
    pub name: String,
    pub comments: String,
}

impl FolderPropsDialog {
    pub fn new(path: Vec<usize>, folder: &Folder) -> Self {
        FolderPropsDialog { path, name: folder.name.clone(), comments: folder.comments.clone() }
    }

    pub fn show(&mut self, ctx: &egui::Context, project: &Project) -> FolderOutcome {
        let mut outcome = FolderOutcome::Open;
        let mut open = true;
        let windows = explorer::folder_at(&project.root, &self.path).map_or(0, |f| f.windows.len());
        let windows = if self.path.is_empty() { explorer::windows_in(project, &[]).len() } else { windows };
        let location = explorer::folder_list(&project.root)
            .into_iter()
            .find(|(p, _)| !self.path.is_empty() && *p == self.path[..self.path.len() - 1])
            .map_or_else(|| "/".to_string(), |(_, n)| n);
        let title = explorer::folder_at(&project.root, &self.path).map_or(String::new(), |f| f.name.clone());
        egui::Window::new(format!("Folder Properties - {title}"))
            .id(egui::Id::new("folder_properties"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Grid::new("folder_props_grid").num_columns(2).show(ui, |ui| {
                    ui.label("Name:");
                    ui.add_enabled(
                        !self.path.is_empty(),
                        egui::TextEdit::singleline(&mut self.name).desired_width(260.0),
                    );
                    ui.end_row();
                    ui.label("Comments:");
                    ui.add(egui::TextEdit::multiline(&mut self.comments).desired_width(260.0).desired_rows(4));
                    ui.end_row();
                });
                ui.group(|ui| {
                    ui.label("Page Info");
                    egui::Grid::new("folder_props_info").num_columns(2).show(ui, |ui| {
                        ui.label("Type:");
                        ui.label("Folder");
                        ui.end_row();
                        ui.label("Contains:");
                        ui.label(format!("{windows} Windows"));
                        ui.end_row();
                        ui.label("Location:");
                        ui.label(location);
                        ui.end_row();
                    });
                });
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        outcome = FolderOutcome::Set {
                            path: self.path.clone(),
                            name: self.name.clone(),
                            comments: self.comments.clone(),
                        };
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = FolderOutcome::Closed;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    outcome = FolderOutcome::Closed;
                }
            });
        if !open { FolderOutcome::Closed } else { outcome }
    }
}
