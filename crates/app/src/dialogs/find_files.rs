//! Edit > Find in Project Files... (T361; Origin 2026's "Find in Project Files", not in Origin 2016, so the window is
//! Sheaf's own): every project file (`.sheaf`, `.opj`, `.opju`) in a folder, and its subfolders when asked, is opened
//! read-only and searched with the rules of Find in Project (`crate::project_find`): names, long names, comments,
//! units, data cells and text on graphs. The files with hits are listed with their hits; Open opens the chosen file
//! (asking first when this project has unsaved changes).

use crate::project_find::{FindHit, FindScope, FindSpec, find_in_project};
use eframe::egui;
use std::path::{Path, PathBuf};

/// The hits of one file, or why it could not be read.
#[derive(Clone, Debug, PartialEq)]
pub struct FileHits {
    pub path: PathBuf,
    pub hits: Vec<FindHit>,
    pub error: Option<String>,
}

/// The project files under `folder` (in name order; subfolders after the files of their parent when `subfolders`).
pub fn project_files(folder: &Path, subfolders: bool) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(folder) else { return Vec::new() };
    let mut entries: Vec<PathBuf> = read.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    let mut out: Vec<PathBuf> =
        entries.iter().filter(|p| p.is_file() && sheaf_io::is_project_file(p)).cloned().collect();
    if subfolders {
        for dir in entries.iter().filter(|p| p.is_dir()) {
            out.extend(project_files(dir, true));
        }
    }
    out
}

/// Searches every project file under `folder` with `spec` (the whole of each project); files without hits are left
/// out, files that cannot be read are kept with their error.
pub fn search_files(folder: &Path, subfolders: bool, spec: &FindSpec) -> Vec<FileHits> {
    let spec = FindSpec { within: FindScope::EntireProject, ..spec.clone() };
    project_files(folder, subfolders)
        .into_iter()
        .filter_map(|path| match sheaf_io::open_project(&path) {
            Ok(opened) => {
                let hits = find_in_project(&opened.project, &spec, &[]);
                (!hits.is_empty()).then_some(FileHits { path, hits, error: None })
            }
            Err(e) => Some(FileHits { path, hits: Vec::new(), error: Some(e.to_string()) }),
        })
        .collect()
}

pub enum FindFilesOutcome {
    Open,
    Closed,
    /// Open this project file.
    OpenFile(PathBuf),
}

pub struct FindFilesDialog {
    pub folder: String,
    pub subfolders: bool,
    pub spec: FindSpec,
    pub results: Option<Vec<FileHits>>,
    pub chosen: Option<usize>,
}

impl FindFilesDialog {
    /// A window starting in `folder` (the folder of the open project, when it has one).
    pub fn new(folder: Option<&Path>) -> Self {
        FindFilesDialog {
            folder: folder.map(|f| f.display().to_string()).unwrap_or_default(),
            subfolders: true,
            spec: FindSpec { data_cells: true, column_units: true, ..FindSpec::default() },
            results: None,
            chosen: None,
        }
    }

    pub fn search(&mut self) {
        self.results = Some(search_files(Path::new(self.folder.trim()), self.subfolders, &self.spec));
        self.chosen = self.results.as_ref().and_then(|r| r.iter().position(|f| f.error.is_none()));
    }

    pub fn show(&mut self, ctx: &egui::Context) -> FindFilesOutcome {
        let mut open = true;
        let mut outcome = FindFilesOutcome::Open;
        egui::Window::new("Find in Project Files")
            .id(egui::Id::new("find_project_files"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                egui::Grid::new("find_files_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    ui.label("Find What");
                    ui.text_edit_singleline(&mut self.spec.text);
                    ui.end_row();
                    ui.label("Folder");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.folder).desired_width(320.0));
                        if ui.button("Browse...").clicked()
                            && let Some(dir) = rfd::FileDialog::new().pick_folder()
                        {
                            self.folder = dir.display().to_string();
                        }
                    });
                    ui.end_row();
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.subfolders, "Include Subfolders");
                    ui.checkbox(&mut self.spec.case_sensitive, "Case Sensitive");
                    ui.checkbox(&mut self.spec.whole_word, "Match Whole Word");
                    ui.checkbox(&mut self.spec.data_cells, "Data in Cells");
                });
                ui.horizontal(|ui| {
                    let can = !self.spec.text.trim().is_empty() && !self.folder.trim().is_empty();
                    if ui.add_enabled(can, egui::Button::new("Find All")).clicked() {
                        self.search();
                    }
                    let openable = self.chosen.is_some();
                    if ui.add_enabled(openable, egui::Button::new("Open")).clicked()
                        && let Some(f) = self.chosen.and_then(|i| self.results.as_ref()?.get(i))
                    {
                        outcome = FindFilesOutcome::OpenFile(f.path.clone());
                    }
                    if ui.button("Close").clicked() {
                        outcome = FindFilesOutcome::Closed;
                    }
                });
                if let Some(results) = &self.results {
                    let files = results.iter().filter(|f| f.error.is_none()).count();
                    let hits: usize = results.iter().map(|f| f.hits.len()).sum();
                    ui.label(format!("{hits} hit(s) in {files} file(s)"));
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        for (i, f) in results.iter().enumerate() {
                            let name = f.path.file_name().map_or(String::new(), |n| n.to_string_lossy().into_owned());
                            let text = match &f.error {
                                Some(e) => format!("{name}: could not be read ({e})"),
                                None => format!("{name}: {} hit(s)", f.hits.len()),
                            };
                            if ui.selectable_label(self.chosen == Some(i), text).clicked() && f.error.is_none() {
                                self.chosen = Some(i);
                            }
                            if self.chosen == Some(i) {
                                for h in f.hits.iter().take(20) {
                                    ui.weak(format!("    {} {}: {}", h.element, h.attribute, h.range));
                                }
                            }
                        }
                    });
                }
            });
        if open { outcome } else { FindFilesOutcome::Closed }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::{ColumnData, Designation, Project};

    fn save(dir: &Path, name: &str, long: &str) -> PathBuf {
        let mut p = Project::new();
        let b = p.add_workbook(long);
        let c = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap();
        p.column_mut(c).unwrap().long_name = format!("{long} current");
        let path = dir.join(name);
        sheaf_io::project_file::save(&p, &path).unwrap();
        path
    }

    #[test]
    fn files_with_hits_are_listed_and_subfolders_follow_the_choice() {
        let dir = std::env::temp_dir().join(format!("sheaf-find-files-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("deeper")).unwrap();
        let cv = save(&dir, "a.sheaf", "Voltammetry");
        save(&dir, "b.sheaf", "Impedance");
        let deep = save(&dir.join("deeper"), "c.sheaf", "Voltammetry");
        std::fs::write(dir.join("notes.txt"), "Voltammetry").unwrap();
        let spec = FindSpec { text: "voltammetry".into(), ..FindSpec::default() };
        let found: Vec<PathBuf> = search_files(&dir, true, &spec).into_iter().map(|f| f.path).collect();
        assert_eq!(found, [cv.clone(), deep]);
        let found: Vec<PathBuf> = search_files(&dir, false, &spec).into_iter().map(|f| f.path).collect();
        assert_eq!(found, [cv]);
        std::fs::write(dir.join("broken.sheaf"), "not a project").unwrap();
        let all = search_files(&dir, false, &spec);
        assert!(all.iter().any(|f| f.error.is_some()), "a file that cannot be read is listed with its error");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
