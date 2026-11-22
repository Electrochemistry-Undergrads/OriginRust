//! Fitting a model to worksheet columns.
//!
//! [`prepare`] checks everything that needs no data, [`run`] looks the columns up in a project
//! and fits. The text of the errors names the command-line options; they have the same names
//! as the fields of [`FitSpec`].

use crate::analyze::row_range;
use crate::pick::{named_columns, pick_book, pick_sheet};
use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use sheaf_core::analysis::RowRange;
use sheaf_core::fitting::{
    BandChoice, CompareOutcome, FitModel, FitOutcome, FitRequest, FitWeighting, MultiRegOutcome, MultiRegRequest,
    ParamSetting, RankOutcome, SurfaceColumns, builtin_functions, surface_functions,
};
use sheaf_core::{ColumnId, Project, WindowId};
use std::path::PathBuf;

/// How points are weighted.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeightKind {
    #[default]
    None,
    Instrumental,
    Statistical,
    Direct,
}

/// Bands around the fitted curve.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BandKind {
    #[default]
    None,
    Confidence,
    Prediction,
    Both,
}

/// What to fit. The defaults are those of the command line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FitSpec {
    /// linear, poly:ORDER or a built-in function name.
    pub model: String,
    /// A formula to fit instead of `model`; its parameters are in `params`.
    pub formula: Option<String>,
    pub params: Vec<String>,
    /// An Origin .FDF file that defines the function.
    pub fdf: Option<PathBuf>,
    /// Starting values, each "NAME=VALUE".
    pub init: Vec<String>,
    /// Parameters held at a value, each "NAME=VALUE".
    pub fix: Vec<String>,
    /// Bounds, each "NAME=LOW:HIGH" (a side may be empty).
    pub bounds: Vec<String>,
    pub weight: WeightKind,
    pub yerr: Vec<String>,
    pub xerr: Option<String>,
    /// Multiple linear regression on these columns.
    pub regress: Vec<String>,
    /// Compare `model` with this model.
    pub compare: Option<String>,
    /// Rank these built-in functions.
    pub rank: Vec<String>,
    pub no_scale_errors: bool,
    /// Add Origin's Covariance Matrix to the parameter sheet.
    pub covariance_matrix: bool,
    /// Add Origin's Correlation Matrix to the parameter sheet.
    pub correlation_matrix: bool,
    pub global: bool,
    pub share: Vec<String>,
    pub confidence: f64,
    pub bands: BandKind,
    /// Confidence level of the bands in percent.
    pub level: f64,
    pub max_iterations: usize,
    pub tolerance: f64,
    /// Auto Parameter Initialization off (T331).
    pub no_auto_init: bool,
    pub y: Vec<String>,
    pub book: Option<String>,
    pub sheet: Option<String>,
    pub rows: Option<String>,
    pub x_range: Option<String>,
    /// A surface fit `z = f(x, y)`: the Z column. `y` then holds the Y column (first entry) and
    /// `x` the X column; either one left out is the sheet's own (fitting.md section 15).
    pub z: Option<String>,
    pub x: Option<String>,
}

impl Default for FitSpec {
    fn default() -> Self {
        Self {
            model: "linear".to_string(),
            formula: None,
            params: Vec::new(),
            fdf: None,
            init: Vec::new(),
            fix: Vec::new(),
            bounds: Vec::new(),
            weight: WeightKind::None,
            yerr: Vec::new(),
            xerr: None,
            regress: Vec::new(),
            compare: None,
            rank: Vec::new(),
            no_scale_errors: false,
            covariance_matrix: false,
            correlation_matrix: false,
            global: false,
            share: Vec::new(),
            confidence: 0.95,
            bands: BandKind::None,
            level: 95.0,
            max_iterations: 400,
            tolerance: 1e-9,
            no_auto_init: false,
            y: Vec::new(),
            book: None,
            sheet: None,
            rows: None,
            x_range: None,
            z: None,
            x: None,
        }
    }
}

