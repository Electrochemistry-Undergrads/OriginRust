//! Simulate Surface (Analysis > Fitting > Simulate Surface): the XYZ data of one of the 22 surface fit functions
//! on a grid, with optional noise, as X, Y and Z columns on a new sheet. The numbers come from
//! `sheaf_fit::simsurface`, which uses the evaluator of the surface fits; what Origin does and what is Sheaf's own
//! choice (the random sequence, the sheet instead of a new workbook, no Recalculate) is in `docs/spec/fitting.md`,
//! section 16.

use super::dist::Output;
use super::*;
use sheaf_fit::simsurface as sim;

/// What [`Tool::SimulateSurface`] does. The tool reads no data: the request's `columns` only name the workbook
/// that gets the new sheet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulateSurfaceSpec {
    /// Origin's name of a surface function, or the library's (case does not matter).
    pub function: String,
    /// Parameter values by name; a parameter that is not listed has the value of Origin's window for the function.
    pub params: Vec<(String, f64)>,
    /// X minimum, maximum and number of points.
    pub x: (f64, f64, usize),
    /// Y minimum, maximum and number of points.
    pub y: (f64, f64, usize),
    /// Noise level in percent of the mean of Z (0 for none).
    pub noise: f64,
    /// Seed of the noise sequence; the same seed gives the same noise.
    pub seed: u64,
}

impl SimulateSurfaceSpec {
    /// Origin's window values for `function` (parameters, X and Y range, 20 points each, noise level), if it is one
    /// of the 22 functions.
    pub fn for_function(function: &str) -> Option<SimulateSurfaceSpec> {
        let s = sim::sample(function)?;
        Some(SimulateSurfaceSpec {
            function: s.origin_name.to_string(),
            params: Vec::new(),
            x: (s.range.0, s.range.1, sim::ORIGIN_POINTS),
            y: (s.range.0, s.range.1, sim::ORIGIN_POINTS),
            noise: s.noise,
            seed: 1,
        })
    }
}

impl Default for SimulateSurfaceSpec {
    /// Origin's window as it opens: Gauss2D, -1 to 1 in X and Y, 20 points each, noise 10 %.
    fn default() -> Self {
        SimulateSurfaceSpec::for_function(sim::DEFAULT_FUNCTION).expect("the default function is in the table")
    }
}

fn option(e: sheaf_fit::FitError) -> AnalysisError {
    AnalysisError::BadOption(e.to_string())
}

