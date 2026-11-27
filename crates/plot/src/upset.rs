//! UpSet plot (T403, Origin 2027's UpSet plot, graphing spec 14.35; Sheaf's look, not measured): the set
//! columns of the plot ([`SankeyStyle::stages`]) hold a membership flag per row, counted by `sheaf_calc::upset`.
//!
//! - A cell is in its set when it holds a number other than 0, or text other than empty, `0`, `false` or `no`
//!   (letter case ignored). Masked rows are left out.
//! - The layer's frame is split: the dot matrix takes the lower 38 %, one row per set from the top in set order,
//!   below one column per intersection (largest first); the intersection bars stand above their columns in the
//!   upper part, up to 12 pt below the frame's top, with the count above each bar. Left of the matrix, the set
//!   names (in the X axis tick font at 0.8 of its size) and, left of them, a bar per set growing leftwards from
//!   the names over 20 % of the frame's width (less room for the largest size), its size left of its end.
//! - Bars are dark grey (64, 64, 64), 60 % of a column wide or of a row high; every other set row has a light
//!   grey (240) band; each column has a dot per set, dark grey for member sets and light grey (210) for the
//!   others, the member dots joined by a 2 pt dark grey line. No axes, no legend.

use crate::LayerGeometry;
use crate::data::PlotData;
use crate::fonts::TextMeasurer;
use crate::scene::{Anchor, Item, Stroke, line_to, move_to};
use crate::text::{self, ASCENT, DESCENT};
use sheaf_calc::upset::{UpSet, count_with};
use sheaf_core::graph::{Font, Layer};
use sheaf_core::{Color, Project};

const LABEL_SCALE: f64 = 0.8;
const MATRIX_SHARE: f64 = 0.38;
const SET_BAR_SHARE: f64 = 0.20;
const TOP_ROOM_PT: f64 = 12.0;
const BAR_SHARE: f64 = 0.6;
const DARK: Color = Color { r: 64, g: 64, b: 64, a: 255 };
const LIGHT: Color = Color { r: 210, g: 210, b: 210, a: 255 };
const BAND: Color = Color { r: 240, g: 240, b: 240, a: 255 };

/// What an UpSet plot draws, held by its plot.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct UpSetData {
    pub names: Vec<String>,
    pub upset: UpSet,
    pub font: Font,
}

/// Whether the text of a cell puts its row in the set.
fn text_member(s: &str) -> bool {
    let s = s.trim();
    !(s.is_empty() || s == "0" || s.eq_ignore_ascii_case("false") || s.eq_ignore_ascii_case("no"))
}

impl UpSetData {
    pub fn new(project: &Project, layer: &Layer, pd: &PlotData) -> Option<UpSetData> {
        let columns: Vec<_> = pd.plot.sankey.stages.iter().filter_map(|id| project.column(*id)).collect();
        if columns.is_empty() {
            return None;
        }
        let n = columns.iter().map(|c| c.len()).max().unwrap_or(0);
        let members: Vec<Vec<bool>> = (0..n)
            .filter(|r| !pd.mask.is_some_and(|m| m.contains(r)))
            .map(|r| {
                columns
                    .iter()
                    .map(|c| match c.numeric() {
                        Some(v) => v.get(r).is_some_and(|x| x.is_finite() && *x != 0.0),
                        None => r < c.len() && text_member(&c.cell_text(r)),
                    })
                    .collect()
            })
            .collect();
        let s = &pd.plot.sankey;
        let upset = count_with(&members, columns.len(), s.upset_sort, s.upset_limit);
        let names = columns.iter().map(|c| c.display_name().to_string()).collect();
        let mut font = layer.x_axis.tick_font.clone();
        font.size_pt *= LABEL_SCALE;
        Some(UpSetData { names, upset, font })
    }
}

fn rect(out: &mut Vec<Item>, (x, y, w, h): (f64, f64, f64, f64), fill: Color) {
    let mut d = String::new();
    move_to(&mut d, x, y);
    line_to(&mut d, x + w, y);
    line_to(&mut d, x + w, y + h);
    line_to(&mut d, x, y + h);
    d.push('Z');
    out.push(Item::Path { d, fill: Some(fill), stroke: None });
}

fn dot(out: &mut Vec<Item>, (cx, cy): (f64, f64), r: f64, fill: Color) {
    let mut d = String::new();
    for k in 0..=24 {
        let a = std::f64::consts::TAU * f64::from(k) / 24.0;
        let (x, y) = (cx + r * a.cos(), cy + r * a.sin());
        if k == 0 { move_to(&mut d, x, y) } else { line_to(&mut d, x, y) }
    }
    d.push('Z');
    out.push(Item::Path { d, fill: Some(fill), stroke: None });
}

