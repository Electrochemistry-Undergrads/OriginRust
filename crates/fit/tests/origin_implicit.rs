//! The Nonlinear Implicit Curve Fit of a circle against the numbers Origin 2016 reported
//! (`tools/origin-ref/t91/imp1.lines.txt`, 24 points of a circle with centre (2, 3), radius 5, noise sd 0.08;
//! `docs/spec/fitting.md` section 18). Origin solves with ODRPACK, whose residual sum of squares and errors are
//! about 0.1 % away from the exact orthogonal distances Sheaf computes, so the parameters are compared to 1e-6 and
//! the statistics to 2e-3 ("ODRPACK, Origin within 0.2 %"); the R-square total sum of squares is exact.
//! The algebraic least-squares fit of `f = 0` is a different answer and is checked to be so.

use sheaf_fit::implicit::{ImplicitFunction, ImplicitOptions, fit_implicit};

const X: [f64; 24] = [
    6.8771, 6.7559, 5.9526, 5.2031, 3.9579, 2.8619, 1.6235, 0.2221, -0.7142, -1.9336, -2.5504, -2.9623, -2.9532,
    -2.5886, -2.2021, -1.2785, -0.1731, 1.1322, 2.7632, 3.8372, 4.8968, 5.8634, 6.5663, 6.9798,
];
const Y: [f64; 24] = [
    3.5294, 4.7287, 5.9146, 6.9587, 7.4546, 7.9726, 8.0536, 7.6544, 7.0358, 6.0738, 5.0668, 3.8005, 2.5562, 1.247,
    0.1363, -0.8845, -1.5579, -1.9176, -1.9789, -1.7822, -1.0059, -0.1353, 0.8643, 2.1296,
];

fn rel(got: f64, want: f64) -> f64 {
    (got - want).abs() / want.abs()
}

#[test]
fn the_circle_matches_origins_report() {
    let fit = fit_implicit(ImplicitFunction::Circle, &X, &Y, &ImplicitOptions::default()).unwrap();
    let r = &fit.report;
    let origin = [
        ("xc", 2.001_331_632_901_927_7, 0.021154562615, 94.6),
        ("yc", 2.971_153_427_353_531, 0.02123133529951, 139.9),
        ("r", 5.003_592_864_948_637, 0.014985779192603, 333.9),
    ];
    for (name, value, error, t) in origin {
        let p = r.param(name).unwrap();
        assert!((p.value - value).abs() < 1e-6, "{name}: {} vs {value}", p.value);
        assert!(rel(p.std_error, error) < 2e-3, "{name} error {} vs {error}", p.std_error);
        assert!(rel(p.t_value, t) < 2e-3 + 1e-3, "{name} t {} vs {t}", p.t_value);
        assert!(p.prob_t < 1e-12, "Prob>|t| is 0 in Origin");
    }
    assert_eq!((r.stats.n_points, r.stats.dof), (24, 21));
    assert!(rel(r.stats.rss, 0.113_108_308_370_855_3) < 2e-3, "{}", r.stats.rss);
    assert!(rel(r.stats.reduced_chi_sqr, 0.005_386_109_922_421_681) < 2e-3);
    assert!(rel(r.stats.r_squared, 0.999_815_405_261_647_7) < 2e-6);
    assert!(rel(r.stats.adj_r_squared, 0.999797824810376) < 2e-6);
    assert!(r.anova.is_none(), "Origin's ANOVA rows hold 0");
    // Exact orthogonal distances (scipy.optimize.least_squares): 0.1132499 for the sum of squares.
    assert!(rel(r.stats.rss, 0.1132499) < 1e-4, "{}", r.stats.rss);
}

#[test]
fn it_is_the_geometric_fit_not_the_algebraic_one() {
    let fit = fit_implicit(ImplicitFunction::Circle, &X, &Y, &ImplicitOptions::default()).unwrap();
    // The algebraic fit of f = 0 gives 2.00176, 2.97179, 5.00406 (spec section 18): not what Origin reports.
    let xc = fit.report.params[0].value;
    assert!((xc - 2.00133163).abs() < 1e-7);
    assert!((xc - 2.00176).abs() > 3e-4);
    // Every distance is the signed distance to the circle: sqrt((x-xc)^2+(y-yc)^2) - r.
    let p: Vec<f64> = fit.report.params.iter().map(|q| q.value).collect();
    for i in 0..24 {
        let d = (X[i] - p[0]).hypot(Y[i] - p[1]) - p[2];
        assert!((fit.distances[i] - d).abs() < 1e-12);
    }
}

/// scipy-checked, not Origin: `scipy.optimize.least_squares` on the exact orthogonal distances of an ellipse (axes
/// along X and Y) gives xc 1.03874781, yc 1.98846985, a 4.01758526, b 2.02210557, sum of squares 0.051897632 and errors
/// 0.02616866, 0.01526792, 0.03093404, 0.01929496. Origin's Ellipse fit was not measured.
#[test]
fn the_ellipse_is_scipy_checked_not_origin() {
    let x = [
        5.0223, 4.4221, 3.926, 3.0074, 1.9864, 0.8935, -0.2941, -1.2098, -2.0767, -2.4679, -2.9425, -2.9855, -2.6893,
        -2.1318, -1.3348, -0.3088, 0.8273, 1.8989, 2.9959, 3.8247,
    ];
    let y = [
        2.3986, 2.999, 3.402, 3.6963, 3.9262, 4.0265, 4.0057, 3.6574, 3.2915, 2.8858, 2.2587, 1.7324, 1.2547, 0.7641,
        0.362, 0.1402, -0.139, 0.1036, 0.2052, 0.5055,
    ];
    let fit = fit_implicit(ImplicitFunction::Ellipse, &x, &y, &ImplicitOptions::default()).unwrap();
    let r = &fit.report;
    let want = [(1.03874781, 0.02616866), (1.98846985, 0.01526792), (4.01758526, 0.03093404), (2.02210557, 0.01929496)];
    for (p, (value, error)) in r.params.iter().zip(want) {
        assert!((p.value - value).abs() < 1e-6, "{}: {} vs {value}", p.name, p.value);
        assert!(rel(p.std_error, error) < 1e-4, "{} error {} vs {error}", p.name, p.std_error);
    }
    assert!(rel(r.stats.rss, 0.051897632) < 1e-6);
}
