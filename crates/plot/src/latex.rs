//! LaTeX math in graph text (T436; Origin 2023b's built-in equation rendering, slice 1, Sheaf's own): a `$...$`
//! span holding a backslash, `^`, `_` or `{` is LaTeX math and becomes the label markup of `crate::markup` before the
//! text is laid out, so it draws (and exports as vectors) with the label's font:
//!
//! - Latin letters in italics, digits and signs upright; spaces are dropped, `\,` `\;` `\quad` give space;
//! - `x^2`, `x_i`, `x_i^2` (one shared column), braces for longer scripts;
//! - Greek letters, operators and arrows as their characters (`\alpha` α, `\pm` ±, `\infty` ∞, `\rightarrow` →);
//! - `\frac{a}{b}` written `a/b` (parenthesised when longer than one item), `\sqrt{x}` √x, `\sqrt[n]{x}`;
//! - `\bar`, `\dot`, `\tilde` as the markup accents, `\hat` and `\vec` as combining marks;
//! - `\mathrm`, `\text`, `\operatorname` upright, `\mathbf` bold, `\mathit` italic; `\sin`, `\log` ... upright;
//! - `\left`, `\right` and the `\big` sizes dropped, their delimiters kept; an unknown command is written by name.
//!
//! A span is math when it holds a backslash, `^`, `_` or `{`, or starts with a letter and has no space at either end
//! (`$x$`); other spans (`$5 and $10`) are left as they are. A `$` before `(` starts a substitution, not math; `\$`
//! is a dollar sign. Not drawn as in LaTeX: stacked fractions, large operators with limits
//! above and below, matrices and stretchy delimiters.

use std::borrow::Cow;

/// The text with every `$...$` math span turned into label markup.
pub fn expand(text: &str) -> Cow<'_, str> {
    if !text.contains('$') {
        return Cow::Borrowed(text);
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let mut changed = false;
    while i < chars.len() {
        let c = chars[i];
        // `\$` is a dollar sign.
        if c == '\\' && chars.get(i + 1) == Some(&'$') {
            out.push('$');
            i += 2;
            changed = true;
            continue;
        }
        if c == '$' && chars.get(i + 1) != Some(&'(') {
            // The closing `$` (not escaped).
            let close = (i + 1..chars.len()).find(|&k| chars[k] == '$' && chars[k - 1] != '\\');
            if let Some(close) = close {
                let body: String = chars[i + 1..close].iter().collect();
                if is_math(&body) {
                    out.push_str(&to_markup(&body));
                    i = close + 1;
                    changed = true;
                    continue;
                }
            }
        }
        out.push(c);
        i += 1;
    }
    if changed { Cow::Owned(out) } else { Cow::Borrowed(text) }
}

/// Whether the text between two `$` is math: it holds a backslash, `^`, `_` or `{`, or it starts with a letter
/// and has no space at either end (`$x$`, `$dE/dt$`; not `$5 and $10`, not `US$ and AU$`).
fn is_math(body: &str) -> bool {
    body.contains(['\\', '^', '_', '{'])
        || (body.starts_with(|c: char| c.is_alphabetic()) && !body.ends_with(char::is_whitespace))
}

/// One piece of math: its markup, and whether it is a single item (no parentheses needed around it).
struct Atom {
    markup: String,
    single: bool,
    /// An italic Latin letter (runs of them share one italic span).
    letter: Option<char>,
}

