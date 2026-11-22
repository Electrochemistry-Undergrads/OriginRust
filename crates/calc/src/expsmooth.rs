//! Exponential smoothing forecasts (Origin 2024b's Time Series Analysis app, "forecast future observations"; not
//! in Origin 2016, so nothing here is measured in Origin; `docs/spec/statistics.md` section 42).
//!
//! Simple, Holt (linear trend) and Holt-Winters (additive or multiplicative season of period `m`) smoothing with the
//! classical recursions and the initial states of statsmodels' "legacy-heuristic":
//!
//! - initial level `y_0` (simple, Holt) or the mean of the first period (Holt-Winters); initial trend `y_1 - y_0`
//!   (Holt) or the mean of `(y_{m+j} - y_j) / m` (Holt-Winters); initial seasonals `y_j - l_0` or `y_j / l_0`;
//! - the one-step forecast of `y_t` is `l + b (+ s)` or `(l + b) s` from the states before `y_t`;
//! - `l' = a (y - s) + (1 - a)(l + b)` (or `a y / s + ...`), `b' = beta (l' - l) + (1 - beta) b`,
//!   `s' = gamma (y - l - b) + (1 - gamma) s` (or `gamma y / (l + b) + ...`).
//!
//! The smoothing constants are given or fitted by least squares of the one-step errors on `[0, 1]` (Nelder-Mead from
//! several starts, kept inside the box). Checked against numpy and scipy in `crates/calc/tests/expsmooth_reference.rs`
//! (made by `tools/expsmooth-ref.py`).

/// The smoothing model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SmoothModel {
    /// Level only.
    Simple,
    /// Level and linear trend (default).
    #[default]
    Holt,
    /// Level, trend and an additive season.
    WintersAdditive,
    /// Level, trend and a multiplicative season.
    WintersMultiplicative,
}

impl SmoothModel {
    pub fn name(self) -> &'static str {
        match self {
            SmoothModel::Simple => "simple",
            SmoothModel::Holt => "Holt linear",
            SmoothModel::WintersAdditive => "additive Holt-Winters",
            SmoothModel::WintersMultiplicative => "multiplicative Holt-Winters",
        }
    }

    /// The number of smoothing constants.
    pub fn constants(self) -> usize {
        match self {
            SmoothModel::Simple => 1,
            SmoothModel::Holt => 2,
            _ => 3,
        }
    }

    fn seasonal(self) -> bool {
        matches!(self, SmoothModel::WintersAdditive | SmoothModel::WintersMultiplicative)
    }
}

/// A smoothed series.
#[derive(Debug, Clone, PartialEq)]
pub struct Smoothed {
    /// `[alpha, beta, gamma]` as far as the model uses them.
    pub constants: Vec<f64>,
    /// One-step forecasts of each value.
    pub fitted: Vec<f64>,
    pub sse: f64,
    /// The states after the last value: level, trend (0 for simple), and the last `m` seasonals in time order.
    pub level: f64,
    pub trend: f64,
    pub seasonals: Vec<f64>,
    /// Forecasts `1..=h` steps after the last value.
    pub forecast: Vec<f64>,
}

/// Why the series could not be smoothed.
#[derive(Debug, Clone, PartialEq)]
pub enum SmoothError {
    TooShort(usize),
    Missing(usize),
    NotPositive(usize),
    BadPeriod(usize),
    BadConstant(f64),
    WrongConstants(usize),
}

impl std::fmt::Display for SmoothError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SmoothError::TooShort(n) => write!(f, "{n} values are too few (Holt needs 3, Holt-Winters two periods)"),
            SmoothError::Missing(i) => write!(f, "value {} is missing: the series must be complete", i + 1),
            SmoothError::NotPositive(i) => {
                write!(f, "value {} is not above 0, which the multiplicative season needs", i + 1)
            }
            SmoothError::BadPeriod(m) => write!(f, "the period must be at least 2 (got {m})"),
            SmoothError::BadConstant(v) => write!(f, "a smoothing constant must lie in [0, 1] (got {v})"),
            SmoothError::WrongConstants(k) => write!(f, "this model takes {k} smoothing constant(s)"),
        }
    }
}

impl std::error::Error for SmoothError {}

