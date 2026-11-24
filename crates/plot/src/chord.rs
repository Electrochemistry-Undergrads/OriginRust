//! Chord diagram (T402, Origin 2021's Chord Diagram, graphing spec 14.34; Sheaf's look, not measured): the flows
//! of the plot's source and target columns ([`SankeyStyle`]), weighted by its Y column or counted, laid out by
//! `sheaf_calc::chord`.
//!
//! - The circle is centred in the layer's frame; the ring of arcs runs from 0.40 to 0.43 of the frame's shorter
//!   side from the centre. No axes, no legend.
//! - Each node's arc is filled in the layer palette's colour by node order, with a white 0.75 pt outline.
//! - Each flow is a ribbon inside the ring from its part of the source's arc to its part of the target's arc,
//!   its sides curving through the centre (cubic curves with both control points at the centre), filled in the
//!   source's colour at 65 % opacity without an outline.
//! - A node's name is written outside its arc at its middle, 4 pt beyond the ring, in the X axis tick font at 0.8
//!   of its size: on the right half starting there, on the left half ending there.

use crate::LayerGeometry;
use crate::data::PlotData;
use crate::fonts::TextMeasurer;
use crate::scene::{Item, Stroke, line_to, move_to, push_num};
use crate::text::{self, ASCENT, DESCENT};
use sheaf_calc::chord::{Chord, layout};
use sheaf_core::graph::{Font, Layer, SankeyStyle};
use sheaf_core::{Color, Project};

const LABEL_SCALE: f64 = 0.8;
const INNER: f64 = 0.40;
const OUTER: f64 = 0.43;
const LABEL_GAP_PT: f64 = 4.0;
const RIBBON_ALPHA: u8 = 166;
/// The longest step of an arc drawn as straight pieces (1 degree).
const ARC_STEP: f64 = std::f64::consts::PI / 180.0;

/// What a chord diagram draws, held by its plot.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ChordData {
    pub chord: Chord,
    pub colors: Vec<Color>,
    pub font: Font,
}

impl ChordData {
    /// The flows of every row: names from the style's columns, values from the plot's Y (1 when not numeric).
    pub fn new(project: &Project, layer: &Layer, pd: &PlotData) -> Option<ChordData> {
        let SankeyStyle { source, target, .. } = &pd.plot.sankey;
        let (source, target) = (project.column((*source)?)?, project.column((*target)?)?);
        let weights = project.numeric(pd.plot.y);
        let n = source.len().max(target.len());
        let rows: Vec<(String, String, f64)> = (0..n)
            .filter(|r| weights.is_none() || !pd.mask.is_some_and(|m| m.contains(r)))
            .map(|r| {
                let w = weights.map_or(1.0, |w| w.get(r).copied().unwrap_or(f64::NAN));
                (source.cell_text(r), target.cell_text(r), w)
            })
            .collect();
        let chord = layout(&rows);
        if chord.names.is_empty() {
            return None;
        }
        let k = chord.names.len();
        let colors = (0..k).map(|i| layer.palette.color(i, k)).collect();
        let mut font = layer.x_axis.tick_font.clone();
        font.size_pt *= LABEL_SCALE;
        Some(ChordData { chord, colors, font })
    }
}

/// The point at `angle` (0 at the top, clockwise) and `radius` from `c`.
fn at(c: (f64, f64), radius: f64, angle: f64) -> (f64, f64) {
    (c.0 + radius * angle.sin(), c.1 - radius * angle.cos())
}

/// Adds straight pieces along the circle of `radius` from angle `a` to `b` (the start point is already in `d`).
fn arc_to(d: &mut String, c: (f64, f64), radius: f64, a: f64, b: f64) {
    let steps = (((b - a).abs() / ARC_STEP).ceil() as usize).max(1);
    for k in 1..=steps {
        let p = at(c, radius, a + (b - a) * k as f64 / steps as f64);
        line_to(d, p.0, p.1);
    }
}

