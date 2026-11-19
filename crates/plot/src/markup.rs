//! Text markup of labels (spec section 5): `\b(..)` bold, `\i(..)` italic,
//! `\+(..)` superscript, `\-(..)` subscript, `\g(..)` Greek, `\pN(..)` size in percent,
//! `\f:Name(..)` font switch, `\x(hhhh)` a character by its code, `\v(..)` literal text,
//! `\h(..)` protected substitution text, `\u(..)` underline, `\s(..)` strikethrough
//! and `\cN(..)` indexed or RGB text color.
//! `\=(sub,sup)` places paired scripts in one shared column. `\ab(..)`, `\ad(..)`, and
//! `\at(..)` draw a bar, dot, or tilde above a span.
//! Escapes nest. A backslash
//! that does not start one of these escapes is literal. The two prime escapes insert
//! Unicode prime characters through the same styled glyph path.

use crate::fonts::{SystemFonts, TextMeasurer};
use sheaf_core::Color;

/// Scale of superscript and subscript text relative to its surroundings.
pub const SCRIPT_SCALE: f64 = 0.65;
/// Superscripts are raised by this fraction of the surrounding size.
pub const SUP_RISE: f64 = 0.35;
/// Subscripts are lowered by this fraction of the surrounding size.
pub const SUB_DROP: f64 = 0.15;

/// A stretch of text with one style.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    /// The characters (Greek already mapped to real Greek letters).
    pub text: String,
    /// Bold on top of the label's own font.
    pub bold: bool,
    /// Italic on top of the label's own font.
    pub italic: bool,
    /// Draw a line below this run.
    pub underline: bool,
    /// Draw a line through this run.
    pub strikeout: bool,
    /// An explicit palette or RGB colour, or the label's own colour when absent.
    pub color: Option<Color>,
    /// An inline font family, or the label's own family when absent.
    pub family: Option<String>,
    /// Size relative to the label's font size (1 for normal text).
    pub scale: f64,
    /// Baseline shift in units of the label's font size; positive is up.
    pub rise: f64,
}

#[derive(Clone)]
struct Style {
    bold: bool,
    italic: bool,
    underline: bool,
    strikeout: bool,
    color: Option<Color>,
    family: Option<String>,
    greek: bool,
    scale: f64,
    rise: f64,
}

/// Deepest escape nesting that is honoured; deeper text is taken literally,
/// so hostile input cannot overflow the stack.
const MAX_DEPTH: usize = 24;

/// A layout segment. Script branches have absolute run styles but share one horizontal column.
#[derive(Clone, Debug)]
pub(crate) enum Segment {
    Text(Run),
    Pair { sub: Vec<Segment>, sup: Vec<Segment> },
    Accent { kind: Accent, content: Vec<Segment>, scale: f64, color: Option<Color> },
    LineGap(u32),
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Accent {
    Bar,
    Dot,
    Tilde,
}

/// Splits label text into styled runs, with paired scripts in logical sub/sup order.
/// Use the internal layout tree when measuring or placing shared script columns.
pub fn parse(text: &str) -> Vec<Run> {
    fn flatten(segments: Vec<Segment>, out: &mut Vec<Run>) {
        for segment in segments {
            match segment {
                Segment::Text(run) => out.push(run),
                Segment::Pair { sub, sup } => {
                    flatten(sub, out);
                    flatten(sup, out);
                }
                Segment::Accent { content, .. } => flatten(content, out),
                Segment::LineGap(_) => {}
            }
        }
    }
    let mut out = Vec::new();
    flatten(segments(text, &|name| SystemFonts::shared().expand_inline_family(name)), &mut out);
    out
}

pub(crate) fn segments(text: &str, expand_family: &impl Fn(&str) -> Option<String>) -> Vec<Segment> {
    parse_segments(text, &ParseOptions { expand_family, label_separators: false, plot_color: None })
}

/// Parses a free label with normalized line endings and access to its layer's plot colors.
pub(crate) fn label_segments(
    text: &str,
    expand_family: &impl Fn(&str) -> Option<String>,
    plot_color: &impl Fn(usize) -> Option<Color>,
) -> Vec<Segment> {
    parse_segments(
        text,
        &ParseOptions { expand_family, label_separators: text.contains('\n'), plot_color: Some(plot_color) },
    )
}

/// Parses a legend cell with access to the legend owner's plot colors.
pub(crate) fn legend_segments(
    text: &str,
    expand_family: &impl Fn(&str) -> Option<String>,
    plot_color: &impl Fn(usize) -> Option<Color>,
) -> Vec<Segment> {
    parse_segments(text, &ParseOptions { expand_family, label_separators: false, plot_color: Some(plot_color) })
}

struct ParseOptions<'a> {
    expand_family: &'a dyn Fn(&str) -> Option<String>,
    label_separators: bool,
    plot_color: Option<&'a dyn Fn(usize) -> Option<Color>>,
}

