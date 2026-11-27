//! Sankey diagram (T379, Origin 2020's Sankey plot, graphing spec 14.32; Sheaf's look, not measured): the flows of
//! the plot's Y column between the nodes named in its [`SankeyStyle`]'s source and target columns, laid out by
//! `sheaf_calc::sankey`.
//!
//! - The diagram fills the layer's frame, less room on the right for the last stage's names: none (they are
//!   written left of their nodes). No axes, no legend.
//! - Each node is a box in the layer palette's colour for its index (nodes in order of first appearance), with a
//!   thin darker outline; each link is a band of two cubic curves between the source's right edge and the target's
//!   left edge, filled with the source node's colour at 40 % opacity.
//! - A node's name is written right of the node, in the X axis tick font at 0.8 of its size, vertically centred;
//!   for the last stage it is written left of the node, ending a little before it.

use crate::LayerGeometry;
use crate::data::PlotData;
use crate::fonts::TextMeasurer;
use crate::scene::{Item, Stroke, line_to, move_to};
use crate::text::{self, ASCENT, DESCENT};
use sheaf_calc::sankey::{Sankey, layout, layout_in_stages};
use sheaf_core::graph::{Font, Layer, SankeyStyle};
use sheaf_core::{Color, Project};

/// The names are this fraction of the X axis tick font's size.
const LABEL_SCALE: f64 = 0.8;
/// The space between a node and its name, in points.
const LABEL_PAD_PT: f64 = 4.0;

/// What a Sankey diagram draws, held by its plot.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SankeyData {
    pub sankey: Sankey,
    pub colors: Vec<Color>,
    pub font: Font,
}

impl SankeyData {
    /// The flows of the rows `pd` shows: names from the style's columns, values from the plot's Y.
    pub fn new(project: &Project, layer: &Layer, pd: &PlotData) -> Option<SankeyData> {
        let SankeyStyle { source, target, stages, .. } = &pd.plot.sankey;
        let visible: Vec<usize> = pd.rows().filter(|r| !pd.mask.is_some_and(|m| m.contains(r))).collect();
        let sankey = if stages.is_empty() {
            let (source, target) = (project.column((*source)?)?, project.column((*target)?)?);
            let rows: Vec<(String, String, f64)> = visible
                .iter()
                .map(|&r| (source.cell_text(r), target.cell_text(r), pd.y.get(r).copied().unwrap_or(f64::NAN)))
                .collect();
            layout(&rows)
        } else {
            alluvial(project, pd, stages, &visible)?
        };
        if sankey.nodes.is_empty() {
            return None;
        }
        let n = sankey.nodes.len();
        let colors = (0..n).map(|i| layer.palette.color(i, n)).collect();
        let mut font = layer.x_axis.tick_font.clone();
        font.size_pt *= LABEL_SCALE;
        Some(SankeyData { sankey, colors, font })
    }
}

/// The separator between a stage number and a name in the node keys of an alluvial diagram.
const STAGE_SEP: char = '\u{1f}';

/// An alluvial diagram (T381): every row flows through its cells of the stage columns, a node per (stage, name),
/// weighted by the plot's Y value, or 1 when its Y column is not numeric.
fn alluvial(project: &Project, pd: &PlotData, stages: &[sheaf_core::ColumnId], rows: &[usize]) -> Option<Sankey> {
    let columns: Vec<_> = stages.iter().filter_map(|id| project.column(*id)).collect();
    if columns.len() < 2 {
        return None;
    }
    let counts = project.numeric(pd.plot.y).is_none();
    // Counted, the plot has no numbers to give its rows: every row of the stage columns counts.
    let all: Vec<usize>;
    let rows = if counts {
        all = (0..columns.iter().map(|c| c.len()).max().unwrap_or(0)).collect();
        &all[..]
    } else {
        rows
    };
    let mut flows = Vec::new();
    for &r in rows {
        let weight = if counts { 1.0 } else { pd.y.get(r).copied().unwrap_or(f64::NAN) };
        for (i, pair) in columns.windows(2).enumerate() {
            let (a, b) = (pair[0].cell_text(r), pair[1].cell_text(r));
            if a.trim().is_empty() || b.trim().is_empty() {
                continue;
            }
            flows.push((format!("{i}{STAGE_SEP}{a}"), format!("{}{STAGE_SEP}{b}", i + 1), weight));
        }
    }
    let stage_of = |key: &str| key.split_once(STAGE_SEP).and_then(|(s, _)| s.parse().ok());
    let mut sankey = layout_in_stages(&flows, &stage_of);
    for n in &mut sankey.nodes {
        if let Some((_, name)) = n.name.split_once(STAGE_SEP) {
            n.name = name.to_string();
        }
    }
    Some(sankey)
}

fn darker(c: Color) -> Color {
    Color::rgb((f64::from(c.r) * 0.6) as u8, (f64::from(c.g) * 0.6) as u8, (f64::from(c.b) * 0.6) as u8)
}

