//! What the PostScript and metafile writers share: the scene's SVG path data as plain segments,
//! the blending that stands in for transparency, and the mapping from text to standard fonts.

use crate::scene::Item;
use sheaf_core::Color;

/// One drawing step of a path, in page points (y grows downwards).
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum Seg {
    Move(f64, f64),
    Line(f64, f64),
    /// Two control points and the end point.
    Cubic([f64; 6]),
    Close,
}

/// Parses the path data this crate writes (`M`, `L`, `C`, `A`, `Z`, absolute) into segments.
/// Arcs become cubic curves. Anything else ends the parse.
pub(crate) fn parse_path(d: &str) -> Vec<Seg> {
    let mut out = Vec::new();
    let mut cmd = 'M';
    let mut nums: Vec<f64> = Vec::new();
    let mut token = String::new();
    let mut cur = (0.0, 0.0);
    let mut start = (0.0, 0.0);
    let take = |token: &mut String, nums: &mut Vec<f64>| {
        if !token.is_empty() {
            if let Ok(v) = token.parse() {
                nums.push(v);
            }
            token.clear();
        }
    };
    let flush = |cmd: char, nums: &mut Vec<f64>, out: &mut Vec<Seg>, cur: &mut (f64, f64), start: &mut (f64, f64)| {
        let per = match cmd {
            'M' | 'L' => 2,
            'C' => 6,
            'A' => 7,
            _ => 0,
        };
        if per > 0 {
            for (k, s) in nums.chunks_exact(per).enumerate() {
                match cmd {
                    'M' if k == 0 => {
                        out.push(Seg::Move(s[0], s[1]));
                        *cur = (s[0], s[1]);
                        *start = *cur;
                    }
                    'M' | 'L' => {
                        out.push(Seg::Line(s[0], s[1]));
                        *cur = (s[0], s[1]);
                    }
                    'C' => {
                        out.push(Seg::Cubic([s[0], s[1], s[2], s[3], s[4], s[5]]));
                        *cur = (s[4], s[5]);
                    }
                    _ => {
                        arc_to_cubics(*cur, s[0], s[1], s[2], s[3] != 0.0, s[4] != 0.0, (s[5], s[6]), out);
                        *cur = (s[5], s[6]);
                    }
                }
            }
        }
        nums.clear();
    };
    for c in d.chars() {
        match c {
            'M' | 'L' | 'C' | 'A' | 'Z' => {
                take(&mut token, &mut nums);
                flush(cmd, &mut nums, &mut out, &mut cur, &mut start);
                if c == 'Z' {
                    out.push(Seg::Close);
                    cur = start;
                }
                cmd = c;
            }
            ' ' | ',' => take(&mut token, &mut nums),
            '-' if !token.is_empty() && !token.ends_with('e') => {
                take(&mut token, &mut nums);
                token.push(c);
            }
            _ => token.push(c),
        }
    }
    take(&mut token, &mut nums);
    flush(cmd, &mut nums, &mut out, &mut cur, &mut start);
    out
}

/// Appends the cubic curves of an SVG elliptical arc from `from` (implementation notes of the
/// SVG specification, appendix B.2.4 and B.2.5).
#[allow(clippy::too_many_arguments)]
fn arc_to_cubics(
    from: (f64, f64),
    rx: f64,
    ry: f64,
    rot_deg: f64,
    large: bool,
    sweep: bool,
    to: (f64, f64),
    out: &mut Vec<Seg>,
) {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if from == to {
        return;
    }
    if rx == 0.0 || ry == 0.0 {
        out.push(Seg::Line(to.0, to.1));
        return;
    }
    let phi = rot_deg.to_radians();
    let (sp, cp) = phi.sin_cos();
    let (dx, dy) = ((from.0 - to.0) / 2.0, (from.1 - to.1) / 2.0);
    let (x1, y1) = (cp * dx + sp * dy, -sp * dx + cp * dy);
    let lambda = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }
    let num = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut coef = if den == 0.0 { 0.0 } else { (num / den).max(0.0).sqrt() };
    if large == sweep {
        coef = -coef;
    }
    let (cxp, cyp) = (coef * rx * y1 / ry, -coef * ry * x1 / rx);
    let cx = cp * cxp - sp * cyp + (from.0 + to.0) / 2.0;
    let cy = sp * cxp + cp * cyp + (from.1 + to.1) / 2.0;
    let angle = |ux: f64, uy: f64, vx: f64, vy: f64| (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
    let theta1 = angle(1.0, 0.0, (x1 - cxp) / rx, (y1 - cyp) / ry);
    let mut delta = angle((x1 - cxp) / rx, (y1 - cyp) / ry, (-x1 - cxp) / rx, (-y1 - cyp) / ry);
    if !sweep && delta > 0.0 {
        delta -= std::f64::consts::TAU;
    } else if sweep && delta < 0.0 {
        delta += std::f64::consts::TAU;
    }
    let n = (delta.abs() / std::f64::consts::FRAC_PI_2).ceil().max(1.0) as usize;
    let step = delta / n as f64;
    let t = 4.0 / 3.0 * (step / 4.0).tan();
    let point = |a: f64| {
        let (sa, ca) = a.sin_cos();
        (cx + rx * ca * cp - ry * sa * sp, cy + rx * ca * sp + ry * sa * cp)
    };
    let deriv = |a: f64| {
        let (sa, ca) = a.sin_cos();
        (-rx * sa * cp - ry * ca * sp, -rx * sa * sp + ry * ca * cp)
    };
    for k in 0..n {
        let a0 = theta1 + step * k as f64;
        let a1 = a0 + step;
        let (p0, p1) = (point(a0), if k + 1 == n { to } else { point(a1) });
        let (d0, d1) = (deriv(a0), deriv(a1));
        out.push(Seg::Cubic([p0.0 + t * d0.0, p0.1 + t * d0.1, p1.0 - t * d1.0, p1.1 - t * d1.1, p1.0, p1.1]));
    }
}

