//! Nonlinear least squares (Levenberg-Marquardt) with bounds and fixed parameters.
//! See `docs/spec/fitting.md`.
//!
//! A [`FitFunction`] describes the model: parameter names, a formula in the `sheaf-expr`
//! language or a Rust closure, an optional routine that guesses starting values from the data,
//! and for each parameter an optional lower and upper bound and a "fixed" flag. [`fit`] minimises
//! the (weighted) sum of squared residuals and returns the shared [`FitReport`].
//!
//! Numerical details that matter for matching Origin:
//! - The covariance matrix of the parameters is `(J' W J)^-1`, multiplied by the reduced
//!   chi-square when errors are scaled (Origin's default, also for weighted fits).
//! - A parameter sitting on a bound stays a free parameter in the statistics (it counts in the
//!   degrees of freedom and gets a standard error); a warning names it.
//! - Points with a missing x, y or weight are dropped.

use std::sync::Arc;

use nalgebra::{DMatrix, DVector};
use sheaf_expr::{Expr, Host};

use crate::dist::{f_upper_p, t_critical, t_two_sided_p};
use crate::report::{Anova, FitError, FitReport, FitStatus, Parameter, Statistics, Weighting};

/// A model evaluated by Rust code: `(parameter values, x) -> y`.
pub type ModelFn = Arc<dyn Fn(&[f64], f64) -> f64 + Send + Sync>;

/// Guesses starting parameter values from the data: `(x, y) -> one value per parameter`. The
/// data passed in have the missing points already removed. A NaN in the result means "no
/// guess; use 1".
pub type InitFn = Arc<dyn Fn(&[f64], &[f64]) -> Vec<f64> + Send + Sync>;

/// How the model is computed.
#[derive(Clone)]
pub enum Model {
    /// A formula in the expression language. The parameters and the independent variable are
    /// variables of the formula.
    Formula(Expr),
    /// Rust code.
    Closure(ModelFn),
}

/// One parameter of a [`FitFunction`] with its constraints.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamSpec {
    pub name: String,
    /// Starting value. NaN means "use the function's initial-guess routine, else 1".
    pub initial: f64,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    /// Held at `initial` and not fitted.
    pub fixed: bool,
}

impl ParamSpec {
    /// A free, unbounded parameter without a starting value.
    pub fn new(name: &str) -> Self {
        ParamSpec { name: name.to_string(), initial: f64::NAN, lower: None, upper: None, fixed: false }
    }
}

/// Checks parameter names of a user-written function: none may repeat (names are case
/// sensitive, as in the expression language), none may equal the independent variable, and none
/// may be a constant of the expression language (`pi` and `e`, in any case), because the
/// formula would read the constant and the parameter would never be used.
pub fn check_param_names<'a>(x_name: &str, names: impl IntoIterator<Item = &'a str>) -> Result<(), String> {
    let mut seen: Vec<&str> = Vec::new();
    for n in names {
        if n == x_name {
            return Err(format!("the parameter '{n}' has the same name as the independent variable"));
        }
        if n.eq_ignore_ascii_case("pi") || n.eq_ignore_ascii_case("e") {
            return Err(format!(
                "'{n}' cannot be a parameter name: the formula language reads it as the constant {}",
                if n.eq_ignore_ascii_case("pi") { "pi" } else { "e" }
            ));
        }
        if seen.contains(&n) {
            return Err(format!("the parameter name '{n}' is used twice"));
        }
        seen.push(n);
    }
    Ok(())
}

/// A fit function: the model and everything known about its parameters.
#[derive(Clone)]
pub struct FitFunction {
    /// Name shown in the report, for example `Gauss` or `User defined`.
    pub name: String,
    /// The model written as mathematics, for example `y = y0 + A*exp(-x/t)`.
    pub equation: String,
    /// Name of the independent variable in the formula (`x`).
    pub x_name: String,
    pub params: Vec<ParamSpec>,
    pub model: Model,
    pub init: Option<InitFn>,
}

impl std::fmt::Debug for FitFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FitFunction")
            .field("name", &self.name)
            .field("equation", &self.equation)
            .field("params", &self.params)
            .finish_non_exhaustive()
    }
}

impl FitFunction {
    /// A user-defined function from a formula such as `y0 + A*exp(-x/t)` (the right-hand side
    /// only; a leading `y =` is accepted and removed) and its parameter names. The independent
    /// variable is `x`.
    pub fn from_formula(name: &str, formula: &str, params: &[&str]) -> Result<Self, FitError> {
        let rhs = strip_lhs(formula);
        let expr = Expr::parse(&rhs).map_err(|e| FitError::BadFunction(e.to_string()))?;
        check_param_names("x", params.iter().copied()).map_err(FitError::BadFunction)?;
        for v in expr.variables() {
            if v != "x" && !params.iter().any(|p| *p == v) {
                return Err(FitError::BadFunction(format!(
                    "the formula uses '{v}', which is neither x nor a parameter"
                )));
            }
        }
        let used = expr.variables();
        if let Some(p) = params.iter().find(|p| !used.iter().any(|v| v == *p)) {
            return Err(FitError::BadFunction(format!("the parameter '{p}' does not appear in the formula")));
        }
        Ok(FitFunction {
            name: name.to_string(),
            equation: format!("y = {rhs}"),
            x_name: "x".to_string(),
            params: params.iter().map(|p| ParamSpec::new(p)).collect(),
            model: Model::Formula(expr),
            init: None,
        })
    }

    /// A function computed by Rust code.
    pub fn from_closure(name: &str, equation: &str, params: &[&str], f: ModelFn) -> Self {
        FitFunction {
            name: name.to_string(),
            equation: equation.to_string(),
            x_name: "x".to_string(),
            params: params.iter().map(|p| ParamSpec::new(p)).collect(),
            model: Model::Closure(f),
            init: None,
        }
    }

