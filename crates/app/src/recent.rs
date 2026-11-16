//! The recent projects and recent imports as the application holds them (T38, `docs/spec/recent-files.md`).
//!
//! The menus are drawn every frame, so the lists live in memory here. The file is read at start and
//! again each time the File menu opens (so a change by another instance shows), and it is written
//! through `sheaf_io::recent::update`, which merges with what another instance wrote meanwhile.

use sheaf_io::recent::{self, ExportEntry, RecentList};
use sheaf_io::window_file::{self, WindowFileKind};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Which list an entry belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecentKind {
    Projects,
    Imports,
    /// Workbook window files (`.sheafbook`, T54).
    Books,
    /// Graph window files (`.sheafgraph`, T54).
    Graphs,
}

impl RecentKind {
    /// The kind of window file a books or graphs list holds; `None` for the other lists.
    pub fn window_kind(self) -> Option<WindowFileKind> {
        match self {
            RecentKind::Books => Some(WindowFileKind::Book),
            RecentKind::Graphs => Some(WindowFileKind::Graph),
            RecentKind::Projects | RecentKind::Imports => None,
        }
    }
}

/// The lists and the file behind them. Without a file there are no lists: nothing is kept or shown.
#[derive(Debug, Default)]
pub struct Recent {
    file: Option<PathBuf>,
    list: RecentList,
    /// A failed write was already reported; reported again only after a write worked.
    reported: bool,
}

impl Recent {
    /// Reads the file once. A missing or damaged file gives empty lists.
    pub fn new(file: Option<PathBuf>) -> Self {
        let list = file.as_deref().map(RecentList::load).unwrap_or_default();
        Recent { file, list, reported: false }
    }

    /// Reads the file again (the File menu was opened).
    pub fn reload(&mut self) {
        if let Some(file) = &self.file {
            self.list = RecentList::load(file);
        }
    }

    pub fn projects(&self) -> &[PathBuf] {
        &self.list.projects
    }

    pub fn imports(&self) -> &[PathBuf] {
        &self.list.imports
    }

    pub fn books(&self) -> &[PathBuf] {
        &self.list.books
    }

    pub fn graphs(&self) -> &[PathBuf] {
        &self.list.graphs
    }

    /// The Most Recently Used command keys of every menu (menu name to keys, newest first).
    pub fn commands(&self) -> &BTreeMap<String, Vec<String>> {
        &self.list.commands
    }

    /// The files written by exports, newest first.
    pub fn exports(&self) -> &[ExportEntry] {
        &self.list.exports
    }

    /// A command was run from a menu: it goes to the top of that menu's Most Recently Used list.
    pub fn push_command(&mut self, menu: &str, key: &str) -> Option<String> {
        self.change(|l| l.push_command(menu, key))
    }

    /// An export wrote a file.
    pub fn push_export(&mut self, entry: &ExportEntry) -> Option<String> {
        self.change(|l| l.push_export(entry.clone()))
    }

    pub fn clear_exports(&mut self) -> Option<String> {
        self.change(RecentList::clear_exports)
    }

    /// A window file was saved or opened: it goes to the top of the books or the graphs.
    pub fn push_window(&mut self, kind: WindowFileKind, path: &Path) -> Option<String> {
        self.change(|l| l.push_window(kind, path))
    }

    /// A project was opened or saved. Returns the text to log when the file could not be written
    /// (once, until a write works again).
    pub fn push_project(&mut self, path: &Path) -> Option<String> {
        self.change(|l| l.push_project(path))
    }

    /// Data files were imported, in import order.
    pub fn push_imports(&mut self, paths: &[PathBuf]) -> Option<String> {
        self.change(|l| l.push_import(paths))
    }

    /// Drops a file from every list (its entry was clicked and the file is not usable).
    pub fn forget(&mut self, path: &Path) -> Option<String> {
        self.change(|l| l.remove(path))
    }

