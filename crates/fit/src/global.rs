//! Global fit: one model fitted to several datasets at once, with some parameters shared
//! between the datasets and the rest fitted separately for each (Origin's "Global Fit" mode of
//! NLFit; the rules are in `docs/spec/fitting.md`, section 10).
//!
//! The datasets are stacked into one problem. A composite [`FitFunction`] has one parameter for
//! every shared parameter and one per dataset for every other one; its independent variable is
//! the position in the stacked data, from which the model looks up the dataset and the real x.
//! The ordinary engine ([`crate::nlsf`]) then does the rest, so bounds, fixed parameters,
//! weights, damping and stopping are exactly those of a single fit, and the statistics are those
//! of the stacked problem (points of all datasets, free parameters counted once).
//!
//! Parameter names of the composite problem follow Origin: the first dataset's copy of a
//! parameter keeps the plain name (`xc`), the copy of dataset k is `xc_k`, a shared parameter is
//! listed once under the plain name.

use std::sync::Arc;

use crate::dist::f_upper_p;
use crate::nlsf::{Algorithm, FitFunction, FitOptions, Fitter, Model, ParamSpec};
use crate::report::{FitError, FitReport, Parameter, Statistics, Weighting};

/// One dataset of a global fit.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalDataset {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

/// How the points of the datasets are weighted. The per-dataset vectors hold one value per point
/// of that dataset: errors for `Instrumental`, weights for `Direct`.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum GlobalWeighting {
    #[default]
    None,
    Instrumental(Vec<Vec<f64>>),
    Statistical,
    Direct(Vec<Vec<f64>>),
}

/// A setting of one parameter of one dataset that differs from the function's own (Origin lets
/// each dataset's copy of a parameter have its own start value or be fixed at its own value). For a
/// shared parameter only dataset 0 counts.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamOverride {
    /// 0-based dataset.
    pub dataset: usize,
    /// Index of the parameter in the function.
    pub param: usize,
    /// Start value, or the value a fixed parameter is held at.
    pub initial: Option<f64>,
    pub fixed: Option<bool>,
}

/// Settings of a global fit.
#[derive(Debug, Clone)]
pub struct GlobalOptions {
    /// One flag per parameter of the function: shared between the datasets or not.
    pub shared: Vec<bool>,
    pub weighting: GlobalWeighting,
    pub max_iterations: usize,
    pub tolerance: f64,
    pub confidence_level: f64,
    /// See [`FitOptions::scale_errors`].
    pub scale_errors: Option<bool>,
    /// Settings that differ between the datasets' copies of a parameter.
    pub overrides: Vec<ParamOverride>,
    pub algorithm: Algorithm,
}

impl GlobalOptions {
    /// Origin's defaults, with the given parameters shared.
    pub fn new(shared: Vec<bool>) -> Self {
        GlobalOptions {
            shared,
            weighting: GlobalWeighting::None,
            max_iterations: 400,
            tolerance: 1e-9,
            confidence_level: 0.95,
            scale_errors: None,
            overrides: Vec::new(),
            algorithm: Algorithm::LevenbergMarquardt,
        }
    }
}

/// The result of a global fit.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalFit {
    /// The fit of the stacked problem: parameters of the composite function (see the module
    /// text), statistics and ANOVA of all points together, covariance of the composite
    /// parameters. `fitted` and `residuals` are in stacked order (dataset 1 first). The ANOVA
    /// corrected total uses the mean of each dataset (degrees of freedom: points minus datasets),
    /// as Origin's report does; R-square and adjusted R-square use the mean of all points.
    pub combined: FitReport,
    /// The same fit seen from each dataset, in the parameter order of the function: the values,
    /// errors and limits of the parameters that dataset uses (a shared parameter appears in every
    /// one), its own number of points, residual sum of squares and R-square, and the degrees of
    /// freedom and reduced chi-square of the whole fit. `covariance` is that of the function's
    /// parameters (shared parameters have the same variance in every dataset).
    pub datasets: Vec<FitReport>,
}

/// The parameter name of dataset `d` (0-based): the plain name for the first one, `name_k` for
/// dataset k (1-based) after it.
pub fn dataset_param_name(name: &str, d: usize) -> String {
    if d == 0 { name.to_string() } else { format!("{name}_{}", d + 1) }
}

