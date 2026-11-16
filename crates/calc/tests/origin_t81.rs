//! Origin 2016 runs of the T81 tools: partial correlation (`pcorrcoef`).
//! The data and printed numbers of the runs are in `fixtures/origin_t81_data.rs` (generated from
//! `tools/origin-ref/stats6/t81-origin.compact.json`); the means comparison, descriptive, Levene and power
//! values of the balanced three-way run are copied from the report sheet cell by cell (scratch dump
//! `a3-bal.raw.txt`, local only). Origin prints 5 decimals or 6 significant digits, so the tolerances are
//! half a unit of the last printed digit plus a little.

#[path = "fixtures/origin_t81_data.rs"]
mod data;

use sheaf_calc::partial_correlation;

fn close(got: f64, want: f64, rel: f64, what: &str) {
    if want.is_nan() {
        assert!(got.is_nan(), "{what}: got {got}, want NaN");
        return;
    }
    assert!((got - want).abs() <= rel * want.abs() + 6e-6, "{what}: got {got}, want {want}");
}

#[test]
fn partial_correlation_reproduces_the_origin_runs() {
    for run in data::pc_runs() {
        let vars: Vec<&[f64]> = run.vars.iter().map(|&i| run.columns[i].as_slice()).collect();
        let controls: Vec<&[f64]> = run.controls.iter().map(|&i| run.columns[i].as_slice()).collect();
        let r = partial_correlation(&vars, &controls, run.listwise, run.one_tailed).unwrap();
        let m = vars.len();
        for i in 0..m {
            for j in 0..m {
                let what = format!("run {} ({i},{j})", run.name);
                close(r.r(i, j), run.r[i * m + j], 0.0, &format!("{what} r"));
                close(r.p(i, j), run.p[i * m + j], 2e-4, &format!("{what} p"));
                assert_eq!(r.df(i, j), run.df[i * m + j], "{what} df");
            }
        }
    }
}

#[test]
fn partial_correlation_refuses_what_origin_refuses() {
    let runs = data::pc_runs();
    let c = &runs[0].columns;
    let (x, y, z) = (c[0].as_slice(), c[1].as_slice(), c[2].as_slice());
    assert!(partial_correlation(&[x, y], &[], false, false).is_err());
    assert!(partial_correlation(&[x], &[z], false, false).is_err());
    assert!(partial_correlation(&[x, y], &[&[1.0; 20]], false, false).is_err());
    assert!(partial_correlation(&[x, y], &[z, z], false, false).is_err());
    assert!(partial_correlation(&[x, y], &[x], false, false).is_err());
    let y2: Vec<f64> = x.iter().map(|v| 2.0 * v + 1.0).collect();
    assert!(partial_correlation(&[x, &y2], &[z], false, false).is_err());
}
