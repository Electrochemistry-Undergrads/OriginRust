//! Cubic B-spline interpolation against numbers from Origin 2016 (`interp1 method:=bspline`).
//!
//! Reference script: `tools/origin-ref/interp-bspline.labtalk.txt` (run with
//! `run-labtalk-data.ps1`); data x = 0..10 step 1, evaluated at -1..11 step 0.25.
//!
//! What can be checked: Origin's B-spline is a smoothing spline whose default (`sf = 0`) smoothing
//! factor is chosen automatically, in a way we did not find. Our `InterpMethod::BSpline` is the
//! interpolating cubic spline. The two agree where Origin's smoothing has nothing to remove: data
//! that already lie on a cubic polynomial (a line and a parabola below), and outside the data,
//! where Origin continues with the straight line through the two nearest data points whatever the
//! smoothing. Inside the data for other inputs Origin's curve differs from ours (by up to 0.27 for
//! a unit impulse); that is documented in `docs/spec/analysis-tools.md`, not tested here.
//! Tolerance: 1e-10 relative.

use sheaf_calc::{InterpMethod, InterpOptions, interp1};

const TOL: f64 = 1e-10;

fn bspline() -> InterpOptions {
    InterpOptions { method: InterpMethod::BSpline, ..Default::default() }
}

fn data(name: &str) -> Vec<f64> {
    let x = (0..11).map(f64::from);
    match name {
        "impulse at x = 0" => x.map(|v| f64::from(v == 0.0)).collect(),
        "impulse at x = 1" => x.map(|v| f64::from(v == 1.0)).collect(),
        "y = x^2" => x.map(|v| v * v).collect(),
        _ => x.map(|v| 3.0 + 2.0 * v).collect(),
    }
}

/// Origin outputs at x = -1, -0.75, ..., 11 for the data x = 0..10 (Origin column index in the dump).
const GRID_OUT: &[(&str, &[f64])] = &[
    (
        "impulse at x = 0",
        &[
            2.0,
            1.75,
            1.5,
            1.25,
            0.8837788612175789,
            0.6792214571079461,
            0.5048016018730923,
            0.35821501833288383,
            0.23715742930718708,
            0.13932455761586832,
            0.06241212607879391,
            0.004115857515830212,
            -0.03786852525315638,
            -0.06584529940829957,
            -0.08211874212973298,
            -0.08899313059759024,
            -0.08877274199200505,
            -0.08353620800479433,
            -0.07445957837450831,
            -0.06249325735138055,
            -0.04858764918564457,
            -0.03369315812753393,
            -0.018760188427282123,
            -0.004739144335122751,
            0.007419569898710685,
            0.016965112769926623,
            0.02394489375800158,
            0.02860588508835405,
            0.031195058986402523,
            0.03195938767756549,
            0.031145843387261462,
            0.029001398340908934,
            0.025773024763926393,
            0.021707694881732346,
            0.017052380919745293,
            0.012054055103383723,
            0.00695968965806613,
            0.0020162568092110212,
            -0.0025292712177631036,
            -0.006429922197437753,
            -0.009438723904394434,
            -0.011308704113214642,
            -0.01179289059847988,
            -0.010644311134771654,
            -0.007615993496671467,
            0.0,
            0.0,
            0.0,
            0.0,
        ],
    ),
    (
        "impulse at x = 1",
        &[
            -1.0,
            -0.75,
            -0.5,
            -0.25,
            0.046553374638451314,
            0.4632086855324111,
            0.7097712921803438,
            0.8155843107914875,
            0.8099908575750803,
            0.7223340487403599,
            0.5819570004965646,
            0.41820282905293227,
            0.2604146506187009,
            0.13241094964569278,
            0.0359116835560666,
            -0.03288782198543466,
            -0.07779224131406814,
            -0.10252395151301744,
            -0.11047614065717241,
            -0.10495969956934958,
            -0.08928551907236539,
            -0.06676448998903631,
            -0.04070750314217878,
            -0.014425449354609288,
            0.0087707805508557,
            0.02623176834707789,
            0.03795398618963157,
            0.04459537882976916,
            0.04681389101874312,
            0.04526746750780588,
            0.040614053048209864,
            0.03351159239120753,
            0.0246180302880513,
            0.014591311489993635,
            0.0040893807482869525,
            -0.0062298171858163,
            -0.015708337561063684,
            -0.023688235626202758,
            -0.029511566629981084,
            -0.03252038582114623,
            -0.03205674844844577,
            -0.027462709760627237,
            -0.018080325006438212,
            -0.0032516494346262444,
            0.017681261706061095,
            0.0,
            0.0,
            0.0,
            0.0,
        ],
    ),
    (
        "y = x^2",
        &[
            -1.0,
            -0.75,
            -0.5,
            -0.25,
            9.77598130818618e-15,
            0.06250000000000684,
            0.25000000000000416,
            0.562500000000002,
            0.9999999999999998,
            1.5624999999999982,
            2.249999999999997,
            3.062499999999995,
            3.9999999999999942,
            5.062499999999995,
            6.249999999999993,
            7.562499999999991,
            8.999999999999991,
            10.562499999999991,
            12.249999999999991,
            14.062499999999995,
            15.999999999999995,
            18.062499999999993,
            20.249999999999993,
            22.562499999999993,
            24.999999999999993,
            27.562499999999993,
            30.249999999999993,
            33.0625,
            36.0,
            39.0625,
            42.25,
            45.562500000000014,
            49.0,
            52.5625,
            56.250000000000014,
            60.0625,
            64.0,
            68.06250000000001,
            72.25,
            76.5625,
            81.00000000000001,
            85.5625,
            90.24999999999999,
            95.06250000000001,
            100.0,
            104.75,
            109.5,
            114.25,
            119.0,
        ],
    ),
    (
        "y = 3 + 2 x",
        &[
            1.0,
            1.5,
            2.0,
            2.5,
            3.0000000000000004,
            3.5000000000000004,
            4.000000000000001,
            4.5,
            5.0,
            5.5,
            6.0,
            6.500000000000001,
            7.000000000000001,
            7.5,
            8.0,
            8.500000000000002,
            9.000000000000004,
            9.500000000000002,
            10.000000000000002,
            10.5,
            11.0,
            11.500000000000002,
            12.0,
            12.499999999999998,
            12.999999999999998,
            13.5,
            14.0,
            14.5,
            15.000000000000004,
            15.500000000000004,
            16.000000000000004,
            16.500000000000004,
            17.000000000000004,
            17.5,
            18.0,
            18.499999999999996,
            18.999999999999996,
            19.499999999999996,
            20.0,
            20.499999999999996,
            20.999999999999996,
            21.5,
            22.0,
            22.5,
            23.0,
            23.5,
            24.0,
            24.5,
            25.0,
        ],
    ),
];

