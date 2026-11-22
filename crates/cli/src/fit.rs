//! `sheaf fit`: fit a model to worksheet columns without a window and print Origin-style tables.

use crate::analyze::short_number;
use crate::args::FitArgs;
use crate::commands::{is_project, load_project, read_data_file, read_settings, save_project, write_sheet};
use anyhow::{Context, Result};
use sheaf_core::Project;
use sheaf_core::fitting::FitOutcome;
use sheaf_ops::cli_fit::fit_spec;
use sheaf_ops::fit::FitRun;

fn list_functions() {
    let functions = sheaf_ops::fit::list_models();
    let width = functions.iter().map(|f| f.name.len()).max().unwrap_or(4);
    let mut category = String::new();
    for f in &functions {
        if f.category != category {
            category = f.category.clone();
            println!(
                "
{category}"
            );
        }
        println!("  {:<width$}  {}", f.name, f.parameters.join(", "));
    }
    println!(
        "
linear and poly:ORDER are also available; any formula can be fitted with --formula and --params."
    );
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
    if !o.band_columns.is_empty() {
        let level = o.bands.as_ref().map_or(f64::NAN, |b| b.level * 100.0);
        println!();
        println!("Bands at {}% confidence level, written to the curve sheet:", short_number(level));
        for b in &o.band_columns {
            println!("  {}", b.what);
        }
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
    let spec = fit_spec(&a);
    let prepared = sheaf_ops::fit::prepare(&spec)?;
    let mut project = if is_project(input) {
        load_project(input)?
    } else {
        let mut p = Project::new();
        p.adopt_workbook(read_data_file(input, &read_settings(&a.read)?)?.workbook);
        p
    };
    let result = sheaf_ops::fit::run(&mut project, &spec, &prepared)?;
    let book = result.book;
    match &result.run {
        FitRun::Regression(out) => print_regression(out),
        FitRun::Comparison(out) => print_comparison(out),
        FitRun::Ranking(out) => print_ranking(out),
        FitRun::BestSubsets(out) => print_best_subsets(out),
        FitRun::Fits { summary, outcomes, .. } => {
            if let Some(message) = summary {
                println!(
                    "{message}
"
                );
            }
            for (i, outcome) in outcomes.iter().enumerate() {
                if i > 0 {
                    println!(
                        "
----------------------------------------------------------------
"
                    );
                }
                print_outcome(outcome);
            }
        }
    }
    if let FitRun::Fits { outcomes, columns, .. } = &result.run
        && let Some(path) = &a.plot
    {
        crate::commands::require_positive("--dpi", Some(a.dpi))?;
        let graph = sheaf_ops::fit::fit_graph(&mut project, &outcomes[0], columns[0])?;
        sheaf_plot::export_file(&project, graph, path, a.dpi)
            .with_context(|| format!("could not write '{}'", path.display()))?;
        println!("wrote {}", path.display());
    }
    if let Some(path) = &a.export {
        write_sheet(&project.workbook(book).expect("picked above").sheets[result.result_sheet()], path)?;
    }
    if let Some(path) = &a.save {
        save_project(&project, path)?;
    }
    Ok(())
}

fn print_regression(out: &sheaf_core::fitting::MultiRegOutcome) {
    println!(
        "{}: {}
",
        out.report.model, out.report.equation
    );
    let mut rows = vec![["Parameter", "Value", "Standard Error", "t-Value", "Prob>|t|", ""].map(String::from).to_vec()];
    for p in &out.report.params {
        rows.push(vec![
            p.name.clone(),
            short_number(p.value),
            short_number(p.std_error),
            short_number(p.t_value),
            short_number(p.prob_t),
            if p.fixed { "fixed".to_string() } else { String::new() },
        ]);
    }
    print_columns(&rows);
    let s = &out.report.stats;
    println!();
    print_columns(&[
        vec!["Number of points".to_string(), s.n_points.to_string()],
        vec!["Degrees of freedom".to_string(), s.dof.to_string()],
        vec!["Residual sum of squares".to_string(), short_number(s.rss)],
        vec!["R-square (COD)".to_string(), short_number(s.r_squared)],
        vec!["Adj. R-square".to_string(), short_number(s.adj_r_squared)],
    ]);
    if let Some(an) = &out.report.anova {
        println!(
            "
F = {}, Prob>F = {}",
            short_number(an.f_value),
            short_number(an.prob_f)
        );
    }
    println!(
        "
{}",
        out.message
    );
}

fn print_comparison(o: &sheaf_core::fitting::CompareOutcome) {
    use sheaf_core::fitting::Verdict;
    let c = &o.comparison;
    let names = [c.models[0].name.clone(), c.models[1].name.clone()];
    let verdict = |v: Verdict| match v {
        Verdict::Model(i) => format!("{} is preferred", names[i]),
        Verdict::Inconclusive => "inconclusive".to_string(),
        Verdict::NoConclusion => "no conclusion".to_string(),
    };
    let mut rows = vec![vec![String::new(), names[0].clone(), names[1].clone()]];
    let mut row = |q: &str, x: String, y: String| rows.push(vec![q.to_string(), x, y]);
    row("Points", c.models[0].n.to_string(), c.models[1].n.to_string());
    row("Parameters", c.models[0].params.to_string(), c.models[1].params.to_string());
    row("Residual sum of squares", short_number(c.models[0].rss), short_number(c.models[1].rss));
    row("AICc", short_number(c.aic[0]), short_number(c.aic[1]));
    row("Akaike weight", short_number(c.aic_weight[0]), short_number(c.aic_weight[1]));
    row("BIC", short_number(c.bic[0]), short_number(c.bic[1]));
    row("BIC minus the smaller", short_number(c.bic_delta[0]), short_number(c.bic_delta[1]));
    print_columns(&rows);
    println!();
    println!("AICc:   {}", verdict(c.aic_verdict));
    println!("BIC:    {}", verdict(c.bic_verdict));
    println!(
        "F-test: F = {}, df1 = {}, df2 = {}, Prob>F = {}: {}",
        short_number(c.ftest.f),
        c.ftest.df1,
        c.ftest.df2,
        short_number(c.ftest.prob),
        verdict(c.ftest.verdict)
    );
    for w in &c.warnings {
        eprintln!("warning: {w}");
    }
    println!("\n{}", o.message);
}

fn print_best_subsets(o: &sheaf_core::fitting::BestSubsetOutcome) {
    let head = ["Predictors", "k", "R^2", "Adj. R^2", "Cp", "AIC", "BIC", "Best by"].map(String::from).to_vec();
    let mut table = vec![head];
    let r = &o.result;
    for (i, m) in r.models.iter().enumerate() {
        let best: Vec<&str> = [(r.best_adj_r2, "adj R^2"), (r.best_cp, "Cp"), (r.best_aic, "AIC"), (r.best_bic, "BIC")]
            .iter()
            .filter(|(b, _)| *b == i)
            .map(|(_, t)| *t)
            .collect();
        table.push(vec![
            m.vars.iter().map(|&j| o.names[j].clone()).collect::<Vec<_>>().join(", "),
            m.vars.len().to_string(),
            short_number(m.r_squared),
            short_number(m.adj_r_squared),
            short_number(m.cp),
            short_number(m.aic),
            short_number(m.bic),
            best.join(", "),
        ]);
    }
    print_columns(&table);
    println!("\n{}", o.message);
}

fn print_ranking(o: &sheaf_core::fitting::RankOutcome) {
    let head =
        ["Rank", "Function", "Parameters", "RSS", "AICc", "AICc delta", "Weight", "BIC"].map(String::from).to_vec();
    let mut table = vec![head];
    for (i, r) in o.ranking.iter().enumerate() {
        table.push(vec![
            (i + 1).to_string(),
            r.summary.name.clone(),
            r.summary.params.to_string(),
            short_number(r.summary.rss),
            short_number(r.aic),
            short_number(r.aic_delta),
            short_number(r.aic_weight),
            short_number(r.bic),
        ]);
    }
    print_columns(&table);
    for (f, e) in &o.skipped {
        eprintln!("warning: {f} was skipped: {e}");
    }
    println!("\n{}", o.message);
}
