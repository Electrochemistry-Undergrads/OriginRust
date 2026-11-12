//! Column formulas that recalculate (Set Column Values: Recalculate None, Auto, Manual).

use sheaf_core::formula::FormulaError;
use sheaf_core::{ColumnData, ColumnId, Designation, Document, FormulaState, Project, RecalcMode, WindowId};

/// A workbook with a source column A = 1, 2, 3, 4 and four empty columns B to E.
fn book() -> (Document, WindowId, Vec<ColumnId>) {
    let mut p = Project::new();
    let book = p.add_workbook("");
    let mut ids = vec![p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0])).unwrap()];
    for _ in 0..4 {
        ids.push(p.add_column(book, 0, Designation::Y, ColumnData::numeric(Vec::new())).unwrap());
    }
    (Document::new(p), book, ids)
}

fn values(d: &Document, c: ColumnId) -> Vec<f64> {
    d.project().numeric(c).unwrap().to_vec()
}

fn edit_cell(d: &mut Document, c: ColumnId, row: usize, text: &str) {
    d.apply("Edit cell", |p| {
        p.column_mut(c).unwrap().set_cell_text(row, text);
    });
}

fn set(d: &mut Document, c: ColumnId, formula: &str, mode: RecalcMode) -> Result<(), FormulaError> {
    d.edit("Set column values", |p| p.set_column_values_with(c, formula, None, mode))
}

fn state(d: &Document, c: ColumnId) -> FormulaState {
    d.project().column(c).unwrap().formula_state
}

#[test]
fn an_auto_chain_follows_the_source_in_dependency_order() {
    let (mut d, _, c) = book();
    // C is set before B exists as a formula, and D reads both: order must come from the
    // dependencies, not from the position of the columns.
    set(&mut d, c[1], "col(A)*10", RecalcMode::Auto).unwrap();
    set(&mut d, c[2], "col(B)+1", RecalcMode::Auto).unwrap();
    set(&mut d, c[3], "col(C)+col(A)", RecalcMode::Auto).unwrap();
    assert_eq!(values(&d, c[3]), [12.0, 23.0, 34.0, 45.0]);
    edit_cell(&mut d, c[0], 1, "20");
    assert_eq!(values(&d, c[1]), [10.0, 200.0, 30.0, 40.0]);
    assert_eq!(values(&d, c[2]), [11.0, 201.0, 31.0, 41.0]);
    assert_eq!(values(&d, c[3]), [12.0, 221.0, 34.0, 45.0]);
    assert!(c.iter().all(|c| state(&d, *c) == FormulaState::Current));
}

#[test]
fn a_chain_declared_against_the_column_order_still_resolves() {
    let (mut d, _, c) = book();
    // E reads D, D reads A: E sits to the right, but make the first-declared column read the later one.
    set(&mut d, c[4], "col(A)*2", RecalcMode::Auto).unwrap();
    set(&mut d, c[1], "col(E)+1", RecalcMode::Auto).unwrap();
    edit_cell(&mut d, c[0], 0, "100");
    assert_eq!(values(&d, c[4])[0], 200.0);
    assert_eq!(values(&d, c[1])[0], 201.0);
}

#[test]
fn a_manual_column_is_marked_pending_until_recalculated() {
    let (mut d, _, c) = book();
    set(&mut d, c[1], "col(A)*2", RecalcMode::Manual).unwrap();
    set(&mut d, c[2], "col(B)+1", RecalcMode::Auto).unwrap();
    assert_eq!(state(&d, c[1]), FormulaState::Current);
    edit_cell(&mut d, c[0], 0, "50");
    assert_eq!(values(&d, c[1])[0], 2.0, "manual values stay until the user recalculates");
    assert_eq!(state(&d, c[1]), FormulaState::Pending);
    assert_eq!(values(&d, c[2])[0], 3.0, "Auto reading a Manual column sees the old values");
    d.edit("Recalculate", |p| p.recalculate(c[1])).unwrap();
    assert_eq!(values(&d, c[1])[0], 100.0);
    assert_eq!(values(&d, c[2])[0], 101.0, "the Auto column that reads it follows");
    assert_eq!(state(&d, c[1]), FormulaState::Current);
    // One undo step undoes the recalculation and the pending mark.
    d.undo();
    assert_eq!(values(&d, c[1])[0], 2.0);
    assert_eq!(state(&d, c[1]), FormulaState::Pending);
}

