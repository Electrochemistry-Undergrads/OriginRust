//! Time Series Explorer with Rescale Y (T488): the detail layer's Y axis fits the interval shown.

use sheaf_core::{ColumnData, Designation, Project};

#[test]
fn rescale_y_fits_the_detail_axis_to_the_interval() {
    // y = x * x for x = 1..100: the first of 10 intervals (x 1 to 10.9) holds y up to about 119
    let mut p = Project::new();
    let b = p.add_workbook("");
    let xs: Vec<f64> = (1..=100).map(f64::from).collect();
    let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(xs.clone())).unwrap();
    let _ = x;
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(xs.iter().map(|v| v * v).collect())).unwrap();
    let g = p.plot_time_series_explorer(&[y]).unwrap();
    let detail = |p: &Project| {
        let geo = crate::layout_geometry(p, g).unwrap();
        (geo[1].y.min, geo[1].y.max)
    };
    let (lo, hi) = detail(&p);
    assert!(hi > 9000.0, "the whole range without Rescale Y: {lo} {hi}");
    p.set_explorer_rescale_y(g, true).unwrap();
    let (lo, hi) = detail(&p);
    assert!(lo <= 1.0 && (119.0..=200.0).contains(&hi), "the first interval's range: {lo} {hi}");
    // the last interval
    p.explore_interval(g, 9).unwrap();
    let (lo, hi) = detail(&p);
    assert!((7000.0..=8200.0).contains(&lo) && hi >= 10_000.0, "{lo} {hi}");
    // off again: the whole range
    p.set_explorer_rescale_y(g, false).unwrap();
    assert!(detail(&p).1 > 9000.0);
}
