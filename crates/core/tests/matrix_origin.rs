//! Matrix operations against numbers that Origin 2016 printed for the same inputs.
//! `tests/data/origin_matrix.json` comes from `tools/origin-ref/matrix-gen-ops.py` and
//! `matrix-gen-conv.py` (run through `run-labtalk-lines.ps1`, turned into JSON by
//! `matrix-make-fixture.py`). Origin printed 15 significant digits; every value is compared
//! to a relative 1e-12.

use serde_json::Value;
use sheaf_core::column::{Column, ColumnData, Designation};
use sheaf_core::matrix::{FlipDirection, MatrixSheet, Rotation};
use sheaf_core::matrix_convert::{
    ColumnParam, LabelSource, M2wMethod, W2mOptions, XyLayout, XyzOrder, matrix_to_worksheet, worksheet_to_matrix,
    xyz_to_matrix,
};
use sheaf_core::{ColumnId, matrix_convert};

fn fixture() -> Value {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/origin_matrix.json"))
        .expect("fixture file");
    serde_json::from_str(&text).expect("fixture json")
}

fn nums(v: &Value) -> Vec<Option<f64>> {
    v.as_array().expect("array").iter().map(Value::as_f64).collect()
}

fn same(a: f64, b: Option<f64>, what: &str) {
    match b {
        None => assert!(a.is_nan(), "{what}: expected missing, got {a}"),
        Some(b) => {
            let tol = 1e-12 * b.abs().max(1e-3);
            assert!((a - b).abs() <= tol, "{what}: got {a}, Origin has {b}");
        }
    }
}

/// Compares a matrix with the fixture case `name` (data, X coordinates, Y coordinates).
fn check(m: &MatrixSheet, name: &str) {
    let fx = fixture();
    let case = &fx[name];
    let rows = case["rows"].as_array().unwrap_or_else(|| panic!("case {name} has no rows"));
    assert_eq!(m.rows, rows.len(), "{name}: rows");
    for (r, row) in rows.iter().enumerate() {
        let row = nums(row);
        assert_eq!(m.cols, row.len(), "{name}: cols");
        for (c, v) in row.iter().enumerate() {
            same(m.value(r, c), *v, &format!("{name}[{r},{c}]"));
        }
    }
    for (c, v) in nums(&case["x"]).iter().enumerate() {
        same(m.x_at(c), *v, &format!("{name} x[{c}]"));
    }
    for (r, v) in nums(&case["y"]).iter().enumerate() {
        same(m.y_at(r), *v, &format!("{name} y[{r}]"));
    }
    assert_eq!(m.cols, case["x"].as_array().unwrap().len(), "{name}: number of X coordinates");
    assert_eq!(m.rows, case["y"].as_array().unwrap().len(), "{name}: number of Y coordinates");
}

/// The matrix the generator builds with `mdim` and `msetvalue`.
fn base(rows: usize, cols: usize, formula: &str, x: (f64, f64), y: (f64, f64)) -> MatrixSheet {
    let mut m = MatrixSheet::new("MSheet1", 1, rows, cols);
    (m.x_from, m.x_to, m.y_from, m.y_to) = (x.0, x.1, y.0, y.1);
    m.set_values(formula).unwrap();
    m
}

fn std_base() -> MatrixSheet {
    base(3, 4, "i*10+j", (10.0, 40.0), (1.0, 3.0))
}

#[test]
fn new_matrix_defaults_are_those_of_origin() {
    let m = MatrixSheet::new("MSheet1", 1, 32, 32);
    check(&m, "default-new");
}

#[test]
fn setting_the_size_keeps_the_ranges() {
    let mut m = MatrixSheet::new("MSheet1", 1, 32, 32);
    m.set_dimensions(2, 5, None).unwrap();
    check(&m, "mdim-only-size");
    let mut m = MatrixSheet::new("MSheet1", 1, 32, 32);
    m.set_dimensions(3, 3, Some((-1.0, 1.0, 5.0, -5.0))).unwrap();
    check(&m, "mdim-range");
    let mut m = MatrixSheet::new("MSheet1", 1, 32, 32);
    m.set_dimensions(3, 1, Some((2.0, 7.0, 0.0, 1.0))).unwrap();
    check(&m, "mdim-1col");
}

