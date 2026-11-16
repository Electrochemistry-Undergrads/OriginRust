//! Swap Columns (Origin 2016's `colswap`, probe `re-workspace/scratch/t86/w1`, local only) and Paste
//! Transpose (T86, workbook-model.md "Swap Columns and Paste Transpose").
use sheaf_core::format::NumberFormat;
use sheaf_core::{ColumnData, Designation, Project};

#[test]
fn swapped_columns_take_everything_along_as_colswap() {
    // Origin's probe: A..E, B with long name Bee and width 12, D a Y-error column Dee (mm).
    let mut p = Project::new();
    let book = p.add_workbook("Book1");
    let mut ids = Vec::new();
    for (k, v) in
        [vec![1.0, 2.0], vec![10.0, 20.0], vec![100.0, 200.0], vec![7.0, 8.0], vec![5.0, 6.0]].into_iter().enumerate()
    {
        let d = if k == 0 { Designation::X } else { Designation::Y };
        ids.push(p.add_column(book, 0, d, ColumnData::numeric(v)).unwrap());
    }
    {
        let b = p.column_mut(ids[1]).unwrap();
        b.long_name = "Bee".into();
        b.width = Some(12);
    }
    {
        let d = p.column_mut(ids[3]).unwrap();
        d.long_name = "Dee".into();
        d.units = "mm".into();
        d.designation = Designation::YError;
        d.format = NumberFormat::default();
    }
    let ws = &mut p.workbook_mut(book).unwrap().sheets[0];
    assert!(ws.swap_columns(ids[1], ids[3]));
    let names: Vec<&str> = ws.columns.iter().map(|c| c.short_name.as_str()).collect();
    assert_eq!(names, ["A", "D", "C", "B", "E"], "Origin's order after colswap");
    let (d, b) = (&ws.columns[1], &ws.columns[3]);
    assert_eq!(
        (d.long_name.as_str(), d.units.as_str(), d.designation, d.width),
        ("Dee", "mm", Designation::YError, None)
    );
    assert_eq!((b.long_name.as_str(), b.designation, b.width), ("Bee", Designation::Y, Some(12)));
    assert_eq!(d.numeric().unwrap(), [7.0, 8.0]);
    // One column is not a swap.
    assert!(!ws.swap_columns(ids[1], ids[1]));
}

#[test]
fn paste_transpose_turns_rows_into_columns() {
    let mut p = Project::new();
    let book = p.add_workbook("Book1");
    let a = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0; 4])).unwrap();
    p.add_column(book, 0, Designation::Y, ColumnData::default()).unwrap();
    // Two rows of three fields at row 1 of column A: three rows of two columns, one column added.
    let report = p.paste_text_transposed(book, 0, 0, 1, "1\t2\t3\nx\t5").unwrap();
    assert_eq!((report.rows, report.columns, report.columns_added), (3, 2, 0));
    let sheet = &p.workbook(book).unwrap().sheets[0];
    assert_eq!(sheet.columns.len(), 2);
    assert_eq!(p.numeric(a).unwrap(), [0.0, 1.0, 2.0, 3.0]);
    let b = &sheet.columns[1];
    assert_eq!(b.cell_text(1), "x");
    assert_eq!(b.cell_text(2), "5");
    assert_eq!(b.cell_text(3), "", "a short row gives an empty cell");
    // Wider than the sheet: columns are added at the right.
    let report = p.paste_text_transposed(book, 0, 1, 0, "1\n2\n3").unwrap();
    assert_eq!((report.rows, report.columns, report.columns_added), (1, 3, 2));
}
