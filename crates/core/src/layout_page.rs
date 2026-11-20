//! Layout page windows (T237): a page that shows pictures of the project's graphs, as Origin's
//! File > New > Layout makes, for arranging graphs for print. Origin's layout page was not measured;
//! the page size and the placement of added graphs are Sheaf's choices.

use crate::ids::WindowId;
use serde::{Deserialize, Serialize};

/// What an item of a layout page shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum LayoutContent {
    /// A live picture of a graph window: it changes when the graph does.
    Graph { graph: WindowId },
}

/// One object on the page; the rectangle is in fractions of the page (0,0 the top left).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayoutItem {
    pub content: LayoutContent,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayoutPage {
    pub id: WindowId,
    /// The short name, `Layout1`, `Layout2`, ...
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub long_name: String,
    /// The page size in points.
    pub width_pt: f64,
    pub height_pt: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<LayoutItem>,
}

impl LayoutPage {
    /// Sheaf's default page: US Letter in landscape, 11 x 8.5 in.
    pub const DEFAULT_SIZE_PT: (f64, f64) = (792.0, 612.0);

    pub fn new(id: WindowId, name: impl Into<String>) -> Self {
        let (width_pt, height_pt) = Self::DEFAULT_SIZE_PT;
        LayoutPage { id, name: name.into(), long_name: String::new(), width_pt, height_pt, items: Vec::new() }
    }

    /// The window title: the short name, with the long name after it when there is one.
    pub fn title(&self) -> String {
        if self.long_name.is_empty() { self.name.clone() } else { format!("{} - {}", self.name, self.long_name) }
    }

    /// Layout > Add Graph: the picture of `graph` in the next free cell of a two-column grid, each
    /// cell keeping the graph's aspect ratio `aspect` (width / height) inside it.
    pub fn add_graph(&mut self, graph: WindowId, aspect: f64) -> usize {
        let k = self.items.len();
        let (col, row) = (k % 2, k / 2);
        let margin = 0.04;
        let cell_w = (1.0 - 3.0 * margin) / 2.0;
        let cell_h = (1.0 - 3.0 * margin) / 2.0;
        let (cx, cy) = (margin + col as f64 * (cell_w + margin), margin + (row % 2) as f64 * (cell_h + margin));
        // Fit the picture in the cell (the page's own aspect is taken into account).
        let page_aspect = self.width_pt / self.height_pt;
        let a = if aspect.is_finite() && aspect > 0.0 { aspect } else { 4.0 / 3.0 };
        let (mut w, mut h) = (cell_w, cell_w * page_aspect / a);
        if h > cell_h {
            h = cell_h;
            w = cell_h * a / page_aspect;
        }
        self.items.push(LayoutItem {
            content: LayoutContent::Graph { graph },
            x: cx + (cell_w - w) / 2.0,
            y: cy + (cell_h - h) / 2.0,
            w,
            h,
        });
        k
    }

    /// The graphs the page shows.
    pub fn graphs(&self) -> impl Iterator<Item = WindowId> + '_ {
        self.items.iter().map(|i| match i.content {
            LayoutContent::Graph { graph } => graph,
        })
    }

    /// Renumbers the windows the page names (when projects are appended).
    pub fn remap_windows(&mut self, map: &dyn Fn(WindowId) -> WindowId) {
        for it in &mut self.items {
            match &mut it.content {
                LayoutContent::Graph { graph } => *graph = map(*graph),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Project;
    use crate::template::Template;

    #[test]
    fn graphs_fill_a_two_column_grid_and_the_page_is_saved_renamed_and_duplicated() {
        let mut p = Project::new();
        let g = p.add_graph(Template::Line);
        let id = p.add_layout();
        assert_eq!(p.layout(id).unwrap().name, "Layout1");
        let l = p.layout_mut(id).unwrap();
        assert_eq!(l.add_graph(g, 4.0 / 3.0), 0);
        assert_eq!(l.add_graph(g, 4.0 / 3.0), 1);
        let (a, b) = (l.items[0], l.items[1]);
        assert!(a.x < b.x && (a.y - b.y).abs() < 1e-12, "side by side: {a:?} {b:?}");
        for it in &l.items {
            assert!(it.x >= 0.0 && it.y >= 0.0 && it.x + it.w <= 1.0 && it.y + it.h <= 1.0, "{it:?}");
            // The picture keeps the graph's 4:3 shape on the page.
            assert!((it.w * l.width_pt / (it.h * l.height_pt) - 4.0 / 3.0).abs() < 1e-9);
        }
        p.rename_window(id, "Figure").unwrap();
        assert_eq!(p.window(id).unwrap().title(), "Figure");
        let c = p.duplicate_window(id).unwrap();
        assert_eq!(p.layout(c).unwrap().items.len(), 2);
        let back: Project = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(back.layout(id), p.layout(id));
        assert_eq!(back.layout(id).unwrap().graphs().collect::<Vec<_>>(), [g, g]);
    }
}
