//! Simulate Surface without a window: the XYZ data of a surface fit function on a grid, as a new sheet. The numbers
//! come from the Simulate Surface tool of `sheaf-core` (`analysis::Tool::SimulateSurface`), so the command line, the
//! MCP server and the app agree. What Origin does and what is Sheaf's choice is in `docs/spec/fitting.md`,
//! section 16.

use crate::pick::{pick_book, pick_sheet};
use anyhow::{Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use sheaf_core::analysis::{Report, Request, RowRange, SimulateSurfaceSpec, Tool};
use sheaf_core::column::{ColumnData, Designation};
use sheaf_core::{Project, WindowId};
use sheaf_fit::simsurface as sim;

/// What to simulate. The defaults are those of Origin's Simulate Surface window for the function: its own parameter
/// values, X and Y range, 20 by 20 points and noise level (Gauss2D: -1 to 1, 10 % noise).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SimulateSurfaceOps {
    /// A surface function, Origin's name or the library's (default Gauss2D).
    pub function: String,
    /// Parameter values, each "NAME=VALUE"; the others keep the window's values.
    pub params: Vec<String>,
    pub x_min: Option<f64>,
    pub x_max: Option<f64>,
    pub x_points: Option<usize>,
    pub y_min: Option<f64>,
    pub y_max: Option<f64>,
    pub y_points: Option<usize>,
    /// Noise in percent of the mean of Z (default: the function's, 10 for the four peak functions, else 0).
    pub noise: Option<f64>,
    /// Seed of the noise sequence (default 1).
    pub seed: Option<u64>,
    /// The workbook that gets the new sheet (default: the first one).
    pub book: Option<String>,
}

/// A function that can be simulated, with Origin's window values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SurfaceFunctionInfo {
    /// Origin's name.
    pub name: String,
    /// The parameters in order, with the values of the window.
    pub parameters: Vec<(String, f64)>,
    /// The X and Y range of the window.
    pub range: (f64, f64),
    /// The noise level of the window in percent.
    pub noise: f64,
}

/// The 22 functions of Origin's Simulate Surface window, in its order.
pub fn list_functions() -> Vec<SurfaceFunctionInfo> {
    sim::SAMPLES
        .iter()
        .filter_map(|s| {
            let f = sheaf_fit::surface::get(s.library_name)?;
            let parameters = f.params.iter().map(|p| p.name.clone()).zip(s.params.iter().copied()).collect();
            Some(SurfaceFunctionInfo { name: s.origin_name.to_string(), parameters, range: s.range, noise: s.noise })
        })
        .collect()
}

/// What a run did.
#[derive(Debug)]
pub struct SimulateSurfaceRun {
    pub book: WindowId,
    /// The sheet that got the X, Y and Z columns.
    pub result_sheet: usize,
    pub report: Report,
}

impl SimulateSurfaceRun {
    /// One sentence for a status line.
    pub fn summary(&self, project: &Project) -> String {
        let Some(ws) = project.workbook(self.book).and_then(|b| b.sheets.get(self.result_sheet)) else {
            return self.report.message.clone();
        };
        let function = ws.columns.get(2).map(|c| c.comments.as_str()).filter(|c| !c.is_empty()).unwrap_or("surface");
        format!("Simulated {function}: {} points, results in sheet {}", ws.row_count(), ws.name)
    }
}

fn parse_params(texts: &[String]) -> Result<Vec<(String, f64)>> {
    texts.iter().flat_map(|t| t.split(',')).map(|t| crate::fit::name_value(t, "--param")).collect()
}

/// The tool's spec for this request, with the function name looked up.
pub fn tool_spec(spec: &SimulateSurfaceOps) -> Result<SimulateSurfaceSpec> {
    let name = if spec.function.trim().is_empty() { sim::DEFAULT_FUNCTION } else { spec.function.trim() };
    let mut tool = SimulateSurfaceSpec::for_function(name).ok_or_else(|| {
        anyhow!(
            "there is no surface function named '{name}'; the functions are {} (see --list)",
            sim::names().join(", ")
        )
    })?;
    tool.params = parse_params(&spec.params)?;
    tool.x = (spec.x_min.unwrap_or(tool.x.0), spec.x_max.unwrap_or(tool.x.1), spec.x_points.unwrap_or(tool.x.2));
    tool.y = (spec.y_min.unwrap_or(tool.y.0), spec.y_max.unwrap_or(tool.y.1), spec.y_points.unwrap_or(tool.y.2));
    tool.noise = spec.noise.unwrap_or(tool.noise);
    tool.seed = spec.seed.unwrap_or(tool.seed);
    Ok(tool)
}

