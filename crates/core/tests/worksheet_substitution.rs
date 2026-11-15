use sheaf_core::format::NumberFormat;
use sheaf_core::worksheet::Worksheet;
use sheaf_core::{ColumnData, ColumnId, Designation, Document, FormulaState, Project, RecalcMode, WindowId};

fn column(p: &mut Project, book: WindowId, sheet: usize, data: ColumnData) -> ColumnId {
    p.add_column(book, sheet, Designation::Y, data).unwrap()
}

fn fixture() -> (Project, WindowId, ColumnId, ColumnId) {
    let mut p = Project::new();
    let src = p.add_workbook("");
    p.workbook_mut(src).unwrap().name = "Src".into();
    p.workbook_mut(src).unwrap().long_name = "Source Book".into();
    let a = column(&mut p, src, 0, ColumnData::numeric(vec![std::f64::consts::PI, 12.5]));
    p.column_mut(a).unwrap().format = NumberFormat::Decimal(2);
    p.column_mut(a).unwrap().long_name = "Voltage V".into();
    let dst = p.add_workbook("");
    p.workbook_mut(dst).unwrap().name = "Dst".into();
    let b = column(&mut p, dst, 0, ColumnData::default());
    (p, src, a, b)
}

#[test]
fn static_book_sheet_and_column_selectors_use_the_existing_lookup_rules() {
    let (mut p, src, _, b) = fixture();
    p.workbook_mut(src).unwrap().sheets.push(Worksheet::new("Other Data"));
    column(&mut p, src, 1, ColumnData::numeric(vec![99.0]));
    let collision = column(&mut p, src, 0, ColumnData::numeric(vec![7.0]));
    p.column_mut(collision).unwrap().long_name = "A".into();
    for (formula, expected) in [
        (r#""%([sRc]sHeEt1,A,1)""#, "3.14159265358979"),
        (r#""%([Src]1,1,2)""#, "12.5"),
        (r#""%(["Source Book"]Sheet1,"Voltage V",1,.2)""#, "3.14"),
        (r#""%([Src]Sheet1,"A",1)""#, "7"),
        (r#""%([Src]"Other Data",A,1)""#, "99"),
        (r#""%([Src]2,1,1)""#, "99"),
    ] {
        p.set_column_values_with(b, formula, Some(0..1), RecalcMode::Auto).unwrap();
        assert_eq!(p.column(b).unwrap().data.text_cell(0), Some(expected), "{formula}");
    }
}

#[test]
fn auto_and_manual_cell_reads_follow_edits_display_changes_and_undo() {
    let (mut p, _, a, b) = fixture();
    let dst = p.locate(b).unwrap().0.id;
    let manual = column(&mut p, dst, 0, ColumnData::default());
    let formula = r#""full:%([Src]Sheet1,A,i); shown:%([Src]Sheet1,A,i,W)""#;
    p.set_column_values_with(b, formula, None, RecalcMode::Auto).unwrap();
    p.set_column_values_with(manual, formula, None, RecalcMode::Manual).unwrap();
    let mut doc = Document::new(p);
    assert_eq!(doc.project().column(b).unwrap().data.text_cell(0), Some("full:3.14159265358979; shown:3.14"));
    doc.apply("Change source and display", |p| {
        p.column_mut(a).unwrap().data = ColumnData::numeric(vec![12.345678901234567]);
        p.column_mut(a).unwrap().format = NumberFormat::Decimal(1);
    });
    assert_eq!(doc.project().column(b).unwrap().len(), 1);
    assert_eq!(doc.project().column(b).unwrap().data.text_cell(0), Some("full:12.3456789012346; shown:12.3"));
    assert_eq!(doc.project().column(manual).unwrap().formula_state, FormulaState::Pending);
    assert_eq!(doc.project().column(manual).unwrap().data.text_cell(0), Some("full:3.14159265358979; shown:3.14"));
    doc.apply("Recalculate manual", |p| p.recalculate(manual).unwrap());
    assert_eq!(doc.project().column(manual).unwrap().data.text_cell(0), Some("full:12.3456789012346; shown:12.3"));
    doc.undo();
    doc.undo();
    assert_eq!(doc.project().column(b).unwrap().data.text_cell(0), Some("full:3.14159265358979; shown:3.14"));
    doc.redo();
    doc.apply("Change only display", |p| p.column_mut(a).unwrap().format = NumberFormat::Decimal(3));
    assert_eq!(doc.project().column(b).unwrap().data.text_cell(0), Some("full:12.3456789012346; shown:12.346"));
}

#[test]
fn unknown_references_and_cycles_are_refused_without_changing_the_project() {
    let (mut p, _, a, b) = fixture();
    p.set_column_values_with(b, "%([Src]Sheet1,A,i)*2", None, RecalcMode::Auto).unwrap();
    let before = p.clone();
    for formula in [r#""%([Absent]Sheet1,A,1)""#, r#""%([Src]Absent,A,1)""#, r#""%([Src]Sheet1,Z,1)""#] {
        assert!(p.set_column_values_with(b, formula, None, RecalcMode::Auto).is_err());
        assert_eq!(p, before);
    }
    assert!(p.set_column_values_with(a, "%([Dst]Sheet1,A,i)+1", None, RecalcMode::Auto).is_err());
    assert_eq!(p, before);
    p.set_column_values_with(a, "%([Src]Sheet1,A,i)*2", None, RecalcMode::Auto).unwrap();
    assert_eq!(p.column(a).unwrap().recalc, RecalcMode::None);
    assert_eq!(p.numeric(a).unwrap()[1], 25.0);
    assert_eq!(p.numeric(b).unwrap()[1], 50.0);
}

#[test]
fn a_cell_supplied_row_expression_is_an_independent_data_dependency() {
    let (mut p, _, _, b) = fixture();
    let dst = p.locate(b).unwrap().0.id;
    let row = column(&mut p, dst, 0, ColumnData::numeric(vec![2.0, 1.0]));
    p.set_column_values_with(b, r#""%([Src]Sheet1,A,col(B))""#, None, RecalcMode::Auto).unwrap();
    let mut doc = Document::new(p);
    assert_eq!(doc.project().column(b).unwrap().data.text_cell(0), Some("12.5"));
    doc.apply("Change row source", |p| p.column_mut(row).unwrap().data = ColumnData::numeric(vec![1.0, 2.0]));
    assert_eq!(doc.project().column(b).unwrap().data.text_cell(0), Some("3.14159265358979"));
    doc.undo();
    assert_eq!(doc.project().column(b).unwrap().data.text_cell(0), Some("12.5"));
}
