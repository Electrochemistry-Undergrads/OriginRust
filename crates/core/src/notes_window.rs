//! Notes windows (T154): a window of free text in the project, as Origin's File > New > Notes makes.
//! Origin 2016's Notes window was not measured (its COM class was unregistered); Sheaf's window keeps
//! plain text only.

use crate::ids::WindowId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Notes {
    pub id: WindowId,
    /// The short name, `Notes1`, `Notes2`, ...
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub long_name: String,
    /// The text, lines separated by `\n`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
}

impl Notes {
    pub fn new(id: WindowId, name: impl Into<String>) -> Self {
        Notes { id, name: name.into(), long_name: String::new(), text: String::new() }
    }

    /// The window title: the short name, with the long name after it when there is one.
    pub fn title(&self) -> String {
        if self.long_name.is_empty() { self.name.clone() } else { format!("{} - {}", self.name, self.long_name) }
    }
}

#[cfg(test)]
mod tests {
    use crate::Project;

    #[test]
    fn notes_windows_are_named_renamed_duplicated_and_saved() {
        let mut p = Project::new();
        let a = p.add_notes();
        let b = p.add_notes();
        assert_eq!(
            (p.notes_window(a).unwrap().name.as_str(), p.notes_window(b).unwrap().name.as_str()),
            ("Notes1", "Notes2")
        );
        p.notes_mut(a).unwrap().text = "first line\nsecond".into();
        p.rename_window(a, "Diary").unwrap();
        assert_eq!(p.window(a).unwrap().title(), "Diary");
        let c = p.duplicate_window(a).unwrap();
        assert_eq!(p.notes_window(c).unwrap().text, "first line\nsecond");
        assert_eq!(p.notes_window(c).unwrap().name, "Notes1", "the first free name (Notes1 was renamed)");
        assert_eq!(p.workbooks().count() + p.graphs().count(), 0, "notes are neither books nor graphs");
        let back: Project = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(back.notes().count(), 3);
        assert_eq!(back.notes_window(a), p.notes_window(a));
    }
}
