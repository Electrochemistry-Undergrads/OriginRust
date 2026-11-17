//! Piper diagrams (T117, graphing spec "Piper diagram"): the two triangles and the diamond of
//! `sheaf_core::piper` with grid lines every 20 %, and each sample's three points as the plot's symbol.

use crate::LayerGeometry;
use crate::data::PlotData;
use crate::scene::{self, Item, Stroke};
use sheaf_core::piper::{PiperSpec, outlines, place};
use sheaf_core::{Color, Project};

pub(crate) struct PiperData {
    /// Each sample's cation, anion and diamond points.
    pub points: Vec<[(f64, f64); 3]>,
}

impl PiperData {
    pub(crate) fn new(project: &Project, pd: &PlotData, spec: PiperSpec) -> PiperData {
        let col = |id| project.numeric(id).unwrap_or_default();
        let (mg, na_k, cl, so4, hco3) = (col(spec.mg), col(spec.na_k), col(spec.cl), col(spec.so4), col(spec.hco3));
        let at = |v: &[f64], r: usize| v.get(r).copied().unwrap_or(f64::NAN);
        let points = pd
            .rows()
            .filter_map(|r| place(at(&pd.y, r), at(mg, r), at(na_k, r), at(cl, r), at(so4, r), at(hco3, r)))
            .collect();
        PiperData { points }
    }
}

fn line(out: &mut Vec<Item>, geom: &LayerGeometry, pts: &[(f64, f64)], closed: bool, stroke: &Stroke) {
    let mut d = String::new();
    for (k, &(x, y)) in pts.iter().enumerate() {
        let (px, py) = geom.data_to_page(x, y);
        if k == 0 { scene::move_to(&mut d, px, py) } else { scene::line_to(&mut d, px, py) }
    }
    if closed {
        d.push('Z');
    }
    out.push(Item::Path { d, fill: None, stroke: Some(stroke.clone()) });
}

/// Grid lines every 20 % parallel to each side of the polygon with corners `a`, `b`, `c` (a triangle) or
/// of the parallelogram `a`, `b`, `c`, `d`.
fn grid(out: &mut Vec<Item>, geom: &LayerGeometry, poly: &[(f64, f64)], stroke: &Stroke) {
    let lerp = |p: (f64, f64), q: (f64, f64), t: f64| (p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t);
    for k in 1..5 {
        let t = k as f64 / 5.0;
        if poly.len() == 3 {
            let (a, b, c) = (poly[0], poly[1], poly[2]);
            line(out, geom, &[lerp(a, b, t), lerp(a, c, t)], false, stroke);
            line(out, geom, &[lerp(b, a, t), lerp(b, c, t)], false, stroke);
            line(out, geom, &[lerp(c, a, t), lerp(c, b, t)], false, stroke);
        } else {
            let (a, b, c, d) = (poly[0], poly[1], poly[2], poly[3]);
            line(out, geom, &[lerp(a, b, t), lerp(d, c, t)], false, stroke);
            line(out, geom, &[lerp(a, d, t), lerp(b, c, t)], false, stroke);
        }
    }
}

pub(crate) fn draw(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let Some(p) = &pd.piper else { return };
    let thin = Stroke::solid(Color::rgb(204, 204, 204), 0.5);
    let edge = Stroke::solid(Color::BLACK, 1.0);
    for poly in outlines() {
        grid(out, geom, &poly, &thin);
        line(out, geom, &poly, true, &edge);
    }
    let r = pd.plot.symbol.size_pt / 2.0;
    for sample in &p.points {
        for &(x, y) in sample {
            let (px, py) = geom.data_to_page(x, y);
            let mut d = String::new();
            scene::move_to(&mut d, px + r, py);
            for k in 1..=24 {
                let a = k as f64 / 24.0 * std::f64::consts::TAU;
                scene::line_to(&mut d, px + r * a.cos(), py + r * a.sin());
            }
            d.push('Z');
            out.push(Item::Path {
                d,
                fill: Some(pd.plot.symbol.color),
                stroke: Some(Stroke::solid(Color::BLACK, 0.5)),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use sheaf_core::template::Template;
    use sheaf_core::{ColumnData, ColumnId, Designation, Project};

    #[test]
    fn each_sample_is_drawn_three_times_on_equal_x_and_y_units() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let ids: Vec<ColumnId> = [[3.1, 1.0], [1.2, 0.5], [0.8, 4.2], [0.6, 4.0], [0.9, 0.6], [3.5, 1.1]]
            .iter()
            .map(|v| p.add_column(b, 0, Designation::Y, ColumnData::numeric(v.to_vec())).unwrap())
            .collect();
        let g = p.plot_columns(Template::Piper, &ids).unwrap();
        let geom = &layout_geometry(&p, g).unwrap()[0];
        // One X unit is as long on the page as one Y unit, so the triangles are equilateral.
        let (x0, y0) = geom.data_to_page(0.0, 0.0);
        let (x1, _) = geom.data_to_page(1.0, 0.0);
        let (_, y1) = geom.data_to_page(0.0, 1.0);
        assert!(((x1 - x0) - (y0 - y1)).abs() < 0.5, "{} vs {}", x1 - x0, y0 - y1);
        let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
        let red = s.layers[0]
            .plots
            .iter()
            .filter(|it| matches!(it, scene::Item::Path { fill: Some(c), .. } if *c == sheaf_core::Color::RED))
            .count();
        assert_eq!(red, 2 * 3, "two samples, three points each");
    }
}
