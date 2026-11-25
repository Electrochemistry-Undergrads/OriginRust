//! Shared Format toolbar contract. Widgets describe a change; the selected target applies it.
//! Selected graph objects and worksheet ranges share controls and stale-request protection.

use sheaf_core::graph::Font;
use sheaf_core::{Color, Document, Project, WindowId};
use sheaf_plot::scene::ObjectKind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    GraphText { graph: WindowId, layer: usize, object: ObjectKind },
    Worksheet { book: WindowId, sheet: usize, selection: sheaf_core::cellformat::FormatTarget },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    Family(String),
    Size(f64),
    Bold(bool),
    Italic(bool),
    Underline(bool),
    Fill(Option<Color>),
    Color(Color),
    FollowPlotColor,
    Scale(f64),
}

#[derive(Clone, Debug, Default)]
pub struct Mixed {
    pub family: bool,
    pub size: bool,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub fill: bool,
    pub color: bool,
}

/// Cell properties beyond the shared font. Graph objects use object_format.
#[derive(Clone, Copy, Debug)]
pub struct WorksheetStyle {
    pub underline: bool,
    pub fill: Option<Color>,
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub target: Target,
    pub font: Font,
    /// The automatic color state of an ordinary legend.
    pub legend_color_follows_plot: Option<bool>,
    pub mixed: Mixed,
    pub worksheet: Option<WorksheetStyle>,
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
        let font = font(doc.project(), target.clone())?.clone();
        let legend_color_follows_plot = legend_color_follows_plot(doc.project(), &target);
        Some(Self {
            target,
            font,
            legend_color_follows_plot,
            mixed: Mixed::default(),
            worksheet: None,
            generation: doc.generation(),
            revision: doc.revision(),
        })
    }

    pub fn scaling_enabled(&self) -> bool {
        matches!(self.target, Target::GraphText { .. })
    }

    pub fn size_text(&self) -> String {
        if self.mixed.size {
            return String::new();
        }
        match self.target {
            Target::Worksheet { .. } => (self.font.size_pt as f32).to_string(),
            Target::GraphText { .. } => self.font.size_pt.to_string(),
        }
    }

    pub fn size_help(&self) -> &'static str {
        match self.target {
            Target::Worksheet { .. } => "Font Size (4 to 96 pt; 0 restores default); Enter applies, Escape cancels",
            Target::GraphText { .. } => "Font Size (1 to 1000 pt); Enter applies, Escape cancels",
        }
    }

    pub fn request(&self, change: Change) -> Request {
        Request { target: self.target.clone(), generation: self.generation, revision: self.revision, change }
    }
}

fn font(project: &Project, target: Target) -> Option<&Font> {
    match target {
        Target::Worksheet { .. } => None,
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

fn legend_color_follows_plot(project: &Project, target: &Target) -> Option<bool> {
    let Target::GraphText { graph, layer, object: ObjectKind::Legend } = target else { return None };
    let layer = project.graph(*graph)?.layers.get(*layer)?;
    (layer.legend.visible && !layer.plots.iter().any(|plot| plot.kind.is_parallel()))
        .then_some(layer.legend.text_color_follows_plot)
}

/// Reject stale frames, invalid input and no-ops before creating an undo step.
/// The app also checks that the target is still selected and no dialog is editing it.
pub fn apply(doc: &mut Document, request: Request) -> bool {
    if doc.generation() != request.generation || doc.revision() != request.revision {
        return false;
    }
    if let Target::Worksheet { book, sheet, selection } = &request.target {
        return crate::worksheet_format::apply(doc, *book, *sheet, selection.clone(), request.change);
    }
    let Some(before) = font(doc.project(), request.target.clone()) else { return false };
    let mut after = before.clone();
    let previous_follow = legend_color_follows_plot(doc.project(), &request.target);
    let mut follows_plot = previous_follow;
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
        Change::Underline(_) | Change::Fill(_) => return false,
        Change::Color(color) => {
            after.color = color;
            follows_plot = previous_follow.map(|_| false);
        }
        Change::FollowPlotColor if previous_follow.is_some() => follows_plot = Some(true),
        Change::FollowPlotColor => return false,
        Change::Scale(factor) => after.size_pt = (after.size_pt * factor).round(),
    }
    if !after.size_pt.is_finite()
        || !(1.0..=1000.0).contains(&after.size_pt)
        || (after == *before && follows_plot == previous_follow)
    {
        return false;
    }
    doc.apply("Format graph text", |project| match request.target {
        Target::Worksheet { .. } => unreachable!("worksheet request handled above"),
        Target::GraphText { graph, layer, object } => {
            let l = &mut project.graph_mut(graph).expect("validated graph").layers[layer];
            let font = match object {
                ObjectKind::XTitle => &mut l.x_axis.title_font,
                ObjectKind::YTitle => &mut l.y_axis.title_font,
                ObjectKind::Legend => {
                    if let Some(follows_plot) = follows_plot {
                        l.legend.text_color_follows_plot = follows_plot;
                    }
                    &mut l.legend.font
                }
                ObjectKind::Label(i) => &mut l.labels[i].font,
                _ => unreachable!("validated text target"),
            };
            *font = after;
        }
    });
    true
}
