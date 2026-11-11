//! Integration against numbers from Origin 2016 (`integ1`).
//!
//! Reference scripts: `tools/origin-ref/integ-suite1.labtalk.txt` .. `integ-suite5.labtalk.txt`
//! (run with `run-labtalk-data.ps1`). Every input below is the closed-form column of the script.
//! Tolerance: relative 1e-12 on areas, positions and cumulative samples (the values agree to
//! rounding); the width at half height uses 1e-10 (1e-8 where the input sits on a tie).

use sheaf_calc::{AreaType, IntegrateOptions, IntegrateResult, integrate};

fn grid(n: usize, start: f64, step: f64) -> Vec<f64> {
    (0..n).map(|i| start + i as f64 * step).collect()
}

fn close(name: &str, got: f64, want: f64, tol: f64) {
    let ok = (got - want).abs() <= tol * want.abs().max(1.0);
    assert!(ok, "{name}: got {got:?}, Origin gave {want:?}");
}

struct Want {
    area: f64,
    x1: f64,
    x2: f64,
    i1: usize,
    i2: usize,
    y0: f64,
    x0: f64,
    dx: f64,
}

fn check(name: &str, r: &IntegrateResult, w: &Want) {
    close(&format!("{name} area"), r.area, w.area, 1e-12);
    close(&format!("{name} x1"), r.x1, w.x1, 1e-12);
    close(&format!("{name} x2"), r.x2, w.x2, 1e-12);
    assert_eq!((r.i1, r.i2), (w.i1, w.i2), "{name} i1/i2");
    close(&format!("{name} y0"), r.peak_height, w.y0, 1e-12);
    close(&format!("{name} x0"), r.peak_x, w.x0, 1e-12);
    close(&format!("{name} dx"), r.width, w.dx, 1e-10);
}

fn check_cum(name: &str, r: &IntegrateResult, samples: &[(usize, f64)]) {
    for &(i, v) in samples {
        close(&format!("{name} cum[{i}]"), r.cumulative[i], v, 1e-12);
    }
}

fn abs_opts() -> IntegrateOptions {
    IntegrateOptions { area_type: AreaType::Absolute, ..Default::default() }
}

fn base_opts() -> IntegrateOptions {
    IntegrateOptions { end_point_baseline: true, ..Default::default() }
}

fn block1() -> (Vec<f64>, Vec<f64>) {
    let a = grid(101, 0.0, 0.1);
    let b = a.iter().map(|&x| (-(x - 5.0).powi(2) / 2.0).exp() + 0.2 * (3.0 * x).sin() + 0.05 * x).collect();
    (a, b)
}