/// `color` as it looks on `under`: neither PostScript nor the metafile format has transparency,
/// so a translucent colour is blended onto the colour beneath it. The writers blend onto the
/// page background; where two translucent shapes overlap, the result is therefore not what the
/// SVG and PDF show.
pub(crate) fn flatten(color: Color, under: Color) -> Color {
    if color.a == 255 {
        return color;
    }
    let under = if under.a == 0 { Color::WHITE } else { Color { a: 255, ..under } };
    let t = color.opacity();
    let mix = |c: u8, u: u8| (f64::from(u) + (f64::from(c) - f64::from(u)) * t).round() as u8;
    Color::rgb(mix(color.r, under.r), mix(color.g, under.g), mix(color.b, under.b))
}

/// The segments of a drawable item (a rectangle becomes four lines and a close); `None` for text.
pub(crate) fn item_segs(item: &Item) -> Option<Vec<Seg>> {
    match item {
        Item::Rect { rect, .. } => Some(vec![
            Seg::Move(rect.x, rect.y),
            Seg::Line(rect.x + rect.w, rect.y),
            Seg::Line(rect.x + rect.w, rect.y + rect.h),
            Seg::Line(rect.x, rect.y + rect.h),
            Seg::Close,
        ]),
        Item::Path { d, .. } => Some(parse_path(d)),
        Item::Text(_) | Item::Image { .. } => None,
    }
}

/// The three families of the standard PostScript fonts.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Family {
    Helvetica,
    Times,
    Courier,
}

/// Maps the family a graph asks for to the nearest standard family: serif faces to Times,
/// monospaced ones to Courier, everything else to Helvetica.
pub(crate) fn standard_family(name: &str) -> Family {
    let n = name.to_ascii_lowercase();
    if ["courier", "consolas", "mono", "lucida console", "menlo", "fixed"].iter().any(|k| n.contains(k)) {
        Family::Courier
    } else if ["times", "serif", "georgia", "garamond", "palatino", "cambria", "book", "minion", "century"]
        .iter()
        .any(|k| n.contains(k))
        && !n.contains("sans")
    {
        Family::Times
    } else {
        Family::Helvetica
    }
}

/// The PostScript name of a standard font.
pub(crate) fn ps_font_name(f: Family, bold: bool, italic: bool) -> &'static str {
    match (f, bold, italic) {
        (Family::Helvetica, false, false) => "Helvetica",
        (Family::Helvetica, true, false) => "Helvetica-Bold",
        (Family::Helvetica, false, true) => "Helvetica-Oblique",
        (Family::Helvetica, true, true) => "Helvetica-BoldOblique",
        (Family::Times, false, false) => "Times-Roman",
        (Family::Times, true, false) => "Times-Bold",
        (Family::Times, false, true) => "Times-Italic",
        (Family::Times, true, true) => "Times-BoldItalic",
        (Family::Courier, false, false) => "Courier",
        (Family::Courier, true, false) => "Courier-Bold",
        (Family::Courier, false, true) => "Courier-Oblique",
        (Family::Courier, true, true) => "Courier-BoldOblique",
    }
}

