use sheaf_core::{ColumnData, ColumnId, Designation, Document, FormulaState, Project, RecalcMode, WindowId};

fn setup() -> (Document, WindowId, Vec<ColumnId>) {
    let mut project = Project::new();
    let book = project.add_workbook("");
    let mut ids = Vec::new();
    for _ in 0..5 {
        ids.push(project.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0])).unwrap());
    }
    project.column_mut(ids[0]).unwrap().params.insert("Scale".into(), "2.5".into());
    (Document::new(project), book, ids)
}

fn set(doc: &mut Document, column: ColumnId, formula: &str, mode: RecalcMode) {
    doc.edit("Set values", |p| p.set_column_values_with(column, formula, None, mode)).unwrap();
}

fn numbers(doc: &Document, column: ColumnId) -> Vec<f64> {
    doc.project().numeric(column).unwrap().to_vec()
}

fn parameter(doc: &mut Document, column: ColumnId, name: &str, value: &str) {
    doc.apply("Edit parameter", |p| {
        p.column_mut(column).unwrap().params.insert(name.into(), value.into());
    });
}

#[test]
fn editing_a_parameter_updates_auto_chains_and_undo_redo_together() {
    let (mut doc, _, c) = setup();
    set(&mut doc, c[1], "value(col(A)[scale]$)*col(A)", RecalcMode::Auto);
    set(&mut doc, c[2], "col(B)+1", RecalcMode::Auto);
    assert_eq!(numbers(&doc, c[1]), [2.5, 5.0, 7.5, 10.0]);
    parameter(&mut doc, c[0], "Scale", "3");
    assert_eq!(numbers(&doc, c[1]), [3.0, 6.0, 9.0, 12.0]);
    assert_eq!(numbers(&doc, c[2]), [4.0, 7.0, 10.0, 13.0]);
    doc.undo();
    assert_eq!(doc.project().column(c[0]).unwrap().params["Scale"], "2.5");
    assert_eq!(numbers(&doc, c[2]), [3.5, 6.0, 8.5, 11.0]);
    doc.redo();
    assert_eq!(numbers(&doc, c[2]), [4.0, 7.0, 10.0, 13.0]);
}

#[test]
fn manual_parameters_wait_for_recalculation_and_none_stays_unchanged() {
    let (mut doc, _, c) = setup();
    set(&mut doc, c[1], "col(A)[Scale]$", RecalcMode::Manual);
    set(&mut doc, c[2], "value(col(A)[Scale]$)", RecalcMode::None);
    parameter(&mut doc, c[0], "Scale", "7");
    assert_eq!(doc.project().column(c[1]).unwrap().formula_state, FormulaState::Pending);
    assert_eq!(doc.project().column(c[1]).unwrap().cell_text(0), "2.5");
    assert_eq!(numbers(&doc, c[2]), [2.5; 4]);
    doc.edit("Recalculate", |p| p.recalculate(c[1])).unwrap();
    assert_eq!(doc.project().column(c[1]).unwrap().cell_text(0), "7");
    doc.undo();
    assert_eq!(doc.project().column(c[1]).unwrap().cell_text(0), "2.5");
    assert_eq!(doc.project().column(c[1]).unwrap().formula_state, FormulaState::Pending);
}

#[test]
fn self_and_mutual_parameter_reads_do_not_make_data_cycles() {
    let (mut doc, _, c) = setup();
    parameter(&mut doc, c[1], "Bath Temp", "37");
    set(&mut doc, c[0], "value(col(A)[Scale]$)*i", RecalcMode::Auto);
    assert_eq!(doc.project().column(c[0]).unwrap().recalc, RecalcMode::Auto);
    assert_eq!(numbers(&doc, c[0]), [2.5, 5.0, 7.5, 10.0]);
    set(&mut doc, c[0], "col(B)[Bath Temp]$", RecalcMode::Auto);
    set(&mut doc, c[1], "col(A)[Scale]$", RecalcMode::Auto);
    parameter(&mut doc, c[0], "Scale", "4");
    assert_eq!(doc.project().column(c[1]).unwrap().cell_text(0), "4");
    assert_eq!(doc.project().column(c[0]).unwrap().cell_text(0), "37");
    assert_eq!(doc.project().column(c[0]).unwrap().formula_state, FormulaState::Current);
}

#[test]
fn missing_and_removed_parameter_rows_update_without_breaking_the_formula() {
    let (mut doc, _, c) = setup();
    set(&mut doc, c[1], "col(A)[Sample]$", RecalcMode::Auto);
    parameter(&mut doc, c[0], "Sample", "run-1");
    assert_eq!(doc.project().column(c[1]).unwrap().cell_text(0), "run-1");
    doc.apply("Delete parameter row", |p| {
        p.column_mut(c[0]).unwrap().params.remove("Sample");
    });
    assert!(doc.project().numeric(c[1]).unwrap().iter().all(|v| v.is_nan()));
    assert_eq!(doc.project().column(c[1]).unwrap().formula_state, FormulaState::Current);
    doc.undo();
    assert_eq!(doc.project().column(c[1]).unwrap().cell_text(0), "run-1");
}

#[test]
fn ordinal_aliases_follow_displayed_sheet_rows_even_after_a_column_is_removed() {
    let (mut doc, book, c) = setup();
    // The current worksheet UI displays user parameters alphabetically: Alpha, Scale.
    parameter(&mut doc, c[1], "Alpha", "ignored");
    set(&mut doc, c[2], "col(A)[D1]$", RecalcMode::Auto);
    set(&mut doc, c[3], "col(A)[D2]$", RecalcMode::Auto);
    assert!(doc.project().numeric(c[2]).unwrap().iter().all(|v| v.is_nan()));
    assert_eq!(doc.project().column(c[3]).unwrap().cell_text(0), "2.5");
    doc.apply("Remove column with unique parameter", |p| {
        p.workbook_mut(book).unwrap().sheets[0].columns.retain(|col| col.id != c[1]);
    });
    assert_eq!(doc.project().column(c[2]).unwrap().cell_text(0), "2.5");
    assert!(doc.project().numeric(c[3]).unwrap().iter().all(|v| v.is_nan()));
    doc.undo();
    assert_eq!(doc.project().column(c[3]).unwrap().cell_text(0), "2.5");
}

#[test]
fn a_new_parameter_row_does_not_invalidate_unrelated_manual_data_formulas() {
    let (mut doc, _, c) = setup();
    set(&mut doc, c[2], "col(A)*2", RecalcMode::Manual);
    parameter(&mut doc, c[1], "Alpha", "new row");
    assert_eq!(doc.project().column(c[2]).unwrap().formula_state, FormulaState::Current);
    assert_eq!(numbers(&doc, c[2]), [2.0, 4.0, 6.0, 8.0]);
}

#[test]
fn an_explicit_row_range_and_missing_ordinals_are_bounded() {
    let (mut doc, _, c) = setup();
    doc.edit("Set range", |p| p.set_column_values_with(c[1], "value(col(A)[Scale]$)", Some(1..3), RecalcMode::Auto))
        .unwrap();
    parameter(&mut doc, c[0], "Scale", "7");
    assert_eq!(numbers(&doc, c[1]), [1.0, 7.0, 7.0, 4.0]);
    for name in ["D0", "D9999999999999999999999999999999"] {
        set(&mut doc, c[2], &format!("col(A)[{name}]$"), RecalcMode::Auto);
        assert!(doc.project().numeric(c[2]).unwrap().iter().all(|v| v.is_nan()));
    }
}
