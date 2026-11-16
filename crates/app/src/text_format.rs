//! Shared Format toolbar contract. Widgets describe a change; the selected target applies it.
//! T59 can add a worksheet target here without changing the toolbar's font controls.

use sheaf_core::graph::Font;
use sheaf_core::{Color, Document, Project, WindowId};
use sheaf_plot::scene::ObjectKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    GraphText { graph: WindowId, layer: usize, object: ObjectKind },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    Family(String),
    Size(f64),
    Bold(bool),
    Italic(bool),
    Color(Color),
    Scale(f64),
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub target: Target,
    pub font: Font,
    pub generation: u64,
    pub revision: u64,
}

#[derive(Clone, Debug)]
pub struct Request {
    pub target: Target,
    pub generation: u64,
    pub revision: u64,
    pub change: Change,
}

impl Snapshot {
    pub fn graph(doc: &Document, graph: WindowId, layer: usize, object: ObjectKind) -> Option<Self> {
        let target = Target::GraphText { graph, layer, object };
        Some(Self {
            target,
            font: font(doc.project(), target)?.clone(),
            generation: doc.generation(),
            revision: doc.revision(),
        })
    }

    pub fn request(&self, change: Change) -> Request {
        Request { target: self.target, generation: self.generation, revision: self.revision, change }
    }
}

fn font(project: &Project, target: Target) -> Option<&Font> {
    match target {
        Target::GraphText { graph, layer, object } => {
            let l = project.graph(graph)?.layers.get(layer)?;
            match object {
                ObjectKind::XTitle => Some(&l.x_axis.title_font),
                ObjectKind::YTitle => Some(&l.y_axis.title_font),
                ObjectKind::Legend if l.legend.visible => Some(&l.legend.font),
                ObjectKind::Label(i) => Some(&l.labels.get(i)?.font),
                _ => None,
            }
        }
    }
}

/// Reject stale frames, invalid input and no-ops before creating an undo step.
/// The app also checks that the target is still selected and no dialog is editing it.
pub fn apply(doc: &mut Document, request: Request) -> bool {
    if doc.generation() != request.generation || doc.revision() != request.revision {
        return false;
    }
    let Some(before) = font(doc.project(), request.target) else { return false };
    let mut after = before.clone();
    match request.change {
        Change::Family(name) => {
            let name = name.trim();
            if name.is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
                return false;
            }
            after.family = name.to_string();
        }
        Change::Size(size) => after.size_pt = size,
        Change::Bold(value) => after.bold = value,
        Change::Italic(value) => after.italic = value,
        Change::Color(color) => after.color = color,
        Change::Scale(factor) => after.size_pt = (after.size_pt * factor).round(),
    }
    if !after.size_pt.is_finite() || !(1.0..=1000.0).contains(&after.size_pt) || after == *before {
        return false;
    }
    doc.apply("Format graph text", |project| match request.target {
        Target::GraphText { graph, layer, object } => {
            let l = &mut project.graph_mut(graph).expect("validated graph").layers[layer];
            let font = match object {
                ObjectKind::XTitle => &mut l.x_axis.title_font,
                ObjectKind::YTitle => &mut l.y_axis.title_font,
                ObjectKind::Legend => &mut l.legend.font,
                ObjectKind::Label(i) => &mut l.labels[i].font,
                _ => unreachable!("validated text target"),
            };
            *font = after;
        }
    });
    true
}
