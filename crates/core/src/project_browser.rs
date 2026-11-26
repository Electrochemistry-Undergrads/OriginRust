//! View > Project Browser (T433; Origin 2025b's Project Browser, Sheaf's own design): one list of the project's
//! workbooks, sheets, columns and graphs with their metadata, searched as you type, from which an item is opened
//! or the Long Name, Units or Comments of many items are set at once.
//!
//! Sheaf's choices (Origin 2025b not measured here, Origin 2016 has none): books, sheets and columns in project
//! order and then graphs; a search matches any of the name, location, Long Name, Units and Comments, ignoring
//! case; a field an item does not have (a graph's Units) is left alone.

use crate::ids::{ColumnId, WindowId};
use crate::project::Project;

/// What a row of the browser is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ItemKind {
    Book,
    Sheet,
    Column,
    Graph,
    /// A matrix book (T486).
    Matrix,
    /// A sheet of a matrix book (T486); its Long Name, Units and Comments are those of its values (Z).
    MatrixSheet,
}

impl ItemKind {
    pub const ALL: [ItemKind; 6] =
        [ItemKind::Book, ItemKind::Sheet, ItemKind::Column, ItemKind::Graph, ItemKind::Matrix, ItemKind::MatrixSheet];

    pub fn name(self) -> &'static str {
        match self {
            ItemKind::Book => "Book",
            ItemKind::Sheet => "Sheet",
            ItemKind::Column => "Column",
            ItemKind::Graph => "Graph",
            ItemKind::Matrix => "Matrix",
            ItemKind::MatrixSheet => "Matrix Sheet",
        }
    }
}

/// An item of the project.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ItemRef {
    Book(WindowId),
    Sheet(WindowId, usize),
    Column(ColumnId),
    Graph(WindowId),
    Matrix(WindowId),
    MatrixSheet(WindowId, usize),
}

/// The metadata the browser shows and sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetaField {
    LongName,
    Units,
    Comments,
}

impl MetaField {
    pub const ALL: [MetaField; 3] = [MetaField::LongName, MetaField::Units, MetaField::Comments];

    pub fn name(self) -> &'static str {
        match self {
            MetaField::LongName => "Long Name",
            MetaField::Units => "Units",
            MetaField::Comments => "Comments",
        }
    }

    /// Whether items of `kind` have this field: columns and matrix sheets have all three, sheets a Long Name and
    /// Comments, books, matrix books and graphs a Long Name.
    pub fn applies_to(self, kind: ItemKind) -> bool {
        match self {
            MetaField::LongName => true,
            MetaField::Units => matches!(kind, ItemKind::Column | ItemKind::MatrixSheet),
            MetaField::Comments => matches!(kind, ItemKind::Column | ItemKind::Sheet | ItemKind::MatrixSheet),
        }
    }
}

/// One row of the browser.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemInfo {
    pub item: ItemRef,
    pub kind: ItemKind,
    /// The short name.
    pub name: String,
    /// Where it is: `[Book1]Sheet1` for a column, `[Book1]` for a sheet, empty for a book or graph.
    pub location: String,
    /// The project folder of its window, as names from the root joined by `/` (`Raw/Day 2`); empty in the root
    /// folder (T486).
    pub folder: String,
    pub long_name: String,
    pub units: String,
    pub comments: String,
}

impl ItemInfo {
    /// Whether `text` (ignoring case) is in any of the row's texts; an empty search matches every row.
    pub fn matches(&self, text: &str) -> bool {
        let t = text.trim().to_lowercase();
        t.is_empty()
            || [&self.name, &self.location, &self.folder, &self.long_name, &self.units, &self.comments]
                .iter()
                .any(|s| s.to_lowercase().contains(&t))
    }
}

impl Project {
    /// The project folder of window `id` as folder names joined by `/`; empty for the root folder (T486).
    pub fn folder_text(&self, id: WindowId) -> String {
        let mut names = Vec::new();
        let mut f = &self.root;
        for i in self.folder_of(id) {
            let Some(sub) = f.folders.get(i) else { break };
            names.push(sub.name.as_str());
            f = sub;
        }
        names.join("/")
    }

