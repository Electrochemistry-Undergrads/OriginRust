//! Which columns a column formula reads, and the text that goes into the file for it (spec
//! section 13.10).

use sheaf_core::worksheet::Worksheet;
use sheaf_expr::{ColumnRef, Expr};

/// The positions of the columns of `sheet` that `formula` reads, in order of first use. `None`
/// when the formula does not parse or reads a column the sheet does not have.
pub(super) fn inputs_of(sheet: &Worksheet, formula: &str) -> Option<Vec<usize>> {
    let expr = Expr::parse(formula).ok()?;
    let mut found = Vec::new();
    for r in expr.columns() {
        let i = find(sheet, &r)?;
        if !found.contains(&i) {
            found.push(i);
        }
    }
    Some(found)
}

/// The column a reference means: a short name ignoring case, then a long name; a long name as
/// written; a 1-based position. A bare `col(j)` is read like any name, as in the sheet host of
/// `sheaf-core`; when no column is called j it means the position of the formula column, which
/// is not known here, so it finds nothing.
fn find(sheet: &Worksheet, r: &ColumnRef) -> Option<usize> {
    let cols = &sheet.columns;
    match r {
        ColumnRef::Short(name) => cols
            .iter()
            .position(|c| c.short_name.eq_ignore_ascii_case(name))
            .or_else(|| cols.iter().position(|c| c.long_name == *name)),
        ColumnRef::Long(name) => cols.iter().position(|c| c.long_name == *name),
        ColumnRef::Index(n) => n.checked_sub(1).filter(|&i| i < cols.len()),
        // Cross-sheet recalculation records are not written; the existing warning/companion
        // path keeps these formulas losslessly without attaching them to local columns.
        ColumnRef::Scoped { .. } => None,
        // `col(j-1)` depends on the column that owns the formula, which is not known here.
        ColumnRef::Relative(_) => None,
    }
}

/// What separates a formula from the part after it in the stored text.
pub(super) const TAIL_MARK: &str = ";\r\n#";

/// The stored text of a formula: the formula, then, for a formula that fills only some rows, the
/// mark and the 1-based inclusive range of rows written as `$[first:last]`.
pub(super) fn stored_text(formula: &str, rows: Option<&std::ops::Range<usize>>) -> String {
    match rows.filter(|r| !r.is_empty()) {
        Some(r) => format!("{formula}{TAIL_MARK}$[{}:{}]", r.start + 1, r.end),
        None => formula.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::{ColumnData, Designation, Project};

    fn sheet_of(count: usize) -> Project {
        let mut p = Project::new();
        let book = p.add_workbook("");
        for _ in 0..count {
            p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap();
        }
        p
    }

    fn first_sheet(p: &Project) -> &Worksheet {
        &p.workbooks().next().unwrap().sheets[0]
    }

    #[test]
    fn a_bare_lowercase_col_j_without_a_column_called_j_reads_no_column() {
        let p = sheet_of(3);
        assert_eq!(inputs_of(first_sheet(&p), "col(A)+wcol(j)"), None);
        assert_eq!(inputs_of(first_sheet(&p), "col(A)*j"), Some(vec![0]), "the variable j is not a read");
        assert_eq!(inputs_of(first_sheet(&p), "col(j-1)"), None);
    }

    #[test]
    fn a_bare_col_j_reads_the_tenth_column_when_there_is_one_called_j() {
        let p = sheet_of(11);
        assert_eq!(first_sheet(&p).columns[9].short_name, "J");
        assert_eq!(inputs_of(first_sheet(&p), "col(A)+wcol(j)*0"), Some(vec![0, 9]));
    }
}
