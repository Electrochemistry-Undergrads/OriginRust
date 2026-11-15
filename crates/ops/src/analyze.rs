//! Running an analysis tool on worksheet columns.
//!
//! A [`Job`] is what was asked for with every column still given by name; [`run_job`] looks the
//! names up in the project, runs the tool and returns what happened.

use crate::pick::{named_columns, pick_book, pick_sheet};
use crate::read::{ReadSettings, read_data_file};
use crate::session::{is_project, load_project};
use anyhow::{Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use sheaf_core::analysis::{self, Tool};
use sheaf_core::template::Template;
use sheaf_core::{Project, WindowId};
use std::path::PathBuf;

/// Reads "FROM:TO" as two numbers.
pub fn parse_pair<T: std::str::FromStr>(text: &str, what: &str, example: &str) -> Result<(T, T)> {
    let parsed = text.split_once(':').and_then(|(a, b)| Some((a.trim().parse().ok()?, b.trim().parse().ok()?)));
    parsed.ok_or_else(|| anyhow!("'{text}' is not a valid {what}; write it as FROM:TO, for example {example}"))
}

/// The rows an analysis or fit works on, as "FROM:TO" text: worksheet rows counted from 1, or the
/// first unbroken run of rows whose X value lies in an interval. `rows` wins over `x_range`.
pub fn row_range(rows: Option<&str>, x_range: Option<&str>) -> Result<analysis::RowRange> {
    Ok(match (rows, x_range) {
        (Some(r), _) => {
            let (from, to) = parse_pair::<usize>(r, "row range", "120:480")?;
            analysis::RowRange::Rows(from, to)
        }
        (None, Some(r)) => {
            let (from, to) = parse_pair::<f64>(r, "X range", "0.2:0.6")?;
            analysis::RowRange::XBetween(from, to)
        }
        (None, None) => analysis::RowRange::All,
    })
}

/// The second operand of curve math: a number, or a reference column.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OperandSpec {
    pub value: Option<f64>,
    /// The Y column of the reference curve (short or long name).
    pub reference: Option<String>,
    /// Take the reference from this data file (or project).
    pub ref_file: Option<PathBuf>,
    pub ref_book: Option<String>,
    pub ref_sheet: Option<String>,
    /// Only the X range that data and reference both cover.
    pub common: bool,
}

/// What was asked for. Sorting and filtering change the sheet as a whole; every other tool
/// works on Y columns.
pub enum Job {
    Tool(Tool),
    /// Coherence; the second signal is looked up once the project is loaded.
    Coherence {
        other: String,
        options: analysis::CoherenceOptions,
    },
    /// Curve math; the reference column is looked up once the project is loaded.
    Math {
        op: analysis::CurveOp,
        operand: OperandSpec,
        /// Simple Column Math: row by row, no X.
        column: bool,
    },
    /// Convolution or correlation; the second column is looked up once the project is loaded.
    Signal {
        conv: bool,
        other: String,
        circular: bool,
        normalize: bool,
        interval: Option<f64>,
    },
    /// Inverse FFT; the imaginary column is looked up once the project is loaded.
    Ifft {
        imaginary: Option<String>,
        options: analysis::IfftOptions,
        auto_interval: bool,
    },
    /// Graphical summary; the group columns are looked up once the project is loaded.
    GraphSummary {
        groups: Vec<String>,
        level: f64,
        arrangement: analysis::GraphArrangement,
    },
    Sort {
        by: String,
        descending: bool,
    },
    Filter {
        condition: String,
        in_place: bool,
    },
}

/// Which data an analysis works on.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Target {
    /// Y columns by short or long name (default: the columns a plot of the sheet would show).
    pub y: Vec<String>,
    pub book: Option<String>,
    pub sheet: Option<String>,
    /// "FROM:TO" worksheet rows.
    pub rows: Option<String>,
    /// "FROM:TO" interval of X.
    pub x_range: Option<String>,
    /// How a reference file of curve math is read.
    pub read: ReadSettings,
}

/// What a job did.
pub enum Outcome {
    /// A tool ran; the report says where the results went.
    Report(analysis::Report),
    Sorted {
        by: String,
    },
    Filtered {
        kept: usize,
        before: usize,
        sheet_name: String,
    },
}