    pub fn clear_projects(&mut self) -> Option<String> {
        self.change(RecentList::clear_projects)
    }

    pub fn clear_imports(&mut self) -> Option<String> {
        self.change(RecentList::clear_imports)
    }

    pub fn clear_books(&mut self) -> Option<String> {
        self.change(RecentList::clear_books)
    }

    pub fn clear_graphs(&mut self) -> Option<String> {
        self.change(RecentList::clear_graphs)
    }

    /// Applies a change to the file (merged with other instances) and keeps the result. When the
    /// file cannot be written the change still shows in this run.
    fn change(&mut self, change: impl Fn(&mut RecentList)) -> Option<String> {
        let file = self.file.clone()?;
        match recent::update(&file, &change) {
            Ok(list) => {
                self.list = list;
                self.reported = false;
                None
            }
            Err(e) => {
                change(&mut self.list);
                if std::mem::replace(&mut self.reported, true) {
                    None
                } else {
                    Some(format!("The recent files list could not be written: {e}"))
                }
            }
        }
    }
}

/// Why an entry cannot be followed, or `Ok` when its file is there, is a regular file (a folder, or a
/// link to one, is not) and is the kind of file the list is for (a project, a data file, a workbook
/// file or a graph file). Asked only when the entry is clicked.
pub fn check_target(path: &Path, kind: RecentKind) -> Result<(), String> {
    match std::fs::metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err("The file is not there any more.".to_string()),
        Err(e) => return Err(format!("The file cannot be read: {e}")),
        Ok(m) if !m.is_file() => return Err("It is not a file (a folder, or a link to one).".to_string()),
        Ok(_) => {}
    }
    if let Some(wanted) = kind.window_kind() {
        // The manifest says which kind a window file is, whatever its name.
        return match window_file::window_file_kind(path) {
            Some(k) if k == wanted => Ok(()),
            Some(_) => Err(format!("It is a {} file, not a {} file.", other_name(wanted), window_name(wanted))),
            None => Err(format!("It is not a {} file.", window_name(wanted))),
        };
    }
    let window = window_file::is_window_file(path);
    match (kind, window || sheaf_io::is_project_file(path)) {
        (RecentKind::Projects, false) => Err("It is not a project file.".to_string()),
        (RecentKind::Projects, true) if window => Err("It is a window file, not a project.".to_string()),
        (RecentKind::Imports, true) if window => Err("It is a window file, not a data file.".to_string()),
        (RecentKind::Imports, true) => Err("It is a project file, not a data file.".to_string()),
        _ => Ok(()),
    }
}

fn window_name(kind: WindowFileKind) -> &'static str {
    match kind {
        WindowFileKind::Book => "workbook",
        WindowFileKind::Graph => "graph",
    }
}

fn other_name(kind: WindowFileKind) -> &'static str {
    match kind {
        WindowFileKind::Book => "graph",
        WindowFileKind::Graph => "workbook",
    }
}

/// The text of an entry as Origin 2016 shows it: the whole path without the extension, shortened
/// to `max_chars` by replacing the folders after the first one with `...` until it fits (the drive and
/// first folder stay, then as many trailing parts as fit; measured, recent-files.md 2). A file name
/// that does not fit even so is shown alone.
pub fn entry_text(path: &Path, max_chars: usize) -> String {
    let full = path.with_extension("").display().to_string();
    if full.chars().count() <= max_chars {
        return full;
    }
    let sep = if full.contains('\\') { '\\' } else { '/' };
    let parts: Vec<&str> = full.split(sep).collect();
    if parts.len() < 3 {
        return full;
    }
    let head = format!("{}{sep}{}{sep}...", parts[0], parts[1]);
    // drop the folders after the first one, from the left, until the rest fits
    for start in 2..parts.len() {
        let tail = parts[start..].join(&sep.to_string());
        let candidate = format!("{head}{sep}{tail}");
        if candidate.chars().count() <= max_chars {
            return candidate;
        }
    }
    parts[parts.len() - 1].to_string()
}

