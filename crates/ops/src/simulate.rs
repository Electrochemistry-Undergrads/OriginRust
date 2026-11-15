//! Simulate Curve without a window: the values of a built-in fit function at X values from a range or from a
//! column, as a new sheet. The numbers come from the Simulate Curve tool of `sheaf-core`
//! (`analysis::Tool::SimulateCurve`), so the command line, the MCP server and the app agree. The choices that
//! Origin has not been measured on are listed in `docs/spec/fitting.md`, section 13.

use crate::pick::{named_columns, pick_book, pick_sheet};
use anyhow::{Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use sheaf_core::analysis::{Report, Request, RowRange, SimulateCurveSpec, SimulateSpacing, SimulateX, Tool};
use sheaf_core::column::{ColumnData, Designation};
use sheaf_core::{ColumnId, Project, WindowId};
use sheaf_fit::{library, simulate as sim};

/// What to simulate. The defaults are those of the Simulate Curve tool: Gauss at its starting values, 100
/// points from 1 to 10, linear spacing.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SimulateSpec {
    /// A built-in fit function (case does not matter).
    pub function: String,
    /// Parameter values, each "NAME=VALUE"; the others keep their starting values.
    pub params: Vec<String>,
    /// First X (default 1).
    pub from: Option<f64>,
    /// Last X (default 10).
    pub to: Option<f64>,
    /// Number of X values (default 100).
    pub points: Option<usize>,
    /// Spread the X values evenly on a logarithmic scale.
    pub log: bool,
    /// Take the X values from this column (short or long name) instead of a range.
    pub x_of: Option<String>,
    /// The workbook that gets the new sheet (default: the first one).
    pub book: Option<String>,
    /// The sheet `x_of` is looked up in (default: the workbook's active sheet).
    pub sheet: Option<String>,
    /// Also draw the curve in a new graph.
    pub plot: bool,
}

impl SimulateSpec {
    /// A spec for the named function with the defaults.
    pub fn new(function: &str) -> SimulateSpec {
        SimulateSpec { function: function.to_string(), ..SimulateSpec::default() }
    }
}

/// A function that can be simulated, with the value each parameter starts from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub category: String,
    /// The parameters in order, with their starting values.
    pub parameters: Vec<(String, f64)>,
}

/// The functions Simulate Curve offers (the library of the fits), in the order of the library.
pub fn list_functions() -> Vec<FunctionInfo> {
    library::list()
        .into_iter()
        .filter_map(|f| {
            let function = library::get(&f.name)?;
            let start = sim::initial_params(&function);
            let parameters = function.params.iter().map(|p| p.name.clone()).zip(start).collect();
            Some(FunctionInfo { name: f.name, category: f.category, parameters })
        })
        .collect()
}

/// What a run did.
#[derive(Debug)]
pub struct SimulateRun {
    pub book: WindowId,
    /// The sheet that got the X and Y columns.
    pub result_sheet: usize,
    pub report: Report,
}

impl SimulateRun {
    /// One sentence for a status line: the points, the sheet and the graph.
    pub fn summary(&self, project: &Project) -> String {
        let Some(ws) = project.workbook(self.book).and_then(|b| b.sheets.get(self.result_sheet)) else {
            return self.report.message.clone();
        };
        let function = ws.columns.get(1).map_or("curve", |c| c.long_name.as_str());
        let mut text = format!("Simulated {function}: {} points, results in sheet {}", ws.row_count(), ws.name);
        if let Some(g) = self.report.new_graph.and_then(|g| project.graph(g)) {
            text.push_str(&format!("; graph {}", g.name));
        }
        text
    }
}

fn parse_params(texts: &[String]) -> Result<Vec<(String, f64)>> {
    texts.iter().flat_map(|t| t.split(',')).map(|t| crate::fit::name_value(t, "--param")).collect()
}

/// The tool's spec for this request, with every number checked and the function name looked up. `x_of` is the
/// X column when the X values come from one.
pub fn tool_spec(spec: &SimulateSpec, x_of: Option<ColumnId>) -> Result<SimulateCurveSpec> {
    if spec.function.trim().is_empty() {
        bail!("give the function to simulate: --function NAME (see --list)");
    }
    let function = library::get(spec.function.trim())
        .ok_or_else(|| anyhow!("there is no built-in fit function named '{}' (see --list)", spec.function.trim()))?;
    let x = match x_of {
        Some(id) => {
            if spec.from.is_some() || spec.to.is_some() || spec.points.is_some() || spec.log {
                bail!(
                    "--x-of takes the X values of a column; it cannot be combined with --from, --to, --points or --log"
                );
            }
            SimulateX::OfColumn(id)
        }
        None => SimulateX::Range {
            from: spec.from.unwrap_or(1.0),
            to: spec.to.unwrap_or(10.0),
            points: spec.points.unwrap_or(100),
            spacing: if spec.log { SimulateSpacing::Log } else { SimulateSpacing::Linear },
        },
    };
    Ok(SimulateCurveSpec { function: function.name, params: parse_params(&spec.params)?, x, plot: spec.plot })
}

