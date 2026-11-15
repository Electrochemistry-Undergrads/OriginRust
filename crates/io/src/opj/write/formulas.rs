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
/// written; a 1-based position.
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