#[test]
fn block1_default_abs_baseline_and_range() {
    let (a, b) = block1();
    let r = integrate(&a, &b, &Default::default()).unwrap();
    check(
        "default",
        &r,
        &Want {
            area: 5.062586533356468,
            x1: 0.0,
            x2: 10.0,
            i1: 1,
            i2: 101,
            y0: 1.413330228616611,
            x0: 4.800000000000001,
            dx: 2.880078322906191,
        },
    );
    check_cum(
        "default",
        &r,
        &[
            (0, 0.0),
            (1, 0.0032056940776703188),
            (2, 0.012558123031847682),
            (3, 0.027289111676483062),
            (4, 0.04619484093895086),
            (5, 0.06774345589685912),
            (10, 0.15674944431314908),
            (25, 0.21513654640842525),
            (50, 1.9947447642826492),
            (75, 4.021169647973555),
            (95, 4.8935230856921486),
            (96, 4.934020910512271),
            (97, 4.969905090628099),
            (98, 5.002280843179919),
            (99, 5.032611228418885),
            (100, 5.062586533356468),
        ],
    );
    let r = integrate(&a, &b, &abs_opts()).unwrap();
    close("abs area", r.area, 5.180258427108602, 1e-12);
    check_cum(
        "abs",
        &r,
        &[
            (0, 0.0),
            (1, 0.0032056940776703188),
            (2, 0.012558123031847682),
            (3, 0.027289111676483062),
            (4, 0.04619484093895086),
            (5, 0.06774345589685912),
            (10, 0.15674944431314908),
            (25, 0.33280844016055966),
            (50, 2.1124166580347836),
            (75, 4.138841541725689),
            (95, 5.011194979444283),
            (96, 5.051692804264405),
            (97, 5.087576984380233),
            (98, 5.119952736932053),
            (99, 5.150283122171019),
            (100, 5.180258427108602),
        ],
    );
    let r = integrate(&a, &b, &base_opts()).unwrap();
    check(
        "baseline",
        &r,
        &Want {
            area: 3.5505808909176073,
            x1: 0.0,
            x2: 10.0,
            i1: 1,
            i2: 101,
            y0: 1.2681775378763536,
            x0: 4.800000000000001,
            dx: 1.7720731879146525,
        },
    );
    check_cum(
        "baseline",
        &r,
        &[
            (0, 0.0),
            (1, 0.003054124574762397),
            (2, 0.01195259035085041),
            (3, 0.025927222142215017),
            (4, 0.043774200876230615),
            (5, 0.06396167163050524),
            (10, 0.1416260339009056),
            (25, 0.12062920628129892),
            (50, 1.616734037040004),
            (75, 3.1706594866269993),
            (95, 3.52893622323082),
            (96, 3.5405550794057983),
            (97, 3.5472578972013005),
            (98, 3.550149893757615),
            (99, 3.5506941293258936),
            (100, 3.5505808909176073),
        ],
    );
    let opts = IntegrateOptions { rows: Some((11, 61)), ..Default::default() };
    let r = integrate(&a, &b, &opts).unwrap();
    check(
        "rows 11..61",
        &r,
        &Want {
            area: 2.8741581959051596,
            x1: 1.0,
            x2: 6.0,
            i1: 11,
            i2: 61,
            y0: 1.413330228616611,
            x0: 4.800000000000001,
            dx: 2.0845167745372715,
        },
    );
    check_cum(
        "rows 11..61",
        &r,
        &[
            (0, 0.0),
            (1, 0.005125414041636481),
            (2, 0.004934240559274654),
            (3, -2.8797032736189304e-05),
            (4, -0.008742287502912201),
            (5, -0.01979728131382367),
            (10, -0.0507033715834616),
            (25, 0.4149967586477456),
            (50, 2.8741581959051596),
        ],
    );
}

#[test]
fn block2_uneven_x() {
    let g: Vec<f64> = grid(61, 0.0, 0.05).iter().map(|t| t * t).collect();
    let h: Vec<f64> = g.iter().map(|x| x.sin() - 0.3).collect();
    let r = integrate(&g, &h, &Default::default()).unwrap();
    close("area", r.area, -0.7960551634901535, 1e-12);
    close("y0", r.peak_height, -1.2959627051561948, 1e-12);
    close("x0", r.peak_x, 4.6225, 1e-12);
    close("dx", r.width, 2.426559046784275, 1e-10);
    check_cum(
        "uneven",
        &r,
        &[
            (0, 0.0),
            (1, -0.0007468750032552074),
            (2, -0.0029500006380177046),
            (3, -0.0064968885446132045),
            (4, -0.011200123481387811),
            (5, -0.016797576146061397),
            (10, -0.04391669760310708),
            (25, 0.5221320237806035),
            (50, -1.8692109617236996),
        ],
    );
    let r = integrate(&g, &h, &abs_opts()).unwrap();
    close("abs area", r.area, 5.345985920803948, 1e-12);
    check_cum(
        "uneven abs",
        &r,
        &[
            (0, 0.0),
            (1, 0.0007468750032552074),
            (2, 0.0029500006380177046),
            (3, 0.0064968885446132045),
            (4, 0.011200123481387811),
            (5, 0.016797576146061397),
            (10, 0.04391669760310708),
            (25, 0.6128411913487279),
            (50, 4.160636694824376),
        ],
    );
    let r = integrate(&g, &h, &base_opts()).unwrap();
    close("baseline area", r.area, 0.04941165292194155, 1e-9);
}

