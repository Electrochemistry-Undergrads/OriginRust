//! LaTeX math in graph text (T436; Origin 2023b's built-in equation rendering, slice 1, Sheaf's own): a `$...$`
//! span holding a backslash, `^`, `_` or `{` is LaTeX math and becomes the label markup of `crate::markup` before the
//! text is laid out, so it draws (and exports as vectors) with the label's font:
//!
//! - Latin letters in italics, digits and signs upright; spaces are dropped, `\,` `\;` `\quad` give space;
//! - `x^2`, `x_i`, `x_i^2` (one shared column), braces for longer scripts;
//! - Greek letters, operators and arrows as their characters (`\alpha` α, `\pm` ±, `\infty` ∞, `\rightarrow` →);
//! - `\sum`, `\prod`, `\bigcup` ... (at 140 %) and `\lim`, `\max`, `\min`, `\sup`, `\inf` with their scripts below and above them (the
//!   markup `\lm(op,below,above)`, T481); `\int` keeps its scripts beside it;
//! - `\frac{a}{b}` stacked (the markup `\fr(a,b)`: `a` over a bar over `b`, T480), `\sqrt{x}` √x, `\sqrt[n]{x}`;
//! - `\bar`, `\dot`, `\tilde` as the markup accents, `\hat` and `\vec` as combining marks;
//! - `\mathrm`, `\text`, `\operatorname` upright, `\mathbf` bold, `\mathit` italic; `\sin`, `\log` ... upright;
//! - `\left` ... `\right` delimiters as tall as what they enclose (`\dl(lr,body)`, T482); the `\big` sizes dropped,
//!   their delimiters kept; an unknown command is written by name.
//!
//! A span is math when it holds a backslash, `^`, `_` or `{`, or starts with a letter and has no space at either end
//! (`$x$`); other spans (`$5 and $10`) are left as they are. A `$` before `(` starts a substitution, not math; `\$`
//! is a dollar sign. Matrix environments (`matrix`, `pmatrix`, `bmatrix`, `Bmatrix`, `vmatrix`) become a grid of cells, `\mx(cols,cells...)`,
//! inside stretchy delimiters (T483).

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
        "coprod" => "∐",
        "bigcup" => "⋃",
        "bigcap" => "⋂",
        "bigoplus" => "⨁",
        "bigotimes" => "⨂",
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
    /// Open `\left` groups: a sequence inside one ends before `\right` (T482).
    in_left: usize,
    /// Open matrix environments: a sequence inside one ends before `&`, `\\` or `\end` (T483).
    in_matrix: usize,
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

    /// The delimiter after `\left` or `\right` as the letter of the markup escape `\dl` (T482): `p` round,
    /// `b` square, `c` curly, `v` a bar, `a` angle, `n` none (`.` or anything else).
    fn delimiter(&mut self) -> char {
        self.skip_spaces();
        let Some(&c) = self.chars.get(self.at) else { return 'n' };
        self.at += 1;
        let name = if c == '\\' { self.command() } else { c.to_string() };
        match name.as_str() {
            "(" | ")" => 'p',
            "[" | "]" | "lbrack" | "rbrack" => 'b',
            "{" | "}" | "lbrace" | "rbrace" => 'c',
            "|" | "vert" | "lvert" | "rvert" => 'v',
            "langle" | "rangle" | "<" | ">" => 'a',
            _ => 'n',
        }
    }

    /// The cells of a matrix environment up to and past its `\end{...}`, as markup, row by row (T483).
    fn matrix_rows(&mut self) -> Vec<Vec<String>> {
        self.in_matrix += 1;
        let mut rows = vec![Vec::new()];
        loop {
            let cell = self.sequence(None);
            rows.last_mut().expect("a row").push(cell.markup);
            self.skip_spaces();
            if self.chars.get(self.at) == Some(&'&') {
                self.at += 1;
            } else if self.at_word("\\\\") {
                self.at += 2;
                rows.push(Vec::new());
            } else {
                if self.at_word("\\end") {
                    self.at += 4;
                    self.raw_argument();
                }
                break;
            }
        }
        self.in_matrix -= 1;
        // a last row left empty by a closing `\\`
        if rows.len() > 1 && rows.last().is_some_and(|r| r.iter().all(String::is_empty)) {
            rows.pop();
        }
        rows
    }

    /// The text at the cursor is `word` (a command not followed by more letters).
    fn at_word(&self, word: &str) -> bool {
        let w: Vec<char> = word.chars().collect();
        self.chars.get(self.at..self.at + w.len()) == Some(&w[..])
            && !(w.last().is_some_and(|c| c.is_ascii_alphabetic())
                && self.chars.get(self.at + w.len()).is_some_and(|c| c.is_ascii_alphabetic()))
    }

    /// The text at the cursor is `\right` (not the start of a longer command name).
    fn at_right(&self) -> bool {
        let word: Vec<char> = "\\right".chars().collect();
        self.chars.get(self.at..self.at + word.len()) == Some(&word[..])
            && !self.chars.get(self.at + word.len()).is_some_and(|c| c.is_ascii_alphabetic())
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
                        // T480: a stacked fraction; a comma outside parentheses in a part would end it early
                        let (a, b) = (self.argument(), self.argument());
                        let part = |x: Atom| escape_outer_commas(&x.markup);
                        Atom { markup: format!("\\fr({},{})", part(a), part(b)), single: true, letter: None }
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
                    "begin" => {
                        // T483: a matrix environment, its cells row by row inside delimiters of its kind
                        let env = self.raw_argument();
                        let kinds = match env.trim() {
                            "pmatrix" => "pp",
                            "bmatrix" => "bb",
                            "Bmatrix" => "cc",
                            "vmatrix" => "vv",
                            "Vmatrix" => "vv",
                            _ => "nn",
                        };
                        let rows = self.matrix_rows();
                        let cols = rows.iter().map(Vec::len).max().unwrap_or(0).max(1);
                        let mut cells = Vec::new();
                        for row in &rows {
                            for k in 0..cols {
                                cells.push(escape_outer_commas(row.get(k).map_or("", String::as_str)));
                            }
                        }
                        let grid = format!("\\mx({cols},{})", cells.join(","));
                        if kinds == "nn" {
                            plain(grid)
                        } else {
                            plain(format!("\\dl({kinds},{})", escape_outer_commas(&grid)))
                        }
                    }
                    "left" => {
                        // T482: a delimiter as tall as what it encloses, up to the matching `\right`
                        let open = self.delimiter();
                        self.in_left += 1;
                        let body = self.sequence(None);
                        self.in_left -= 1;
                        let close = if self.at_right() {
                            self.at += "\\right".len();
                            self.delimiter()
                        } else {
                            'n'
                        };
                        plain(format!("\\dl({open}{close},{})", escape_outer_commas(&body.markup)))
                    }
                    "right" | "big" | "Big" | "bigg" | "Bigg" | "bigl" | "bigr" | "Bigl" | "Bigr" | "displaystyle" => {
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
            if self.in_left > 0 && self.at_right() {
                break;
            }
            if self.in_matrix > 0
                && (self.chars.get(self.at) == Some(&'&') || self.at_word("\\\\") || self.at_word("\\end"))
            {
                break;
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
                    // T481: a large operator or a limit function takes its scripts below and above it
                    if (sub.is_some() || sup.is_some())
                        && let Some(op) = limit_operator(&atom.markup)
                    {
                        let part = |x: Option<String>| escape_outer_commas(&x.unwrap_or_default());
                        out.push_str(&format!("\\lm({op},{},{})", part(sub), part(sup)));
                        continue;
                    }
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
    Parser { chars: math.chars().collect(), at: 0, upright: false, in_left: 0, in_matrix: 0 }.sequence(None).markup
}

/// The operator markup of an atom that takes its scripts below and above it (T481): the large operators
/// (drawn at 140 %) and the limit functions; `None` for anything else (`\int` keeps its scripts beside it).
fn limit_operator(markup: &str) -> Option<String> {
    match markup {
        "∑" | "∏" | "∐" | "⋃" | "⋂" | "⨁" | "⨂" => Some(format!("\\p140({markup})")),
        "lim" | "max" | "min" | "sup" | "inf" => Some(markup.to_string()),
        _ => None,
    }
}

/// `text` with its commas outside parentheses written as `\(44)`, so that it can be one part of a
/// two-part escape such as `\fr(num,den)` (T480).
fn escape_outer_commas(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut nesting = 0usize;
    for c in text.chars() {
        match c {
            '(' => nesting += 1,
            ')' => nesting = nesting.saturating_sub(1),
            _ => {}
        }
        if c == ',' && nesting == 0 {
            out.push_str("\\(44)");
        } else {
            out.push(c);
        }
    }
    out
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
        assert_eq!(to_markup(r"\frac{1}{2}"), r"\fr(1,2)");
        assert_eq!(to_markup(r"\frac{a+b}{c}"), r"\fr(\i(a)+\i(b),\i(c))");
        assert_eq!(to_markup(r"\sum_{i=1}^{n} x_i"), r"\lm(\p140(∑),\i(i)=1,\i(n))\i(x)\-(\i(i))");
        assert_eq!(to_markup(r"\lim_{x\to 0}"), r"\lm(lim,\i(x)→0,)");
        assert_eq!(to_markup(r"\left(\frac{a}{b}\right)"), r"\dl(pp,\fr(\i(a),\i(b)))");
        assert_eq!(to_markup(r"\left\{ x, y \right."), r"\dl(cn,\i(x)\(44)\i(y))");
        assert_eq!(to_markup(r"\left[a\right|"), r"\dl(bv,\i(a))");
        assert_eq!(to_markup(r"\begin{pmatrix} a & b \\ 1 & 0 \end{pmatrix}"), r"\dl(pp,\mx(2,\i(a),\i(b),1,0))");
        assert_eq!(to_markup(r"\begin{matrix} x \\ y \\ \end{matrix}"), r"\mx(1,\i(x),\i(y))");
        assert_eq!(to_markup(r"\begin{bmatrix} 1 & 2 \\ 3 \end{bmatrix}"), r"\dl(bb,\mx(2,1,2,3,))");
        assert_eq!(to_markup(r"\int_0^1"), r"∫\=(0,1)", "an integral keeps its scripts beside it");
        assert_eq!(to_markup(r"\frac{1,5}{2}"), r"\fr(1\(44)5,2)", "a comma of a part is not the separator");
        assert_eq!(to_markup(r"\sqrt{x}"), r"√\i(x)");
        assert_eq!(to_markup(r"\sqrt[3]{x+1}"), r"\+(3)√(\i(x)+1)");
        assert_eq!(to_markup(r"\bar{x}"), r"\ab(\i(x))");
        assert_eq!(to_markup(r"\mathrm{d}t"), r"d\i(t)");
        assert_eq!(to_markup(r"\text{rate}\,(\mathrm{s^{-1}})"), "rate\u{2009}(s\\+(-1))");
        assert_eq!(to_markup(r"\sin\theta"), "sinθ");
        assert_eq!(to_markup(r"\left( x \right)"), r"\dl(pp,\i(x))", "stretchy since T482");
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
