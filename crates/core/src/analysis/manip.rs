//! The data manipulation tools of `Project::analyze` (Analysis > Data Manipulation) and the median
//! filter (Analysis > Mathematics): Subtract Straight Line, Reduce Duplicate X Data, Reduce to
//! Evenly Spaced X, Curve Translate X and Y, Median Filter. The numerics are in `sheaf_calc::manip`; what is known about
//! Origin's behaviour is in `docs/spec/analysis-tools.md` section 12.

use super::*;
use sheaf_calc::{DUP_TOL, DupMethod, LinePoint, MedianEdge, ReduceMethod};

/// What [`Tool::SubtractLine`] does: subtract the straight line through two points from each curve.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SubtractLineSpec {
    pub p1: LinePoint,
    pub p2: LinePoint,
}

/// What [`Tool::ReduceDup`] does (Origin's `reducedup`, measured; analysis-tools.md section 12).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReduceDupSpec {
    pub method: DupMethod,
    /// Neighbouring X values this close, as a fraction of the X range, are one X.
    pub tol: f64,
    /// Also write the X values that had duplicates and how many points each had (Origin's
    /// `xcount`, off by default).
    pub count: bool,
}

impl Default for ReduceDupSpec {
    fn default() -> Self {
        ReduceDupSpec { method: DupMethod::Mean, tol: DUP_TOL, count: false }
    }
}

/// Origin's word for a method, as in the comment of its result column.
pub fn dup_method_name(m: DupMethod) -> &'static str {
    match m {
        DupMethod::Mean => "Mean",
        DupMethod::Median => "Median",
        DupMethod::Min => "Min",
        DupMethod::Max => "Max",
        DupMethod::Sum => "Sum",
        DupMethod::Sd => "SD",
        DupMethod::Se => "SE",
        DupMethod::Rms => "RMS",
    }
}

/// The direction of a [`Tool::Translate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TranslateAxis {
    X,
    Y,
}

/// How far [`Tool::Translate`] moves the curve.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Shift {
    /// By this amount (added to X or to Y).
    By(f64),
    /// So that the point of the curve at X = `at` lands on `to`: on the X axis the point moves
    /// from X = `at` to X = `to`; on the Y axis the curve's value at X = `at` (read between the
    /// data points by straight lines) becomes `to`.
    PointTo { at: f64, to: f64 },
}

/// What [`Tool::Translate`] does.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TranslateSpec {
    pub axis: TranslateAxis,
    pub shift: Shift,
}

/// What [`Tool::MedianFilter`] does.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MedianFilterSpec {
    /// Points of the window; an even number acts like the next odd one, as in Origin's smoothing.
    pub npts: usize,
    pub edge: MedianEdge,
}

impl Default for MedianFilterSpec {
    fn default() -> Self {
        MedianFilterSpec { npts: 5, edge: MedianEdge::Shrink }
    }
}

