use sheaf_core::{
    Project,
    analysis::{Request, RowRange, RowStatsSpec, Tool},
    column::{ColumnData, Designation},
    data_range::DataRange,
    ids::ColumnId,
};

fn fixture() -> (Project, ColumnId, ColumnId) {
    let mut p = Project::new();
    let b = p.add_workbook("Rows");
    let a = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1., 2., 3., 4., 5.])).unwrap();
    let c = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![10., 20., 30., 40., 50.])).unwrap();
    p.column_mut(a).unwrap().units = "mA".into();
    p.column_mut(c).unwrap().units = "mA".into();
    (p, a, c)
}

fn run(p: &mut Project, ids: &[ColumnId], rows: RowRange) -> Vec<(String, Vec<f64>)> {
    let spec = RowStatsSpec { sem: true, min: true, max: true, n: true, sum: true, ..Default::default() };
    let report = p.analyze(&Request { columns: ids.to_vec(), rows, tool: Tool::RowStatistics(spec) }).unwrap();
    report
        .new_columns
        .iter()
        .map(|id| {
            let c = p.column(*id).unwrap();
            assert_eq!(c.units, if c.long_name == "Number of Points" { "" } else { "mA" });
            (c.long_name.clone(), c.numeric().unwrap().to_vec())
        })
        .collect()
}

fn data<'a>(result: &'a [(String, Vec<f64>)], name: &str) -> &'a [f64] {
    &result.iter().find(|(n, _)| n == name).unwrap().1
}

#[test]
fn each_contributor_uses_its_own_range_without_compressing_original_rows() {
    let (mut p, a, c) = fixture();
    p.column_mut(a).unwrap().data_range = Some(DataRange { begin: 1, end: Some(3) });
    p.column_mut(c).unwrap().data_range = Some(DataRange { begin: 2, end: Some(4) });
    let before: Vec<_> = [a, c].map(|id| p.column(id).unwrap().clone()).into();
    let r = run(&mut p, &[a, c], RowRange::All);
    assert_eq!(r.len(), 7);
    assert!(r.iter().all(|(_, values)| values.len() == 5 && values[0].is_nan()));
    assert_eq!(&data(&r, "Mean")[1..], [2., 16.5, 22., 50.]);
    assert_eq!(&data(&r, "Number of Points")[1..], [1., 2., 2., 1.]);
    assert_eq!(&data(&r, "Sum")[1..], [2., 33., 44., 50.]);
    assert_eq!(&data(&r, "Minimum")[1..], [2., 3., 4., 50.]);
    assert_eq!(&data(&r, "Maximum")[1..], [2., 30., 40., 50.]);
    assert!(data(&r, "Standard Deviation")[1].is_nan() && data(&r, "SE of Mean")[4].is_nan());
    assert!((data(&r, "Standard Deviation")[2] - 27. / 2.0_f64.sqrt()).abs() < 1e-12);
    assert!((data(&r, "SE of Mean")[2] - 13.5).abs() < 1e-12);
    for (id, source) in [a, c].into_iter().zip(before) {
        assert_eq!(p.column(id).unwrap(), &source);
    }
}

#[test]
fn explicit_requests_intersect_ranges_without_shifting_the_output() {
    for rows in [RowRange::Rows(2, 4), RowRange::XBetween(2., 4.)] {
        let (mut p, a, c) = fixture();
        p.column_mut(a).unwrap().data_range = Some(DataRange { begin: 2, end: Some(4) });
        p.column_mut(c).unwrap().data_range = Some(DataRange { begin: 0, end: Some(2) });
        let r = run(&mut p, &[a, c], rows);
        assert_eq!(&data(&r, "Mean")[1..4], [20., 16.5, 4.]);
        assert!(r.iter().all(|(_, values)| values[0].is_nan() && values[4].is_nan()));
    }
}

#[test]
fn ragged_masks_missing_and_filters_stay_missing_inside_the_range() {
    let (mut p, a, c) = fixture();
    let book = p.workbooks().next().unwrap().id;
    p.column_mut(a).unwrap().data_range = Some(DataRange { begin: 1, end: None });
    p.column_mut(a).unwrap().mask.insert(2);
    p.column_mut(c).unwrap().data = ColumnData::numeric(vec![10., f64::NAN, 30.]);
    p.column_mut(c).unwrap().data_range = Some(DataRange { begin: 1, end: Some(usize::MAX) });
    p.set_row_filter(book, 0, "i != 4").unwrap();
    let a_before = serde_json::to_value(p.column(a).unwrap()).unwrap();
    let c_before = serde_json::to_value(p.column(c).unwrap()).unwrap();
    let r = run(&mut p, &[a, c], RowRange::All);
    assert_eq!(data(&r, "Mean")[1], 2.);
    assert_eq!(data(&r, "Mean")[2], 30.);
    assert_eq!(data(&r, "Mean")[4], 5.);
    assert!(r.iter().all(|(_, values)| values[0].is_nan() && values[3].is_nan()));
    assert_eq!(serde_json::to_value(p.column(a).unwrap()).unwrap(), a_before);
    assert_eq!(serde_json::to_value(p.column(c).unwrap()).unwrap(), c_before);
}

#[test]
fn empty_and_extreme_ranges_can_be_reset_without_losing_values() {
    let (mut p, a, c) = fixture();
    p.column_mut(a).unwrap().data_range = Some(DataRange { begin: usize::MAX, end: None });
    p.column_mut(c).unwrap().data_range = Some(DataRange { begin: 9, end: Some(10) });
    let r = run(&mut p, &[a, c], RowRange::All);
    assert!(r.iter().all(|(_, values)| values.iter().all(|v| v.is_nan())));
    assert_eq!(p.reset_data_range(&[a, c]), 2);
    let r = run(&mut p, &[a, c], RowRange::All);
    assert_eq!(data(&r, "Mean"), [5.5, 11., 16.5, 22., 27.5]);
    assert_eq!(data(&r, "Number of Points"), [2.; 5]);
}
