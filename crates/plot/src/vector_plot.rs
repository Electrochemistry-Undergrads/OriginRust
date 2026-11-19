//! Vector XYAM and Vector XYXY plots (T107, graphing spec "Vector plots"): an arrow per row. XYAM
//! anchors a point of the arrow at (X, Y) and runs `magnitude x multiplier` points at the row's angle.
//! XYXY runs from (X, Y) to the row's end point, whose X and Y count for the axis ranges.

use crate::data::PlotData;
use crate::scene::{self, Item, Stroke};
use crate::{LayerGeometry, plots::line_stroke};
use sheaf_core::Project;
use sheaf_core::graph::LineStyle;
use sheaf_core::vector::{VectorPosition, VectorSpec};

/// The per-row data of a vector plot.
#[derive(Clone, Debug, PartialEq)]
pub enum VectorRows {
    /// Angle (radians) and length (points) of each row.
    Am(Vec<(f64, f64)>),
    /// The end point of each row, in data units.
    Xy(Vec<(f64, f64)>),
}

pub(crate) struct VectorData {
    pub spec: VectorSpec,
    pub rows: VectorRows,
}

impl VectorData {
    /// The rows' data, read from the spec's columns.
    pub(crate) fn new(project: &Project, spec: VectorSpec, xyam: bool, rows: usize) -> VectorData {
        let col = |id| project.numeric(id).unwrap_or_default();
        let (a, b) = (col(spec.a), col(spec.b));
        let at = |v: &[f64], i: usize| v.get(i).copied().unwrap_or(f64::NAN);
        let rows = if xyam {
            VectorRows::Am(
                (0..rows)
                    .map(|i| {
                        let angle = if spec.degrees { at(a, i).to_radians() } else { at(a, i) };
                        (angle, at(b, i) * spec.multiplier)
                    })
                    .collect(),
            )
        } else {
            VectorRows::Xy((0..rows).map(|i| (at(a, i), at(b, i))).collect())
        };
        VectorData { spec, rows }
    }

    /// XYXY: the end points, for the axis ranges.
    pub(crate) fn ends(&self) -> &[(f64, f64)] {
        match &self.rows {
            VectorRows::Xy(e) => e,
            VectorRows::Am(_) => &[],
        }
    }
}

/// The arrows of a vector plot.
pub(crate) fn draw_vectors(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let Some(v) = &pd.vector else { return };
    let stroke = line_stroke(&pd.plot.line);
    let spread = (v.spec.head_angle_deg / 2.0).to_radians().tan();
    for i in pd.start..pd.end {
        let (x0, y0) = geom.data_to_page(pd.x_at(i), pd.y_at(i));
        let (start, end) = match &v.rows {
            VectorRows::Am(r) => {
                let Some(&(angle, len)) = r.get(i) else { continue };
                let (dx, dy) = (len * angle.cos(), -len * angle.sin());
                match v.spec.position {
                    VectorPosition::Tail => ((x0, y0), (x0 + dx, y0 + dy)),
                    VectorPosition::Midpoint => ((x0 - dx / 2.0, y0 - dy / 2.0), (x0 + dx / 2.0, y0 + dy / 2.0)),
                    VectorPosition::Head => ((x0 - dx, y0 - dy), (x0, y0)),
                }
            }
            VectorRows::Xy(e) => {
                let Some(&(ex, ey)) = e.get(i) else { continue };
                ((x0, y0), geom.data_to_page(ex, ey))
            }
        };
        draw_arrow(out, start, end, &stroke, v.spec.head_pt, spread, v.spec.open_head);
    }
}

/// Legend arrows point right and keep their head inside the sample's available space.
pub(crate) fn draw_legend_sample(
    out: &mut Vec<Item>,
    (x, y): (f64, f64),
    width: f64,
    height: f64,
    style: &LineStyle,
    spec: &VectorSpec,
) {
    let spread = (spec.head_angle_deg / 2.0).to_radians().tan();
    let head = spec.head_pt.max(0.0).min(width / 2.0).min(height / 2.0 / spread.abs());
    draw_arrow(out, (x, y), (x + width, y), &line_stroke(style), head, spread, spec.open_head);
}

fn draw_arrow(
    out: &mut Vec<Item>,
    (x0, y0): (f64, f64),
    (x1, y1): (f64, f64),
    stroke: &Stroke,
    head: f64,
    spread: f64,
    open_head: bool,
) {
    if ![x0, y0, x1, y1].iter().all(|c| c.is_finite()) {
        return;
    }
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len = dx.hypot(dy);
    if len <= 0.0 {
        return;
    }
    let (ux, uy) = (dx / len, dy / len);
    let head = head.min(len);
    let w = head * spread;
    let (bx, by) = (x1 - ux * head, y1 - uy * head);
    let (left, right) = ((bx - uy * w, by + ux * w), (bx + uy * w, by - ux * w));
    // Trim the shaft at the base of a filled head and connect it to the tip of an open head.
    let (sx, sy) = if open_head { (x1, y1) } else { (bx, by) };
    let mut d = String::new();
    scene::move_to(&mut d, x0, y0);
    scene::line_to(&mut d, sx, sy);
    out.push(Item::Path { d, fill: None, stroke: Some(stroke.clone()) });
    let mut h = String::new();
    if open_head {
        scene::move_to(&mut h, left.0, left.1);
        scene::line_to(&mut h, x1, y1);
        scene::line_to(&mut h, right.0, right.1);
        let mut head_stroke = stroke.clone();
        head_stroke.dash.clear();
        out.push(Item::Path { d: h, fill: None, stroke: Some(head_stroke) });
    } else {
        scene::move_to(&mut h, x1, y1);
        scene::line_to(&mut h, left.0, left.1);
        scene::line_to(&mut h, right.0, right.1);
        h.push('Z');
        out.push(Item::Path { d: h, fill: Some(stroke.color), stroke: None });
    }
}
