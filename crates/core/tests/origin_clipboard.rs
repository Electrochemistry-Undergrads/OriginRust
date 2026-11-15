//! Origin 2016 menu 57634/38798/38799 measurements, 2026-11-14. Raw synthetic probes:
//! scratch/t07-origin-copy-defaults. Clipboard CRLFs are normalized for Sheaf's LF transport.
use sheaf_core::clipboard::{CopyOptions, parse_table};
use sheaf_core::format::{NumberFormat, format_full_precision};
use sheaf_core::{ColumnData, Designation, Project};

#[test]
fn origin_decimal_copy_keeps_missing_numeric_cells_distinct_from_empty_text() {
    let mut p = Project::new();
    let book = p.add_workbook("");
    p.add_column(
        book,
        0,
        Designation::Y,
        ColumnData::numeric(vec![2.345678901234567, f64::NAN, 0.3, 1.234567890123456e-8]),
    )
    .unwrap();
    p.add_column(
        book,
        0,
        Designation::Y,
        ColumnData::mixed_numbers(vec![2.345678901234567, f64::NAN, 0.3, 1.234567890123456e-8]),
    )
    .unwrap();
    p.add_column(
        book,
        0,
        Designation::Label,
        ColumnData::text(vec!["word".into(), "".into(), "other".into(), "".into()]),
    )
    .unwrap();
    let ws = &mut p.workbook_mut(book).unwrap().sheets[0];
    ws.columns[0].format = NumberFormat::Decimal(2);
    ws.columns[1].format = NumberFormat::Decimal(2);
    assert_eq!(
        ws.copy_text(&[0, 1, 2], 0..4, CopyOptions::default()),
        "2.35\t2.35\tword\n--\t--\t\n0.30\t0.30\tother\n0.00\t0.00\t\n"
    );
}

#[test]
fn origin_labels_include_an_empty_formula_row_before_parameter_values() {
    let mut p = Project::new();
    let book = p.add_workbook("");
    p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0])).unwrap();
    p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![2.0])).unwrap();
    let ws = &mut p.workbook_mut(book).unwrap().sheets[0];
    ws.columns[0].long_name = "Potential".into();
    ws.columns[1].long_name = "Current".into();
    ws.columns[0].units = "V".into();
    ws.columns[1].units = "mA".into();
    ws.columns[0].comments = "First comment".into();
    ws.columns[1].comments = "Second comment".into();
    ws.columns[0].params.insert("Temperature".into(), "25".into());
    let options = CopyOptions { label_rows: true, ..Default::default() };
    assert_eq!(
        ws.copy_text(&[0, 1], 0..1, options),
        "Potential\tCurrent\nV\tmA\nFirst comment\tSecond comment\n\t\n25\t\n1\t2\n"
    );
    ws.columns[1].formula = Some("i*2".into());
    assert_eq!(parse_table(&ws.copy_text(&[0, 1], 0..1, options))[3], ["", "i*2"]);
}

#[test]
fn origin_external_full_precision_is_fifteen_significant_digits() {
    for (input, expected) in [
        (1.2345678901234567, "1.23456789012346"),
        (2.345678901234567, "2.34567890123457"),
        (3.456789012345678, "3.45678901234568"),
        (123456789.1234567, "123456789.123457"),
        (1.234567890123456e-8, "1.23456789012346E-8"),
        (0.30000000000000004, "0.3"),
    ] {
        assert_eq!(format_full_precision(input), expected);
    }
}

#[test]
fn missing_value_tokens_do_not_reduce_internal_numeric_paste_precision() {
    let mut p = Project::new();
    let book = p.add_workbook("");
    let values = vec![0.30000000000000004, f64::NAN, 1.2345678901234567];
    p.add_column(book, 0, Designation::Y, ColumnData::mixed_numbers(values.clone())).unwrap();
    let text = p.workbook(book).unwrap().sheet().copy_text(
        &[0],
        0..3,
        CopyOptions { full_precision: true, ..Default::default() },
    );
    assert!(text.contains("\n--\n"));
    p.paste_text(book, 0, 1, 0, &text).unwrap();
    let pasted = p.workbook(book).unwrap().sheet().columns[1].numeric().unwrap();
    for (got, expected) in pasted.iter().zip(values) {
        assert_eq!(got.to_bits(), expected.to_bits());
    }
}
