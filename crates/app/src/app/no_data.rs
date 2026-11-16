//! File > New > Graph, Save Project without Data... and Duplicate Project without Data (T77).

use std::path::PathBuf;

use sheaf_core::template::Template;
use sheaf_io::project_file;

use super::SheafApp;

impl SheafApp {
    /// File > New > Graph: an empty graph from the default template, one undo step, shown on top.
    pub(super) fn new_graph(&mut self) {
        let mut new = None;
        self.doc.apply("New graph", |p| new = Some(p.add_graph(Template::default())));
        if let Some(id) = new {
            self.focus(id);
            self.say("New graph".to_string());
        }
    }

    /// File > Save Project without Data...: asks for a file, then writes a copy of the project
    /// with its values taken out. The open project, its file name and its saved state are
    /// not touched.
    pub(super) fn save_project_without_data(&mut self) {
        let named = self.path.as_ref().or(self.recovered_from.as_ref());
        let suggested = named
            .and_then(|p| p.file_stem())
            .map(|n| format!("{}.opj", n.to_string_lossy()))
            .unwrap_or_else(|| "Untitled.opj".to_string());
        let mut chooser = rfd::FileDialog::new()
            .set_title("Save project without data")
            .add_filter("Origin project (*.opj)", &["opj"])
            .add_filter("Sheaf project (*.sheaf)", &[project_file::EXTENSION])
            .set_file_name(suggested);
        if let Some(dir) = self.folder() {
            chooser = chooser.set_directory(dir);
        }
        if let Some(path) = chooser.save_file() {
            self.save_project_without_data_to(path);
        }
    }

    /// Writes the data-free copy to `path` (`.opj` is added to a name that is neither `.opj`
    /// nor `.sheaf`). Returns `true` when a file was written.
    pub(super) fn save_project_without_data_to(&mut self, mut path: PathBuf) -> bool {
        let known = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("opj") || e.eq_ignore_ascii_case(project_file::EXTENSION));
        if !known {
            let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
            name.push(".opj");
            path.set_file_name(name);
        }
        // Never write over the file the open project came from in another program's format.
        if self.foreign_source.as_ref().is_some_and(|src| super::same_file(src, &path)) {
            self.message = Some((
                "Could not save the project".to_string(),
                "That is the Origin file this project was opened from.".to_string(),
            ));
            return false;
        }
        let copy = self.doc.project().without_data();
        match sheaf_io::save::save_project(&copy, &path) {
            Ok(outcome) => {
                self.say(format!("Saved without data: {}", outcome.status()));
                if let Some(text) = outcome.details() {
                    let title = if outcome.refusal.is_some() {
                        "The Origin project could not be written"
                    } else {
                        "Saved with a companion file"
                    };
                    self.message = Some((title.to_string(), text));
                }
                true
            }
            Err(e) => {
                self.message = Some(("Could not save the project".to_string(), e.to_string()));
                false
            }
        }
    }

    /// File > Duplicate Project without Data: the open project is replaced by an unsaved copy
    /// without values (the file of the project that was open is not touched).
    pub(super) fn duplicate_project_without_data(&mut self) {
        let places = self.workspace.places();
        if self.doc.project().window_places != places {
            self.doc.adjust_view(|p| p.window_places = places);
        }
        let copy = self.doc.project().without_data();
        self.doc.replace(copy);
        self.doc.mark_unsaved();
        self.path = None;
        self.recovered_from = None;
        self.foreign_source = None;
        self.reset_views();
        self.say("Duplicate of the project without data".to_string());
    }
}
