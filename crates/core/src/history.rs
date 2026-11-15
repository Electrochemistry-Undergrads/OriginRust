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
    /// Counts every change of the project, including undo and redo. Views
    /// compare it with the value they last drew to know when to redraw.
    revision: u64,
    /// Identity of the undoable content, independent of redraw revisions and view adjustments.
    state: u64,
    next_state: u64,
    saved_state: Option<u64>,
    /// Counts the times [`Document::replace`] ran, so a holder can tell that the whole project was
    /// swapped for another one.
    generation: u64,
}

#[derive(Debug)]
struct Step {
    label: String,
    project: Project,
    state: u64,
    /// Steps with the same non-zero token are merged into one (see
    /// [`Document::apply_merging`]).
    merge_token: u64,
}

impl Default for Document {
    fn default() -> Self {
        Document::new(Project::new())
    }
}

impl Document {
    pub fn new(mut project: Project) -> Self {
        project.apply_mask_switch();
        Document {
            project,
            undo: Vec::new(),
            redo: Vec::new(),
            dirty: false,
            limit: 100,
            revision: 0,
            state: 0,
            next_state: 1,
            saved_state: Some(0),
            generation: 0,
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    /// A number that changes whenever the project changes.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// A number that grows each time the project is replaced as a whole (see [`Document::replace`]).
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Keeps at most `limit` undo steps from now on (0 keeps none) and drops the oldest ones that
    /// are over it. The redo steps are not limited by it.
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
        let extra = self.undo.len().saturating_sub(limit);
        self.undo.drain(..extra);
    }

    /// Records that the current content was successfully saved. A subsequent live-dialog change
    /// must start a new step, so undo can return exactly to this checkpoint.
    pub fn mark_saved(&mut self) {
        self.saved_state = Some(self.state);
        self.dirty = false;
        if let Some(step) = self.undo.last_mut() {
            step.merge_token = 0;
        }
    }

    /// Marks content with no saved checkpoint (for example a recovered project).
    pub fn mark_unsaved(&mut self) {
        self.saved_state = None;
        self.dirty = true;
    }

    /// Changes the project as one undoable step named `label`.
    /// If `change` returns an error the project is left as it was.
    pub fn edit<T, E>(&mut self, label: &str, change: impl FnOnce(&mut Project) -> Result<T, E>) -> Result<T, E> {
        self.edit_with_token(label, 0, change)
    }

    /// Like [`Document::apply`], but consecutive changes made with the same
    /// non-zero `token` become a single undo step. A dialog that previews
    /// every change live uses one token for as long as it is open, so one
    /// Undo reverts everything the dialog did.
    pub fn apply_merging<T>(&mut self, label: &str, token: u64, change: impl FnOnce(&mut Project) -> T) -> T {
        let result: Result<T, std::convert::Infallible> = self.edit_with_token(label, token, |p| Ok(change(p)));
        match result {
            Ok(v) => v,
        }
    }

    fn edit_with_token<T, E>(
        &mut self,
        label: &str,
        token: u64,
        change: impl FnOnce(&mut Project) -> Result<T, E>,
    ) -> Result<T, E> {
        let before = self.project.clone();
        // A panic in the change or in the recalculation that follows puts the project back and goes on
        // unwinding: no half-made change stays, and no undo step or dirty flag is left out of step with it.
        let done = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let result = change(&mut self.project);
            if result.is_ok() {
                // Recalculating columns follow their inputs inside the same undo step, so Undo
                // restores the source and everything computed from it together.
                self.project.recalculate_since(&before);
                self.project.update_analysis_links(&before);
                self.project.apply_mask_switch();
            }
            result
        }));
        let result = match done {
            Ok(result) => result,
            Err(panic) => {
                self.project = before;
                std::panic::resume_unwind(panic);
            }
        };
        match result {
            Ok(value) => {
                if self.project != before {
                    let merge = token != 0 && self.undo.last().is_some_and(|s| s.merge_token == token);
                    if !merge {
                        self.undo.push(Step {
                            label: label.to_string(),
                            project: before,
                            state: self.state,
                            merge_token: token,
                        });
                        if self.undo.len() > self.limit {
                            self.undo.remove(0);
                        }
                    }
                    self.redo.clear();
                    self.state = self.next_state;
                    self.next_state = self.next_state.checked_add(1).expect("document state counter exhausted");
                    self.dirty = self.saved_state != Some(self.state);
                    self.revision += 1;
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

    /// Changes view state that lives in the project (for example which sheet
    /// of a workbook is shown) without adding an undo step and without
    /// marking the project as changed.
    pub fn adjust_view(&mut self, change: impl FnOnce(&mut Project)) {
        change(&mut self.project);
        self.revision += 1;
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
        let current_state = std::mem::replace(&mut self.state, step.state);
        self.redo.push(Step { label: step.label.clone(), project: current, state: current_state, merge_token: 0 });
        self.dirty = self.saved_state != Some(self.state);
        self.revision += 1;
        Some(step.label)
    }

    pub fn redo(&mut self) -> Option<String> {
        let step = self.redo.pop()?;
        let current = std::mem::replace(&mut self.project, step.project);
        let current_state = std::mem::replace(&mut self.state, step.state);
        self.undo.push(Step { label: step.label.clone(), project: current, state: current_state, merge_token: 0 });
        self.dirty = self.saved_state != Some(self.state);
        self.revision += 1;
        Some(step.label)
    }

    /// Replaces the project (for example after opening a file) and clears the history.
    pub fn replace(&mut self, mut project: Project) {
        project.apply_mask_switch();
        self.project = project;
        self.undo.clear();
        self.redo.clear();
        self.state = self.next_state;
        self.next_state = self.next_state.checked_add(1).expect("document state counter exhausted");
        self.saved_state = Some(self.state);
        self.dirty = false;
        self.revision += 1;
        self.generation += 1;
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
    fn merged_edits_undo_as_one_step() {
        let mut doc = Document::new(Project::new());
        doc.apply("Base", |p| p.add_workbook(""));
        let r0 = doc.revision();
        doc.apply_merging("Dialog", 7, |p| p.add_workbook("a"));
        doc.apply_merging("Dialog", 7, |p| p.add_workbook("b"));
        doc.apply_merging("Other dialog", 8, |p| p.add_workbook("c"));
        assert_eq!(doc.project().windows.len(), 4);
        assert_eq!(doc.revision(), r0 + 3);
        doc.undo();
        assert_eq!(doc.project().windows.len(), 3);
        doc.undo();
        assert_eq!(doc.project().windows.len(), 1, "both token-7 edits are reverted together");
    }

    #[test]
    fn a_new_edit_clears_redo() {
        let mut doc = Document::new(Project::new());
        doc.apply("One", |p| p.add_workbook(""));
        doc.undo();
        doc.apply("Two", |p| p.add_workbook(""));
        assert!(doc.redo_label().is_none());
    }

    #[test]
    fn undo_and_redo_follow_saved_checkpoint_across_branches() {
        let mut doc = Document::default();
        doc.apply("A", |p| p.add_workbook("a"));
        doc.mark_saved();
        doc.apply("B", |p| p.add_workbook("b"));
        assert!(doc.dirty);
        doc.undo();
        assert!(!doc.dirty, "undo returns to saved content");
        doc.redo();
        assert!(doc.dirty);
        doc.mark_saved();
        doc.undo();
        assert!(doc.dirty);
        doc.redo();
        assert!(!doc.dirty, "redo returns to saved content");
        doc.undo();
        doc.mark_saved();
        doc.apply("C", |p| p.add_workbook("c"));
        assert!(doc.dirty, "a new branch never reuses the abandoned state id");
        assert!(doc.redo().is_none());
        doc.undo();
        assert!(!doc.dirty, "saved checkpoint after undo is reachable");
    }

    #[test]
    fn saving_during_a_merged_dialog_is_an_undo_boundary() {
        let mut doc = Document::default();
        doc.apply_merging("Dialog", 5, |p| p.add_workbook("first"));
        doc.mark_saved();
        doc.apply_merging("Dialog", 5, |p| p.add_workbook("second"));
        doc.apply_merging("Dialog", 5, |p| p.add_workbook("third"));
        assert!(doc.dirty);
        doc.undo();
        assert!(!doc.dirty);
        assert_eq!(doc.project().windows.len(), 1);
        doc.undo();
        assert!(doc.dirty);
        doc.redo();
        assert!(!doc.dirty);
        doc.redo();
        assert!(doc.dirty);
        assert_eq!(doc.project().windows.len(), 3);
    }

    #[test]
    fn truncated_history_and_recovered_content_never_become_clean_by_accident() {
        let mut doc = Document { limit: 2, ..Document::default() };
        doc.mark_saved();
        for name in ["a", "b", "c"] {
            doc.apply("Add", |p| p.add_workbook(name));
        }
        doc.undo();
        doc.undo();
        assert!(doc.undo().is_none());
        assert!(doc.dirty, "saved state was evicted");
        doc.replace(Project::new());
        assert!(!doc.dirty);
        doc.mark_unsaved();
        assert!(doc.dirty);
        doc.apply("Edit recovery", |p| p.add_workbook("a"));
        doc.undo();
        assert!(doc.dirty, "recoveries have no saved checkpoint");
        doc.mark_saved();
        assert!(!doc.dirty);
        doc.redo();
        assert!(doc.dirty);
        doc.undo();
        assert!(!doc.dirty);
    }

    #[test]
    fn view_changes_noops_and_failed_edits_do_not_move_the_saved_checkpoint() {
        let mut doc = Document::default();
        doc.apply("A", |p| p.add_workbook("a"));
        doc.mark_saved();
        let revision = doc.revision();
        doc.adjust_view(|p| p.window_places.clear());
        assert!(!doc.dirty);
        assert!(doc.revision() > revision);
        let revision = doc.revision();
        doc.apply("Noop", |_| ());
        let result: Result<(), ()> = doc.edit("Fail", |p| {
            p.add_workbook("failed");
            Err(())
        });
        assert!(result.is_err());
        assert_eq!(doc.revision(), revision);
        assert!(!doc.dirty);
        doc.apply("B", |p| p.add_workbook("b"));
        doc.undo();
        assert!(!doc.dirty);
        doc.replace(Project::new());
        assert!(!doc.dirty);
        assert!(doc.undo().is_none());
    }

    #[test]
    fn a_limit_of_zero_keeps_no_steps_and_replace_counts_generations() {
        let mut doc = Document::new(Project::new());
        doc.set_limit(0);
        doc.apply("One", |p| p.add_workbook(""));
        assert_eq!(doc.project().windows.len(), 1);
        assert!(doc.undo_label().is_none());
        assert_eq!(doc.generation(), 0);
        doc.replace(Project::new());
        doc.replace(Project::new());
        assert_eq!(doc.generation(), 2);
        doc.set_limit(2);
        for i in 0..4 {
            doc.apply(&format!("Step {i}"), |p| p.add_workbook(""));
        }
        doc.set_limit(1);
        assert_eq!(doc.undo_label(), Some("Step 3"));
        assert!(doc.undo().is_some());
        assert!(doc.undo().is_none());
    }

    #[test]
    fn a_panic_in_an_edit_puts_the_project_back_and_leaves_no_step() {
        let mut doc = Document::new(Project::new());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            doc.apply("Boom", |p| {
                p.add_workbook("");
                panic!("boom");
            })
        }));
        assert!(result.is_err());
        assert!(doc.project().windows.is_empty(), "the half-made change is gone");
        assert!(doc.undo_label().is_none());
        assert!(!doc.dirty);
        assert_eq!(doc.revision(), 0);
    }
}