fn symbol(name: &str) -> Option<&'static str> {
    Some(match name {
        "alpha" => "α",
        "beta" => "β",
        "gamma" => "γ",
        "delta" => "δ",
        "epsilon" | "varepsilon" => "ε",
        "zeta" => "ζ",
        "eta" => "η",
        "theta" | "vartheta" => "θ",
        "iota" => "ι",
        "kappa" => "κ",
        "lambda" => "λ",
        "mu" => "μ",
        "nu" => "ν",
        "xi" => "ξ",
        "pi" => "π",
        "rho" => "ρ",
        "sigma" => "σ",
        "tau" => "τ",
        "upsilon" => "υ",
        "phi" | "varphi" => "φ",
        "chi" => "χ",
        "psi" => "ψ",
        "omega" => "ω",
        "Gamma" => "Γ",
        "Delta" => "Δ",
        "Theta" => "Θ",
        "Lambda" => "Λ",
        "Xi" => "Ξ",
        "Pi" => "Π",
        "Sigma" => "Σ",
        "Upsilon" => "Υ",
        "Phi" => "Φ",
        "Psi" => "Ψ",
        "Omega" => "Ω",
        "pm" => "±",
        "mp" => "∓",
        "times" => "×",
        "cdot" => "·",
        "div" => "÷",
        "leq" | "le" => "≤",
        "geq" | "ge" => "≥",
        "neq" | "ne" => "≠",
        "approx" => "≈",
        "equiv" => "≡",
        "sim" => "∼",
        "propto" => "∝",
        "infty" => "∞",
        "partial" => "∂",
        "nabla" => "∇",
        "sum" => "∑",
        "prod" => "∏",
        "int" => "∫",
        "oint" => "∮",
        "circ" | "degree" => "°",
        "prime" => "′",
        "rightarrow" | "to" => "→",
        "leftarrow" | "gets" => "←",
        "leftrightarrow" => "↔",
        "Rightarrow" => "⇒",
        "Leftarrow" => "⇐",
        "Leftrightarrow" => "⇔",
        "uparrow" => "↑",
        "downarrow" => "↓",
        "in" => "∈",
        "notin" => "∉",
        "subset" => "⊂",
        "cup" => "∪",
        "cap" => "∩",
        "forall" => "∀",
        "exists" => "∃",
        "ldots" | "dots" => "…",
        "cdots" => "⋯",
        "hbar" => "ℏ",
        "ell" => "ℓ",
        "AA" | "angstrom" => "Å",
        "langle" => "⟨",
        "rangle" => "⟩",
        "," | "thinspace" => "\u{2009}",
        ";" | ":" | " " => " ",
        "quad" => "\u{2003}",
        "qquad" => "\u{2003}\u{2003}",
        "!" => "",
        "{" => "{",
        "}" => "}",
        "%" => "%",
        "$" => "$",
        "_" => "_",
        "&" => "&",
        "#" => "#",
        "|" => "‖",
        _ => return None,
    })
}

/// Function names written upright.
const FUNCTIONS: &[&str] = &[
    "sin", "cos", "tan", "cot", "sec", "csc", "arcsin", "arccos", "arctan", "sinh", "cosh", "tanh", "log", "ln", "lg",
    "exp", "lim", "max", "min", "sup", "inf", "det", "deg", "dim", "arg", "gcd", "mod",
];

struct Parser {
    chars: Vec<char>,
    at: usize,
    /// Inside `\mathrm{...}`: letters upright.
    upright: bool,
}

impl Parser {
    fn skip_spaces(&mut self) {
        while self.chars.get(self.at).is_some_and(|c| c.is_whitespace()) {
            self.at += 1;
        }
    }

    /// A `{...}` group (or one item) as markup, read upright or with letters in italics.
    fn argument(&mut self) -> Atom {
        self.skip_spaces();
        if self.chars.get(self.at) == Some(&'{') {
            self.at += 1;
            let inner = self.sequence(Some('}'));
            self.at += 1; // the `}`
            inner
        } else {
            match self.atom() {
                Some(Atom { letter: Some(l), .. }) => Atom { markup: format!("\\i({l})"), single: true, letter: None },
                Some(a) => a,
                None => Atom { markup: String::new(), single: true, letter: None },
            }
        }
    }

    /// The raw text of a `{...}` argument (for `\text` and `\mathrm`).
    fn raw_argument(&mut self) -> String {
        self.skip_spaces();
        if self.chars.get(self.at) != Some(&'{') {
            return self
                .chars
                .get(self.at)
                .map(|c| {
                    self.at += 1;
                    c.to_string()
                })
                .unwrap_or_default();
        }
        self.at += 1;
        let mut depth = 1;
        let mut out = String::new();
        while let Some(&c) = self.chars.get(self.at) {
            self.at += 1;
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            out.push(c);
        }
        out
    }

