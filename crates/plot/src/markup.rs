//! Text markup of labels (spec section 5): `\b(..)` bold, `\i(..)` italic,
//! `\+(..)` superscript, `\-(..)` subscript, `\g(..)` Greek. Escapes nest.
//! A backslash that does not start one of these escapes is literal.

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

fn parse_seq(chars: &[char], pos: &mut usize, st: Style, in_escape: bool, depth: usize, out: &mut Vec<Run>) {
    let mut parens = 0usize;
    while *pos < chars.len() {
        let c = chars[*pos];
        if c == '\\' && depth < MAX_DEPTH && chars.get(*pos + 2) == Some(&'(') {
            let mut inner = st;
            let known = match chars.get(*pos + 1) {
                Some('b') => {
                    inner.bold = true;
                    true
                }
                Some('i') => {
                    inner.italic = true;
                    true
                }
                Some('g') => {
                    inner.greek = true;
                    true
                }
                Some('+') => {
                    inner.rise = st.rise + SUP_RISE * st.scale;
                    inner.scale = st.scale * SCRIPT_SCALE;
                    true
                }
                Some('-') => {
                    inner.rise = st.rise - SUB_DROP * st.scale;
                    inner.scale = st.scale * SCRIPT_SCALE;
                    true
                }
                _ => false,
            };
            if known {
                *pos += 3;
                parse_seq(chars, pos, inner, true, depth + 1, out);
                continue;
            }
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
    fn greek_mapping() {
        assert_eq!(plain("\\g(m)m"), "μm");
        assert_eq!(plain("\\g(D)E"), "ΔE");
        assert_eq!(plain("\\g(abgdpw)"), "αβγδπω");
        assert_eq!(plain("\\g(W) \\g(1)"), "Ω 1");
    }

    #[test]
    fn deep_nesting_does_not_overflow() {
        let s = "\\b(".repeat(5000) + "x" + &")".repeat(5000);
        let r = parse(&s);
        assert!(!r.is_empty());
    }
}