pub(super) fn run(spec: &SimulateSurfaceSpec) -> Result<Output, AnalysisError> {
    let (sample, function) = sim::function(&spec.function).map_err(option)?;
    let params = sim::resolve_params(sample, &function, &spec.params).map_err(option)?;
    let xs = sim::axis(spec.x.0, spec.x.1, spec.x.2, "X").map_err(option)?;
    let ys = sim::axis(spec.y.0, spec.y.1, spec.y.2, "Y").map_err(option)?;
    let (gx, gy, mut gz) = sim::simulate_surface(&function, &params, &xs, &ys).map_err(option)?;
    sim::add_noise(&mut gz, spec.noise, spec.seed).map_err(option)?;
    let mut notes = vec![
        format!("{}: z = {}", sample.origin_name, sheaf_fit::surface::formula(sample.library_name).unwrap_or("")),
        format!("Parameters: {}", sheaf_fit::simulate::describe_params(&function, &params)),
        format!("{} by {} points", xs.len(), ys.len()),
    ];
    if spec.noise > 0.0 {
        notes.push(format!("Noise {}% of the mean of Z (Sheaf's seeded generator, seed {})", spec.noise, spec.seed));
    }
    let column = |designation, long_name: &str, data: Vec<f64>| SheetColumn {
        designation,
        long_name: long_name.to_string(),
        units: String::new(),
        comments: sample.origin_name.to_string(),
        data: ColumnData::numeric(data),
    };
    // Origin's names (fitting.md 16.2): Simulated X1, Y1 and Z1, the function as the comment.
    let on_sheet = vec![
        column(Designation::X, "Simulated X1", gx),
        column(Designation::Y, "Simulated Y1", gy),
        column(Designation::Z, "Simulated Z1", gz),
    ];
    Ok(Output { table: None, on_sheet, notes, graph: None, more_graphs: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Document;

    fn project() -> (Project, WindowId, ColumnId) {
        let mut p = Project::new();
        let book = p.add_workbook("Book1");
        let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0])).unwrap();
        (p, book, x)
    }

    fn request(column: ColumnId, spec: SimulateSurfaceSpec) -> Request {
        Request { columns: vec![column], rows: RowRange::All, tool: Tool::SimulateSurface(spec) }
    }

    fn plane() -> SimulateSurfaceSpec {
        SimulateSurfaceSpec {
            x: (0.0, 2.0, 3),
            y: (10.0, 20.0, 2),
            ..SimulateSurfaceSpec::for_function("Plane").unwrap()
        }
    }

    #[test]
    fn the_default_is_origins_window() {
        let d = SimulateSurfaceSpec::default();
        assert_eq!((d.function.as_str(), d.x, d.y, d.noise), ("Gauss2D", (-1.0, 1.0, 20), (-1.0, 1.0, 20), 10.0));
        assert_eq!(SimulateSurfaceSpec::for_function("Cosine2D").unwrap().function, "Cosine");
        assert_eq!(SimulateSurfaceSpec::for_function("Plane").unwrap().noise, 0.0);
        assert!(SimulateSurfaceSpec::for_function("nope").is_none());
    }

    #[test]
    fn a_simulated_plane_goes_to_a_new_sheet_with_x_y_and_z() {
        let (mut p, book, x) = project();
        let r = p.analyze(&request(x, plane())).unwrap();
        assert!(r.message.starts_with("Simulate surface"), "{}", r.message);
        let sheet = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert!(sheet.name.starts_with("SimulateSurface"), "{}", sheet.name);
        assert_eq!(
            sheet.columns.iter().map(|c| c.designation).collect::<Vec<_>>(),
            [Designation::X, Designation::Y, Designation::Z]
        );
        let long: Vec<&str> = sheet.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert_eq!(long, ["Simulated X1", "Simulated Y1", "Simulated Z1"]);
        assert!(sheet.columns.iter().all(|c| c.comments == "Plane"));
        let col = |i: usize| sheet.columns[i].numeric().unwrap().to_vec();
        assert_eq!(col(0), [0.0, 1.0, 2.0, 0.0, 1.0, 2.0]);
        assert_eq!(col(1), [10.0, 10.0, 10.0, 20.0, 20.0, 20.0]);
        // z0 0, a 1, b 2: z = x + 2y.
        assert_eq!(col(2), [20.0, 21.0, 22.0, 40.0, 41.0, 42.0]);
        assert_eq!(p.workbook(book).unwrap().sheets[0].columns.len(), 1, "the input sheet is untouched");
    }

    #[test]
    fn noise_changes_z_only_and_the_seed_repeats_it() {
        let (mut p, book, x) = project();
        let noisy = SimulateSurfaceSpec { noise: 10.0, seed: 3, ..plane() };
        let a = p.analyze(&request(x, noisy.clone())).unwrap();
        let b = p.analyze(&request(x, noisy)).unwrap();
        let cols = |p: &Project, r: &Report| -> Vec<Vec<f64>> {
            let s = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
            s.columns.iter().map(|c| c.numeric().unwrap().to_vec()).collect()
        };
        let (ca, cb) = (cols(&p, &a), cols(&p, &b));
        assert_eq!(ca, cb);
        assert_eq!(ca[0], [0.0, 1.0, 2.0, 0.0, 1.0, 2.0], "X stays exact");
        assert_ne!(ca[2], [20.0, 21.0, 22.0, 40.0, 41.0, 42.0]);
        assert!(a.notes.iter().any(|n| n.contains("Noise 10%")));
    }

    #[test]
    fn simulating_is_one_undo_step() {
        let (p, book, x) = project();
        let mut doc = Document::new(p);
        let before = doc.project().clone();
        doc.edit(Tool::SimulateSurface(plane()).name(), |p| p.analyze(&request(x, plane()))).unwrap();
        assert_eq!(doc.project().workbook(book).unwrap().sheets.len(), 2);
        assert_eq!(doc.undo_label(), Some("Simulate surface"));
        doc.undo();
        assert_eq!(*doc.project(), before);
    }

    #[test]
    fn bad_requests_are_refused_and_change_nothing() {
        let (mut p, _, x) = project();
        let before = p.clone();
        let err = |p: &mut Project, spec: SimulateSurfaceSpec| match p.analyze(&request(x, spec)) {
            Err(AnalysisError::BadOption(m)) => m,
            other => panic!("expected BadOption, got {other:?}"),
        };
        assert!(
            err(&mut p, SimulateSurfaceSpec { function: "Nope".into(), ..plane() }).contains("no surface function")
        );
        assert!(
            err(&mut p, SimulateSurfaceSpec { params: vec![("q".into(), 1.0)], ..plane() }).contains("no parameter")
        );
        assert!(err(&mut p, SimulateSurfaceSpec { x: (0.0, 1.0, 1), ..plane() }).contains("too few"));
        assert!(err(&mut p, SimulateSurfaceSpec { y: (2.0, 2.0, 5), ..plane() }).contains("different"));
        assert!(err(&mut p, SimulateSurfaceSpec { noise: -5.0, ..plane() }).contains("noise"));
        assert_eq!(p, before);
        assert!(matches!(
            p.analyze(&Request { columns: vec![], rows: RowRange::All, tool: Tool::SimulateSurface(plane()) }),
            Err(AnalysisError::NoColumns)
        ));
    }
}