/// The byte of the PostScript `Symbol` font that draws `c`, for the Greek letters and the
/// mathematical signs that Latin-1 lacks.
pub(crate) fn symbol_byte(c: char) -> Option<u8> {
    // The letters in the order of the Symbol encoding, A to Z and a to z.
    const UPPER: &str = "ΑΒΧΔΕΦΓΗΙϑΚΛΜΝΟΠΘΡΣΤΥςΩΞΨΖ";
    const LOWER: &str = "αβχδεφγηιϕκλμνοπθρστυϖωξψζ";
    if let Some(i) = UPPER.chars().position(|u| u == c) {
        return Some(b'A' + i as u8);
    }
    if let Some(i) = LOWER.chars().position(|u| u == c) {
        return Some(b'a' + i as u8);
    }
    Some(match c {
        '\u{2212}' => 0x2D,
        '\u{221E}' => 0xA5,
        '\u{2264}' => 0xA3,
        '\u{2265}' => 0xB3,
        '\u{2260}' => 0xB9,
        '\u{2248}' => 0xBB,
        '\u{2202}' => 0xB6,
        '\u{221A}' => 0xD6,
        '\u{2211}' => 0xE5,
        '\u{220F}' => 0xD5,
        '\u{222B}' => 0xF2,
        '\u{2192}' => 0xAE,
        '\u{2190}' => 0xAC,
        '\u{2191}' => 0xAD,
        '\u{2193}' => 0xAF,
        '\u{2022}' => 0xB7,
        '\u{2206}' => 0x44,
        '\u{2207}' => 0xD1,
        '\u{221D}' => 0xB5,
        '\u{2261}' => 0xBA,
        _ => return None,
    })
}

/// The nearest Latin-1 character for one that neither Latin-1 nor the Symbol font has, or `None`
/// when there is no sensible stand-in.
pub(crate) fn latin1_stand_in(c: char) -> Option<char> {
    Some(match c {
        '\u{2018}' | '\u{2019}' | '\u{201A}' => '\'',
        '\u{201C}' | '\u{201D}' | '\u{201E}' => '"',
        '\u{2013}' | '\u{2014}' | '\u{2010}' | '\u{2011}' => '-',
        '\u{2026}' => '.',
        '\u{2009}' | '\u{200A}' | '\u{202F}' | '\u{2002}' | '\u{2003}' => ' ',
        '\u{2122}' => 'T',
        '\u{20AC}' => 'E',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arcs_become_cubics_ending_where_the_arc_does() {
        let segs = parse_path("M0 5A5 5 0 1 0 10 5A5 5 0 1 0 0 5Z");
        assert_eq!(segs[0], Seg::Move(0.0, 5.0));
        assert_eq!(segs.iter().filter(|s| matches!(s, Seg::Cubic(_))).count(), 4);
        assert_eq!(*segs.last().unwrap(), Seg::Close);
        // Every curve's end point is on the circle of radius 5 about (5, 5).
        for s in &segs {
            if let Seg::Cubic(c) = s {
                assert!(((c[4] - 5.0).hypot(c[5] - 5.0) - 5.0).abs() < 1e-9, "{c:?}");
            }
        }
        let ends: Vec<_> =
            segs.iter().filter_map(|s| if let Seg::Cubic(c) = s { Some((c[4], c[5])) } else { None }).collect();
        assert!((ends[1].0 - 10.0).abs() < 1e-9 && (ends[1].1 - 5.0).abs() < 1e-9, "{ends:?}");
        assert!(ends[3].0.abs() < 1e-9 && (ends[3].1 - 5.0).abs() < 1e-9, "{ends:?}");
    }

    #[test]
    fn translucent_colours_are_blended_onto_the_background() {
        let c = flatten(Color::rgba(255, 0, 0, 128), Color::WHITE);
        assert_eq!((c.r, c.g, c.b, c.a), (255, 127, 127, 255));
        assert_eq!(flatten(Color::BLACK, Color::WHITE), Color::BLACK);
        let c = flatten(Color::rgba(0, 0, 0, 127), Color::TRANSPARENT);
        assert_eq!((c.r, c.a), (128, 255));
    }

    #[test]
    fn families_map_to_the_standard_fonts() {
        assert_eq!(standard_family("Arial"), Family::Helvetica);
        assert_eq!(standard_family("Times New Roman"), Family::Times);
        assert_eq!(standard_family("Courier New"), Family::Courier);
        assert_eq!(standard_family("DejaVu Sans Mono"), Family::Courier);
        assert_eq!(standard_family("DejaVu Sans"), Family::Helvetica);
        assert_eq!(ps_font_name(Family::Times, true, true), "Times-BoldItalic");
        assert_eq!(symbol_byte('α'), Some(b'a'));
        assert_eq!(symbol_byte('Ω'), Some(b'W'));
    }
}
