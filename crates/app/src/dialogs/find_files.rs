//! File > Find in Project Files... (T361; Origin 2026's "Find in Project Files", not in Origin 2016, so the window is
//! Sheaf's own): every project file (`.sheaf`, `.opj`, `.opju`) in a folder, and its subfolders when asked, is opened
//! read-only and searched with the rules of Find in Project (`crate::project_find`): names, long names, comments,
//! units, data cells and text on graphs. The files with hits are listed with their hits; Open opens the chosen file
//! (asking first when this project has unsaved changes).
//!
//! Find All searches on a background thread (T441): the files appear as they are searched, a line counts them, Stop
//! ends the search after the file in hand, and the window and the rest of the program stay responsive.

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
/// (The window runs the same search on a thread, file by file, T441.)
#[cfg(test)]
pub fn search_files(folder: &Path, subfolders: bool, spec: &FindSpec) -> Vec<FileHits> {
    project_files(folder, subfolders).into_iter().filter_map(|path| search_file(path, spec)).collect()
}

/// The hits of one project file: `None` when it has none.
fn search_file(path: PathBuf, spec: &FindSpec) -> Option<FileHits> {
    let spec = FindSpec { within: FindScope::EntireProject, ..spec.clone() };
    match sheaf_io::open_project(&path) {
        Ok(opened) => {
            let hits = find_in_project(&opened.project, &spec, &[]);
            (!hits.is_empty()).then_some(FileHits { path, hits, error: None })
        }
        Err(e) => Some(FileHits { path, hits: Vec::new(), error: Some(e.to_string()) }),
    }
}

/// What the search thread reports (T441).
enum Progress {
    /// How many project files there are.
    Total(usize),
    /// One file searched: its hits when it has any.
    File(Option<FileHits>),
    /// The search ended (all files, or stopped).
    Done,
}

/// A search running on its own thread (T441).
pub struct Running {
    rx: std::sync::mpsc::Receiver<Progress>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub total: Option<usize>,
    pub searched: usize,
}

impl Running {
    fn start(folder: PathBuf, subfolders: bool, spec: FindSpec) -> Running {
        let (tx, rx) = std::sync::mpsc::channel();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let halt = stop.clone();
        std::thread::spawn(move || {
            let files = project_files(&folder, subfolders);
            let _ = tx.send(Progress::Total(files.len()));
            for path in files {
                if halt.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                if tx.send(Progress::File(search_file(path, &spec))).is_err() {
                    return;
                }
            }
            let _ = tx.send(Progress::Done);
        });
        Running { rx, stop, total: None, searched: 0 }
    }
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
    /// The search in progress (T441).
    pub running: Option<Running>,
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
            running: None,
        }
    }

    /// Starts a search on its own thread (T441); the results grow as [`poll`](Self::poll) takes them in.
    pub fn start_search(&mut self) {
        self.stop_search();
        self.results = Some(Vec::new());
        self.chosen = None;
        self.running = Some(Running::start(PathBuf::from(self.folder.trim()), self.subfolders, self.spec.clone()));
    }

    /// Asks a running search to stop after the file in hand.
    pub fn stop_search(&mut self) {
        if let Some(r) = &self.running {
            r.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Takes in what the search thread found so far; `true` while it still runs.
    pub fn poll(&mut self) -> bool {
        let Some(r) = &mut self.running else { return false };
        let mut done = false;
        while let Ok(p) = r.rx.try_recv() {
            match p {
                Progress::Total(n) => r.total = Some(n),
                Progress::File(found) => {
                    r.searched += 1;
                    if let Some(f) = found {
                        self.results.get_or_insert_with(Vec::new).push(f);
                    }
                }
                Progress::Done => done = true,
            }
        }
        if self.chosen.is_none() {
            self.chosen = self.results.as_ref().and_then(|r| r.iter().position(|f| f.error.is_none()));
        }
        if done {
            self.running = None;
        }
        !done
    }

    pub fn show(&mut self, ctx: &egui::Context) -> FindFilesOutcome {
        if self.poll() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
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
                    if ui.add_enabled(can && self.running.is_none(), egui::Button::new("Find All")).clicked() {
                        self.start_search();
                    }
                    if ui.add_enabled(self.running.is_some(), egui::Button::new("Stop")).clicked() {
                        self.stop_search();
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
                if let Some(r) = &self.running {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        match r.total {
                            Some(n) => ui.label(format!("Searching {} of {n} files...", (r.searched + 1).min(n))),
                            None => ui.label("Listing the project files..."),
                        };
                    });
                }
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
        if !open || matches!(outcome, FindFilesOutcome::Closed | FindFilesOutcome::OpenFile(_)) {
            self.stop_search();
        }
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

    #[test]
    fn the_text_of_a_notes_window_is_found_in_a_saved_project() {
        // T407: Find in Project Files searches notes windows too.
        let dir = std::env::temp_dir().join(format!("sheaf-find-notes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut p = sheaf_core::Project::new();
        let id = p.add_notes();
        p.notes_mut(id).unwrap().text = "calibrated against the platinum reference".into();
        let path = dir.join("notes.sheaf");
        sheaf_io::project_file::save(&p, &path).unwrap();
        let spec = FindSpec { text: "platinum".into(), ..FindSpec::default() };
        let found = search_files(&dir, false, &spec);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_background_search_finds_what_the_plain_one_finds() {
        // T441: the thread reports the total, each file, and the end; the results match `search_files`.
        let dir = std::env::temp_dir().join(format!("sheaf-find-files-bg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("deeper")).unwrap();
        save(&dir, "a.sheaf", "Voltammetry");
        save(&dir, "b.sheaf", "Impedance");
        save(&dir.join("deeper"), "c.sheaf", "Voltammetry");
        let mut d = FindFilesDialog::new(Some(&dir));
        d.spec.text = "voltammetry".into();
        d.start_search();
        let start = std::time::Instant::now();
        while d.poll() {
            assert!(start.elapsed() < std::time::Duration::from_secs(60), "the search ends");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(d.running.is_none());
        let plain = search_files(&dir, true, &d.spec);
        assert_eq!(d.results.as_ref().unwrap(), &plain);
        assert_eq!(d.chosen, Some(0));
        // Stop before it starts: it ends without going through the files.
        d.start_search();
        d.stop_search();
        while d.poll() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(d.results.as_ref().unwrap().len() <= plain.len());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
