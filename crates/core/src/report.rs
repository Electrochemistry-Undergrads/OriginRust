//! Linked reports (T439; Origin 2019b's linked HTML/Markdown analysis reports, Sheaf's own design): a Notes window's
//! text read as a report. Links in it show live values of the project and a figure line embeds a graph; the report
//! is shown resolved in the Notes window's Preview and written as HTML or Markdown with the graphs as pictures.
//!
//! - `%([Book1]Sheet1,B,3)` (Origin's worksheet cell substitution, with an optional format `,.2`) is that cell;
//! - `$(formula)` is the first value of any formula over named sheets, `$(mean([Book1]Sheet1!col(B)))` for one;
//! - a line that is only `![caption](Graph1)` is graph Graph1 as a figure with its caption;
//! - a link that does not work shows `[link error: ...]` in its place.
//!
//! In the HTML a line starting `# `, `## ` or `### ` is a heading, blank lines separate paragraphs and the text is
//! escaped; the Markdown is the resolved text as typed.

use crate::ids::WindowId;
use crate::project::Project;

/// A piece of a resolved report.
#[derive(Clone, Debug, PartialEq)]
pub enum ReportPart {
    /// Resolved text, lines separated by `\n`.
    Text(String),
    /// A figure line: the graph named (when the project has it) and the caption.
    Figure { name: String, graph: Option<WindowId>, caption: String },
}

/// `![caption](name)` alone on a line.
fn figure(line: &str) -> Option<(String, String)> {
    let rest = line.trim().strip_prefix("![")?;
    let (caption, rest) = rest.split_once("](")?;
    let name = rest.strip_suffix(')')?;
    (!name.is_empty() && !name.contains(['(', ')'])).then(|| (caption.to_string(), name.trim().to_string()))
}

/// The end (exclusive, after the `)`) of a link whose `(` is at `open`; parentheses inside quotes do not count.
fn link_end(chars: &[char], open: usize) -> Option<usize> {
    let mut depth = 0;
    let mut quote = false;
    for (k, &c) in chars.iter().enumerate().skip(open) {
        match c {
            '"' => quote = !quote,
            '(' if !quote => depth += 1,
            ')' if !quote => {
                depth -= 1;
                if depth == 0 {
                    return Some(k + 1);
                }
            }
            _ => {}
        }
    }
    None
}

impl Project {
    /// `line` with its `%(...)` and `$(...)` links replaced by their values.
    pub fn resolve_links(&self, line: &str) -> String {
        let chars: Vec<char> = line.chars().collect();
        let mut out = String::new();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if matches!(c, '%' | '$')
                && chars.get(i + 1) == Some(&'(')
                && let Some(end) = link_end(&chars, i + 1)
            {
                // `%(...)` is read whole (the formula language knows it); `$(...)` is its inside.
                let src: String =
                    if c == '%' { chars[i..end].iter().collect() } else { chars[i + 2..end - 1].iter().collect() };
                match self.evaluate_link(&src) {
                    Ok(v) => out.push_str(&v),
                    Err(e) => out.push_str(&format!("[link error: {e}]")),
                }
                i = end;
                continue;
            }
            out.push(c);
            i += 1;
        }
        out
    }

    /// `text` as a report: its links resolved and its figure lines split out.
    pub fn report_parts(&self, text: &str) -> Vec<ReportPart> {
        let mut parts = Vec::new();
        let mut buf: Vec<String> = Vec::new();
        for line in text.lines() {
            if let Some((caption, name)) = figure(line) {
                if !buf.is_empty() {
                    parts.push(ReportPart::Text(buf.join("\n")));
                    buf.clear();
                }
                let graph = self.graphs().find(|g| g.name.eq_ignore_ascii_case(&name)).map(|g| g.id);
                parts.push(ReportPart::Figure { name, graph, caption });
            } else {
                buf.push(self.resolve_links(line));
            }
        }
        if !buf.is_empty() {
            parts.push(ReportPart::Text(buf.join("\n")));
        }
        parts
    }
}

