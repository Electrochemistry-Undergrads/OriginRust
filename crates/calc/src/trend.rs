//! Trendline fits for on-graph trendlines (T43, graphing spec 18.11).
//!
//! Pure functions of `(x, y)` points. Points with a non-finite coordinate are skipped, and the
//! transformed fits skip the points that cannot be transformed (see [`TrendKind`]).
//!
//! * Linear and polynomial: ordinary least squares (the same centred, scaled solver the smoothing
//!   tools use; the coefficients are reported as powers of x).
//! * Power `y = a x^b`, exponential `y = a e^(bx)`, logarithmic `y = a + b ln x`: least squares on
//!   the transformed data (`ln y` against `ln x`, `ln y` against `x`, `y` against `ln x`), the
//!   method of spreadsheet trendlines. It is not a nonlinear fit of `y`.
//! * R-squared is `1 - SSres / SStot` in the space that was fitted: on `y` for linear, polynomial
//!   and logarithmic, on `ln y` for power and exponential (Excel-style). It is `None` when the
//!   total sum of squares is zero.
//! * Moving average: the trailing mean of `window` consecutive points taken in increasing X order
//!   (stable for equal X), reported at the X of the last point of each window.

use crate::util::polyfit_centered;
use serde::{Deserialize, Serialize};

/// The kinds of trendline.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrendKind {
    Linear,
    /// Order 2 to 10 in the dialog (any order of 1 or more is computed).
    Polynomial {
        order: u8,
    },
    /// `y = a x^b`; uses the points with x > 0 and y > 0.
    Power,
    /// `y = a + b ln x`; uses the points with x > 0.
    Logarithmic,
    /// `y = a e^(bx)`; uses the points with y > 0.
    Exponential,
    /// Trailing mean of `window` points (2 to 10 in the dialog); no equation, no R-squared.
    MovingAverage {
        window: u8,
    },
}

impl TrendKind {
    /// The name shown in the dialog and the command line (`linear`, `poly:3`, `ma:5`, ...).
    pub fn name(self) -> String {
        match self {
            TrendKind::Linear => "linear".into(),
            TrendKind::Polynomial { order } => format!("poly:{order}"),
            TrendKind::Power => "power".into(),
            TrendKind::Logarithmic => "log".into(),
            TrendKind::Exponential => "exp".into(),
            TrendKind::MovingAverage { window } => format!("ma:{window}"),
        }
    }

    /// Parses [`TrendKind::name`] (also `polynomial`, `logarithmic`, `exponential`, `moving-average`;
    /// plain `poly` is order 2, `ma` window 2).
    pub fn parse(s: &str) -> Result<TrendKind, String> {
        let s = s.trim().to_ascii_lowercase();
        let (head, arg) = match s.split_once(':') {
            Some((h, a)) => (h, Some(a)),
            None => (s.as_str(), None),
        };
        let num = |lo: u8, hi: u8, what: &str, default: u8| -> Result<u8, String> {
            match arg {
                None => Ok(default),
                Some(a) => match a.trim().parse::<u8>() {
                    Ok(n) if (lo..=hi).contains(&n) => Ok(n),
                    _ => Err(format!("{what} must be a whole number from {lo} to {hi}, got '{a}'")),
                },
            }
        };
        match head {
            "linear" | "lin" if arg.is_none() => Ok(TrendKind::Linear),
            "poly" | "polynomial" => Ok(TrendKind::Polynomial { order: num(2, 10, "polynomial order", 2)? }),
            "power" if arg.is_none() => Ok(TrendKind::Power),
            "log" | "logarithmic" if arg.is_none() => Ok(TrendKind::Logarithmic),
            "exp" | "exponential" if arg.is_none() => Ok(TrendKind::Exponential),
            "ma" | "moving-average" | "movingaverage" => Ok(TrendKind::MovingAverage { window: num(2, 10, "moving average window", 2)? }),
            _ => Err(format!("unknown trendline '{s}' (linear, poly:N, power, log, exp, ma:N)")),
        }
    }