    fn command(&mut self) -> String {
        let start = self.at;
        while self.chars.get(self.at).is_some_and(|c| c.is_ascii_alphabetic()) {
            self.at += 1;
        }
        if self.at == start && self.at < self.chars.len() {
            self.at += 1; // a one-character command such as `\,` or `\{`
        }
        self.chars[start..self.at].iter().collect()
    }

    /// One item: a letter, a sign, a group or a command with its arguments. `None` at the end or before `}`.
    fn atom(&mut self) -> Option<Atom> {
        self.skip_spaces();
        let c = *self.chars.get(self.at)?;
        if c == '}' {
            return None;
        }
        self.at += 1;
        let plain = |s: String| Atom { markup: s, single: true, letter: None };
        Some(match c {
            '{' => {
                let inner = self.sequence(Some('}'));
                self.at += 1;
                inner
            }
            '\\' => {
                let name = self.command();
                match name.as_str() {
                    "frac" | "dfrac" | "tfrac" => {
                        let (a, b) = (self.argument(), self.argument());
                        let wrap = |x: Atom| if x.single { x.markup } else { format!("({})", x.markup) };
                        Atom { markup: format!("{}/{}", wrap(a), wrap(b)), single: false, letter: None }
                    }
                    "sqrt" => {
                        self.skip_spaces();
                        let index = if self.chars.get(self.at) == Some(&'[') {
                            let end = (self.at..self.chars.len()).find(|&k| self.chars[k] == ']');
                            let index: String =
                                end.map(|e| self.chars[self.at + 1..e].iter().collect()).unwrap_or_default();
                            self.at = end.map_or(self.chars.len(), |e| e + 1);
                            format!("\\+({index})")
                        } else {
                            String::new()
                        };
                        let x = self.argument();
                        let body = if x.single { x.markup } else { format!("({})", x.markup) };
                        Atom { markup: format!("{index}√{body}"), single: false, letter: None }
                    }
                    "bar" | "overline" => plain(format!("\\ab({})", self.argument().markup)),
                    "dot" => plain(format!("\\ad({})", self.argument().markup)),
                    "tilde" => plain(format!("\\at({})", self.argument().markup)),
                    "hat" => plain(format!("{}\u{0302}", self.argument().markup)),
                    "vec" => plain(format!("{}\u{20D7}", self.argument().markup)),
                    "mathrm" | "operatorname" | "rm" => {
                        let was = std::mem::replace(&mut self.upright, true);
                        let a = self.argument();
                        self.upright = was;
                        plain(a.markup)
                    }
                    "text" | "textrm" => plain(self.raw_argument()),
                    "mathbf" | "textbf" | "bf" => plain(format!("\\b({})", self.argument().markup)),
                    "mathit" | "textit" => plain(format!("\\i({})", self.raw_argument())),
                    "left" | "right" | "big" | "Big" | "bigg" | "Bigg" | "bigl" | "bigr" | "Bigl" | "Bigr"
                    | "displaystyle" => {
                        return self.atom();
                    }
                    n if FUNCTIONS.contains(&n) => plain(n.to_string()),
                    n => plain(symbol(n).map_or_else(|| n.to_string(), str::to_string)),
                }
            }
            c if c.is_ascii_alphabetic() && !self.upright => {
                Atom { markup: String::new(), single: true, letter: Some(c) }
            }
            c => plain(c.to_string()),
        })
    }