    /// Sets the starting value of a parameter (builder style). Unknown names are ignored.
    pub fn with_initial(mut self, name: &str, value: f64) -> Self {
        if let Some(p) = self.params.iter_mut().find(|p| p.name == name) {
            p.initial = value;
        }
        self
    }

    /// Sets bounds of a parameter (builder style).
    pub fn with_bounds(mut self, name: &str, lower: Option<f64>, upper: Option<f64>) -> Self {
        if let Some(p) = self.params.iter_mut().find(|p| p.name == name) {
            p.lower = lower;
            p.upper = upper;
        }
        self
    }

    /// Holds a parameter at `value` (builder style).
    pub fn with_fixed(mut self, name: &str, value: f64) -> Self {
        if let Some(p) = self.params.iter_mut().find(|p| p.name == name) {
            p.initial = value;
            p.fixed = true;
        }
        self
    }

    /// The model value at `x` for the given parameter values (in `params` order). NaN if the
    /// formula cannot be evaluated.
    pub fn eval(&self, params: &[f64], x: f64) -> f64 {
        match &self.model {
            Model::Closure(f) => f(params, x),
            Model::Formula(e) => {
                let host = PointHost { names: &self.params, vals: params, x_name: &self.x_name, x };
                e.eval(&host).unwrap_or(f64::NAN)
            }
        }
    }
}

/// Removes a leading `name =` from a formula typed as an equation.
fn strip_lhs(formula: &str) -> String {
    let t = formula.trim();
    if let Some(eq) = t.find('=') {
        let (lhs, rest) = t.split_at(eq);
        let rest_ok = !rest.starts_with("==");
        let lhs_ok = !lhs.trim().is_empty()
            && lhs.trim().chars().all(|c| c.is_alphanumeric() || c == '_' || c == '(' || c == ')');
        if rest_ok && lhs_ok {
            return rest[1..].trim().to_string();
        }
    }
    t.to_string()
}

struct PointHost<'a> {
    names: &'a [ParamSpec],
    vals: &'a [f64],
    x_name: &'a str,
    x: f64,
}

impl Host for PointHost<'_> {
    fn variable(&self, name: &str) -> Option<f64> {
        if name == self.x_name {
            return Some(self.x);
        }
        self.names.iter().position(|p| p.name == name).map(|i| self.vals[i])
    }
    fn cell(&self, _col: &sheaf_expr::ColumnRef, _row: usize) -> Option<f64> {
        None
    }
}

/// The iteration algorithm of a nonlinear fit (Origin's "Iteration Algorithm" box).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Algorithm {
    /// Levenberg-Marquardt (Origin's default).
    #[default]
    LevenbergMarquardt,
    /// The Nelder-Mead simplex method: no derivatives; a bound is kept by clamping every vertex.
    /// One iteration is [`SIMPLEX_STEPS_PER_ITERATION`] simplex steps. See `docs/spec/fitting.md`.
    Simplex,
}

impl Algorithm {
    pub const ALL: [Algorithm; 2] = [Algorithm::LevenbergMarquardt, Algorithm::Simplex];

    /// The name in Origin's Iteration Algorithm box.
    pub fn label(self) -> &'static str {
        match self {
            Algorithm::LevenbergMarquardt => "Levenberg Marquardt",
            Algorithm::Simplex => "Simplex",
        }
    }
}

/// Simplex steps (one reflection, expansion, contraction or shrink each) that one iteration of
/// [`Algorithm::Simplex`] makes. Origin does not document its unit; with this value the default
/// limit of 400 iterations is ample.
pub const SIMPLEX_STEPS_PER_ITERATION: usize = 10;

/// Settings of a fit. The defaults are Origin's: tolerance 1e-9, at most 400 iterations,
/// 95 % confidence limits, errors scaled by the reduced chi-square (also with instrumental weights).
#[derive(Debug, Clone)]
pub struct FitOptions {
    pub weighting: Weighting,
    pub max_iterations: usize,
    /// Convergence tolerance on the relative decrease of the (weighted) sum of squares.
    pub tolerance: f64,
    pub confidence_level: f64,
    /// Scale the covariance matrix by the reduced chi-square. `None`: on, as in Origin (also for
    /// instrumental weighting).
    pub scale_errors: Option<bool>,
    pub algorithm: Algorithm,
}

impl Default for FitOptions {
    fn default() -> Self {
        FitOptions {
            weighting: Weighting::None,
            max_iterations: 400,
            tolerance: 1e-9,
            confidence_level: 0.95,
            scale_errors: None,
            algorithm: Algorithm::LevenbergMarquardt,
        }
    }
}

/// Fits `func` to the points `(x, y)`: runs [`Fitter`] until it converges or `max_iterations`
/// iterations were made.
pub fn fit(func: &FitFunction, x: &[f64], y: &[f64], opts: &FitOptions) -> Result<FitReport, FitError> {
    let mut f = Fitter::new(func, x, y, opts)?;
    for _ in 0..opts.max_iterations {
        if f.step().converged {
            break;
        }
    }
    Ok(f.finish())
}

/// The state after one iteration of a [`Fitter`].
#[derive(Debug, Clone, PartialEq)]
pub struct StepOutcome {
    /// Iterations made so far.
    pub iteration: usize,
    /// The (weighted) sum of squared residuals at `params`.
    pub chi2: f64,
    /// Reduced chi-square (`chi2` over the degrees of freedom); NaN with no degrees of freedom.
    pub reduced_chi2: f64,
    /// The damping factor the next iteration starts from.
    pub lambda: f64,
    /// The fit has reached its tolerance; further steps change nothing.
    pub converged: bool,
    /// Parameter values, one per parameter of the function (fixed ones included).
    pub params: Vec<f64>,
}

