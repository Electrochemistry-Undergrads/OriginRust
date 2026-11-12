//! `sheaf fit`: fit a model to worksheet columns without a window and print Origin-style tables.

use crate::analyze::{parse_pair, short_number};
use crate::args::{FitArgs, WeightKind};
use crate::commands::{
    is_project, load_project, named_columns, pick_book, pick_sheet, read_data_file, save_project, write_sheet,
};
use anyhow::{Context, Result, anyhow, bail};
use sheaf_core::Project;
use sheaf_core::analysis::RowRange;
use sheaf_core::fitting::{FitModel, FitOutcome, FitRequest, FitWeighting, ParamSetting, builtin_functions};
use sheaf_core::template::Template;

/// Reads `--model`: `linear`, `poly:N` or a built-in function name.
fn model_from(text: &str) -> Result<FitModel> {
    let t = text.trim();
    if t.eq_ignore_ascii_case("linear") || t.eq_ignore_ascii_case("line") {
        return Ok(FitModel::Linear);
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
fn name_value(text: &str, option: &str) -> Result<(String, f64)> {
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
fn name_bounds(text: &str) -> Result<(String, Option<f64>, Option<f64>)> {
    let err = || anyhow!("--bounds '{text}' must be written NAME=LOW:HIGH, for example --bounds w=0: or A=0:100");
    let (name, range) = text.split_once('=').ok_or_else(err)?;
    let (lo, hi) = range.split_once(':').ok_or_else(err)?;
    let side = |s: &str| -> Result<Option<f64>> {
        let s = s.trim();
        if s.is_empty() { Ok(None) } else { s.parse().map(Some).map_err(|_| err()) }
    };
    Ok((name.trim().to_string(), side(lo)?, side(hi)?))
}

/// Collects --init, --fix and --bounds into one setting per parameter name.
fn settings_from(a: &FitArgs) -> Result<Vec<ParamSetting>> {
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

fn list_functions() {
    let mut functions = builtin_functions();
    functions.sort_by(|a, b| a.priority.cmp(&b.priority).then(a.category.cmp(&b.category)).then(a.name.cmp(&b.name)));
    let width = functions.iter().map(|f| f.name.len()).max().unwrap_or(4);
    let mut category = String::new();
    for f in &functions {
        if f.category != category {
            category = f.category.clone();
            println!("\n{category}");
        }
        println!("  {:<width$}  {}", f.name, f.params.join(", "));
    }
    println!("\nlinear and poly:ORDER are also available; any formula can be fitted with --formula and --params.");
}

/// Prints rows of cells as aligned columns; the first row is the header.
fn print_columns(rows: &[Vec<String>]) {
    let n = rows.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<usize> =
        (0..n).map(|c| rows.iter().filter_map(|r| r.get(c)).map(|s| s.chars().count()).max().unwrap_or(0)).collect();
    for r in rows {
        let line: Vec<String> = r.iter().enumerate().map(|(c, s)| format!("{s:<w$}", w = widths[c])).collect();
        println!("{}", line.join("  ").trim_end());
    }
}

fn print_outcome(o: &FitOutcome) {
    let r = &o.report;
    println!("{}: {}", r.model, r.equation);
    let level = format!("{}%", short_number(r.confidence_level * 100.0));
    let mut rows = vec![
        [
            "Parameter",
            "Value",
            "Standard Error",
            "t-Value",
            "Prob>|t|",
            &format!("LCL {level}"),
            &format!("UCL {level}"),
            "",
        ]
        .map(String::from)
        .to_vec(),
    ];
    for p in &r.params {
        rows.push(vec![
            p.name.clone(),
            short_number(p.value),
            short_number(p.std_error),
            short_number(p.t_value),
            short_number(p.prob_t),
            short_number(p.lcl),
            short_number(p.ucl),
            if p.fixed { "fixed".to_string() } else { String::new() },
        ]);
    }
    print_columns(&rows);

    let s = &r.stats;
    println!();
    let mut stats = vec![
        vec!["Number of points".to_string(), s.n_points.to_string()],
        vec!["Degrees of freedom".to_string(), s.dof.to_string()],
        vec!["Residual sum of squares".to_string(), short_number(s.rss)],
        vec!["Reduced chi-square".to_string(), short_number(s.reduced_chi_sqr)],
        vec!["R-square (COD)".to_string(), short_number(s.r_squared)],
        vec!["Adj. R-square".to_string(), short_number(s.adj_r_squared)],
    ];
    if let Some(p) = s.pearson_r {
        stats.push(vec!["Pearson's r".to_string(), short_number(p)]);
    }
    if let Some(i) = s.iterations {
        stats.push(vec!["Iterations".to_string(), i.to_string()]);
    }
    for (name, value) in &o.derived {
        stats.push(vec![name.clone(), short_number(*value)]);
    }
    print_columns(&stats);

    if let Some(a) = &r.anova {
        println!();
        let mut rows = vec![
            ["", "DF", "Sum of Squares", "Mean Square", "F Value", "Prob>F"].map(String::from).to_vec(),
            vec![
                "Model".into(),
                a.model_df.to_string(),
                short_number(a.model_ss),
                short_number(a.model_ms),
                short_number(a.f_value),
                short_number(a.prob_f),
            ],
            vec!["Error".into(), a.error_df.to_string(), short_number(a.error_ss), short_number(a.error_ms)],
        ];
        if a.uncorrected_total_ss.is_finite() {
            rows.push(vec![
                "Uncorrected Total".into(),
                (a.model_df + a.error_df).to_string(),
                short_number(a.uncorrected_total_ss),
            ]);
        }
        rows.push(vec!["Corrected Total".into(), a.total_df.to_string(), short_number(a.total_ss)]);
        print_columns(&rows);
    }
    for w in &r.warnings {
        eprintln!("warning: {w}");
    }
    println!();
    println!("{}", o.message);
}

pub fn fit(a: FitArgs) -> Result<()> {
    if a.list {
        list_functions();
        return Ok(());
    }
    let input = a.input.as_ref().expect("clap requires an input unless --list is given");
    if !(a.confidence > 0.0 && a.confidence < 1.0) {
        bail!("--confidence must be between 0 and 1, for example 0.95");
    }
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
    let params = settings_from(&a)?;
    let rows = match (&a.rows, &a.x_range) {
        (Some(r), _) => {
            let (from, to) = parse_pair::<usize>(r, "row range", "120:480")?;
            RowRange::Rows(from, to)
        }
        (None, Some(r)) => {
            let (from, to) = parse_pair::<f64>(r, "X range", "0.2:0.6")?;
            RowRange::XBetween(from, to)
        }
        (None, None) => RowRange::All,
    };
    let weighting = match a.weight {
        WeightKind::None => FitWeighting::None,
        WeightKind::Instrumental => FitWeighting::Instrumental,
        WeightKind::Statistical => FitWeighting::Statistical,
    };

    let mut project = if is_project(input) {
        load_project(input)?
    } else {
        let mut p = Project::new();
        p.adopt_workbook(read_data_file(input, &a.read)?.workbook);
        p
    };
    let book = pick_book(&project, a.book.as_deref())?;
    let sheet = pick_sheet(&project, book, a.sheet.as_deref())?;
    let errors = named_columns(&project, book, sheet, &a.yerr)?;
    if !errors.is_empty() && weighting != FitWeighting::Instrumental {
        bail!("--yerr names the error columns of instrumental weighting; add --weight instrumental");
    }
    let columns: Vec<_> = if a.y.is_empty() {
        // Error columns given with --yerr are not fitted themselves.
        project.default_plot_columns(book, sheet, Template::Line).into_iter().filter(|c| !errors.contains(c)).collect()
    } else {
        named_columns(&project, book, sheet, &a.y)?
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

    let mut first: Option<(FitOutcome, sheaf_core::ColumnId)> = None;
    for (i, y) in columns.iter().enumerate() {
        if i > 0 {
            println!("\n----------------------------------------------------------------\n");
        }
        let mut request = FitRequest::new(*y, model.clone());
        request.rows = rows;
        request.params = params.clone();
        request.weighting = weighting;
        request.y_error = errors.get(i).or(errors.first()).copied();
        request.confidence_level = a.confidence;
        let outcome = project.fit(&request)?;
        print_outcome(&outcome);
        if first.is_none() {
            first = Some((outcome, *y));
        }
    }
    let (outcome, source) = first.expect("at least one column was fitted");

    if let Some(path) = &a.plot {
        crate::commands::require_positive("--dpi", Some(a.dpi))?;
        let graph = match outcome.graphs.first() {
            Some(g) => *g,
            None => project.plot_fit(&outcome, source)?,
        };
        sheaf_plot::export_file(&project, graph, path, a.dpi)
            .with_context(|| format!("could not write '{}'", path.display()))?;
        println!("wrote {}", path.display());
    }
    if let Some(path) = &a.export {
        write_sheet(&project.workbook(book).expect("picked above").sheets[outcome.param_sheet], path)?;
    }
    if let Some(path) = &a.save {
        save_project(&project, path)?;
    }
    Ok(())
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
}
