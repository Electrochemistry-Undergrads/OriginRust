//! Edit > Find in Project (F3, T95): the search of Origin 2016's "Find" window, measured on 2026-11-16
//! (`docs/spec/workbook-model.md`, "Find in Project"):
//!
//! - It looks in folder names and comments and the names and labels of graphs, books, sheets and
//!   columns (the "Metadata" types, each
//!   with its attributes under Select...; Units, F(x) and others are off by default), in the cells of
//!   columns when "Data in Cells (1st Matched in Column)" is ticked (off by default), and in the text
//!   objects of graphs (axis titles and legends as drawn) when "Text Objects on Graphs" is ticked (on).
//! - Each hit is a row: Range (`[Graph1]Layer1!YL`, `[Graph1]Layer1!Legend`, `[Graph1]`,
//!   `[Book1]Sheet1!B"Current density"`), Element (Graph, Book, Sheet, Column) and Attribute (Short Name,
//!   Long Name, Axis Title, Legend, Data Cells, ...) and Folder. A column gives one Data Cells hit
//!   however many of its cells match. The text is found anywhere in a field, letter case ignored
//!   unless Case Sensitive.

use std::collections::HashMap;

use sheaf_core::{ColumnId, Project, WindowId};

use crate::panels::explorer;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FindScope {
    #[default]
    EntireProject,
    ActiveFolder,
    ActiveFolderAndSubfolders,
}

impl FindScope {
    pub const ALL: [Self; 3] = [Self::EntireProject, Self::ActiveFolder, Self::ActiveFolderAndSubfolders];

    pub fn name(self) -> &'static str {
        match self {
            Self::EntireProject => "Entire Project",
            Self::ActiveFolder => "Active Folder",
            Self::ActiveFolderAndSubfolders => "Active Folder and Subfolders",
        }
    }

    fn includes_path(self, path: &[usize], active_folder: &[usize]) -> bool {
        match self {
            Self::EntireProject => true,
            Self::ActiveFolder => path == active_folder,
            Self::ActiveFolderAndSubfolders => path.starts_with(active_folder),
        }
    }
}

/// The search and the parts of the project it looks at (Origin's defaults by `Default`).
#[derive(Clone, Debug, PartialEq)]
pub struct FindSpec {
    pub text: String,
    pub within: FindScope,
    pub case_sensitive: bool,
    pub whole_word: bool,
    /// `*` any run of characters, `?` one character.
    pub wildcards: bool,
    pub folders: bool,
    pub folder_comments: bool,
    pub graphs: bool,
    pub books: bool,
    pub sheets: bool,
    pub sheet_comments: bool,
    pub columns: bool,
    /// Column attributes off by default in Origin: Units and F(x).
    pub column_units: bool,
    pub column_formula: bool,
    pub data_cells: bool,
    pub graph_text: bool,
    /// Notes windows: their names and text (T407; Sheaf's own, Origin 2026 searches notes in project files).
    pub notes: bool,
}

impl Default for FindSpec {
    fn default() -> Self {
        FindSpec {
            text: String::new(),
            within: FindScope::default(),
            case_sensitive: false,
            whole_word: false,
            wildcards: false,
            folders: true,
            folder_comments: true,
            graphs: true,
            books: true,
            sheets: true,
            sheet_comments: true,
            columns: true,
            column_units: false,
            column_formula: false,
            data_cells: false,
            graph_text: true,
            notes: true,
        }
    }
}

/// What a hit leads to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FindTarget {
    Folder(Vec<usize>),
    Window(WindowId),
    Sheet { book: WindowId, sheet: usize },
    Column { book: WindowId, sheet: usize, column: ColumnId },
}

impl FindTarget {
    pub fn window_id(&self) -> Option<WindowId> {
        match *self {
            Self::Folder(_) => None,
            Self::Window(id) => Some(id),
            Self::Sheet { book, .. } | Self::Column { book, .. } => Some(book),
        }
    }
}