#[test]
fn block3_descending_x() {
    let (a, b) = block1();
    let l: Vec<f64> = a.iter().map(|x| 10.0 - x).collect();
    let r = integrate(&l, &b, &Default::default()).unwrap();
    check(
        "descending",
        &r,
        &Want {
            area: -5.062586533356468,
            x1: 10.0,
            x2: 0.0,
            i1: 1,
            i2: 101,
            y0: 1.413330228616611,
            x0: 5.199999999999999,
            dx: -2.880078322906191,
        },
    );
    check_cum(
        "descending",
        &r,
        &[
            (0, 0.0),
            (1, -0.0032056940776703075),
            (2, -0.012558123031847638),
            (3, -0.02728911167648322),
            (4, -0.04619484093895096),
            (5, -0.06774345589685915),
            (10, -0.15674944431314913),
            (25, -0.2151365464084253),
            (50, -1.9947447642826492),
            (75, -4.021169647973555),
            (95, -4.8935230856921486),
            (96, -4.934020910512271),
            (97, -4.969905090628099),
            (98, -5.002280843179919),
            (99, -5.032611228418885),
            (100, -5.062586533356468),
        ],
    );
    let r = integrate(&l, &b, &abs_opts()).unwrap();
    close("abs area", r.area, 5.180258427108602, 1e-12);
}

#[test]
fn block4_missing_values_are_skipped() {
    let (a, b) = block1();
    let mut p = b.clone();
    p[4] = f64::NAN;
    let r = integrate(&a, &p, &Default::default()).unwrap();
    close("area y missing", r.area, 5.061754230268852, 1e-12);
    assert_eq!((r.i1, r.i2), (1, 100));
    assert!(r.cumulative[4].is_nan());
    check_cum(
        "y missing",
        &r,
        &[
            (0, 0.0),
            (1, 0.0032056940776703188),
            (2, 0.012558123031847682),
            (3, 0.027289111676483062),
            (5, 0.0669111528092429),
            (10, 0.15591714122553285),
            (25, 0.21430424332080902),
            (50, 1.993912461195033),
            (75, 4.020337344885939),
            (95, 4.892690782604532),
            (96, 4.933188607424655),
            (97, 4.969072787540482),
            (98, 5.001448540092302),
            (99, 5.031778925331269),
            (100, 5.061754230268852),
        ],
    );
    let mut x = a.clone();
    x[7] = f64::NAN;
    let r = integrate(&x, &b, &Default::default()).unwrap();
    close("area x missing", r.area, 5.061816308289612, 1e-12);
    assert_eq!((r.i1, r.i2), (1, 100));
    assert!(r.cumulative[7].is_nan());
}

