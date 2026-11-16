//! Set Sampling Interval and Show X Column against Origin 2016's `colint` and `colshowx` (probes
//! `re-workspace/scratch/t84/s1`-`s3`, local only; workbook-model.md "Sampling interval").

use super::*;
use crate::template::Template;

/// Origin's probe sheet: A = 1..5 (X), B = 10..50, C = 100..500, D = 7..11.
fn sheet() -> (Project, Vec<ColumnId>) {
    let mut p = Project::new();
    let book = p.add_workbook("Book1");
    let mut ids = Vec::new();
    for (k, start) in [1.0, 10.0, 100.0, 7.0].into_iter().enumerate() {
        let step = if k == 0 || k == 3 { 1.0 } else { start };
        let d = if k == 0 { Designation::X } else { Designation::Y };
        let v = (0..5).map(|i| start + i as f64 * step).collect();
        ids.push(p.add_column(book, 0, d, ColumnData::numeric(v)).unwrap());
    }
    (p, ids)
}

fn names(p: &Project, any: ColumnId) -> Vec<(String, Designation)> {
    let (_, sheet, _) = p.locate(any).unwrap();
    sheet.columns.iter().map(|c| (c.short_name.clone(), c.designation)).collect()
}

#[test]
fn set_and_clear_the_interval_as_colint() {
    let (mut p, c) = sheet();
    p.set_sampling_interval(&[c[1]], 0.0, 0.5, "Time", "s").unwrap();
    p.set_sampling_interval(&[c[2]], 2.0, 3.0, "", "").unwrap();
    let b = p.column(c[1]).unwrap().sampling.clone().unwrap();
    assert_eq!((b.x0, b.inc, b.long_name.as_str(), b.units.as_str()), (0.0, 0.5, "Time", "s"));
    assert_eq!(b.values(5), [0.0, 0.5, 1.0, 1.5, 2.0], "Origin's xof(col(B))");
    assert_eq!(b.title(), "Time (s)");
    // Initial value and increment of 1 clear it.
    p.set_sampling_interval(&[c[2]], 1.0, 1.0, "", "").unwrap();
    assert_eq!(p.column(c[2]).unwrap().sampling, None);
    assert_eq!(p.set_sampling_interval(&[c[2]], f64::NAN, 1.0, "", ""), Err(SamplingError::NotFinite));
}

#[test]
fn show_x_column_inserts_left_of_the_column_as_colshowx() {
    let (mut p, c) = sheet();
    p.set_sampling_interval(&[c[1]], 0.0, 0.5, "Time", "s").unwrap();
    p.set_sampling_interval(&[c[3]], 0.0, 0.25, "", "").unwrap();
    // Without clearing: a Disregard column E before B, B keeps its interval.
    let e = p.show_x_column(c[1], false).unwrap();
    // With clearing: an X column F before D, D's interval cleared.
    let f = p.show_x_column(c[3], true).unwrap();
    use Designation::*;
    let want = [("A", X), ("E", Disregard), ("B", Y), ("C", Y), ("F", X), ("D", Y)];
    assert_eq!(names(&p, c[0]), want.map(|(n, d)| (n.to_string(), d)));
    let ec = p.column(e).unwrap();
    assert_eq!((ec.long_name.as_str(), ec.units.as_str()), ("Time", "s"));
    assert_eq!(p.numeric(e).unwrap(), [0.0, 0.5, 1.0, 1.5, 2.0]);
    assert_eq!(p.numeric(f).unwrap(), [0.0, 0.25, 0.5, 0.75, 1.0]);
    assert!(p.column(c[1]).unwrap().sampling.is_some());
    assert_eq!(p.column(c[3]).unwrap().sampling, None);
    // Showing it again inserts another copy.
    p.show_x_column(c[1], false).unwrap();
    assert_eq!(names(&p, c[0]).len(), 7);
}

#[test]
fn show_x_column_needs_an_interval_and_takes_the_column_length() {
    let mut p = Project::new();
    let book = p.add_workbook("Book1");
    let _a = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0, 5.0])).unwrap();
    let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![10.0, 20.0, 30.0])).unwrap();
    let c = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![5.0, 4.0, 3.0, 2.0, 1.0])).unwrap();
    assert_eq!(p.show_x_column(c, false), Err(SamplingError::NoInterval("C".into())));
    p.set_sampling_interval(&[b], 100.0, -2.0, "", "").unwrap();
    let x = p.show_x_column(b, true).unwrap();
    assert_eq!(p.numeric(x).unwrap(), [100.0, 98.0, 96.0], "as many rows as the column");
    // After clearing, the column plots against the new X column (Origin's axis went 95 to 101).
    let g = p.plot_columns(Template::Scatter, &[b]).unwrap();
    assert_eq!(p.graph(g).unwrap().layers[0].plots[0].x, Some(x));
}

#[test]
fn a_plot_of_a_column_with_an_interval_has_no_x_column() {
    let (mut p, c) = sheet();
    p.set_sampling_interval(&[c[1]], 0.0, 0.5, "Time", "s").unwrap();
    let g = p.plot_columns(Template::Scatter, &[c[1], c[2]]).unwrap();
    let plots = &p.graph(g).unwrap().layers[0].plots;
    assert_eq!(plots[0].x, None, "the interval, not column A");
    assert_eq!(plots[1].x, Some(c[0]));
}

#[test]
fn the_interval_is_part_of_the_column_description() {
    let (mut p, c) = sheet();
    p.set_sampling_interval(&[c[1]], 0.0, 0.5, "Time", "s").unwrap();
    let col = p.column(c[1]).unwrap();
    let json = serde_json::to_string(col).unwrap();
    assert!(json.contains("\"sampling\""), "{json}");
    let back: Column = serde_json::from_str(&json).unwrap();
    assert_eq!(back.sampling, col.sampling);
    let plain = serde_json::to_string(p.column(c[2]).unwrap()).unwrap();
    assert!(!plain.contains("sampling"), "{plain}");
}