/// A nonlinear fit that can be advanced one Levenberg-Marquardt iteration at a time, so that a
/// dialog can show the curve move. [`fit`] is a loop over it. Bounds and fixed parameters
/// behave exactly as in [`fit`].
pub struct Fitter<'a> {
    func: &'a FitFunction,
    opts: FitOptions,
    n_all: usize,
    /// Indices of the input points that take part.
    used: Vec<usize>,
    xs: Vec<f64>,
    ys: Vec<f64>,
    ws: Vec<f64>,
    sw: Vec<f64>,
    /// Indices of the free parameters.
    free: Vec<usize>,
    p: Vec<f64>,
    cost: f64,
    lambda: f64,
    iterations: usize,
    converged: bool,
    warnings: Vec<String>,
    simplex: Option<Simplex>,
}

impl<'a> Fitter<'a> {
    /// Prepares a fit: checks the function, drops invalid points and sets the starting values.
    pub fn new(func: &'a FitFunction, x: &[f64], y: &[f64], opts: &FitOptions) -> Result<Self, FitError> {
        if x.len() != y.len() {
            return Err(FitError::LengthMismatch { x: x.len(), y: y.len() });
        }
        let n_all = x.len();
        let mut warnings = Vec::new();

        for s in &func.params {
            if s.fixed && !s.initial.is_finite() {
                return Err(FitError::BadFunction(format!(
                    "parameter {} is fixed at a value that is not a finite number",
                    s.name
                )));
            }
            if s.lower.is_some_and(f64::is_nan) || s.upper.is_some_and(f64::is_nan) {
                return Err(FitError::BadFunction(format!("parameter {} has a bound that is not a number", s.name)));
            }
            if let (Some(lo), Some(hi)) = (s.lower, s.upper)
                && lo > hi
            {
                return Err(FitError::BadFunction(format!(
                    "parameter {} has its lower bound ({lo}) above its upper bound ({hi})",
                    s.name
                )));
            }
        }

        // Weights, one per input point (sqrt applied later).
        let raw_w: Vec<f64> = match &opts.weighting {
            Weighting::None => vec![1.0; n_all],
            Weighting::Instrumental(err) => {
                check_len(err.len(), n_all)?;
                err.iter().map(|e| if *e > 0.0 { 1.0 / (e * e) } else { f64::NAN }).collect()
            }
            Weighting::Statistical => {
                // Same rule as the linear fits: w = 1 / |y| (Origin's result for a negative y equals
                // this), and a y of exactly zero cannot be weighted.
                if y.contains(&0.0) {
                    return Err(FitError::Failed(
                        "statistical weighting needs y different from zero (the weight is 1/|y|); \
                         a point with y = 0 cannot be weighted"
                            .to_string(),
                    ));
                }
                y.iter().map(|v| 1.0 / v.abs()).collect()
            }
            Weighting::Direct(w) => {
                check_len(w.len(), n_all)?;
                // A weight of zero means the point does not take part in the fit; it is left out
                // like a missing value so that it does not count in the degrees of freedom.
                w.iter().map(|w| if *w > 0.0 { *w } else { f64::NAN }).collect()
            }
        };
        let used: Vec<usize> =
            (0..n_all).filter(|&i| x[i].is_finite() && y[i].is_finite() && raw_w[i].is_finite()).collect();
        if used.len() < n_all {
            warnings.push(format!(
                "{} point(s) with a missing or invalid x, y or weight were left out",
                n_all - used.len()
            ));
        }
        let xs: Vec<f64> = used.iter().map(|&i| x[i]).collect();
        let ys: Vec<f64> = used.iter().map(|&i| y[i]).collect();
        let ws: Vec<f64> = used.iter().map(|&i| raw_w[i]).collect();
        let sw: Vec<f64> = ws.iter().map(|w| w.sqrt()).collect();
        let n = xs.len();

        let np = func.params.len();
        let free: Vec<usize> = (0..np).filter(|&j| !func.params[j].fixed).collect();
        let nfree = free.len();
        if n < nfree || nfree == 0 && n == 0 {
            return Err(FitError::TooFewPoints { have: n, need: nfree });
        }

        // Starting values: explicit, else from the guess routine, else 1.
        let guess = func.init.as_ref().map(|f| f(&xs, &ys)).unwrap_or_default();
        let mut p: Vec<f64> = (0..np)
            .map(|j| {
                let s = &func.params[j];
                if s.initial.is_finite() {
                    s.initial
                } else if guess.get(j).is_some_and(|g| g.is_finite()) {
                    guess[j]
                } else {
                    1.0
                }
            })
            .collect();
        for (j, s) in func.params.iter().enumerate() {
            if !s.fixed {
                if let Some(lo) = s.lower {
                    p[j] = p[j].max(lo);
                }
                if let Some(hi) = s.upper {
                    p[j] = p[j].min(hi);
                }
            }
        }

        let cost_of = |p: &[f64]| -> f64 {
            let mut c = 0.0;
            for i in 0..n {
                let r = sw[i] * (ys[i] - func.eval(p, xs[i]));
                c += r * r;
            }
            c
        };
        let cost = cost_of(&p);
        if !cost.is_finite() {
            if xs.iter().all(|&xv| func.eval(&p, xv).is_finite()) {
                return Err(FitError::Failed(
                    "the sum of squared residuals is too large to be computed (it overflows); scale the data".into(),
                ));
            }
            return Err(FitError::Failed(
                "the model is not defined (gives a missing value) at the starting parameters".into(),
            ));
        }

        Ok(Fitter {
            func,
            opts: opts.clone(),
            n_all,
            used,
            xs,
            ys,
            ws,
            sw,
            free,
            p,
            cost,
            // Origin's damping: the diagonal of J'J is scaled by (1 + lambda). Lambda starts at 1e-3,
            // is divided by 10 after every step that does not raise the sum of squares and
            // multiplied by 10 after a step that does. See docs/spec/fitting.md.
            lambda: INITIAL_DAMPING,
            iterations: 0,
            converged: nfree == 0,
            warnings,
            simplex: None,
        })
    }