#[test]
fn block5_half_height_width() {
    let a = grid(101, 0.0, 0.1);
    let t: Vec<f64> = a.iter().map(|x| (-(x - 5.0).powi(2) / 2.0).exp()).collect();
    let u: Vec<f64> = t.iter().map(|v| 3.0 + v).collect();
    let v: Vec<f64> = a
        .iter()
        .map(|&x| if x < 5.0 { (-(x - 5.0).powi(2) / 0.5).exp() } else { (-(x - 5.0).powi(2) / 8.0).exp() })
        .collect();
    let w: Vec<f64> = t.iter().map(|v| -v).collect();
    check(
        "gauss",
        &integrate(&a, &t, &Default::default()).unwrap(),
        &Want { area: 2.5066268066310777, x1: 0.0, x2: 10.0, i1: 1, i2: 101, y0: 1.0, x0: 5.0, dx: 2.355336280219587 },
    );
    check(
        "gauss+3",
        &integrate(&a, &u, &Default::default()).unwrap(),
        &Want { area: 32.50662680663109, x1: 0.0, x2: 10.0, i1: 1, i2: 101, y0: 4.0, x0: 5.0, dx: 15.000037266670603 },
    );
    check(
        "asymmetric",
        &integrate(&a, &v, &Default::default()).unwrap(),
        &Want { area: 3.102108936485046, x1: 0.0, x2: 10.0, i1: 1, i2: 101, y0: 1.0, x0: 5.0, dx: 2.9439611586411 },
    );
    check(
        "negative",
        &integrate(&a, &w, &Default::default()).unwrap(),
        &Want {
            area: -2.5066268066310777,
            x1: 0.0,
            x2: 10.0,
            i1: 1,
            i2: 101,
            y0: -1.0,
            x0: 5.0,
            dx: 2.355336280219587,
        },
    );
    check(
        "gauss+3 baseline",
        &integrate(&a, &u, &base_opts()).unwrap(),
        &Want {
            area: 2.506589540099354,
            x1: 0.0,
            x2: 10.0,
            i1: 1,
            i2: 101,
            y0: 0.9999962733468277,
            x0: 5.0,
            dx: 2.355329998161581,
        },
    );
    check(
        "asymmetric baseline",
        &integrate(&a, &v, &base_opts()).unwrap(),
        &Want {
            area: 2.8824242683680095,
            x1: 0.0,
            x2: 10.0,
            i1: 1,
            i2: 101,
            y0: 0.9780315331882963,
            x0: 5.0,
            dx: 2.8657891866099963,
        },
    );
}

#[test]
fn suite2_width_when_half_height_is_not_reached() {
    let a = grid(101, 0.0, 0.1);
    let b: Vec<f64> =
        a.iter().map(|&x| if x < 2.0 { (-(x - 2.0).powi(2)).exp() } else { 1.0 - 0.05 * (x - 2.0) }).collect();
    let c: Vec<f64> =
        a.iter().map(|&x| if x > 8.0 { (-(x - 8.0).powi(2)).exp() } else { 1.0 - 0.05 * (8.0 - x) }).collect();
    let e: Vec<f64> = a.iter().map(|&x| x.sin() + 0.5 * x).collect();
    let k: Vec<f64> = a.iter().map(|x| x.sin()).collect();
    let m: Vec<f64> = a.iter().map(|&x| -1.0 - (-(x - 5.0).powi(2)).exp()).collect();
    check(
        "right side stays high",
        &integrate(&a, &b, &Default::default()).unwrap(),
        &Want { area: 7.282020440395562, x1: 0.0, x2: 10.0, i1: 1, i2: 101, y0: 1.0, x0: 2.0, dx: 10.833108068971201 },
    );
    check(
        "left side stays high",
        &integrate(&a, &c, &Default::default()).unwrap(),
        &Want { area: 7.282020440395562, x1: 0.0, x2: 10.0, i1: 1, i2: 101, y0: 1.0, x0: 8.0, dx: 8.833108068971201 },
    );
    let rows = |lo, hi| IntegrateOptions { rows: Some((lo, hi)), ..Default::default() };
    check(
        "rows 31..71",
        &integrate(&a, &e, &rows(31, 71)).unwrap(),
        &Want {
            area: 8.257558736947322,
            x1: 3.0,
            x2: 7.0,
            i1: 31,
            i2: 71,
            y0: 4.156986598718789,
            x0: 7.0,
            dx: 5.527386026406871,
        },
    );
    let o = IntegrateOptions { rows: Some((31, 71)), area_type: AreaType::Absolute, end_point_baseline: true };
    check(
        "rows abs baseline",
        &integrate(&a, &e, &o).unwrap(),
        &Want {
            area: 3.338654476609987,
            x1: 3.0,
            x2: 7.0,
            i1: 31,
            i2: 71,
            y0: -1.369424582692223,
            x0: 4.800000000000001,
            dx: 2.5208796068050767,
        },
    );
    check(
        "sine, abs",
        &integrate(&a, &k, &abs_opts()).unwrap(),
        &Want {
            area: 6.155796341367698,
            x1: 0.0,
            x2: 10.0,
            i1: 1,
            i2: 101,
            y0: -0.9999232575641008,
            x0: 4.7,
            dx: 9.949055476094582,
        },
    );
    check(
        "trough",
        &integrate(&a, &m, &Default::default()).unwrap(),
        &Want {
            area: -11.77245385090256,
            x1: 0.0,
            x2: 10.0,
            i1: 1,
            i2: 101,
            y0: -2.0,
            x0: 5.0,
            dx: 10.000000000069441,
        },
    );
    check(
        "two points",
        &integrate(&a, &e, &rows(1, 2)).unwrap(),
        &Want {
            area: 0.007491670832341407,
            x1: 0.0,
            x2: 0.1,
            i1: 1,
            i2: 2,
            y0: 0.14983341664682814,
            x0: 0.1,
            dx: 0.05,
        },
    );
}

