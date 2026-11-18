//! Vector XYAM and Vector XYXY plots (T107, graphing spec "Vector plots"): an arrow per row, a 0.5 pt line
//! with a closed head at its end. XYAM arrows start at (X, Y) and run `magnitude x multiplier` points at
//! the row's angle (counterclockwise from +X, radians unless the spec says degrees); XYXY arrows run from
//! (X, Y) to the row's end point, whose X and Y count for the axis ranges.

use crate::data::PlotData;
use crate::scene::{self, Item};
use crate::{LayerGeometry, plots::line_stroke};
use sheaf_core::Project;
use sheaf_core::vector::VectorSpec;

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
    let color = pd.plot.line.color;
    for i in pd.start..pd.end {
        let (x0, y0) = geom.data_to_page(pd.x_at(i), pd.y_at(i));
        let (x1, y1) = match &v.rows {
            VectorRows::Am(r) => {
                let Some(&(angle, len)) = r.get(i) else { continue };
                (x0 + len * angle.cos(), y0 - len * angle.sin())
            }
            VectorRows::Xy(e) => {
                let Some(&(ex, ey)) = e.get(i) else { continue };
                geom.data_to_page(ex, ey)
            }
        };
        if ![x0, y0, x1, y1].iter().all(|c| c.is_finite()) {
            continue;
        }
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len = dx.hypot(dy);
        if len <= 0.0 {
            continue;
        }
        let (ux, uy) = (dx / len, dy / len);
        let head = v.spec.head_pt.min(len);
        let half = (v.spec.head_angle_deg / 2.0).to_radians();
        let w = head * half.tan();
        // The shaft stops at the head's base so the line does not poke through the point.
        let (bx, by) = (x1 - ux * head, y1 - uy * head);
        let mut d = String::new();
        scene::move_to(&mut d, x0, y0);
        scene::line_to(&mut d, bx, by);
        out.push(Item::Path { d, fill: None, stroke: Some(stroke.clone()) });
        let mut h = String::new();
        scene::move_to(&mut h, x1, y1);
        scene::line_to(&mut h, bx - uy * w, by + ux * w);
        scene::line_to(&mut h, bx + uy * w, by - ux * w);
        h.push('Z');
        out.push(Item::Path { d: h, fill: Some(color), stroke: None });
    }
}
