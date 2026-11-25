//! Layout page windows (T237): a page that shows pictures of the project's graphs, as Origin's
//! File > New > Layout makes, for arranging graphs for print. Origin's layout page was not measured;
//! the page size and the placement of added graphs are Sheaf's choices.

use crate::color::Color;
use crate::ids::WindowId;
use serde::{Deserialize, Serialize};

/// What an item of a layout page shows.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum LayoutContent {
    /// A live picture of a graph window: it changes when the graph does.
    Graph { graph: WindowId },
    /// A live picture of sheet `sheet` (index) of workbook `book` (T237 slice 5).
    Sheet { book: WindowId, sheet: usize },
    /// Text on the page (T237 slice 8): lines in Arial, black, `size` points.
    Text { text: String, size: u16 },
    /// A drawn object (T237 slice 11) filling the item's box: its line `color` and `width` in tenths
    /// of a point; a `fill` inside a rectangle or ellipse and an arrowhead at a line's end
    /// (slice 12).
    Shape {
        shape: LayoutShape,
        color: Color,
        width: u16,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fill: Option<Color>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        arrow: bool,
    },
}

/// The drawn objects of a layout page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutShape {
    Rectangle,
    Ellipse,
    /// From the top left corner of the box to the bottom right.
    Line,
    /// From the bottom left corner of the box to the top right.
    LineUp,
    /// Across the middle of the box, left to right (T237 slice 12).
    HLine,
    /// Down the middle of the box, top to bottom (T237 slice 12).
    VLine,
}

impl LayoutShape {
    pub const ALL: [LayoutShape; 6] = [
        LayoutShape::Rectangle,
        LayoutShape::Ellipse,
        LayoutShape::HLine,
        LayoutShape::VLine,
        LayoutShape::Line,
        LayoutShape::LineUp,
    ];

    /// A rectangle or an ellipse, which can be filled; the others are lines, which can have an
    /// arrowhead.
    pub fn closed(self) -> bool {
        matches!(self, LayoutShape::Rectangle | LayoutShape::Ellipse)
    }

    pub fn label(self) -> &'static str {
        match self {
            LayoutShape::Rectangle => "Rectangle",
            LayoutShape::Ellipse => "Ellipse",
            LayoutShape::Line => "Line (down)",
            LayoutShape::LineUp => "Line (up)",
            LayoutShape::HLine => "Line (horizontal)",
            LayoutShape::VLine => "Line (vertical)",
        }
    }
}

impl LayoutContent {
    /// The window the item shows; `None` for text.
    pub fn window(&self) -> Option<WindowId> {
        match *self {
            LayoutContent::Graph { graph } => Some(graph),
            LayoutContent::Sheet { book, .. } => Some(book),
            LayoutContent::Text { .. } | LayoutContent::Shape { .. } => None,
        }
    }
}

/// One object on the page; the rectangle is in fractions of the page (0,0 the top left).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    /// The page colour (Format > Layout Page..., T237 slice 6); white by default.
    #[serde(default = "white", skip_serializing_if = "is_white")]
    pub background: Color,
    /// The layout page's sticky note (T460).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sticky: Option<crate::sticky::StickyNote>,
}

fn white() -> Color {
    Color::WHITE
}

fn is_white(c: &Color) -> bool {
    *c == Color::WHITE
}

impl LayoutPage {
    /// The default page, as Origin 2016's new layout page (T274, measured 2026-11-21: `page.width`
    /// 6400 and `page.height` 4800 at `page.resx` 600 dpi), 10.667 x 8 in.
    pub const DEFAULT_SIZE_PT: (f64, f64) = (768.0, 576.0);

    pub fn new(id: WindowId, name: impl Into<String>) -> Self {
        let (width_pt, height_pt) = Self::DEFAULT_SIZE_PT;
        LayoutPage {
            id,
            name: name.into(),
            long_name: String::new(),
            width_pt,
            height_pt,
            items: Vec::new(),
            background: Color::WHITE,
            sticky: None,
        }
    }

    /// The window title: the short name, with the long name after it when there is one.
    pub fn title(&self) -> String {
        if self.long_name.is_empty() { self.name.clone() } else { format!("{} - {}", self.name, self.long_name) }
    }

    /// Layout > Add Graph: the picture of `graph` in the next free cell of a two-column grid, each
    /// cell keeping the graph's aspect ratio `aspect` (width / height) inside it.
    pub fn add_graph(&mut self, graph: WindowId, aspect: f64) -> usize {
        self.add(LayoutContent::Graph { graph }, aspect)
    }

    /// Layout > Add Worksheet: the picture of a sheet, placed as [`LayoutPage::add_graph`] places a
    /// graph.
    pub fn add_sheet(&mut self, book: WindowId, sheet: usize, aspect: f64) -> usize {
        self.add(LayoutContent::Sheet { book, sheet }, aspect)
    }