/// Runs the recursions with the constants `c`; `None` when a state stops being finite.
fn run(y: &[f64], model: SmoothModel, m: usize, c: &[f64], h: usize) -> Option<Smoothed> {
    let n = y.len();
    let (a, beta, gamma) = (c[0], c.get(1).copied().unwrap_or(0.0), c.get(2).copied().unwrap_or(0.0));
    let mult = model == SmoothModel::WintersMultiplicative;
    let (mut l, mut b, mut s): (f64, f64, Vec<f64>) = match model {
        SmoothModel::Simple => (y[0], 0.0, Vec::new()),
        SmoothModel::Holt => (y[0], y[1] - y[0], Vec::new()),
        _ => {
            let l0 = y[..m].iter().sum::<f64>() / m as f64;
            let b0 = (0..m).map(|j| (y[m + j] - y[j]) / m as f64).sum::<f64>() / m as f64;
            let s0 = y[..m].iter().map(|v| if mult { v / l0 } else { v - l0 }).collect();
            (l0, b0, s0)
        }
    };
    let mut fitted = Vec::with_capacity(n);
    let mut sse = 0.0;
    for (t, &v) in y.iter().enumerate() {
        let season = if model.seasonal() {
            s[t % m]
        } else if mult {
            1.0
        } else {
            0.0
        };
        let f = if mult { (l + b) * season } else { l + b + season };
        fitted.push(f);
        sse += (v - f).powi(2);
        let new_l = match model {
            SmoothModel::Simple => a * v + (1.0 - a) * l,
            SmoothModel::Holt => a * v + (1.0 - a) * (l + b),
            SmoothModel::WintersAdditive => a * (v - season) + (1.0 - a) * (l + b),
            SmoothModel::WintersMultiplicative => a * v / season + (1.0 - a) * (l + b),
        };
        if model.seasonal() {
            s[t % m] = if mult {
                gamma * v / (l + b) + (1.0 - gamma) * season
            } else {
                gamma * (v - l - b) + (1.0 - gamma) * season
            };
        }
        if model != SmoothModel::Simple {
            b = beta * (new_l - l) + (1.0 - beta) * b;
        }
        l = new_l;
        if !(l.is_finite() && b.is_finite()) {
            return None;
        }
    }
    let forecast = (1..=h)
        .map(|k| {
            let kf = k as f64;
            match model {
                SmoothModel::Simple => l,
                SmoothModel::Holt => l + kf * b,
                SmoothModel::WintersAdditive => l + kf * b + s[(n + k - 1) % m],
                SmoothModel::WintersMultiplicative => (l + kf * b) * s[(n + k - 1) % m],
            }
        })
        .collect();
    let seasonals = if model.seasonal() { (0..m).map(|j| s[(n + j) % m]).collect() } else { Vec::new() };
    Some(Smoothed { constants: c.to_vec(), fitted, sse, level: l, trend: b, seasonals, forecast })
}

/// Nelder-Mead on the box `[0, 1]^k` (points clamped into it) from `start`.
fn nelder_mead(f: &dyn Fn(&[f64]) -> f64, start: &[f64]) -> (Vec<f64>, f64) {
    let k = start.len();
    let clamp = |p: Vec<f64>| p.into_iter().map(|v| v.clamp(0.0, 1.0)).collect::<Vec<f64>>();
    let mut pts: Vec<Vec<f64>> = vec![start.to_vec()];
    for i in 0..k {
        let mut p = start.to_vec();
        p[i] = if p[i] + 0.1 <= 1.0 { p[i] + 0.1 } else { p[i] - 0.1 };
        pts.push(p);
    }
    let mut vals: Vec<f64> = pts.iter().map(|p| f(p)).collect();
    for _ in 0..2000 {
        let mut order: Vec<usize> = (0..=k).collect();
        order.sort_by(|&i, &j| vals[i].total_cmp(&vals[j]));
        pts = order.iter().map(|&i| pts[i].clone()).collect();
        vals = order.iter().map(|&i| vals[i]).collect();
        let spread = pts.iter().flat_map(|p| p.iter().zip(&pts[0]).map(|(a, b)| (a - b).abs())).fold(0.0, f64::max);
        if spread < 1e-11 && (vals[k] - vals[0]).abs() <= 1e-15 * vals[0].abs().max(1e-300) {
            break;
        }
        let centroid: Vec<f64> = (0..k).map(|d| pts[..k].iter().map(|p| p[d]).sum::<f64>() / k as f64).collect();
        let along = |t: f64| clamp(centroid.iter().zip(&pts[k]).map(|(c, w)| c + t * (c - w)).collect());
        let r = along(1.0);
        let fr = f(&r);
        if fr < vals[0] {
            let e = along(2.0);
            let fe = f(&e);
            if fe < fr {
                pts[k] = e;
                vals[k] = fe;
            } else {
                pts[k] = r;
                vals[k] = fr;
            }
        } else if fr < vals[k - 1] {
            pts[k] = r;
            vals[k] = fr;
        } else {
            let cpt = if fr < vals[k] { along(0.5) } else { along(-0.5) };
            let fc = f(&cpt);
            if fc < vals[k].min(fr) {
                pts[k] = cpt;
                vals[k] = fc;
            } else {
                for i in 1..=k {
                    pts[i] = clamp(pts[i].iter().zip(&pts[0]).map(|(p, b)| b + 0.5 * (p - b)).collect());
                    vals[i] = f(&pts[i]);
                }
            }
        }
    }
    let best = (0..=k).min_by(|&i, &j| vals[i].total_cmp(&vals[j])).unwrap_or(0);
    (pts[best].clone(), vals[best])
}