#[test]
fn suite3_plateau_and_largest_magnitude() {
    let a = grid(101, 0.0, 0.1);
    let b: Vec<f64> = a
        .iter()
        .map(|&x| {
            let d = (x - 5.0).abs();
            if d <= 2.0 { 1.0 - if d > 1.0 { d - 1.0 } else { 0.0 } } else { 0.0 }
        })
        .collect();
    let r = integrate(&a, &b, &Default::default()).unwrap();
    close("plateau area", r.area, 2.999999999999999, 1e-9);
    close("plateau y0", r.peak_height, 1.0, 1e-12);
    close("plateau x0 (first row of the flat top)", r.peak_x, 4.0, 1e-9);
    close("plateau dx", r.width, 3.0, 1e-9);
    let d: Vec<f64> = a.iter().map(|&x| (-(x - 2.0).powi(2)).exp() - (-(x - 7.0).powi(2)).exp()).collect();
    let r = integrate(&a, &d, &Default::default()).unwrap();
    close("two lobes area", r.area, -0.00418629388465145, 1e-9);
    close("two lobes y0", r.peak_height, 0.9999999999861121, 1e-12);
    close("two lobes x0", r.peak_x, 2.0, 1e-12);
    // negative area with a positive largest lobe: the half-height search is mirrored
    close("two lobes dx", r.width, 5.999506421696049, 1e-8);
}

#[test]
fn suite4_second_lobes() {
    let a = grid(101, 0.0, 0.1);
    let g = |c: f64, x: f64| (-(x - c).powi(2)).exp();
    let cases: [(&str, Vec<f64>, [f64; 4]); 5] = [
        ("offset -0.3", a.iter().map(|&x| g(2.0, x) - 0.3).collect(), [-1.2317526341516805, 0.7, 2.0, 4.8]),
        (
            "trough -0.5",
            a.iter().map(|&x| g(2.0, x) - 0.5 * g(7.0, x)).collect(),
            [0.8820305359818339, 0.999999999993056, 2.0, 1.6662161190407248],
        ),
        (
            "mirror",
            a.iter().map(|&x| g(8.0, x) - 0.5 * g(3.0, x)).collect(),
            [0.8820305359818335, 0.999999999993056, 8.0, 1.6662161190407252],
        ),
        (
            "second lobe above half",
            a.iter().map(|&x| g(2.0, x) + 0.5 * g(7.0, x)).collect(),
            [2.654464195714806, 1.000000000006944, 2.0, 5.833108069176353],
        ),
        (
            "second lobe 0.8",
            a.iter().map(|&x| g(5.0, x) + 0.8 * g(8.0, x)).collect(),
            [3.187051743581219, 1.0000987278432694, 5.0, 4.518429353575116],
        ),
    ];
    for (name, y, w) in cases {
        let r = integrate(&a, &y, &Default::default()).unwrap();
        close(&format!("{name} area"), r.area, w[0], 1e-11);
        close(&format!("{name} y0"), r.peak_height, w[1], 1e-11);
        close(&format!("{name} x0"), r.peak_x, w[2], 1e-12);
        close(&format!("{name} dx"), r.width, w[3], 1e-9);
    }
}

