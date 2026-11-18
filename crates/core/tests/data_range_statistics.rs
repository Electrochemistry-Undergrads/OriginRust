use sheaf_core::{
    Project,
    analysis::{Request, ResultTable, RowRange, StatisticsSpec, Tool},
    column::{ColumnData, Designation},
    data_range::{DataRange, RangeEnd},
    ids::ColumnId,
};

fn fixture() -> (Project, ColumnId, ColumnId) {
    let mut p = Project::new();
    let book = p.add_workbook("Range");
    let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1., 2., 3., 4., 5., 6.])).unwrap();
    let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![10., 20., 30., 40., 50., 60.])).unwrap();
    p.column_mut(y).unwrap().units = "mA".into();
    (p, x, y)
}

fn stats(p: &mut Project, columns: &[ColumnId], rows: RowRange) -> ResultTable {
    p.analyze(&Request { columns: columns.to_vec(), rows, tool: Tool::Statistics(StatisticsSpec::all()) })
        .unwrap()
        .table
        .unwrap()
}

fn value(t: &ResultTable, row: usize, name: &str) -> f64 {
    t.rows[row].1[t.headers.iter().position(|h| h == name).unwrap()]
}

#[test]
fn descriptive_range_matches_the_accepted_origin2016_fixture_without_changing_source() {
    let (mut p, x, y) = fixture();
    p.set_data_range_end(&[y], RangeEnd::Begin, 1).unwrap();
    p.set_data_range_end(&[y], RangeEnd::End, 3).unwrap();
    // The X range is irrelevant to univariate descriptive statistics on Y.
    p.set_data_range_end(&[x], RangeEnd::Begin, 5).unwrap();
    let source = p.column(y).unwrap().clone();
    let t = stats(&mut p, &[y], RowRange::All);
    assert_eq!((value(&t, 0, "N"), value(&t, 0, "Mean")), (3., 30.));
    assert_eq!((value(&t, 0, "Missing"), value(&t, 0, "Sum")), (0., 90.));
    assert_eq!((value(&t, 0, "Minimum"), value(&t, 0, "Maximum")), (20., 40.));
    assert_eq!(value(&t, 0, "SD"), 10.);
    assert_eq!(t.unit(0, t.headers.iter().position(|h| h == "Mean").unwrap()), "mA");
    assert_eq!(serde_json::to_value(p.column(y).unwrap()).unwrap(), serde_json::to_value(&source).unwrap());
    p.reset_data_range(&[y]);
    let t = stats(&mut p, &[y], RowRange::All);
    assert_eq!((value(&t, 0, "N"), value(&t, 0, "Mean")), (6., 35.));
}

#[test]
fn explicit_rows_and_x_requests_intersect_the_saved_range_in_original_row_coordinates() {
    let (mut p, _, y) = fixture();
    p.column_mut(y).unwrap().data_range = Some(DataRange { begin: 1, end: Some(3) });
    for rows in [RowRange::Rows(3, 6), RowRange::XBetween(3., 6.), RowRange::XBetween(6., 3.)] {
        let t = stats(&mut p, &[y], rows);
        assert_eq!((value(&t, 0, "N"), value(&t, 0, "Mean")), (2., 35.));
        assert_eq!(value(&t, 0, "Missing"), 0.);
    }
}

#[test]
fn disjoint_and_outside_ranges_are_empty_without_counting_excluded_cells_as_missing() {
    for (range, rows) in [
        (DataRange { begin: 4, end: None }, RowRange::Rows(1, 2)),
        (DataRange { begin: 0, end: Some(1) }, RowRange::Rows(5, 6)),
        (DataRange { begin: usize::MAX, end: None }, RowRange::All),
        (DataRange { begin: 5, end: Some(2) }, RowRange::All),
    ] {
        let (mut p, _, y) = fixture();
        p.column_mut(y).unwrap().data_range = Some(range);
        let t = stats(&mut p, &[y], rows);
        assert_eq!((value(&t, 0, "N"), value(&t, 0, "Missing"), value(&t, 0, "Sum")), (0., 0., 0.));
        assert!(value(&t, 0, "Mean").is_nan() && value(&t, 0, "Minimum").is_nan());
    }
    let (mut p, _, y) = fixture();
    p.column_mut(y).unwrap().data_range = Some(DataRange { begin: 0, end: Some(usize::MAX) });
    assert_eq!(value(&stats(&mut p, &[y], RowRange::All), 0, "N"), 6.);
}

#[test]
fn each_column_keeps_its_own_range_and_existing_missing_and_mask_semantics() {
    let (mut p, x, y) = fixture();
    p.column_mut(x).unwrap().data_range = Some(DataRange { begin: 0, end: Some(1) });
    let c = p.column_mut(y).unwrap();
    c.data_range = Some(DataRange { begin: 1, end: Some(4) });
    c.numeric_mut().unwrap()[2] = f64::NAN;
    c.mask.insert(3);
    let source = c.clone();
    let t = stats(&mut p, &[x, y], RowRange::All);
    assert_eq!((value(&t, 0, "N"), value(&t, 0, "Mean")), (2., 1.5));
    assert_eq!((value(&t, 1, "N"), value(&t, 1, "Mean"), value(&t, 1, "Missing")), (2., 35., 2.));
    assert_eq!(serde_json::to_value(p.column(y).unwrap()).unwrap(), serde_json::to_value(&source).unwrap());
}

#[test]
fn a_filter_inside_the_range_retains_existing_missing_counts_and_can_be_disabled() {
    let (mut p, _, y) = fixture();
    p.column_mut(y).unwrap().data_range = Some(DataRange { begin: 1, end: Some(4) });
    let book = p.workbooks().next().unwrap().id;
    p.set_row_filter(book, 0, "col(B) != 30").unwrap();
    let t = stats(&mut p, &[y], RowRange::All);
    assert_eq!((value(&t, 0, "N"), value(&t, 0, "Missing")), (3., 1.));
    assert!((value(&t, 0, "Mean") - 110. / 3.).abs() < 1e-12);
    p.workbook_mut(book).unwrap().sheets[0].filter.as_mut().unwrap().enabled = false;
    let t = stats(&mut p, &[y], RowRange::All);
    assert_eq!((value(&t, 0, "N"), value(&t, 0, "Mean"), value(&t, 0, "Missing")), (4., 35., 0.));
}