fn parse_segments(text: &str, options: &ParseOptions<'_>) -> Vec<Segment> {
    let chars: Vec<char> = text.chars().collect();
    let mut pos = 0;
    let mut out = Vec::new();
    let base = Style {
        bold: false,
        italic: false,
        underline: false,
        strikeout: false,
        color: None,
        family: None,
        greek: false,
        scale: 1.0,
        rise: 0.0,
    };
    parse_seq(&chars, &mut pos, base, false, 0, &mut out, options);
    out
}

/// The text without any markup.
pub fn plain(text: &str) -> String {
    parse(text).into_iter().map(|r| r.text).collect()
}

fn push(out: &mut Vec<Segment>, c: char, st: &Style) {
    let c = if st.greek { greek(c) } else { c };
    if let Some(Segment::Text(last)) = out.last_mut()
        && last.bold == st.bold
        && last.italic == st.italic
        && last.underline == st.underline
        && last.strikeout == st.strikeout
        && last.color == st.color
        && last.family == st.family
        && last.scale == st.scale
        && last.rise == st.rise
    {
        last.text.push(c);
        return;
    }
    out.push(Segment::Text(Run {
        text: c.to_string(),
        bold: st.bold,
        italic: st.italic,
        underline: st.underline,
        strikeout: st.strikeout,
        color: st.color,
        family: st.family.clone(),
        scale: st.scale,
        rise: st.rise,
    }));
}

/// What a recognised escape does with the text in its parentheses.
enum Escape {
    /// The text is parsed further with this style.
    Styled(Style),
    /// A shared column containing subscript, then superscript, separated by the first outer comma.
    Pair,
    /// A decorated span with one accent over its contents.
    Accent(Accent),
    /// `\v(..)` or `\h(..)`: the enclosed text is taken literally.
    Literal,
    /// `\x(hhhh)`: one character given by its hexadecimal code.
    Unicode,
    /// `\(nnn)`: one character given by its decimal code, for example `\(40)` for `(`.
    Decimal,
    /// One of the two exact prime escapes, emitted without Greek remapping.
    Prime(char),
}

/// Longest font name accepted in `\f:Name(..)`; longer text is not an escape.
const MAX_FONT_NAME: usize = 64;

