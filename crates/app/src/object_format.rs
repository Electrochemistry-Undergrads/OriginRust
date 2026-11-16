//! Format-bar controls that act on a whole selected graph object but are not font changes:
//! Underline, Superscript, Subscript and Greek (whole-text markup) and Fill / Line Color.
//! Kept apart from `text_format` so the font contract and the worksheet adapter stay untouched.
//! Whole-text wrapping, the target restrictions and the palettes are unmeasured Sheaf choices.

use sheaf_core::{Color, Document, Project, WindowId};
use sheaf_plot::scene::ObjectKind;

/// The named basic colours offered by the Font, Fill and Line Color palettes.
pub const PALETTE: [(&str, Color); 8] = [
    ("Black", Color::rgb(0, 0, 0)),
    ("Red", Color::rgb(255, 0, 0)),
    ("Green", Color::rgb(0, 255, 0)),
    ("Blue", Color::rgb(0, 0, 255)),
    ("Cyan", Color::rgb(0, 255, 255)),
    ("Magenta", Color::rgb(255, 0, 255)),
    ("Yellow", Color::rgb(255, 255, 0)),
    ("White", Color::rgb(255, 255, 255)),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    pub graph: WindowId,
    pub layer: usize,
    pub object: ObjectKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Markup {
    Underline,
    Superscript,
    Subscript,
    Greek,
}

impl Markup {
    fn prefix(self) -> &'static str {
        match self {
            Markup::Underline => "\\u(",
            Markup::Superscript => "\\+(",
            Markup::Subscript => "\\-(",
            Markup::Greek => "\\g(",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Change {
    Markup(Markup),
    Fill(Option<Color>),
    Line(Color),
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub target: Target,
    /// The explicit text when whole-text markup applies.
    pub markup: Option<String>,
    /// `Some` when Fill Color applies; the inner value is the current fill.
    pub fill: Option<Option<Color>>,
    /// `Some` when Line Color applies; the current line colour.
    pub line: Option<Color>,
    pub generation: u64,
    pub revision: u64,
}

#[derive(Clone, Debug)]
pub struct Request {
    pub target: Target,
    pub generation: u64,
    pub revision: u64,
    pub change: Change,
}

impl Snapshot {
    /// `None` when no control of this module applies to the object.
    pub fn graph(doc: &Document, graph: WindowId, layer: usize, object: ObjectKind) -> Option<Self> {
        let target = Target { graph, layer, object };
        let project = doc.project();
        let markup = text(project, target).map(str::to_owned);
        let fill = fill(project, target);
        let line = line(project, target);
        if markup.is_none() && fill.is_none() && line.is_none() {
            return None;
        }
        Some(Self { target, markup, fill, line, generation: doc.generation(), revision: doc.revision() })
    }

    pub fn request(&self, change: Change) -> Request {
        Request { target: self.target, generation: self.generation, revision: self.revision, change }
    }

    /// Whether the whole text is already wrapped by this kind of group.
    pub fn wrapped(&self, markup: Markup) -> bool {
        self.markup.as_deref().is_some_and(|t| is_wrapped(t, markup))
    }
}

fn text(project: &Project, target: Target) -> Option<&str> {
    let l = project.graph(target.graph)?.layers.get(target.layer)?;
    let t = match target.object {
        ObjectKind::Label(i) => l.labels.get(i)?.text.as_str(),
        ObjectKind::XTitle => l.x_axis.title.as_deref()?,
        ObjectKind::YTitle => l.y_axis.title.as_deref()?,
        _ => return None,
    };
    (!t.is_empty()).then_some(t)
}

fn fill(project: &Project, target: Target) -> Option<Option<Color>> {
    let l = project.graph(target.graph)?.layers.get(target.layer)?;
    match target.object {
        ObjectKind::Shape(i) => {
            let s = l.shapes.get(i)?;
            (!s.kind.is_linear()).then_some(s.fill)
        }
        ObjectKind::Legend if l.legend.visible => Some(l.legend.background),
        _ => None,
    }
}

fn line(project: &Project, target: Target) -> Option<Color> {
    let l = project.graph(target.graph)?.layers.get(target.layer)?;
    match target.object {
        ObjectKind::Shape(i) => Some(l.shapes.get(i)?.color),
        ObjectKind::StraightLine(i) => Some(l.straight_lines.get(i)?.color),
        _ => None,
    }
}

/// Index of the `)` that closes a group whose `(` is just before `from`, counting nested
/// parentheses the way the markup parser does.
fn close_of(chars: &[char], from: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, &c) in chars.iter().enumerate().skip(from) {
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => return Some(i),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// The whole text is exactly one group of this kind.
pub fn is_wrapped(text: &str, markup: Markup) -> bool {
    let prefix = markup.prefix();
    if !text.starts_with(prefix) {
        return false;
    }
    let chars: Vec<char> = text.chars().collect();
    close_of(&chars, prefix.chars().count()) == Some(chars.len() - 1)
}

/// Wraps the whole text in one group, or removes that group when it is already there. A group
/// may span lines (the parser balances parentheses across them), so multi-line text is one group.
pub fn toggle(text: &str, markup: Markup) -> String {
    if is_wrapped(text, markup) {
        let prefix = markup.prefix();
        text[prefix.len()..text.len() - 1].to_string()
    } else {
        let escaped = escape_unmatched(text);
        let wrapped = format!("{}{escaped})", markup.prefix());
        // Greek would remap the letters, so the visible text is compared through a neutral group.
        let same_text = sheaf_plot::markup::plain(&format!("\\u({escaped})")) == sheaf_plot::markup::plain(text);
        // Text whose parentheses are tangled with an unclosed escape (`x\(`) cannot be wrapped
        // safely; leave it as it is rather than corrupt it.
        if is_wrapped(&wrapped, markup) && same_text { wrapped } else { text.to_string() }
    }
}

/// Replaces each unmatched `(` or `)` with its decimal escape (`\(40)`, `\(41)`) so that a group
/// closed with a bare `)` still ends where it should and the text looks the same.
fn escape_unmatched(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut unmatched = vec![false; chars.len()];
    let mut open: Vec<usize> = Vec::new();
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '(' => open.push(i),
            ')' if open.pop().is_none() => unmatched[i] = true,
            _ => {}
        }
    }
    open.into_iter().for_each(|i| unmatched[i] = true);
    let mut out = String::with_capacity(text.len());
    for (i, c) in chars.into_iter().enumerate() {
        match (unmatched[i], c) {
            (true, '(') => out.push_str("\\(40)"),
            (true, _) => out.push_str("\\(41)"),
            _ => out.push(c),
        }
    }
    out
}

/// Reject stale frames, invalid input and no-ops before creating an undo step.
pub fn apply(doc: &mut Document, request: Request) -> bool {
    if doc.generation() != request.generation || doc.revision() != request.revision {
        return false;
    }
    let Target { graph, layer, object } = request.target;
    let project = doc.project();
    match request.change {
        Change::Markup(markup) => {
            let Some(old) = text(project, request.target) else { return false };
            let new = toggle(old, markup);
            if new == old {
                return false;
            }
            doc.apply("Format graph text", |p| {
                let l = &mut p.graph_mut(graph).expect("validated graph").layers[layer];
                match object {
                    ObjectKind::Label(i) => l.labels[i].text = new,
                    ObjectKind::XTitle => l.x_axis.title = Some(new),
                    ObjectKind::YTitle => l.y_axis.title = Some(new),
                    _ => unreachable!("validated text target"),
                }
            });
        }
        Change::Fill(color) => {
            if fill(project, request.target).is_none_or(|old| old == color) {
                return false;
            }
            doc.apply("Format graph object", |p| {
                let l = &mut p.graph_mut(graph).expect("validated graph").layers[layer];
                match object {
                    ObjectKind::Shape(i) => l.shapes[i].fill = color,
                    ObjectKind::Legend => l.legend.background = color,
                    _ => unreachable!("validated fill target"),
                }
            });
        }
        Change::Line(color) => {
            if line(project, request.target).is_none_or(|old| old == color) {
                return false;
            }
            doc.apply("Format graph object", |p| {
                let l = &mut p.graph_mut(graph).expect("validated graph").layers[layer];
                match object {
                    ObjectKind::Shape(i) => l.shapes[i].color = color,
                    ObjectKind::StraightLine(i) => l.straight_lines[i].color = color,
                    _ => unreachable!("validated line target"),
                }
            });
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_plot::markup::plain;

    #[test]
    fn unbalanced_parentheses_survive_wrapping() {
        for t in ["f(x", "1) a", "a)b", "((", "x)(y"] {
            let w = toggle(t, Markup::Underline);
            assert!(is_wrapped(&w, Markup::Underline), "{t}: {w}");
            assert_eq!(plain(&w.replacen("\\u(", "\\b(", 1)), plain(t), "{t}");
        }
        // An unclosed escape cannot be wrapped safely: unchanged.
        assert_eq!(toggle("x\\(", Markup::Underline), "x\\(");
    }

    #[test]
    fn wraps_and_unwraps_the_whole_text() {
        assert_eq!(toggle("cm-2", Markup::Superscript), "\\+(cm-2)");
        assert_eq!(toggle("\\+(cm-2)", Markup::Superscript), "cm-2");
        assert_eq!(toggle("x", Markup::Greek), "\\g(x)");
        assert_eq!(toggle("x", Markup::Subscript), "\\-(x)");
        assert_eq!(toggle("x", Markup::Underline), "\\u(x)");
    }

    #[test]
    fn nested_groups_and_parentheses_are_respected() {
        assert_eq!(toggle("\\u(a\\+(2)b (c))", Markup::Underline), "a\\+(2)b (c)");
        assert!(!is_wrapped("\\u(a)(b)", Markup::Underline));
        assert_eq!(toggle("\\u(a)(b)", Markup::Underline), "\\u(\\u(a)(b))");
    }

    #[test]
    fn a_different_kind_or_partial_group_is_wrapped_not_unwrapped() {
        assert_eq!(toggle("\\u(a)", Markup::Greek), "\\g(\\u(a))");
        assert_eq!(toggle("\\u(a)b", Markup::Underline), "\\u(\\u(a)b)");
        assert_eq!(toggle("\\u(a)\\u(b)", Markup::Underline), "\\u(\\u(a)\\u(b))");
        assert!(!is_wrapped("\\u(abc", Markup::Underline));
    }

    #[test]
    fn multi_line_and_unicode_text_is_one_parseable_group() {
        let t = "line one\nline \u{b0}C \u{3b1}";
        let wrapped = toggle(t, Markup::Underline);
        assert_eq!(wrapped, "\\u(line one\nline \u{b0}C \u{3b1})");
        assert_eq!(plain(&wrapped), t, "the group spans the newline");
        assert!(sheaf_plot::markup::parse(&wrapped).iter().all(|r| r.underline));
        assert_eq!(toggle(&wrapped, Markup::Underline), t);
        assert_eq!(toggle("\u{3b1}\u{3b2}", Markup::Greek), "\\g(\u{3b1}\u{3b2})");
    }
}