/// Reads `--model`: `linear`, `poly:N` or a built-in function name.
pub fn model_from(text: &str) -> Result<FitModel> {
    let t = text.trim();
    if t.eq_ignore_ascii_case("linear") || t.eq_ignore_ascii_case("line") {
        return Ok(FitModel::Linear);
    }
    if t.eq_ignore_ascii_case("linearx") || t.eq_ignore_ascii_case("xerror") {
        return Ok(FitModel::LinearXError);
    }
    let lower = t.to_ascii_lowercase();
    if let Some(order) = lower.strip_prefix("poly:").or_else(|| lower.strip_prefix("polynomial:")) {
        let order: usize = order
            .trim()
            .parse()
            .map_err(|_| anyhow!("'{t}': the polynomial order must be a whole number, as in poly:3"))?;
        if !(1..=9).contains(&order) {
            bail!("the polynomial order must be between 1 and 9, not {order}");
        }
        return Ok(FitModel::Polynomial { order });
    }
    if lower == "poly" || lower == "polynomial" {
        bail!("give the polynomial order, for example --model poly:2");
    }
    Ok(FitModel::BuiltIn(t.to_string()))
}

/// Reads `NAME=VALUE`.
pub fn name_value(text: &str, option: &str) -> Result<(String, f64)> {
    let (name, value) = text
        .split_once('=')
        .ok_or_else(|| anyhow!("{option} '{text}' must be written NAME=VALUE, for example {option} xc=0.45"))?;
    let v: f64 = value.trim().parse().map_err(|_| anyhow!("{option} '{text}': '{}' is not a number", value.trim()))?;
    if !v.is_finite() {
        bail!("{option} '{text}': the value must be a finite number");
    }
    Ok((name.trim().to_string(), v))
}

/// Reads `NAME=LOW:HIGH`, where either side may be empty.
pub fn name_bounds(text: &str) -> Result<(String, Option<f64>, Option<f64>)> {
    let err = || anyhow!("--bounds '{text}' must be written NAME=LOW:HIGH, for example --bounds w=0: or A=0:100");
    let (name, range) = text.split_once('=').ok_or_else(err)?;
    let (lo, hi) = range.split_once(':').ok_or_else(err)?;
    let side = |s: &str| -> Result<Option<f64>> {
        let s = s.trim();
        if s.is_empty() { Ok(None) } else { s.parse().map(Some).map_err(|_| err()) }
    };
    Ok((name.trim().to_string(), side(lo)?, side(hi)?))
}

/// Collects init, fix and bounds into one setting per parameter name.
pub fn settings_from(a: &FitSpec) -> Result<Vec<ParamSetting>> {
    let mut out: Vec<ParamSetting> = Vec::new();
    fn entry<'a>(out: &'a mut Vec<ParamSetting>, name: &str) -> &'a mut ParamSetting {
        if let Some(i) = out.iter().position(|s| s.name == name) {
            return &mut out[i];
        }
        out.push(ParamSetting { name: name.to_string(), ..Default::default() });
        out.last_mut().expect("just pushed")
    }
    for t in &a.init {
        let (n, v) = name_value(t, "--init")?;
        entry(&mut out, &n).initial = Some(v);
    }
    for t in &a.fix {
        let (n, v) = name_value(t, "--fix")?;
        let s = entry(&mut out, &n);
        s.initial = Some(v);
        s.fixed = true;
    }
    for t in &a.bounds {
        let (n, lo, hi) = name_bounds(t)?;
        let s = entry(&mut out, &n);
        s.lower = lo;
        s.upper = hi;
    }
    Ok(out)
}

/// One built-in fit function.
#[derive(Clone, Debug, Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub category: String,
    pub parameters: Vec<String>,
    pub priority: String,
}

/// The built-in fit functions, sorted by priority, category and name.
pub fn list_models() -> Vec<ModelInfo> {
    let mut functions = builtin_functions();
    functions.extend(surface_functions());
    functions.sort_by(|a, b| a.priority.cmp(&b.priority).then(a.category.cmp(&b.category)).then(a.name.cmp(&b.name)));
    functions
        .into_iter()
        .map(|f| ModelInfo { name: f.name, category: f.category, parameters: f.params, priority: f.priority })
        .collect()
}