    fn cost_of(&self, p: &[f64]) -> f64 {
        let mut c = 0.0;
        for i in 0..self.xs.len() {
            let r = self.sw[i] * (self.ys[i] - self.func.eval(p, self.xs[i]));
            c += r * r;
        }
        c
    }

    fn outcome(&self) -> StepOutcome {
        let dof = self.xs.len().saturating_sub(self.free.len());
        StepOutcome {
            iteration: self.iterations,
            chi2: self.cost,
            reduced_chi2: if dof > 0 { self.cost / dof as f64 } else { f64::NAN },
            lambda: self.lambda,
            converged: self.converged,
            params: self.p.clone(),
        }
    }

    /// The current parameter values.
    pub fn params(&self) -> &[f64] {
        &self.p
    }

    /// Makes one iteration (a no-op once converged) and returns the state after it.
    pub fn step(&mut self) -> StepOutcome {
        if self.converged {
            return self.outcome();
        }
        if self.opts.algorithm == Algorithm::Simplex {
            return self.simplex_iteration();
        }
        let func = self.func;
        let nfree = self.free.len();
        self.iterations += 1;
        let (jac, res) = jacobian(func, &self.p, &self.free, &self.xs, &self.ys, &self.sw);
        let g = jac.transpose() * &res;
        let a = jac.transpose() * &jac;
        let mut accepted = false;
        while self.lambda < 1e14 {
            let mut am = a.clone();
            for k in 0..nfree {
                am[(k, k)] += self.lambda * a[(k, k)].max(1e-300);
            }
            let step = match am.lu().solve(&g) {
                Some(s) if s.iter().all(|v| v.is_finite()) => s,
                _ => {
                    self.lambda *= 10.0;
                    continue;
                }
            };
            // The whole step is computed as if no parameter were bounded; each component is
            // then cut back to its bounds. A parameter held on its bound therefore still
            // takes part in the step of the others.
            let mut pn = self.p.clone();
            for (k, &j) in self.free.iter().enumerate() {
                let mut v = self.p[j] + step[k];
                if let Some(lo) = func.params[j].lower {
                    v = v.max(lo);
                }
                if let Some(hi) = func.params[j].upper {
                    v = v.min(hi);
                }
                pn[j] = v;
            }
            let cn = self.cost_of(&pn);
            if cn.is_finite() && cn <= self.cost {
                let prev = self.cost;
                self.lambda = (self.lambda / 10.0).max(1e-15);
                self.p = pn;
                self.cost = cn;
                accepted = true;
                if prev - cn <= self.opts.tolerance * (prev + cn) || self.cost == 0.0 {
                    self.converged = true;
                }
                break;
            }
            self.lambda *= 10.0;
        }
        if !accepted {
            // No step can lower the sum of squares any more: at the minimum to machine precision.
            self.converged = true;
        }
        self.outcome()
    }

    /// One iteration of the simplex algorithm: [`SIMPLEX_STEPS_PER_ITERATION`] Nelder-Mead steps.
    fn simplex_iteration(&mut self) -> StepOutcome {
        self.iterations += 1;
        if self.simplex.is_none() {
            let base: Vec<f64> = self.free.iter().map(|&j| self.p[j]).collect();
            self.simplex = Some(self.new_simplex(&base, self.cost));
        }
        let mut sx = self.simplex.take().expect("built above");
        for _ in 0..SIMPLEX_STEPS_PER_ITERATION {
            if !self.simplex_step(&mut sx) {
                // The simplex has collapsed onto a point. Start again from the best vertex; when
                // that no longer lowers the sum of squares the fit has converged.
                let best = sx.f[sx.order[0]];
                let improved = sx.last_restart.is_none_or(|prev| prev - best > SIMPLEX_RESTART_GAIN * best.abs());
                if !improved || best == 0.0 {
                    self.converged = true;
                    break;
                }
                let base = sx.pts[sx.order[0]].clone();
                sx = self.new_simplex(&base, best);
                sx.last_restart = Some(best);
            }
        }
        let b = sx.order[0];
        for (k, &j) in self.free.iter().enumerate() {
            self.p[j] = sx.pts[b][k];
        }
        self.cost = sx.f[b];
        self.simplex = Some(sx);
        self.outcome()
    }

    fn clamp_free(&self, v: &mut [f64]) {
        for (k, &j) in self.free.iter().enumerate() {
            if let Some(lo) = self.func.params[j].lower {
                v[k] = v[k].max(lo);
            }
            if let Some(hi) = self.func.params[j].upper {
                v[k] = v[k].min(hi);
            }
        }
    }

    fn cost_free(&self, v: &[f64]) -> f64 {
        let mut p = self.p.clone();
        for (k, &j) in self.free.iter().enumerate() {
            p[j] = v[k];
        }
        let c = self.cost_of(&p);
        if c.is_finite() { c } else { f64::INFINITY }
    }

    /// A simplex around `base` (whose cost is `base_cost`): one vertex moved by 20 % along each
    /// axis (inwards when that axis is held on a bound).
    fn new_simplex(&self, base: &[f64], base_cost: f64) -> Simplex {
        let n = base.len();
        let mut pts = vec![base.to_vec()];
        let mut f = vec![base_cost];
        for k in 0..n {
            let mut v = base.to_vec();
            let h = if base[k] != 0.0 { SIMPLEX_INIT_STEP * base[k].abs() } else { 0.00025 };
            v[k] += h;
            self.clamp_free(&mut v);
            if v[k] == base[k] {
                v[k] -= h;
                self.clamp_free(&mut v);
            }
            f.push(self.cost_free(&v));
            pts.push(v);
        }
        let mut sx = Simplex { pts, f, order: Vec::new(), last_restart: None };
        sx.sort();
        sx
    }