/// Recognises an escape starting at the backslash at `pos`. Returns what it does and the index of
/// the first character inside its parentheses.
fn escape_at(chars: &[char], pos: usize, st: &Style, options: &ParseOptions<'_>) -> Option<(Escape, usize)> {
    let mut inner = st.clone();
    let code = *chars.get(pos + 1)?;
    let accent = match chars.get(pos + 1..pos + 4) {
        Some(['a', 'b', '(']) => Some(Accent::Bar),
        Some(['a', 'd', '(']) => Some(Accent::Dot),
        Some(['a', 't', '(']) => Some(Accent::Tilde),
        _ => None,
    };
    if let Some(kind) = accent {
        return Some((Escape::Accent(kind), pos + 4));
    }
    if code == '(' {
        return Some((Escape::Decimal, pos + 2));
    }
    if chars.get(pos + 2) == Some(&'(') {
        let esc = match code {
            'b' => {
                inner.bold = true;
                Escape::Styled(inner)
            }
            'i' => {
                inner.italic = true;
                Escape::Styled(inner)
            }
            'g' => {
                inner.greek = true;
                Escape::Styled(inner)
            }
            '+' => {
                inner.rise = st.rise + SUP_RISE * st.scale;
                inner.scale = st.scale * SCRIPT_SCALE;
                Escape::Styled(inner)
            }
            '-' => {
                inner.rise = st.rise - SUB_DROP * st.scale;
                inner.scale = st.scale * SCRIPT_SCALE;
                Escape::Styled(inner)
            }
            'u' => {
                inner.underline = true;
                Escape::Styled(inner)
            }
            's' => {
                inner.strikeout = true;
                Escape::Styled(inner)
            }
            '=' => Escape::Pair,
            'v' | 'h' => Escape::Literal,
            'x' => Escape::Unicode,
            'y' if chars.get(pos + 4) == Some(&')') => match chars.get(pos + 3) {
                Some('\'') => Escape::Prime('\u{2032}'),
                Some('"') => Escape::Prime('\u{2033}'),
                _ => return None,
            },
            _ => return None,
        };
        return Some((esc, pos + 3));
    }
    match code {
        // `\f:Name(..)`: select this family for the span. A
        // switch away from the Symbol font ends Greek mapping, and a switch to it starts it.
        'f' if chars.get(pos + 2) == Some(&':') => {
            let start = pos + 3;
            let open = (start..chars.len().min(start + MAX_FONT_NAME + 1)).find(|&i| chars[i] == '(')?;
            let name: String = chars[start..open].iter().collect();
            // Origin 2016 renders an empty-name escape literally (graphing 7.6).
            if name.is_empty() || name.contains(['\\', ')']) {
                return None;
            }
            inner.greek = name.trim().eq_ignore_ascii_case("symbol");
            if name.trim() == name {
                let name = (options.expand_family)(&name).unwrap_or(name);
                inner.greek = name.eq_ignore_ascii_case("symbol");
                inner.family = Some(name);
            }
            Some((Escape::Styled(inner), open + 1))
        }
        // Plot references are one-based. Missing slots keep the enclosing style.
        'c' if chars.get(pos + 2) == Some(&'p') => {
            let plot_color = options.plot_color?;
            let start = pos + 3;
            let digits = chars[start..].iter().take_while(|c| c.is_ascii_digit()).count();
            let open = start + digits;
            if digits == 0 || digits > 10 || chars.get(open) != Some(&'(') {
                return None;
            }
            let index = chars[start..open].iter().collect::<String>().parse::<u32>().ok()? as usize;
            inner.color = index.checked_sub(1).and_then(plot_color).or(st.color);
            Some((Escape::Styled(inner), open + 1))
        }
        // Size percentages retain their four-digit limit; colour codes use a bounded u32.
        'p' | 'c' => {
            let digits = chars[pos + 2..].iter().take_while(|c| c.is_ascii_digit()).count();
            let open = pos + 2 + digits;
            let limit = if code == 'p' { 4 } else { 10 };
            if digits == 0 || digits > limit || chars.get(open) != Some(&'(') {
                return None;
            }
            if code == 'p' {
                let percent: f64 = chars[pos + 2..open].iter().collect::<String>().parse().ok()?;
                if percent > 0.0 {
                    inner.scale = st.scale * percent / 100.0;
                }
            } else {
                let index = chars[pos + 2..open].iter().collect::<String>().parse::<u32>().ok()?;
                // Preserve the existing text and inherited style for zero codes. Origin's
                // invalid zero escape discards its contents; supported indices are positive.
                if index > 0 {
                    inner.color = Some(match index {
                        1..=0x0100_0000 => text_palette_color(index),
                        0x0100_0001..=0x0200_0000 => {
                            // Text escapes are one-based, including RGB-tagged values.
                            // Color(r,g,b) returns 0x01BBGGRR; add one for the same RGB
                            // in a text escape (Origin 2016 vector controls, graphing 7.3).
                            let [r, g, b, _] = (index - 1).to_le_bytes();
                            Color::rgb(r, g, b)
                        }
                        _ => return None,
                    });
                }
            }
            Some((Escape::Styled(inner), open + 1))
        }
        _ => None,
    }
}

