//! Piper diagrams (T117, graphing spec "Piper diagram"): the two triangles and the diamond of
//! `sheaf_core::piper`, drawn as Origin 2016 draws them (measured by T389): solid gray grid lines every 20 %
//! and dotted light gray ones between, each side labelled 0 to 100 every 20 toward the corner it ends at,
//! the sides titled with the column names (the diamond's "D+E" and "A+B"), and each sample's three points
//! with the next symbol and colour of Origin's increment list (black square, red circle, green triangle...).

use crate::LayerGeometry;
use crate::data::PlotData;
use crate::scene::{self, Anchor, Item, Stroke, TextItem};
use sheaf_core::piper::{PiperSpec, outlines, place};
use sheaf_core::{Color, Project};

pub(crate) struct PiperData {
    /// Each sample's cation, anion and diamond points.
    pub points: Vec<[(f64, f64); 3]>,
    /// The names of the six columns: Ca, Mg, Na+K, Cl, SO4, HCO3+CO3.
    pub names: [String; 6],
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
        let name = |id| project.column(id).map(|c| c.display_name().to_string()).unwrap_or_default();
        let names = [name(pd.plot.y), name(spec.mg), name(spec.na_k), name(spec.cl), name(spec.so4), name(spec.hco3)];
        PiperData { points, names }
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

fn lerp(p: (f64, f64), q: (f64, f64), t: f64) -> (f64, f64) {
    (p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t)
}

/// Grid lines at the fractions `ts` parallel to each side of the polygon with corners `a`, `b`, `c` (a
/// triangle) or of the parallelogram `a`, `b`, `c`, `d`.
fn grid(out: &mut Vec<Item>, geom: &LayerGeometry, poly: &[(f64, f64)], ts: &[f64], stroke: &Stroke) {
    for &t in ts {
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

/// Origin's increment colours of the samples (black, red, green, blue, cyan, magenta).
pub const SAMPLE_COLORS: [Color; 6] = [
    Color::rgb(0, 0, 0),
    Color::rgb(255, 0, 0),
    Color::rgb(0, 255, 0),
    Color::rgb(0, 0, 255),
    Color::rgb(0, 255, 255),
    Color::rgb(255, 0, 255),
];

/// Sample `k`'s symbol outline round `(px, py)` with half-size `r`: Origin's square, circle, up triangle,
/// down triangle and diamond in turn.
fn symbol(k: usize, px: f64, py: f64, r: f64) -> String {
    let mut d = String::new();
    let poly = |d: &mut String, pts: &[(f64, f64)]| {
        for (i, (x, y)) in pts.iter().enumerate() {
            if i == 0 { scene::move_to(d, px + x, py + y) } else { scene::line_to(d, px + x, py + y) }
        }
        d.push('Z');
    };
    match k % 5 {
        0 => poly(&mut d, &[(-r, -r), (r, -r), (r, r), (-r, r)]),
        1 => {
            let pts: Vec<(f64, f64)> = (0..24)
                .map(|i| {
                    let a = i as f64 / 24.0 * std::f64::consts::TAU;
                    (r * a.cos(), r * a.sin())
                })
                .collect();
            poly(&mut d, &pts);
        }
        2 => poly(&mut d, &[(0.0, -1.15 * r), (r, 0.85 * r), (-r, 0.85 * r)]),
        3 => poly(&mut d, &[(0.0, 1.15 * r), (r, -0.85 * r), (-r, -0.85 * r)]),
        _ => poly(&mut d, &[(0.0, -1.2 * r), (1.2 * r, 0.0), (0.0, 1.2 * r), (-1.2 * r, 0.0)]),
    }
    d
}

pub(crate) fn draw(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry, font: &sheaf_core::graph::Font) {
    let Some(p) = &pd.piper else { return };
    let major = Stroke::solid(Color::rgb(128, 128, 128), 0.5);
    let minor = Stroke { dash: vec![1.0, 2.0], ..Stroke::solid(Color::rgb(192, 192, 192), 0.5) };
    let edge = Stroke::solid(Color::BLACK, 1.0);
    let polys = outlines();
    for poly in &polys {
        grid(out, geom, poly, &[0.1, 0.3, 0.5, 0.7, 0.9], &minor);
        grid(out, geom, poly, &[0.2, 0.4, 0.6, 0.8], &major);
        line(out, geom, poly, true, &edge);
    }
    // Labels: each labelled side runs from corner `from` to corner `to` and counts 0 to 100 toward `to`;
    // its title is written further out at its middle.
    let text = |out: &mut Vec<Item>, (x, y): (f64, f64), t: String, size: f64| {
        let (px, py) = geom.data_to_page(x, y);
        out.push(Item::Text(TextItem {
            x: px,
            y: py + 0.35 * size,
            text: t,
            family: font.family.clone(),
            size,
            bold: false,
            italic: false,
            color: Color::BLACK,
            anchor: Anchor::Middle,
            rotate_ccw: 0.0,
        }));
    };
    let n = &p.names;
    let (cat, an, dia) = (&polys[0], &polys[1], &polys[2]);
    let sides: [((f64, f64), (f64, f64), (f64, f64), String); 8] = [
        (cat[1], cat[0], centroid(cat), n[0].clone()),
        (cat[0], cat[2], centroid(cat), n[1].clone()),
        (cat[2], cat[1], centroid(cat), n[2].clone()),
        (an[0], an[1], centroid(an), n[3].clone()),
        (an[1], an[2], centroid(an), n[4].clone()),
        (an[2], an[0], centroid(an), n[5].clone()),
        (dia[3], dia[2], centroid(dia), format!("{}+{}", n[3], n[4])),
        (dia[1], dia[2], centroid(dia), format!("{}+{}", n[0], n[1])),
    ];
    for (from, to, centre, title) in sides {
        let mid = lerp(from, to, 0.5);
        let (ox, oy) = (mid.0 - centre.0, mid.1 - centre.1);
        let len = ox.hypot(oy).max(1e-9);
        let (ux, uy) = (ox / len, oy / len);
        for k in 0..=5 {
            let at = lerp(from, to, k as f64 / 5.0);
            text(out, (at.0 + 0.06 * ux, at.1 + 0.06 * uy), (k * 20).to_string(), 0.8 * font.size_pt);
        }
        text(out, (mid.0 + 0.16 * ux, mid.1 + 0.16 * uy), title, font.size_pt);
    }
    let r = pd.plot.symbol.size_pt / 2.0;
    for (k, sample) in p.points.iter().enumerate() {
        for &(x, y) in sample {
            let (px, py) = geom.data_to_page(x, y);
            out.push(Item::Path {
                d: symbol(k, px, py, r),
                fill: Some(SAMPLE_COLORS[k % SAMPLE_COLORS.len()]),
                stroke: None,
            });
        }
    }
}

fn centroid(poly: &[(f64, f64)]) -> (f64, f64) {
    let n = poly.len() as f64;
    (poly.iter().map(|p| p.0).sum::<f64>() / n, poly.iter().map(|p| p.1).sum::<f64>() / n)
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
        // Origin's increment (T389): the first sample black, the second red, three points each.
        let filled = |c: sheaf_core::Color| {
            s.layers[0]
                .plots
                .iter()
                .filter(|it| matches!(it, scene::Item::Path { fill: Some(f), .. } if *f == c))
                .count()
        };
        assert_eq!((filled(sheaf_core::Color::BLACK), filled(sheaf_core::Color::RED)), (3, 3));
        // Each labelled side counts 0 to 100 by 20; the sides are titled with the column names.
        let texts: Vec<&str> = s.layers[0]
            .plots
            .iter()
            .filter_map(|it| if let scene::Item::Text(t) = it { Some(t.text.as_str()) } else { None })
            .collect();
        assert_eq!(texts.iter().filter(|t| **t == "100").count(), 8, "eight labelled sides");
        let name = |k: usize| p.column(ids[k]).unwrap().display_name().to_string();
        assert!(texts.contains(&name(0).as_str()) && texts.contains(&format!("{}+{}", name(3), name(4)).as_str()));
    }
}
