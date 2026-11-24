//! The text of a legend: Origin's notation with `\l(n)` for the symbol of plot n and `%(n)` for
//! its name, and the ids those numbers refer to.

use crate::data::PlotData;
use sheaf_core::graph::Layer;

/// One piece of a legend line.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Part<'a> {
    /// Literal text (with the label markup).
    Text(&'a str),
    /// `\ww(text)`: a wrapped caption whose contents are kept literal.
    Wrap(&'a str),
    /// `\l(id)`: the line and symbol sample of a plot.
    Symbol(&'a str),
    /// `%(id)` or `%(id, option)`: the name of a plot, or what the option names (`@LL` the long
    /// name, ...; T339). The option is empty without a comma.
    Name(&'a str, &'a str),
}

/// Whether `text` refers to a plot with `\l(n)` or `%(n)` on any line: a free text label with such a
/// reference is drawn with the legend's line layout (T178).
pub(crate) fn has_plot_refs(text: &str) -> bool {
    lines(text).iter().any(|l| parse_line(l).iter().any(|p| matches!(p, Part::Symbol(_) | Part::Name(..))))
}

/// Splits one legend line into its pieces. An escape that is not closed, or that holds anything
/// but a plot id, stays literal text.
pub(crate) fn parse_line(line: &str) -> Vec<Part<'_>> {
    let mut out = Vec::new();
    let mut text_from = 0;
    let mut i = 0;
    let mut parens = 0usize;
    let bytes = line.as_bytes();
    while i < bytes.len() {
        let rest = &line[i..];
        if let Some(skip) = literal_block_len(rest) {
            i += skip;
            continue;
        }
        if parens == 0
            && let Some(body) = rest.strip_prefix("\\ww(")
        {
            let Some(close) = closing_parenthesis(body) else { break };
            if text_from < i {
                out.push(Part::Text(&line[text_from..i]));
            }
            out.push(Part::Wrap(&body[..close]));
            i += 5 + close;
            text_from = i;
            continue;
        }
        let (kind, skip) = if rest.starts_with("\\l(") || rest.starts_with("\\L(") {
            (0, 3)
        } else if rest.starts_with("%(") {
            (1, 2)
        } else {
            match bytes[i] {
                b'(' => parens += 1,
                b')' => parens = parens.saturating_sub(1),
                _ => {}
            }
            i += rest.chars().next().map_or(1, char::len_utf8);
            continue;
        };
        let Some(close) = rest[skip..].find(')') else {
            i += skip;
            continue;
        };
        let inner = &rest[skip..skip + close];
        let (id, option) = inner.split_once(',').unwrap_or((inner, ""));
        let (id, option) = (id.trim(), option.trim());
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit() || c == '.') {
            i += skip;
            continue;
        }
        if text_from < i {
            out.push(Part::Text(&line[text_from..i]));
        }
        out.push(if kind == 0 { Part::Symbol(id) } else { Part::Name(id, option) });
        i += skip + close + 1;
        text_from = i;
    }
    if text_from < line.len() {
        out.push(Part::Text(&line[text_from..]));
    }
    out
}

/// Leaves a protected block whole so the markup renderer can draw its literal contents.
fn literal_block_len(rest: &str) -> Option<usize> {
    let body = rest.strip_prefix("\\h(").or_else(|| rest.strip_prefix("\\v("))?;
    Some(closing_parenthesis(body).map_or(rest.len(), |i| i + 4))
}