#[test]
fn polynomial_data_and_the_straight_continuation_match_origin() {
    let x: Vec<f64> = (0..11).map(f64::from).collect();
    let at: Vec<f64> = (0..49).map(|i| -1.0 + 0.25 * f64::from(i)).collect();
    for &(name, want) in GRID_OUT {
        let y = data(name);
        let got = interp1(&x, &y, &at, &bspline());
        let polynomial = name.starts_with("y =");
        for (i, (&t, (&g, &w))) in at.iter().zip(got.iter().zip(want)).enumerate() {
            let outside = !(0.0..=10.0).contains(&t);
            if polynomial || outside {
                assert!(
                    (g - w).abs() <= TOL * w.abs().max(1.0),
                    "{name} at {t} (row {i}): got {g:?}, Origin gave {w:?}"
                );
            }
        }
    }
}

#[test]
fn missing_and_repeat_options_apply_outside_the_data() {
    use sheaf_calc::Extrapolation;
    let x: Vec<f64> = (0..6).map(f64::from).collect();
    let y = [1.0, 4.0, 2.0, 8.0, 5.0, 6.0];
    let o = |extrapolation| InterpOptions { extrapolation, ..bspline() };
    let r = interp1(&x, &y, &[-1.0, 6.0], &o(Extrapolation::Missing));
    assert!(r.iter().all(|v| v.is_nan()));
    assert_eq!(interp1(&x, &y, &[-1.0, 6.0], &o(Extrapolation::Repeat)), vec![1.0, 6.0]);
    // the straight continuation uses the data points, not the end slope of the curve
    let r = interp1(&x, &y, &[-1.0, 7.0], &o(Extrapolation::Extrapolate));
    assert!((r[0] - (-2.0)).abs() < 1e-12 && (r[1] - 8.0).abs() < 1e-12, "{r:?}");
}

#[test]
fn the_data_points_are_reproduced_and_unsupported_inputs_give_nan() {
    let x: Vec<f64> = (0..8).map(f64::from).collect();
    let y = [2.0, -1.0, 0.5, 4.0, 3.0, 3.5, -2.0, 1.0];
    for (g, w) in interp1(&x, &y, &x, &bspline()).iter().zip(&y) {
        assert!((g - w).abs() < 1e-12);
    }
    // a smoothing factor above zero is not implemented
    let smooth = InterpOptions { smoothing: 0.3, ..bspline() };
    assert!(interp1(&x, &y, &[2.5], &smooth)[0].is_nan());
    // Origin needs four points
    assert!(interp1(&x[..3], &y[..3], &[1.5], &bspline())[0].is_nan());
}