    /// Items up to `end` (or the end of the text), with their scripts; runs of letters share one italic span.
    fn sequence(&mut self, end: Option<char>) -> Atom {
        let mut out = String::new();
        let mut letters = String::new();
        let mut items = 0;
        let flush = |out: &mut String, letters: &mut String| {
            if !letters.is_empty() {
                out.push_str(&format!("\\i({letters})"));
                letters.clear();
            }
        };
        loop {
            self.skip_spaces();
            match self.chars.get(self.at) {
                None => break,
                Some(&c) if Some(c) == end => break,
                _ => {}
            }
            // A script with nothing before it (`$^{-1}$`) attaches to an empty base.
            let atom = if matches!(self.chars.get(self.at), Some('^' | '_')) {
                Atom { markup: String::new(), single: true, letter: None }
            } else {
                let Some(atom) = self.atom() else { break };
                atom
            };
            items += 1;
            // Its scripts, in either order.
            let (mut sub, mut sup) = (None, None);
            loop {
                self.skip_spaces();
                match self.chars.get(self.at) {
                    Some('^') if sup.is_none() => {
                        self.at += 1;
                        sup = Some(self.argument().markup);
                    }
                    Some('_') if sub.is_none() => {
                        self.at += 1;
                        sub = Some(self.argument().markup);
                    }
                    _ => break,
                }
            }
            match atom.letter {
                Some(l) if sub.is_none() && sup.is_none() => letters.push(l),
                Some(l) => {
                    letters.push(l);
                    flush(&mut out, &mut letters);
                }
                None => {
                    flush(&mut out, &mut letters);
                    out.push_str(&atom.markup);
                }
            }
            match (sub, sup) {
                (Some(b), Some(p)) => out.push_str(&format!("\\=({b},{p})")),
                (Some(b), None) => out.push_str(&format!("\\-({b})")),
                (None, Some(p)) => out.push_str(&format!("\\+({p})")),
                (None, None) => {}
            }
        }
        flush(&mut out, &mut letters);
        Atom { markup: out, single: items <= 1, letter: None }
    }
}

/// LaTeX math (without the `$`s) as label markup.
pub fn to_markup(math: &str) -> String {
    Parser { chars: math.chars().collect(), at: 0, upright: false }.sequence(None).markup
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn math_spans_become_markup() {
        assert_eq!(to_markup("x^2"), r"\i(x)\+(2)");
        assert_eq!(to_markup("E = mc^2"), r"\i(E)=\i(mc)\+(2)");
        assert_eq!(to_markup(r"x_i^{2n}"), r"\i(x)\=(\i(i),2\i(n))");
        assert_eq!(to_markup(r"\alpha + \beta \leq \infty"), "α+β≤∞");
        assert_eq!(to_markup(r"\frac{1}{2}"), "1/2");
        assert_eq!(to_markup(r"\frac{a+b}{c}"), r"(\i(a)+\i(b))/\i(c)");
        assert_eq!(to_markup(r"\sqrt{x}"), r"√\i(x)");
        assert_eq!(to_markup(r"\sqrt[3]{x+1}"), r"\+(3)√(\i(x)+1)");
        assert_eq!(to_markup(r"\bar{x}"), r"\ab(\i(x))");
        assert_eq!(to_markup(r"\mathrm{d}t"), r"d\i(t)");
        assert_eq!(to_markup(r"\text{rate}\,(\mathrm{s^{-1}})"), "rate\u{2009}(s\\+(-1))");
        assert_eq!(to_markup(r"\sin\theta"), "sinθ");
        assert_eq!(to_markup(r"\left( x \right)"), r"(\i(x))");
        assert_eq!(to_markup(r"\mathbf{F}"), r"\b(\i(F))");
        assert_eq!(to_markup(r"\unknown"), "unknown");
    }

    #[test]
    fn only_math_spans_change() {
        assert_eq!(expand("Price ($)"), "Price ($)");
        assert_eq!(expand("$5 and $10"), "$5 and $10", "no math characters");
        assert_eq!(expand("$(x) and $(y)"), "$(x) and $(y)", "substitutions");
        assert_eq!(expand(r"Rate $k_{obs}$ (s$^{-1}$)"), r"Rate \i(k)\-(\i(obs)) (s\+(-1))");
        assert_eq!(expand(r"cost \$5 $x^2$"), r"cost $5 \i(x)\+(2)");
    }
}

#[cfg(test)]
mod markup_tests {
    #[test]
    fn a_label_with_math_draws_its_scripts_and_letters() {
        // Through the label parser: α upright at full size, the 1 as a subscript, x italic.
        let runs = crate::markup::parse(r"Peak $\alpha_1$ at $x$");
        let texts: Vec<&str> = runs.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(texts.concat(), "Peak α1 at x");
        let one = runs.iter().find(|r| r.text == "1").unwrap();
        assert!((one.scale - crate::markup::SCRIPT_SCALE).abs() < 1e-9 && one.rise < 0.0);
        assert!(runs.iter().find(|r| r.text == "x").unwrap().italic);
        assert_eq!(crate::markup::plain("Costs $5, $10"), "Costs $5, $10");
    }
}
