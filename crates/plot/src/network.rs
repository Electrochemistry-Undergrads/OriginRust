//! Network plot (T383, Origin 2021b's Network plot, graphing spec 14.33; Sheaf's look, not measured): the edges
//! of the plot's source and target columns ([`SankeyStyle`]), weighted by its Y column or counted, laid out by
//! `sheaf_calc::network`.
//!
//! - The layout fills a square centred in the layer's frame (its side the frame's shorter side). No axes, no
//!   legend.
//! - Edges are straight lines in mid grey at 60 % opacity, from 0.5 pt (the lightest) to 4 pt (the heaviest)
//!   wide; nodes are filled circles in the layer palette's colours by node order, with a white 0.75 pt
//!   outline, their radius from 3 pt to 10 pt by the square root of the weighted degree.
//! - A node's name is written right of its circle in the X axis tick font at 0.8 of its size.
//! - Directed (T510, `SankeyStyle::directed`): every arc is drawn from its source's circle to its target's with a
//!   filled head at the target (as long as 4 pt plus the line width, as wide as 0.6 of its length on each side); a pair
//!   joined both ways gets two arcs set 2.5 pt apart on either side of the line between the centres.

use crate::LayerGeometry;
use crate::data::PlotData;
use crate::fonts::TextMeasurer;
use crate::scene::{Item, Stroke, line_to, move_to};
use crate::text::{self, ASCENT, DESCENT};
use sheaf_calc::network::{Network, layout};
use sheaf_core::graph::{Font, Layer, SankeyStyle};
use sheaf_core::{Color, Project};

const LABEL_SCALE: f64 = 0.8;
const MIN_RADIUS_PT: f64 = 3.0;
const MAX_RADIUS_PT: f64 = 10.0;
const MIN_EDGE_PT: f64 = 0.5;
const MAX_EDGE_PT: f64 = 4.0;

/// What a network plot draws, held by its plot.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NetworkData {
    pub network: Network,
    pub colors: Vec<Color>,
    pub font: Font,
    /// Draw the arcs with arrow heads (T510).
    pub directed: bool,
}

impl NetworkData {
    /// The edges of every row: names from the style's columns, weights from the plot's Y (1 when not numeric).
    pub fn new(project: &Project, layer: &Layer, pd: &PlotData) -> Option<NetworkData> {
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
        let network = layout(&rows);
        if network.names.is_empty() {
            return None;
        }
        let k = network.names.len();
        let colors = (0..k).map(|i| layer.palette.color(i, k)).collect();
        let mut font = layer.x_axis.tick_font.clone();
        font.size_pt *= LABEL_SCALE;
        Some(NetworkData { network, colors, font, directed: pd.plot.sankey.directed })
    }
}