/// Fits `func` to all `datasets` at once.
pub fn fit_global(func: &FitFunction, datasets: &[GlobalDataset], opts: &GlobalOptions) -> Result<GlobalFit, FitError> {
    let np = func.params.len();
    let nd = datasets.len();
    if nd == 0 {
        return Err(FitError::Failed("a global fit needs at least one dataset".into()));
    }
    if opts.shared.len() != np {
        return Err(FitError::BadFunction(format!(
            "{} sharing flags for {np} parameters of {}",
            opts.shared.len(),
            func.name
        )));
    }
    for ds in datasets {
        if ds.x.len() != ds.y.len() {
            return Err(FitError::LengthMismatch { x: ds.x.len(), y: ds.y.len() });
        }
    }
    let per_set_len = |v: &Vec<Vec<f64>>| -> Result<(), FitError> {
        if v.len() != nd {
            return Err(FitError::Failed(format!("{} weight or error columns for {nd} datasets", v.len())));
        }
        for (d, w) in v.iter().enumerate() {
            if w.len() != datasets[d].x.len() {
                return Err(FitError::LengthMismatch { x: datasets[d].x.len(), y: w.len() });
            }
        }
        Ok(())
    };
    // Weights of every point (NaN: the point is left out), then stacked.
    let mut weights: Vec<Vec<f64>> = Vec::with_capacity(nd);
    match &opts.weighting {
        GlobalWeighting::None => weights.extend(datasets.iter().map(|d| vec![1.0; d.x.len()])),
        GlobalWeighting::Instrumental(e) => {
            per_set_len(e)?;
            weights
                .extend(e.iter().map(|e| e.iter().map(|e| if *e > 0.0 { 1.0 / (e * e) } else { f64::NAN }).collect()));
        }
        GlobalWeighting::Direct(w) => {
            per_set_len(w)?;
            weights.extend(w.iter().map(|w| w.iter().map(|w| if *w > 0.0 { *w } else { f64::NAN }).collect()));
        }
        GlobalWeighting::Statistical => {
            for d in datasets {
                if d.y.contains(&0.0) {
                    return Err(FitError::Failed(
                        "statistical weighting needs y different from zero (the weight is 1/|y|); \
                         a point with y = 0 cannot be weighted"
                            .to_string(),
                    ));
                }
                weights.push(d.y.iter().map(|v| 1.0 / v.abs()).collect());
            }
        }
    }

    let mut stack_x = Vec::new();
    let mut stack_y = Vec::new();
    let mut stack_w = Vec::new();
    let mut start = Vec::with_capacity(nd + 1);
    for (d, ds) in datasets.iter().enumerate() {
        start.push(stack_x.len());
        #[allow(clippy::needless_range_loop)]
        for i in 0..ds.x.len() {
            let ok = ds.x[i].is_finite() && ds.y[i].is_finite() && weights[d][i].is_finite();
            stack_x.push(ds.x[i]);
            stack_y.push(if ok { ds.y[i] } else { f64::NAN });
            stack_w.push(if ok { weights[d][i] } else { f64::NAN });
        }
    }
    start.push(stack_x.len());

    // Composite parameters.
    let mut specs: Vec<ParamSpec> = Vec::new();
    let mut map: Vec<Vec<usize>> = vec![vec![0; np]; nd];
    let guesses: Vec<Vec<f64>> = datasets
        .iter()
        .enumerate()
        .map(|(d, _)| {
            let (xs, ys): (Vec<f64>, Vec<f64>) =
                (start[d]..start[d + 1]).filter(|&k| stack_y[k].is_finite()).map(|k| (stack_x[k], stack_y[k])).unzip();
            func.init.as_ref().map(|f| f(&xs, &ys)).unwrap_or_default()
        })
        .collect();
    for j in 0..np {
        let base = &func.params[j];
        let initial_for = |d: usize| -> f64 {
            if base.initial.is_finite() {
                base.initial
            } else if guesses[d].get(j).is_some_and(|g| g.is_finite()) {
                guesses[d][j]
            } else {
                1.0
            }
        };
        let over = |d: usize| opts.overrides.iter().rev().find(|o| o.dataset == d && o.param == j);
        let make = |d: usize, name: String| -> ParamSpec {
            let mut s = ParamSpec { name, initial: initial_for(d), ..base.clone() };
            if let Some(o) = over(d) {
                if let Some(v) = o.initial {
                    s.initial = v;
                }
                if let Some(f) = o.fixed {
                    s.fixed = f;
                }
            }
            s
        };
        if opts.shared[j] {
            specs.push(make(0, base.name.clone()));
            for row in map.iter_mut() {
                row[j] = specs.len() - 1;
            }
        } else {
            for (d, row) in map.iter_mut().enumerate() {
                row[j] = specs.len();
                specs.push(make(d, dataset_param_name(&base.name, d)));
            }
        }
    }

    // The composite model: x is the position in the stacked data.
    let inner = Arc::new(func.clone());
    let xs_real = Arc::new(stack_x.clone());
    let owner: Arc<Vec<usize>> =
        Arc::new((0..nd).flat_map(|d| std::iter::repeat_n(d, start[d + 1] - start[d])).collect());
    let map_arc = Arc::new(map.clone());
    let model = {
        let (inner, xs_real, owner, map_arc) = (inner.clone(), xs_real.clone(), owner.clone(), map_arc.clone());
        Arc::new(move |p: &[f64], pos: f64| -> f64 {
            let k = pos as usize;
            let Some(&d) = owner.get(k) else { return f64::NAN };
            let local: Vec<f64> = map_arc[d].iter().map(|&c| p[c]).collect();
            inner.eval(&local, xs_real[k])
        })
    };
    let composite = FitFunction {
        name: func.name.clone(),
        equation: func.equation.clone(),
        x_name: "x".to_string(),
        params: specs,
        model: Model::Closure(model),
        init: None,
    };

    let positions: Vec<f64> = (0..stack_x.len()).map(|k| k as f64).collect();
    let fit_opts = FitOptions {
        weighting: Weighting::Direct(stack_w.clone()),
        max_iterations: opts.max_iterations,
        tolerance: opts.tolerance,
        confidence_level: opts.confidence_level,
        scale_errors: opts.scale_errors,
        algorithm: opts.algorithm,
    };
    let mut fitter = Fitter::new(&composite, &positions, &stack_y, &fit_opts)?;
    for _ in 0..opts.max_iterations {
        if fitter.step().converged {
            break;
        }
    }
    let mut combined = fitter.finish();
    combined.model = format!("{} (global fit)", func.name);

    // Statistics of each dataset, and the ANOVA corrected total of Origin's report.
    let ncp = combined.params.len();
    let mut tss_sum = 0.0;
    let mut n_sum = 0usize;
    let mut reports = Vec::with_capacity(nd);
    for d in 0..nd {
        let range = start[d]..start[d + 1];
        let idx: Vec<usize> = range.clone().filter(|&k| combined.fitted[k].is_finite()).collect();
        let n_d = idx.len();
        let sw: f64 = idx.iter().map(|&k| stack_w[k]).sum();
        let ybar = if sw > 0.0 { idx.iter().map(|&k| stack_w[k] * stack_y[k]).sum::<f64>() / sw } else { f64::NAN };
        let tss: f64 = idx.iter().map(|&k| stack_w[k] * (stack_y[k] - ybar).powi(2)).sum();
        let rss: f64 = idx.iter().map(|&k| stack_w[k] * combined.residuals[k].powi(2)).sum();
        tss_sum += tss;
        n_sum += n_d;
        let params: Vec<Parameter> = (0..np)
            .map(|j| {
                let mut q = combined.params[map[d][j]].clone();
                q.name = func.params[j].name.clone();
                q
            })
            .collect();
        let mut cov = vec![0.0; np * np];
        for a in 0..np {
            for b in 0..np {
                cov[a * np + b] = combined.covariance[map[d][a] * ncp + map[d][b]];
            }
        }
        let mut warnings = combined.warnings.clone();
        let shared_names: Vec<&str> =
            (0..np).filter(|&j| opts.shared[j]).map(|j| func.params[j].name.as_str()).collect();
        warnings.push(if shared_names.is_empty() {
            format!("global fit of {nd} datasets with no shared parameter; dataset {} of {nd}", d + 1)
        } else {
            format!("global fit of {nd} datasets, shared: {}; dataset {} of {nd}", shared_names.join(", "), d + 1)
        });
        reports.push(FitReport {
            model: func.name.clone(),
            equation: func.equation.clone(),
            params,
            stats: Statistics {
                n_points: n_d,
                dof: combined.stats.dof,
                rss,
                reduced_chi_sqr: combined.stats.reduced_chi_sqr,
                r_squared: if tss > 0.0 { 1.0 - rss / tss } else { f64::NAN },
                adj_r_squared: f64::NAN,
                pearson_r: None,
                iterations: combined.stats.iterations,
            },
            anova: None,
            covariance: cov,
            confidence_level: combined.confidence_level,
            fitted: combined.fitted[range.clone()].to_vec(),
            residuals: combined.residuals[range].to_vec(),
            status: combined.status.clone(),
            warnings,
        });
    }
    if let Some(a) = combined.anova.as_mut() {
        a.total_df = n_sum.saturating_sub(nd);
        a.total_ss = tss_sum;
        a.prob_f = f_upper_p(a.f_value, a.model_df.max(1), a.error_df);
    }
    Ok(GlobalFit { combined, datasets: reports })
}