/// Smooths `y` with `model` (period `m` for Holt-Winters) and forecasts `h` steps; `constants` fixes the smoothing
/// constants, `None` fits them.
pub fn smooth(
    y: &[f64],
    model: SmoothModel,
    m: usize,
    constants: Option<&[f64]>,
    h: usize,
) -> Result<Smoothed, SmoothError> {
    if let Some(i) = y.iter().position(|v| !v.is_finite()) {
        return Err(SmoothError::Missing(i));
    }
    if model.seasonal() && m < 2 {
        return Err(SmoothError::BadPeriod(m));
    }
    let need = match model {
        SmoothModel::Simple => 2,
        SmoothModel::Holt => 3,
        _ => 2 * m,
    };
    if y.len() < need {
        return Err(SmoothError::TooShort(y.len()));
    }
    if model == SmoothModel::WintersMultiplicative
        && let Some(i) = y.iter().position(|v| *v <= 0.0)
    {
        return Err(SmoothError::NotPositive(i));
    }
    let k = model.constants();
    if let Some(c) = constants {
        if c.len() != k {
            return Err(SmoothError::WrongConstants(k));
        }
        if let Some(v) = c.iter().find(|v| !(0.0..=1.0).contains(*v)) {
            return Err(SmoothError::BadConstant(*v));
        }
        return run(y, model, m, c, h).ok_or(SmoothError::BadConstant(c[0]));
    }
    let sse = |c: &[f64]| run(y, model, m, c, 0).map_or(f64::INFINITY, |s| s.sse);
    let grid = [0.1, 0.5, 0.9];
    let mut best = (vec![0.5; k], f64::INFINITY);
    let mut starts: Vec<Vec<f64>> = vec![vec![]];
    for _ in 0..k {
        starts = starts.into_iter().flat_map(|s| grid.iter().map(move |g| [s.clone(), vec![*g]].concat())).collect();
    }
    for s in starts {
        let (p, v) = nelder_mead(&sse, &s);
        if v < best.1 {
            best = (p, v);
        }
    }
    // A last run from the best point settles it.
    let (p, _) = nelder_mead(&sse, &best.0);
    run(y, model, m, &p, h).ok_or(SmoothError::BadConstant(p[0]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holt_forecasts_go_on_along_the_last_trend() {
        let y: Vec<f64> = (0..10).map(|i| 2.0 + 3.0 * f64::from(i)).collect();
        let s = smooth(&y, SmoothModel::Holt, 0, Some(&[0.3, 0.1]), 3).unwrap();
        // The states start before y_0 (level y_0, trend y_1 - y_0), so the first one-step forecast is y_1.
        assert!((s.fitted[0] - 5.0).abs() < 1e-12);
        assert!((s.forecast[0] - (s.level + s.trend)).abs() < 1e-12);
        assert!((s.forecast[2] - s.forecast[1] - s.trend).abs() < 1e-12);
        let simple = smooth(&[1.0, 2.0, 3.0], SmoothModel::Simple, 0, Some(&[0.5]), 1).unwrap();
        assert_eq!(simple.fitted, [1.0, 1.0, 1.5]);
        assert!((simple.forecast[0] - 2.25).abs() < 1e-15);
    }

    #[test]
    fn fitted_constants_beat_a_grid_of_fixed_ones() {
        let y: Vec<f64> = (0..24).map(|i| 10.0 + 0.2 * f64::from(i) + [2.0, -1.0, 0.5, -1.5][i as usize % 4]).collect();
        let fit = smooth(&y, SmoothModel::WintersAdditive, 4, None, 4).unwrap();
        for a in [0.1, 0.5, 0.9] {
            for g in [0.1, 0.5, 0.9] {
                let fixed = smooth(&y, SmoothModel::WintersAdditive, 4, Some(&[a, 0.1, g]), 0).unwrap();
                assert!(fit.sse <= fixed.sse + 1e-12, "{} vs {} at {a}, {g}", fit.sse, fixed.sse);
            }
        }
        assert_eq!((fit.forecast.len(), fit.seasonals.len()), (4, 4));
    }

    #[test]
    fn bad_input_is_refused() {
        assert_eq!(smooth(&[1.0, 2.0], SmoothModel::Holt, 0, None, 1), Err(SmoothError::TooShort(2)));
        assert_eq!(smooth(&[1.0; 8], SmoothModel::WintersAdditive, 1, None, 1), Err(SmoothError::BadPeriod(1)));
        assert_eq!(
            smooth(&[1.0, 0.0, 1.0, 2.0], SmoothModel::WintersMultiplicative, 2, None, 1),
            Err(SmoothError::NotPositive(1))
        );
        assert_eq!(
            smooth(&[1.0, 2.0, 3.0], SmoothModel::Holt, 0, Some(&[0.5]), 1),
            Err(SmoothError::WrongConstants(2))
        );
        assert_eq!(
            smooth(&[1.0, 2.0, 3.0], SmoothModel::Simple, 0, Some(&[1.5]), 1),
            Err(SmoothError::BadConstant(1.5))
        );
    }
}