/// Adds a cubic curve to `p` with both control points at the centre `c`.
fn curve_to(d: &mut String, c: (f64, f64), p: (f64, f64)) {
    d.push('C');
    for (k, v) in [c.0, c.1, c.0, c.1, p.0, p.1].into_iter().enumerate() {
        if k > 0 {
            d.push(' ');
        }
        push_num(d, v);
    }
}

/// Draws the ribbons, then the arcs over them, then the names.
pub(crate) fn draw_chord(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry, m: &dyn TextMeasurer) {
    let Some(cd) = pd.chord.as_deref() else { return };
    let f = geom.frame;
    let side = f.w.min(f.h);
    let c = (f.x + f.w / 2.0, f.y + f.h / 2.0);
    let (r0, r1) = (INNER * side, OUTER * side);
    let chord = &cd.chord;
    for rb in &chord.ribbons {
        let mut d = String::new();
        let p = at(c, r0, rb.from.0);
        move_to(&mut d, p.0, p.1);
        arc_to(&mut d, c, r0, rb.from.0, rb.from.1);
        curve_to(&mut d, c, at(c, r0, rb.to.0));
        arc_to(&mut d, c, r0, rb.to.0, rb.to.1);
        curve_to(&mut d, c, p);
        d.push('Z');
        let base = cd.colors[rb.source];
        out.push(Item::Path { d, fill: Some(Color { a: RIBBON_ALPHA, ..base }), stroke: None });
    }
    for (i, &(a, b)) in chord.arcs.iter().enumerate() {
        let mut d = String::new();
        let p = at(c, r0, a);
        move_to(&mut d, p.0, p.1);
        arc_to(&mut d, c, r0, a, b);
        let q = at(c, r1, b);
        line_to(&mut d, q.0, q.1);
        arc_to(&mut d, c, r1, b, a);
        d.push('Z');
        out.push(Item::Path { d, fill: Some(cd.colors[i]), stroke: Some(Stroke::solid(Color::WHITE, 0.75)) });
    }
    let size = cd.font.size_pt;
    for (i, &(a, b)) in chord.arcs.iter().enumerate() {
        let mid = (a + b) / 2.0;
        let (x, y) = at(c, r1 + LABEL_GAP_PT, mid);
        let name = &chord.names[i];
        let width = m.width(name, &cd.font, size);
        // Right half: the name starts at the point; left half: it ends there. Names near the top and the
        // bottom are moved off the ring by half their height.
        let x = if mid.sin() >= -1e-9 { x } else { x - width };
        let y = y + (ASCENT - DESCENT) / 2.0 * size - mid.cos() * size / 2.0;
        text::place_label(out, m, name, &cd.font, (x, y), 0.0, &|_| None);
    }
}

#[cfg(test)]
mod tests {
    use sheaf_core::template::Template;
    use sheaf_core::{ColumnData, Designation, Project};

    #[test]
    fn the_svg_has_a_ribbon_per_flow_an_arc_per_name_and_the_names() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let text = |v: &[&str]| ColumnData::text(v.iter().map(|s| s.to_string()).collect());
        let s = p.add_column(b, 0, Designation::Y, text(&["Ann", "Bob", "Ann", "Cy"])).unwrap();
        let t = p.add_column(b, 0, Designation::Y, text(&["Bob", "Ann", "Bob", "Dee"])).unwrap();
        // Two text columns and no weight: counted flows.
        let g = p.plot_columns(Template::Chord, &[s, t]).unwrap();
        let svg = crate::render_svg(&p, g).unwrap();
        for name in ["Ann", "Bob", "Cy", "Dee"] {
            assert!(svg.contains(&format!(">{name}<")), "{name}");
        }
        let graph = p.graph(g).unwrap();
        let data = crate::data::collect(&p, &graph.layers[0]);
        let cd = data.plots[0].chord.as_deref().unwrap();
        assert_eq!(cd.chord.ribbons.len(), 3, "Ann to Bob twice is one flow, Bob to Ann another");
        assert_eq!(cd.chord.ribbons[0].value, 2.0);
        assert_eq!(svg.matches("fill-opacity=\"0.651\"").count(), 3, "a translucent ribbon per flow");
        assert!(!svg.contains(">0<"), "no axes");
    }
}
