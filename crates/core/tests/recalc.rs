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
fn a_shorter_or_emptied_source_drops_the_stale_results() {
    let (mut d, _, c) = book();
    set(&mut d, c[1], "col(A)*2", RecalcMode::Auto).unwrap();
    assert_eq!(values(&d, c[1]), [2.0, 4.0, 6.0, 8.0]);
    d.apply("Replace source", |p| {
        p.column_mut(c[0]).unwrap().data = ColumnData::numeric(vec![5.0, 6.0]);
    });
    assert_eq!(values(&d, c[1]), [10.0, 12.0]);
    d.apply("Empty source", |p| {
        p.column_mut(c[0]).unwrap().data = ColumnData::numeric(Vec::new());
    });
    assert!(values(&d, c[1]).is_empty());
    assert_eq!(state(&d, c[1]), FormulaState::Current);
    d.undo();
    assert_eq!(values(&d, c[1]), [10.0, 12.0]);
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

#[test]
fn cell_reads_are_dependencies() {
    let (mut d, _, c) = book();
    // B reads row 2 of column 1 by position, C reads a column that the formula computes.
    set(&mut d, c[1], "cell(2, 1)*10", RecalcMode::Auto).unwrap();
    set(&mut d, c[2], "cell(3, 1+0*i)", RecalcMode::Auto).unwrap();
    assert_eq!(values(&d, c[1])[0], 20.0);
    assert_eq!(values(&d, c[2])[0], 3.0);
    edit_cell(&mut d, c[0], 1, "7");
    edit_cell(&mut d, c[0], 2, "9");
    assert_eq!(values(&d, c[1])[0], 70.0, "a constant column is tracked");
    assert_eq!(values(&d, c[2])[0], 9.0, "a computed column may be any column");
    // A formula that reads a column by position, and that column reads it back: a loop.
    set(&mut d, c[3], "cell(1, 5)", RecalcMode::Auto).unwrap();
    let e = set(&mut d, c[4], "cell(1, 4)", RecalcMode::Auto).unwrap_err();
    assert!(matches!(e, FormulaError::Cycle(_)), "{e}");
}

#[test]
fn a_text_result_makes_a_text_and_numeric_column() {
    let (mut d, _, c) = book();
    set(&mut d, c[1], "\"n=\" + text(col(A))", RecalcMode::Auto).unwrap();
    let col = d.project().column(c[1]).unwrap();
    assert!(col.data.is_mixed());
    assert_eq!(col.data.text_cell(0), Some("n=1"));
    assert_eq!(col.data.text_cell(3), Some("n=4"));
    // The text follows its source.
    edit_cell(&mut d, c[0], 0, "2.5");
    assert_eq!(d.project().column(c[1]).unwrap().data.text_cell(0), Some("n=2.5"));
    // A formula that gives text in some rows and numbers in others fills both kinds of cell.
    set(&mut d, c[2], "ifelse(col(A) > 2.6, left(\"abcdef\", col(A)), col(A))", RecalcMode::None).unwrap();
    let col = d.project().column(c[2]).unwrap();
    assert_eq!(col.numeric().unwrap()[..2], [2.5, 2.0]);
    assert_eq!(col.data.text_cell(2), Some("abc"));
    assert_eq!(col.data.text_cell(3), Some("abcd"));
    assert_eq!(col.data.text_cell(0), None);
}

#[test]
fn text_columns_can_be_read_and_copied() {
    let (mut d, _, c) = book();
    d.apply("Text", |p| {
        let col = p.column_mut(c[1]).unwrap();
        col.data = ColumnData::text(vec!["Hello".into(), "World".into(), String::new(), "x y".into()]);
    });
    set(&mut d, c[2], "upper(left(col(B)$, 3)) + \"-\" + text(col(A))", RecalcMode::None).unwrap();
    let col = d.project().column(c[2]).unwrap();
    assert_eq!(col.data.text_cell(0), Some("HEL-1"));
    assert_eq!(col.data.text_cell(2), Some("-3"));
    assert_eq!(col.data.text_cell(3), Some("X Y-4"));
    // A plain copy keeps the text; a number formula sees text as missing.
    set(&mut d, c[3], "col(B)", RecalcMode::None).unwrap();
    assert_eq!(d.project().column(c[3]).unwrap().data.text_cell(1), Some("World"));
    set(&mut d, c[4], "len(col(B)) + col(B)*0", RecalcMode::None).unwrap();
    assert!(d.project().numeric(c[4]).unwrap().iter().all(|v| v.is_nan()));
    set(&mut d, c[4], "len(col(B))", RecalcMode::None).unwrap();
    assert_eq!(values(&d, c[4]), [5.0, 5.0, 0.0, 3.0]);
}

#[test]
fn substituted_text_follows_both_source_and_occurrence_columns_and_undo() {
    let (mut d, _, c) = book();
    d.apply("Text", |p| {
        p.column_mut(c[1]).unwrap().data =
            ColumnData::text(vec!["raw raw".into(), "raw raw".into(), "raw raw raw".into(), String::new()]);
    });
    set(&mut d, c[2], r#"substitute(col(B),"clean","raw")$"#, RecalcMode::Auto).unwrap();
    set(&mut d, c[3], r#"substitute(col(B),"clean","raw",col(A))$"#, RecalcMode::Auto).unwrap();
    let cell = |d: &Document, col, row| d.project().column(col).unwrap().cell_text(row);
    assert!(d.project().column(c[2]).unwrap().data.is_mixed());
    // An empty text result remains a missing worksheet cell, displayed as --.
    for (row, expected) in ["clean clean", "clean clean", "clean clean clean", "--"].iter().enumerate() {
        assert_eq!(cell(&d, c[2], row), *expected);
    }
    for (row, expected) in ["clean raw", "raw clean", "raw raw clean", "--"].iter().enumerate() {
        assert_eq!(cell(&d, c[3], row), *expected);
    }
    edit_cell(&mut d, c[1], 0, "raw/éraw");
    assert_eq!(cell(&d, c[2], 0), "clean/éclean");
    assert_eq!(cell(&d, c[3], 0), "clean/éraw");
    edit_cell(&mut d, c[0], 0, "2");
    assert_eq!(cell(&d, c[3], 0), "raw/éclean");
    d.undo();
    assert_eq!(cell(&d, c[3], 0), "clean/éraw");
    d.redo();
    assert_eq!(cell(&d, c[3], 0), "raw/éclean");
    assert_eq!(state(&d, c[2]), FormulaState::Current);
    assert_eq!(state(&d, c[3]), FormulaState::Current);
}

#[test]
fn column_label_formulas_follow_header_edits_and_undo() {
    let (mut d, _, c) = book();
    d.apply("Headers", |p| {
        let col = p.column_mut(c[0]).unwrap();
        col.long_name = "Current".into();
        col.units = "mA".into();
        col.comments = "run 1".into();
    });
    set(&mut d, c[1], r#"col(A)[L]$+" ("+col(A)[U]$+") "+col(A)[C]$"#, RecalcMode::Auto).unwrap();
    set(&mut d, c[2], "len(col(B))", RecalcMode::Auto).unwrap();
    let text = |d: &Document| d.project().column(c[1]).unwrap().cell_text(0);
    assert_eq!(text(&d), "Current (mA) run 1");
    d.apply("Edit headers", |p| {
        let col = p.column_mut(c[0]).unwrap();
        col.units = "µA".into();
        col.comments = "run 2".into();
    });
    assert_eq!(text(&d), "Current (µA) run 2");
    assert_eq!(values(&d, c[2]), [18.0; 4]);
    d.undo();
    assert_eq!(text(&d), "Current (mA) run 1");
    d.redo();
    assert_eq!(text(&d), "Current (µA) run 2");
    assert_eq!(state(&d, c[1]), FormulaState::Current);
}

#[test]
fn reading_own_column_labels_keeps_auto_recalculation() {
    let (mut d, _, c) = book();
    d.apply("Scale", |p| p.column_mut(c[0]).unwrap().units = "2".into());
    set(&mut d, c[0], "col(A)[U]*i", RecalcMode::Auto).unwrap();
    assert_eq!(d.project().column(c[0]).unwrap().recalc, RecalcMode::Auto);
    assert_eq!(values(&d, c[0]), [2.0, 4.0, 6.0, 8.0]);
    d.apply("Change scale", |p| p.column_mut(c[0]).unwrap().units = "3".into());
    assert_eq!(values(&d, c[0]), [3.0, 6.0, 9.0, 12.0]);
    d.undo();
    assert_eq!(values(&d, c[0]), [2.0, 4.0, 6.0, 8.0]);
}

#[test]
fn mutual_header_reads_do_not_make_a_data_cycle() {
    let (mut d, _, c) = book();
    d.apply("Names", |p| {
        p.column_mut(c[1]).unwrap().long_name = "beta".into();
        p.column_mut(c[2]).unwrap().long_name = "gamma".into();
    });
    set(&mut d, c[1], "col(C)[L]$", RecalcMode::Auto).unwrap();
    set(&mut d, c[2], "col(B)[L]$", RecalcMode::Auto).unwrap();
    assert_eq!(d.project().column(c[1]).unwrap().cell_text(0), "gamma");
    assert_eq!(d.project().column(c[2]).unwrap().cell_text(0), "beta");
    d.apply("Rename", |p| p.column_mut(c[1]).unwrap().long_name = "renamed".into());
    assert_eq!(d.project().column(c[2]).unwrap().cell_text(0), "renamed");
    assert_eq!(state(&d, c[1]), FormulaState::Current);
    assert_eq!(state(&d, c[2]), FormulaState::Current);
}

#[test]
fn header_dependencies_mark_manual_results_pending_and_read_formula_text() {
    let (mut d, _, c) = book();
    d.apply("Units", |p| p.column_mut(c[0]).unwrap().units = "mA".into());
    set(&mut d, c[1], "col(A)[U]$", RecalcMode::Manual).unwrap();
    set(&mut d, c[2], "col(A)[O]$", RecalcMode::Auto).unwrap();
    d.apply("Units", |p| p.column_mut(c[0]).unwrap().units = "µA".into());
    assert_eq!(state(&d, c[1]), FormulaState::Pending);
    assert_eq!(d.project().column(c[1]).unwrap().cell_text(0), "mA");
    d.apply("Recalculate", |p| p.recalculate(c[1]).unwrap());
    assert_eq!(d.project().column(c[1]).unwrap().cell_text(0), "µA");
    set(&mut d, c[0], "i*2", RecalcMode::Auto).unwrap();
    assert_eq!(d.project().column(c[2]).unwrap().cell_text(0), "i*2");
    set(&mut d, c[0], "i*3", RecalcMode::Auto).unwrap();
    assert_eq!(d.project().column(c[2]).unwrap().cell_text(0), "i*3");
}

#[test]
fn text_without_a_format_tracks_its_numeric_columns_display_format() {
    use sheaf_core::format::NumberFormat;
    let (mut d, _, c) = book();
    d.apply("Decimal places", |p| p.column_mut(c[0]).unwrap().format = NumberFormat::Decimal(2));
    set(&mut d, c[1], "text(col(A))$", RecalcMode::Auto).unwrap();
    set(&mut d, c[2], "format(col(A))$", RecalcMode::Auto).unwrap();
    set(&mut d, c[3], r#"text(col(A),".1")$"#, RecalcMode::Auto).unwrap();
    set(&mut d, c[4], "text(col(A)[2])$", RecalcMode::Auto).unwrap();
    let text = |d: &Document, col, row| d.project().column(col).unwrap().cell_text(row);
    assert_eq!(text(&d, c[1], 0), "1.00");
    assert_eq!(text(&d, c[2], 0), "1");
    assert_eq!(text(&d, c[3], 0), "1.0");
    assert_eq!(text(&d, c[4], 0), "2.00");
    d.apply("Decimal places", |p| p.column_mut(c[0]).unwrap().format = NumberFormat::Decimal(3));
    assert_eq!(text(&d, c[1], 0), "1.000");
    assert_eq!(text(&d, c[2], 0), "1");
    assert_eq!(text(&d, c[3], 0), "1.0");
    assert_eq!(text(&d, c[4], 3), "2.000");
    d.undo();
    assert_eq!(text(&d, c[1], 0), "1.00");
    d.redo();
    assert_eq!(text(&d, c[1], 0), "1.000");
    edit_cell(&mut d, c[0], 1, "missing text");
    assert_eq!(text(&d, c[1], 1), "--", "Text still reads a numeric value, not a text cell");
    assert_eq!(text(&d, c[4], 0), "--");
    set(&mut d, c[4], "text(col(A))$", RecalcMode::Manual).unwrap();
    d.apply("Decimal places", |p| p.column_mut(c[0]).unwrap().format = NumberFormat::Decimal(4));
    assert_eq!(state(&d, c[4]), FormulaState::Pending);
}

#[test]
fn text_uses_engineering_and_month_column_formats_but_explicit_general_overrides_them() {
    use sheaf_core::format::{NameStyle, NumberFormat};
    let (mut d, _, c) = book();
    d.apply("Engineering", |p| {
        let a = p.column_mut(c[0]).unwrap();
        a.data = ColumnData::numeric(vec![1000.0, 2500.0]);
        a.format = NumberFormat::EngineeringSi(1);
    });
    set(&mut d, c[1], "text(col(A))$", RecalcMode::Auto).unwrap();
    set(&mut d, c[2], r#"text(col(A),"*")$"#, RecalcMode::Auto).unwrap();
    assert_eq!(d.project().column(c[1]).unwrap().cell_text(0), "1.0k");
    assert_eq!(d.project().column(c[1]).unwrap().cell_text(1), "2.5k");
    assert_eq!(d.project().column(c[2]).unwrap().cell_text(0), "1000");
    d.apply("Month", |p| {
        let a = p.column_mut(c[0]).unwrap();
        a.data = ColumnData::numeric(vec![1.0, 2.0]);
        a.format = NumberFormat::Month(NameStyle::Short);
    });
    assert_eq!(d.project().column(c[1]).unwrap().cell_text(0), "Jan");
    assert_eq!(d.project().column(c[1]).unwrap().cell_text(1), "Feb");
    assert_eq!(d.project().column(c[2]).unwrap().cell_text(0), "1");
}
