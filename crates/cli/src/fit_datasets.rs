//! `sheaf fit-implicit` and `sheaf compare-datasets`.

use crate::analyze::short_number;
use crate::args::{CompareDatasetsArgs, FitImplicitArgs};
use crate::commands::{is_project, load_project, read_data_file, read_settings, save_project, write_sheet};
use anyhow::Result;
use sheaf_core::Project;
use sheaf_ops::fit_datasets::{CompareDatasetsOps, ImplicitOps, run_compare_datasets, run_implicit};
use std::path::{Path, PathBuf};

fn load(input: &Path, read: &crate::args::ReadOptions) -> Result<Project> {
    if is_project(input) {
        return load_project(input);
    }
    let mut p = Project::new();
    p.adopt_workbook(read_data_file(input, &read_settings(read)?)?.workbook);
    Ok(p)
}

fn print_rows(rows: &[Vec<String>]) {
    let n = rows.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<usize> =
        (0..n).map(|c| rows.iter().filter_map(|r| r.get(c)).map(|s| s.chars().count()).max().unwrap_or(0)).collect();
    for r in rows {
        let line: Vec<String> = r.iter().enumerate().map(|(c, s)| format!("{s:<w$}", w = widths[c])).collect();
        println!("{}", line.join("  ").trim_end());
    }
}

fn finish(
    project: &Project,
    book: sheaf_core::WindowId,
    sheet: usize,
    out: &Option<PathBuf>,
    save: &Option<PathBuf>,
) -> Result<()> {
    if let Some(path) = out {
        write_sheet(&project.workbook(book).expect("the run found it").sheets[sheet], path)?;
    }
    if let Some(path) = save {
        save_project(project, path)?;
    }
    Ok(())
}

pub fn fit_implicit(a: FitImplicitArgs) -> Result<()> {
    let mut project = load(&a.input, &a.read)?;
    let spec = ImplicitOps {
        function: a.function,
        x: a.x,
        y: a.y,
        init: a.init,
        rows: a.rows,
        x_range: a.x_range,
        confidence: a.confidence,
        max_iterations: a.max_iterations,
        tolerance: a.tolerance,
        book: a.book,
        sheet: a.sheet,
    };
    let out = run_implicit(&mut project, &spec)?;
    let r = &out.report;
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
        ]);
    }
    print_rows(&rows);
    let s = &r.stats;
    println!();
    print_rows(&[
        vec!["Number of points".to_string(), s.n_points.to_string()],
        vec!["Degrees of freedom".to_string(), s.dof.to_string()],
        vec!["Residual sum of squares".to_string(), short_number(s.rss)],
        vec!["Reduced chi-square".to_string(), short_number(s.reduced_chi_sqr)],
        vec!["R-square (COD)".to_string(), short_number(s.r_squared)],
        vec!["Adj. R-square".to_string(), short_number(s.adj_r_squared)],
    ]);
    for w in &r.warnings {
        eprintln!("warning: {w}");
    }
    println!("\n{}", out.message);
    finish(&project, out.book, out.param_sheet, &a.output, &a.save)
}

pub fn compare_datasets(a: CompareDatasetsArgs) -> Result<()> {
    let mut project = load(&a.input, &a.read)?;
    let spec = CompareDatasetsOps {
        model: a.model,
        y1: a.y1,
        y2: a.y2,
        init: a.init,
        name1: a.name1,
        name2: a.name2,
        no_parameters: a.no_parameters,
        no_statistics: a.no_statistics,
        book: a.book,
        sheet: a.sheet,
    };
    let out = run_compare_datasets(&mut project, &spec)?;
    let ws = &project.workbook(out.book).expect("the run found it").sheets[out.sheet];
    let text = |i: usize, r: usize| ws.columns[i].text().and_then(|t| t.get(r)).cloned().unwrap_or_default();
    let num = |i: usize, r: usize| {
        ws.columns[i]
            .numeric()
            .and_then(|v| v.get(r))
            .filter(|v| v.is_finite())
            .map(|v| short_number(*v))
            .unwrap_or_default()
    };
    let mut rows =
        vec![vec![String::new(), ws.columns[1].long_name.clone(), ws.columns[2].long_name.clone(), String::new()]];
    for r in 0..ws.row_count() {
        rows.push(vec![text(0, r), num(1, r), num(2, r), text(3, r)]);
    }
    print_rows(&rows);
    for w in &out.comparison.warnings {
        eprintln!("warning: {w}");
    }
    println!("\n{}", out.message);
    finish(&project, out.book, out.sheet, &a.output, &a.save)
}