    pub fn is_fit(self) -> bool {
        !matches!(self, TrendKind::MovingAverage { .. })
    }
}

/// A fitted trend curve.
#[derive(Clone, Debug, PartialEq)]
pub struct TrendFit {
    pub kind: TrendKind,
    /// Linear and polynomial: coefficients of `1, x, x^2, ...`. Power, exponential, logarithmic:
    /// `[a, b]` of the equation in the [`TrendKind`] docs.
    pub coeffs: Vec<f64>,
    /// R-squared in the fitted space, see the module docs.
    pub r2: Option<f64>,
    /// Points that entered the fit.
    pub n: usize,
    // Evaluation basis of the polynomial (accurate when x is far from 0).
    x0: f64,
    scale: f64,
    ucoeffs: Vec<f64>,
}

impl TrendFit {
    /// The fitted curve at `x` (NaN where the curve is undefined: `ln x` for x <= 0).
    pub fn eval(&self, x: f64) -> f64 {
        match self.kind {
            TrendKind::Linear | TrendKind::Polynomial { .. } => {
                let u = (x - self.x0) / self.scale;
                self.ucoeffs.iter().rev().fold(0.0, |acc, c| acc * u + c)
            }
            TrendKind::Power => self.coeffs[0] * x.powf(self.coeffs[1]),
            TrendKind::Exponential => self.coeffs[0] * (self.coeffs[1] * x).exp(),
            TrendKind::Logarithmic => {
                if x > 0.0 { self.coeffs[0] + self.coeffs[1] * x.ln() } else { f64::NAN }
            }
            TrendKind::MovingAverage { .. } => f64::NAN,
        }
    }

    /// The equation in the label markup (`\+(..)` superscript), numbers to `sig` significant digits.
    pub fn equation(&self, sig: usize) -> String {
        let c = &self.coeffs;
        match self.kind {
            TrendKind::Linear | TrendKind::Polynomial { .. } => {
                let mut terms = Vec::new();
                for k in (0..c.len()).rev() {
                    let body = match k {
                        0 => String::new(),
                        1 => "x".to_string(),
                        _ => format!("x\\+({k})"),
                    };
                    terms.push((c[k], body));
                }
                format!("y = {}", join_terms(&terms, sig))
            }
            TrendKind::Power => format!("y = {}x\\+({})", sig_str(c[0], sig), sig_str(c[1], sig)),
            TrendKind::Exponential => format!("y = {}e\\+({}x)", sig_str(c[0], sig), sig_str(c[1], sig)),
            TrendKind::Logarithmic => {
                format!("y = {}", join_terms(&[(c[1], "ln(x)".to_string()), (c[0], String::new())], sig))
            }
            TrendKind::MovingAverage { .. } => String::new(),
        }
    }
}

fn join_terms(terms: &[(f64, String)], sig: usize) -> String {
    let mut s = String::new();
    for (i, (v, body)) in terms.iter().enumerate() {
        let num = sig_str(v.abs(), sig);
        let neg = *v < 0.0 && num != "0";
        if i == 0 {
            if neg {
                s.push('-');
            }
        } else {
            s.push_str(if neg { " - " } else { " + " });
        }
        s.push_str(&num);
        s.push_str(body);
    }
    s
}

