//! The data manipulation tools of `Project::analyze` (Analysis > Data Manipulation) and the median
//! filter (Analysis > Mathematics): Subtract Straight Line, Reduce to Evenly Spaced X, Curve
//! Translate X and Y, Median Filter. The numerics are in `sheaf_calc::manip`; what is known about
//! Origin's behaviour is in `docs/spec/analysis-tools.md` section 11.

use super::*;
use sheaf_calc::{LinePoint, MedianEdge, ReduceMethod};

/// What [`Tool::SubtractLine`] does: subtract the straight line through two points from each curve.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SubtractLineSpec {
    pub p1: LinePoint,
    pub p2: LinePoint,
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
    /// Points of the window; odd.
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
        Tool::ReduceX(opts) => {
            for inp in inputs {
                let (gx, gy) = calc::reduce_evenly(inp.xs(), inp.ys(), opts).ok_or_else(|| {
                    AnalysisError::BadOption(format!(
                        "column {} cannot be reduced: it needs two points with different X values, and the \
                         spacing or number of points must be valid for the X range",
                        inp.short
                    ))
                })?;
                let how = match opts.method {
                    ReduceMethod::Interpolate => "interpolated",
                    ReduceMethod::Average => "averaged in bins",
                };
                let n = gx.len();
                on_sheet.push(SheetColumn {
                    designation: Designation::X,
                    long_name: inp.x_name.clone(),
                    units: inp.x_units.clone(),
                    comments: String::new(),
                    data: ColumnData::numeric(gx),
                });
                on_sheet.push(SheetColumn {
                    designation: Designation::Y,
                    long_name: inp.name.clone(),
                    units: inp.units.clone(),
                    comments: format!("{} reduced to {n} evenly spaced X values, {how}{}", inp.label, inp.range_note()),
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
                    (Shift::PointTo { at, to }, TranslateAxis::Y) => {
                        calc::shift_to_land(inp.xs(), inp.ys(), at, to).ok_or_else(|| {
                            AnalysisError::BadOption(format!(
                                "X = {at} is outside the X range of column {}, so its value there is unknown",
                                inp.short
                            ))
                        })
                    }
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
            if spec.npts == 0 || spec.npts % 2 == 0 {
                return Err(AnalysisError::BadOption(format!(
                    "the median filter window must be an odd number of points, not {}",
                    spec.npts
                )));
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
                    comments: format!("{} median filter, {} points, {edge}{}", inp.short, spec.npts, inp.range_note()),
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
    }

    #[test]
    fn reduce_writes_a_new_sheet_with_even_x() {
        let (mut p, y, book) = setup();
        let opts = ReduceOptions { grid: calc::EvenGrid::Count(3), ..ReduceOptions::default() };
        let r = p.analyze(&req(y, Tool::ReduceX(opts))).unwrap();
        let sheet = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(sheet.columns[0].numeric().unwrap(), [0.0, 2.0, 4.0]);
        assert_eq!(sheet.columns[1].numeric().unwrap(), [1.0, 8.0, 9.0]);
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
        for bad in [0, 4] {
            let t = Tool::MedianFilter(MedianFilterSpec { npts: bad, edge: MedianEdge::Shrink });
            assert!(p.analyze(&req(y, t)).is_err(), "window {bad}");
        }
    }
}