/// What a spec says without looking at any data.
pub struct Prepared {
    pub model: FitModel,
    pub params: Vec<ParamSetting>,
    pub rows: RowRange,
    pub weighting: FitWeighting,
    pub bands: BandChoice,
}

/// Checks the numbers of the spec and reads the model, parameter settings and row range.
pub fn prepare(a: &FitSpec) -> Result<Prepared> {
    if !(a.confidence > 0.0 && a.confidence < 1.0) {
        bail!("--confidence must be between 0 and 1, for example 0.95");
    }
    if !(a.level > 0.0 && a.level < 100.0) {
        bail!("--level is a percentage between 0 and 100, for example --level 95");
    }
    if a.max_iterations == 0 {
        bail!("--max-iterations must be at least 1");
    }
    if !(a.tolerance > 0.0 && a.tolerance.is_finite()) {
        bail!("--tolerance must be a positive number, for example 1e-9");
    }
    let bands = match a.bands {
        BandKind::None => BandChoice::None,
        BandKind::Confidence => BandChoice::Confidence,
        BandKind::Prediction => BandChoice::Prediction,
        BandKind::Both => BandChoice::Both,
    };
    let model = match (&a.formula, &a.fdf) {
        (Some(f), _) => {
            FitModel::Formula { formula: f.clone(), params: a.params.iter().map(|p| p.trim().to_string()).collect() }
        }
        (None, Some(path)) => {
            let text =
                sheaf_io::read_text_file(path).with_context(|| format!("could not read '{}'", path.display()))?;
            FitModel::Fdf(text)
        }
        (None, None) => model_from(&a.model)?,
    };
    let params = settings_from(a)?;
    let rows = row_range(a.rows.as_deref(), a.x_range.as_deref())?;
    let weighting = match a.weight {
        WeightKind::None => FitWeighting::None,
        WeightKind::Instrumental => FitWeighting::Instrumental,
        WeightKind::Statistical => FitWeighting::Statistical,
        WeightKind::Direct => FitWeighting::Direct,
    };
    Ok(Prepared { model, params, rows, weighting, bands })
}

/// What a fit produced.
pub enum FitRun {
    /// One outcome per fitted column. A global fit has a summary line first.
    Fits {
        summary: Option<String>,
        outcomes: Vec<FitOutcome>,
        columns: Vec<ColumnId>,
    },
    Regression(MultiRegOutcome),
    Comparison(CompareOutcome),
    Ranking(RankOutcome),
}

/// The result of [`run`].
pub struct FitResult {
    pub book: WindowId,
    pub run: FitRun,
}

impl FitResult {
    /// The sheet of the book that holds the (first) result, for exporting.
    pub fn result_sheet(&self) -> usize {
        match &self.run {
            FitRun::Fits { outcomes, .. } => outcomes[0].param_sheet,
            FitRun::Regression(o) => o.param_sheet,
            FitRun::Comparison(o) => o.sheet,
            FitRun::Ranking(o) => o.sheet,
        }
    }
}

