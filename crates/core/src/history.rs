//! Undo and redo.
//!
//! The history keeps whole copies of the project. That is cheap because
//! column data is shared between copies (see [`crate::column::ColumnData`]):
//! a copy duplicates only names and styles, and a column's numbers are
//! duplicated only when that column is edited.

use crate::project::Project;

/// A project together with its undo history and unsaved-changes flag.
#[derive(Debug)]
pub struct Document {
    project: Project,
    undo: Vec<Step>,
    redo: Vec<Step>,
    /// True when the project differs from what was last saved or loaded.
    pub dirty: bool,
    limit: usize,
}

#[derive(Debug)]
struct Step {
    label: String,
    project: Project,
}

impl Default for Document {
    fn default() -> Self {
        Document::new(Project::new())
    }
}

impl Document {
    pub fn new(project: Project) -> Self {
        Document { project, undo: Vec::new(), redo: Vec::new(), dirty: false, limit: 100 }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    /// Changes the project as one undoable step named `label`.
    /// If `change` returns an error the project is left as it was.
    pub fn edit<T, E>(&mut self, label: &str, change: impl FnOnce(&mut Project) -> Result<T, E>) -> Result<T, E> {
        let before = self.project.clone();
        match change(&mut self.project) {
            Ok(value) => {
                if self.project != before {
                    self.undo.push(Step { label: label.to_string(), project: before });
                    if self.undo.len() > self.limit {
                        self.undo.remove(0);
                    }
                    self.redo.clear();
                    self.dirty = true;
                }
                Ok(value)
            }
            Err(e) => {
                self.project = before;
                Err(e)
            }
        }
    }

    /// Like [`Document::edit`] for changes that cannot fail.
    pub fn apply<T>(&mut self, label: &str, change: impl FnOnce(&mut Project) -> T) -> T {
        let result: Result<T, std::convert::Infallible> = self.edit(label, |p| Ok(change(p)));
        match result {
            Ok(v) => v,
        }
    }

    /// The label of the step that [`Document::undo`] would revert.
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|s| s.label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|s| s.label.as_str())
    }

    /// Reverts the last step. Returns its label, or `None` if there was nothing to undo.
    pub fn undo(&mut self) -> Option<String> {
        let step = self.undo.pop()?;
        let current = std::mem::replace(&mut self.project, step.project);
        self.redo.push(Step { label: step.label.clone(), project: current });
        self.dirty = true;
        Some(step.label)
    }

    pub fn redo(&mut self) -> Option<String> {
        let step = self.redo.pop()?;
        let current = std::mem::replace(&mut self.project, step.project);
        self.undo.push(Step { label: step.label.clone(), project: current });
        self.dirty = true;
        Some(step.label)
    }

    /// Replaces the project (for example after opening a file) and clears the history.
    pub fn replace(&mut self, project: Project) {
        self.project = project;
        self.undo.clear();
        self.redo.clear();
        self.dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};

    #[test]
    fn undo_and_redo_restore_data_and_structure() {
        let mut doc = Document::new(Project::new());
        let book = doc.apply("New workbook", |p| p.add_workbook(""));
        let col = doc
            .edit("Add column", |p| p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])))
            .unwrap();
        doc.apply("Edit cell", |p| p.column_mut(col).unwrap().numeric_mut().unwrap()[0] = 5.0);
        assert_eq!(doc.project().numeric(col).unwrap(), &[5.0, 2.0]);
        assert!(doc.dirty);

        assert_eq!(doc.undo().as_deref(), Some("Edit cell"));
        assert_eq!(doc.project().numeric(col).unwrap(), &[1.0, 2.0]);
        assert_eq!(doc.undo().as_deref(), Some("Add column"));
        assert!(doc.project().column(col).is_none());
        assert_eq!(doc.redo().as_deref(), Some("Add column"));
        assert_eq!(doc.redo().as_deref(), Some("Edit cell"));
        assert_eq!(doc.project().numeric(col).unwrap(), &[5.0, 2.0]);
        assert!(doc.redo().is_none());
    }

    #[test]
    fn failed_and_empty_edits_leave_no_step() {
        let mut doc = Document::new(Project::new());
        let r: Result<(), &str> = doc.edit("Broken", |p| {
            p.add_workbook("");
            Err("nope")
        });
        assert!(r.is_err());
        assert_eq!(doc.project().windows.len(), 0);
        doc.apply("Nothing", |_| ());
        assert!(doc.undo_label().is_none());
        assert!(!doc.dirty);
    }

    #[test]
    fn a_new_edit_clears_redo() {
        let mut doc = Document::new(Project::new());
        doc.apply("One", |p| p.add_workbook(""));
        doc.undo();
        doc.apply("Two", |p| p.add_workbook(""));
        assert!(doc.redo_label().is_none());
    }
}
