//! Edit > Find in Project (F3, T95): the search of Origin 2016's "Find" window, measured on 2026-11-16
//! (`docs/spec/workbook-model.md`, "Find in Project"):
//!
//! - It looks in the names and labels of graphs, books, sheets and columns (the "Metadata" types, each
//!   with its attributes under Select...; Units, F(x) and others are off by default), in the cells of
//!   columns when "Data in Cells (1st Matched in Column)" is ticked (off by default), and in the text
//!   objects of graphs (axis titles and legends as drawn) when "Text Objects on Graphs" is ticked (on).
//! - Each hit is a row: Range (`[Graph1]Layer1!YL`, `[Graph1]Layer1!Legend`, `[Graph1]`,
//!   `[Book1]Sheet1!B"Current density"`), Element (Graph, Book, Sheet, Column) and Attribute (Short Name,
//!   Long Name, Axis Title, Legend, Data Cells, ...). A column gives one Data Cells hit however many of
//!   its cells match. The text is found anywhere in a field, letter case ignored unless Case Sensitive.

use sheaf_core::{ColumnId, Project, WindowId};

/// The search and the parts of the project it looks at (Origin's defaults by `Default`).
#[derive(Clone, Debug, PartialEq)]
pub struct FindSpec {
    pub text: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    /// `*` any run of characters, `?` one character.
    pub wildcards: bool,
    pub graphs: bool,
    pub books: bool,
    pub sheets: bool,
    pub columns: bool,
    /// Column attributes off by default in Origin: Units and F(x).
    pub column_units: bool,
    pub column_formula: bool,
    pub data_cells: bool,
    pub graph_text: bool,
}

impl Default for FindSpec {
    fn default() -> Self {
        FindSpec {
            text: String::new(),
            case_sensitive: false,
            whole_word: false,
            wildcards: false,
            graphs: true,
            books: true,
            sheets: true,
            columns: true,
            column_units: false,
            column_formula: false,
            data_cells: false,
            graph_text: true,
        }
    }
}

/// What a hit leads to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindTarget {
    Window(WindowId),
    Sheet { book: WindowId, sheet: usize },
    Column { book: WindowId, sheet: usize, column: ColumnId },
}

/// One row of the result list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindHit {
    pub range: String,
    pub element: &'static str,
    pub attribute: &'static str,
    pub target: FindTarget,
}

impl FindSpec {
    /// Whether `field` contains the text (as the options ask).
    pub fn matches(&self, field: &str) -> bool {
        if self.text.is_empty() || field.is_empty() {
            return false;
        }
        let (field, pat) = if self.case_sensitive {
            (field.to_string(), self.text.clone())
        } else {
            (field.to_lowercase(), self.text.to_lowercase())
        };
        let f: Vec<char> = field.chars().collect();
        let p: Vec<char> = pat.chars().collect();
        (0..f.len()).any(|start| match match_at(&f, start, &p, self.wildcards) {
            Some(end) => !self.whole_word || (word_starts(&f, start) && word_ends(&f, end)),
            None => false,
        })
    }
}