#[test]
fn set_values_formulas() {
    for (name, formula) in [
        ("sv-ij", "i*100+j"),
        ("sv-xy", "x*1000+y"),
        ("sv-expr", "sin(x/10)*cos(y)+ln(i+j)"),
        ("sv-iif", "(i>1 && j<3) ? 100+i : -j"),
        ("sv-missing-const", "(i==2 && j==3) ? na() : i*10+j"),
        ("sv-missing-div", "(i==2 && j==3) ? 0/0 : i*10+j"),
        ("sv-pow-int", "i^2 - j^0.5 + mod(i*j,3)"),
    ] {
        check(&base(3, 4, formula, (10.0, 40.0), (1.0, 3.0)), name);
    }
}

#[test]
fn a_formula_error_leaves_the_matrix_alone() {
    let mut m = std_base();
    let before = m.clone();
    assert!(m.set_values("i + nosuchvariable").is_err());
    assert!(m.set_values("i +").is_err());
    assert_eq!(m, before);
}

#[test]
fn structure_operations() {
    check(&std_base().transpose(), "transpose");
    check(&std_base().flip(FlipDirection::Horizontal, false), "flip-h");
    check(&std_base().flip(FlipDirection::Vertical, false), "flip-v");
    check(&std_base().flip(FlipDirection::Horizontal, true), "flip-h-coord");
    check(&std_base().flip(FlipDirection::Vertical, true), "flip-v-coord");
    check(&std_base().rotate90(Rotation::Cw90), "rot-cw90");
    check(&std_base().rotate90(Rotation::Ccw90), "rot-ccw90");
    check(&std_base().rotate90(Rotation::Half), "rot-180");
}

#[test]
fn expand_and_shrink() {
    check(&std_base().expand(2, 2).unwrap(), "expand-default");
    check(&std_base().expand(3, 2).unwrap(), "expand-3x2");
    check(&std_base().expand(1, 3).unwrap(), "expand-1x3");
    let six = || base(4, 6, "i*10+j", (10.0, 60.0), (1.0, 4.0));
    check(&six().shrink(2, 2).unwrap(), "shrink-default");
    check(&six().shrink(2, 1).unwrap(), "shrink-2x1");
    check(&base(3, 5, "i*10+j", (10.0, 50.0), (1.0, 3.0)).shrink(2, 2).unwrap(), "shrink-uneven");
    check(&base(6, 6, "i*i+j", (0.0, 5.0), (0.0, 5.0)).shrink(3, 3).unwrap(), "shrink-3x3");
}

#[test]
fn statistics_skip_missing_cells() {
    let mut m = base(3, 4, "i*10+j+x/7", (10.0, 40.0), (1.0, 3.0));
    m.set_value(1, 2, f64::NAN).unwrap();
    m.set_value(2, 0, f64::NAN).unwrap();
    let s = m.stats();
    let fx = fixture();
    let o = &fx["mstats"]["stats"];
    same(s.mean, o["mean"].as_f64(), "mean");
    same(s.sd, o["sd"].as_f64(), "sd");
    same(s.min, o["min"].as_f64(), "min");
    same(s.max, o["max"].as_f64(), "max");
    same(s.sum, o["sum"].as_f64(), "sum");
    assert_eq!(s.n as f64, o["n"].as_f64().unwrap());
    assert_eq!(s.missing as f64, o["missing"].as_f64().unwrap());
}

fn col(id: u64, name: &str, values: &[Option<f64>]) -> Column {
    let v = values.iter().map(|v| v.unwrap_or(f64::NAN)).collect();
    Column::new(ColumnId(id), name, Designation::Y, ColumnData::numeric(v))
}

