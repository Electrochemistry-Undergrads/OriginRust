//! Cause-and-effect (fishbone, Ishikawa) diagram (T375, Origin 2026): a graph drawn from a sheet where each
//! chosen column is a category (its long name, else its short name) and its non-empty cells are the causes.
//! The diagram is made of ordinary drawn objects and text labels of one layer, so every part can be moved,
//! restyled or edited afterwards like any other object. Sheaf's layout (Origin's was not measured):
//!
//! - a horizontal spine with an arrow head, the effect's text right of the head;
//! - one slanted rib per category, alternately above and below the spine (the first above), evenly spaced
//!   along it, with the category's name (bold) at the rib's outer end;
//! - one short horizontal twig per cause, evenly spaced along its rib, with the cause's text starting at the
//!   twig's left end, just above it.
//!
//! Positions are fractions of the layer frame (0, 0 top left). Rules: `docs/spec/graphing.md`, "Cause-and-effect".

use crate::graph::{Font, TextLabel};
use crate::ids::ColumnId;
use crate::project::{Project, ProjectError};
use crate::shape::{Shape, ShapeKind};
use crate::template::Template;
use crate::{Color, WindowId};

/// The effect's text when none is given.
pub const DEFAULT_EFFECT: &str = "Effect";

/// The spine's y, start and end (the arrow head's tip) in the layer frame.
const SPINE_Y: f64 = 0.5;
const SPINE_START: f64 = 0.02;
const SPINE_END: f64 = 0.80;
/// How far a rib's outer end lies left of where it meets the spine, and how far it lies from the frame's edge.
const RIB_SLANT: f64 = 0.10;
const RIB_EDGE: f64 = 0.10;

/// One category and its causes.
#[derive(Debug, Clone, PartialEq)]
pub struct Category {
    pub name: String,
    pub causes: Vec<String>,
}

/// The objects of a fishbone diagram: lines and arrows, and text labels.
#[derive(Debug, Clone, PartialEq)]
pub struct FishboneLayout {
    pub shapes: Vec<Shape>,
    pub labels: Vec<TextLabel>,
}

fn line(kind: ShapeKind, a: (f64, f64), b: (f64, f64), width: f64) -> Shape {
    let mut s = Shape::new(kind, a, b);
    s.color = Color::BLACK;
    s.width_pt = width;
    s
}

/// Lays out the diagram of `categories` with the effect `effect`.
pub fn layout(effect: &str, categories: &[Category]) -> FishboneLayout {
    let mut shapes = vec![line(ShapeKind::Arrow, (SPINE_START, SPINE_Y), (SPINE_END, SPINE_Y), 2.0)];
    let mut effect_font = Font::new("Arial", 14.0);
    effect_font.bold = true;
    let mut labels = vec![TextLabel::new(effect, SPINE_END + 0.015, SPINE_Y, effect_font)];
    let per_side = categories.len().div_ceil(2).max(1);
    // Ribs meet the spine at evenly spaced points, the last one some way before the head.
    let step = (SPINE_END - SPINE_START - 0.04) / per_side as f64;
    let mut name_font = Font::new("Arial", 11.0);
    name_font.bold = true;
    let cause_font = Font::new("Arial", 9.0);
    for (k, cat) in categories.iter().enumerate() {
        let (slot, above) = (k / 2, k % 2 == 0);
        let meet = SPINE_START + 0.02 + step * (slot as f64 + 1.0);
        let outer_y = if above { RIB_EDGE } else { 1.0 - RIB_EDGE };
        let outer = (meet - RIB_SLANT, outer_y);
        shapes.push(line(ShapeKind::Line, outer, (meet, SPINE_Y), 1.0));
        let name_y = if above { outer_y - 0.04 } else { outer_y + 0.04 };
        labels.push(TextLabel::new(cat.name.clone(), outer.0 - 0.02, name_y, name_font.clone()));
        let n = cat.causes.len();
        // Twigs are as long as most of the space between two ribs.
        let twig = step * 0.75;
        for (j, cause) in cat.causes.iter().enumerate() {
            // From the outer end (t = 0) to the spine (t = 1), leaving both ends free.
            let t = (j as f64 + 1.0) / (n as f64 + 1.0);
            let at = (outer.0 + (meet - outer.0) * t, outer.1 + (SPINE_Y - outer.1) * t);
            shapes.push(line(ShapeKind::Line, (at.0 - twig, at.1), at, 0.75));
            labels.push(TextLabel::new(cause.clone(), at.0 - twig, at.1 - 0.022, cause_font.clone()));
        }
    }
    FishboneLayout { shapes, labels }
}

