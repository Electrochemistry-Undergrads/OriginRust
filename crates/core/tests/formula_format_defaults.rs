use sheaf_core::format::NumberFormat;
use sheaf_core::{ColumnData, Designation, Document, Project, RecalcMode};

#[test]
fn an_empty_format_overrides_the_columns_display_without_changing_omitted_formats() {
    let mut p = Project::new();
    let book = p.add_workbook("");
    let source = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![12.345678901234567])).unwrap();
    p.column_mut(source).unwrap().format = NumberFormat::Decimal(2);
    for (formula, expected) in [
        (r#"text(col(A))$"#, "12.35"),
        (r#"format(col(A))$"#, "12.34568"),
        (r#"text(col(A),"*")$"#, "12.34568"),
        (r#"text(col(A),"")$"#, "12.345678901235"),
        (r#"format(col(A),"")$"#, "12.345678901235"),
    ] {
        let target = p.add_column(book, 0, Designation::Y, ColumnData::default()).unwrap();
        p.set_column_values_with(target, formula, None, RecalcMode::Auto).unwrap();
        assert_eq!(p.column(target).unwrap().data.text_cell(0), Some(expected), "{formula}");
    }
}

#[test]
fn empty_format_formulas_follow_source_edits_and_undo_at_full_default_precision() {
    let mut p = Project::new();
    let book = p.add_workbook("");
    let a = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![12.345678901234567])).unwrap();
    let b = p.add_column(book, 0, Designation::Y, ColumnData::default()).unwrap();
    p.set_column_values_with(b, r#"text(col(A),"")$"#, None, RecalcMode::Auto).unwrap();
    let mut d = Document::new(p);
    d.apply("Edit source", |p| p.column_mut(a).unwrap().data = ColumnData::numeric(vec![std::f64::consts::PI]));
    assert_eq!(d.project().column(b).unwrap().data.text_cell(0), Some("3.1415926535898"));
    d.undo();
    assert_eq!(d.project().column(b).unwrap().data.text_cell(0), Some("12.345678901235"));
    d.redo();
    d.apply("Change display", |p| p.column_mut(a).unwrap().format = NumberFormat::Decimal(1));
    assert_eq!(d.project().column(b).unwrap().data.text_cell(0), Some("3.1415926535898"));
}
