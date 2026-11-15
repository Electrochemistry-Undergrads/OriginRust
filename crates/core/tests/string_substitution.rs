use sheaf_core::{ColumnData, ColumnId, Designation, Document, FormulaState, Project, RecalcMode, WindowId};

fn col(p: &mut Project, book: WindowId, data: ColumnData) -> ColumnId {
    p.add_column(book, 0, Designation::Y, data).unwrap()
}

#[test]
fn substituted_remote_cells_and_labels_recalculate_and_undo_in_one_step() {
    let mut p = Project::new();
    let src = p.add_workbook("");
    p.workbook_mut(src).unwrap().name = "Src".into();
    let a = col(&mut p, src, ColumnData::text(vec!["one".into(), "two".into()]));
    p.column_mut(a).unwrap().units = "mV".into();
    let dst = p.add_workbook("");
    let b = col(&mut p, dst, ColumnData::default());
    let formula = r#""%([Src]Sheet1!col(A)[U]$):%([Src]Sheet1!col(A)[i]$)""#;
    p.set_column_values_with(b, formula, None, RecalcMode::Auto).unwrap();
    let mut doc = Document::new(p);
    assert_eq!(doc.project().column(b).unwrap().data.text_cell(0), Some("mV:one"));
    doc.apply("Edit remote text and units", |p| {
        let a = p.column_mut(a).unwrap();
        a.units = "V".into();
        a.data = ColumnData::text(vec!["changed".into(), "two".into()]);
    });
    assert_eq!(doc.project().column(b).unwrap().data.text_cell(0), Some("V:changed"));
    doc.undo();
    assert_eq!(doc.project().column(b).unwrap().data.text_cell(0), Some("mV:one"));
    doc.redo();
    assert_eq!(doc.project().column(b).unwrap().data.text_cell(0), Some("V:changed"));
}

#[test]
fn substituted_self_metadata_is_not_a_data_cycle_and_manual_and_none_keep_their_modes() {
    let mut p = Project::new();
    let book = p.add_workbook("");
    let a = col(&mut p, book, ColumnData::numeric(vec![1.0, 2.0]));
    let b = col(&mut p, book, ColumnData::default());
    let c = col(&mut p, book, ColumnData::default());
    p.column_mut(a).unwrap().params.insert("Scale".into(), "2.5".into());
    for (id, mode) in [(a, RecalcMode::Auto), (b, RecalcMode::Manual), (c, RecalcMode::None)] {
        p.set_column_values_with(id, "%(col(A)[Scale]$)*i", None, mode).unwrap();
    }
    let mut doc = Document::new(p);
    assert_eq!(doc.project().column(a).unwrap().recalc, RecalcMode::Auto);
    doc.apply("Change scale", |p| {
        p.column_mut(a).unwrap().params.insert("Scale".into(), "4".into());
    });
    assert_eq!(doc.project().numeric(a).unwrap(), [4.0, 8.0]);
    assert_eq!(doc.project().numeric(b).unwrap(), [2.5, 5.0]);
    assert_eq!(doc.project().column(b).unwrap().formula_state, FormulaState::Pending);
    assert_eq!(doc.project().numeric(c).unwrap(), [2.5, 5.0]);
    doc.apply("Recalculate", |p| p.recalculate(b).unwrap());
    assert_eq!(doc.project().numeric(b).unwrap(), [4.0, 8.0]);
}

#[test]
fn a_substituted_data_self_read_still_runs_once() {
    let mut p = Project::new();
    let book = p.add_workbook("");
    let a = col(&mut p, book, ColumnData::text(vec!["12.5".into(), "7".into()]));
    p.set_column_values_with(a, "%(col(A)[i]$)*2", None, RecalcMode::Auto).unwrap();
    assert_eq!(p.column(a).unwrap().recalc, RecalcMode::None);
    assert_eq!(p.numeric(a).unwrap(), [25.0, 14.0]);
}

#[test]
fn substituted_cell_text_is_never_executed_or_resolved_as_a_new_reference() {
    let mut p = Project::new();
    let book = p.add_workbook("");
    let a = col(&mut p, book, ColumnData::text(vec!["%(col(Nope)$)".into(), "2+3".into()]));
    let b = col(&mut p, book, ColumnData::default());
    p.set_column_values_with(b, r#""literal:%(col(A)$)""#, None, RecalcMode::Auto).unwrap();
    assert_eq!(p.column(b).unwrap().data.text_cell(0), Some("literal:%(col(Nope)$)"));
    assert_eq!(p.column(b).unwrap().data.text_cell(1), Some("literal:2+3"));
    p.set_column_values(a, "%(col(A)$)", None).unwrap();
    assert!(p.numeric(a).unwrap().iter().all(|x| x.is_nan()));
}