/// Draws the edges, then the nodes over them, then the names.
pub(crate) fn draw_network(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry, m: &dyn TextMeasurer) {
    let Some(nd) = pd.network.as_deref() else { return };
    let f = geom.frame;
    let side = f.w.min(f.h);
    let (ox, oy) = (f.x + (f.w - side) / 2.0, f.y + (f.h - side) / 2.0);
    let at = |p: (f64, f64)| (ox + p.0 * side, oy + p.1 * side);
    let net = &nd.network;
    let max_deg = net.degree.iter().copied().fold(0.0, f64::max);
    let radius = |deg: f64| {
        if max_deg > 0.0 {
            MIN_RADIUS_PT + (MAX_RADIUS_PT - MIN_RADIUS_PT) * (deg / max_deg).sqrt()
        } else {
            MIN_RADIUS_PT
        }
    };
    let grey = Color::rgba(128, 128, 128, 153);
    let links = if nd.directed { &net.arcs } else { &net.edges };
    let max_w = links.iter().map(|e| e.2).fold(0.0, f64::max);
    for &(a, b, w) in links {
        let (p, q) = (at(net.positions[a]), at(net.positions[b]));
        let width = if max_w > 0.0 { MIN_EDGE_PT + (MAX_EDGE_PT - MIN_EDGE_PT) * w / max_w } else { MIN_EDGE_PT };
        if !nd.directed {
            let mut d = String::new();
            move_to(&mut d, p.0, p.1);
            line_to(&mut d, q.0, q.1);
            out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(grey, width)) });
            continue;
        }
        // T510: an arrow from the source's circle to the target's, set aside when the pair is joined both ways.
        let (dx, dy) = (q.0 - p.0, q.1 - p.1);
        let len = dx.hypot(dy);
        if len <= 0.0 {
            continue;
        }
        let (ux, uy) = (dx / len, dy / len);
        let both = net.arcs.iter().any(|e| (e.0, e.1) == (b, a));
        let off = if both { 2.5 } else { 0.0 };
        let (nx, ny) = (-uy * off, ux * off);
        let (ra, rb) = (radius(net.degree[a]), radius(net.degree[b]) + 1.0);
        let start = (p.0 + nx + ux * ra, p.1 + ny + uy * ra);
        let tip = (q.0 + nx - ux * rb, q.1 + ny - uy * rb);
        let head = (4.0 + width).min((len - ra - rb).max(0.0));
        let base = (tip.0 - ux * head, tip.1 - uy * head);
        let mut d = String::new();
        move_to(&mut d, start.0, start.1);
        line_to(&mut d, base.0, base.1);
        out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(grey, width)) });
        let half = 0.6 * head;
        let mut h = String::new();
        move_to(&mut h, tip.0, tip.1);
        line_to(&mut h, base.0 - uy * half, base.1 + ux * half);
        line_to(&mut h, base.0 + uy * half, base.1 - ux * half);
        h.push('Z');
        out.push(Item::Path { d: h, fill: Some(grey), stroke: None });
    }
    for (i, &p) in net.positions.iter().enumerate() {
        let (cx, cy) = at(p);
        let r = radius(net.degree[i]);
        let mut d = String::new();
        for k in 0..=32 {
            let a = 2.0 * std::f64::consts::PI * f64::from(k) / 32.0;
            let (x, y) = (cx + r * a.cos(), cy + r * a.sin());
            if k == 0 { move_to(&mut d, x, y) } else { line_to(&mut d, x, y) }
        }
        d.push('Z');
        out.push(Item::Path { d, fill: Some(nd.colors[i]), stroke: Some(Stroke::solid(Color::WHITE, 0.75)) });
    }
    let size = nd.font.size_pt;
    for (i, &p) in net.positions.iter().enumerate() {
        let (cx, cy) = at(p);
        let x = cx + radius(net.degree[i]) + 2.0;
        let y = cy + (ASCENT - DESCENT) / 2.0 * size;
        text::place_label(out, m, &net.names[i], &nd.font, (x, y), 0.0, &|_| None);
    }
}

#[cfg(test)]
mod tests {
    use sheaf_core::template::Template;
    use sheaf_core::{ColumnData, Designation, Project};

    #[test]
    fn the_svg_has_an_edge_per_pair_a_node_per_name_and_the_names() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let text = |v: &[&str]| ColumnData::text(v.iter().map(|s| s.to_string()).collect());
        let s = p.add_column(b, 0, Designation::Y, text(&["Ann", "Bob", "Ann", "Cy"])).unwrap();
        let t = p.add_column(b, 0, Designation::Y, text(&["Bob", "Ann", "Cy", "Dee"])).unwrap();
        // Two text columns and no weight: counted edges.
        let g = p.plot_columns(Template::Network, &[s, t]).unwrap();
        let svg = crate::render_svg(&p, g).unwrap();
        for name in ["Ann", "Bob", "Cy", "Dee"] {
            assert!(svg.contains(&format!(">{name}<")), "{name}");
        }
        let graph = p.graph(g).unwrap();
        let data = crate::data::collect(&p, &graph.layers[0]);
        let nd = data.plots[0].network.as_deref().unwrap();
        assert_eq!(nd.network.edges.len(), 3, "Ann-Bob twice is one edge");
        assert_eq!(nd.network.edges[0].2, 2.0);
        assert!(!svg.contains(">0<"), "no axes");
        // T510: directed, an arrow per arc: a shaft and a filled head each, Ann-Bob both ways.
        let plain_paths = svg.matches("<path").count();
        p.graph_mut(g).unwrap().layers[0].plots[0].sankey.directed = true;
        let arrows = crate::render_svg(&p, g).unwrap();
        let graph = p.graph(g).unwrap();
        let data = crate::data::collect(&p, &graph.layers[0]);
        assert_eq!(data.plots[0].network.as_deref().unwrap().network.arcs.len(), 4, "Ann-Bob, Bob-Ann, Ann-Cy, Cy-Dee");
        assert_eq!(arrows.matches("<path").count(), plain_paths - 3 + 2 * 4, "3 lines become 4 shafts and 4 heads");
    }
}
