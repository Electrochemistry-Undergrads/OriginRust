//! The recent projects and recent imports as the application holds them (T38, `docs/spec/recent-files.md`).
//!
//! The menus are drawn every frame, so the lists live in memory here. The file is read at start and
//! again each time the File menu opens (so a change by another instance shows), and it is written
//! through `sheaf_io::recent::update`, which merges with what another instance wrote meanwhile.

use sheaf_io::recent::{self, RecentList};
use std::path::{Path, PathBuf};

/// Which list an entry belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecentKind {
    Projects,
    Imports,
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

    /// A project was opened or saved. Returns the text to log when the file could not be written
    /// (once, until a write works again).
    pub fn push_project(&mut self, path: &Path) -> Option<String> {
        self.change(|l| l.push_project(path))
    }

    /// Data files were imported, in import order.
    pub fn push_imports(&mut self, paths: &[PathBuf]) -> Option<String> {
        self.change(|l| l.push_import(paths))
    }

    /// Drops a file from both lists (its entry was clicked and the file is not usable).
    pub fn forget(&mut self, path: &Path) -> Option<String> {
        self.change(|l| l.remove(path))
    }

    pub fn clear_projects(&mut self) -> Option<String> {
        self.change(RecentList::clear_projects)
    }

    pub fn clear_imports(&mut self) -> Option<String> {
        self.change(RecentList::clear_imports)
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
/// link to one, is not) and is the kind of file the list is for. Asked only when the entry is clicked.
pub fn check_target(path: &Path, kind: RecentKind) -> Result<(), String> {
    match std::fs::metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err("The file is not there any more.".to_string()),
        Err(e) => return Err(format!("The file cannot be read: {e}")),
        Ok(m) if !m.is_file() => return Err("It is not a file (a folder, or a link to one).".to_string()),
        Ok(_) => {}
    }
    match (kind, sheaf_io::is_project_file(path)) {
        (RecentKind::Projects, false) => Err("It is not a project file.".to_string()),
        (RecentKind::Imports, true) => Err("It is a project file, not a data file.".to_string()),
        _ => Ok(()),
    }
}

/// The file name and the folder of an entry, as the menu shows them. The folder is shortened from
/// the left (`...`) when longer than `max_chars`; the whole path is the tooltip.
pub fn entry_parts(path: &Path, max_chars: usize) -> (String, String) {
    let name = path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned());
    let folder = path.parent().map(|p| p.display().to_string()).unwrap_or_default();
    (name, shorten_start(&folder, max_chars))
}

/// The text cut from the left to `max_chars` characters, with `...` in front when something was cut.
fn shorten_start(text: &str, max_chars: usize) -> String {
    let n = text.chars().count();
    if n <= max_chars {
        return text.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    let tail: String = text.chars().skip(n - keep).collect();
    format!("...{tail}")
}

/// The label of the `n`-th entry (from 0): `&1 name` to `&9 name`, then `1&0 name` for the tenth,
/// and no accelerator after that. `&` in the name is doubled.
pub fn numbered(n: usize, name: &str) -> String {
    let name = name.replace('&', "&&");
    match n {
        0..=8 => format!("&{} {name}", n + 1),
        9 => format!("1&0 {name}"),
        _ => format!("{} {name}", n + 1),
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
    fn the_tenth_entry_has_its_accelerator_on_the_zero() {
        assert_eq!(numbered(0, "a.sheaf"), "&1 a.sheaf");
        assert_eq!(numbered(8, "a.sheaf"), "&9 a.sheaf");
        assert_eq!(numbered(9, "a.sheaf"), "1&0 a.sheaf");
        assert_eq!(numbered(10, "a.sheaf"), "11 a.sheaf");
    }

    #[test]
    fn an_ampersand_in_a_file_name_is_not_an_accelerator() {
        assert_eq!(numbered(1, "salt & pepper.csv"), "&2 salt && pepper.csv");
        assert_eq!(crate::menus::mnemonic(&numbered(1, "a&b")), Some('2'));
    }

    #[test]
    fn a_long_folder_is_cut_from_the_left_and_a_short_one_is_kept() {
        let long = Path::new("/a/very/long/folder/name/that/goes/on/and/on/file.csv");
        let (name, folder) = entry_parts(long, 20);
        assert_eq!(name, "file.csv");
        assert_eq!(folder.chars().count(), 20);
        assert!(folder.starts_with("...") && folder.ends_with("on/and/on"), "{folder}");
        assert_eq!(entry_parts(Path::new("/a/b/c.csv"), 20), ("c.csv".to_string(), "/a/b".to_string()));
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