    /// One Nelder-Mead step with the adaptive coefficients of Gao and Han. False when the simplex
    /// has collapsed onto a point.
    fn simplex_step(&self, sx: &mut Simplex) -> bool {
        let n = sx.pts[0].len();
        if n == 0 {
            return false;
        }
        let nf = n as f64;
        let (best, worst) = (sx.order[0], sx.order[n]);
        let second_worst = sx.order[n - 1];
        let spread = sx.f[worst] - sx.f[best];
        let mut size = 0.0f64;
        for v in &sx.pts {
            for (a, b) in v.iter().zip(&sx.pts[best]) {
                size = size.max((a - b).abs() / b.abs().max(1e-12));
            }
        }
        if sx.f[best] == 0.0 || (spread <= SIMPLEX_F_TOL * sx.f[best].abs() && size <= SIMPLEX_X_TOL) {
            return false;
        }
        let mut centroid = vec![0.0; n];
        for &i in &sx.order[..n] {
            for (c, p) in centroid.iter_mut().zip(&sx.pts[i]) {
                *c += p / nf;
            }
        }
        let along = |t: f64| -> Vec<f64> {
            let mut v: Vec<f64> = (0..n).map(|k| centroid[k] + t * (sx.pts[worst][k] - centroid[k])).collect();
            self.clamp_free(&mut v);
            v
        };
        let (beta, gamma, delta) = (1.0 + 2.0 / nf, 0.75 - 1.0 / (2.0 * nf), 1.0 - 1.0 / nf);
        let xr = along(-1.0);
        let fr = self.cost_free(&xr);
        if fr < sx.f[best] {
            let xe = along(-beta);
            let fe = self.cost_free(&xe);
            if fe < fr {
                sx.replace(worst, xe, fe);
            } else {
                sx.replace(worst, xr, fr);
            }
        } else if fr < sx.f[second_worst] {
            sx.replace(worst, xr, fr);
        } else {
            let x = if fr < sx.f[worst] { along(-gamma) } else { along(gamma) };
            let fc = self.cost_free(&x);
            if fc < fr.min(sx.f[worst]) {
                sx.replace(worst, x, fc);
            } else {
                let b = sx.pts[best].clone();
                for i in 0..=n {
                    if i == best {
                        continue;
                    }
                    let mut v: Vec<f64> = (0..n).map(|k| b[k] + delta * (sx.pts[i][k] - b[k])).collect();
                    self.clamp_free(&mut v);
                    sx.f[i] = self.cost_free(&v);
                    sx.pts[i] = v;
                }
                sx.sort();
            }
        }
        true
    }

    /// The report at the current parameters. A fit that was not stepped to convergence is
    /// reported as stopped at the maximum number of iterations.
    pub fn finish(self) -> FitReport {
        let Fitter {
            func, opts, n_all, used, xs, ys, ws, sw, free, p, cost, iterations, converged, mut warnings, ..
        } = self;
        let status = if converged { FitStatus::Converged } else { FitStatus::MaxIterations };
        let n = xs.len();
        let np = func.params.len();
        let nfree = free.len();
        let opts = &opts;
        // Statistics at the optimum.
        let dof = n.saturating_sub(nfree);
        let rss = cost;
        let reduced = if dof > 0 { rss / dof as f64 } else { f64::NAN };

        let (jac, _) = jacobian(func, &p, &free, &xs, &ys, &sw);
        let jtj = jac.transpose() * &jac;
        let scale = opts.scale_errors.unwrap_or(true);
        let mut cov_free: Option<DMatrix<f64>> = None;
        let mut inv_free: Option<DMatrix<f64>> = None;
        if nfree > 0 {
            match jtj.clone().try_inverse() {
                Some(inv) if inv.iter().all(|v| v.is_finite()) => {
                    let factor = if scale { reduced } else { 1.0 };
                    cov_free = Some(&inv * factor);
                    inv_free = Some(inv);
                }
                _ => warnings.push("the covariance matrix is singular; standard errors are not available".into()),
            }
        }
        let level = opts.confidence_level;
        let tcrit = t_critical(level, dof);
        let mut params = Vec::with_capacity(np);
        let mut covariance = vec![0.0; np * np];
        for (j, s) in func.params.iter().enumerate() {
            let k = free.iter().position(|&f| f == j);
            let (mut se, mut dep) = (f64::NAN, f64::NAN);
            if let (Some(k), Some(cov)) = (k, cov_free.as_ref()) {
                let var = cov[(k, k)];
                se = if var >= 0.0 { var.sqrt() } else { f64::NAN };
                // Dependency is 1 - 1/(C_ii (C^-1)_ii); the scale factor cancels.
                if let Some(ci) = inv_free.as_ref() {
                    // (JtJ^-1)_ii * (JtJ)_ii is the same product as C_ii (C^-1)_ii.
                    let prod = ci[(k, k)] * jtj[(k, k)];
                    if prod.is_finite() && prod != 0.0 {
                        dep = 1.0 - 1.0 / prod;
                    }
                }
                for (k2, &j2) in free.iter().enumerate() {
                    covariance[j * np + j2] = cov[(k, k2)];
                }
            }
            let value = p[j];
            let fixed = s.fixed;
            let (t, prob, lcl, ucl) = if fixed || !se.is_finite() {
                (f64::NAN, f64::NAN, f64::NAN, f64::NAN)
            } else {
                let t = value / se;
                (t, t_two_sided_p(t, dof), value - tcrit * se, value + tcrit * se)
            };
            if !fixed {
                if s.lower.is_some_and(|lo| value <= lo) {
                    warnings.push(format!("parameter {} reached its lower bound", s.name));
                }
                if s.upper.is_some_and(|hi| value >= hi) {
                    warnings.push(format!("parameter {} reached its upper bound", s.name));
                }
            }
            params.push(Parameter {
                name: s.name.clone(),
                value,
                std_error: if fixed { 0.0 } else { se },
                fixed,
                t_value: t,
                prob_t: prob,
                lcl,
                ucl,
                dependency: if fixed { 0.0 } else { dep },
            });
        }
        if status == FitStatus::MaxIterations {
            warnings.push(
                "the fit stopped before converging (iteration limit reached, or no iterations were asked for)".into(),
            );
        }
        if params.iter().any(|q| q.dependency.is_finite() && q.dependency > 0.9999) {
            warnings.push("some parameters are strongly dependent on each other (dependency above 0.9999)".into());
        }

        // R-square and ANOVA, weighted by the fit weights.
        let sum_w: f64 = ws.iter().sum();
        let ybar_w = if sum_w > 0.0 { ws.iter().zip(&ys).map(|(w, v)| w * v).sum::<f64>() / sum_w } else { f64::NAN };
        let tss: f64 = ws.iter().zip(&ys).map(|(w, v)| w * (v - ybar_w) * (v - ybar_w)).sum();
        let utss: f64 = ws.iter().zip(&ys).map(|(w, v)| w * v * v).sum();
        let r2 = if tss > 0.0 { 1.0 - rss / tss } else { f64::NAN };
        let adj =
            if dof > 0 && n > 1 && tss > 0.0 { 1.0 - (rss / dof as f64) / (tss / (n - 1) as f64) } else { f64::NAN };
        let anova = if dof > 0 && n > 1 {
            let model_df = nfree;
            let model_ss = utss - rss;
            let model_ms = model_ss / model_df.max(1) as f64;
            let err_ms = rss / dof as f64;
            let f = model_ms / err_ms;
            Some(Anova {
                model_df,
                model_ss,
                model_ms,
                error_df: dof,
                error_ss: rss,
                error_ms: err_ms,
                total_df: n - 1,
                total_ss: tss,
                uncorrected_total_ss: utss,
                f_value: f,
                prob_f: f_upper_p(f, model_df.max(1), dof),
            })
        } else {
            None
        };

        // Fitted values and residuals for every input row.
        let mut fitted = vec![f64::NAN; n_all];
        let mut residuals = vec![f64::NAN; n_all];
        for (k, &i) in used.iter().enumerate() {
            let f = func.eval(&p, xs[k]);
            fitted[i] = f;
            residuals[i] = ys[k] - f;
        }

        FitReport {
            model: func.name.clone(),
            equation: func.equation.clone(),
            params,
            stats: Statistics {
                n_points: n,
                dof,
                rss,
                reduced_chi_sqr: reduced,
                r_squared: r2,
                adj_r_squared: adj,
                pearson_r: None,
                iterations: Some(iterations),
            },
            anova,
            covariance,
            confidence_level: level,
            fitted,
            residuals,
            status,
            warnings,
        }
    }
}