/// Draws the bands, then the nodes over them, then the names.
pub(crate) fn draw_sankey(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry, m: &dyn TextMeasurer) {
    let Some(sd) = pd.sankey.as_deref() else { return };
    let f = geom.frame;
    let size = sd.font.size_pt;
    // Room right of the frame's content for the names of nodes before the last stage is inside the frame:
    // the last stage's names go left of their nodes, so the diagram uses the whole width.
    let at = |x: f64, y: f64| (f.x + x * f.w, f.y + y * f.h);
    let last = sd.sankey.stages.saturating_sub(1);
    for l in &sd.sankey.links {
        let (s, t) = (&sd.sankey.nodes[l.source], &sd.sankey.nodes[l.target]);
        let (x0, x1) = (s.x1, t.x0);
        let xm = (x0 + x1) / 2.0;
        let mut d = String::new();
        let p = at(x0, l.source_y.0);
        move_to(&mut d, p.0, p.1);
        let (c1, c2, e) = (at(xm, l.source_y.0), at(xm, l.target_y.0), at(x1, l.target_y.0));
        d.push_str(&format!(" C {} {} {} {} {} {}", c1.0, c1.1, c2.0, c2.1, e.0, e.1));
        let p = at(x1, l.target_y.1);
        line_to(&mut d, p.0, p.1);
        let (c1, c2, e) = (at(xm, l.target_y.1), at(xm, l.source_y.1), at(x0, l.source_y.1));
        d.push_str(&format!(" C {} {} {} {} {} {}", c1.0, c1.1, c2.0, c2.1, e.0, e.1));
        d.push('Z');
        let c = sd.colors[l.source];
        out.push(Item::Path { d, fill: Some(Color::rgba(c.r, c.g, c.b, 102)), stroke: None });
    }
    for (i, n) in sd.sankey.nodes.iter().enumerate() {
        if n.y1 <= n.y0 {
            continue;
        }
        let (p0, p1) = (at(n.x0, n.y0), at(n.x1, n.y1));
        let mut d = String::new();
        move_to(&mut d, p0.0, p0.1);
        line_to(&mut d, p1.0, p0.1);
        line_to(&mut d, p1.0, p1.1);
        line_to(&mut d, p0.0, p1.1);
        d.push('Z');
        let c = sd.colors[i];
        out.push(Item::Path { d, fill: Some(c), stroke: Some(Stroke::solid(darker(c), 0.5)) });
    }
    for n in &sd.sankey.nodes {
        if n.y1 <= n.y0 {
            continue;
        }
        let mid = at(0.0, (n.y0 + n.y1) / 2.0).1 + (ASCENT - DESCENT) / 2.0 * size;
        let x = if n.stage == last && last > 0 {
            at(n.x0, 0.0).0 - LABEL_PAD_PT - text::width(m, &n.name, &sd.font, size)
        } else {
            at(n.x1, 0.0).0 + LABEL_PAD_PT
        };
        text::place_label(out, m, &n.name, &sd.font, (x, mid), 0.0, &|_| None);
    }
}

#[cfg(test)]
mod tests {
    use sheaf_core::template::Template;
    use sheaf_core::{ColumnData, Designation, Project};

    #[test]
    fn the_svg_has_a_band_per_link_a_box_per_node_and_the_names() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let text = |v: &[&str]| ColumnData::text(v.iter().map(|s| s.to_string()).collect());
        let s = p.add_column(b, 0, Designation::Y, text(&["Coal", "Gas", "Power", "Power"])).unwrap();
        let t = p.add_column(b, 0, Designation::Y, text(&["Power", "Power", "Homes", "Industry"])).unwrap();
        let v = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![3.0, 2.0, 2.0, 3.0])).unwrap();
        let g = p.plot_columns(Template::Sankey, &[s, t, v]).unwrap();
        let svg = crate::render_svg(&p, g).unwrap();
        // Four links drawn as closed bands of two cubic curves each.
        assert_eq!(svg.matches(" C ").count(), 8, "two curves per band");
        for name in ["Coal", "Gas", "Power", "Homes", "Industry"] {
            assert!(svg.contains(&format!(">{name}<")), "{name}");
        }
        // No axes: no tick labels such as "0".
        assert!(!svg.contains(">0<"));
    }

    #[test]
    fn an_alluvial_diagram_counts_rows_through_its_stages_and_keeps_names_apart_per_stage() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let text = |v: &[&str]| ColumnData::text(v.iter().map(|s| s.to_string()).collect());
        // "x" appears in both stages: two nodes, not one.
        let a = p.add_column(b, 0, Designation::Y, text(&["x", "x", "y"])).unwrap();
        let c = p.add_column(b, 0, Designation::Y, text(&["x", "z", "z"])).unwrap();
        let g = p.plot_columns(Template::Alluvial, &[a, c]).unwrap();
        let svg = crate::render_svg(&p, g).unwrap();
        assert_eq!(svg.matches(" C ").count(), 6, "three flows: x-x, x-z, y-z");
        assert_eq!(svg.matches(">x<").count(), 2, "x on the left and x on the right");
        let graph = p.graph(g).unwrap();
        let data = crate::data::collect(&p, &graph.layers[0]);
        let sd = data.plots[0].sankey.as_deref().unwrap();
        assert_eq!((sd.sankey.stages, sd.sankey.nodes.len()), (2, 4));
        let weight: f64 = sd.sankey.links.iter().map(|l| l.value).sum();
        assert_eq!(weight, 3.0, "one per row");
    }
}