    /// Every workbook with its sheets and columns, then every graph, then every matrix book with its sheets.
    pub fn project_items(&self) -> Vec<ItemInfo> {
        let mut out = Vec::new();
        for b in self.workbooks() {
            let folder = self.folder_text(b.id);
            out.push(ItemInfo {
                item: ItemRef::Book(b.id),
                kind: ItemKind::Book,
                name: b.name.clone(),
                location: String::new(),
                folder: folder.clone(),
                long_name: b.long_name.clone(),
                units: String::new(),
                comments: String::new(),
            });
            for (i, s) in b.sheets.iter().enumerate() {
                out.push(ItemInfo {
                    item: ItemRef::Sheet(b.id, i),
                    kind: ItemKind::Sheet,
                    name: s.name.clone(),
                    location: format!("[{}]", b.name),
                    folder: folder.clone(),
                    long_name: s.long_name.clone(),
                    units: String::new(),
                    comments: s.comments.clone(),
                });
                for c in &s.columns {
                    out.push(ItemInfo {
                        item: ItemRef::Column(c.id),
                        kind: ItemKind::Column,
                        name: c.short_name.clone(),
                        location: format!("[{}]{}", b.name, s.name),
                        folder: folder.clone(),
                        long_name: c.long_name.clone(),
                        units: c.units.clone(),
                        comments: c.comments.clone(),
                    });
                }
            }
        }
        for g in self.graphs() {
            out.push(ItemInfo {
                item: ItemRef::Graph(g.id),
                kind: ItemKind::Graph,
                name: g.name.clone(),
                location: String::new(),
                folder: self.folder_text(g.id),
                long_name: g.long_name.clone(),
                units: String::new(),
                comments: String::new(),
            });
        }
        for m in self.matrix_books() {
            let folder = self.folder_text(m.id);
            out.push(ItemInfo {
                item: ItemRef::Matrix(m.id),
                kind: ItemKind::Matrix,
                name: m.name.clone(),
                location: String::new(),
                folder: folder.clone(),
                long_name: m.long_name.clone(),
                units: String::new(),
                comments: String::new(),
            });
            for (i, s) in m.sheets.iter().enumerate() {
                out.push(ItemInfo {
                    item: ItemRef::MatrixSheet(m.id, i),
                    kind: ItemKind::MatrixSheet,
                    name: s.name.clone(),
                    location: format!("[{}]", m.name),
                    folder: folder.clone(),
                    long_name: s.z.long_name.clone(),
                    units: s.z.units.clone(),
                    comments: s.z.comments.clone(),
                });
            }
        }
        out
    }

