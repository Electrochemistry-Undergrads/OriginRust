//! Simulate Curve (Analysis > Fitting > Simulate Curve): the values of a built-in fit function for typed
//! parameters at X values from a range or from a column, written as an X and a Y column on a new sheet.
//! The numbers come from `sheaf_fit::simulate`, which uses the evaluator of the fits; the choices that
//! Origin has not been measured on are listed in `docs/spec/fitting.md`, section 13.

use super::dist::{GraphPlot, Output, XyGraph};
use super::*;
use crate::graph::{PlotKind, ScaleKind};
use sheaf_fit::library;
use sheaf_fit::simulate::{self as sim, Spacing};

/// Where the X values of [`Tool::SimulateCurve`] come from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SimulateX {
    /// `points` values from `from` to `to`, both included.
    Range { from: f64, to: f64, points: usize, spacing: Spacing },
    /// Every value of this numeric column, row by row (a missing X gives a missing Y).
    OfColumn(ColumnId),
}

/// What [`Tool::SimulateCurve`] does. The tool reads no data except an X column: the request's `columns` only
/// name the workbook that gets the new sheet (with [`SimulateX::OfColumn`], pass that column, so that a linked
/// run follows it).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulateCurveSpec {
    /// Name of a built-in fit function (case does not matter).
    pub function: String,
    /// Parameter values by name. A parameter that is not listed has the starting value of
    /// `sheaf_fit::simulate::initial_params`.
    pub params: Vec<(String, f64)>,
    pub x: SimulateX,
    /// Also draw the curve in a new graph.
    pub plot: bool,
}

impl Default for SimulateCurveSpec {
    fn default() -> Self {
        SimulateCurveSpec {
            function: "Gauss".to_string(),
            params: Vec::new(),
            x: SimulateX::Range { from: 1.0, to: 10.0, points: 100, spacing: Spacing::Linear },
            plot: false,
        }
    }
}

fn option(e: sheaf_fit::FitError) -> AnalysisError {
    AnalysisError::BadOption(e.to_string())
}