#[test]
fn suite5_width_flips_with_the_sign_of_the_area() {
    // offsets that make the area negative (cases 3 and 4) send the half-height search the other way
    let x = grid(101, 0.0, 0.1);
    let g = |c: f64, x: f64| (-(x - c).powi(2)).exp();
    let cases: Vec<(Vec<f64>, [f64; 2])> = vec![
        (x.iter().map(|&x| g(2.0, x) - 0.02).collect(), [0.98, 1.641954409545556]),
        (x.iter().map(|&x| g(2.0, x) - 0.05).collect(), [0.95, 1.605561816950285]),
        (x.iter().map(|&x| g(2.0, x) - 0.1).collect(), [0.9, 1.5467795160137676]),
        (x.iter().map(|&x| g(2.0, x) - 0.2).collect(), [0.8, 5.2]),
        (x.iter().map(|&x| g(2.0, x) - 0.4).collect(), [0.6, 4.4]),
        (x.iter().map(|&x| g(2.0, x) - 0.6 * g(7.0, x)).collect(), [0.9999999999916672, 1.6662161152603894]),
        (x.iter().map(|&x| g(2.0, x) - 0.7 * g(7.0, x)).collect(), [0.9999999999902784, 1.6662161114800547]),
        (x.iter().map(|&x| g(2.0, x) - 0.8 * g(7.0, x)).collect(), [0.9999999999888897, 1.6662161076997202]),
        (x.iter().map(|&x| g(2.0, x) - 0.9 * g(7.0, x)).collect(), [0.9999999999875009, 1.6662161039193855]),
    ];
    for (n, (y, w)) in cases.iter().enumerate() {
        let r = integrate(&x, y, &Default::default()).unwrap();
        close(&format!("case {n} x0"), r.peak_x, 2.0, 1e-12);
        close(&format!("case {n} y0"), r.peak_height, w[0], 1e-11);
        close(&format!("case {n} dx"), r.width, w[1], 1e-9);
    }
}

#[test]
fn too_few_points_give_none() {
    assert!(integrate(&[1.0], &[2.0], &Default::default()).is_none());
    assert!(integrate(&[1.0, f64::NAN], &[2.0, 3.0], &Default::default()).is_none());
}

#[test]
fn charge_is_the_signed_trapezoid_area() {
    let t = [0.0, 1.0, 2.0, 4.0];
    let i = [0.0, 2.0, 2.0, -1.0];
    // 1 + 2 + 0.5 * 2
    assert!((sheaf_calc::charge(&t, &i, None).unwrap() - 4.0).abs() < 1e-15);
}

#[test]
fn suite6_peak_on_the_last_or_first_point() {
    // tools/origin-ref/integ-suite6.labtalk.txt: x = 2..10 step 0.1
    let a = grid(81, 2.0, 0.1);
    let cases: Vec<(&str, Vec<f64>, [f64; 4])> = vec![
        ("x^2", a.iter().map(|x| x * x).collect(), [330.68, 100.0, 10.0, 7.070921985815603]),
        ("(12-x)^2", a.iter().map(|x| (12.0 - x).powi(2)).collect(), [330.68, 100.0, 2.0, 2.9290780141843973]),
        ("x^2-30", a.iter().map(|x| x * x - 30.0).collect(), [90.68000000000002, 70.0, 10.0, 8.062111801242235]),
        ("5+x", a.iter().map(|x| 5.0 + x).collect(), [87.99999999999997, 15.0, 10.0, 2.5]),
    ];
    for (name, y, w) in cases {
        let r = integrate(&a, &y, &Default::default()).unwrap();
        close(&format!("{name} area"), r.area, w[0], 1e-12);
        close(&format!("{name} y0"), r.peak_height, w[1], 1e-12);
        close(&format!("{name} x0"), r.peak_x, w[2], 1e-12);
        close(&format!("{name} dx"), r.width, w[3], 1e-10);
    }
}
