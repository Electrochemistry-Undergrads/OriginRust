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
}

impl ItemKind {
    pub const ALL: [ItemKind; 4] = [ItemKind::Book, ItemKind::Sheet, ItemKind::Column, ItemKind::Graph];

    pub fn name(self) -> &'static str {
        match self {
            ItemKind::Book => "Book",
            ItemKind::Sheet => "Sheet",
            ItemKind::Column => "Column",
            ItemKind::Graph => "Graph",
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

    /// Whether items of `kind` have this field: columns have all three, sheets a Long Name and Comments, books
    /// and graphs a Long Name.
    pub fn applies_to(self, kind: ItemKind) -> bool {
        match self {
            MetaField::LongName => true,
            MetaField::Units => kind == ItemKind::Column,
            MetaField::Comments => matches!(kind, ItemKind::Column | ItemKind::Sheet),
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
    pub long_name: String,
    pub units: String,
    pub comments: String,
}

impl ItemInfo {
    /// Whether `text` (ignoring case) is in any of the row's texts; an empty search matches every row.
    pub fn matches(&self, text: &str) -> bool {
        let t = text.trim().to_lowercase();
        t.is_empty()
            || [&self.name, &self.location, &self.long_name, &self.units, &self.comments]
                .iter()
                .any(|s| s.to_lowercase().contains(&t))
    }
}

impl Project {
    /// Every workbook with its sheets and columns, then every graph.
    pub fn project_items(&self) -> Vec<ItemInfo> {
        let mut out = Vec::new();
        for b in self.workbooks() {
            out.push(ItemInfo {
                item: ItemRef::Book(b.id),
                kind: ItemKind::Book,
                name: b.name.clone(),
                location: String::new(),
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
                long_name: g.long_name.clone(),
                units: String::new(),
                comments: String::new(),
            });
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
}