/// The label of the `n`-th entry (from 0), numbered as Origin 2016 does: `&1 name` to `&9 name`, then
/// `&A name` to `&F name`; no number after the fifteenth. `&` in the name is doubled.
pub fn numbered(n: usize, name: &str) -> String {
    let name = name.replace('&', "&&");
    match n {
        0..=8 => format!("&{} {name}", n + 1),
        9..=14 => format!("&{} {name}", char::from(b'A' + (n - 9) as u8)),
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sheaf-app-recent-unit-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn entries_are_numbered_one_to_nine_then_a_to_f_as_in_origin() {
        assert_eq!(numbered(0, "a"), "&1 a");
        assert_eq!(numbered(8, "a"), "&9 a");
        assert_eq!(numbered(9, "a"), "&A a");
        assert_eq!(numbered(14, "a"), "&F a");
        assert_eq!(numbered(15, "a"), "a");
        assert_eq!(numbered(0, "R&D"), "&1 R&&D");
    }

    #[test]
    fn long_paths_lose_folders_after_the_first_as_origin_shows_them() {
        // the two cases measured in Origin 2016's File menu (80 characters, extension left out)
        let p = Path::new(r"C:\Users\RenLab\Downloads\OriginRust\re-workspace\scratch\t21-legend-position\legend.opj");
        assert_eq!(entry_text(p, 80), r"C:\Users\...\OriginRust\re-workspace\scratch\t21-legend-position\legend");
        let b = Path::new(r"C:\Program Files\OriginLab\Origin2016\Samples\Curve Fitting\Dose Response Analysis.ogw");
        assert_eq!(entry_text(b, 80), r"C:\Program Files\...\Origin2016\Samples\Curve Fitting\Dose Response Analysis");
        // a 76-character path stays whole
        let short = Path::new(r"C:\Program Files\OriginLab\Origin2016\Samples\Curve Fitting\Single Peak Fit.ogw");
        assert_eq!(
            entry_text(short, 80),
            r"C:\Program Files\OriginLab\Origin2016\Samples\Curve Fitting\Single Peak Fit"
        );
        // a name too long to fit is shown alone
        let name = "x".repeat(90);
        assert_eq!(entry_text(Path::new(&format!(r"C:\a\b\{name}.opj")), 80), name);
    }

    #[test]
    fn an_ampersand_in_a_file_name_is_not_an_accelerator() {
        assert_eq!(numbered(1, "salt & pepper.csv"), "&2 salt && pepper.csv");
        assert_eq!(crate::menus::mnemonic(&numbered(1, "a&b")), Some('2'));
    }

    #[test]
    fn unix_paths_are_shortened_the_same_way() {
        let long =
            Path::new("/home/user/a/very/long/folder/name/that/goes/on/and/on/and/on/and/on/and/on/and/on/file.csv");
        let text = entry_text(long, 40);
        assert!(text.starts_with("/home/...") && text.ends_with("/on/file") && text.chars().count() <= 40, "{text}");
        assert_eq!(entry_text(Path::new("/a/b/c.csv"), 40), "/a/b/c");
    }

    #[test]
    fn a_missing_file_a_folder_and_the_wrong_kind_of_file_are_refused() {
        let dir = temp_dir("check");
        let project = dir.join("a.sheaf");
        let data = dir.join("d.csv");
        std::fs::write(&project, "x").unwrap();
        std::fs::write(&data, "x").unwrap();
        assert!(check_target(&project, RecentKind::Projects).is_ok());
        assert!(check_target(&data, RecentKind::Imports).is_ok());
        assert!(check_target(&dir.join("gone.sheaf"), RecentKind::Projects).unwrap_err().contains("not there"));
        assert!(check_target(&dir, RecentKind::Projects).unwrap_err().contains("not a file"));
        assert!(check_target(&data, RecentKind::Projects).unwrap_err().contains("not a project"));
        assert!(check_target(&project, RecentKind::Imports).unwrap_err().contains("project file"));
    }

    #[test]
    fn a_window_file_is_checked_by_its_kind_and_is_never_a_project_or_a_data_file() {
        let dir = temp_dir("window-kinds");
        let mut p = sheaf_core::Project::new();
        let book = p.add_workbook("B");
        let (book_file, renamed) = (dir.join("b.sheafbook"), dir.join("b-renamed.sheaf"));
        sheaf_io::window_file::save_window(&p, book, &book_file).unwrap();
        std::fs::copy(&book_file, &renamed).unwrap();
        assert!(check_target(&book_file, RecentKind::Books).is_ok());
        assert!(check_target(&renamed, RecentKind::Books).is_ok(), "the manifest decides, not the name");
        assert!(check_target(&book_file, RecentKind::Graphs).unwrap_err().contains("workbook file, not a graph"));
        assert!(check_target(&book_file, RecentKind::Projects).unwrap_err().contains("window file"));
        assert!(check_target(&renamed, RecentKind::Projects).unwrap_err().contains("window file"));
        assert!(check_target(&book_file, RecentKind::Imports).unwrap_err().contains("window file"));
        let project = dir.join("p.sheaf");
        sheaf_io::save::save_project(&p, &project).unwrap();
        assert!(check_target(&project, RecentKind::Books).unwrap_err().contains("not a workbook file"));
        assert!(check_target(&project, RecentKind::Projects).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn a_link_to_a_folder_is_refused_and_a_link_to_a_file_is_followed() {
        let dir = temp_dir("link");
        let folder = dir.join("f.sheaf");
        std::fs::create_dir(&folder).unwrap();
        let file = dir.join("real.sheaf");
        std::fs::write(&file, "x").unwrap();
        let to_folder = dir.join("to-folder.sheaf");
        let to_file = dir.join("to-file.sheaf");
        std::os::unix::fs::symlink(&folder, &to_folder).unwrap();
        std::os::unix::fs::symlink(&file, &to_file).unwrap();
        assert!(check_target(&to_folder, RecentKind::Projects).unwrap_err().contains("not a file"));
        assert!(check_target(&to_file, RecentKind::Projects).is_ok());
    }

    #[test]
    fn without_a_file_nothing_is_kept() {
        let mut r = Recent::new(None);
        assert_eq!(r.push_project(Path::new("/tmp/x.sheaf")), None);
        assert!(r.projects().is_empty());
    }

    #[test]
    fn a_change_is_merged_with_what_another_instance_wrote() {
        let dir = temp_dir("merge");
        let file = dir.join("recent.json");
        let (a, b) = (dir.join("a.sheaf"), dir.join("b.sheaf"));
        std::fs::write(&a, "x").unwrap();
        std::fs::write(&b, "x").unwrap();
        let mut one = Recent::new(Some(file.clone()));
        let mut two = Recent::new(Some(file));
        assert_eq!(one.push_project(&a), None);
        assert_eq!(two.push_project(&b), None);
        assert_eq!(two.projects().len(), 2, "the second instance keeps the first one's entry");
        one.reload();
        assert_eq!(one.projects(), two.projects());
    }

    #[test]
    fn a_write_that_fails_is_reported_once_and_still_shows_in_this_run() {
        let dir = temp_dir("fail");
        // The "file" is a folder, so it can never be written.
        let file = dir.join("recent.json");
        std::fs::create_dir(&file).unwrap();
        let mut r = Recent::new(Some(file));
        let a = dir.join("a.sheaf");
        assert!(r.push_project(&a).is_some_and(|t| t.contains("could not be written")));
        assert_eq!(r.push_project(&dir.join("b.sheaf")), None, "not reported twice");
        assert_eq!(r.projects().len(), 2);
    }
}
