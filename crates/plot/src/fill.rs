//! Fills with a transparency, a hatch pattern or a gradient (Plot Details > Pattern and the
//! layer's Background tab), for the bars, areas and rectangles of a figure.
//!
//! The scene has no clip item and no gradient item, so both are computed here: a hatch is cut
//! against the polygon ([`crate::pie::hatch`]), and a gradient is a run of bands, each the polygon
//! cut to a strip across the gradient and filled with one colour of the blend. A plain fill
//! (solid, no pattern, no gradient) is a single item, drawn as before these settings existed.

use crate::Rect;
use crate::pie::hatch;
use crate::scene::{Item, Stroke, line_to, move_to};
use sheaf_core::Color;
use sheaf_core::graph::{BorderStyle, FillStyle, Gradient, LayerBackground, opacity_for};

/// A gradient is drawn as at least this many bands, and one per this many points of its length.
const MIN_BANDS: usize = 4;
const BAND_PT: f64 = 2.0;
const MAX_BANDS: usize = 64;
/// A band of an opaque fill reaches this far over the next one, so that no seam of the page shows
/// between them. A see-through fill has no overlap: the strip would be painted twice and show as a
/// darker line at every band.
const BAND_OVERLAP_PT: f64 = 0.2;

/// `color` made `transparency_pct` percent see-through (its own transparency counts too).
pub(crate) fn tint(color: Color, transparency_pct: u8) -> Color {
    if transparency_pct == 0 {
        return color;
    }
    let a = (f64::from(color.a) * opacity_for(transparency_pct)).round().clamp(0.0, 255.0) as u8;
    Color { a, ..color }
}

/// The four corners of a rectangle as a polygon.
pub(crate) fn rect_poly(r: Rect) -> [(f64, f64); 4] {
    [(r.x, r.y), (r.x + r.w, r.y), (r.x + r.w, r.y + r.h), (r.x, r.y + r.h)]
}

fn path(poly: &[(f64, f64)]) -> String {
    let mut d = String::new();
    for (i, &(x, y)) in poly.iter().enumerate() {
        if i == 0 {
            move_to(&mut d, x, y);
        } else {
            line_to(&mut d, x, y);
        }
    }
    d.push('Z');
    d
}

/// The part of `poly` whose coordinate `axis` (0 for x, 1 for y) lies in `lo..=hi` (Sutherland and
/// Hodgman against two parallel half-planes; fine for a concave polygon that is being filled).
fn clip_slab(poly: &[(f64, f64)], axis: usize, lo: f64, hi: f64) -> Vec<(f64, f64)> {
    let coord = |p: &(f64, f64)| if axis == 0 { p.0 } else { p.1 };
    let cut = |input: Vec<(f64, f64)>, bound: f64, keep_above: bool| -> Vec<(f64, f64)> {
        let inside = |p: &(f64, f64)| if keep_above { coord(p) >= bound } else { coord(p) <= bound };
        let mut out = Vec::with_capacity(input.len() + 2);
        for i in 0..input.len() {
            let (a, b) = (input[i], input[(i + 1) % input.len()]);
            let (ia, ib) = (inside(&a), inside(&b));
            if ia {
                out.push(a);
            }
            if ia != ib {
                let t = (bound - coord(&a)) / (coord(&b) - coord(&a));
                out.push((a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1)));
            }
        }
        out
    };
    let above = cut(poly.to_vec(), lo, true);
    if above.is_empty() { above } else { cut(above, hi, false) }
}