/// Size of the first simplex along each axis, relative to the parameter.
const SIMPLEX_INIT_STEP: f64 = 0.2;
/// A simplex stops when the costs of its vertices agree to this fraction ...
const SIMPLEX_F_TOL: f64 = 1e-13;
/// ... and the vertices lie this close together (relative to the best vertex).
const SIMPLEX_X_TOL: f64 = 1e-8;
/// A restart from the best vertex must lower the cost by this fraction to go on.
const SIMPLEX_RESTART_GAIN: f64 = 1e-12;

/// The vertices of a simplex over the free parameters, their costs, and their order (best first).
struct Simplex {
    pts: Vec<Vec<f64>>,
    f: Vec<f64>,
    order: Vec<usize>,
    last_restart: Option<f64>,
}

impl Simplex {
    fn sort(&mut self) {
        let mut o: Vec<usize> = (0..self.pts.len()).collect();
        o.sort_by(|&a, &b| self.f[a].total_cmp(&self.f[b]));
        self.order = o;
    }
    fn replace(&mut self, i: usize, v: Vec<f64>, f: f64) {
        self.pts[i] = v;
        self.f[i] = f;
        self.sort();
    }
}

/// Starting value of the damping factor of every Levenberg-Marquardt iteration.
const INITIAL_DAMPING: f64 = 1e-3;

fn check_len(have: usize, want: usize) -> Result<(), FitError> {
    if have != want { Err(FitError::LengthMismatch { x: want, y: have }) } else { Ok(()) }
}

