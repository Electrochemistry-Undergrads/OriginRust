//! Trace interpolation of a curve whose X is not monotonic (a loop such as a CV scan), as
//! Origin's Analysis > Mathematics > Trace Interpolation (`interp1trace`). See
//! `docs/spec/analysis-tools.md` 16.

use crate::interp::{InterpMethod, InterpOptions, SplineBoundary, interp1};

/// Interpolation method of [`trace_interp`] (Origin's `method`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TraceMethod {
    #[default]
    Linear,
    /// Natural cubic spline of X and of Y against the path length.
    CubicSpline,
    /// Interpolating cubic spline with not-a-knot ends (needs four points after merging).
    CubicBSpline,
}

/// Relative tolerance (to the range of the column) under which neighbouring values count as equal.
/// Measured: values 5e-8 apart in a range of 9 merge, 1e-7 apart do not; 5e-6 apart in a range of
/// 504 merge, 1e-5 apart do not.
const TOLERANCE: f64 = 1e-8;

/// One merging pass: runs of consecutive points whose value in `column` differs from the previous
/// point by at most `TOLERANCE` times the range of that column become one point at their mean.
fn merge_runs(pts: &[(f64, f64)], column: usize) -> Vec<(f64, f64)> {
    let get = |p: &(f64, f64)| if column == 0 { p.0 } else { p.1 };
    let lo = pts.iter().map(get).fold(f64::INFINITY, f64::min);
    let hi = pts.iter().map(get).fold(f64::NEG_INFINITY, f64::max);
    let tol = TOLERANCE * (hi - lo);
    let mut out = Vec::with_capacity(pts.len());
    let mut i = 0;
    while i < pts.len() {
        let mut j = i;
        while j + 1 < pts.len() && (get(&pts[j + 1]) - get(&pts[j])).abs() <= tol {
            j += 1;
        }
        let n = (j - i + 1) as f64;
        let (sx, sy) = pts[i..=j].iter().fold((0.0, 0.0), |a, p| (a.0 + p.0, a.1 + p.1));
        out.push(if j == i { pts[i] } else { (sx / n, sy / n) });
        i = j + 1;
    }
    out
}

/// The points Origin works on: rows with a missing X or Y dropped, then neighbouring points with
/// the same X merged (mean of X and Y), then neighbouring points with the same Y merged (so a
/// merge by Y can use the result of the merge by X, but not the other way round).
pub fn trace_points(x: &[f64], y: &[f64]) -> Vec<(f64, f64)> {
    let pts: Vec<(f64, f64)> =
        x.iter().zip(y).filter(|(a, b)| a.is_finite() && b.is_finite()).map(|(a, b)| (*a, *b)).collect();
    if pts.is_empty() {
        return pts;
    }
    merge_runs(&merge_runs(&pts, 0), 1)
}