/// A multiple comparisons plot to draw after a one-way ANOVA or two-sample t-test (`--comparison-plot`).
#[derive(Clone, Debug)]
pub struct ComparisonPlotRequest {
    pub path: PathBuf,
    pub options: sheaf_core::compplot::ComparisonPlotOptions,
}

/// Makes the multiple comparisons plot of the report `run` wrote and writes it to the request's file (the
/// format follows the extension). The graph stays in the project; its window id is returned.
pub fn write_comparison_plot(project: &mut Project, run: &Run, request: &ComparisonPlotRequest) -> Result<WindowId> {
    let Outcome::Report(report) = &run.outcome else { bail!("the tool gave no report to draw") };
    let sheet = report.new_sheet.ok_or_else(|| anyhow!("the report has no sheet to draw"))?;
    let graph = project.comparison_plot(run.book, sheet, &request.options).map_err(|e| anyhow!("{e}"))?;
    crate::render::write_graph(project, graph, &request.path, &crate::render::ExportSettings::default())?;
    Ok(graph)
}

/// The result of [`run_job`].
pub struct Run {
    pub book: WindowId,
    /// The sheet the job started from.
    pub sheet: usize,
    /// The sheet that holds the results.
    pub result_sheet: usize,
    pub outcome: Outcome,
    /// Warnings of a reference project that was opened.
    pub load_warnings: Vec<String>,
    /// Origin projects that were read for a reference.
    pub foreign_inputs: Vec<PathBuf>,
}

/// Finds the reference column (loading its file if one was named) and builds the tool.
fn math_tool(
    project: &mut Project,
    book: WindowId,
    sheet: usize,
    op: analysis::CurveOp,
    operand: &OperandSpec,
    column: bool,
    read: &ReadSettings,
    run: &mut Run,
) -> Result<Tool> {
    let operand_value = match (&operand.reference, operand.value) {
        (None, Some(v)) => analysis::CurveOperand::Constant(v),
        (None, None) => bail!("give the second operand: --value NUMBER or --ref COLUMN"),
        (Some(name), _) => {
            let (rb, rs) = match &operand.ref_file {
                Some(file) => {
                    let loaded = if is_project(file) {
                        let l = load_project(file)?;
                        run.load_warnings.extend(l.warnings);
                        if l.foreign {
                            run.foreign_inputs.push(file.clone());
                        }
                        l.project
                    } else {
                        let mut p = Project::new();
                        p.adopt_workbook(read_data_file(file, read)?.workbook);
                        p
                    };
                    let from = pick_book(&loaded, operand.ref_book.as_deref())?;
                    let book = loaded.workbook(from).expect("picked above").clone();
                    let id = project.adopt_workbook(book);
                    (id, pick_sheet(project, id, operand.ref_sheet.as_deref())?)
                }
                None => {
                    let rb = match &operand.ref_book {
                        Some(n) => pick_book(project, Some(n))?,
                        None => book,
                    };
                    let rs = match (&operand.ref_sheet, rb == book) {
                        (Some(n), _) => pick_sheet(project, rb, Some(n))?,
                        (None, true) => sheet,
                        (None, false) => pick_sheet(project, rb, None)?,
                    };
                    (rb, rs)
                }
            };
            analysis::CurveOperand::Reference(named_columns(project, rb, rs, std::slice::from_ref(name))?[0])
        }
    };
    if column {
        if operand.common {
            bail!("--common only applies to curve math; column math goes row by row");
        }
        return Ok(Tool::ColumnMath(analysis::ColumnMathSpec { op, operand: operand_value }));
    }
    if operand.common && matches!(operand_value, analysis::CurveOperand::Constant(_)) {
        bail!("--common only applies to a reference curve");
    }
    Ok(Tool::CurveMath(analysis::CurveMathSpec { op, operand: operand_value, common_only: operand.common }))
}

/// Runs a job on the project.
pub fn run_job(project: &mut Project, target: &Target, job: Job) -> Result<Run> {
    run_job_to(project, target, job, &analysis::OutputTarget::NextToInput)
}