/// Fits the columns the spec names.
pub fn run(project: &mut Project, a: &FitSpec, prepared: &Prepared) -> Result<FitResult> {
    // clap's `requires` is skipped when --model is given explicitly (it conflicts with --formula),
    // so the rule that --params belongs to --formula is also checked here.
    if a.formula.is_none() && !a.params.is_empty() {
        bail!("--params names the parameters of --formula; add --formula, or drop --params");
    }
    let weighting = prepared.weighting;
    if a.z.is_some() {
        return run_surface(project, a, prepared);
    }
    if a.x.is_some() {
        bail!("--x names the X column of a surface fit; add --z with the Z column, or drop --x");
    }
    let book = pick_book(project, a.book.as_deref())?;
    let sheet = pick_sheet(project, book, a.sheet.as_deref())?;
    let errors = named_columns(project, book, sheet, &a.yerr)?;
    if !errors.is_empty() && !matches!(weighting, FitWeighting::Instrumental | FitWeighting::Direct) {
        bail!("--yerr names the error or weight columns of --weight instrumental or direct; add one of them");
    }
    let columns: Vec<ColumnId> = if a.y.is_empty() {
        // Error columns given with --yerr are not fitted themselves.
        project.default_fit_columns(book, sheet).into_iter().filter(|c| !errors.contains(c)).collect()
    } else {
        named_columns(project, book, sheet, &a.y)?
    };
    if columns.is_empty() {
        bail!("there are no numeric Y columns to fit; name them with --y");
    }
    if errors.len() > 1 && errors.len() != columns.len() {
        bail!(
            "--yerr names {} columns for {} fitted columns; give one error column for all, or one per --y column",
            errors.len(),
            columns.len()
        );
    }
    let x_error = match &a.xerr {
        Some(n) => named_columns(project, book, sheet, std::slice::from_ref(n))?.first().copied(),
        None => None,
    };
    let mut requests = Vec::new();
    for (i, y) in columns.iter().enumerate() {
        let mut request = FitRequest::new(*y, prepared.model.clone());
        request.rows = prepared.rows;
        request.params = prepared.params.clone();
        request.weighting = weighting;
        request.y_error = errors.get(i).or(errors.first()).copied();
        request.confidence_level = a.confidence;
        request.bands = prepared.bands;
        request.band_level = a.level / 100.0;
        request.max_iterations = a.max_iterations;
        request.tolerance = a.tolerance;
        request.auto_init = !a.no_auto_init;
        request.scale_errors = !a.no_scale_errors;
        request.covariance_matrix = a.covariance_matrix;
        request.correlation_matrix = a.correlation_matrix;
        request.x_error = x_error;
        requests.push(request);
    }
    if !a.regress.is_empty() {
        return Ok(FitResult { book, run: FitRun::Regression(regress(project, a, book, sheet, columns[0])?) });
    }
    if let Some(other) = &a.compare {
        let mut second = requests[0].clone();
        second.model = model_from(other)?;
        second.params = Vec::new();
        let out = project.compare_models(&requests[0], &second)?;
        return Ok(FitResult { book, run: FitRun::Comparison(out) });
    }
    if !a.rank.is_empty() {
        let out = project.rank_models(&requests[0], &a.rank)?;
        return Ok(FitResult { book, run: FitRun::Ranking(out) });
    }
    let (summary, outcomes) = if a.global {
        let multi = project.fit_multi(&requests, Some(&a.share))?;
        (Some(multi.message), multi.outcomes)
    } else {
        let mut outcomes = Vec::new();
        for request in &requests {
            outcomes.push(project.fit(request)?);
        }
        (None, outcomes)
    };
    Ok(FitResult { book, run: FitRun::Fits { summary, outcomes, columns } })
}

/// A surface fit `z = f(x, y)`: --z is the Z column, --y (first entry) the Y column and --x the X
/// column; the two left out are the sheet's own (the X column to the left of Z, the nearest Y
/// column to its left).
fn run_surface(project: &mut Project, a: &FitSpec, prepared: &Prepared) -> Result<FitResult> {
    let unsupported = [
        (a.global, "--global"),
        (!a.regress.is_empty(), "--regress"),
        (a.compare.is_some(), "--compare"),
        (!a.rank.is_empty(), "--rank"),
        (a.xerr.is_some(), "--xerr"),
        (prepared.bands != BandChoice::None, "--bands"),
    ];
    if let Some((_, name)) = unsupported.iter().find(|(on, _)| *on) {
        bail!("{name} does not work with a surface fit (--z)");
    }
    if a.y.len() > 1 {
        bail!("a surface fit takes one --y column, the Y column of the surface");
    }
    let book = pick_book(project, a.book.as_deref())?;
    let sheet = pick_sheet(project, book, a.sheet.as_deref())?;
    let one = |name: &Option<String>| -> Result<Option<ColumnId>> {
        match name {
            Some(n) => Ok(named_columns(project, book, sheet, std::slice::from_ref(n))?.first().copied()),
            None => Ok(None),
        }
    };
    let z = one(&a.z)?.expect("--z is set");
    let (x, y) = (one(&a.x)?, named_columns(project, book, sheet, &a.y)?.first().copied());
    let errors = named_columns(project, book, sheet, &a.yerr)?;
    if !errors.is_empty() && !matches!(prepared.weighting, FitWeighting::Instrumental | FitWeighting::Direct) {
        bail!("--yerr names the error or weight columns of --weight instrumental or direct; add one of them");
    }
    let mut request = FitRequest::new(z, prepared.model.clone());
    request.surface = Some(SurfaceColumns { x, y });
    request.rows = prepared.rows;
    request.params = prepared.params.clone();
    request.weighting = prepared.weighting;
    request.y_error = errors.first().copied();
    request.confidence_level = a.confidence;
    request.max_iterations = a.max_iterations;
    request.tolerance = a.tolerance;
    request.auto_init = !a.no_auto_init;
    request.scale_errors = !a.no_scale_errors;
    request.covariance_matrix = a.covariance_matrix;
    request.correlation_matrix = a.correlation_matrix;
    let outcome = project.fit(&request)?;
    Ok(FitResult { book, run: FitRun::Fits { summary: None, outcomes: vec![outcome], columns: vec![z] } })
}