/// `v` to `sig` significant digits without trailing zeros; scientific (`1.234×10\+(-5)` in the
/// label markup) outside 1e-3 .. 1e6.
pub fn sig_str(v: f64, sig: usize) -> String {
    if !v.is_finite() {
        return "NaN".into();
    }
    if v == 0.0 {
        return "0".into();
    }
    let sig = sig.max(1);
    let e = format!("{:.*e}", sig - 1, v);
    let (mant, exp) = e.split_once('e').unwrap_or((&e, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    if (-3..6).contains(&exp) {
        let decimals = (sig as i32 - 1 - exp).max(0) as usize;
        let s = format!("{v:.decimals$}");
        if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
    } else {
        let mant = if mant.contains('.') { mant.trim_end_matches('0').trim_end_matches('.') } else { mant };
        format!("{mant}\u{d7}10\\+({exp})")
    }
}

fn r_squared(y: &[f64], pred: impl Fn(usize) -> f64) -> Option<f64> {
    let mean = y.iter().sum::<f64>() / y.len() as f64;
    let sstot: f64 = y.iter().map(|v| (v - mean).powi(2)).sum();
    if !(sstot > 0.0) {
        return None;
    }
    let ssres: f64 = y.iter().enumerate().map(|(i, v)| (v - pred(i)).powi(2)).sum();
    Some(1.0 - ssres / sstot)
}

/// Converts `sum c_k u^k` with `u = (x - x0) / scale` to powers of x.
fn to_powers(uc: &[f64], x0: f64, scale: f64) -> Vec<f64> {
    let n = uc.len();
    // (x - x0)^k / scale^k expanded by the binomial theorem.
    let mut out = vec![0.0; n];
    let mut binom = vec![1.0_f64];
    for k in 0..n {
        if k > 0 {
            let mut next = vec![1.0; k + 1];
            for j in 1..k {
                next[j] = binom[j - 1] + binom[j];
            }
            binom = next;
        }
        let f = uc[k] / scale.powi(k as i32);
        for j in 0..=k {
            out[j] += f * binom[j] * (-x0).powi((k - j) as i32);
        }
    }
    out
}

/// Fits `kind` to the finite points of `(x, y)`. `None` when the kind is a moving average, when
/// too few usable points remain (a polynomial of order `m` needs `m + 1` distinct x; the others
/// need 2) or the system is singular.
pub fn fit(kind: TrendKind, x: &[f64], y: &[f64]) -> Option<TrendFit> {
    let pts: Vec<(f64, f64)> =
        x.iter().zip(y).filter(|(a, b)| a.is_finite() && b.is_finite()).map(|(a, b)| (*a, *b)).collect();
    let order = match kind {
        TrendKind::Linear | TrendKind::Power | TrendKind::Logarithmic | TrendKind::Exponential => 1,
        TrendKind::Polynomial { order } => usize::from(order.max(1)),
        TrendKind::MovingAverage { .. } => return None,
    };
    // The transformed points the least squares runs on.
    let t: Vec<(f64, f64)> = match kind {
        TrendKind::Power => pts.iter().filter(|p| p.0 > 0.0 && p.1 > 0.0).map(|p| (p.0.ln(), p.1.ln())).collect(),
        TrendKind::Exponential => pts.iter().filter(|p| p.1 > 0.0).map(|p| (p.0, p.1.ln())).collect(),
        TrendKind::Logarithmic => pts.iter().filter(|p| p.0 > 0.0).map(|p| (p.0.ln(), p.1)).collect(),
        _ => pts,
    };
    if t.len() <= order {
        return None;
    }
    let xs: Vec<f64> = t.iter().map(|p| p.0).collect();
    let ys: Vec<f64> = t.iter().map(|p| p.1).collect();
    let x0 = xs.iter().sum::<f64>() / xs.len() as f64;
    let (uc, scale) = polyfit_centered(&xs, &ys, x0, order)?;
    if uc.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let eval_u = |x: f64| uc.iter().rev().fold(0.0, |acc, c| acc * ((x - x0) / scale) + c);
    let r2 = r_squared(&ys, |i| eval_u(xs[i]));
    let pw = to_powers(&uc, x0, scale);
    let coeffs = match kind {
        TrendKind::Linear | TrendKind::Polynomial { .. } => pw,
        TrendKind::Power | TrendKind::Exponential => vec![pw[0].exp(), pw[1]],
        _ => pw,
    };
    Some(TrendFit { kind, coeffs, r2, n: t.len(), x0, scale, ucoeffs: uc })
}

/// The trailing moving average: points sorted by X (stable), one output per full window, at the
/// X of the window's last point. Empty when fewer than `window` finite points exist.
pub fn moving_average(x: &[f64], y: &[f64], window: usize) -> Vec<(f64, f64)> {
    let window = window.max(1);
    let mut pts: Vec<(f64, f64)> =
        x.iter().zip(y).filter(|(a, b)| a.is_finite() && b.is_finite()).map(|(a, b)| (*a, *b)).collect();
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    if pts.len() < window {
        return Vec::new();
    }
    (window - 1..pts.len())
        .map(|i| {
            let s: f64 = pts[i + 1 - window..=i].iter().map(|p| p.1).sum();
            (pts[i].0, s / window as f64)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reference values: numpy 2.5.4 (`np.polyfit` on the raw or log-transformed data; R-squared
    // as 1 - SSres/SStot in the fitted space), computed with python3 -I.
    const X: [f64; 12] = [0.5, 1.0, 1.7, 2.2, 3.0, 3.9, 4.5, 5.2, 6.0, 7.1, 8.0, 9.3];
    const Y: [f64; 12] = [1.9, 2.4, 3.9, 4.1, 6.8, 8.2, 10.5, 12.0, 16.1, 19.5, 24.7, 31.0];

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() <= 2e-8 * b.abs().max(1.0), "{a} vs {b}");
    }

    #[test]
    fn linear_matches_numpy() {
        let f = fit(TrendKind::Linear, &X, &Y).unwrap();
        close(f.coeffs[1], 3.22743696);
        close(f.coeffs[0], -2.33480805);
        close(f.r2.unwrap(), 0.9604111504831553);
        close(f.eval(4.0), 3.22743696 * 4.0 - 2.33480805);
    }

    #[test]
    fn polynomial_matches_numpy() {
        let f = fit(TrendKind::Polynomial { order: 3 }, &X, &Y).unwrap();
        for (c, r) in f.coeffs.iter().zip([1.36883131, 0.92782796, 0.22570534, 0.00198718]) {
            assert!((c - r).abs() < 1e-7, "{c} {r}");
        }
        close(f.r2.unwrap(), 0.9979527919743844);
    }

    #[test]
    fn power_exponential_logarithmic_match_numpy() {
        let p = fit(TrendKind::Power, &X, &Y).unwrap();
        close(p.coeffs[0], 2.5657769862461954);
        close(p.coeffs[1], 0.9951005044212502);
        close(p.r2.unwrap(), 0.9497529287785794); // on ln y
        let e = fit(TrendKind::Exponential, &X, &Y).unwrap();
        close(e.coeffs[0], 2.1204629460955036);
        close(e.coeffs[1], 0.31521060652145116);
        close(e.r2.unwrap(), 0.9664837362703195); // on ln y
        let l = fit(TrendKind::Logarithmic, &X, &Y).unwrap();
        close(l.coeffs[0], 1.1674064272099731);
        close(l.coeffs[1], 8.887756195038989);
        close(l.r2.unwrap(), 0.7181410738734294); // on y
        close(l.eval(2.0), 1.1674064272099731 + 8.887756195038989 * 2.0_f64.ln());
        assert!(l.eval(-1.0).is_nan());
    }

    #[test]
    fn far_from_zero_x_stays_accurate() {
        let x: Vec<f64> = (0..11).map(|i| 100000.0 + f64::from(i)).collect();
        let d = [0.1, -0.2, 0.15, 0.0, 0.05, -0.1, 0.2, -0.15, 0.1, 0.0, -0.05];
        let y: Vec<f64> = x.iter().zip(d).map(|(x, d)| 3.5 + 0.25 * (x - 100000.0) + d).collect();
        let f = fit(TrendKind::Linear, &x, &y).unwrap();
        assert!((f.coeffs[1] - 0.24772727272727).abs() < 1e-7);
        close(f.r2.unwrap(), 0.9770559210526315);
        assert!((f.eval(100005.0) - (3.52045455 + 0.24772727 * 5.0)).abs() < 1e-6);
    }

    #[test]
    fn transformed_fits_skip_unusable_points() {
        let x = [-1.0, 0.0, 1.0, 2.0, 4.0, f64::NAN];
        let y = [5.0, 3.0, 2.0, 4.0, 8.0, 1.0];
        // power needs x > 0 and y > 0: x = 1, 2, 4
        assert_eq!(fit(TrendKind::Power, &x, &y).unwrap().n, 3);
        assert_eq!(fit(TrendKind::Logarithmic, &x, &y).unwrap().n, 3);
        assert_eq!(fit(TrendKind::Exponential, &x, &y).unwrap().n, 5);
        assert_eq!(fit(TrendKind::Linear, &x, &y).unwrap().n, 5);
        // too few points, equal x, a moving average
        assert!(fit(TrendKind::Linear, &[1.0], &[1.0]).is_none());
        assert!(fit(TrendKind::Linear, &[2.0, 2.0, 2.0], &[1.0, 2.0, 3.0]).is_none());
        assert!(fit(TrendKind::Polynomial { order: 3 }, &[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]).is_none());
        assert!(fit(TrendKind::MovingAverage { window: 2 }, &x, &y).is_none());
    }

    #[test]
    fn moving_average_is_trailing_over_sorted_x() {
        let ma = moving_average(&X, &Y, 3);
        let want = [2.7333333333333, 3.4666666666667, 4.9333333333333, 6.3666666666667, 8.5, 10.2333333333333, 12.8666666666667, 15.8666666666667, 20.1, 25.0666666666667];
        assert_eq!(ma.len(), 10);
        for (p, w) in ma.iter().zip(want) {
            assert!((p.1 - w).abs() < 1e-9);
        }
        assert_eq!(ma[0].0, 1.7);
        // unsorted input gives the same result; NaN is dropped
        let xs = [3.0, 1.0, f64::NAN, 2.0];
        let ys = [30.0, 10.0, 99.0, 20.0];
        assert_eq!(moving_average(&xs, &ys, 2), vec![(2.0, 15.0), (3.0, 25.0)]);
        assert!(moving_average(&xs, &ys, 4).is_empty());
    }

    #[test]
    fn equations_and_numbers() {
        assert_eq!(sig_str(1.23456, 4), "1.235");
        assert_eq!(sig_str(0.56, 4), "0.56");
        assert_eq!(sig_str(1234567.0, 4), "1.235\u{d7}10\\+(6)");
        assert_eq!(sig_str(0.0000123456, 4), "1.235\u{d7}10\\+(-5)");
        assert_eq!(sig_str(100.0, 4), "100");
        assert_eq!(fit(TrendKind::Linear, &X, &Y).unwrap().equation(4), "y = 3.227x - 2.335");
        assert_eq!(
            fit(TrendKind::Polynomial { order: 3 }, &X, &Y).unwrap().equation(4),
            "y = 0.001987x\\+(3) + 0.2257x\\+(2) + 0.9278x + 1.369"
        );
        assert_eq!(fit(TrendKind::Power, &X, &Y).unwrap().equation(4), "y = 2.566x\\+(0.9951)");
        assert_eq!(fit(TrendKind::Exponential, &X, &Y).unwrap().equation(4), "y = 2.12e\\+(0.3152x)");
        assert_eq!(fit(TrendKind::Logarithmic, &X, &Y).unwrap().equation(4), "y = 8.888ln(x) + 1.167");
    }

    #[test]
    fn kind_names_round_trip() {
        for k in [
            TrendKind::Linear,
            TrendKind::Polynomial { order: 4 },
            TrendKind::Power,
            TrendKind::Logarithmic,
            TrendKind::Exponential,
            TrendKind::MovingAverage { window: 7 },
        ] {
            assert_eq!(TrendKind::parse(&k.name()), Ok(k));
        }
        assert_eq!(TrendKind::parse("poly"), Ok(TrendKind::Polynomial { order: 2 }));
        assert!(TrendKind::parse("poly:11").is_err());
        assert!(TrendKind::parse("ma:1").is_err());
        assert!(TrendKind::parse("cubic").is_err());
    }
}