fn cols(data: &[&[f64]]) -> Vec<Column> {
    data.iter()
        .enumerate()
        .map(|(k, d)| {
            col(k as u64 + 1, &format!("{}", (b'A' + k as u8) as char), &d.iter().map(|v| Some(*v)).collect::<Vec<_>>())
        })
        .collect()
}

fn convert(cs: &[Column], opts: &W2mOptions) -> Result<MatrixSheet, sheaf_core::matrix::MatrixError> {
    let refs: Vec<&Column> = cs.iter().collect();
    worksheet_to_matrix(&refs, opts, "MSheet1")
}

fn opts(layout: XyLayout, labels: LabelSource, first: bool) -> W2mOptions {
    W2mOptions { layout, labels, coords_in_first_column: first, ..W2mOptions::default() }
}

#[test]
fn worksheet_to_matrix_without_coordinates() {
    let d = cols(&[&[1.0, 2.0, 3.0, 4.0], &[5.0, 6.0, 7.0, 8.0], &[9.0, 10.0, 11.0, 12.0]]);
    check(&convert(&d, &W2mOptions::default()).unwrap(), "w2m-noxy");
    check(&convert(&d, &W2mOptions::default()).unwrap(), "w2m-default");

    let nan = f64::NAN;
    let gap = cols(&[&[1.0, 2.0, nan, 4.0], &[5.0, 6.0, nan, 8.0], &[9.0, 10.0, nan, 12.0]]);
    check(&convert(&gap, &W2mOptions { trim: false, ..W2mOptions::default() }).unwrap(), "w2m-noxy-trim0-missing");
    check(&convert(&gap, &W2mOptions::default()).unwrap(), "w2m-noxy-trim1-missing");
    let gap_col = cols(&[&[1.0, 2.0, 3.0, 4.0], &[nan, nan, nan, nan], &[9.0, 10.0, 11.0, 12.0]]);
    check(&convert(&gap_col, &W2mOptions::default()).unwrap(), "w2m-noxy-trim1-missingcol");
    let ragged = cols(&[&[1.0, 2.0, 3.0, 4.0], &[5.0, 6.0, 7.0], &[9.0, 10.0, 11.0, 12.0]]);
    check(&convert(&ragged, &W2mOptions::default()).unwrap(), "w2m-noxy-ragged");
}

#[test]
fn worksheet_to_matrix_with_coordinates() {
    let x = XyLayout::XAcrossColumns;
    let row1 = LabelSource::FirstRow;
    let conv_x =
        cols(&[&[0.0, 100.0, 200.0, 300.0], &[10.0, 1.0, 2.0, 3.0], &[20.0, 4.0, 5.0, 6.0], &[30.0, 7.0, 8.0, 9.0]]);
    check(&convert(&conv_x, &opts(x, row1.clone(), true)).unwrap(), "w2m-xcol-row1-ycol");
    let no_y = cols(&[&[0.0, 1.0, 2.0], &[10.0, 11.0, 12.0], &[20.0, 21.0, 22.0], &[30.0, 31.0, 32.0]]);
    check(&convert(&no_y, &opts(x, row1.clone(), false)).unwrap(), "w2m-xcol-row1-noycol");

    let mut labelled = cols(&[&[100.0, 200.0, 300.0], &[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0], &[7.0, 8.0, 9.0]]);
    for (c, n) in labelled.iter_mut().zip(["", "10", "20", "30"]) {
        c.long_name = n.to_string();
    }
    let param = LabelSource::Parameter(ColumnParam::LongName);
    check(&convert(&labelled, &opts(x, param.clone(), true)).unwrap(), "w2m-xcol-param");
    let mut labelled = cols(&[&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0], &[7.0, 8.0, 9.0]]);
    for (c, n) in labelled.iter_mut().zip(["5", "10", "15"]) {
        c.long_name = n.to_string();
    }
    check(&convert(&labelled, &opts(x, param.clone(), false)).unwrap(), "w2m-xcol-param-noycol");

    let y = XyLayout::YAcrossColumns;
    let conv_y =
        cols(&[&[0.0, 10.0, 20.0, 30.0], &[100.0, 1.0, 2.0, 3.0], &[200.0, 4.0, 5.0, 6.0], &[300.0, 7.0, 8.0, 9.0]]);
    check(&convert(&conv_y, &opts(y, row1.clone(), true)).unwrap(), "w2m-ycol-row1-xcol");
    let mut labelled = cols(&[&[10.0, 20.0, 30.0], &[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0], &[7.0, 8.0, 9.0]]);
    for (c, n) in labelled.iter_mut().zip(["", "100", "200", "300"]) {
        c.long_name = n.to_string();
    }
    check(&convert(&labelled, &opts(y, param, true)).unwrap(), "w2m-ycol-param");
}