fn regress(project: &mut Project, a: &FitSpec, book: WindowId, sheet: usize, y: ColumnId) -> Result<MultiRegOutcome> {
    let xs = named_columns(project, book, sheet, &a.regress)?;
    let mut request = MultiRegRequest::new(y, xs);
    request.confidence_level = a.confidence;
    for text in &a.fix {
        let (n, v) = name_value(text, "--fix")?;
        if n != "Intercept" {
            bail!("--fix with --regress holds only the Intercept, as in --fix Intercept=0");
        }
        request.fix_intercept = Some(v);
    }
    Ok(project.fit_multiple_regression(&request)?)
}

/// The graph of a fit: the one it made, or a new one showing the data and the fitted curve.
pub fn fit_graph(project: &mut Project, outcome: &FitOutcome, source: ColumnId) -> Result<WindowId> {
    if outcome.surface {
        bail!(
            "a surface fit has no 2D graph (--plot is for curve fits); the fitted surface is in the FitSurfaceCurve sheet"
        );
    }
    Ok(match outcome.graphs.first() {
        Some(g) => *g,
        None => project.plot_fit(outcome, source)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn models_are_read_from_text() {
        assert_eq!(model_from("linear").unwrap(), FitModel::Linear);
        assert_eq!(model_from("poly:3").unwrap(), FitModel::Polynomial { order: 3 });
        assert_eq!(model_from("Polynomial:2").unwrap(), FitModel::Polynomial { order: 2 });
        assert_eq!(model_from("Gauss").unwrap(), FitModel::BuiltIn("Gauss".into()));
        assert!(model_from("poly").is_err());
        assert!(model_from("poly:0").is_err());
        assert!(model_from("poly:x").is_err());
    }

    #[test]
    fn parameter_options_are_read() {
        assert_eq!(name_value("xc=0.45", "--init").unwrap(), ("xc".into(), 0.45));
        assert_eq!(name_value(" A = -2 ", "--init").unwrap(), ("A".into(), -2.0));
        assert!(name_value("xc", "--init").is_err());
        assert!(name_value("xc=abc", "--init").is_err());
        assert!(name_value("xc=inf", "--init").is_err());
        assert_eq!(name_bounds("w=0:").unwrap(), ("w".into(), Some(0.0), None));
        assert_eq!(name_bounds("A=:5").unwrap(), ("A".into(), None, Some(5.0)));
        assert_eq!(name_bounds("A=-1:5").unwrap(), ("A".into(), Some(-1.0), Some(5.0)));
        assert!(name_bounds("A=1").is_err());
        assert!(name_bounds("A=x:1").is_err());
    }

    #[test]
    fn the_defaults_are_those_of_the_command_line() {
        let a = FitSpec::default();
        assert_eq!((a.model.as_str(), a.max_iterations, a.confidence, a.level), ("linear", 400, 0.95, 95.0));
        assert!(prepare(&a).is_ok());
    }
}
