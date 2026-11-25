//! Sticky Notes (T421; Origin 2027's Sticky Notes, Sheaf's own design): a coloured note pinned to a graph window,
//! a workbook or one sheet of a workbook. It shows as a strip at the top of the window (the sheet's note while that
//! sheet is shown) that can be collapsed to a one-line indicator, and Find in Project finds its text.
//!
//! Sheaf's choices (Origin 2027 not measured here, Origin 2016 has none): one note per graph, workbook and sheet;
//! five colours; a new note is yellow and open; the note follows its sheet when sheets are moved, as it is stored
//! on the sheet.

use crate::Color;
use crate::ids::WindowId;
use crate::project::{Project, ProjectError, Window};
use serde::{Deserialize, Serialize};

/// The colours a note can take.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteColor {
    #[default]
    Yellow,
    Green,
    Blue,
    Pink,
    Orange,
}

impl NoteColor {
    pub const ALL: [NoteColor; 5] =
        [NoteColor::Yellow, NoteColor::Green, NoteColor::Blue, NoteColor::Pink, NoteColor::Orange];

    /// The note's background: light shades, so black text reads on all of them.
    pub fn rgb(self) -> Color {
        match self {
            NoteColor::Yellow => Color::rgb(255, 245, 157),
            NoteColor::Green => Color::rgb(197, 232, 183),
            NoteColor::Blue => Color::rgb(187, 222, 251),
            NoteColor::Pink => Color::rgb(248, 187, 208),
            NoteColor::Orange => Color::rgb(255, 204, 153),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            NoteColor::Yellow => "Yellow",
            NoteColor::Green => "Green",
            NoteColor::Blue => "Blue",
            NoteColor::Pink => "Pink",
            NoteColor::Orange => "Orange",
        }
    }
}

/// A sticky note.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StickyNote {
    pub text: String,
    #[serde(default)]
    pub color: NoteColor,
    /// Shown as a one-line indicator only.
    #[serde(default)]
    pub collapsed: bool,
}

impl StickyNote {
    /// The one line a collapsed note shows: its first line, or "(empty note)".
    pub fn first_line(&self) -> &str {
        self.text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("(empty note)")
    }
}

/// What a note is pinned to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NoteTarget {
    /// A graph window or a workbook.
    Window(WindowId),
    /// Sheet `sheet` (index) of workbook `book`.
    Sheet(WindowId, usize),
}

impl Project {
    /// The note pinned to `target`, if any.
    pub fn sticky_note(&self, target: NoteTarget) -> Option<&StickyNote> {
        match target {
            NoteTarget::Window(id) => match self.window(id)? {
                Window::Graph(g) => g.sticky.as_ref(),
                Window::Workbook(b) => b.sticky.as_ref(),
                Window::Matrix(m) => m.sticky.as_ref(),
                Window::Layout(l) => l.sticky.as_ref(),
                _ => None,
            },
            NoteTarget::Sheet(book, sheet) => self.workbook(book)?.sheets.get(sheet)?.sticky.as_ref(),
        }
    }

    /// Where `target`'s note is kept; an error for a window that takes no note.
    fn sticky_slot(&mut self, target: NoteTarget) -> Result<&mut Option<StickyNote>, ProjectError> {
        match target {
            NoteTarget::Window(id) => {
                if self.graph(id).is_some() {
                    return self.graph_mut(id).map(|g| &mut g.sticky).ok_or(ProjectError::NoStickyNote);
                }
                if self.workbook(id).is_some() {
                    return self.workbook_mut(id).map(|b| &mut b.sticky).ok_or(ProjectError::NoStickyNote);
                }
                if self.matrix_book(id).is_some() {
                    return self.matrix_book_mut(id).map(|m| &mut m.sticky).ok_or(ProjectError::NoStickyNote);
                }
                self.layout_mut(id).map(|l| &mut l.sticky).ok_or(ProjectError::NoStickyNote)
            }
            NoteTarget::Sheet(book, sheet) => self
                .workbook_mut(book)
                .and_then(|b| b.sheets.get_mut(sheet))
                .map(|s| &mut s.sticky)
                .ok_or(ProjectError::NoStickyNote),
        }
    }

