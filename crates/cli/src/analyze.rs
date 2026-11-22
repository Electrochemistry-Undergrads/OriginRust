//! `sheaf analyze`: run an analysis tool on worksheet columns without a window.

use crate::args::*;
use crate::commands::{is_project, load_project, read_data_file, save_project, write_sheet};
use anyhow::Result;
use sheaf_core::Project;
use sheaf_core::analysis;
use sheaf_ops::analyze::{Outcome, Target};
use sheaf_ops::cli_analyze::{job_from, power_spec};

/// A number with six significant digits, without trailing zeros.
pub(crate) fn short_number(v: f64) -> String {
    if v.is_nan() {
        return "--".to_string();
    }
    if v == 0.0 || !v.is_finite() {
        return format!("{v}");
    }
    let magnitude = v.abs().log10().floor() as i32;
    if !(-5..=8).contains(&magnitude) {
        return format!("{v:.5e}");
    }
    let text = format!("{:.*}", (5 - magnitude).max(0) as usize, v);
    if text.contains('.') { text.trim_end_matches('0').trim_end_matches('.').to_string() } else { text }
}

fn print_table(t: &analysis::ResultTable) {
    let headers: Vec<String> = t
        .headers
        .iter()
        .enumerate()
        .map(|(i, h)| match t.units.get(i).map(String::as_str) {
            Some(u) if !u.is_empty() => format!("{h} ({u})"),
            _ => h.clone(),
        })
        .collect();
    // Where the rows have different units (A and mA columns), each number carries its own unit.
    let cells: Vec<Vec<String>> = t
        .rows
        .iter()
        .enumerate()
        .map(|(r, (_, v))| {
            v.iter()
                .enumerate()
                .map(|(c, x)| match t.unit(r, c) {
                    u if t.mixed_units(c) && !u.is_empty() => format!("{} {u}", short_number(*x)),
                    _ => short_number(*x),
                })
                .collect()
        })
        .collect();
    let label_width = t.rows.iter().map(|(l, _)| l.chars().count()).max().unwrap_or(0).max(4);
    let widths: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| cells.iter().map(|r| r[i].chars().count()).max().unwrap_or(0).max(h.chars().count()))
        .collect();
    let mut line = format!("{:<label_width$}", "Data");
    for (h, w) in headers.iter().zip(&widths) {
        line.push_str(&format!("  {h:>w$}"));
    }
    println!("{line}");
    for ((label, _), row) in t.rows.iter().zip(&cells) {
        let mut line = format!("{label:<label_width$}");
        for (c, w) in row.iter().zip(&widths) {
            line.push_str(&format!("  {c:>w$}"));
        }
        println!("{line}");
    }
}

/// `analyze power`: no data file, the table goes to the terminal (and `--export`).
fn power(a: AnalyzeTool) -> Result<()> {
    let (spec, export) = power_spec(a)?;
    let (project, report) = sheaf_ops::analyze::run_power(&spec)?;
    let ws = &project.workbook(report.book).expect("just added").sheets[0];
    println!("{:>12}  {:>12}", "Sample Size", "Power");
    let (size, power) = (ws.columns[0].numeric().unwrap_or(&[]), ws.columns[1].numeric().unwrap_or(&[]));
    for (n, p) in size.iter().zip(power) {
        println!("{:>12}  {:>12}", short_number(*n), short_number(*p));
    }
    for note in &report.notes {
        println!("{note}");
    }
    if let Some(path) = export {
        write_sheet(ws, &path)?;
    }
    Ok(())
}

/// `analyze acceptance`: no data file; the plan and its numbers go to the terminal (T369).
fn acceptance(a: AnalyzeTool) -> Result<()> {
    let AnalyzeTool::Acceptance { aql, ltpd, alpha, beta, n, c, lot, export, save } = a else {
        unreachable!("called for the acceptance tool only");
    };
    let spec = sheaf_core::analysis::AcceptanceSpec { aql, ltpd, alpha, beta, plan: n.zip(c), lot };
    let mut project = Project::new();
    let report = project.acceptance_sampling(&spec)?;
    let book = project.workbook(report.book).expect("just added");
    let plan = &book.sheets[0];
    let values = plan.columns[1].numeric().unwrap_or(&[]);
    for (i, v) in values.iter().enumerate() {
        println!("{:<28}  {}", plan.columns[0].shown_text(i), short_number(*v));
    }
    if let Some(path) = export {
        write_sheet(&book.sheets[1], &path)?;
    }
    if let Some(path) = save {
        save_project(&project, &path)?;
    }
    Ok(())
}

pub fn analyze(a: AnalyzeArgs) -> Result<()> {
    if matches!(a.tool, AnalyzeTool::Power { .. }) {
        return power(a.tool);
    }
    if matches!(a.tool, AnalyzeTool::Acceptance { .. }) {
        return acceptance(a.tool);
    }
    let plot = sheaf_ops::cli_analyze::comparison_plot_request(&a.tool)?;
    let (target, job) = job_from(a.tool)?;
    let mut project = if is_project(&target.input) {
        load_project(&target.input)?
    } else {
        let mut p = Project::new();
        p.adopt_workbook(read_data_file(&target.input, &crate::commands::read_settings(&target.read)?)?.workbook);
        p
    };
    let spec = Target {
        y: target.y.clone(),
        book: target.book.clone(),
        sheet: target.sheet.clone(),
        rows: target.rows.clone(),
        x_range: target.x_range.clone(),
        read: crate::commands::read_settings(&target.read)?,
    };
    let run = sheaf_ops::analyze::run_job(&mut project, &spec, job)?;
    for w in &run.load_warnings {
        eprintln!("note: {w}");
    }
    crate::commands::remember_foreign(&run.foreign_inputs);
    match &run.outcome {
        Outcome::Report(report) => {
            if let Some(t) = &report.table {
                print_table(t);
            }
            for note in &report.notes {
                println!("{note}");
            }
            println!("{}", report.message);
        }
        Outcome::Sorted { by } => println!("sorted by column {by}"),
        Outcome::Filtered { kept, before, sheet_name } => {
            println!("kept {kept} of {before} rows in sheet {sheet_name}")
        }
    }
    if let Some(request) = &plot {
        let graph = sheaf_ops::analyze::write_comparison_plot(&mut project, &run, request)?;
        let name = project.graph(graph).map_or_else(String::new, |g| g.name.clone());
        println!("multiple comparisons plot {name} written to {}", request.path.display());
    }
    if let Some(path) = &target.export {
        write_sheet(&project.workbook(run.book).expect("picked above").sheets[run.result_sheet], path)?;
    }
    if let Some(path) = &target.save {
        save_project(&project, path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_print_with_six_significant_digits() {
        assert_eq!(short_number(20.0), "20");
        assert_eq!(short_number(0.000123456789), "0.000123457");
        assert_eq!(short_number(-1234.5678), "-1234.57");
        assert_eq!(short_number(1.5e-9), "1.50000e-9");
        assert_eq!(short_number(2.5e12), "2.50000e12");
        assert_eq!(short_number(0.0), "0");
        assert_eq!(short_number(f64::NAN), "--");
        assert_eq!(short_number(123456789.0), "123456789");
    }
}