/// The report as Markdown; `picture(name)` is the file a figure's graph is written to.
pub fn to_markdown(parts: &[ReportPart], picture: &dyn Fn(&str) -> String) -> String {
    let mut out = String::new();
    for p in parts {
        match p {
            ReportPart::Text(t) => out.push_str(t),
            ReportPart::Figure { name, caption, .. } => out.push_str(&format!("![{caption}]({})", picture(name))),
        }
        out.push('\n');
    }
    out
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// The report as a web page titled `title`; `picture(name)` as for [`to_markdown`].
pub fn to_html(title: &str, parts: &[ReportPart], picture: &dyn Fn(&str) -> String) -> String {
    let mut body = String::new();
    for p in parts {
        match p {
            ReportPart::Text(t) => {
                let mut para: Vec<String> = Vec::new();
                let flush = |para: &mut Vec<String>, body: &mut String| {
                    if !para.is_empty() {
                        body.push_str(&format!("<p>{}</p>\n", para.join("<br>\n")));
                        para.clear();
                    }
                };
                for line in t.lines() {
                    let heading = ["### ", "## ", "# "].iter().zip([3, 2, 1]).find(|(m, _)| line.starts_with(**m));
                    if let Some((m, level)) = heading {
                        flush(&mut para, &mut body);
                        body.push_str(&format!("<h{level}>{}</h{level}>\n", escape(&line[m.len()..])));
                    } else if line.trim().is_empty() {
                        flush(&mut para, &mut body);
                    } else {
                        para.push(escape(line));
                    }
                }
                flush(&mut para, &mut body);
            }
            ReportPart::Figure { name, caption, .. } => body.push_str(&format!(
                "<figure><img src=\"{}\" alt=\"{}\"><figcaption>{}</figcaption></figure>\n",
                escape(&picture(name)),
                escape(caption),
                escape(caption)
            )),
        }
    }
    format!(
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>{}</title>\n</head>\n<body>\n{body}</body>\n</html>\n",
        escape(title)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};
    use crate::template::Template;

    #[test]
    fn links_resolve_and_figures_split_the_report() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![10.0, 20.0, 60.0])).unwrap();
        let g = p.plot_columns(Template::Line, &[y]).unwrap();
        let (book, graph) = (p.workbook(b).unwrap().name.clone(), p.graph(g).unwrap().name.clone());
        let text = format!(
            "# Run 3\nThe last value is %([{book}]Sheet1,B,3) and the mean $(mean([{book}]Sheet1!col(B))).\n![Figure 1. B against A]({graph})\nBad: $(nope(1))"
        );
        let parts = p.report_parts(&text);
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], ReportPart::Text("# Run 3\nThe last value is 60 and the mean 30.".into()));
        assert_eq!(
            parts[1],
            ReportPart::Figure { name: graph.clone(), graph: Some(g), caption: "Figure 1. B against A".into() }
        );
        let ReportPart::Text(bad) = &parts[2] else { panic!() };
        assert!(bad.starts_with("Bad: [link error:"), "{bad}");
        let md = to_markdown(&parts, &|n| format!("run_{n}.png"));
        assert!(md.contains(&format!("![Figure 1. B against A](run_{graph}.png)")));
        let html = to_html("Run <3>", &parts, &|n| format!("run_{n}.png"));
        assert!(html.contains("<title>Run &lt;3&gt;</title>") && html.contains("<h1>Run 3</h1>"));
        assert!(html.contains("<p>The last value is 60 and the mean 30.</p>"));
        assert!(html.contains(&format!("<img src=\"run_{graph}.png\"")));
        // Text that only looks like a link stays.
        assert_eq!(p.resolve_links("costs $5 (approx.)"), "costs $5 (approx.)");
    }
}