/// Weighted residual vector `sqrt(w) (y - f)` and the weighted Jacobian of the model with
/// respect to the free parameters (central differences). The Jacobian is of `f`, so that
/// `J' r` is the descent direction of the sum of squares.
fn jacobian(
    func: &FitFunction,
    p: &[f64],
    free: &[usize],
    xs: &[f64],
    ys: &[f64],
    sw: &[f64],
) -> (DMatrix<f64>, DVector<f64>) {
    let n = xs.len();
    let mut jac = DMatrix::<f64>::zeros(n, free.len());
    let mut res = DVector::<f64>::zeros(n);
    for i in 0..n {
        res[i] = sw[i] * (ys[i] - func.eval(p, xs[i]));
    }
    let mut q = p.to_vec();
    for (k, &j) in free.iter().enumerate() {
        let h = 6e-6 * p[j].abs().max(1e-3);
        // Central differences, also across a bound: Origin differentiates the model itself, so a
        // parameter resting on its bound must not get a one-sided (less accurate) derivative.
        for i in 0..n {
            q[j] = p[j] + h;
            let fu = func.eval(&q, xs[i]);
            q[j] = p[j] - h;
            let fd = func.eval(&q, xs[i]);
            let d = if fu.is_finite() && fd.is_finite() {
                (fu - fd) / (2.0 * h)
            } else {
                // The model is not defined on one side (for example a bound at the edge of its
                // domain): use the other side.
                let f0 = func.eval(p, xs[i]);
                if fu.is_finite() { (fu - f0) / h } else { (f0 - fd) / h }
            };
            jac[(i, k)] = sw[i] * d;
        }
        q[j] = p[j];
    }
    (jac, res)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unusable_parameter_names_are_refused() {
        for (formula, params) in
            [("E*x+3", vec!["E"]), ("pi*a*x", vec!["a", "Pi"]), ("a*x", vec!["a", "a"]), ("a*x", vec!["x", "a"])]
        {
            let e = FitFunction::from_formula("U", formula, &params).unwrap_err().to_string();
            assert!(e.contains("parameter"), "{formula} {params:?}: {e}");
        }
        let e = FitFunction::from_formula("U", "a*x", &["a", "e1"]).unwrap_err().to_string();
        assert!(e.contains("'e1' does not appear"), "{e}");
        // Names differ by case in the expression language, so these are two parameters.
        assert!(FitFunction::from_formula("U", "a*x + A", &["a", "A"]).is_ok());
    }

    fn data() -> (Vec<f64>, Vec<f64>) {
        let x: Vec<f64> = (1..=30).map(|i| i as f64 * 0.3).collect();
        let y: Vec<f64> = x
            .iter()
            .enumerate()
            .map(|(i, &x)| 1.5 + 4.0 * (-x / 2.0).exp() + 0.01 * (37.0 * (i + 1) as f64).sin())
            .collect();
        (x, y)
    }

    #[test]
    fn exponential_decay_is_recovered() {
        let (x, y) = data();
        let f = FitFunction::from_formula("User", "y0 + A*exp(-x/t)", &["y0", "A", "t"]).unwrap();
        let r = fit(&f, &x, &y, &FitOptions::default()).unwrap();
        assert_eq!(r.status, FitStatus::Converged);
        assert!((r.param("y0").unwrap().value - 1.5).abs() < 0.02);
        assert!((r.param("t").unwrap().value - 2.0).abs() < 0.05);
        assert_eq!(r.stats.dof, 27);
        assert!(r.param("t").unwrap().std_error > 0.0);
        assert!(r.stats.r_squared > 0.99);
    }

    #[test]
    fn fixed_parameter_stays_put() {
        let (x, y) = data();
        let f = FitFunction::from_formula("User", "y0 + A*exp(-x/t)", &["y0", "A", "t"]).unwrap().with_fixed("y0", 1.5);
        let r = fit(&f, &x, &y, &FitOptions::default()).unwrap();
        let p = r.param("y0").unwrap();
        assert!(p.fixed && p.value == 1.5 && p.std_error == 0.0 && p.dependency == 0.0);
        assert_eq!(r.stats.dof, 28);
    }

    #[test]
    fn active_bound_is_reported() {
        let (x, y) = data();
        let f = FitFunction::from_formula("User", "y0 + A*exp(-x/t)", &["y0", "A", "t"]).unwrap().with_bounds(
            "t",
            None,
            Some(1.5),
        );
        let r = fit(&f, &x, &y, &FitOptions::default()).unwrap();
        assert!((r.param("t").unwrap().value - 1.5).abs() < 1e-9);
        assert!(r.warnings.iter().any(|w| w.contains("upper bound")));
    }

    #[test]
    fn too_few_points_is_an_error() {
        let f = FitFunction::from_formula("User", "a + b*x", &["a", "b"]).unwrap();
        let e = fit(&f, &[1.0], &[1.0], &FitOptions::default()).unwrap_err();
        assert!(matches!(e, FitError::TooFewPoints { .. }));
    }

    #[test]
    fn formula_with_unknown_name_is_rejected() {
        assert!(FitFunction::from_formula("U", "a + q*x", &["a", "b"]).is_err());
        assert!(FitFunction::from_formula("U", "y = a + b*x", &["a", "b"]).is_ok());
    }

    fn user() -> FitFunction {
        FitFunction::from_formula("User", "y0 + A*exp(-x/t)", &["y0", "A", "t"]).unwrap()
    }

    #[test]
    fn direct_weight_of_zero_leaves_the_point_out() {
        let (x, y) = data();
        let mut w = vec![1.0; x.len()];
        w[5] = 0.0;
        let opts = FitOptions { weighting: Weighting::Direct(w), ..FitOptions::default() };
        let r = fit(&user(), &x, &y, &opts).unwrap();
        assert_eq!(r.stats.n_points, 29);
        assert_eq!(r.stats.dof, 26);
    }

    #[test]
    fn direct_weights_all_zero_is_an_error() {
        let (x, y) = data();
        let opts = FitOptions { weighting: Weighting::Direct(vec![0.0; x.len()]), ..FitOptions::default() };
        assert!(matches!(fit(&user(), &x, &y, &opts), Err(FitError::TooFewPoints { .. })));
    }

    #[test]
    fn statistical_weights_use_the_absolute_value_of_y() {
        // Cathodic currents: all y negative. The weight is 1/|y|, so the fit equals the fit of the
        // mirrored data (y -> -y) with the same weights.
        let t: Vec<f64> = (0..40).map(|i| 0.25 * i as f64).collect();
        let y: Vec<f64> = t
            .iter()
            .enumerate()
            .map(|(i, t)| -2e-6 - 1e-5 * (-t / 1.5).exp() + 1e-8 * (37.0 * i as f64).sin())
            .collect();
        let mirrored: Vec<f64> = y.iter().map(|v| -v).collect();
        let opts = FitOptions { weighting: Weighting::Statistical, ..FitOptions::default() };
        let neg = fit(&user(), &t, &y, &opts).unwrap();
        let pos = fit(&user(), &t, &mirrored, &opts).unwrap();
        assert_eq!(neg.stats.n_points, 40);
        assert!((neg.param("t").unwrap().value - 1.5).abs() < 0.05, "{}", neg.param("t").unwrap().value);
        assert!((neg.param("t").unwrap().value - pos.param("t").unwrap().value).abs() < 1e-6);
        assert!((neg.param("y0").unwrap().value + pos.param("y0").unwrap().value).abs() < 1e-12);
    }

    #[test]
    fn statistical_weights_on_negative_y_match_origin() {
        // Origin, nlfit ExpDec1 with statistical weights on this data (script
        // tools/origin-ref/nlfit-negative-stat.lines.txt): the fit of the negated data gives the
        // negated parameters of the fit of the positive data, t1 unchanged.
        let t: Vec<f64> = (1..=30).map(|i| 0.3 * i as f64).collect();
        let y: Vec<f64> = t
            .iter()
            .enumerate()
            .map(|(k, t)| {
                let d = (k + 1) as f64;
                -(1.5 + 4.0 * (-t / 2.0).exp() + 0.02 * (37.0 * d).sin() + 0.015 * (101.3 * d).sin())
            })
            .collect();
        let opts = FitOptions { weighting: Weighting::Statistical, ..FitOptions::default() };
        // ExpDec1 from the library, so that its starting-value routine is used (a start at 1 for
        // every parameter ends in a different, worse minimum).
        let f = crate::library::get("ExpDec1").unwrap();
        let r = fit(&f, &t, &y, &opts).unwrap();
        let v = |n: &str| r.param(n).unwrap().value;
        assert!((v("y0") + 1.495_489_362_101_76).abs() < 1e-6, "{}", v("y0"));
        assert!((v("A1") + 3.993_133_656_464_53).abs() < 1e-6, "{}", v("A1"));
        assert!((v("t1") - 2.011_509_367_437_72).abs() < 1e-6, "{}", v("t1"));
    }

    #[test]
    fn statistical_weights_refuse_a_zero_y_and_say_why() {
        let (x, mut y) = data();
        y[7] = 0.0;
        let opts = FitOptions { weighting: Weighting::Statistical, ..FitOptions::default() };
        match fit(&user(), &x, &y, &opts) {
            Err(FitError::Failed(m)) => assert!(m.contains("statistical weighting") && m.contains("zero"), "{m}"),
            other => panic!("expected a clear error, got {other:?}"),
        }
    }

    #[test]
    fn contradictory_or_non_finite_settings_are_errors() {
        let (x, y) = data();
        let o = FitOptions::default();
        let f = user().with_bounds("t", Some(5.0), Some(1.0));
        assert!(matches!(fit(&f, &x, &y, &o), Err(FitError::BadFunction(_))));
        let f = user().with_fixed("y0", f64::NAN);
        assert!(matches!(fit(&f, &x, &y, &o), Err(FitError::BadFunction(_))));
        let f = user().with_bounds("t", Some(f64::NAN), None);
        assert!(matches!(fit(&f, &x, &y, &o), Err(FitError::BadFunction(_))));
    }

    #[test]
    fn overflowing_cost_is_reported_as_such() {
        let (x, y) = data();
        let big: Vec<f64> = y.iter().map(|v| v * 1e200).collect();
        match fit(&user(), &x, &big, &FitOptions::default()) {
            Err(FitError::Failed(m)) => assert!(m.contains("overflow"), "{m}"),
            other => panic!("expected an overflow error, got {other:?}"),
        }
    }

    #[test]
    fn missing_points_are_dropped() {
        let (mut x, y) = data();
        x[3] = f64::NAN;
        let f = FitFunction::from_formula("U", "y0 + A*exp(-x/t)", &["y0", "A", "t"]).unwrap();
        let r = fit(&f, &x, &y, &FitOptions::default()).unwrap();
        assert_eq!(r.stats.n_points, 29);
        assert!(r.fitted[3].is_nan() && r.residuals[3].is_nan() && r.fitted[4].is_finite());
    }

    fn start_func() -> FitFunction {
        FitFunction::from_formula("User", "y0 + A*exp(-x/t)", &["y0", "A", "t"])
            .unwrap()
            .with_initial("y0", 1.0)
            .with_initial("A", 3.0)
            .with_initial("t", 1.0)
            .with_bounds("t", Some(0.2), Some(5.0))
    }

    #[test]
    fn n_single_steps_equal_a_fit_limited_to_n_iterations() {
        let (x, y) = data();
        let f = start_func();
        for n in 1..=5 {
            let opts = FitOptions { max_iterations: n, ..FitOptions::default() };
            let whole = fit(&f, &x, &y, &opts).unwrap();
            let mut s = Fitter::new(&f, &x, &y, &FitOptions::default()).unwrap();
            let mut last = None;
            for _ in 0..n {
                last = Some(s.step());
            }
            let last = last.unwrap();
            for (a, b) in whole.params.iter().zip(&last.params) {
                assert_eq!(a.value, *b, "{} after {n} steps", a.name);
            }
            assert_eq!(whole.stats.rss, last.chi2);
            let report = s.finish();
            assert_eq!(report.stats.iterations, whole.stats.iterations);
            assert_eq!(report.status, whole.status);
        }
    }

    #[test]
    fn stepping_to_convergence_equals_fit() {
        let (x, y) = data();
        let f = start_func();
        let whole = fit(&f, &x, &y, &FitOptions::default()).unwrap();
        let mut s = Fitter::new(&f, &x, &y, &FitOptions::default()).unwrap();
        let mut steps = 0;
        while !s.step().converged {
            steps += 1;
            assert!(steps < 400);
        }
        let again = s.step();
        assert!(again.converged);
        let report = s.finish();
        assert_eq!(report.status, FitStatus::Converged);
        assert_eq!(report.stats.iterations, whole.stats.iterations);
        for (a, b) in whole.params.iter().zip(&report.params) {
            assert_eq!((a.value, a.std_error), (b.value, b.std_error));
        }
        assert_eq!(whole.covariance, report.covariance);
    }
}