/// The end of a match of `p` starting at `f[start]`, if there is one (wildcards matched lazily).
fn match_at(f: &[char], start: usize, p: &[char], wildcards: bool) -> Option<usize> {
    fn go(f: &[char], i: usize, p: &[char], j: usize, wild: bool) -> Option<usize> {
        if j == p.len() {
            return Some(i);
        }
        if wild && p[j] == '*' {
            return (i..=f.len()).find_map(|k| go(f, k, p, j + 1, wild));
        }
        if i < f.len() && ((wild && p[j] == '?') || f[i] == p[j]) {
            return go(f, i + 1, p, j + 1, wild);
        }
        None
    }
    go(f, start, p, 0, wildcards)
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// No word character just before `at`.
fn word_starts(f: &[char], at: usize) -> bool {
    at.checked_sub(1).and_then(|i| f.get(i)).is_none_or(|c| !is_word(*c))
}

/// No word character at `at` (just after a match).
fn word_ends(f: &[char], at: usize) -> bool {
    f.get(at).is_none_or(|c| !is_word(*c))
}

/// Searches the project as Origin's Find in Project does.
pub fn find_in_project(project: &Project, spec: &FindSpec) -> Vec<FindHit> {
    let mut hits = Vec::new();
    for g in project.graphs() {
        if spec.graphs {
            for (attr, text) in [("Short Name", g.name.as_str()), ("Long Name", g.long_name.as_str())] {
                if spec.matches(text) {
                    hits.push(FindHit {
                        range: format!("[{}]", g.name),
                        element: "Graph",
                        attribute: attr,
                        target: FindTarget::Window(g.id),
                    });
                }
            }
        }
        if spec.graph_text {
            for (i, layer) in g.layers.iter().enumerate() {
                let at = |what: &str| format!("[{}]Layer{}!{what}", g.name, i + 1);
                for (vertical, code) in [(false, "XB"), (true, "YL")] {
                    let title = sheaf_plot::axis_title(project, g.id, i, vertical).unwrap_or_default();
                    if spec.matches(&title) {
                        hits.push(FindHit {
                            range: at(code),
                            element: "Graph",
                            attribute: "Axis Title",
                            target: FindTarget::Window(g.id),
                        });
                    }
                }
                if layer.legend.visible && spec.matches(&legend_shown(project, g.id, i)) {
                    hits.push(FindHit {
                        range: at("Legend"),
                        element: "Graph",
                        attribute: "Legend",
                        target: FindTarget::Window(g.id),
                    });
                }
                for (k, label) in layer.labels.iter().enumerate() {
                    if spec.matches(&label.text) {
                        hits.push(FindHit {
                            range: at(&format!("Text{}", k + 1)),
                            element: "Graph",
                            attribute: "Text",
                            target: FindTarget::Window(g.id),
                        });
                    }
                }
            }
        }
    }
    for b in project.workbooks() {
        if spec.books {
            for (attr, text) in [("Short Name", b.name.as_str()), ("Long Name", b.long_name.as_str())] {
                if spec.matches(text) {
                    hits.push(FindHit {
                        range: format!("[{}]", b.name),
                        element: "Book",
                        attribute: attr,
                        target: FindTarget::Window(b.id),
                    });
                }
            }
        }
        for (si, s) in b.sheets.iter().enumerate() {
            if spec.sheets {
                for (attr, text) in [("Short Name", s.name.as_str()), ("Long Name", s.long_name.as_str())] {
                    if spec.matches(text) {
                        hits.push(FindHit {
                            range: format!("[{}]{}", b.name, s.name),
                            element: "Sheet",
                            attribute: attr,
                            target: FindTarget::Sheet { book: b.id, sheet: si },
                        });
                    }
                }
            }
            for c in &s.columns {
                let range = if c.long_name.is_empty() {
                    format!("[{}]{}!{}", b.name, s.name, c.short_name)
                } else {
                    format!("[{}]{}!{}\"{}\"", b.name, s.name, c.short_name, c.long_name)
                };
                let target = FindTarget::Column { book: b.id, sheet: si, column: c.id };
                let mut push =
                    |attribute| hits.push(FindHit { range: range.clone(), element: "Column", attribute, target });
                if spec.columns {
                    let formula = c.formula.clone().unwrap_or_default();
                    let mut fields: Vec<(&'static str, &str, bool)> = vec![
                        ("Short Name", c.short_name.as_str(), true),
                        ("Long Name", c.long_name.as_str(), true),
                        ("Comments", c.comments.as_str(), true),
                        ("Units", c.units.as_str(), spec.column_units),
                        ("F(x)", formula.as_str(), spec.column_formula),
                    ];
                    fields.extend(c.params.values().map(|v| ("User Parameters", v.as_str(), true)));
                    let mut seen = Vec::new();
                    for (attr, text, on) in fields {
                        if on && !seen.contains(&attr) && spec.matches(text) {
                            seen.push(attr);
                            push(attr);
                        }
                    }
                }
                if spec.data_cells && (0..c.len()).any(|r| spec.matches(&c.cell_text(r))) {
                    push("Data Cells");
                }
            }
        }
    }
    hits
}

/// The legend as drawn: its own text, or the names of the plots it lists, with `%(n)` replaced by
/// the n-th plot's name.
fn legend_shown(project: &Project, graph: WindowId, layer: usize) -> String {
    let Some(l) = project.graph(graph).and_then(|g| g.layers.get(layer)) else { return String::new() };
    let names: Vec<String> =
        l.plots.iter().map(|p| project.column(p.y).map(|c| c.display_name().to_string()).unwrap_or_default()).collect();
    let mut text = sheaf_plot::legend_text(project, graph, layer).unwrap_or_default();
    for (i, n) in names.iter().enumerate().rev() {
        text = text.replace(&format!("%({})", i + 1), n);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::template::Template;
    use sheaf_core::{ColumnData, Designation};

    /// Origin's probe project: Book1 with A 1..4 and B 10..40 (Current density, mA), a line graph of B.
    fn project() -> Project {
        let mut p = Project::new();
        let b = p.add_workbook("");
        p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0])).unwrap();
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![10.0, 20.0, 30.0, 40.0])).unwrap();
        p.column_mut(y).unwrap().long_name = "Current density".into();
        p.column_mut(y).unwrap().units = "mA".into();
        p.plot_columns(Template::Line, &[y]).unwrap();
        p
    }

    fn find(p: &Project, spec: FindSpec) -> Vec<(String, &'static str, &'static str)> {
        find_in_project(p, &spec).into_iter().map(|h| (h.range, h.element, h.attribute)).collect()
    }

    fn text(t: &str) -> FindSpec {
        FindSpec { text: t.into(), ..FindSpec::default() }
    }

    #[test]
    fn density_is_found_where_origin_found_it() {
        let p = project();
        let g = p.graphs().next().unwrap().name.clone();
        let b = p.workbooks().next().unwrap().name.clone();
        // Origin: the Y axis title, the legend and column B's long name (three hits).
        assert_eq!(
            find(&p, text("density")),
            [
                (format!("[{g}]Layer1!YL"), "Graph", "Axis Title"),
                (format!("[{g}]Layer1!Legend"), "Graph", "Legend"),
                (format!("[{b}]Sheet1!B\"Current density\""), "Column", "Long Name"),
            ]
        );
    }

    #[test]
    fn names_units_and_cells_follow_the_switches() {
        let p = project();
        let g = p.graphs().next().unwrap().name.clone();
        assert_eq!(find(&p, text("Graph")), [(format!("[{g}]"), "Graph", "Short Name")]);
        // Units are off by default in Origin's Select Metadata, so "mA" finds nothing but the axis title.
        assert!(find(&p, text("mA")).iter().all(|h| h.2 == "Axis Title"));
        let with_units = FindSpec { column_units: true, ..text("mA") };
        assert!(find(&p, with_units).iter().any(|h| h.2 == "Units"));
        // Data in Cells: one hit for the column, not per cell.
        assert!(find(&p, text("30")).is_empty(), "off by default");
        let cells = FindSpec { data_cells: true, ..text("0") };
        assert_eq!(find(&p, cells).iter().filter(|h| h.2 == "Data Cells").count(), 1);
    }

    #[test]
    fn case_whole_word_and_wildcards() {
        let s = FindSpec { case_sensitive: true, ..text("Density") };
        assert!(!s.matches("Current density"));
        assert!(text("Density").matches("Current density"));
        let w = FindSpec { whole_word: true, ..text("dens") };
        assert!(!w.matches("Current density"));
        assert!(FindSpec { whole_word: true, ..text("density") }.matches("Current density"));
        let wild = FindSpec { wildcards: true, ..text("cur*ty") };
        assert!(wild.matches("Current density"));
        assert!(FindSpec { wildcards: true, ..text("de?sity") }.matches("Current density"));
        assert!(!text("cur*ty").matches("Current density"), "a star is a star without wildcards");
    }
}
