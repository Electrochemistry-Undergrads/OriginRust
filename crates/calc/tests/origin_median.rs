//! The median filter against Origin 2016's median of a curve: `smooth method:=pf percent:=50`
//! (Origin has no separate median filter for curves; analysis-tools.md 12.3).
//!
//! Reference script `re-workspace/scratch/t23m/m1.lines.txt` (local only), run with
//! `tools/origin-ref/run-labtalk-lines.ps1`; its sheet is `tests/fixtures/origin_median.tsv`.

use sheaf_calc::{MedianEdge, median_filter};

fn origin() -> Vec<Vec<f64>> {
    let rows: Vec<Vec<f64>> = include_str!("fixtures/origin_median.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split('\t').map(|v| v.parse().unwrap()).collect())
        .collect();
    assert_eq!(rows.len(), 40);
    (0..rows[0].len()).map(|c| rows.iter().map(|r| r[c]).collect()).collect()
}

#[test]
fn the_median_filter_gives_origins_percentile_filter_at_50_percent() {
    let cols = origin();
    let y = &cols[1];
    // Columns C to J of the probe: (npts, boundary).
    let cases = [
        (5, MedianEdge::Shrink),
        (5, MedianEdge::Reflect),
        (5, MedianEdge::Repeat),
        (4, MedianEdge::Shrink),
        (7, MedianEdge::Shrink),
        (4, MedianEdge::Reflect),
        (1, MedianEdge::Shrink),
        (8, MedianEdge::Repeat),
    ];
    for (k, (npts, edge)) in cases.into_iter().enumerate() {
        let got = median_filter(y, npts, edge).unwrap();
        // A median picks one of the values, so the result is exact.
        assert_eq!(got, cols[k + 2], "npts {npts}, {edge:?}");
    }
}

#[test]
fn smooth_percentile_at_50_percent_gives_origins_numbers_too() {
    use sheaf_calc::{Boundary, SmoothMethod, SmoothOptions, smooth};
    let cols = origin();
    let cases = [
        (5, Boundary::None, 2),
        (5, Boundary::Reflect, 3),
        (5, Boundary::Repeat, 4),
        (4, Boundary::None, 5),
        (7, Boundary::None, 6),
        (4, Boundary::Reflect, 7),
        (1, Boundary::None, 8),
        (8, Boundary::Repeat, 9),
    ];
    for (npts, boundary, col) in cases {
        let opts = SmoothOptions { method: SmoothMethod::Percentile { percent: 50.0 }, npts, boundary };
        assert_eq!(smooth(&cols[1], &opts), cols[col], "npts {npts}, {boundary:?}");
    }
}