impl Project {
    /// A new cause-and-effect graph from `columns`: each column is a category named by its long name (else its
    /// short name), whose non-empty cells (as shown) are the causes. `effect` is the text at the head.
    pub fn plot_fishbone(&mut self, columns: &[ColumnId], effect: &str) -> Result<WindowId, ProjectError> {
        let mut categories = Vec::new();
        for &id in columns {
            let c = self.column(id).ok_or(ProjectError::NoSuchColumn(id))?;
            let name = if c.long_name.is_empty() { c.short_name.clone() } else { c.long_name.clone() };
            let causes: Vec<String> =
                (0..c.len()).map(|r| c.shown_text(r).trim().to_string()).filter(|t| !t.is_empty()).collect();
            categories.push(Category { name, causes });
        }
        if categories.is_empty() {
            return Err(ProjectError::FishboneColumns);
        }
        let book_title = self.locate(columns[0]).map(|(b, _, _)| b.title_for_graph()).unwrap_or_default();
        let graph = self.add_graph(Template::Fishbone);
        let FishboneLayout { shapes, labels } = layout(effect, &categories);
        if let Some(g) = self.graph_mut(graph) {
            g.long_name = book_title;
            let layer = &mut g.layers[0];
            layer.shapes = shapes;
            layer.labels = labels;
        }
        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};

    fn cat(name: &str, causes: &[&str]) -> Category {
        Category { name: name.into(), causes: causes.iter().map(|s| s.to_string()).collect() }
    }

    #[test]
    fn the_layout_has_a_spine_a_rib_per_category_and_a_twig_per_cause() {
        let l = layout("Late delivery", &[cat("People", &["training", "shifts"]), cat("Machines", &["old press"])]);
        // Spine + 2 ribs + 3 twigs; effect + 2 names + 3 causes.
        assert_eq!((l.shapes.len(), l.labels.len()), (6, 6));
        assert_eq!(l.shapes[0].kind, ShapeKind::Arrow);
        assert_eq!(l.labels[0].text, "Late delivery");
        // The first category above the spine, the second below.
        assert!(l.shapes[1].y1 < SPINE_Y && l.shapes[4].y1 > SPINE_Y);
        // Every twig ends on its rib and lies flat.
        for s in &l.shapes[2..4] {
            assert_eq!(s.y1, s.y2);
            assert!(s.x2 > s.x1);
        }
        // Everything stays inside the frame.
        for s in &l.shapes {
            assert!([s.x1, s.x2, s.y1, s.y2].iter().all(|v| (0.0..=1.0).contains(v)), "{s:?}");
        }
        for t in &l.labels {
            assert!((0.0..=1.0).contains(&t.x) && (0.0..=1.0).contains(&t.y), "{t:?}");
        }
    }

    #[test]
    fn a_graph_is_made_from_text_columns_skipping_empty_cells() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let text = |v: &[&str]| ColumnData::text(v.iter().map(|s| s.to_string()).collect());
        let a = p.add_column(book, 0, Designation::Y, text(&["training", "", "shifts"])).unwrap();
        let b = p.add_column(book, 0, Designation::Y, text(&["old press"])).unwrap();
        if let Some(c) = p.column_mut(a) {
            c.long_name = "People".into();
        }
        let g = p.plot_fishbone(&[a, b], "Late").unwrap();
        let layer = &p.graph(g).unwrap().layers[0];
        let texts: Vec<&str> = layer.labels.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["Late", "People", "training", "shifts", "B", "old press"]);
        assert!(p.plot_fishbone(&[], "x").is_err());
    }
}