    /// Layout > Add Text: `text` at its own size (`size_pt` = its box in points), centred at the top
    /// of the page.
    pub fn add_text(&mut self, text: String, size: u16, size_pt: (f64, f64)) -> usize {
        let (w, h) = ((size_pt.0 / self.width_pt).min(1.0), (size_pt.1 / self.height_pt).min(1.0));
        self.items.push(LayoutItem { content: LayoutContent::Text { text, size }, x: (1.0 - w) / 2.0, y: 0.04, w, h });
        self.items.len() - 1
    }

    /// Layout > Add Rectangle / Ellipse / Line / Arrow: a black 1 pt `shape` (with an arrowhead when
    /// `arrow`) in the middle of the page, 30 % of its width and 20 % of its height.
    pub fn add_shape(&mut self, shape: LayoutShape, arrow: bool) -> usize {
        let content = LayoutContent::Shape { shape, color: Color::BLACK, width: 10, fill: None, arrow };
        self.items.push(LayoutItem { content, x: 0.35, y: 0.4, w: 0.3, h: 0.2 });
        self.items.len() - 1
    }

    /// Puts `content` in the next free cell of the two-column grid, keeping `aspect`.
    pub fn add(&mut self, content: LayoutContent, aspect: f64) -> usize {
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
        self.items.push(LayoutItem { content, x: cx + (cell_w - w) / 2.0, y: cy + (cell_h - h) / 2.0, w, h });
        k
    }

    /// The graphs the page shows.
    pub fn graphs(&self) -> impl Iterator<Item = WindowId> + '_ {
        self.items.iter().filter_map(|i| match i.content {
            LayoutContent::Graph { graph } => Some(graph),
            LayoutContent::Sheet { .. } | LayoutContent::Text { .. } | LayoutContent::Shape { .. } => None,
        })
    }

    /// Renumbers the windows the page names (when projects are appended).
    pub fn remap_windows(&mut self, map: &dyn Fn(WindowId) -> WindowId) {
        for it in &mut self.items {
            match &mut it.content {
                LayoutContent::Graph { graph } => *graph = map(*graph),
                LayoutContent::Sheet { book, .. } => *book = map(*book),
                LayoutContent::Text { .. } | LayoutContent::Shape { .. } => {}
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
        let l = p.layout(id).unwrap();
        assert_eq!((l.width_pt, l.height_pt), (768.0, 576.0), "Origin 2016's 6400 x 4800 at 600 dpi");
        // The placements below are checked on an 11 x 8.5 in page.
        let l = p.layout_mut(id).unwrap();
        (l.width_pt, l.height_pt) = (792.0, 612.0);
        let l = p.layout_mut(id).unwrap();
        assert_eq!(l.add_graph(g, 4.0 / 3.0), 0);
        assert_eq!(l.add_graph(g, 4.0 / 3.0), 1);
        let (a, b) = (l.items[0].clone(), l.items[1].clone());
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
        let json = serde_json::to_string(&p).unwrap();
        let page_json = serde_json::to_string(p.layout(id).unwrap()).unwrap();
        assert!(!page_json.contains("background"), "a white page leaves it out");
        let back: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(back.layout(id), p.layout(id));
        assert_eq!(back.layout(id).unwrap().graphs().collect::<Vec<_>>(), [g, g]);
        // A worksheet item goes in the next cell, is saved, and is not one of the graphs.
        let book = p.add_workbook("Book1");
        let l = p.layout_mut(id).unwrap();
        assert_eq!(l.add_sheet(book, 0, 2.0), 2);
        assert_eq!(l.items[2].content, super::LayoutContent::Sheet { book, sheet: 0 });
        assert_eq!(l.items[2].content.window(), Some(book));
        // Text sits at its own size, centred at the top.
        assert_eq!(l.add_text("Figure 1".into(), 12, (79.2, 30.6)), 3);
        let t = &l.items[3];
        assert!((t.w - 0.1).abs() < 1e-12 && (t.h - 0.05).abs() < 1e-12 && (t.x - 0.45).abs() < 1e-12, "{t:?}");
        assert_eq!(t.content.window(), None);
        assert_eq!(l.add_shape(super::LayoutShape::Ellipse, false), 4);
        assert_eq!(
            l.items[4].content,
            super::LayoutContent::Shape {
                shape: super::LayoutShape::Ellipse,
                color: crate::Color::BLACK,
                width: 10,
                fill: None,
                arrow: false
            }
        );
        // A drawn object saved before fills and arrows (slice 11) still loads.
        let old = r##"{"kind":"shape","shape":"line","color":"#000000","width":10}"##;
        let back: super::LayoutContent = serde_json::from_str(old).unwrap();
        assert!(matches!(back, super::LayoutContent::Shape { fill: None, arrow: false, .. }), "{back:?}");
        l.background = crate::Color::rgb(255, 255, 200);
        assert_eq!(l.graphs().count(), 2);
        let back: Project = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(back.layout(id), p.layout(id));
    }
}