/// Origin's original text palette. The low byte selects a one-based colour; unsupported
/// byte values give black (Origin 2016 EMF fixtures in scratch/text-run-colors).
fn text_palette_color(index: u32) -> Color {
    const CLASSIC: [Color; 24] = [
        Color::BLACK,
        Color::RED,
        Color::rgb(0, 255, 0),
        Color::BLUE,
        Color::rgb(0, 255, 255),
        Color::rgb(255, 0, 255),
        Color::rgb(255, 255, 0),
        Color::rgb(128, 128, 0),
        Color::rgb(0, 0, 128),
        Color::rgb(128, 0, 128),
        Color::rgb(128, 0, 0),
        Color::rgb(0, 128, 0),
        Color::rgb(0, 128, 128),
        Color::rgb(0, 0, 160),
        Color::rgb(255, 128, 0),
        Color::rgb(128, 0, 255),
        Color::rgb(255, 0, 128),
        Color::WHITE,
        Color::rgb(192, 192, 192),
        Color::rgb(128, 128, 128),
        Color::rgb(255, 255, 128),
        Color::rgb(128, 255, 255),
        Color::rgb(255, 128, 255),
        Color::rgb(64, 64, 64),
    ];
    let index = (index & 255) as usize;
    index.checked_sub(1).and_then(|i| CLASSIC.get(i)).copied().unwrap_or(Color::BLACK)
}

/// Copies text up to the `)` that matches an already opened `(`, balancing inner parentheses,
/// and moves `pos` past it.
fn take_enclosed(chars: &[char], pos: &mut usize) -> String {
    let mut depth = 0usize;
    let mut text = String::new();
    while let Some(&c) = chars.get(*pos) {
        *pos += 1;
        match c {
            ')' if depth == 0 => break,
            ')' => depth -= 1,
            '(' => depth += 1,
            _ => {}
        }
        text.push(c);
    }
    text
}

fn parse_seq(
    chars: &[char],
    pos: &mut usize,
    st: Style,
    in_escape: bool,
    depth: usize,
    out: &mut Vec<Segment>,
    options: &ParseOptions<'_>,
) {
    let mut parens = 0usize;
    while *pos < chars.len() {
        let c = chars[*pos];
        if options.label_separators && depth == 0 && (*pos == 0 || chars[*pos - 1] == '\n') {
            let line: String = chars[*pos..].iter().take_while(|&&c| c != '\n').collect();
            if let Some(percent) = crate::legend::separation_percent(&line) {
                out.push(Segment::LineGap(percent));
                // A valid separator contains only ASCII syntax and digits, plus optional
                // whitespace. Count characters because the parser indexes Unicode scalars.
                *pos += line.chars().count();
                continue;
            }
        }
        if c == '\\'
            && depth < MAX_DEPTH
            && let Some((esc, start)) = escape_at(chars, *pos, &st, options)
        {
            *pos = start;
            match esc {
                Escape::Styled(inner) => parse_seq(chars, pos, inner, true, depth + 1, out, options),
                Escape::Accent(kind) => {
                    let mut content = Vec::new();
                    parse_seq(chars, pos, st.clone(), true, depth + 1, &mut content, options);
                    if !content.is_empty() {
                        out.push(Segment::Accent { kind, content, scale: st.scale, color: st.color });
                    }
                }
                Escape::Pair => {
                    let body: Vec<char> = take_enclosed(chars, pos).chars().collect();
                    let mut nesting = 0usize;
                    let comma = body.iter().position(|&c| match c {
                        '(' => {
                            nesting += 1;
                            false
                        }
                        ')' => {
                            nesting = nesting.saturating_sub(1);
                            false
                        }
                        ',' => nesting == 0,
                        _ => false,
                    });
                    let (lower, upper) =
                        comma.map_or((&body[..], &body[body.len()..]), |at| (&body[..at], &body[at + 1..]));
                    // Origin 2016 Arial vector controls: 60% size, baseline -49%/+35%.
                    // Keep these pair measurements separate from the older standalone scripts.
                    let branch = |chars: &[char], rise: f64| {
                        let inner = Style { scale: st.scale * 0.6, rise: st.rise + rise * st.scale, ..st.clone() };
                        let mut out = Vec::new();
                        parse_seq(chars, &mut 0, inner, false, depth + 1, &mut out, options);
                        out
                    };
                    let sub = branch(lower, -0.35);
                    let sup = branch(upper, 0.49);
                    if !sub.is_empty() || !sup.is_empty() {
                        out.push(Segment::Pair { sub, sup });
                    }
                }
                Escape::Literal => {
                    let plain = Style { greek: false, ..st.clone() };
                    take_enclosed(chars, pos).chars().for_each(|c| push(out, c, &plain));
                }
                Escape::Unicode => {
                    let code = take_enclosed(chars, pos);
                    let ch =
                        u32::from_str_radix(code.trim(), 16).ok().and_then(char::from_u32).filter(|c| !c.is_control());
                    match ch {
                        Some(ch) => push(out, ch, &Style { greek: false, ..st.clone() }),
                        None => format!("\\x({code})").chars().for_each(|c| push(out, c, &st)),
                    }
                }
                Escape::Prime(ch) => {
                    take_enclosed(chars, pos);
                    push(out, ch, &Style { greek: false, ..st.clone() });
                }
                Escape::Decimal => {
                    let code = take_enclosed(chars, pos);
                    let ch = code.trim().parse::<u32>().ok().and_then(char::from_u32).filter(|c| !c.is_control());
                    match ch {
                        Some(ch) => push(out, ch, &Style { greek: false, ..st.clone() }),
                        None => format!("\\({code})").chars().for_each(|c| push(out, c, &st)),
                    }
                }
            }
            continue;
        }
        if in_escape {
            if c == ')' && parens == 0 {
                *pos += 1;
                return;
            }
            if c == '(' {
                parens += 1;
            } else if c == ')' {
                parens -= 1;
            }
        }
        push(out, c, &st);
        *pos += 1;
    }
}