/// The bands of a gradient over `poly`: each a polygon and its colour, in drawing order. With
/// `translucent` the bands meet edge to edge instead of overlapping.
fn bands(poly: &[(f64, f64)], color: Color, gradient: &Gradient, translucent: bool) -> Vec<(Vec<(f64, f64)>, Color)> {
    let overlap = if translucent { 0.0 } else { BAND_OVERLAP_PT };
    let (vx, vy) = gradient.direction.vector();
    let axis = if vx != 0.0 { 0 } else { 1 };
    let sign = if vx + vy < 0.0 { -1.0 } else { 1.0 };
    let along = |p: &(f64, f64)| sign * if axis == 0 { p.0 } else { p.1 };
    let (lo, hi) = poly.iter().map(along).fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(v), b.max(v)));
    let length = hi - lo;
    if !(length.is_finite() && length > 0.0) {
        return vec![(poly.to_vec(), color)];
    }
    let n = ((length / BAND_PT).ceil() as usize).clamp(MIN_BANDS, MAX_BANDS);
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let (a, b) = (lo + length * k as f64 / n as f64, lo + length * (k + 1) as f64 / n as f64);
        // Back in page coordinates: a band along a negative direction runs from -b to -a.
        let (from, to) = if sign > 0.0 { (a, b + overlap) } else { (-b - overlap, -a) };
        let piece = clip_slab(poly, axis, from, to);
        if piece.len() >= 3 {
            out.push((piece, color.lerp(&gradient.color, (k as f64 + 0.5) / n as f64)));
        }
    }
    out
}

/// Draws `poly` filled with `color` in `style`: the flat colour or the bands of the gradient, then
/// the hatch. A plain style draws one path, as a flat fill always was.
pub(crate) fn fill_polygon(out: &mut Vec<Item>, poly: &[(f64, f64)], color: Color, style: &FillStyle) {
    if poly.len() < 3 {
        return;
    }
    match &style.gradient {
        Some(g) => {
            let translucent = style.transparency_pct > 0 || color.a < 255 || g.color.a < 255;
            for (piece, c) in bands(poly, color, g, translucent) {
                out.push(Item::Path { d: path(&piece), fill: Some(tint(c, style.transparency_pct)), stroke: None });
            }
        }
        None => out.push(Item::Path { d: path(poly), fill: Some(tint(color, style.transparency_pct)), stroke: None }),
    }
    hatch_polygon(out, poly, style);
}

/// The hatch of `style` inside `poly` (nothing for the pattern "None").
pub(crate) fn hatch_polygon(out: &mut Vec<Item>, poly: &[(f64, f64)], style: &FillStyle) {
    let spacing = style.pattern_spacing_pt.max(0.5);
    let mut d = String::new();
    for &angle in style.pattern.angles() {
        for (p, q) in hatch(poly, angle.to_radians(), spacing) {
            move_to(&mut d, p.0, p.1);
            line_to(&mut d, q.0, q.1);
        }
    }
    if !d.is_empty() {
        let color = tint(style.pattern_color, style.transparency_pct);
        out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(color, style.pattern_width_pt.max(0.05))) });
    }
}

/// A rectangle filled in `style`, then outlined with `outline` when given.
pub(crate) fn fill_rect(out: &mut Vec<Item>, rect: Rect, color: Color, style: &FillStyle, outline: Option<Stroke>) {
    if style.is_plain() {
        out.push(Item::Rect { rect, fill: Some(color), stroke: outline });
        return;
    }
    fill_polygon(out, &rect_poly(rect), color, style);
    if outline.is_some() {
        out.push(Item::Rect { rect, fill: None, stroke: outline });
    }
}

/// The stroke of a border line.
pub(crate) fn border_stroke(b: &BorderStyle) -> Stroke {
    let dash: Vec<f64> = b.dash.pattern().iter().map(|m| m * b.width_pt).collect();
    Stroke { color: b.color, width: b.width_pt, dash, cap: crate::scene::Cap::Butt, join: crate::scene::Join::Miter }
}

/// The colour behind a layer's plots: the flat colour or the bands of its gradient, over the frame.
pub(crate) fn draw_background(out: &mut Vec<Item>, bg: &LayerBackground, frame: Rect) {
    if let Some(color) = bg.color {
        let style = FillStyle { transparency_pct: bg.transparency_pct, gradient: bg.gradient, ..FillStyle::default() };
        fill_rect(out, frame, color, &style, None);
    }
}