/// One row of the result list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindHit {
    pub range: String,
    pub element: &'static str,
    pub attribute: &'static str,
    pub folder: String,
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
pub fn find_in_project(project: &Project, spec: &FindSpec, active_folder: &[usize]) -> Vec<FindHit> {
    let folders = explorer::folder_list(&project.root);
    let mut window_folders = HashMap::new();
    let mut hits = Vec::new();
    for (path, range) in &folders {
        let Some(folder) = explorer::folder_at(&project.root, path) else { continue };
        // Like Explorer, use the first explicit subfolder membership and fall back to the root.
        if !path.is_empty() {
            for &id in &folder.windows {
                window_folders.entry(id).or_insert((path.as_slice(), folder.name.as_str()));
            }
        }
        if spec.folders && spec.within.includes_path(path, active_folder) {
            for (attribute, value, enabled) in
                [("Name", folder.name.as_str(), true), ("Comments", folder.comments.as_str(), spec.folder_comments)]
            {
                if enabled && spec.matches(value) {
                    hits.push(FindHit {
                        range: range.clone(),
                        element: "Folder",
                        attribute,
                        folder: folder.name.clone(),
                        target: FindTarget::Folder(path.clone()),
                    });
                }
            }
        }
    }
    let folder_for = |id| {
        let (path, name) = window_folders.get(&id).copied().unwrap_or((&[], &project.root.name));
        spec.within.includes_path(path, active_folder).then_some(name)
    };
    for (g, folder) in project.graphs().filter_map(|g| folder_for(g.id).map(|folder| (g, folder))) {
        if spec.graphs {
            for (attr, text) in [("Short Name", g.name.as_str()), ("Long Name", g.long_name.as_str())] {
                if spec.matches(text) {
                    hits.push(FindHit {
                        range: format!("[{}]", g.name),
                        element: "Graph",
                        attribute: attr,
                        folder: folder.to_string(),
                        target: FindTarget::Window(g.id),
                    });
                }
            }
        }
        // Sticky notes (T421) are searched with the notes.
        if spec.notes && g.sticky.as_ref().is_some_and(|n| spec.matches(&n.text)) {
            hits.push(FindHit {
                range: format!("[{}]", g.name),
                element: "Graph",
                attribute: "Sticky Note",
                folder: folder.to_string(),
                target: FindTarget::Window(g.id),
            });
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
                            folder: folder.to_string(),
                            target: FindTarget::Window(g.id),
                        });
                    }
                }
                if layer.legend.visible && spec.matches(&legend_shown(project, g.id, i)) {
                    hits.push(FindHit {
                        range: at("Legend"),
                        element: "Graph",
                        attribute: "Legend",
                        folder: folder.to_string(),
                        target: FindTarget::Window(g.id),
                    });
                }
                for (k, label) in layer.labels.iter().enumerate() {
                    if spec.matches(&label.text) {
                        hits.push(FindHit {
                            range: at(&format!("Text{}", k + 1)),
                            element: "Graph",
                            attribute: "Text",
                            folder: folder.to_string(),
                            target: FindTarget::Window(g.id),
                        });
                    }
                }
            }
        }
    }
    for (b, folder) in project.workbooks().filter_map(|b| folder_for(b.id).map(|folder| (b, folder))) {
        if spec.books {
            for (attr, text) in [("Short Name", b.name.as_str()), ("Long Name", b.long_name.as_str())] {
                if spec.matches(text) {
                    hits.push(FindHit {
                        range: format!("[{}]", b.name),
                        element: "Book",
                        attribute: attr,
                        folder: folder.to_string(),
                        target: FindTarget::Window(b.id),
                    });
                }
            }
        }
        if spec.notes && b.sticky.as_ref().is_some_and(|n| spec.matches(&n.text)) {
            hits.push(FindHit {
                range: format!("[{}]", b.name),
                element: "Book",
                attribute: "Sticky Note",
                folder: folder.to_string(),
                target: FindTarget::Window(b.id),
            });
        }
        for (si, s) in b.sheets.iter().enumerate() {
            if spec.notes && s.sticky.as_ref().is_some_and(|n| spec.matches(&n.text)) {
                hits.push(FindHit {
                    range: format!("[{}]{}", b.name, s.name),
                    element: "Sheet",
                    attribute: "Sticky Note",
                    folder: folder.to_string(),
                    target: FindTarget::Sheet { book: b.id, sheet: si },
                });
            }
            if spec.sheets {
                for (attr, text, enabled) in [
                    ("Short Name", s.name.as_str(), true),
                    ("Long Name", s.long_name.as_str(), true),
                    ("Comments", s.comments.as_str(), spec.sheet_comments),
                ] {
                    if enabled && spec.matches(text) {
                        hits.push(FindHit {
                            range: format!("[{}]{}", b.name, s.name),
                            element: "Sheet",
                            attribute: attr,
                            folder: folder.to_string(),
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
                let mut push = |attribute| {
                    hits.push(FindHit {
                        range: range.clone(),
                        element: "Column",
                        attribute,
                        folder: folder.to_string(),
                        target: FindTarget::Column { book: b.id, sheet: si, column: c.id },
                    });
                };
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
    // Notes windows (T407): their short and long names and their text, one hit each.
    if spec.notes {
        for (n, folder) in project.notes().filter_map(|n| folder_for(n.id).map(|folder| (n, folder))) {
            for (attr, text) in
                [("Short Name", n.name.as_str()), ("Long Name", n.long_name.as_str()), ("Text", n.text.as_str())]
            {
                if spec.matches(text) {
                    hits.push(FindHit {
                        range: format!("[{}]", n.name),
                        element: "Notes",
                        attribute: attr,
                        folder: folder.to_string(),
                        target: FindTarget::Window(n.id),
                    });
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
        find_in_project(p, &spec, &[]).into_iter().map(|h| (h.range, h.element, h.attribute)).collect()
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
    fn notes_windows_are_searched_by_name_and_text_unless_switched_off() {
        let mut p = project();
        let id = p.add_notes();
        let n = p.notes_mut(id).unwrap();
        n.long_name = "Lab log".into();
        n.text = "Day 1\nThe baseline drifted after the second sweep".into();
        let name = n.name.clone();
        assert_eq!(find(&p, text("drifted")), [(format!("[{name}]"), "Notes", "Text")]);
        assert_eq!(find(&p, text("lab")), [(format!("[{name}]"), "Notes", "Long Name")]);
        let hits = find_in_project(&p, &text("drifted"), &[]);
        assert_eq!(hits[0].target, FindTarget::Window(id));
        assert!(find(&p, FindSpec { notes: false, ..text("drifted") }).is_empty());
    }

    #[test]
    fn sticky_notes_are_found_with_the_notes() {
        use sheaf_core::sticky::NoteTarget;
        let mut p = project();
        let (g, b) = (p.graphs().next().unwrap().id, p.workbooks().next().unwrap().id);
        for (t, text) in [
            (NoteTarget::Window(g), "redo the fit"),
            (NoteTarget::Window(b), "raw data, do not edit"),
            (NoteTarget::Sheet(b, 0), "fit range 2 to 4"),
        ] {
            p.add_sticky_note(t).unwrap();
            p.edit_sticky_note(t, |n| n.text = text.into()).unwrap();
        }
        let (gn, bn) = (p.graph(g).unwrap().name.clone(), p.workbook(b).unwrap().name.clone());
        assert_eq!(
            find(&p, text("fit")),
            [(format!("[{gn}]"), "Graph", "Sticky Note"), (format!("[{bn}]Sheet1"), "Sheet", "Sticky Note")]
        );
        assert_eq!(find(&p, text("raw data")), [(format!("[{bn}]"), "Book", "Sticky Note")]);
        assert!(find(&p, FindSpec { notes: false, ..text("fit") }).is_empty());
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
    fn worksheet_comments_search_the_full_text_and_respect_metadata_switches() {
        let mut p = project();
        let book = p.workbooks().next().unwrap().id;
        let mut control = sheaf_core::Worksheet::new("Control");
        control.long_name = "Baseline reference".into();
        control.comments = "Second sweep\nBaseline and baseline drift".into();
        p.workbook_mut(book).unwrap().sheets.push(control);

        let spec = FindSpec { whole_word: true, ..text("BASELINE") };
        let hits = find_in_project(&p, &spec, &[]);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].attribute, "Long Name");
        assert_eq!(hits[1].attribute, "Comments");
        assert_eq!(hits[1].element, "Sheet");
        assert_eq!(hits[1].target, FindTarget::Sheet { book, sheet: 1 });
        let names = FindSpec { sheet_comments: false, ..spec.clone() };
        assert_eq!(find_in_project(&p, &names, &[]), hits[..1]);
        let no_sheets = FindSpec { sheets: false, ..spec };
        assert!(find_in_project(&p, &no_sheets, &[]).is_empty());
    }

    #[test]
    fn folder_scope_gates_graph_and_workbook_hits_by_window_membership() {
        use sheaf_core::project::Folder;

        let mut p = project();
        let root_book = p.workbooks().next().unwrap().id;
        let graph = p.graphs().next().unwrap().id;
        let active_book = p.add_workbook("");
        let child_book = p.add_workbook("");
        let sibling_book = p.add_workbook("");
        for id in [root_book, active_book, child_book, sibling_book] {
            p.workbook_mut(id).unwrap().sheets[0].comments = "scope term".into();
        }
        p.graph_mut(graph).unwrap().long_name = "scope term".into();
        p.root.folders = vec![
            Folder {
                name: "Active".into(),
                windows: vec![active_book],
                folders: vec![Folder { name: "Child".into(), windows: vec![graph, child_book], ..Folder::default() }],
                ..Folder::default()
            },
            Folder { name: "Sibling".into(), windows: vec![sibling_book], ..Folder::default() },
        ];
        let targets = |within, path: &[usize]| {
            let spec = FindSpec { within, ..text("scope term") };
            find_in_project(&p, &spec, path).into_iter().map(|hit| hit.target).collect::<Vec<_>>()
        };
        let hits = find_in_project(&p, &text("scope term"), &[]);
        assert_eq!(
            hits.iter().map(|hit| hit.folder.as_str()).collect::<Vec<_>>(),
            ["Child", p.root.name.as_str(), "Active", "Child", "Sibling"],
        );
        let sheet = |book| FindTarget::Sheet { book, sheet: 0 };
        let all =
            [FindTarget::Window(graph), sheet(root_book), sheet(active_book), sheet(child_book), sheet(sibling_book)];
        assert_eq!(targets(FindScope::EntireProject, &[0]), all);
        assert_eq!(targets(FindScope::ActiveFolder, &[]), [sheet(root_book)]);
        assert_eq!(targets(FindScope::ActiveFolder, &[0]), [sheet(active_book)]);
        assert_eq!(
            targets(FindScope::ActiveFolderAndSubfolders, &[0]),
            [FindTarget::Window(graph), sheet(active_book), sheet(child_book)],
        );
        assert_eq!(targets(FindScope::ActiveFolderAndSubfolders, &[]), all);
    }

    #[test]
    fn folder_metadata_uses_names_and_full_comments_with_the_selected_scope() {
        use sheaf_core::project::Folder;

        let mut p = Project::new();
        p.root.name = "Survey".into();
        p.root.comments = "QC".into();
        p.root.folders = vec![
            Folder {
                name: "QC".into(),
                comments: "First line\nQC accepted".into(),
                folders: vec![Folder { name: "Repeat".into(), comments: "QC".into(), ..Folder::default() }],
                ..Folder::default()
            },
            Folder { name: "Other".into(), comments: "QC".into(), ..Folder::default() },
        ];
        let spec = text("qc");
        let all = find_in_project(&p, &spec, &[0]);
        assert_eq!(all.len(), 5);
        assert_eq!(
            all.iter().map(|hit| hit.folder.as_str()).collect::<Vec<_>>(),
            ["Survey", "QC", "QC", "Repeat", "Other"],
        );
        assert_eq!(all[0].target, FindTarget::Folder(vec![]));
        assert_eq!((&*all[1].range, all[1].element, all[1].attribute), ("/Survey/QC/", "Folder", "Name"));
        assert_eq!(all[2].attribute, "Comments");
        assert_eq!(all[2].target, FindTarget::Folder(vec![0]));
        let active = FindSpec { within: FindScope::ActiveFolder, ..spec.clone() };
        assert_eq!(find_in_project(&p, &active, &[0]), all[1..3]);
        let subtree = FindSpec { within: FindScope::ActiveFolderAndSubfolders, ..spec.clone() };
        assert_eq!(find_in_project(&p, &subtree, &[0]), all[1..4]);
        let names = FindSpec { folder_comments: false, ..spec.clone() };
        assert_eq!(find_in_project(&p, &names, &[]), all[1..2]);
        let disabled = FindSpec { folders: false, ..spec };
        assert!(find_in_project(&p, &disabled, &[]).is_empty());
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
