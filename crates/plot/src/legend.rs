//! The text of a legend: Origin's notation with `\l(n)` for the symbol of plot n and `%(n)` for
//! its name, and the ids those numbers refer to.

use sheaf_core::graph::Graph;

/// One piece of a legend line.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Part<'a> {
    /// Literal text (with the label markup).
    Text(&'a str),
    /// `\l(id)`: the line and symbol sample of a plot.
    Symbol(&'a str),
    /// `%(id)` or `%(id, option)`: the name of a plot, or what the option names (`@LL` the long
    /// name, ...; T339). The option is empty without a comma.
    Name(&'a str, &'a str),
}

/// Whether `text` refers to a plot with `\l(n)` or `%(n)` on any line: a free text label with such a
/// reference is drawn with the legend's line layout (T178).
pub(crate) fn has_plot_refs(text: &str) -> bool {
    lines(text).iter().any(|l| parse_line(l).iter().any(|p| !matches!(p, Part::Text(_))))
}

/// Splits one legend line into its pieces. An escape that is not closed, or that holds anything
/// but a plot id, stays literal text.
pub(crate) fn parse_line(line: &str) -> Vec<Part<'_>> {
    let mut out = Vec::new();
    let mut text_from = 0;
    let mut i = 0;
    let bytes = line.as_bytes();
    while i < bytes.len() {
        let rest = &line[i..];
        if let Some(skip) = literal_block_len(rest) {
            i += skip;
            continue;
        }
        let (kind, skip) = if rest.starts_with("\\l(") || rest.starts_with("\\L(") {
            (0, 3)
        } else if rest.starts_with("%(") {
            (1, 2)
        } else {
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
    let mut depth = 1;
    for (i, byte) in body.bytes().enumerate() {
        if byte == b'(' {
            depth += 1;
        } else if byte == b')' {
            depth -= 1;
            if depth == 0 {
                return Some(i + 4);
            }
        }
    }
    Some(rest.len())
}

/// The lines of a legend text (CR LF, LF or CR end a line); a final empty line is dropped.
pub(crate) fn lines(text: &str) -> Vec<String> {
    let normal = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut out: Vec<String> = normal.split('\n').map(str::to_string).collect();
    if out.last().is_some_and(String::is_empty) {
        out.pop();
    }
    out
}

/// A standalone `\sep:nn` adds this percentage of font height before the next entry.
pub(crate) fn separation_percent(line: &str) -> Option<u32> {
    let number = line.trim().strip_prefix("\\sep:")?;
    if !number.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    number.parse().ok()
}

/// The ids of the plots the legend of layer `index` lists, given the number of drawable plots of
/// each layer (see [`Graph::legend_ids`]).
pub(crate) fn ids(g: &Graph, index: usize, plots_in: &dyn Fn(usize) -> usize) -> Vec<String> {
    g.legend_ids(index, plots_in)
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
}