/// The border line of a layer, drawn over its plots along the frame.
pub(crate) fn draw_border(out: &mut Vec<Item>, bg: &LayerBackground, frame: Rect) {
    if let Some(b) = bg.border.as_ref().filter(|b| b.width_pt > 0.0) {
        out.push(Item::Rect { rect: frame, fill: None, stroke: Some(border_stroke(b)) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::graph::{FillPattern, GradientDirection};

    fn square() -> Rect {
        Rect { x: 10.0, y: 20.0, w: 40.0, h: 30.0 }
    }

    #[test]
    fn a_plain_fill_is_one_rectangle_as_before() {
        let mut out = Vec::new();
        let stroke = Stroke::solid(Color::BLACK, 0.5);
        fill_rect(&mut out, square(), Color::RED, &FillStyle::default(), Some(stroke.clone()));
        assert_eq!(out, vec![Item::Rect { rect: square(), fill: Some(Color::RED), stroke: Some(stroke) }]);
    }

    #[test]
    fn transparency_scales_the_alpha() {
        assert_eq!(tint(Color::RED, 0), Color::RED);
        assert_eq!(tint(Color::RED, 50).a, 128);
        assert_eq!(tint(Color::RED, 100).a, 0);
        assert_eq!(tint(Color::rgba(1, 2, 3, 100), 50).a, 50);
    }

    /// The segments of a hatch path "M x y L x y ...", as (x0, y0, x1, y1).
    fn segments(d: &str) -> Vec<[f64; 4]> {
        d.split('M')
            .skip(1)
            .map(|s| {
                let n: Vec<f64> = s.replace('L', " ").split_whitespace().map(|v| v.parse().unwrap()).collect();
                [n[0], n[1], n[2], n[3]]
            })
            .collect()
    }

    fn hatch_of(poly: &[(f64, f64)]) -> Vec<[f64; 4]> {
        let style = FillStyle { pattern: FillPattern::Horizontal, pattern_spacing_pt: 5.0, ..FillStyle::default() };
        let mut out = Vec::new();
        fill_polygon(&mut out, poly, Color::RED, &style);
        // The fill, then one path of hatch lines.
        assert_eq!(out.len(), 2);
        let Item::Path { d, stroke: Some(s), .. } = &out[1] else { panic!("{:?}", out[1]) };
        assert_eq!(s.color, Color::BLACK);
        segments(d)
    }

    #[test]
    fn a_hatch_has_a_line_every_spacing_and_stays_inside_the_polygon() {
        // A rectangle x 10..50, y 20..50: the lines at 25, 30, 35, 40 and 45 run across it, and no
        // line lies outside it.
        let lines = hatch_of(&rect_poly(square()));
        let mut ys: Vec<f64> = lines.iter().map(|l| l[1]).collect();
        ys.sort_by(f64::total_cmp);
        ys.dedup();
        for y in [25.0, 30.0, 35.0, 40.0, 45.0] {
            assert!(ys.contains(&y), "a line at {y}: {ys:?}");
        }
        for l in &lines {
            assert_eq!(l[1], l[3], "horizontal");
            assert!(l[1] >= 20.0 && l[1] <= 50.0, "{l:?}");
            assert_eq!((l[0].min(l[2]), l[0].max(l[2])), (10.0, 50.0), "across the whole width: {l:?}");
        }
        assert_eq!(lines.len(), ys.len(), "one segment per line");
        // A triangle with its apex at (30, 20) and its base y = 50 from x 10 to 50: each line is
        // cut to the triangle, so it is as wide as the triangle at its height.
        let tri = [(10.0, 50.0), (50.0, 50.0), (30.0, 20.0)];
        let lines = hatch_of(&tri);
        assert!(lines.len() >= 5, "{lines:?}");
        for l in &lines {
            let half = (l[1] - 20.0) / 30.0 * 20.0;
            let (lo, hi) = (l[0].min(l[2]), l[0].max(l[2]));
            assert!((lo - (30.0 - half)).abs() < 2e-3 && (hi - (30.0 + half)).abs() < 2e-3, "{l:?}");
        }
        assert!(lines.iter().any(|l| l[1] == 25.0 && l[0].min(l[2]) > 26.0), "short near the apex");
    }

    #[test]
    fn a_gradient_is_a_run_of_bands_from_the_fill_to_the_end_colour() {
        let style = FillStyle {
            gradient: Some(Gradient { color: Color::BLUE, direction: GradientDirection::LeftToRight }),
            ..FillStyle::default()
        };
        let mut out = Vec::new();
        fill_polygon(&mut out, &rect_poly(square()), Color::RED, &style);
        assert!(out.len() >= MIN_BANDS);
        let fills: Vec<Color> = out
            .iter()
            .filter_map(|i| match i {
                Item::Path { fill, .. } => *fill,
                _ => None,
            })
            .collect();
        assert!(fills[0].r > fills[0].b, "starts red: {:?}", fills[0]);
        let last = fills.last().unwrap();
        assert!(last.b > last.r, "ends blue: {last:?}");
        // The reverse direction starts with the fill colour at the right edge.
        let style = FillStyle {
            gradient: Some(Gradient { color: Color::BLUE, direction: GradientDirection::RightToLeft }),
            ..FillStyle::default()
        };
        let mut rev = Vec::new();
        fill_polygon(&mut rev, &rect_poly(square()), Color::RED, &style);
        let Item::Path { d: first, fill: Some(c), .. } = &rev[0] else { panic!() };
        assert!(c.r > c.b, "the first band holds the fill colour");
        assert!(first.starts_with("M50") || first.contains("50 "), "and lies at the right edge: {first}");
    }

    #[test]
    fn bands_together_cover_the_shape() {
        let poly = rect_poly(square());
        let g = Gradient { color: Color::BLUE, direction: GradientDirection::TopToBottom };
        let pieces = bands(&poly, Color::RED, &g, false);
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for (p, _) in &pieces {
            for q in p {
                lo = lo.min(q.1);
                hi = hi.max(q.1);
            }
        }
        assert!((lo - 20.0).abs() < 1e-9 && (hi - 50.0).abs() < 1e-9);
    }

    #[test]
    fn a_translucent_gradient_keeps_its_alpha_and_its_bands_meet_edge_to_edge() {
        let gradient = Gradient { color: Color::WHITE, direction: GradientDirection::TopToBottom };
        let style = FillStyle { transparency_pct: 40, gradient: Some(gradient), ..FillStyle::default() };
        let mut out = Vec::new();
        fill_polygon(&mut out, &rect_poly(square()), Color::BLACK, &style);
        assert!(out.iter().all(|i| matches!(i, Item::Path { fill: Some(c), .. } if c.a == 153)));
        // Every band starts where the one before it ends: a strip painted twice would be darker.
        let spans: Vec<(f64, f64)> = out
            .iter()
            .map(|i| {
                let Item::Path { d, .. } = i else { panic!("{i:?}") };
                let ys = d
                    .trim_end_matches('Z')
                    .split(['M', 'L'])
                    .skip(1)
                    .map(|p| -> f64 { p.split_whitespace().nth(1).unwrap().parse().unwrap() });
                (ys.clone().fold(f64::INFINITY, f64::min), ys.fold(f64::NEG_INFINITY, f64::max))
            })
            .collect();
        assert!(spans.len() >= MIN_BANDS);
        assert!(spans.windows(2).all(|w| (w[1].0 - w[0].1).abs() < 1e-9), "{spans:?}");
    }

    #[test]
    fn the_bands_of_an_opaque_gradient_overlap_and_those_of_a_translucent_one_do_not() {
        let poly = rect_poly(square());
        let g = Gradient { color: Color::BLUE, direction: GradientDirection::TopToBottom };
        let spans = |translucent: bool| -> Vec<(f64, f64)> {
            bands(&poly, Color::RED, &g, translucent)
                .iter()
                .map(|(p, _)| {
                    let ys = p.iter().map(|q| q.1);
                    (ys.clone().fold(f64::INFINITY, f64::min), ys.fold(f64::NEG_INFINITY, f64::max))
                })
                .collect()
        };
        let opaque = spans(false);
        assert!(opaque.windows(2).all(|w| w[1].0 < w[0].1), "each band reaches over the next: {opaque:?}");
        let clear = spans(true);
        assert!(clear.len() >= MIN_BANDS);
        assert!(clear.windows(2).all(|w| (w[1].0 - w[0].1).abs() < 1e-9), "edge to edge: {clear:?}");
    }
}