#[test]
fn mode_none_never_recalculates() {
    let (mut d, _, c) = book();
    set(&mut d, c[1], "col(A)*2", RecalcMode::None).unwrap();
    edit_cell(&mut d, c[0], 0, "50");
    assert_eq!(values(&d, c[1])[0], 2.0);
    assert_eq!(state(&d, c[1]), FormulaState::Current);
    assert!(!d.project().column(c[1]).unwrap().is_locked());
    let err = d.edit("r", |p| p.recalculate(c[1])).unwrap_err();
    assert!(matches!(err, FormulaError::NotRecalculating(_)));
}

#[test]
fn cycles_are_refused_with_the_chain_named_and_nothing_changes() {
    let (mut d, _, c) = book();
    set(&mut d, c[1], "col(A)*2", RecalcMode::Auto).unwrap();
    set(&mut d, c[2], "col(B)+1", RecalcMode::Auto).unwrap();
    // B would read C, which reads B.
    let before = d.project().clone();
    let err = set(&mut d, c[1], "col(C)+1", RecalcMode::Auto).unwrap_err();
    let text = err.to_string();
    assert!(matches!(err, FormulaError::Cycle(_)), "{text}");
    assert!(text.contains("B -> C -> B"), "{text}");
    assert_eq!(*d.project(), before);
    // A column that reads itself runs once and does not recalculate (as in Origin): no loop.
    set(&mut d, c[3], "col(A)^2", RecalcMode::None).unwrap();
    set(&mut d, c[3], "col(D)+1", RecalcMode::Auto).unwrap();
    assert_eq!(values(&d, c[3]), [2.0, 5.0, 10.0, 17.0]);
    assert_eq!(d.project().column(c[3]).unwrap().recalc, RecalcMode::None);
    edit_cell(&mut d, c[0], 0, "9");
    assert_eq!(values(&d, c[3])[0], 2.0);
}

#[test]
fn deleting_or_renaming_a_source_breaks_the_column_and_keeps_its_values() {
    let (mut d, _, c) = book();
    set(&mut d, c[1], "col(A)*2", RecalcMode::Auto).unwrap();
    d.apply("Rename", |p| p.column_mut(c[0]).unwrap().short_name = "Src".into());
    assert_eq!(state(&d, c[1]), FormulaState::Broken);
    assert_eq!(values(&d, c[1]), [2.0, 4.0, 6.0, 8.0]);
    // Renaming it back repairs the column on the next change.
    d.apply("Rename", |p| p.column_mut(c[0]).unwrap().short_name = "A".into());
    assert_eq!(state(&d, c[1]), FormulaState::Current);
    d.apply("Delete", |p| {
        for w in &mut p.windows {
            if let sheaf_core::Window::Workbook(b) = w {
                b.sheets[0].columns.retain(|x| x.id != c[0]);
            }
        }
    });
    assert_eq!(state(&d, c[1]), FormulaState::Broken);
    assert_eq!(values(&d, c[1]), [2.0, 4.0, 6.0, 8.0]);
}

#[test]
fn undo_and_redo_restore_the_source_and_the_dependents_together() {
    let (mut d, _, c) = book();
    set(&mut d, c[1], "col(A)*10", RecalcMode::Auto).unwrap();
    set(&mut d, c[2], "col(B)+1", RecalcMode::Auto).unwrap();
    edit_cell(&mut d, c[0], 2, "7");
    assert_eq!(values(&d, c[2])[2], 71.0);
    assert_eq!(d.undo_label(), Some("Edit cell"));
    d.undo();
    assert_eq!(values(&d, c[0])[2], 3.0);
    assert_eq!(values(&d, c[1])[2], 30.0);
    assert_eq!(values(&d, c[2])[2], 31.0);
    d.redo();
    assert_eq!(values(&d, c[0])[2], 7.0);
    assert_eq!(values(&d, c[1])[2], 70.0);
    assert_eq!(values(&d, c[2])[2], 71.0);
}

