//! The curve mathematics tools of `Project::analyze`: Average Multiple Curves, Intersect and Polygon
//! Area. The numerics are in `sheaf_calc::curves`; this module reads the chosen curves and writes the
//! result sheets. The rules are in `docs/spec/analysis-curve-math.md`; none was measured in Origin.

use super::*;
use sheaf_calc::{AverageOptions, MeetKind};

/// What [`Tool::AverageCurves`] does: the mean of all chosen curves on a common X.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AverageSpec {
    pub options: AverageOptions,
    /// Add the sample standard deviation as an error column of the mean.
    pub sd: bool,
    /// Add the number of curves that contributed at each X.
    pub count: bool,
}

impl Default for AverageSpec {
    fn default() -> Self {
        AverageSpec { options: AverageOptions::default(), sd: true, count: true }
    }
}

/// What [`Tool::Intersect`] does: the points where the two chosen curves meet.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct IntersectSpec {
    /// Also list the points where the curves touch without crossing.
    pub include_touching: bool,
}

/// Output of one of these tools; `analyze` places it.
pub(super) struct Output {
    pub table: Option<ResultTable>,
    pub on_sheet: Vec<SheetColumn>,
    pub notes: Vec<String>,
}

fn numeric(designation: Designation, name: &str, units: &str, comments: String, data: Vec<f64>) -> SheetColumn {
    SheetColumn {
        designation,
        long_name: name.to_string(),
        units: units.to_string(),
        comments,
        data: ColumnData::numeric(data),
    }
}

pub(super) fn run(tool: &Tool, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    match tool {
        Tool::AverageCurves(spec) => average(spec, inputs),
        Tool::Intersect(spec) => intersect(spec, inputs),
        Tool::PolygonArea => polygon_area(inputs),
        _ => unreachable!("only the curve mathematics tools come here"),
    }
}