/// Draws the bands, the set bars and names, the intersection bars and counts, then the dot matrix.
pub(crate) fn draw_upset(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry, m: &dyn TextMeasurer) {
    let Some(ud) = pd.upset.as_deref() else { return };
    let f = geom.frame;
    let (font, size) = (&ud.font, ud.font.size_pt);
    let sets = ud.names.len();
    let combos = &ud.upset.intersections;
    if sets == 0 {
        return;
    }
    let name_w = ud.names.iter().map(|n| m.width(n, font, size)).fold(0.0, f64::max) + 6.0;
    let set_bar_w = SET_BAR_SHARE * f.w;
    let (x_names, x_matrix) = (f.x + set_bar_w, f.x + set_bar_w + name_w);
    let matrix_w = (f.x + f.w - x_matrix).max(1.0);
    let matrix_h = MATRIX_SHARE * f.h;
    let y_matrix = f.y + f.h - matrix_h;
    let row_h = matrix_h / sets as f64;
    let col_w = matrix_w / combos.len().max(1) as f64;
    let baseline = |y_mid: f64| y_mid + (ASCENT - DESCENT) / 2.0 * size;
    // Bands, set bars, sizes and names.
    let max_set = ud.upset.set_sizes.iter().copied().max().unwrap_or(0).max(1) as f64;
    // The longest bar leaves room for its size on its left.
    let number_w = m.width(&(max_set as usize).to_string(), font, size) + 4.0;
    for (s, name) in ud.names.iter().enumerate() {
        let y = y_matrix + s as f64 * row_h;
        let mid = y + row_h / 2.0;
        if s % 2 == 1 {
            rect(out, (x_names, y, f.x + f.w - x_names, row_h), BAND);
        }
        let size_s = ud.upset.set_sizes[s];
        let len = (set_bar_w - number_w - 2.0).max(0.0) * size_s as f64 / max_set;
        let bar_h = BAR_SHARE * row_h;
        rect(out, (x_names - 2.0 - len, mid - bar_h / 2.0, len, bar_h), DARK);
        let at = (x_names - 4.0 - len, baseline(mid));
        text::place(out, m, &size_s.to_string(), font, size, at, Anchor::End, 0.0);
        text::place(out, m, name, font, size, (x_names + 3.0, baseline(mid)), Anchor::Start, 0.0);
    }
    // Intersection bars and counts.
    let max_count = combos.iter().map(|c| c.count).max().unwrap_or(0).max(1) as f64;
    let bars_bottom = y_matrix - 4.0;
    let bars_h = (bars_bottom - f.y - TOP_ROOM_PT).max(0.0);
    for (k, c) in combos.iter().enumerate() {
        let cx = x_matrix + (k as f64 + 0.5) * col_w;
        let h = bars_h * c.count as f64 / max_count;
        let w = BAR_SHARE * col_w;
        rect(out, (cx - w / 2.0, bars_bottom - h, w, h), DARK);
        text::place(out, m, &c.count.to_string(), font, size, (cx, bars_bottom - h - 2.0), Anchor::Middle, 0.0);
    }
    // The dot matrix.
    let r = (0.3 * col_w.min(row_h)).min(5.0);
    for (k, c) in combos.iter().enumerate() {
        let cx = x_matrix + (k as f64 + 0.5) * col_w;
        let cy = |s: usize| y_matrix + (s as f64 + 0.5) * row_h;
        if let (Some(&first), Some(&last)) = (c.sets.first(), c.sets.last())
            && last > first
        {
            let mut d = String::new();
            move_to(&mut d, cx, cy(first));
            line_to(&mut d, cx, cy(last));
            out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(DARK, 2.0)) });
        }
        for s in 0..sets {
            dot(out, (cx, cy(s)), r, if c.sets.contains(&s) { DARK } else { LIGHT });
        }
    }
}

#[cfg(test)]
mod tests {
    use sheaf_core::template::Template;
    use sheaf_core::{ColumnData, Designation, Project};

    #[test]
    fn the_svg_has_the_set_names_the_counts_and_a_dot_per_set_and_intersection() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let a = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 1.0, 0.0, 1.0, 0.0])).unwrap();
        let yes = |v: &[&str]| ColumnData::text(v.iter().map(|s| s.to_string()).collect());
        let c = p.add_column(b, 0, Designation::Y, yes(&["yes", "", "x", "no", "FALSE"])).unwrap();
        p.column_mut(a).unwrap().long_name = "Alpha".into();
        p.column_mut(c).unwrap().long_name = "Gamma".into();
        let g = p.plot_columns(Template::UpSet, &[a, c]).unwrap();
        let svg = crate::render_svg(&p, g).unwrap();
        for name in ["Alpha", "Gamma"] {
            assert!(svg.contains(&format!(">{name}<")), "{name}");
        }
        let graph = p.graph(g).unwrap();
        let data = crate::data::collect(&p, &graph.layers[0]);
        let ud = data.plots[0].upset.as_deref().unwrap();
        // Rows: {A,G}, {A}, {G}, {A}, {}: A only twice, then A and G, G only.
        assert_eq!(ud.upset.set_sizes, [3, 2]);
        let counts: Vec<(Vec<usize>, usize)> =
            ud.upset.intersections.iter().map(|i| (i.sets.clone(), i.count)).collect();
        assert_eq!(counts, [(vec![0], 2), (vec![1], 1), (vec![0, 1], 1)]);
        assert!(svg.contains(">3<") && svg.contains(">2<"), "set sizes and counts are written");
        assert_eq!(svg.matches("fill=\"#404040\"").count(), 2 + 3 + 4, "set bars, intersection bars, member dots");
        assert_eq!(svg.matches("fill=\"#D2D2D2\"").count(), 2, "non-member dots");
    }
}
