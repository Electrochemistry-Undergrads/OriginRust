//! Text markup of labels (spec section 5): `\b(..)` bold, `\i(..)` italic,
//! `\+(..)` superscript, `\-(..)` subscript, `\g(..)` Greek, `\pN(..)` size in percent,
//! `\f:Name(..)` font switch, `\x(hhhh)` a character by its code, `\v(..)` literal text; the
//! text of `\u(..)`, `\s(..)` and `\cN(..)` is kept without underline, strikethrough or color.
//! Escapes nest. A backslash that does not start one of these escapes is literal.

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
    /// Size relative to the label's font size (1 for normal text).
    pub scale: f64,
    /// Baseline shift in units of the label's font size; positive is up.
    pub rise: f64,
}

#[derive(Clone, Copy)]
struct Style {
    bold: bool,
    italic: bool,
    greek: bool,
    scale: f64,
    rise: f64,
}

/// Deepest escape nesting that is honoured; deeper text is taken literally,
/// so hostile input cannot overflow the stack.
const MAX_DEPTH: usize = 24;

/// Splits label text into styled runs. Empty text gives no runs.
pub fn parse(text: &str) -> Vec<Run> {
    let chars: Vec<char> = text.chars().collect();
    let mut pos = 0;
    let mut out = Vec::new();
    let base = Style { bold: false, italic: false, greek: false, scale: 1.0, rise: 0.0 };
    parse_seq(&chars, &mut pos, base, false, 0, &mut out);
    out
}

/// The text without any markup.
pub fn plain(text: &str) -> String {
    parse(text).into_iter().map(|r| r.text).collect()
}

fn push(out: &mut Vec<Run>, c: char, st: Style) {
    let c = if st.greek { greek(c) } else { c };
    if let Some(last) = out.last_mut()
        && last.bold == st.bold
        && last.italic == st.italic
        && last.scale == st.scale
        && last.rise == st.rise
    {
        last.text.push(c);
        return;
    }
    out.push(Run { text: c.to_string(), bold: st.bold, italic: st.italic, scale: st.scale, rise: st.rise });
}

/// What a recognised escape does with the text in its parentheses.
enum Escape {
    /// The text is parsed further with this style.
    Styled(Style),
    /// `\v(..)`: the text is taken as it is, without escapes.
    Literal,
    /// `\x(hhhh)`: one character given by its hexadecimal code.
    Unicode,
    /// `\(nnn)`: one character given by its decimal code, for example `\(40)` for `(`.
    Decimal,
}

/// Longest font name accepted in `\f:Name(..)`; longer text is not an escape.
const MAX_FONT_NAME: usize = 64;

/// Recognises an escape starting at the backslash at `pos`. Returns what it does and the index of
/// the first character inside its parentheses.
fn escape_at(chars: &[char], pos: usize, st: Style) -> Option<(Escape, usize)> {
    let mut inner = st;
    let code = *chars.get(pos + 1)?;
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
            // Underline and strikethrough are not drawn; their text is kept.
            'u' | 's' => Escape::Styled(inner),
            'v' => Escape::Literal,
            'x' => Escape::Unicode,
            _ => return None,
        };
        return Some((esc, pos + 3));
    }
    match code {
        // `\f:Name(..)`: the font itself is not switched (labels keep their own font), but a
        // switch away from the Symbol font ends Greek mapping, and a switch to it starts it.
        'f' if chars.get(pos + 2) == Some(&':') => {
            let start = pos + 3;
            let open = (start..chars.len().min(start + MAX_FONT_NAME + 1)).find(|&i| chars[i] == '(')?;
            let name: String = chars[start..open].iter().collect();
            if name.contains(['\\', ')']) {
                return None;
            }
            inner.greek = name.trim().eq_ignore_ascii_case("symbol");
            Some((Escape::Styled(inner), open + 1))
        }
        // `\pN(..)` size in percent of the surrounding text; `\cN(..)` color, which is not drawn.
        'p' | 'c' => {
            let digits = chars[pos + 2..].iter().take_while(|c| c.is_ascii_digit()).count();
            let open = pos + 2 + digits;
            if digits == 0 || digits > 4 || chars.get(open) != Some(&'(') {
                return None;
            }
            if code == 'p' {
                let percent: f64 = chars[pos + 2..open].iter().collect::<String>().parse().ok()?;
                if percent > 0.0 {
                    inner.scale = st.scale * percent / 100.0;
                }
            }
            Some((Escape::Styled(inner), open + 1))
        }
        _ => None,
    }
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

fn parse_seq(chars: &[char], pos: &mut usize, st: Style, in_escape: bool, depth: usize, out: &mut Vec<Run>) {
    let mut parens = 0usize;
    while *pos < chars.len() {
        let c = chars[*pos];
        if c == '\\'
            && depth < MAX_DEPTH
            && let Some((esc, start)) = escape_at(chars, *pos, st)
        {
            *pos = start;
            match esc {
                Escape::Styled(inner) => parse_seq(chars, pos, inner, true, depth + 1, out),
                Escape::Literal => {
                    let plain = Style { greek: false, ..st };
                    take_enclosed(chars, pos).chars().for_each(|c| push(out, c, plain));
                }
                Escape::Unicode => {
                    let code = take_enclosed(chars, pos);
                    let ch =
                        u32::from_str_radix(code.trim(), 16).ok().and_then(char::from_u32).filter(|c| !c.is_control());
                    match ch {
                        Some(ch) => push(out, ch, Style { greek: false, ..st }),
                        None => format!("\\x({code})").chars().for_each(|c| push(out, c, st)),
                    }
                }
                Escape::Decimal => {
                    let code = take_enclosed(chars, pos);
                    let ch = code.trim().parse::<u32>().ok().and_then(char::from_u32).filter(|c| !c.is_control());
                    match ch {
                        Some(ch) => push(out, ch, Style { greek: false, ..st }),
                        None => format!("\\({code})").chars().for_each(|c| push(out, c, st)),
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
        push(out, c, st);
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
