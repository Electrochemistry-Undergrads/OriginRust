//! Set Multiple Columns Values: one formula on several columns, `j` being each column's position.

use sheaf_core::formula::FormulaError;
use sheaf_core::{ColumnData, ColumnId, Designation, Document, FormulaState, Project, RecalcMode, WindowId};

/// A workbook with a source column A = 1, 2, 3 and `empty` empty columns B, C, ...
fn book(empty: usize) -> (Document, WindowId, Vec<ColumnId>) {
    let mut p = Project::new();
    let book = p.add_workbook("");
    let mut ids = vec![p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap()];
    for _ in 0..empty {
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

#[test]
fn j_is_the_sheet_position_of_each_column_set() {
    let (mut d, _, c) = book(3);
    d.set_columns_values(&c[1..], "col(A)*j", None, RecalcMode::None).unwrap();
    assert_eq!(values(&d, c[1]), [2.0, 4.0, 6.0]);
    assert_eq!(values(&d, c[2]), [3.0, 6.0, 9.0]);
    assert_eq!(values(&d, c[3]), [4.0, 8.0, 12.0]);
    for id in &c[1..] {
        let col = d.project().column(*id).unwrap();
        assert_eq!(col.formula.as_deref(), Some("col(A)*j"), "the text with j is stored as typed");
        assert_eq!(col.recalc, RecalcMode::None);
    }
}

#[test]
fn j_counts_the_sheet_position_not_the_place_in_the_selection() {
    let (mut d, _, c) = book(3);
    // Only D is set: j is 4, not 1.
    d.set_columns_values(&c[3..], "j", None, RecalcMode::None).unwrap();
    assert_eq!(values(&d, c[3]), [4.0, 4.0, 4.0]);
}

#[test]
fn the_whole_set_is_one_undo_step() {
    let (mut d, _, c) = book(3);
    d.set_columns_values(&c[1..], "col(A)*j", None, RecalcMode::Auto).unwrap();
    assert_eq!(d.undo_label(), Some("Set multiple columns values"));
    d.undo().unwrap();
    for id in &c[1..] {
        let col = d.project().column(*id).unwrap();
        assert!(col.numeric().unwrap().is_empty() && col.formula.is_none());
    }
    assert_eq!(d.undo_label(), None, "one step covered the three columns");
    d.redo().unwrap();
    assert_eq!(values(&d, c[3]), [4.0, 8.0, 12.0]);
}

#[test]
fn auto_columns_follow_a_change_in_the_source_each_with_its_own_j() {
    let (mut d, _, c) = book(3);
    d.set_columns_values(&c[1..], "col(A)*j", None, RecalcMode::Auto).unwrap();
    edit_cell(&mut d, c[0], 0, "10");
    assert_eq!(values(&d, c[1])[0], 20.0);
    assert_eq!(values(&d, c[2])[0], 30.0);
    assert_eq!(values(&d, c[3])[0], 40.0);
    assert!(c[1..].iter().all(|id| d.project().column(*id).unwrap().formula_state == FormulaState::Current));
}

#[test]
fn manual_columns_go_pending_and_recalculate_with_their_own_j() {
    let (mut d, _, c) = book(2);
    d.set_columns_values(&c[1..], "col(A)*j", None, RecalcMode::Manual).unwrap();
    edit_cell(&mut d, c[0], 0, "10");
    assert_eq!(values(&d, c[2])[0], 3.0, "not recalculated yet");
    assert!(c[1..].iter().all(|id| d.project().column(*id).unwrap().formula_state == FormulaState::Pending));
    d.edit("recalculate", |p| p.recalculate(c[2])).unwrap();
    assert_eq!(values(&d, c[2])[0], 30.0);
}

#[test]
fn a_row_range_applies_to_every_column() {
    let (mut d, _, c) = book(2);
    d.set_columns_values(&c[1..], "j", Some(1..3), RecalcMode::None).unwrap();
    assert!(values(&d, c[1])[0].is_nan());
    assert_eq!(values(&d, c[1])[1..], [2.0, 2.0]);
    assert_eq!(values(&d, c[2])[1..], [3.0, 3.0]);
}

#[test]
fn a_chain_over_the_column_to_the_left_runs_from_left_to_right_whatever_the_selection_order() {
    let (mut d, _, c) = book(4);
    let shuffled = [c[3], c[1], c[4], c[2]];
    d.set_columns_values(&shuffled, "wcol(j-1)*2", None, RecalcMode::None).unwrap();
    assert_eq!(values(&d, c[1]), [2.0, 4.0, 6.0]);
    assert_eq!(values(&d, c[2]), [4.0, 8.0, 12.0]);
    assert_eq!(values(&d, c[3]), [8.0, 16.0, 24.0]);
    assert_eq!(values(&d, c[4]), [16.0, 32.0, 48.0]);
}

#[test]
fn an_auto_chain_over_wcol_j_minus_one_recalculates_in_dependency_order() {
    let (mut d, _, c) = book(4);
    d.set_columns_values(&c[1..], "wcol(j-1)+1", None, RecalcMode::Auto).unwrap();
    assert_eq!(values(&d, c[4]), [5.0, 6.0, 7.0]);
    edit_cell(&mut d, c[0], 1, "100");
    assert_eq!(values(&d, c[1])[1], 101.0);
    assert_eq!(values(&d, c[4])[1], 104.0, "the change travels down the chain B, C, D, E");
    assert!(c[1..].iter().all(|id| d.project().column(*id).unwrap().formula_state == FormulaState::Current));
}

#[test]
fn a_chain_that_reads_to_the_right_also_resolves_by_dependencies() {
    let (mut d, _, c) = book(3);
    // B reads C, C reads D, D reads A: B is set before C has values, and follows when C is set.
    d.set_columns_values(&c[3..], "col(1)", None, RecalcMode::Auto).unwrap();
    d.set_columns_values(&c[1..3], "wcol(j+1)+1", None, RecalcMode::Auto).unwrap();
    assert_eq!(values(&d, c[1]), [3.0, 4.0, 5.0]);
    edit_cell(&mut d, c[0], 0, "10");
    assert_eq!(values(&d, c[1])[0], 12.0);
}

#[test]
fn a_loop_between_the_columns_set_is_refused_and_changes_nothing() {
    let (mut d, _, c) = book(3);
    let before = d.project().clone();
    // B reads C and C reads B.
    let err = d.set_columns_values(&c[1..3], "wcol(j+1)+wcol(j-1)", None, RecalcMode::Auto).unwrap_err();
    let FormulaError::Cycle(chain) = &err else { panic!("{err}") };
    assert!(chain.contains('B') && chain.contains('C'), "the chain names both columns: {chain}");
    assert_eq!(d.project(), &before);
    assert_eq!(d.undo_label(), None);
}

#[test]
fn a_column_that_reads_its_own_position_runs_once() {
    let (mut d, _, c) = book(2);
    d.set_columns_values(&c[1..], "1", None, RecalcMode::None).unwrap();
    d.set_columns_values(&c[1..], "wcol(j+0) + j", None, RecalcMode::Auto).unwrap();
    for id in &c[1..] {
        assert_eq!(d.project().column(*id).unwrap().recalc, RecalcMode::None);
    }
    assert_eq!(values(&d, c[1]), [3.0, 3.0, 3.0]);
    assert_eq!(values(&d, c[2]), [4.0, 4.0, 4.0]);
}

#[test]
fn a_relative_column_that_does_not_exist_names_the_column_and_changes_nothing() {
    let (mut d, _, c) = book(2);
    let before = d.project().clone();
    // A is column 1: there is no column to its left.
    let err = d.set_columns_values(&c[0..2], "wcol(j-1)", None, RecalcMode::None).unwrap_err();
    assert!(err.to_string().contains("column A") && err.to_string().contains("col(j-1)"), "{err}");
    assert_eq!(d.project(), &before);
    // Past the right edge.
    let err = d.set_columns_values(&c[1..], "wcol(j+1)", None, RecalcMode::None).unwrap_err();
    assert!(err.to_string().contains("column C"), "{err}");
    assert_eq!(d.project(), &before);
}

#[test]
fn the_selection_must_be_one_sheet_and_not_empty() {
    let (mut d, _, c) = book(1);
    let second = d.apply("sheet", |p| p.add_workbook(""));
    let x = d.apply("col", |p| p.add_column(second, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap());
    assert!(matches!(d.set_columns_values(&[c[1], x], "1", None, RecalcMode::None), Err(FormulaError::Broken(_))));
    assert!(matches!(d.set_columns_values(&[], "1", None, RecalcMode::None), Err(FormulaError::Broken(_))));
    assert!(matches!(
        d.set_columns_values(&[ColumnId(999)], "1", None, RecalcMode::None),
        Err(FormulaError::NoSuchColumn(_))
    ));
}

#[test]
fn a_selection_of_one_column_behaves_like_set_column_values() {
    let (mut d, _, c) = book(1);
    d.set_columns_values(&c[1..], "col(A)*j", None, RecalcMode::None).unwrap();
    let (mut e, _, c2) = book(1);
    e.edit("one", |p| p.set_column_values_with(c2[1], "col(A)*j", None, RecalcMode::None)).unwrap();
    assert_eq!(values(&d, c[1]), values(&e, c2[1]));
}

#[test]
fn single_column_formulas_without_j_store_and_evaluate_as_before() {
    let (mut d, _, c) = book(2);
    d.apply("name", |p| p.column_mut(c[0]).unwrap().long_name = "Source".into());
    // The long name is not a short name: col("A") finds nothing and nothing changes.
    let err = d.edit("one", |p| p.set_column_values_with(c[1], "col(\"A\")", None, RecalcMode::None)).unwrap_err();
    assert!(err.to_string().contains("col(\"A\")"), "{err}");
    assert!(d.project().column(c[1]).unwrap().formula.is_none());
    for (formula, expected) in
        [("col(A)*10", [10.0, 20.0, 30.0]), ("col(\"Source\")+1", [2.0, 3.0, 4.0]), ("col(1)*2", [2.0, 4.0, 6.0])]
    {
        d.edit("one", |p| p.set_column_values_with(c[1], formula, None, RecalcMode::Auto)).unwrap();
        let col = d.project().column(c[1]).unwrap();
        assert_eq!(col.formula.as_deref(), Some(formula), "the text is stored as typed");
        assert_eq!(col.recalc, RecalcMode::Auto);
        assert_eq!(values(&d, c[1]), expected, "{formula}");
    }
    d.edit("two", |p| p.set_column_values_with(c[2], "col(B)+col(A)", None, RecalcMode::Auto)).unwrap();
    assert_eq!(values(&d, c[2]), [3.0, 6.0, 9.0]);
}

#[test]
fn j_alone_is_unknown_where_there_is_no_column_to_belong_to() {
    let (mut p, _, c) = {
        let (d, b, c) = book(1);
        (d.project().clone(), b, c)
    };
    // A sheet filter has no column, so no j.
    let book = p.workbooks().next().unwrap().id;
    assert!(p.set_row_filter(book, 0, "j > 1").is_err());
    // A column formula has one: j is its position.
    p.set_column_values(c[1], "j", None).unwrap();
    assert_eq!(p.numeric(c[1]).unwrap(), [2.0, 2.0, 2.0]);
}

#[test]
fn col_j_reads_the_column_called_j_and_is_the_position_only_when_there_is_none() {
    let (mut d, _, c) = book(10); // A to K; the tenth column is J
    let tenth = c[9];
    assert_eq!(d.project().column(tenth).unwrap().short_name, "J");
    d.edit("tenth", |p| p.set_column_values_with(tenth, "col(A)*100", None, RecalcMode::None)).unwrap();
    // As before T53, the single-column window reads column J whatever the case: col(j) on K.
    d.edit("k", |p| p.set_column_values_with(c[10], "col(j) + 1", None, RecalcMode::None)).unwrap();
    assert_eq!(values(&d, c[10]), [101.0, 201.0, 301.0]);
    d.edit("k", |p| p.set_column_values_with(c[10], "col(J)*2", None, RecalcMode::Auto)).unwrap();
    assert_eq!(values(&d, c[10]), [200.0, 400.0, 600.0]);
    // The explicit offset form is the position: K is column 11.
    d.set_columns_values(&c[10..], "wcol(j-1) + 1", None, RecalcMode::None).unwrap();
    assert_eq!(values(&d, c[10]), [101.0, 201.0, 301.0]);
    // Without a column called j, a bare col(j) is the position: for B, B itself, a self read.
    let (mut d, _, c) = book(3);
    d.edit("five", |p| p.set_column_values_with(c[1], "5", None, RecalcMode::None)).unwrap();
    d.edit("bare", |p| p.set_column_values_with(c[1], "col(j)*0 + col(A)", None, RecalcMode::Auto)).unwrap();
    assert_eq!(values(&d, c[1]), [1.0, 2.0, 3.0]);
    assert_eq!(d.project().column(c[1]).unwrap().recalc, RecalcMode::None);
}

#[test]
fn a_column_named_j_keeps_the_name_for_col_j() {
    let (mut d, _, c) = book(2);
    d.apply("name", |p| {
        let col = p.column_mut(c[1]).unwrap();
        col.short_name = "j".into();
    });
    d.edit("fill", |p| p.set_column_values_with(c[1], "col(A)*7", None, RecalcMode::None)).unwrap();
    d.edit("read", |p| p.set_column_values_with(c[2], "col(j)", None, RecalcMode::None)).unwrap();
    assert_eq!(values(&d, c[2]), [7.0, 14.0, 21.0], "col(j) is the column called j");
    d.edit("relative", |p| p.set_column_values_with(c[2], "col(j-1)", None, RecalcMode::None)).unwrap();
    assert_eq!(values(&d, c[2]), [7.0, 14.0, 21.0], "col(j-1) is still the position, here column B");
}

#[test]
fn moving_a_column_moves_j_and_the_formula_columns_follow() {
    let (mut d, _, c) = book(2);
    d.set_columns_values(&c[1..], "wcol(j-1)+100", None, RecalcMode::Auto).unwrap();
    assert_eq!(values(&d, c[2]), [201.0, 202.0, 203.0]);
    // Reordering the sheet changes what each column reads: the layout change is noticed.
    d.apply("swap", |p| {
        let book = p.workbooks().next().unwrap().id;
        let ws = &mut p.workbook_mut(book).unwrap().sheets[0];
        ws.columns.swap(1, 2);
    });
    // C is now second and reads A; B is now third and reads C.
    assert_eq!(values(&d, c[2]), [101.0, 102.0, 103.0]);
    assert_eq!(values(&d, c[1]), [201.0, 202.0, 203.0]);
}

#[test]
fn deleting_a_column_to_the_left_gives_the_columns_a_new_j() {
    let (mut d, _, c) = book(3); // A, B, C, D
    d.set_columns_values(&c[2..], "col(A)*j", None, RecalcMode::Auto).unwrap();
    assert_eq!(values(&d, c[2]), [3.0, 6.0, 9.0]);
    assert_eq!(values(&d, c[3]), [4.0, 8.0, 12.0]);
    d.apply("delete B", |p| {
        let book = p.workbooks().next().unwrap().id;
        p.workbook_mut(book).unwrap().sheets[0].columns.remove(1);
    });
    // C is now the second column, D the third.
    assert_eq!(values(&d, c[2]), [2.0, 4.0, 6.0]);
    assert_eq!(values(&d, c[3]), [3.0, 6.0, 9.0]);
}

#[test]
fn inserting_a_column_to_the_left_marks_a_manual_j_column_pending() {
    let (mut d, book_id, c) = book(2); // A, B, C
    d.set_columns_values(&c[2..], "col(A)*j", None, RecalcMode::Manual).unwrap();
    assert_eq!(d.project().column(c[2]).unwrap().formula_state, FormulaState::Current);
    d.apply("insert", |p| {
        p.add_column(book_id, 0, Designation::Y, ColumnData::numeric(Vec::new())).unwrap();
        let ws = &mut p.workbook_mut(book_id).unwrap().sheets[0];
        let added = ws.columns.pop().unwrap();
        ws.columns.insert(1, added);
    });
    assert_eq!(d.project().column(c[2]).unwrap().formula_state, FormulaState::Pending);
    assert_eq!(values(&d, c[2]), [3.0, 6.0, 9.0], "a Manual column waits");
    d.edit("recalculate", |p| p.recalculate(c[2])).unwrap();
    assert_eq!(values(&d, c[2]), [4.0, 8.0, 12.0]);
}

#[test]
fn a_formula_that_does_not_read_j_ignores_a_shift_in_position() {
    let (mut d, _, c) = book(3);
    d.set_columns_values(&c[2..], "col(A)*10", None, RecalcMode::Manual).unwrap();
    d.edit("recalculate", |p| p.recalculate(c[2])).unwrap();
    d.apply("delete B", |p| {
        let book = p.workbooks().next().unwrap().id;
        p.workbook_mut(book).unwrap().sheets[0].columns.remove(1);
    });
    assert_eq!(d.project().column(c[2]).unwrap().formula_state, FormulaState::Current);
}