#[test]
fn a_new_formula_on_a_source_column_refreshes_its_readers_in_the_same_step() {
    let (mut d, _, c) = book();
    set(&mut d, c[1], "col(A)*10", RecalcMode::None).unwrap();
    set(&mut d, c[2], "col(B)+1", RecalcMode::Auto).unwrap();
    assert_eq!(values(&d, c[2]), [11.0, 21.0, 31.0, 41.0]);
    set(&mut d, c[1], "col(A)*100", RecalcMode::None).unwrap();
    assert_eq!(values(&d, c[2]), [101.0, 201.0, 301.0, 401.0]);
    assert_eq!(d.undo_label(), Some("Set column values"));
    d.undo();
    assert_eq!(values(&d, c[2]), [11.0, 21.0, 31.0, 41.0]);
}

#[test]
fn a_row_range_is_kept_and_reused() {
    let (mut d, _, c) = book();
    d.edit("s", |p| p.set_column_values_with(c[1], "col(A)*10", Some(1..3), RecalcMode::Auto)).unwrap();
    edit_cell(&mut d, c[0], 0, "9");
    edit_cell(&mut d, c[0], 1, "5");
    let v = values(&d, c[1]);
    assert_eq!(&v[1..3], &[50.0, 30.0]);
    assert!(v[0].is_nan());
}

#[test]
fn rows_added_to_the_source_grow_the_auto_column() {
    let (mut d, _, c) = book();
    set(&mut d, c[1], "col(A)*2", RecalcMode::Auto).unwrap();
    edit_cell(&mut d, c[0], 5, "10");
    let v = values(&d, c[1]);
    assert_eq!(v.len(), 6);
    assert_eq!(v[5], 20.0);
    assert!(v[4].is_nan());
}

#[test]
fn the_mode_and_pending_mark_survive_json_and_old_projects_load() {
    let (mut d, _, c) = book();
    set(&mut d, c[1], "col(A)*2", RecalcMode::Manual).unwrap();
    edit_cell(&mut d, c[0], 0, "3");
    let json = serde_json::to_string(d.project()).unwrap();
    let back: Project = serde_json::from_str(&json).unwrap();
    let col = back.column(c[1]).unwrap();
    assert_eq!((col.recalc, col.formula_state), (RecalcMode::Manual, FormulaState::Pending));
    // A project written before this feature has neither key.
    assert!(!serde_json::to_string(d.project().column(c[0]).unwrap()).unwrap().contains("recalc"));
    let old = r#"{"id":5,"short_name":"B","formula":"col(A)*2"}"#;
    let col: sheaf_core::Column = serde_json::from_str(old).unwrap();
    assert_eq!((col.recalc, col.formula_state, col.formula_rows), (RecalcMode::None, FormulaState::Current, None));
}

#[test]
fn a_million_row_column_recalculates_quickly() {
    let n = 1_000_000;
    let mut p = Project::new();
    let book = p.add_workbook("");
    let a = p.add_column(book, 0, Designation::X, ColumnData::numeric((0..n).map(|i| i as f64).collect())).unwrap();
    let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(Vec::new())).unwrap();
    let c = p.add_column(book, 0, Designation::Y, ColumnData::numeric(Vec::new())).unwrap();
    let mut d = Document::new(p);
    set(&mut d, b, "sin(col(A))*2+1", RecalcMode::Auto).unwrap();
    set(&mut d, c, "col(B)*col(B)", RecalcMode::Auto).unwrap();
    let start = std::time::Instant::now();
    edit_cell(&mut d, a, 10, "0");
    let elapsed = start.elapsed();
    eprintln!("one cell edit with two dependent 1,000,000-row columns: {elapsed:?}");
    assert_eq!(values(&d, b)[10], 1.0);
    assert_eq!(values(&d, c)[10], 1.0);
    // A debug build is several times slower than release; this only guards against a quadratic mistake.
    assert!(elapsed.as_secs_f64() < 20.0, "{elapsed:?}");
}