/// Maps a Latin letter to the Greek letter at the same place in the Symbol
/// font layout. Other characters are unchanged.
pub fn greek(c: char) -> char {
    match c {
        'a' => 'α',
        'b' => 'β',
        'c' => 'χ',
        'd' => 'δ',
        'e' => 'ε',
        'f' => 'φ',
        'g' => 'γ',
        'h' => 'η',
        'i' => 'ι',
        'j' => 'ϕ',
        'k' => 'κ',
        'l' => 'λ',
        'm' => 'μ',
        'n' => 'ν',
        'o' => 'ο',
        'p' => 'π',
        'q' => 'θ',
        'r' => 'ρ',
        's' => 'σ',
        't' => 'τ',
        'u' => 'υ',
        'v' => 'ϖ',
        'w' => 'ω',
        'x' => 'ξ',
        'y' => 'ψ',
        'z' => 'ζ',
        'A' => 'Α',
        'B' => 'Β',
        'C' => 'Χ',
        'D' => 'Δ',
        'E' => 'Ε',
        'F' => 'Φ',
        'G' => 'Γ',
        'H' => 'Η',
        'I' => 'Ι',
        'J' => 'ϑ',
        'K' => 'Κ',
        'L' => 'Λ',
        'M' => 'Μ',
        'N' => 'Ν',
        'O' => 'Ο',
        'P' => 'Π',
        'Q' => 'Θ',
        'R' => 'Ρ',
        'S' => 'Σ',
        'T' => 'Τ',
        'U' => 'Υ',
        'V' => 'ς',
        'W' => 'Ω',
        'X' => 'Ξ',
        'Y' => 'Ψ',
        'Z' => 'Ζ',
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_one_run() {
        let r = parse("Current (mA)");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].text, "Current (mA)");
        assert_eq!((r[0].scale, r[0].rise), (1.0, 0.0));
        assert!(parse("").is_empty());
    }

    #[test]
    fn superscript_and_subscript() {
        let r = parse("cm\\+(-2) H\\-(2)O");
        let texts: Vec<_> = r.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(texts, ["cm", "-2", " H", "2", "O"]);
        assert_eq!(r[1].scale, 0.65);
        assert!((r[1].rise - 0.35).abs() < 1e-12);
        assert!((r[3].rise + 0.15).abs() < 1e-12);
    }

    #[test]
    fn escapes_nest() {
        let r = parse("\\b(a\\i(b)c)");
        assert_eq!(r.len(), 3);
        assert!(r[0].bold && !r[0].italic);
        assert!(r[1].bold && r[1].italic);
        assert!(r[2].bold && !r[2].italic);
        // A script inside a script is smaller and higher still.
        let r = parse("x\\+(y\\+(z))");
        assert!((r[2].scale - 0.65 * 0.65).abs() < 1e-12);
        assert!((r[2].rise - (0.35 + 0.35 * 0.65)).abs() < 1e-12);
    }

    #[test]
    fn parentheses_inside_an_escape_balance() {
        let r = parse("\\b(f(x)) y");
        assert_eq!(r[0].text, "f(x)");
        assert!(r[0].bold);
        assert_eq!(r[1].text, " y");
    }

    #[test]
    fn backslash_is_literal_unless_it_starts_an_escape() {
        assert_eq!(plain("C:\\temp"), "C:\\temp");
        assert_eq!(plain("a\\b c"), "a\\b c");
        assert_eq!(plain("\\\\"), "\\\\");
        assert_eq!(plain("\\x(1)"), "\\x(1)");
        assert_eq!(plain("trailing\\"), "trailing\\");
    }

    #[test]
    fn protected_substitution_text_is_literal_with_the_surrounding_style() {
        let text = r"\b(\g(\h(\l(1) %(1) \i(raw))a))Z";
        let runs = parse(text);
        assert_eq!(runs[0].text, r"\l(1) %(1) \i(raw)α");
        assert!(runs[0].bold && !runs[0].italic);
        assert_eq!(runs[1].text, "Z");
        assert!(!runs[1].bold);
        assert_eq!(plain(r"\v(\h(%(1)))"), r"\h(%(1))");
    }

    #[test]
    fn unclosed_escape_runs_to_the_end() {
        let r = parse("a\\+(b");
        assert_eq!(r.len(), 2);
        assert_eq!(r[1].text, "b");
    }

    #[test]
    fn decimal_character_codes() {
        // An XRD title as Origin stored it: brackets written as character codes.
        assert_eq!(plain("2\\g(q) \\(40)degree\\(41)"), "2θ (degree)");
        assert_eq!(plain("\\(176)C"), "°C");
        assert!(parse("\\b(\\(65))").iter().all(|r| r.bold), "the character keeps the style around it");
        // A code that is not a printable character stays as written.
        assert_eq!(plain("\\(7)x"), "\\(7)x");
        assert_eq!(plain("\\(abc)"), "\\(abc)");
    }

    #[test]
    fn greek_mapping() {
        assert_eq!(plain("\\g(m)m"), "μm");
        assert_eq!(plain("\\g(D)E"), "ΔE");
        assert_eq!(plain("\\g(abgdpw)"), "αβγδπω");
        assert_eq!(plain("\\g(W) \\g(1)"), "Ω 1");
    }

    #[test]
    fn a_font_switch_inside_greek_ends_the_greek_letters() {
        // The X title of an XRD graph saved by Origin.
        assert_eq!(plain("\\b(2\\g(q \\f:Default: Arial((degree))))"), "2θ (degree)");
        assert!(parse("\\b(2\\g(q \\f:Default: Arial((degree))))").iter().all(|r| r.bold));
        assert_eq!(plain("\\f:Symbol(a)b"), "αb");
        assert_eq!(plain("\\f:Times New Roman(x)"), "x");
        // Not an escape: no parenthesis, or a parenthesis too far away.
        assert_eq!(plain("\\f:Arial"), "\\f:Arial");
        assert_eq!(plain(&format!("\\f:{}(x)", "n".repeat(100))), format!("\\f:{}(x)", "n".repeat(100)));
    }

    #[test]
    fn size_color_underline_literal_and_unicode_escapes() {
        let r = parse("a\\p200(b)");
        assert_eq!(r[1].text, "b");
        assert!((r[1].scale - 2.0).abs() < 1e-12);
        assert_eq!(plain("\\c4(red) \\u(under) \\s(gone)"), "red under gone");
        assert_eq!(plain("5\\v(\\b(x))Cu"), "5\\b(x)Cu");
        assert_eq!(plain("\\g(\\x(03b8))"), "θ");
        assert_eq!(plain("\\x(zz)"), "\\x(zz)");
        assert_eq!(plain("\\p(x) \\c(y) \\p12345(z)"), "\\p(x) \\c(y) \\p12345(z)");
    }

    #[test]
    fn deep_nesting_does_not_overflow() {
        let s = "\\b(".repeat(5000) + "x" + &")".repeat(5000);
        let r = parse(&s);
        assert!(!r.is_empty());
    }
}