/// Resamples the curve `(x, y)` at `npts` points evenly spaced along its path.
///
/// The path parameter is the cumulative chord length of [`trace_points`]. X and Y are each
/// interpolated against it (linear, natural cubic spline, or not-a-knot cubic spline) at
/// `npts` values from 0 to the total length, so the first and last output points are the first
/// and last points of the path. Returns `None`, where Origin makes no output, for `npts < 2`, for
/// fewer than two points after merging, and for the B-spline with fewer than four.
pub fn trace_interp(x: &[f64], y: &[f64], method: TraceMethod, npts: usize) -> Option<(Vec<f64>, Vec<f64>)> {
    let pts = trace_points(x, y);
    let need = if method == TraceMethod::CubicBSpline { 4 } else { 2 };
    if npts < 2 || pts.len() < need {
        return None;
    }
    let mut s = Vec::with_capacity(pts.len());
    let mut acc = 0.0;
    s.push(0.0);
    for w in pts.windows(2) {
        acc += (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
        s.push(acc);
    }
    let total = acc;
    let at: Vec<f64> =
        (0..npts).map(|k| if k + 1 == npts { total } else { total * k as f64 / (npts - 1) as f64 }).collect();
    let opts = match method {
        TraceMethod::Linear => InterpOptions { method: InterpMethod::Linear, ..Default::default() },
        TraceMethod::CubicSpline => {
            InterpOptions { method: InterpMethod::CubicSpline, boundary: SplineBoundary::Natural, ..Default::default() }
        }
        TraceMethod::CubicBSpline => InterpOptions {
            method: InterpMethod::CubicSpline,
            boundary: SplineBoundary::NotAKnot,
            ..Default::default()
        },
    };
    let px: Vec<f64> = pts.iter().map(|p| p.0).collect();
    let py: Vec<f64> = pts.iter().map(|p| p.1).collect();
    Some((interp1(&s, &px, &at, &opts), interp1(&s, &py, &at, &opts)))
}

#[cfg(test)]
#[allow(clippy::excessive_precision)]
mod tests {
    use super::*;

    // Every fixture below is Origin's printed output of interp1trace (probe files in
    // analysis-tools.md 16).

    const P1_LINEAR: [(f64, f64); 10] = [
        (1.0, 0.0),
        (1.6666666666666665, 0.3333333333333333),
        (2.333333333333333, 0.6666666666666666),
        (3.0, 1.0),
        (2.3333333333333335, 1.3333333333333333),
        (1.666666666666667, 1.6666666666666665),
        (1.0, 2.0),
        (0.3333333333333338, 1.666666666666667),
        (-0.33333333333333304, 1.3333333333333335),
        (-1.0, 1.0),
    ];

    const P1_SPLINE: [(f64, f64); 10] = [
        (1.0, 0.0),
        (1.982716049382716, 0.2938271604938271),
        (2.728395061728395, 0.6172839506172839),
        (3.0, 1.0),
        (2.6493827160493826, 1.4419753086419753),
        (1.883950617283951, 1.8246913580246913),
        (1.0, 2.0),
        (0.23456790123456844, 1.8641975308641978),
        (-0.4123456790123454, 1.4913580246913583),
        (-1.0, 1.0),
    ];

    const P1_BSPLINE: [(f64, f64); 10] = [
        (0.9999999999999996, 6.648807513813266e-17),
        (2.358024691358023, 0.2098765432098764),
        (2.975308641975308, 0.567901234567901),
        (2.999999999999999, 0.9999999999999996),
        (2.580246913580246, 1.4320987654320985),
        (1.8641975308641974, 1.7901234567901232),
        (0.9999999999999994, 1.9999999999999998),
        (0.13580246913580296, 1.9876543209876543),
        (-0.5802469135802469, 1.6790123456790125),
        (-1.0, 1.0),
    ];

    const P4_LINEAR: [(f64, f64); 9] = [
        (0.5, 0.0),
        (1.1795126371966898, 0.2718050548786759),
        (1.8590252743933795, 0.5436101097573518),
        (2.538537911590069, 0.8154151646360276),
        (2.77720397744448, 1.0742653408518401),
        (2.08290298308336, 1.30569900563888),
        (1.38860198872224, 1.5371326704259198),
        (0.6943009943611198, 1.7685663352129601),
        (0.0, 2.0),
    ];

    const R1_SPLINE: [(f64, f64); 30] = [
        (0.0, 0.0),
        (0.7265657900316244, 0.14036843939448707),
        (1.4663586837443736, 0.3269032161277903),
        (2.2326057848193726, 0.6057706675387258),
        (3.0385314942795327, 1.023135175007181),
        (3.866536190176555, 1.6028532592112759),
        (4.591738721586474, 2.291140004910882),
        (5.0659328009735365, 3.0173297055794297),
        (5.140912140801989, 3.710756654690351),
        (4.693399782650787, 4.306978277830535),
        (4.081162779445929, 4.861635474202501),
        (4.009783086673235, 5.518900929113951),
        (4.49406620086059, 6.2643765524433315),
        (5.205289758708606, 7.004653248352683),
        (5.812443006071848, 7.645795593787638),
        (5.986162618416266, 8.094214375370248),
        (5.572915950849328, 8.293271447179475),
        (4.748693812503649, 8.255578485925614),
        (3.725728946966073, 8.00136346240096),
        (2.7162540978234477, 7.550854347397808),
        (1.9324276391011117, 6.924290221334159),
        (1.5069353988163465, 6.1537820947911985),
        (1.341956765966995, 5.305874961953991),
        (1.2981173109687032, 4.453321294394465),
        (1.2360426042371144, 3.6688735636845506),
        (1.0163582161878728, 3.0252842413961667),
        (0.5309181585452283, 2.5768636047283797),
        (-0.1847916037277148, 2.293098984592016),
        (-1.054016914691218, 2.1191064667279367),
        (-2.0, 2.0),
    ];

    const R1_BSPLINE: [(f64, f64); 30] = [
        (-6.174815921070124e-17, 2.744362631586722e-17),
        (0.4422698552728315, -0.06601292072080814),
        (1.1745191335505178, 0.1149704833957141),
        (2.0806101432853876, 0.49532110719737626),
        (3.0444051929297697, 1.0274098455319882),
        (3.949766590935991, 1.6636075932473593),
        (4.680556645756382, 2.356285245191298),
        (5.12063766584327, 3.057813696211616),
        (5.153871959648983, 3.7205638411561215),
        (4.687490362319329, 4.302092190662371),
        (4.079649996192876, 4.8600184945079095),
        (4.011602497545954, 5.523624850619949),
        (4.493145346596395, 6.272915258441458),
        (5.200420679237242, 7.013718212976259),
        (5.807348005702957, 7.651368093422124),
        (5.989450101783858, 8.091563500893262),
        (5.593366779399309, 8.278876555994124),
        (4.786427487241683, 8.230732060321234),
        (3.771232039903287, 7.972567362247609),
        (2.7503802519764324, 7.52981981014626),
        (1.9264067686594963, 6.9279325525564674),
        (1.4322051406933742, 6.198546891814051),
        (1.198678149689928, 5.391281588281552),
        (1.1203153307965306, 4.558996231656212),
        (1.0916062191605538, 3.7545504116352753),
        (1.0070403499293703, 3.0308037179159752),
        (0.7611072582503504, 2.440615740195557),
        (0.2482964792708695, 2.036846068171262),
        (-0.6369024518617051, 1.8723542915403288),
        (-2.0000000000000004, 2.0000000000000004),
    ];

    const S1_Y_MERGE: [(f64, f64); 9] = [
        (0.5, 0.0),
        (0.5, 0.125),
        (0.5, 0.25),
        (0.5, 0.375),
        (0.5, 0.5),
        (0.5, 0.625),
        (0.5, 0.75),
        (0.5, 0.875),
        (0.5, 1.0),
    ];

    const NAN_X: [(f64, f64); 7] = [
        (1.0, 0.0),
        (2.015556364802257, 0.5077781824011285),
        (2.966999966873137, 1.0110000110422876),
        (1.8898397785298848, 1.370053407156705),
        (0.8126795901866322, 1.7291068032711228),
        (-0.19713219873600318, 1.802867801263997),
        (-1.0, 1.0),
    ];

    const NAN_Y: [(f64, f64); 7] = [
        (1.0, 0.0),
        (2.015556364802257, 0.5077781824011285),
        (2.9754032275561966, 1.0245967724438034),
        (2.1725354262922, 1.8274645737078),
        (1.1543203766865053, 1.7181067922288351),
        (0.07716018834325222, 1.3590533961144176),
        (-1.0000000000000004, 1.0),
    ];

    const S3_CASCADE: [(f64, f64); 7] = [
        (0.5, 0.0),
        (0.5, 1.2820970672612086),
        (0.5, 2.5641941345224173),
        (0.5, 3.846291201783626),
        (0.6192055151912343, 5.047682206076494),
        (1.8096027575956168, 5.523841103038246),
        (3.0, 6.0),
    ];

    const S2_X_MERGE: [(f64, f64); 9] = [
        (0.0, 0.0),
        (0.44368871959954903, 0.44368871959954903),
        (0.8873774391990981, 0.8873774391990981),
        (1.25970966215454, 1.38956449323181),
        (1.607767729723632, 1.911651594585448),
        (1.955825797292724, 2.433738695939086),
        (2.3038838648618163, 2.955825797292724),
        (2.651941932430908, 3.4779128986463617),
        (3.0, 4.0),
    ];

    fn check(got: Option<(Vec<f64>, Vec<f64>)>, want: &[(f64, f64)]) {
        let (gx, gy) = got.expect("a result");
        assert_eq!(gx.len(), want.len());
        for (k, (wx, wy)) in want.iter().enumerate() {
            assert!((gx[k] - wx).abs() < 1e-9, "x[{k}]: {} vs {wx}", gx[k]);
            assert!((gy[k] - wy).abs() < 1e-9, "y[{k}]: {} vs {wy}", gy[k]);
        }
    }

    const HEX_X: [f64; 6] = [0.0, 2.0, 3.0, 2.0, 0.0, -1.0];
    const HEX_Y: [f64; 6] = [0.0, 0.0, 1.0, 2.0, 2.0, 1.0];

    #[test]
    fn matches_origin_for_the_three_methods() {
        // equal Y neighbours (0,0)-(2,0) and (2,2)-(0,2) merge, leaving the diamond (1,0) (3,1) (1,2) (-1,1)
        check(trace_interp(&HEX_X, &HEX_Y, TraceMethod::Linear, 10), &P1_LINEAR);
        check(trace_interp(&HEX_X, &HEX_Y, TraceMethod::CubicSpline, 10), &P1_SPLINE);
        check(trace_interp(&HEX_X, &HEX_Y, TraceMethod::CubicBSpline, 10), &P1_BSPLINE);
    }

    #[test]
    fn a_path_without_equal_neighbours_is_followed_exactly() {
        let x = [0.0, 3.0, 5.0, 4.0, 6.0, 2.0, 1.0, -2.0];
        let y = [0.0, 1.0, 4.0, 5.0, 8.0, 7.0, 3.0, 2.0];
        check(trace_interp(&x, &y, TraceMethod::CubicSpline, 30), &R1_SPLINE);
        check(trace_interp(&x, &y, TraceMethod::CubicBSpline, 30), &R1_BSPLINE);
        let (lx, ly) = trace_interp(&x, &y, TraceMethod::Linear, 101).unwrap();
        assert_eq!((lx[0], ly[0]), (0.0, 0.0));
        assert!((lx[100] + 2.0).abs() < 1e-12 && (ly[100] - 2.0).abs() < 1e-12);
        // even steps of one hundredth of the path length 23.196...
        let d = (lx[1] - lx[0]).hypot(ly[1] - ly[0]);
        assert!((d - 0.23196).abs() < 1e-4, "{d}");
    }

    #[test]
    fn x_merges_before_y_and_the_mean_is_taken() {
        // (3,0) and (3,2) have the same X and merge to (3,1) first; then (0,0) and (1,0) share Y
        let x = [0.0, 1.0, 3.0, 3.0, 0.0];
        let y = [0.0, 0.0, 0.0, 2.0, 2.0];
        assert_eq!(trace_points(&x, &y), vec![(0.5, 0.0), (3.0, 1.0), (0.0, 2.0)]);
        check(trace_interp(&x, &y, TraceMethod::Linear, 9), &P4_LINEAR);
        // only neighbours merge: a repeated X elsewhere stays
        check(trace_interp(&[0.0, 1.0, 0.0, 1.0], &[0.0, 0.0, 1.0, 1.0], TraceMethod::Linear, 9), &S1_Y_MERGE);
        check(
            trace_interp(&[0.0, 1.0, 2.0, 2.0, 3.0], &[0.0, 1.0, 2.0, 3.0, 4.0], TraceMethod::Linear, 9),
            &S2_X_MERGE,
        );
        // a merge by Y can make a new equal X, which is not merged again
        let p = trace_points(&[0.0, 1.0, 0.5, 3.0], &[0.0, 0.0, 5.0, 6.0]);
        assert_eq!(p, vec![(0.5, 0.0), (0.5, 5.0), (3.0, 6.0)]);
        check(trace_interp(&[0.0, 1.0, 0.5, 3.0], &[0.0, 0.0, 5.0, 6.0], TraceMethod::Linear, 7), &S3_CASCADE);
        // a merge by X can make an equal Y that the Y pass then merges
        let p = trace_points(&[0.0, 1.0, 2.0, 2.0], &[3.0, 1.0, 0.0, 2.0]);
        assert_eq!(p, vec![(0.0, 3.0), (1.5, 1.0)]);
    }

    #[test]
    fn near_equal_values_merge_within_a_relative_tolerance() {
        // 5e-8 apart in an X range of 9 merge, 1e-6 apart do not
        let near = |d: f64| trace_points(&[-5.0, 1.0, 1.0 + d, 4.0, -4.0], &[0.0, 1.0, 2.0, 4.0, 1.5]).len();
        assert_eq!(near(5e-8), 4);
        assert_eq!(near(1e-6), 5);
        // the same distance inside a much wider range
        let wide = |d: f64| trace_points(&[-500.0, 1.0, 1.0 + d, 4.0, -4.0], &[0.0, 1.0, 2.0, 4.0, 1.5]).len();
        assert_eq!(wide(5e-6), 4);
        assert_eq!(wide(1e-5), 5);
    }

    #[test]
    fn missing_rows_are_dropped_and_degenerate_input_gives_nothing() {
        let nan = f64::NAN;
        check(trace_interp(&[0.0, 2.0, 3.0, nan, 0.0, -1.0], &HEX_Y, TraceMethod::Linear, 7), &NAN_X);
        check(trace_interp(&HEX_X, &[0.0, 0.0, 1.0, 2.0, nan, 1.0], TraceMethod::Linear, 7), &NAN_Y);
        let x = [0.0, 1.0, 2.0, 3.0];
        assert!(trace_interp(&x, &[0.0; 4], TraceMethod::Linear, 7).is_none(), "one point is left");
        assert!(trace_interp(&x, &[0.0, 1.0, 0.0, 1.0], TraceMethod::Linear, 1).is_none());
        assert!(trace_interp(&x, &[0.0, 1.0, 0.0, 1.0], TraceMethod::Linear, 0).is_none());
        let (rx, ry) = trace_interp(&x, &[0.0, 1.0, 0.0, 1.0], TraceMethod::Linear, 2).unwrap();
        assert_eq!((rx, ry), (vec![0.0, 3.0], vec![0.0, 1.0]));
        // two points: the spline is the straight line, the B-spline needs four, three is too few
        assert!(trace_interp(&[0.0, 1.0], &[0.0, 1.0], TraceMethod::CubicSpline, 5).is_some());
        assert!(trace_interp(&[0.0, 1.0], &[0.0, 1.0], TraceMethod::CubicBSpline, 5).is_none());
        assert!(trace_interp(&[0.0, 2.0, 1.0], &[0.0, 1.0, 3.0], TraceMethod::CubicBSpline, 7).is_none());
        assert!(trace_interp(&[0.0, 2.0, 1.0], &[0.0, 1.0, 3.0], TraceMethod::CubicSpline, 7).is_some());
    }
}