    /// Sets `field` to `value` on each of `items` that has the field; returns how many changed.
    pub fn set_metadata(&mut self, items: &[ItemRef], field: MetaField, value: &str) -> usize {
        let mut changed = 0;
        let mut set = |slot: &mut String| {
            if slot != value {
                *slot = value.to_string();
                changed += 1;
            }
        };
        for &item in items {
            match (item, field) {
                (ItemRef::Book(id), MetaField::LongName) => {
                    if let Some(b) = self.workbook_mut(id) {
                        set(&mut b.long_name);
                    }
                }
                (ItemRef::Graph(id), MetaField::LongName) => {
                    if let Some(g) = self.graph_mut(id) {
                        set(&mut g.long_name);
                    }
                }
                (ItemRef::Matrix(id), MetaField::LongName) => {
                    if let Some(m) = self.matrix_book_mut(id) {
                        set(&mut m.long_name);
                    }
                }
                (ItemRef::MatrixSheet(id, i), _) => {
                    if let Some(s) = self.matrix_book_mut(id).and_then(|m| m.sheets.get_mut(i)) {
                        set(match field {
                            MetaField::LongName => &mut s.z.long_name,
                            MetaField::Units => &mut s.z.units,
                            MetaField::Comments => &mut s.z.comments,
                        });
                    }
                }
                (ItemRef::Sheet(id, i), MetaField::LongName | MetaField::Comments) => {
                    if let Some(s) = self.workbook_mut(id).and_then(|b| b.sheets.get_mut(i)) {
                        set(if field == MetaField::LongName { &mut s.long_name } else { &mut s.comments });
                    }
                }
                (ItemRef::Column(id), _) => {
                    if let Some(c) = self.column_mut(id) {
                        set(match field {
                            MetaField::LongName => &mut c.long_name,
                            MetaField::Units => &mut c.units,
                            MetaField::Comments => &mut c.comments,
                        });
                    }
                }
                _ => {}
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};
    use crate::template::Template;

    #[test]
    fn the_browser_lists_everything_and_sets_metadata_on_many_items() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0])).unwrap();
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![2.0])).unwrap();
        let g = p.plot_columns(Template::Line, &[y]).unwrap();
        let items = p.project_items();
        let kinds: Vec<ItemKind> = items.iter().map(|i| i.kind).collect();
        use ItemKind::*;
        assert_eq!(kinds, [Book, Sheet, Column, Column, Graph]);
        assert_eq!(items[2].location, format!("[{}]Sheet1", p.workbook(b).unwrap().name));
        // Units on both columns and the graph: the graph has none, so two change.
        let picked = [ItemRef::Column(x), ItemRef::Column(y), ItemRef::Graph(g)];
        assert_eq!(p.set_metadata(&picked, MetaField::Units, "mA"), 2);
        assert_eq!(p.set_metadata(&picked, MetaField::Units, "mA"), 0, "nothing new");
        assert_eq!(p.set_metadata(&picked, MetaField::LongName, "Current"), 3);
        assert_eq!(p.graph(g).unwrap().long_name, "Current");
        assert_eq!(p.set_metadata(&[ItemRef::Sheet(b, 0)], MetaField::Comments, "run 3"), 1);
        let items = p.project_items();
        assert_eq!(items.iter().filter(|i| i.matches("MA")).count(), 2, "units, ignoring case");
        assert_eq!(items.iter().filter(|i| i.matches("run 3")).count(), 1);
        assert_eq!(items.iter().filter(|i| i.matches("")).count(), 5);
        assert!(MetaField::Comments.applies_to(Sheet) && !MetaField::Units.applies_to(Graph));
    }

    #[test]
    fn the_browser_shows_folders_and_matrices() {
        // T486
        let mut p = Project::new();
        let b = p.add_workbook("");
        p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap();
        let m = p.add_matrix_book("Surface");
        p.root.folders.push(crate::project::Folder {
            name: "Raw".into(),
            folders: vec![crate::project::Folder { name: "Day 2".into(), windows: vec![m], ..Default::default() }],
            ..Default::default()
        });
        let items = p.project_items();
        let kinds: Vec<ItemKind> = items.iter().map(|i| i.kind).collect();
        use ItemKind::*;
        assert_eq!(kinds, [Book, Sheet, Column, Matrix, MatrixSheet]);
        assert_eq!((items[0].folder.as_str(), items[3].folder.as_str()), ("", "Raw/Day 2"));
        assert_eq!(items[4].location, format!("[{}]", p.matrix_book(m).unwrap().name));
        assert_eq!(items.iter().filter(|i| i.matches("day 2")).count(), 2, "the folder is searched");
        // a matrix sheet's metadata is its Z labels
        let picked = [ItemRef::MatrixSheet(m, 0), ItemRef::Matrix(m)];
        assert_eq!(p.set_metadata(&picked, MetaField::Units, "nm"), 1, "the book has no units");
        assert_eq!(p.set_metadata(&picked, MetaField::LongName, "Height"), 2);
        let book = p.matrix_book(m).unwrap();
        assert_eq!((book.sheets[0].z.units.as_str(), book.sheets[0].z.long_name.as_str()), ("nm", "Height"));
        assert_eq!(book.long_name, "Height");
        assert!(MetaField::Units.applies_to(MatrixSheet) && !MetaField::Units.applies_to(Matrix));
    }
}