    /// Pins an empty yellow note to `target`, or opens the one it has.
    pub fn add_sticky_note(&mut self, target: NoteTarget) -> Result<(), ProjectError> {
        let slot = self.sticky_slot(target)?;
        match slot {
            Some(n) => n.collapsed = false,
            None => *slot = Some(StickyNote::default()),
        }
        Ok(())
    }

    /// Changes `target`'s note through `change`; an error when it has none.
    pub fn edit_sticky_note(
        &mut self,
        target: NoteTarget,
        change: impl FnOnce(&mut StickyNote),
    ) -> Result<(), ProjectError> {
        let note = self.sticky_slot(target)?.as_mut().ok_or(ProjectError::NoStickyNote)?;
        change(note);
        Ok(())
    }

    /// Removes `target`'s note. False when it had none.
    pub fn remove_sticky_note(&mut self, target: NoteTarget) -> bool {
        self.sticky_slot(target).is_ok_and(|s| s.take().is_some())
    }

    /// Every note of the project with what it is pinned to: graphs, then workbooks with their sheets.
    pub fn sticky_notes(&self) -> Vec<(NoteTarget, &StickyNote)> {
        let mut out = Vec::new();
        for g in self.graphs() {
            if let Some(n) = &g.sticky {
                out.push((NoteTarget::Window(g.id), n));
            }
        }
        for w in &self.windows {
            let (id, note) = match w {
                Window::Matrix(m) => (m.id, &m.sticky),
                Window::Layout(l) => (l.id, &l.sticky),
                _ => continue,
            };
            if let Some(n) = note {
                out.push((NoteTarget::Window(id), n));
            }
        }
        for b in self.workbooks() {
            if let Some(n) = &b.sticky {
                out.push((NoteTarget::Window(b.id), n));
            }
            for (i, s) in b.sheets.iter().enumerate() {
                if let Some(n) = &s.sticky {
                    out.push((NoteTarget::Sheet(b.id, i), n));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};
    use crate::template::Template;

    #[test]
    fn notes_pin_to_graphs_books_and_sheets_and_save() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
        let g = p.plot_columns(Template::Line, &[y]).unwrap();
        for t in [NoteTarget::Window(g), NoteTarget::Window(b), NoteTarget::Sheet(b, 0)] {
            assert!(p.sticky_note(t).is_none());
            p.add_sticky_note(t).unwrap();
            assert_eq!(p.sticky_note(t), Some(&StickyNote::default()));
        }
        assert!(p.add_sticky_note(NoteTarget::Sheet(b, 5)).is_err(), "no such sheet");
        p.edit_sticky_note(NoteTarget::Sheet(b, 0), |n| {
            n.text = "\n  check run 3 \nsecond".into();
            n.color = NoteColor::Pink;
            n.collapsed = true;
        })
        .unwrap();
        let n = p.sticky_note(NoteTarget::Sheet(b, 0)).unwrap();
        assert_eq!((n.first_line(), n.color, n.collapsed), ("check run 3", NoteColor::Pink, true));
        // Adding again opens the note and keeps its text.
        p.add_sticky_note(NoteTarget::Sheet(b, 0)).unwrap();
        let n = p.sticky_note(NoteTarget::Sheet(b, 0)).unwrap();
        assert!(!n.collapsed && n.text.contains("second"));
        assert_eq!(p.sticky_notes().len(), 3);
        let back: Project = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(back.sticky_note(NoteTarget::Sheet(b, 0)), p.sticky_note(NoteTarget::Sheet(b, 0)));
        assert!(p.remove_sticky_note(NoteTarget::Window(g)));
        assert!(!p.remove_sticky_note(NoteTarget::Window(g)));
        assert!(p.edit_sticky_note(NoteTarget::Window(g), |n| n.text.clear()).is_err());
        assert_eq!(StickyNote::default().first_line(), "(empty note)");
        // T460: a matrix book and a layout page.
        let m = p.add_matrix_book("");
        let l = p.add_layout();
        for t in [NoteTarget::Window(m), NoteTarget::Window(l)] {
            p.add_sticky_note(t).unwrap();
            assert!(p.sticky_note(t).is_some());
        }
        assert_eq!(p.sticky_notes().len(), 4, "graph note removed above; book, sheet, matrix, layout");
    }
}