/// Runs the simulation in `project`: one new sheet in the workbook, one undo step for a caller that keeps history.
/// A project without a workbook gets one that holds only the new sheet. A failed run leaves the project as it was.
pub fn run(project: &mut Project, spec: &SimulateSurfaceOps) -> Result<SimulateSurfaceRun> {
    let tool = tool_spec(spec)?;
    let fresh = spec.book.is_none() && project.workbooks().next().is_none();
    let (book, anchor) = if fresh {
        let book = project.add_workbook("");
        let id = project.add_column(book, 0, Designation::X, ColumnData::numeric(Vec::new()))?;
        (book, id)
    } else {
        let book = pick_book(project, spec.book.as_deref())?;
        let sheet = pick_sheet(project, book, None)?;
        let first = project.workbook(book).and_then(|b| b.sheets.get(sheet)).and_then(|s| s.columns.first());
        let id = first
            .map(|c| c.id)
            .ok_or_else(|| anyhow!("the active sheet has no column; use a project that has data"))?;
        (book, id)
    };
    let request = Request { columns: vec![anchor], rows: RowRange::All, tool: Tool::SimulateSurface(tool) };
    let mut report = match project.analyze(&request) {
        Ok(r) => r,
        Err(e) => {
            if fresh {
                project.remove_window(book);
            }
            bail!("{e}");
        }
    };
    let mut result_sheet = report.new_sheet.expect("Simulate Surface adds a sheet");
    if fresh && let Some(b) = project.workbook_mut(book) {
        // The workbook was made for this run: it keeps the new sheet only, not the empty one it started with.
        b.sheets.remove(0);
        b.active_sheet = 0;
        result_sheet -= 1;
        report.sheet = 0;
        report.new_sheet = Some(result_sheet);
    }
    Ok(SimulateSurfaceRun { book, result_sheet, report })
}

/// A project that holds only the simulated surface, for a run with no input file.
pub fn run_standalone(spec: &SimulateSurfaceOps) -> Result<(Project, SimulateSurfaceRun)> {
    let mut project = Project::new();
    let run = run(&mut project, spec)?;
    Ok((project, run))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(project: &Project, run: &SimulateSurfaceRun, i: usize) -> Vec<f64> {
        project.workbook(run.book).unwrap().sheets[run.result_sheet].columns[i].numeric().unwrap().to_vec()
    }

    #[test]
    fn a_standalone_run_holds_only_the_new_sheet_with_origins_defaults() {
        let (project, run) = run_standalone(&SimulateSurfaceOps::default()).unwrap();
        let book = project.workbook(run.book).unwrap();
        assert_eq!(book.sheets.len(), 1);
        assert_eq!(book.sheets[0].name, "SimulateSurface1");
        assert_eq!(column(&project, &run, 0).len(), 400, "20 by 20 points");
        assert_eq!(column(&project, &run, 0)[0], -1.0);
        assert_eq!(column(&project, &run, 1)[399], 1.0);
        assert!(run.summary(&project).contains("Gauss2D") && run.summary(&project).contains("400 points"));
    }

    #[test]
    fn options_override_the_window_values_and_noise_zero_is_exact() {
        let spec = SimulateSurfaceOps {
            function: "plane".into(),
            params: vec!["a=3,b=4".into()],
            x_min: Some(0.0),
            x_max: Some(2.0),
            x_points: Some(3),
            y_min: Some(0.0),
            y_max: Some(1.0),
            y_points: Some(2),
            ..Default::default()
        };
        let (project, run) = run_standalone(&spec).unwrap();
        // z0 0 (window value), a 3, b 4: z = 3x + 4y.
        assert_eq!(column(&project, &run, 2), [0.0, 3.0, 6.0, 4.0, 7.0, 10.0]);
        let noisy = SimulateSurfaceOps { noise: Some(20.0), seed: Some(5), ..spec.clone() };
        let (p2, r2) = run_standalone(&noisy).unwrap();
        assert_ne!(column(&p2, &r2, 2), column(&project, &run, 2));
        assert_eq!(column(&p2, &r2, 0), column(&project, &run, 0), "X is not noisy");
    }

    #[test]
    fn a_failed_run_leaves_no_workbook_and_names_the_problem() {
        let mut project = Project::new();
        let e = run(&mut project, &SimulateSurfaceOps { function: "Nope".into(), ..Default::default() }).unwrap_err();
        assert!(e.to_string().contains("no surface function"), "{e}");
        let e = run(&mut project, &SimulateSurfaceOps { x_points: Some(1), ..Default::default() }).unwrap_err();
        assert!(e.to_string().contains("too few"), "{e}");
        assert!(project.workbooks().next().is_none());
    }

    #[test]
    fn the_list_has_the_22_functions_in_origins_order_with_their_window_values() {
        let list = list_functions();
        assert_eq!(list.len(), 22);
        assert_eq!(list[0].name, "Chebyshev2D");
        let gauss = list.iter().find(|f| f.name == "Gauss2D").unwrap();
        assert_eq!(gauss.parameters[3], ("w1".to_string(), 0.3));
        assert_eq!((gauss.range, gauss.noise), ((-1.0, 1.0), 10.0));
    }
}