#[test]
fn uneven_coordinates_are_refused_beyond_the_tolerance() {
    let x = XyLayout::XAcrossColumns;
    let uneven =
        cols(&[&[0.0, 100.0, 200.0, 300.0], &[10.0, 1.0, 2.0, 3.0], &[20.0, 4.0, 5.0, 6.0], &[50.0, 7.0, 8.0, 9.0]]);
    let o = opts(x, LabelSource::FirstRow, true);
    assert!(convert(&uneven, &o).is_err(), "Origin refuses (w2m-xcol-uneven)");
    assert!(convert(&uneven, &W2mOptions { linear_fit: true, ..o.clone() }).is_err(), "w2m-xcol-uneven-lr");
    check(&convert(&uneven, &W2mOptions { tolerance_percent: 50.0, ..o.clone() }).unwrap(), "w2m-xcol-uneven-tol50");
    let desc =
        cols(&[&[0.0, 300.0, 200.0, 100.0], &[30.0, 1.0, 2.0, 3.0], &[20.0, 4.0, 5.0, 6.0], &[10.0, 7.0, 8.0, 9.0]]);
    check(&convert(&desc, &o).unwrap(), "w2m-xcol-descending");
    let near =
        cols(&[&[0.0, 100.0, 200.0, 300.0], &[10.0, 1.0, 2.0, 3.0], &[20.0, 4.0, 5.0, 6.0], &[30.5, 7.0, 8.0, 9.0]]);
    check(&convert(&near, &o).unwrap(), "w2m-xcol-tiny-uneven");
}

fn xyz_cols(order: &str) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let (xs, ys) = ([1.0, 2.0, 3.0], [10.0, 20.0, 30.0, 40.0]);
    let mut pts: Vec<(f64, f64)> = xs.iter().flat_map(|&x| ys.iter().map(move |&y| (x, y))).collect();
    match order {
        "y-major" => pts = ys.iter().flat_map(|&y| xs.iter().map(move |&x| (x, y))).collect(),
        "shuffled" => {
            let idx = [5, 0, 11, 3, 7, 2, 9, 4, 10, 1, 6, 8];
            pts = idx.iter().map(|&i| pts[i]).collect();
        }
        "descending" => pts.reverse(),
        _ => {}
    }
    (
        pts.iter().map(|p| p.0).collect(),
        pts.iter().map(|p| p.1).collect(),
        pts.iter().map(|p| p.0 * 100.0 + p.1).collect(),
    )
}

#[test]
fn regular_xyz_gridding() {
    for (name, order) in [
        ("xyz-x-major", "x-major"),
        ("xyz-y-major", "y-major"),
        ("xyz-shuffled", "shuffled"),
        ("xyz-descending", "descending"),
    ] {
        let (x, y, z) = xyz_cols(order);
        check(&xyz_to_matrix(&x, &y, &z, "MSheet1").unwrap(), name);
    }
    let (mut x, mut y, mut z) = xyz_cols("x-major");
    // The point (2, 20) is missing from the grid.
    x.remove(5);
    y.remove(5);
    z.remove(5);
    check(&xyz_to_matrix(&x, &y, &z, "MSheet1").unwrap(), "xyz-hole");
    // x in {0, 1, 3}, y in {0, 2}: the grid keeps its gap at x = 2.
    let (mut ux, mut uy, mut uz) = (vec![], vec![], vec![]);
    for x in [0.0, 1.0, 3.0] {
        for y in [0.0, 2.0] {
            ux.push(x);
            uy.push(y);
            uz.push(x * 100.0 + y);
        }
    }
    check(&xyz_to_matrix(&ux, &uy, &uz, "MSheet1").unwrap(), "xyz-uneven");
    // A repeated point is averaged with the first.
    let (mut x, mut y, mut z) = xyz_cols("x-major");
    x.push(x[0]);
    y.push(y[0]);
    z.push(-5.0);
    check(&xyz_to_matrix(&x, &y, &z, "MSheet1").unwrap(), "xyz-duplicate");
}