/// Runs the simulation in `project`: one new sheet (and with `plot` one graph) in the workbook, one undo step
/// for a caller that keeps history. A project without a workbook gets one that holds only the new sheet. A
/// failed run leaves the project as it was.
pub fn run(project: &mut Project, spec: &SimulateSpec) -> Result<SimulateRun> {
    if spec.x_of.is_none() && spec.sheet.is_some() {
        bail!("--sheet names the sheet of the --x-of column; a simulated range needs no sheet");
    }
    let fresh = spec.book.is_none() && project.workbooks().next().is_none();
    if fresh && spec.x_of.is_some() {
        bail!("--x-of needs a project or data file with the column to take the X values from");
    }
    let (book, anchor, x_of) = if fresh {
        let book = project.add_workbook("");
        let id = project.add_column(book, 0, Designation::X, ColumnData::numeric(Vec::new()))?;
        (book, id, None)
    } else {
        let book = pick_book(project, spec.book.as_deref())?;
        let sheet = pick_sheet(project, book, spec.sheet.as_deref())?;
        match &spec.x_of {
            Some(name) => {
                let id = named_columns(project, book, sheet, std::slice::from_ref(name))?[0];
                (book, id, Some(id))
            }
            None => {
                let first = project.workbook(book).and_then(|b| b.sheets.get(sheet)).and_then(|s| s.columns.first());
                let id = first.map(|c| c.id).ok_or_else(|| {
                    anyhow!("the sheet has no column; choose another sheet with --sheet, or a project that has data")
                })?;
                (book, id, None)
            }
        }
    };
    let result = tool_spec(spec, x_of).and_then(|tool| {
        let request = Request { columns: vec![anchor], rows: RowRange::All, tool: Tool::SimulateCurve(tool) };
        Ok(project.analyze(&request)?)
    });
    let mut report = match result {
        Ok(r) => r,
        Err(e) => {
            if fresh {
                project.remove_window(book);
            }
            return Err(e);
        }
    };
    let mut result_sheet = report.new_sheet.expect("Simulate Curve adds a sheet");
    if fresh && let Some(b) = project.workbook_mut(book) {
        // The workbook was made for this run: it keeps the new sheet only, not the empty one it started with.
        b.sheets.remove(0);
        b.active_sheet = 0;
        result_sheet -= 1;
        report.sheet = 0;
        report.new_sheet = Some(result_sheet);
    }
    Ok(SimulateRun { book, result_sheet, report })
}

/// A project that holds only the simulated curve, for a run with no input file.
pub fn run_standalone(spec: &SimulateSpec) -> Result<(Project, SimulateRun)> {
    let mut project = Project::new();
    let run = run(&mut project, spec)?;
    Ok((project, run))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numbers(project: &Project, run: &SimulateRun, column: usize) -> Vec<f64> {
        project.workbook(run.book).unwrap().sheets[run.result_sheet].columns[column].numeric().unwrap().to_vec()
    }

    #[test]
    fn a_standalone_run_holds_only_the_new_sheet() {
        let mut spec = SimulateSpec::new("gauss");
        spec.params = vec!["y0=1".into(), "xc=5,w=2".into(), "A=4".into()];
        spec.from = Some(0.0);
        spec.to = Some(10.0);
        spec.points = Some(5);
        let (project, run) = run_standalone(&spec).unwrap();
        let book = project.workbook(run.book).unwrap();
        assert_eq!(book.sheets.len(), 1);
        assert_eq!(book.sheets[0].name, "Simulate1");
        let x = numbers(&project, &run, 0);
        assert_eq!(x, [0.0, 2.5, 5.0, 7.5, 10.0]);
        let gauss = library::get("Gauss").unwrap();
        let params = sim::resolve_params(&gauss, &parse_params(&spec.params).unwrap()).unwrap();
        assert_eq!(numbers(&project, &run, 1), sim::simulate(&gauss, &params, &x).unwrap());
    }

    #[test]
    fn a_failed_standalone_run_leaves_no_workbook() {
        let mut project = Project::new();
        let mut spec = SimulateSpec::new("Gauss");
        spec.points = Some(1);
        assert!(run(&mut project, &spec).is_err());
        assert!(project.workbooks().next().is_none());
        let e = run(&mut project, &SimulateSpec::new("NoSuchFunction")).unwrap_err();
        assert!(e.to_string().contains("no built-in"), "{e}");
        assert!(project.workbooks().next().is_none());
    }

    #[test]
    fn the_x_of_a_column_and_a_range_cannot_be_mixed() {
        let mut project = Project::new();
        let book = project.add_workbook("");
        project.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, 2.0])).unwrap();
        let mut spec = SimulateSpec::new("Gauss");
        spec.x_of = Some("A".into());
        let first = run(&mut project, &spec).unwrap();
        assert_eq!(numbers(&project, &first, 0), [0.0, 1.0, 2.0]);
        assert_eq!(project.workbook(book).unwrap().sheets.len(), 2);
        spec.points = Some(5);
        let e = run(&mut project, &spec).unwrap_err();
        assert!(e.to_string().contains("cannot be combined"), "{e}");
        assert_eq!(project.workbook(book).unwrap().sheets.len(), 2);
    }

    #[test]
    fn the_list_has_every_function_with_its_starting_values() {
        let list = list_functions();
        let gauss = list.iter().find(|f| f.name == "Gauss").unwrap();
        assert_eq!(gauss.parameters.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), ["y0", "xc", "w", "A"]);
        assert!(list.len() > 100);
        assert!(list.iter().flat_map(|f| &f.parameters).all(|(_, v)| v.is_finite()));
    }
}