fn names(inputs: &[Input<'_>]) -> String {
    inputs.iter().map(|i| i.short.as_str()).collect::<Vec<_>>().join(", ")
}

fn average(spec: &AverageSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    if inputs.len() < 2 {
        return Err(AnalysisError::BadOption(
            "Average Multiple Curves needs at least two curves; choose two or more columns".into(),
        ));
    }
    let curves: Vec<(&[f64], &[f64])> = inputs.iter().map(|i| (i.xs(), i.ys())).collect();
    let r = calc::average_curves(&curves, &spec.options).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let first = &inputs[0];
    let comment = format!("mean of {}", names(inputs));
    let points = r.x.len();
    let mut on_sheet = vec![
        numeric(Designation::X, &first.x_name, &first.x_units, String::new(), r.x),
        numeric(Designation::Y, "Mean", &first.units, comment.clone(), r.mean),
    ];
    if spec.sd {
        on_sheet.push(numeric(Designation::YError, "SD", &first.units, comment, r.sd));
    }
    if spec.count {
        on_sheet.push(numeric(
            Designation::Disregard,
            "N",
            "",
            "number of curves at this X".to_string(),
            r.n.iter().map(|&n| n as f64).collect(),
        ));
    }
    let notes = vec![format!("{} curves averaged at {points} X values", inputs.len())];
    Ok(Output { table: None, on_sheet, notes })
}

fn intersect(spec: &IntersectSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    let [a, b] = inputs else {
        return Err(AnalysisError::BadOption(format!(
            "Intersect needs exactly two curves, not {}; choose two columns",
            inputs.len()
        )));
    };
    let found = calc::intersections(a.xs(), a.ys(), b.xs(), b.ys(), spec.include_touching);
    let comment = format!("{} and {}", a.short, b.short);
    let mut on_sheet = vec![
        numeric(
            Designation::X,
            &format!("Intersection X ({})", a.x_name),
            &a.x_units,
            comment.clone(),
            found.iter().map(|h| h.x).collect(),
        ),
        numeric(
            Designation::Y,
            &format!("Intersection Y ({})", a.name),
            &a.units,
            comment.clone(),
            found.iter().map(|h| h.y).collect(),
        ),
    ];
    if spec.include_touching {
        let kinds = found
            .iter()
            .map(|h| match h.kind {
                MeetKind::Crossing => "Crossing".to_string(),
                MeetKind::Touching => "Touching".to_string(),
            })
            .collect();
        on_sheet.push(SheetColumn {
            designation: Designation::Label,
            long_name: "Type".to_string(),
            units: String::new(),
            comments: comment,
            data: ColumnData::text(kinds),
        });
    }
    let notes = vec![match found.len() {
        0 => format!("{} and {} do not meet", a.short, b.short),
        1 => format!("{} and {} meet at 1 point", a.short, b.short),
        n => format!("{} and {} meet at {n} points", a.short, b.short),
    }];
    Ok(Output { table: None, on_sheet, notes })
}

fn polygon_area(inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    let mut t = ResultTable::new(&[
        "Area",
        "Signed area",
        "Centroid X",
        "Centroid Y",
        "Perimeter",
        "Self-intersecting",
        "Points",
    ]);
    let mut notes = Vec::new();
    for inp in inputs {
        let p = calc::polygon_area(inp.xs(), inp.ys()).ok_or_else(|| {
            AnalysisError::BadOption(format!("column {} has fewer than three different points with numbers", inp.short))
        })?;
        let area_unit = unit_product(&inp.units, &inp.x_units);
        let (cx, cy) = p.centroid.unwrap_or((f64::NAN, f64::NAN));
        t.push_row(
            inp.label.clone(),
            vec![p.area, p.signed_area, cx, cy, p.perimeter, f64::from(u8::from(p.self_intersecting)), p.points as f64],
            vec![
                area_unit.clone(),
                area_unit,
                inp.x_units.clone(),
                inp.units.clone(),
                inp.x_units.clone(),
                String::new(),
                String::new(),
            ],
        );
        if p.self_intersecting {
            notes.push(format!(
                "{} crosses itself: its signed area counts the parts that run the other way as negative, so \
                 the area is not the total enclosed",
                inp.short
            ));
        }
    }
    Ok(Output { table: Some(t), on_sheet: Vec::new(), notes })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{FormulaState, RecalcMode};

    fn req(columns: &[ColumnId], tool: Tool) -> Request {
        Request { columns: columns.to_vec(), rows: RowRange::All, tool }
    }

    /// One shared X column and two Y columns (a ramp and a shifted ramp).
    fn scans() -> (Project, WindowId, Vec<ColumnId>) {
        let mut p = Project::new();
        let book = p.add_workbook("");
        p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, 2.0, 3.0])).unwrap();
        let a = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.0, 1.0, 2.0, 3.0])).unwrap();
        let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![2.0, 3.0, 4.0, 5.0])).unwrap();
        (p, book, vec![a, b])
    }

    #[test]
    fn average_writes_mean_sd_and_count_to_a_new_sheet() {
        let (mut p, book, ys) = scans();
        let r = p.analyze(&req(&ys, Tool::AverageCurves(AverageSpec::default()))).unwrap();
        let ws = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(ws.name, "Average1");
        let cols: Vec<_> = ws.columns.iter().map(|c| (c.designation, c.numeric().unwrap().to_vec())).collect();
        assert_eq!(cols[0], (Designation::X, vec![0.0, 1.0, 2.0, 3.0]));
        assert_eq!(cols[1], (Designation::Y, vec![1.0, 2.0, 3.0, 4.0]));
        assert_eq!(cols[2].0, Designation::YError);
        assert!(cols[2].1.iter().all(|s| (s - 2.0_f64.sqrt()).abs() < 1e-12));
        assert_eq!(cols[3].1, vec![2.0; 4]);
        // Without the optional columns only X and the mean are written.
        let spec = AverageSpec { sd: false, count: false, ..AverageSpec::default() };
        let r = p.analyze(&req(&ys, Tool::AverageCurves(spec))).unwrap();
        assert_eq!(r.new_columns.len(), 2);
    }

    #[test]
    fn average_needs_two_curves_and_refuses_a_turning_x() {
        let (mut p, _, ys) = scans();
        assert!(p.analyze(&req(&ys[..1], Tool::AverageCurves(AverageSpec::default()))).is_err());
        // Two curves, each with its own X, one of them a full cycle.
        let book = p.add_workbook("Cycle");
        p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, 0.0])).unwrap();
        let a = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.0, 1.0, 2.0])).unwrap();
        p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 0.5, 1.0])).unwrap();
        let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.0, 1.0, 2.0])).unwrap();
        let e = p.analyze(&req(&[a, b], Tool::AverageCurves(AverageSpec::default()))).unwrap_err();
        assert!(e.to_string().contains("turn back"), "{e}");
    }

    #[test]
    fn intersect_lists_the_crossings() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 4.0])).unwrap();
        let a = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.0, 4.0])).unwrap();
        p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 4.0])).unwrap();
        let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![4.0, 0.0])).unwrap();
        let r = p.analyze(&req(&[a, b], Tool::Intersect(IntersectSpec::default()))).unwrap();
        let ws = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(ws.columns[0].numeric().unwrap(), [2.0]);
        assert_eq!(ws.columns[1].numeric().unwrap(), [2.0]);
        assert!(r.notes[0].contains("1 point"));
        // Three curves are refused.
        assert!(p.analyze(&req(&[a, b, a], Tool::Intersect(IntersectSpec::default()))).is_err());
    }

    #[test]
    fn polygon_area_of_a_loop_in_a_table() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 2.0, 2.0, 0.0])).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.0, 0.0, 3.0, 3.0])).unwrap();
        p.column_mut(x).unwrap().units = "V".into();
        p.column_mut(y).unwrap().units = "A".into();
        let r = p.analyze(&req(&[y], Tool::PolygonArea)).unwrap();
        let t = r.table.unwrap();
        assert_eq!(t.headers[0], "Area");
        assert_eq!(t.rows[0].1[..2], [6.0, 6.0]);
        assert_eq!(t.rows[0].1[2..4], [1.0, 1.5]);
        assert!(t.unit(0, 0).contains('A') && t.unit(0, 0).contains('V'));
        assert!(r.notes.is_empty());
    }

    #[test]
    fn linked_average_follows_its_input() {
        let (mut p, _, ys) = scans();
        let request = req(&ys, Tool::AverageCurves(AverageSpec::default()));
        assert!(Project::can_link(&request.tool));
        let r = p.analyze_linked(&request, &OutputTarget::NextToInput, RecalcMode::Auto).unwrap();
        let mean = r.new_columns[1];
        assert_eq!(p.numeric(mean).unwrap(), [1.0, 2.0, 3.0, 4.0]);
        let before = p.clone();
        p.column_mut(ys[1]).unwrap().data = ColumnData::numeric(vec![4.0, 3.0, 4.0, 5.0]);
        let problems = p.update_analysis_links(&before);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(p.numeric(mean).unwrap()[0], 2.0);
    }
    /// Two curves with their own X, as the Intersect and Polygon Area tests need.
    fn pair() -> (Project, WindowId, ColumnId, ColumnId) {
        let mut p = Project::new();
        let book = p.add_workbook("");
        p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 4.0])).unwrap();
        let a = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.0, 4.0])).unwrap();
        p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 4.0])).unwrap();
        let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![4.0, 0.0])).unwrap();
        (p, book, a, b)
    }

    #[test]
    fn a_manual_intersect_link_waits_then_follows_its_input() {
        let (mut p, _, a, b) = pair();
        let request = req(&[a, b], Tool::Intersect(IntersectSpec::default()));
        assert!(Project::can_link(&request.tool));
        let r = p.analyze_linked(&request, &OutputTarget::NewSheet, RecalcMode::Manual).unwrap();
        let (x, y) = (r.new_columns[0], r.new_columns[1]);
        assert_eq!((p.numeric(x).unwrap(), p.numeric(y).unwrap()), ([2.0].as_slice(), [2.0].as_slice()));
        let before = p.clone();
        // The second curve now runs from 8 down to 0: the crossing moves to (8/3, 8/3).
        p.column_mut(b).unwrap().data = ColumnData::numeric(vec![8.0, 0.0]);
        p.update_analysis_links(&before);
        assert_eq!(p.numeric(x).unwrap(), [2.0].as_slice(), "not recalculated yet");
        assert_eq!(p.column(x).unwrap().analysis.unwrap().state, FormulaState::Pending);
        let link = p.analysis_link_of(x).unwrap();
        p.recalculate_analysis(link).unwrap();
        assert!((p.numeric(x).unwrap()[0] - 8.0 / 3.0).abs() < 1e-12);
        assert_eq!(p.column(x).unwrap().analysis.unwrap().state, FormulaState::Current);
    }

    #[test]
    fn an_auto_polygon_area_link_replaces_its_table() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 2.0, 2.0, 0.0])).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.0, 0.0, 3.0, 3.0])).unwrap();
        let request = req(&[y], Tool::PolygonArea);
        assert!(Project::can_link(&request.tool));
        let r = p.analyze_linked(&request, &OutputTarget::NextToInput, RecalcMode::Auto).unwrap();
        let area = r.new_columns[1];
        assert_eq!(p.numeric(area).unwrap(), [6.0].as_slice());
        let sheets = p.workbook(book).unwrap().sheets.len();
        let before = p.clone();
        p.column_mut(y).unwrap().data = ColumnData::numeric(vec![0.0, 0.0, 1.0, 1.0]);
        let problems = p.update_analysis_links(&before);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(p.numeric(area).unwrap(), [2.0].as_slice());
        assert_eq!(p.workbook(book).unwrap().sheets.len(), sheets, "the result was replaced, not added");
    }

    #[test]
    fn an_intersect_link_whose_curve_loses_its_numbers_gives_an_empty_result() {
        let (mut p, _, a, b) = pair();
        let request = req(&[a, b], Tool::Intersect(IntersectSpec::default()));
        let r = p.analyze_linked(&request, &OutputTarget::NewSheet, RecalcMode::Auto).unwrap();
        let x = r.new_columns[0];
        let before = p.clone();
        // A column with no numbers cannot be a curve any more.
        p.column_mut(b).unwrap().data = ColumnData::numeric(vec![f64::NAN, f64::NAN]);
        let problems = p.update_analysis_links(&before);
        assert!(problems.is_empty(), "{problems:?}");
        assert!(p.numeric(x).unwrap().is_empty(), "nothing meets any more");
    }

    #[test]
    fn the_new_links_survive_a_save_as_json() {
        let (mut p, _, a, b) = pair();
        p.analyze_linked(
            &req(&[a, b], Tool::Intersect(IntersectSpec { include_touching: true })),
            &OutputTarget::NewSheet,
            RecalcMode::Manual,
        )
        .unwrap();
        p.analyze_linked(
            &req(&[a, b], Tool::AverageCurves(AverageSpec::default())),
            &OutputTarget::NewSheet,
            RecalcMode::Auto,
        )
        .unwrap();
        p.analyze_linked(&req(&[a], Tool::PolygonArea), &OutputTarget::NewSheet, RecalcMode::Auto).ok();
        let text = serde_json::to_string(&p).unwrap();
        let back: Project = serde_json::from_str(&text).unwrap();
        assert_eq!(back.analysis_links, p.analysis_links);
        assert!(back.analysis_links.len() >= 2);
    }

    #[test]
    fn intersect_can_list_touching_points_with_their_kind() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, 2.0])).unwrap();
        let a = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.0, 1.0, 0.0])).unwrap();
        let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 1.0, 1.0])).unwrap();
        let none = p.analyze(&req(&[a, b], Tool::Intersect(IntersectSpec::default()))).unwrap();
        assert!(none.notes[0].contains("do not meet"), "{:?}", none.notes);
        let r = p.analyze(&req(&[a, b], Tool::Intersect(IntersectSpec { include_touching: true }))).unwrap();
        let ws = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(ws.columns[0].numeric().unwrap(), [1.0]);
        assert_eq!(ws.columns.len(), 3, "X, Y and the kind");
    }
}
