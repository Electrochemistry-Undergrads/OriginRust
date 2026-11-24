//! Add a matching column to many sheets at once (T359; Origin 2026b's "insert columns across sheets", not in
//! Origin 2016, so the rules are Sheaf's own; `docs/spec/workbook-model.md` 13q).
//!
//! The same column (long name, units, comments, designation and, if given, a Set Column Values formula with its
//! Recalculate mode) is added at the end of every sheet of one workbook or of every workbook of the project. The
//! formula runs in each sheet against that sheet's own columns, so `col(B)*2` reads each sheet's B.
//!
//! T412: the sheets can also be those of the workbooks of a project folder (with its subfolders), and the column can
//! go right after a named column of each sheet instead of at the end.

use crate::column::{ColumnData, Designation, RecalcMode};
use crate::formula::FormulaError;
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError};

/// Which sheets get the column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcrossScope {
    /// Every sheet of this workbook.
    Workbook(WindowId),
    /// Every sheet of every workbook.
    Project,
    /// Every sheet of every workbook in this project folder (the path of folder indices from the root, as in
    /// [`Project::folder_of`]) and its subfolders (T412). The root folder holds every workbook.
    Folder(Vec<usize>),
}

/// The column to add.
#[derive(Debug, Clone, PartialEq)]
pub struct AcrossColumn {
    pub long_name: String,
    pub units: String,
    pub comments: String,
    pub designation: Designation,
    /// A Set Column Values formula run in each sheet; `None` leaves the column empty.
    pub formula: Option<String>,
    pub recalc: RecalcMode,
    /// Put the column right after the column of this short name (letter case ignored), else long name, in each
    /// sheet (T412); a sheet without one, or `None`, gets it at the end.
    pub after: Option<String>,
}

impl Default for AcrossColumn {
    fn default() -> Self {
        AcrossColumn {
            long_name: String::new(),
            units: String::new(),
            comments: String::new(),
            designation: Designation::Y,
            formula: None,
            recalc: RecalcMode::None,
            after: None,
        }
    }
}

/// Why the columns could not be added.
#[derive(Debug, thiserror::Error)]
pub enum AcrossError {
    #[error("there are no sheets to add the column to")]
    NoSheets,
    #[error(transparent)]
    Project(#[from] ProjectError),
    #[error("sheet {sheet} of {book}: {error}")]
    Formula { book: String, sheet: String, error: FormulaError },
}

impl Project {
    /// Adds the column `spec` at the end of every sheet in `scope` and returns the new columns, in book and sheet
    /// order. A formula that fails in any sheet makes the whole call fail (inside [`crate::Document::edit`] nothing
    /// then changes).
    pub fn add_column_across_sheets(
        &mut self,
        scope: AcrossScope,
        spec: &AcrossColumn,
    ) -> Result<Vec<ColumnId>, AcrossError> {
        let in_folder: Option<Vec<WindowId>> = match &scope {
            AcrossScope::Folder(path) if !path.is_empty() => Some(self.folder_windows(path)),
            _ => None,
        };
        let targets: Vec<(WindowId, usize)> = self
            .workbooks()
            .filter(|b| match &scope {
                AcrossScope::Workbook(id) => b.id == *id,
                AcrossScope::Project => true,
                AcrossScope::Folder(_) => in_folder.as_ref().is_none_or(|w| w.contains(&b.id)),
            })
            .flat_map(|b| (0..b.sheets.len()).map(move |s| (b.id, s)))
            .collect();
        if targets.is_empty() {
            return Err(AcrossError::NoSheets);
        }
        let mut made = Vec::with_capacity(targets.len());
        for (book, sheet) in targets {
            let after = spec.after.as_deref().map(str::trim).filter(|a| !a.is_empty()).and_then(|name| {
                let cols = &self.workbook(book)?.sheets.get(sheet)?.columns;
                cols.iter()
                    .position(|c| c.short_name.eq_ignore_ascii_case(name))
                    .or_else(|| cols.iter().position(|c| c.long_name == name))
            });
            let id = match after {
                Some(k) => {
                    let id = self.insert_columns(book, sheet, k + 1, 1)?[0];
                    if let Some(c) = self.column_mut(id) {
                        c.designation = spec.designation;
                        c.data = ColumnData::numeric(Vec::new());
                    }
                    id
                }
                None => self.add_column(book, sheet, spec.designation, ColumnData::numeric(Vec::new()))?,
            };
            if let Some(c) = self.column_mut(id) {
                c.long_name = spec.long_name.clone();
                c.units = spec.units.clone();
                c.comments = spec.comments.clone();
            }
            if let Some(formula) = spec.formula.as_deref().filter(|f| !f.trim().is_empty()) {
                self.set_column_values_with(id, formula, None, spec.recalc).map_err(|error| {
                    let b = self.workbook(book);
                    AcrossError::Formula {
                        book: b.map(|b| b.name.clone()).unwrap_or_default(),
                        sheet: b.and_then(|b| b.sheets.get(sheet)).map(|s| s.name.clone()).unwrap_or_default(),
                        error,
                    }
                })?;
            }
            made.push(id);
        }
        Ok(made)
    }