/// Runs a job with a choice of destination for per-row results. Table-producing tools retain
/// their intrinsic destinations. Sorting and filtering have their own sheet-level semantics.
pub fn run_job_to(project: &mut Project, target: &Target, job: Job, output: &analysis::OutputTarget) -> Result<Run> {
    if !matches!(output, analysis::OutputTarget::NextToInput) && matches!(&job, Job::Sort { .. } | Job::Filter { .. }) {
        bail!("sort and filter require next_to_input output");
    }
    let book = pick_book(project, target.book.as_deref())?;
    let sheet = pick_sheet(project, book, target.sheet.as_deref())?;
    let mut run = Run {
        book,
        sheet,
        result_sheet: sheet,
        outcome: Outcome::Sorted { by: String::new() },
        load_warnings: Vec::new(),
        foreign_inputs: Vec::new(),
    };
    let tool = match job {
        Job::Tool(tool) => tool,
        Job::Math { op, operand, column } => {
            math_tool(project, book, sheet, op, &operand, column, &target.read, &mut run)?
        }
        Job::Coherence { other, options } => {
            let other = named_columns(project, book, sheet, std::slice::from_ref(&other))?[0];
            Tool::Coherence(analysis::CoherenceSpec { other, options })
        }
        Job::Ifft { imaginary, options, auto_interval } => {
            let imaginary = match imaginary {
                Some(name) => Some(named_columns(project, book, sheet, std::slice::from_ref(&name))?[0]),
                None => None,
            };
            Tool::Ifft(analysis::IfftSpec { imaginary, options, auto_interval })
        }
        Job::Signal { conv, other, circular, normalize, interval } => {
            let other = named_columns(project, book, sheet, std::slice::from_ref(&other))?[0];
            if conv {
                Tool::Convolution(analysis::ConvolutionSpec { response: other, circular, normalize, interval })
            } else {
                let interval = interval.unwrap_or(1.0);
                Tool::SignalCorrelation(analysis::SignalCorrelationSpec { other, circular, normalize, interval })
            }
        }
        Job::GraphSummary { groups, level, arrangement } => {
            let groups = if groups.is_empty() { Vec::new() } else { named_columns(project, book, sheet, &groups)? };
            Tool::GraphSummary(analysis::GraphSummarySpec { groups, level, arrangement })
        }
        Job::Sort { by, descending } => {
            let key = named_columns(project, book, sheet, std::slice::from_ref(&by))?[0];
            project.sort_sheet(book, sheet, key, descending)?;
            run.outcome = Outcome::Sorted { by };
            return Ok(run);
        }
        Job::Filter { condition, in_place } => {
            let before = project.workbook(book).expect("picked above").sheets[sheet].row_count();
            let (kept, at) = project.filter_sheet(book, sheet, &condition, !in_place)?;
            let sheet_name = project.workbook(book).expect("picked above").sheets[at].name.clone();
            run.result_sheet = at;
            run.outcome = Outcome::Filtered { kept, before, sheet_name };
            return Ok(run);
        }
    };
    let columns = if target.y.is_empty() {
        project.default_plot_columns(book, sheet, Template::Line)
    } else {
        named_columns(project, book, sheet, &target.y)?
    };
    if columns.is_empty() {
        bail!("there are no numeric Y columns to work on; name them with --y");
    }
    let rows = row_range(target.rows.as_deref(), target.x_range.as_deref())?;
    let report = project.analyze_to(&analysis::Request { columns, rows, tool }, output)?;
    run.result_sheet = report.new_sheet.unwrap_or(sheet);
    run.outcome = Outcome::Report(report);
    Ok(run)
}

/// The power and sample size tool: it needs no data, and its table is in a new project.
pub fn run_power(spec: &analysis::PowerSpec) -> Result<(Project, analysis::Report)> {
    let mut project = Project::new();
    let report = project.power_analysis(spec)?;
    Ok((project, report))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_are_read_as_from_colon_to() {
        assert_eq!(parse_pair::<usize>("120:480", "row range", "120:480").unwrap(), (120, 480));
        assert_eq!(parse_pair::<f64>("-0.5: 0.1", "X range", "0.2:0.6").unwrap(), (-0.5, 0.1));
        assert!(parse_pair::<usize>("12-48", "row range", "120:480").is_err());
        assert!(parse_pair::<usize>("a:3", "row range", "120:480").is_err());
    }
}
