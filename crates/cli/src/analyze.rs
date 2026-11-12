//! `sheaf analyze`: run an analysis tool on worksheet columns without a window.

use crate::args::*;
use crate::commands::{
    is_project, load_project, named_columns, pick_book, pick_sheet, read_data_file, save_project, write_sheet,
};
use anyhow::{Result, anyhow, bail};
use sheaf_core::Project;
use sheaf_core::analysis::{self, BaselineSpec, IntegrateSpec, InterpSpec, Tool};
use sheaf_core::template::Template;

/// Reads "FROM:TO" as two numbers.
pub(crate) fn parse_pair<T: std::str::FromStr>(text: &str, what: &str, example: &str) -> Result<(T, T)> {
    let parsed = text.split_once(':').and_then(|(a, b)| Some((a.trim().parse().ok()?, b.trim().parse().ok()?)));
    parsed.ok_or_else(|| anyhow!("'{text}' is not a valid {what}; write it as FROM:TO, for example {example}"))
}

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

/// What was asked for. Sorting and filtering change the sheet as a whole; every other tool
/// works on Y columns.
enum Job {
    Tool(Tool),
    Sort { by: String, descending: bool },
    Filter { condition: String, in_place: bool },
}

fn job_from(tool: AnalyzeTool) -> Result<(TargetArgs, Job)> {
    Ok(match tool {
        AnalyzeTool::Integrate { target, baseline, absolute, no_cumulative } => (
            target,
            Job::Tool(Tool::Integrate(IntegrateSpec {
                area_type: if absolute { analysis::AreaType::Absolute } else { analysis::AreaType::Mathematical },
                end_point_baseline: baseline,
                cumulative: !no_cumulative,
            })),
        ),
        AnalyzeTool::Differentiate { target, order, smooth, poly, points } => {
            (target, Job::Tool(Tool::Differentiate(analysis::DiffOptions { order, smooth, poly, npts: points })))
        }
        AnalyzeTool::Smooth { target, method, points, poly, percent, boundary } => {
            let method = match method {
                SmoothKind::SavitzkyGolay => analysis::SmoothMethod::SavitzkyGolay { poly_order: poly },
                SmoothKind::Average => analysis::SmoothMethod::AdjacentAveraging { weighted: false },
                SmoothKind::WeightedAverage => analysis::SmoothMethod::AdjacentAveraging { weighted: true },
                SmoothKind::Percentile => analysis::SmoothMethod::Percentile { percent },
            };
            let boundary = match boundary {
                BoundaryKind::None => analysis::Boundary::None,
                BoundaryKind::Reflect => analysis::Boundary::Reflect,
                BoundaryKind::Repeat => analysis::Boundary::Repeat,
                BoundaryKind::Periodic => analysis::Boundary::Periodic,
                BoundaryKind::Extrapolate => analysis::Boundary::Extrapolate,
            };
            (target, Job::Tool(Tool::Smooth(analysis::SmoothOptions { method, npts: points, boundary })))
        }
        AnalyzeTool::Normalize { target, method, value } => {
            use analysis::NormalizeMethod as M;
            let method = match method {
                NormalizeKind::Max => M::Max,
                NormalizeKind::Range => M::ZeroToOne,
                NormalizeKind::Range100 => M::ZeroToHundred,
                NormalizeKind::ZScore => M::ZScore,
                NormalizeKind::Min => M::Min,
                NormalizeKind::Mean => M::Mean,
                NormalizeKind::Median => M::Median,
                NormalizeKind::Sd => M::Sd,
                NormalizeKind::Norm => M::Norm,
                NormalizeKind::Mode => M::Mode,
                NormalizeKind::Sum => M::Sum,
                NormalizeKind::Divide => {
                    M::Divide(value.ok_or_else(|| anyhow!("--method divide needs the divisor: --value NUMBER"))?)
                }
            };
            (target, Job::Tool(Tool::Normalize(method)))
        }
        AnalyzeTool::Baseline { target, constant, end_points, line } => {
            let spec = match (constant, end_points, line.as_slice()) {
                (Some(level), _, _) => BaselineSpec::Constant(level),
                (None, true, _) => BaselineSpec::EndPoints,
                (None, false, [x1, y1, x2, y2]) => BaselineSpec::Line((*x1, *y1), (*x2, *y2)),
                (None, false, []) => BaselineSpec::Minimum,
                _ => bail!("--line needs four numbers: X1,Y1,X2,Y2"),
            };
            (target, Job::Tool(Tool::SubtractBaseline(spec)))
        }
        AnalyzeTool::Interpolate { target, method, points, x_min, x_max } => {
            let method = match method {
                InterpKind::Linear => analysis::InterpMethod::Linear,
                InterpKind::Spline => analysis::InterpMethod::CubicSpline,
                InterpKind::Akima => analysis::InterpMethod::Akima,
            };
            let range = match (x_min, x_max) {
                (None, None) => None,
                (Some(lo), Some(hi)) => Some((lo, hi)),
                _ => bail!("give both --x-min and --x-max, or neither"),
            };
            (target, Job::Tool(Tool::Interpolate(InterpSpec { method, npts: points, range, ..InterpSpec::default() })))
        }
        AnalyzeTool::Peaks { target, direction, points, min_height, count, all } => {
            let direction = match direction {
                PeakKind::Both => analysis::PeakDirection::Both,
                PeakKind::Positive => analysis::PeakDirection::Positive,
                PeakKind::Negative => analysis::PeakDirection::Negative,
            };
            let filter = match (min_height, count, all) {
                (Some(h), _, _) => analysis::PeakFilter::Height(h),
                (None, Some(n), _) => analysis::PeakFilter::Number(n),
                (None, None, true) => analysis::PeakFilter::None,
                (None, None, false) => analysis::PeakFilter::Height(0.0),
            };
            (target, Job::Tool(Tool::FindPeaks(analysis::PeakOptions { npts: points, direction, filter })))
        }
        AnalyzeTool::Stats { target } => (target, Job::Tool(Tool::Statistics)),
        AnalyzeTool::Sort { target, by, descending } => {
            whole_sheet_only(&target, "sort")?;
            (target, Job::Sort { by, descending })
        }
        AnalyzeTool::Filter { target, condition, in_place } => {
            whole_sheet_only(&target, "filter")?;
            (target, Job::Filter { condition, in_place })
        }
    })
}