#[test]
fn scattered_points_are_not_a_regular_grid() {
    let x = [0.1, 0.9, 0.5, 0.2, 0.8, 0.4, 0.65];
    let y = [0.2, 0.1, 0.5, 0.9, 0.8, 0.15, 0.35];
    let z = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    assert!(xyz_to_matrix(&x, &y, &z, "MSheet1").is_err());
}

fn worksheet_rows(ws: &sheaf_core::Worksheet) -> Vec<Vec<f64>> {
    let n = ws.row_count();
    (0..n)
        .map(|r| ws.columns.iter().map(|c| c.numeric().unwrap().get(r).copied().unwrap_or(f64::NAN)).collect())
        .collect()
}

fn check_sheet(ws: &sheaf_core::Worksheet, name: &str) {
    let fx = fixture();
    let rows = fx[name]["rows"].as_array().unwrap_or_else(|| panic!("case {name}"));
    let got = worksheet_rows(ws);
    assert_eq!(got.len(), rows.len(), "{name}: rows");
    for (r, row) in rows.iter().enumerate() {
        let row = nums(row);
        assert_eq!(got[r].len(), row.len(), "{name}: columns");
        for (c, v) in row.iter().enumerate() {
            same(got[r][c], *v, &format!("{name}[{r},{c}]"));
        }
    }
}

fn to_ws(m: &MatrixSheet, method: &M2wMethod) -> sheaf_core::Worksheet {
    let mut next = 0;
    matrix_to_worksheet(m, method, "Sheet1", || {
        next += 1;
        ColumnId(next)
    })
}

#[test]
fn matrix_to_worksheet_direct_and_xyz() {
    let m = std_base();
    let direct = |layout, labels, first| M2wMethod::Direct { layout, labels, coords_in_first_column: first };
    check_sheet(&to_ws(&m, &M2wMethod::default()), "m2w-direct-noxy");
    check_sheet(
        &to_ws(&m, &direct(XyLayout::XAcrossColumns, LabelSource::FirstRow, true)),
        "m2w-direct-xcol-row1-ycol",
    );
    check_sheet(&to_ws(&m, &direct(XyLayout::XAcrossColumns, LabelSource::None, true)), "m2w-direct-xcol-none");
    check_sheet(
        &to_ws(&m, &direct(XyLayout::YAcrossColumns, LabelSource::FirstRow, true)),
        "m2w-direct-ycol-row1-xcol",
    );
    let xyz = |order, skip| M2wMethod::Xyz { order, skip_missing: skip };
    check_sheet(&to_ws(&m, &xyz(XyzOrder::XConstFirst, false)), "m2w-xyz-xconst");
    check_sheet(&to_ws(&m, &xyz(XyzOrder::YConstFirst, false)), "m2w-xyz-yconst");
    let mut gap = std_base();
    gap.set_value(1, 2, f64::NAN).unwrap();
    check_sheet(&to_ws(&gap, &xyz(XyzOrder::XConstFirst, false)), "m2w-xyz-missing-notrim");
    check_sheet(&to_ws(&gap, &xyz(XyzOrder::XConstFirst, true)), "m2w-xyz-missing-trim");
    let _ = matrix_convert::XyzOrder::default();
}