    /// The path (folder indices from the root) of the project folder that holds window `id`: the first subfolder
    /// listing it, else the root (`[]`).
    pub fn folder_of(&self, id: WindowId) -> Vec<usize> {
        fn find(f: &crate::project::Folder, id: WindowId, path: &mut Vec<usize>) -> bool {
            for (i, sub) in f.folders.iter().enumerate() {
                path.push(i);
                if sub.windows.contains(&id) || find(sub, id, path) {
                    return true;
                }
                path.pop();
            }
            false
        }
        let mut path = Vec::new();
        if find(&self.root, id, &mut path) { path } else { Vec::new() }
    }

    /// The windows of the folder at `path` and of its subfolders.
    fn folder_windows(&self, path: &[usize]) -> Vec<WindowId> {
        fn all(f: &crate::project::Folder, out: &mut Vec<WindowId>) {
            out.extend(&f.windows);
            for sub in &f.folders {
                all(sub, out);
            }
        }
        let mut f = &self.root;
        for &i in path {
            match f.folders.get(i) {
                Some(sub) => f = sub,
                None => return Vec::new(),
            }
        }
        let mut out = Vec::new();
        all(f, &mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Book A with two sheets (B = 1, 2, 3 and B = 10, 20), book B with one sheet (B = 5).
    fn project() -> (Project, WindowId, WindowId) {
        let mut p = Project::new();
        let a = p.add_workbook("A");
        p.add_column(a, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        p.add_column(a, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        p.workbook_mut(a).unwrap().sheets.push(crate::worksheet::Worksheet::new("Sheet2"));
        p.add_column(a, 1, Designation::X, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
        p.add_column(a, 1, Designation::Y, ColumnData::numeric(vec![10.0, 20.0])).unwrap();
        let b = p.add_workbook("B");
        p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0])).unwrap();
        p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![5.0])).unwrap();
        (p, a, b)
    }

    #[test]
    fn each_sheet_of_the_workbook_gets_the_column_computed_from_its_own_data() {
        let (mut p, a, _) = project();
        let spec = AcrossColumn {
            long_name: "Double".into(),
            units: "mA".into(),
            formula: Some("col(B)*2".into()),
            ..Default::default()
        };
        let made = p.add_column_across_sheets(AcrossScope::Workbook(a), &spec).unwrap();
        assert_eq!(made.len(), 2);
        assert_eq!(p.numeric(made[0]).unwrap(), [2.0, 4.0, 6.0]);
        assert_eq!(p.numeric(made[1]).unwrap(), [20.0, 40.0]);
        let c = p.column(made[1]).unwrap();
        assert_eq!((c.long_name.as_str(), c.units.as_str()), ("Double", "mA"));
        assert_eq!(p.workbook(a).unwrap().sheets[1].columns.last().unwrap().id, made[1], "at the end of the sheet");
    }

    #[test]
    fn the_project_scope_reaches_every_book_and_a_bad_formula_fails() {
        let (mut p, _, b) = project();
        let made = p.add_column_across_sheets(AcrossScope::Project, &AcrossColumn::default()).unwrap();
        assert_eq!(made.len(), 3);
        assert!(p.workbook(b).unwrap().sheets[0].columns.iter().any(|c| c.id == made[2]));
        let bad = AcrossColumn { formula: Some("col(Q)+1".into()), ..Default::default() };
        assert!(matches!(p.add_column_across_sheets(AcrossScope::Project, &bad), Err(AcrossError::Formula { .. })));
    }

    #[test]
    fn a_folder_scope_reaches_its_books_and_the_column_can_follow_a_named_one() {
        let (mut p, a, b) = project();
        // Book B in folder "Runs" (with a subfolder holding nothing), book A in the root.
        p.root.folders.push(crate::project::Folder { name: "Runs".into(), windows: vec![b], ..Default::default() });
        p.root.folders[0].folders.push(crate::project::Folder { name: "Old".into(), ..Default::default() });
        assert_eq!((p.folder_of(b), p.folder_of(a)), (vec![0], vec![]));
        let spec = AcrossColumn { long_name: "Next".into(), after: Some("a".into()), ..Default::default() };
        let made = p.add_column_across_sheets(AcrossScope::Folder(vec![0]), &spec).unwrap();
        assert_eq!(made.len(), 1, "only book B");
        let cols = &p.workbook(b).unwrap().sheets[0].columns;
        assert_eq!(cols[1].id, made[0], "right after A");
        assert_eq!(cols.len(), 3);
        // The root folder holds every book; a sheet without the named column gets it at the end.
        let spec =
            AcrossColumn { after: Some("Nowhere".into()), formula: Some("col(B)+1".into()), ..Default::default() };
        let made = p.add_column_across_sheets(AcrossScope::Folder(vec![]), &spec).unwrap();
        assert_eq!(made.len(), 3);
        assert_eq!(p.workbook(a).unwrap().sheets[0].columns.last().unwrap().id, made[0]);
        assert_eq!(p.numeric(made[0]).unwrap(), [2.0, 3.0, 4.0]);
        assert!(p.add_column_across_sheets(AcrossScope::Folder(vec![0, 0]), &spec).is_err(), "an empty folder");
    }
}