/// Sorting and filtering work on all rows and columns of a sheet; refuse the options that choose
/// rows or columns instead of silently ignoring them.
fn whole_sheet_only(target: &TargetArgs, tool: &str) -> Result<()> {
    let given: Vec<&str> =
        [(!target.y.is_empty(), "--y"), (target.rows.is_some(), "--rows"), (target.x_range.is_some(), "--x-range")]
            .into_iter()
            .filter_map(|(set, name)| set.then_some(name))
            .collect();
    if given.is_empty() {
        Ok(())
    } else {
        bail!("{tool} works on all rows and columns of the sheet; {} cannot be used with it", given.join(", "))
    }
}

pub fn analyze(a: AnalyzeArgs) -> Result<()> {
    let (target, job) = job_from(a.tool)?;
    let mut project = if is_project(&target.input) {
        load_project(&target.input)?
    } else {
        let mut p = Project::new();
        p.adopt_workbook(read_data_file(&target.input, &target.read)?.workbook);
        p
    };
    let book = pick_book(&project, target.book.as_deref())?;
    let sheet = pick_sheet(&project, book, target.sheet.as_deref())?;

    // The sheet that holds the results, for --export.
    let result_sheet = match job {
        Job::Tool(tool) => {
            let columns = if target.y.is_empty() {
                project.default_plot_columns(book, sheet, Template::Line)
            } else {
                named_columns(&project, book, sheet, &target.y)?
            };
            if columns.is_empty() {
                bail!("there are no numeric Y columns to work on; name them with --y");
            }
            let rows = match (&target.rows, &target.x_range) {
                (Some(r), _) => {
                    let (from, to) = parse_pair::<usize>(r, "row range", "120:480")?;
                    analysis::RowRange::Rows(from, to)
                }
                (None, Some(r)) => {
                    let (from, to) = parse_pair::<f64>(r, "X range", "0.2:0.6")?;
                    analysis::RowRange::XBetween(from, to)
                }
                (None, None) => analysis::RowRange::All,
            };
            let report = project.analyze(&analysis::Request { columns, rows, tool })?;
            if let Some(t) = &report.table {
                print_table(t);
            }
            println!("{}", report.message);
            report.new_sheet.unwrap_or(sheet)
        }
        Job::Sort { by, descending } => {
            let key = named_columns(&project, book, sheet, std::slice::from_ref(&by))?[0];
            project.sort_sheet(book, sheet, key, descending)?;
            println!("sorted by column {by}");
            sheet
        }
        Job::Filter { condition, in_place } => {
            let before = project.workbook(book).expect("picked above").sheets[sheet].row_count();
            let (kept, at) = project.filter_sheet(book, sheet, &condition, !in_place)?;
            let name = &project.workbook(book).expect("picked above").sheets[at].name;
            println!("kept {kept} of {before} rows in sheet {name}");
            at
        }
    };

    if let Some(path) = &target.export {
        write_sheet(&project.workbook(book).expect("picked above").sheets[result_sheet], path)?;
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

    #[test]
    fn ranges_are_read_as_from_colon_to() {
        assert_eq!(parse_pair::<usize>("120:480", "row range", "120:480").unwrap(), (120, 480));
        assert_eq!(parse_pair::<f64>("-0.5: 0.1", "X range", "0.2:0.6").unwrap(), (-0.5, 0.1));
        assert!(parse_pair::<usize>("12-48", "row range", "120:480").is_err());
        assert!(parse_pair::<usize>("a:3", "row range", "120:480").is_err());
    }
}