pub(super) fn run(
    tool: &Tool,
    inputs: &[Input<'_>],
    beside: &mut Vec<NewColumn>,
    on_sheet: &mut Vec<SheetColumn>,
) -> Result<(), AnalysisError> {
    match tool {
        Tool::SubtractLine(spec) => {
            if !(spec.p1.x.is_finite() && spec.p2.x.is_finite()) || spec.p1.x == spec.p2.x {
                return Err(AnalysisError::BadOption("the two points need different X values".into()));
            }
            for inp in inputs {
                if inp.xs().iter().zip(inp.ys()).filter(|(a, b)| a.is_finite() && b.is_finite()).count() < 2 {
                    return Err(AnalysisError::TooFewPoints(inp.short.clone()));
                }
                let v = calc::subtract_line_through(inp.xs(), inp.ys(), spec.p1, spec.p2).ok_or_else(|| {
                    AnalysisError::BadOption(format!(
                        "the two points must lie inside the X range of column {} (or be given a Y value)",
                        inp.short
                    ))
                })?;
                beside.push(NewColumn {
                    after: inp.id,
                    long_name: format!("{} minus line", inp.name),
                    units: inp.units.clone(),
                    comments: format!(
                        "{} minus the straight line through X = {} and X = {}{}",
                        inp.short,
                        spec.p1.x,
                        spec.p2.x,
                        inp.range_note()
                    ),
                    data: inp.aligned(&v),
                });
            }
        }
        Tool::ReduceDup(spec) => {
            if !(spec.tol.is_finite() && spec.tol >= 0.0) {
                return Err(AnalysisError::BadOption(format!("{} is not a usable tolerance", spec.tol)));
            }
            for (k, inp) in inputs.iter().enumerate() {
                let r = calc::reduce_duplicates(inp.xs(), inp.ys(), spec.method, spec.tol)
                    .ok_or_else(|| AnalysisError::TooFewPoints(inp.short.clone()))?;
                let n = k + 1;
                on_sheet.push(SheetColumn {
                    designation: Designation::X,
                    long_name: format!("ReduceDupX{n}"),
                    units: inp.x_units.clone(),
                    comments: String::new(),
                    data: ColumnData::numeric(r.x),
                });
                on_sheet.push(SheetColumn {
                    designation: Designation::Y,
                    long_name: format!("ReduceDupY{n}"),
                    units: inp.units.clone(),
                    comments: format!(
                        "Duplicates replaced with {} of {}{}",
                        dup_method_name(spec.method),
                        inp.short,
                        inp.range_note()
                    ),
                    data: ColumnData::numeric(r.y),
                });
                if spec.count {
                    on_sheet.push(SheetColumn {
                        designation: Designation::X,
                        long_name: format!("DupX{n}"),
                        units: inp.x_units.clone(),
                        comments: String::new(),
                        data: ColumnData::numeric(r.dup_x),
                    });
                    on_sheet.push(SheetColumn {
                        designation: Designation::Y,
                        long_name: format!("DupXCount{n}"),
                        units: String::new(),
                        comments: String::new(),
                        data: ColumnData::numeric(r.dup_count),
                    });
                }
            }
        }
        Tool::ReduceX(opts) => {
            for inp in inputs {
                let (gx, gy) = calc::reduce_evenly(inp.xs(), inp.ys(), opts).ok_or_else(|| {
                    AnalysisError::BadOption(format!(
                        "column {} cannot be reduced: it needs two points with different X values, and the \
                         number of points, interval, number of intervals or frequency must be valid for its X range",
                        inp.short
                    ))
                })?;
                // Origin's comment names the sampling method (measured).
                let how = match opts.method {
                    ReduceMethod::First => "Y First(X First)",
                    ReduceMethod::Mean => "Y Mean(X Average)",
                };
                let n = on_sheet.len() / 2 + 1;
                on_sheet.push(SheetColumn {
                    designation: Designation::X,
                    long_name: format!("ReduceEXX{n}"),
                    units: inp.x_units.clone(),
                    comments: String::new(),
                    data: ColumnData::numeric(gx),
                });
                on_sheet.push(SheetColumn {
                    designation: Designation::Y,
                    long_name: format!("ReduceEXY{n}"),
                    units: inp.units.clone(),
                    comments: format!("{how} Stats Reduction of {}{}", inp.short, inp.range_note()),
                    data: ColumnData::numeric(gy),
                });
            }
        }
        Tool::Translate(spec) => {
            let amount = |inp: &Input<'_>| -> Result<f64, AnalysisError> {
                match (spec.shift, spec.axis) {
                    (Shift::By(d), _) if d.is_finite() => Ok(d),
                    (Shift::By(d), _) => Err(AnalysisError::BadOption(format!("{d} is not a usable shift"))),
                    (Shift::PointTo { at, to }, TranslateAxis::X) if at.is_finite() && to.is_finite() => Ok(to - at),
                    (Shift::PointTo { at, to }, TranslateAxis::Y) => calc::shift_to_land(inp.xs(), inp.ys(), at, to)
                        .ok_or_else(|| {
                            AnalysisError::BadOption(format!(
                                "X = {at} is outside the X range of column {}, so its value there is unknown, \
                                 or the new Y value is not a number",
                                inp.short
                            ))
                        }),
                    (Shift::PointTo { .. }, TranslateAxis::X) => {
                        Err(AnalysisError::BadOption("the point and its new X must be numbers".into()))
                    }
                }
            };
            for inp in inputs {
                let d = amount(inp)?;
                match spec.axis {
                    TranslateAxis::Y => beside.push(NewColumn {
                        after: inp.id,
                        long_name: format!("{} shifted", inp.name),
                        units: inp.units.clone(),
                        comments: format!("{} translated in Y by {d}{}", inp.short, inp.range_note()),
                        data: inp.aligned(&calc::translate(inp.ys(), d)),
                    }),
                    TranslateAxis::X => {
                        on_sheet.push(SheetColumn {
                            designation: Designation::X,
                            long_name: inp.x_name.clone(),
                            units: inp.x_units.clone(),
                            comments: format!("X translated by {d}"),
                            data: ColumnData::numeric(calc::translate(inp.xs(), d)),
                        });
                        on_sheet.push(SheetColumn {
                            designation: Designation::Y,
                            long_name: inp.name.clone(),
                            units: inp.units.clone(),
                            comments: format!("{} translated in X by {d}{}", inp.label, inp.range_note()),
                            data: ColumnData::numeric(inp.ys().to_vec()),
                        });
                    }
                }
            }
        }
        Tool::MedianFilter(spec) => {
            if spec.npts == 0 {
                return Err(AnalysisError::BadOption("the median filter window needs at least 1 point".to_string()));
            }
            let edge = match spec.edge {
                MedianEdge::Shrink => "window shrinks at the ends",
                MedianEdge::Repeat => "end values repeated",
                MedianEdge::Reflect => "ends reflected",
            };
            for inp in inputs {
                let v = calc::median_filter(inp.ys(), spec.npts, spec.edge).expect("the window was checked");
                beside.push(NewColumn {
                    after: inp.id,
                    long_name: format!("Median filtered {}", inp.name),
                    units: inp.units.clone(),
                    comments: format!(
                        "{} median filter, {} points, {edge}{}",
                        inp.short,
                        2 * (spec.npts / 2) + 1,
                        inp.range_note()
                    ),
                    data: inp.aligned(&v),
                });
            }
        }
        _ => unreachable!("only the data manipulation tools are sent here"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::RecalcMode;

    /// X = 0..=4 and Y = 2x + 1 with a bump of 3 at x = 2.
    fn setup() -> (Project, ColumnId, WindowId) {
        let mut p = Project::new();
        let book = p.add_workbook("Book1");
        p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, 2.0, 3.0, 4.0])).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 3.0, 8.0, 7.0, 9.0])).unwrap();
        (p, y, book)
    }

    fn req(y: ColumnId, tool: Tool) -> Request {
        Request { columns: vec![y], rows: RowRange::All, tool }
    }

    #[test]
    fn subtract_line_leaves_the_bump_beside_the_data() {
        let (mut p, y, book) = setup();
        let spec = SubtractLineSpec { p1: LinePoint { x: 0.0, y: None }, p2: LinePoint { x: 4.0, y: None } };
        let r = p.analyze(&req(y, Tool::SubtractLine(spec))).unwrap();
        assert_eq!(p.numeric(r.new_columns[0]).unwrap(), [0.0, 0.0, 3.0, 0.0, 0.0]);
        assert_eq!(p.workbook(book).unwrap().sheets[0].columns[2].id, r.new_columns[0]);
        let same_x = SubtractLineSpec { p1: LinePoint { x: 1.0, y: None }, p2: LinePoint { x: 1.0, y: None } };
        assert!(p.analyze(&req(y, Tool::SubtractLine(same_x))).is_err());
        let outside = SubtractLineSpec { p1: LinePoint { x: 0.0, y: None }, p2: LinePoint { x: 9.0, y: None } };
        assert!(p.analyze(&req(y, Tool::SubtractLine(outside))).is_err());
        // One valid point is not a curve, even with both Y values given.
        let given = SubtractLineSpec { p1: LinePoint { x: 0.0, y: Some(0.0) }, p2: LinePoint { x: 1.0, y: Some(1.0) } };
        let one = p.analyze(&Request { columns: vec![y], rows: RowRange::Rows(1, 1), tool: Tool::SubtractLine(given) });
        assert!(one.is_err(), "{one:?}");
    }

    #[test]
    fn reduce_duplicates_writes_origins_columns_on_a_new_sheet() {
        let mut p = Project::new();
        let book = p.add_workbook("Book1");
        p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![3.0, 1.0, 2.0, 3.0, 2.0])).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0, 5.0])).unwrap();
        // reducedup4/5 in Origin: X 1 2 3, Mean 2 4 2.5, SD 2 1.414 2.121 (a lone point keeps its Y).
        let r = p.analyze(&req(y, Tool::ReduceDup(ReduceDupSpec::default()))).unwrap();
        let sheet = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(sheet.columns.len(), 2);
        assert_eq!(sheet.columns[0].numeric().unwrap(), [1.0, 2.0, 3.0]);
        assert_eq!(sheet.columns[1].numeric().unwrap(), [2.0, 4.0, 2.5]);
        assert_eq!(sheet.columns[0].long_name, "ReduceDupX1");
        assert_eq!(sheet.columns[1].long_name, "ReduceDupY1");
        assert_eq!(sheet.columns[1].comments, "Duplicates replaced with Mean of B");
        let spec = ReduceDupSpec { method: DupMethod::Sd, count: true, ..ReduceDupSpec::default() };
        let r = p.analyze(&req(y, Tool::ReduceDup(spec))).unwrap();
        let sheet = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        let names: Vec<&str> = sheet.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert_eq!(names, ["ReduceDupX1", "ReduceDupY1", "DupX1", "DupXCount1"]);
        let sd = sheet.columns[1].numeric().unwrap();
        assert_eq!(sd[0], 2.0);
        assert!(
            (sd[1] - std::f64::consts::SQRT_2).abs() < 1e-12 && (sd[2] - 2.1213203435596424).abs() < 1e-12,
            "{sd:?}"
        );
        assert_eq!(sheet.columns[2].numeric().unwrap(), [2.0, 3.0]);
        assert_eq!(sheet.columns[3].numeric().unwrap(), [2.0, 2.0]);
        let bad = ReduceDupSpec { tol: f64::NAN, ..ReduceDupSpec::default() };
        assert!(p.analyze(&req(y, Tool::ReduceDup(bad))).is_err());
        assert!(Project::can_link(&Tool::ReduceDup(ReduceDupSpec::default())));
    }

    #[test]
    fn reduce_writes_a_new_sheet_with_even_x() {
        let (mut p, y, book) = setup();
        // Every 2 points: (0, 1), (2, 3) and the short run (4) at the place of a full one.
        let opts = ReduceOptions { group: calc::ReduceGroup::EveryN(2), ..ReduceOptions::default() };
        let r = p.analyze(&req(y, Tool::ReduceX(opts))).unwrap();
        let sheet = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(sheet.columns[0].numeric().unwrap(), [0.5, 2.5, 4.5]);
        assert_eq!(sheet.columns[1].numeric().unwrap(), [2.0, 7.5, 9.0]);
        assert_eq!(
            (sheet.columns[0].long_name.as_str(), sheet.columns[1].long_name.as_str()),
            ("ReduceEXX1", "ReduceEXY1")
        );
        assert_eq!(sheet.columns[1].comments, "Y Mean(X Average) Stats Reduction of B");
    }

    #[test]
    fn translate_y_by_a_point_and_x_by_a_constant() {
        let (mut p, y, book) = setup();
        // The curve is 7 at x = 3; bring it to 0.
        let spec = TranslateSpec { axis: TranslateAxis::Y, shift: Shift::PointTo { at: 3.0, to: 0.0 } };
        let r = p.analyze(&req(y, Tool::Translate(spec))).unwrap();
        assert_eq!(p.numeric(r.new_columns[0]).unwrap(), [-6.0, -4.0, 1.0, 0.0, 2.0]);
        let spec = TranslateSpec { axis: TranslateAxis::X, shift: Shift::By(-1.5) };
        let r = p.analyze(&req(y, Tool::Translate(spec))).unwrap();
        let sheet = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(sheet.columns[0].numeric().unwrap(), [-1.5, -0.5, 0.5, 1.5, 2.5]);
        assert_eq!(sheet.columns[1].numeric().unwrap(), [1.0, 3.0, 8.0, 7.0, 9.0]);
        let outside = TranslateSpec { axis: TranslateAxis::Y, shift: Shift::PointTo { at: 10.0, to: 0.0 } };
        assert!(p.analyze(&req(y, Tool::Translate(outside))).is_err());
        for to in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let spec = TranslateSpec { axis: TranslateAxis::Y, shift: Shift::PointTo { at: 1.0, to } };
            assert!(p.analyze(&req(y, Tool::Translate(spec))).is_err(), "to = {to}");
        }
    }

    #[test]
    fn median_filter_and_its_link() {
        let (mut p, y, book) = setup();
        let tool = Tool::MedianFilter(MedianFilterSpec { npts: 3, edge: MedianEdge::Shrink });
        let r = p.analyze_linked(&req(y, tool), &OutputTarget::NextToInput, RecalcMode::Auto).unwrap();
        let out = r.new_columns[0];
        // 1 3 8 7 9: ends stay, median(1,3,8)=3, (3,8,7)=7, (8,7,9)=8
        assert_eq!(p.numeric(out).unwrap(), [1.0, 3.0, 7.0, 8.0, 9.0]);
        let before = p.clone();
        p.column_mut(y).unwrap().data = ColumnData::numeric(vec![5.0, 5.0, 100.0, 5.0, 5.0]);
        assert!(p.update_analysis_links(&before).is_empty());
        assert_eq!(p.numeric(out).unwrap(), [5.0; 5], "the spike is gone and the same column was updated");
        assert_eq!(p.workbook(book).unwrap().sheets[0].columns.len(), 3);
        let t = Tool::MedianFilter(MedianFilterSpec { npts: 0, edge: MedianEdge::Shrink });
        assert!(p.analyze(&req(y, t)).is_err(), "window 0");
        // An even window acts like the next odd one, as in Origin's smoothing.
        let even = |n| {
            let mut q = before.clone();
            let t = Tool::MedianFilter(MedianFilterSpec { npts: n, edge: MedianEdge::Shrink });
            let r = q.analyze(&req(y, t)).unwrap();
            q.numeric(r.new_columns[0]).unwrap().to_vec()
        };
        assert_eq!(even(2), even(3));
    }

    /// Edits the Y column and runs the Auto links; the results must follow in the same columns.
    fn edit_y(p: &mut Project, y: ColumnId, values: Vec<f64>) {
        let before = p.clone();
        p.column_mut(y).unwrap().data = ColumnData::numeric(values);
        let problems = p.update_analysis_links(&before);
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn all_four_tools_can_be_linked_and_an_auto_result_follows_an_edit() {
        let new_y = vec![0.0, 2.0, 4.0, 6.0, 9.0];
        // Subtract Straight Line: beside the input; the new column is reused.
        let (mut p, y, book) = setup();
        let spec = SubtractLineSpec { p1: LinePoint { x: 0.0, y: None }, p2: LinePoint { x: 4.0, y: None } };
        assert!(Project::can_link(&Tool::SubtractLine(spec)));
        let r = p.analyze_linked(&req(y, Tool::SubtractLine(spec)), &OutputTarget::NextToInput, RecalcMode::Auto);
        let out = r.unwrap().new_columns[0];
        edit_y(&mut p, y, new_y.clone());
        assert_eq!(p.numeric(out).unwrap(), [0.0, -0.25, -0.5, -0.75, 0.0]);
        assert_eq!(p.workbook(book).unwrap().sheets[0].columns.len(), 3);

        // Translate Y by a point: the shift is worked out again from the edited curve (3 -> 0 at x = 3).
        let (mut p, y, _) = setup();
        let spec = TranslateSpec { axis: TranslateAxis::Y, shift: Shift::PointTo { at: 3.0, to: 0.0 } };
        assert!(Project::can_link(&Tool::Translate(spec)));
        let r = p.analyze_linked(&req(y, Tool::Translate(spec)), &OutputTarget::NextToInput, RecalcMode::Auto);
        let out = r.unwrap().new_columns[0];
        edit_y(&mut p, y, new_y.clone());
        assert_eq!(p.numeric(out).unwrap(), [-6.0, -4.0, -2.0, 0.0, 3.0]);

        // Translate X and Reduce to Evenly Spaced X write a new sheet; it is replaced, not added.
        let (mut p, y, book) = setup();
        let spec = TranslateSpec { axis: TranslateAxis::X, shift: Shift::By(10.0) };
        assert!(Project::can_link(&Tool::Translate(spec)));
        let r = p.analyze_linked(&req(y, Tool::Translate(spec)), &OutputTarget::NextToInput, RecalcMode::Auto);
        let out = r.unwrap().new_columns[1];
        edit_y(&mut p, y, new_y.clone());
        assert_eq!(p.numeric(out).unwrap(), new_y.as_slice());
        assert_eq!(p.workbook(book).unwrap().sheets.len(), 2);

        let (mut p, y, book) = setup();
        let opts = ReduceOptions { group: calc::ReduceGroup::EveryN(2), ..ReduceOptions::default() };
        assert!(Project::can_link(&Tool::ReduceX(opts)));
        let r = p.analyze_linked(&req(y, Tool::ReduceX(opts)), &OutputTarget::NextToInput, RecalcMode::Auto);
        let out = r.unwrap().new_columns[1];
        edit_y(&mut p, y, new_y);
        assert_eq!(p.numeric(out).unwrap(), [1.0, 5.0, 9.0]);
        assert_eq!(p.workbook(book).unwrap().sheets.len(), 2);
    }

    #[test]
    fn a_manual_link_of_a_manipulation_tool_waits_for_recalculate_and_its_link_survives_json() {
        let (mut p, y, _) = setup();
        let spec = SubtractLineSpec { p1: LinePoint { x: 0.0, y: None }, p2: LinePoint { x: 4.0, y: None } };
        let r = p.analyze_linked(&req(y, Tool::SubtractLine(spec)), &OutputTarget::NextToInput, RecalcMode::Manual);
        let out = r.unwrap().new_columns[0];
        let old = p.numeric(out).unwrap().to_vec();
        edit_y(&mut p, y, vec![0.0, 2.0, 4.0, 6.0, 9.0]);
        assert_eq!(p.numeric(out).unwrap(), old.as_slice(), "not recalculated yet");
        let link = p.analysis_link_of(out).unwrap();
        p.recalculate_analysis(link).unwrap();
        assert_eq!(p.numeric(out).unwrap(), [0.0, -0.25, -0.5, -0.75, 0.0]);
        // The link (not the cell data, which a project file stores apart) goes through JSON.
        let text = serde_json::to_string(&p).unwrap();
        let back: Project = serde_json::from_str(&text).unwrap();
        assert_eq!(back.analysis_links, p.analysis_links);
    }
}