fn closing_parenthesis(body: &str) -> Option<usize> {
    let mut depth = 1;
    for (i, byte) in body.bytes().enumerate() {
        if byte == b'(' {
            depth += 1;
        } else if byte == b')' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

/// Wraps a literal caption at whitespace and returns its lines and maximum measured width.
/// A word wider than the available space is kept whole.
pub(crate) fn wrap_literal(text: &str, available: f64, measure: impl Fn(&str) -> f64) -> (Vec<String>, f64) {
    let text = text.trim();
    let mut lines = Vec::new();
    let mut widest: f64 = 0.0;
    let (mut line_start, mut word_start, mut last_word_end) = (0, 0, 0);
    let mut current_width = 0.0;
    for part in text.split_inclusive(char::is_whitespace) {
        let word_end = word_start + part.trim_end().len();
        if word_end > word_start {
            let width = measure(&text[line_start..word_end]);
            if width > available && word_start > line_start {
                lines.push(text[line_start..last_word_end].to_string());
                widest = widest.max(current_width);
                line_start = word_start;
                current_width = measure(&text[line_start..word_end]);
            } else {
                current_width = width;
            }
            last_word_end = word_end;
        }
        word_start += part.len();
    }
    if !text.is_empty() {
        lines.push(text[line_start..].to_string());
        widest = widest.max(current_width);
    }
    (lines, widest)
}

/// The lines of a legend text. Authored line-break and tab tokens are expanded before resolving
/// plot references. CR LF, LF and CR end a line, and a final empty line is dropped.
pub(crate) fn lines(text: &str) -> Vec<String> {
    let expanded;
    let text = if text.contains("%(") {
        expanded = expand_keywords(text);
        expanded.as_str()
    } else {
        text
    };
    let normal = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut out: Vec<String> = normal.split('\n').map(str::to_string).collect();
    if out.last().is_some_and(String::is_empty) {
        out.pop();
    }
    out
}

fn expand_keywords(text: &str) -> String {
    let mut expanded = String::with_capacity(text.len());
    for mut line in text.split_inclusive(['\r', '\n']) {
        while let Some(start) = line.find(['%', '\\']) {
            expanded.push_str(&line[..start]);
            line = &line[start..];
            let protected = literal_block_len(line).or_else(|| {
                let body = line.strip_prefix("\\ww(")?;
                Some(closing_parenthesis(body).map_or(line.len(), |end| end + 5))
            });
            if let Some(len) = protected {
                expanded.push_str(&line[..len]);
                line = &line[len..];
            } else if let Some((value, rest)) = keyword_token(line) {
                expanded.push_str(value);
                line = rest;
            } else {
                // Both starting characters are one byte.
                expanded.push_str(&line[..1]);
                line = &line[1..];
            }
        }
        expanded.push_str(line);
    }
    expanded
}

fn keyword_token(text: &str) -> Option<(&'static str, &str)> {
    let (keyword, rest) = text.strip_prefix("%(")?.split_once(')')?;
    let keyword = keyword.trim();
    let value = if keyword.eq_ignore_ascii_case("CRLF") {
        "\r\n"
    } else if keyword.eq_ignore_ascii_case("CR") {
        "\r"
    } else if keyword.eq_ignore_ascii_case("LF") {
        "\n"
    } else if keyword.eq_ignore_ascii_case("TAB") {
        "\t"
    } else {
        return None;
    };
    Some((value, rest))
}

/// The next strictly greater tab stop, measured from the start of the legend row.
pub(crate) fn tab_advance(offset: f64, step: f64) -> f64 {
    if step.is_finite() && step > 0.0 { ((offset / step).floor() + 1.0) * step - offset } else { 0.0 }
}

/// A standalone `\sep:nn` adds this percentage of font height before the next entry.
pub(crate) fn separation_percent(line: &str) -> Option<u32> {
    let number = line.trim().strip_prefix("\\sep:")?;
    if !number.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    number.parse().ok()
}

/// Ordinary legends number the drawable plots consecutively. Stock multi-Y legends
/// preserve model slots so that hiding a first plot does not change its layer's other rows.
pub(crate) fn plot_slots(layer: &Layer, plots: &[PlotData], stock_multi_y: bool) -> Vec<usize> {
    if stock_multi_y { layer.stock_legend_plot_slots(plots.iter().map(|p| p.plot)) } else { (0..plots.len()).collect() }
}

/// The entry that `id` names among `ids`: an exact match, or else a plain number counts the
/// entries from 1.
pub(crate) fn resolve(ids: &[String], id: &str) -> Option<usize> {
    ids.iter().position(|s| s == id).or_else(|| {
        let n: usize = id.parse().ok()?;
        (n >= 1 && n <= ids.len()).then(|| n - 1)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_split_on_any_line_end() {
        assert_eq!(lines("a\r\nb\nc\rd"), vec!["a", "b", "c", "d"]);
        assert_eq!(lines("a\n\nb\n"), vec!["a", "", "b"]);
        assert!(lines("").is_empty());
    }

    #[test]
    fn legend_line_break_tokens_act_like_authored_cr_and_lf() {
        assert_eq!(lines("a%(crlf)b%( CR )%(lF)c%(CRLF)%(LF)d%(CRLF)"), ["a", "b", "c", "", "d"]);
        assert_eq!(lines("α%(LF)β\r\nγ%(cr)δ"), ["α", "β", "γ", "δ"]);
        assert_eq!(lines(r"start %(CRLFX) end %(CRLF"), ["start %(CRLFX) end %(CRLF"]);
    }

    #[test]
    fn literal_legend_spans_protect_line_break_tokens() {
        let text = r"\v(a%(CRLF))\h(b%(CR))\ww(c%(LF))%(CRLF)end";
        assert_eq!(lines(text), [r"\v(a%(CRLF))\h(b%(CR))\ww(c%(LF))", "end"]);
        assert_eq!(lines(r"\h(unclosed %(CRLF)"), [r"\h(unclosed %(CRLF)"]);
    }

    #[test]
    fn legend_tabs_are_tokens_and_respect_literal_spans() {
        let text = r"\l(1) %(1)%( TaB )\h(%(TAB))%(TAB)\ww(%(TAB))";
        let expanded = lines(text);
        assert_eq!(
            parse_line(&expanded[0]),
            [
                Part::Symbol("1"),
                Part::Text(" "),
                Part::Name("1", ""),
                Part::Text("\t\\h(%(TAB))\t"),
                Part::Wrap("%(TAB)"),
            ]
        );
        assert_eq!(lines("A%(TAB)%(TAB)B%(TAB)"), ["A\t\tB\t"]);
        assert_eq!(tab_advance(24.0, 20.0), 16.0);
        assert_eq!(tab_advance(40.0, 20.0), 20.0);
    }

    #[test]
    fn symbols_names_and_text_are_found() {
        assert_eq!(
            parse_line("\\l(1) %(1) at 5\\+(2) K"),
            vec![Part::Symbol("1"), Part::Text(" "), Part::Name("1", ""), Part::Text(" at 5\\+(2) K")]
        );
        assert_eq!(parse_line("%(2.1,@LG)"), vec![Part::Name("2.1", "@LG")]);
        assert_eq!(parse_line("\\L(2.1)"), vec![Part::Symbol("2.1")], "the capital form is the same");
        // Not plot ids, or not closed: literal.
        assert_eq!(parse_line("%(x) \\l(1"), vec![Part::Text("%(x) \\l(1")]);
        assert_eq!(parse_line("\u{b0}C \u{3b1}%(1)"), vec![Part::Text("\u{b0}C \u{3b1}"), Part::Name("1", "")]);
    }

    #[test]
    fn plain_numbers_count_entries_and_dotted_ids_match_exactly() {
        let ids: Vec<String> = vec!["1.1".into(), "2.1".into()];
        assert_eq!(resolve(&ids, "2.1"), Some(1));
        assert_eq!(resolve(&ids, "2"), Some(1));
        assert_eq!(resolve(&ids, "3"), None);
        assert_eq!(resolve(&ids, "0"), None);
    }

    #[test]
    fn literal_blocks_keep_legend_references_together() {
        let protected = r"\b(\h(μ \l(1) %(1))) \v(\L(2) %(2)) ";
        let line = format!("{protected}\\l(1) %(2)");
        assert_eq!(parse_line(&line), [Part::Text(protected), Part::Symbol("1"), Part::Text(" "), Part::Name("2", "")],);
        let open = r"\h(Text %(1)";
        assert_eq!(parse_line(open), [Part::Text(open)]);
    }

    #[test]
    fn wrapped_captions_are_balanced_and_protect_literal_plot_references() {
        let text = r"\l(1) \ww(α (%(2)) \l(3)) %(1)";
        assert_eq!(
            parse_line(text),
            [Part::Symbol("1"), Part::Text(" "), Part::Wrap(r"α (%(2)) \l(3)"), Part::Text(" "), Part::Name("1", "")],
        );
        assert!(!has_plot_refs(r"\ww(%(2))"));
        let protected = r"\v(\ww(text))";
        let unclosed = r"\ww(caption %(1)";
        assert_eq!(parse_line(protected), [Part::Text(protected)]);
        assert_eq!(parse_line(unclosed), [Part::Text(unclosed)]);
    }

    #[test]
    fn literal_wrap_keeps_internal_spacing_and_does_not_split_long_words() {
        let (lines, width) = wrap_literal("élan  A Longword tail", 7.0, |text| text.chars().count() as f64);
        assert_eq!(lines, ["élan  A", "Longword", "tail"]);
        assert_eq!(width, 8.0);
    }
}