pub(super) fn run(spec: &SimulateCurveSpec, project: &Project) -> Result<Output, AnalysisError> {
    let function = library::get(&spec.function).ok_or_else(|| {
        AnalysisError::BadOption(format!("there is no built-in fit function named '{}'", spec.function))
    })?;
    let params = sim::resolve_params(&function, &spec.params).map_err(option)?;
    let (xs, x_name, x_units, x_comment, log_x) = match &spec.x {
        SimulateX::Range { from, to, points, spacing } => {
            let xs = sim::x_values(*from, *to, *points, *spacing).map_err(option)?;
            let how = if *spacing == Spacing::Log { "logarithmic" } else { "linear" };
            let comment = format!("{points} points from {from} to {to}, {how} spacing");
            (xs, "X".to_string(), String::new(), comment, *spacing == Spacing::Log)
        }
        SimulateX::OfColumn(id) => {
            let (_, _, c) = project.locate(*id).ok_or(AnalysisError::NoSuchColumn(*id))?;
            let v = c.numeric().ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))?;
            if v.is_empty() {
                return Err(AnalysisError::Empty(c.short_name.clone()));
            }
            let comment = format!("X of column {}", c.short_name);
            (v.to_vec(), c.display_name().to_string(), c.units.clone(), comment, false)
        }
    };
    let ys = sim::simulate(&function, &params, &xs).map_err(option)?;
    let notes = vec![
        format!("{}: {}", function.name, function.equation),
        format!("Parameters: {}", sim::describe_params(&function, &params)),
    ];
    let y_comment = format!("Simulated {}: {}", function.name, sim::describe_params(&function, &params));
    let on_sheet = vec![
        SheetColumn {
            designation: Designation::X,
            long_name: x_name.clone(),
            units: x_units,
            comments: x_comment,
            data: ColumnData::numeric(xs),
        },
        SheetColumn {
            designation: Designation::Y,
            long_name: function.name.clone(),
            units: String::new(),
            comments: y_comment,
            data: ColumnData::numeric(ys),
        },
    ];
    let graph = spec.plot.then(|| XyGraph {
        plots: vec![GraphPlot { x: 0, y: 1, kind: PlotKind::Line, label: function.name.clone(), line_of: None }],
        x_scale: if log_x { ScaleKind::Log10 } else { ScaleKind::Linear },
        y_scale: ScaleKind::Linear,
        x_title: x_name,
        y_title: function.name.clone(),
        look: Default::default(),
    });
    Ok(Output { table: None, on_sheet, notes, graph, more_graphs: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Document;

    fn project() -> (Project, WindowId, ColumnId) {
        let mut p = Project::new();
        let book = p.add_workbook("Book1");
        let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, 2.0, 4.0])).unwrap();
        p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0; 4])).unwrap();
        (p, book, x)
    }

    fn request(column: ColumnId, spec: SimulateCurveSpec) -> Request {
        Request { columns: vec![column], rows: RowRange::All, tool: Tool::SimulateCurve(spec) }
    }

    fn gauss() -> SimulateCurveSpec {
        SimulateCurveSpec {
            function: "gauss".into(),
            params: vec![("y0".into(), 1.0), ("xc".into(), 5.0), ("w".into(), 2.0), ("A".into(), 4.0)],
            x: SimulateX::Range { from: 0.0, to: 10.0, points: 5, spacing: Spacing::Linear },
            plot: false,
        }
    }

    #[test]
    fn a_simulated_gauss_goes_to_a_new_sheet_with_x_and_y() {
        let (mut p, book, x) = project();
        let r = p.analyze(&request(x, gauss())).unwrap();
        assert!(r.message.starts_with("Simulate curve") && r.message.contains("Simulate1"), "{}", r.message);
        let sheet = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(sheet.name, "Simulate1");
        assert_eq!(sheet.columns.iter().map(|c| c.designation).collect::<Vec<_>>(), [Designation::X, Designation::Y]);
        assert_eq!(sheet.columns[0].numeric().unwrap(), [0.0, 2.5, 5.0, 7.5, 10.0]);
        assert_eq!(sheet.columns[1].long_name, "Gauss");
        assert_eq!(sheet.columns[1].comments, "Simulated Gauss: y0=1, xc=5, w=2, A=4");
        // At the centre the curve is y0 + A / (w sqrt(pi/2)).
        let y = sheet.columns[1].numeric().unwrap();
        let top = 1.0 + 4.0 / (2.0 * (std::f64::consts::PI / 2.0).sqrt());
        assert!((y[2] - top).abs() < 1e-12, "{} vs {top}", y[2]);
        assert!((y[0] - y[4]).abs() < 1e-15, "symmetric about xc");
        assert!(r.new_graph.is_none());
        assert_eq!(p.workbook(book).unwrap().sheets[0].columns.len(), 2, "the input sheet is untouched");
        assert_eq!(r.notes.len(), 2);
    }

    #[test]
    fn parameters_not_given_start_from_their_initial_values() {
        let (mut p, book, x) = project();
        let spec = SimulateCurveSpec { function: "ExpDec1".into(), params: vec![("t1".into(), 2.0)], ..gauss() };
        let r = p.analyze(&request(x, spec)).unwrap();
        let sheet = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        // y0 = 1, A1 = 1 (initial values), t1 = 2: y(0) = 2.
        assert!((sheet.columns[1].numeric().unwrap()[0] - 2.0).abs() < 1e-12);
        assert_eq!(sheet.columns[1].comments, "Simulated ExpDec1: y0=1, A1=1, t1=2");
    }

    #[test]
    fn the_x_of_an_existing_column_can_be_used() {
        let (mut p, book, x) = project();
        let spec = SimulateCurveSpec { x: SimulateX::OfColumn(x), ..gauss() };
        let r = p.analyze(&request(x, spec)).unwrap();
        let sheet = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(sheet.columns[0].numeric().unwrap(), [0.0, 1.0, 2.0, 4.0]);
        assert_eq!(sheet.columns[0].comments, format!("X of column {}", p.column(x).unwrap().short_name));
        // A missing X gives a missing Y.
        let gap = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, f64::NAN, 3.0])).unwrap();
        let r = p.analyze(&request(gap, SimulateCurveSpec { x: SimulateX::OfColumn(gap), ..gauss() })).unwrap();
        let sheet = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        let y = sheet.columns[1].numeric().unwrap();
        assert!(y[0].is_finite() && y[1].is_nan() && y[2].is_finite());
        assert_eq!(sheet.name, "Simulate2");
    }

    #[test]
    fn log_spacing_makes_a_log_x_axis_and_plot_makes_a_graph() {
        let (mut p, book, x) = project();
        let spec = SimulateCurveSpec {
            function: "Hill".into(),
            x: SimulateX::Range { from: 0.01, to: 100.0, points: 5, spacing: Spacing::Log },
            plot: true,
            ..Default::default()
        };
        let r = p.analyze(&request(x, spec)).unwrap();
        let xs = p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()].columns[0].numeric().unwrap().to_vec();
        for (got, want) in xs.iter().zip([0.01, 0.1, 1.0, 10.0, 100.0]) {
            assert!((got / want - 1.0).abs() < 1e-12);
        }
        let g = p.graph(r.new_graph.expect("Plot makes a graph")).unwrap();
        assert_eq!(g.layers[0].plots.len(), 1);
        assert_eq!(g.layers[0].x_axis.scale, ScaleKind::Log10);
        assert!(r.message.contains("graph"), "{}", r.message);
        // Without Plot there is no graph, and a linear range keeps a linear axis.
        let r = p.analyze(&request(x, SimulateCurveSpec { plot: true, ..gauss() })).unwrap();
        assert_eq!(p.graph(r.new_graph.unwrap()).unwrap().layers[0].x_axis.scale, ScaleKind::Linear);
    }

    #[test]
    fn simulating_is_one_undo_step() {
        let (p, book, x) = project();
        let mut doc = Document::new(p);
        let spec = SimulateCurveSpec { plot: true, ..gauss() };
        let tool = Tool::SimulateCurve(spec.clone());
        let before = doc.project().clone();
        let label = tool.name();
        doc.edit(label, |p| p.analyze(&request(x, spec))).unwrap();
        assert_eq!(doc.project().workbook(book).unwrap().sheets.len(), 2);
        assert_eq!(doc.project().graphs().count(), 1);
        assert_eq!(doc.undo_label(), Some("Simulate curve"));
        doc.undo();
        assert_eq!(*doc.project(), before, "the sheet and the graph go in one step");
        doc.redo();
        assert_eq!(doc.project().workbook(book).unwrap().sheets.len(), 2);
    }

    #[test]
    fn bad_requests_are_refused_and_change_nothing() {
        let (mut p, _, x) = project();
        let before = p.clone();
        let err = |p: &mut Project, spec: SimulateCurveSpec| match p.analyze(&request(x, spec)) {
            Err(AnalysisError::BadOption(m)) => m,
            other => panic!("expected BadOption, got {other:?}"),
        };
        assert!(
            err(&mut p, SimulateCurveSpec { function: "Nope".into(), ..gauss() }).contains("no built-in fit function")
        );
        assert!(
            err(&mut p, SimulateCurveSpec { params: vec![("q".into(), 1.0)], ..gauss() }).contains("no parameter 'q'")
        );
        let range = |from, to, points, spacing| SimulateX::Range { from, to, points, spacing };
        let linear = Spacing::Linear;
        assert!(err(&mut p, SimulateCurveSpec { x: range(0.0, 1.0, 1, linear), ..gauss() }).contains("too few"));
        assert!(err(&mut p, SimulateCurveSpec { x: range(3.0, 3.0, 10, linear), ..gauss() }).contains("different"));
        assert!(
            err(&mut p, SimulateCurveSpec { x: range(0.0, 3.0, 10, Spacing::Log), ..gauss() }).contains("positive")
        );
        assert!(
            err(&mut p, SimulateCurveSpec { params: vec![("w".into(), f64::NAN)], ..gauss() }).contains("not a finite")
        );
        assert_eq!(p, before);
        // An X column that is text, or missing, is refused.
        let (mut p, book, x) = project();
        let t = p.add_column(book, 0, Designation::X, ColumnData::text(vec!["a".into()])).unwrap();
        let spec = SimulateCurveSpec { x: SimulateX::OfColumn(t), ..gauss() };
        assert!(matches!(p.analyze(&request(x, spec)), Err(AnalysisError::NotNumeric(_))));
        assert!(matches!(
            p.analyze(&Request { columns: vec![], rows: RowRange::All, tool: Tool::SimulateCurve(gauss()) }),
            Err(AnalysisError::NoColumns)
        ));
    }

    #[test]
    fn a_function_with_no_value_on_the_range_is_refused() {
        let (mut p, _, x) = project();
        let spec = SimulateCurveSpec {
            function: "Logarithm".into(),
            params: Vec::new(),
            x: SimulateX::Range { from: -5.0, to: -1.0, points: 4, spacing: Spacing::Linear },
            ..gauss()
        };
        assert!(matches!(p.analyze(&request(x, spec)), Err(AnalysisError::BadOption(m)) if m.contains("no value")));
    }
}
