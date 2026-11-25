//! The worksheet adapter for the shared Format toolbar. No data values are edited here.

use crate::text_format::{Change, Mixed, Snapshot, Target, WorksheetStyle};
use sheaf_core::cellformat::{CellFormat, FormatIndex, FormatTarget, MAX_CELL_PT, MIN_CELL_PT, ResolvedStyle};
use sheaf_core::graph::Font;
use sheaf_core::{Color, Document, WindowId, Worksheet};
use std::cell::RefCell;

/// Selection summaries are rebuilt only after a document or selection change.
#[derive(Default)]
pub struct Cache(RefCell<Option<Snapshot>>);

#[derive(Clone, PartialEq)]
struct Values {
    family: String,
    size: f32,
    bold: bool,
    italic: bool,
    underline: bool,
    fill: Option<Color>,
    color: Color,
}

impl Values {
    fn of(style: ResolvedStyle<'_>) -> Self {
        Self {
            family: style.font.unwrap_or("Arial").to_owned(),
            size: style.size_pt.unwrap_or(9.0),
            bold: style.bold,
            italic: style.italic,
            underline: style.underline,
            fill: style.fill,
            color: style.text_color.unwrap_or(Color::BLACK),
        }
    }

    fn font(&self) -> Font {
        Font {
            family: self.family.clone(),
            size_pt: f64::from(self.size),
            bold: self.bold,
            italic: self.italic,
            underline: false,
            color: self.color,
        }
    }
}

impl Cache {
    pub fn snapshot(&self, doc: &Document, book: WindowId, sheet: usize, selection: FormatTarget) -> Option<Snapshot> {
        let target = Target::Worksheet { book, sheet, selection };
        if let Some(old) = self.0.borrow().as_ref()
            && old.target == target
            && old.generation == doc.generation()
            && old.revision == doc.revision()
        {
            return Some(old.clone());
        }
        let Target::Worksheet { selection, .. } = &target else { unreachable!() };
        let ws = doc.project().workbook(book)?.sheets.get(sheet)?;
        if !valid(ws, selection) {
            return None;
        }
        let index = ws.format_index();
        let column = ws.columns.iter().find(|c| selection.columns().contains(&c.id))?.id;
        let first = Values::of(match selection {
            FormatTarget::Cells { rows, .. } => index.resolve(ws, column, rows.map_or(0, |r| r.0)),
            FormatTarget::LabelRows { rows, .. } => index.resolve_label(ws, column, rows.first()?),
        });
        let mut mixed = Mixed::default();
        visit(ws, selection, &index, |_, style| {
            let value = Values::of(style);
            mixed.family |= first.family != value.family;
            mixed.size |= first.size != value.size;
            mixed.bold |= first.bold != value.bold;
            mixed.italic |= first.italic != value.italic;
            mixed.underline |= first.underline != value.underline;
            mixed.fill |= first.fill != value.fill;
            mixed.color |= first.color != value.color;
        });
        // Origin's B/I buttons follow the first selected cell even in a mixed selection.
        let font = first.font();
        let worksheet = Some(WorksheetStyle { underline: first.underline, fill: first.fill });
        let snapshot = Snapshot {
            target,
            font,
            legend_color_follows_plot: None,
            mixed,
            worksheet,
            generation: doc.generation(),
            revision: doc.revision(),
        };
        self.0.replace(Some(snapshot.clone()));
        Some(snapshot)
    }
}

fn valid(sheet: &Worksheet, target: &FormatTarget) -> bool {
    !target.columns().is_empty()
        && target.columns().iter().all(|c| sheet.column_index(*c).is_some())
        && match target {
            FormatTarget::Cells { rows: Some((first, Some(last))), .. } => first <= last,
            FormatTarget::LabelRows { rows, .. } => !rows.is_empty(),
            _ => true,
        }
}

/// Visit style intervals, never individual data rows. A million-row whole column with one
/// style needs one lookup. Include open-ended tails so future rows keep their formatting.
fn visit<'a>(
    sheet: &'a Worksheet,
    target: &FormatTarget,
    index: &FormatIndex,
    mut visitor: impl FnMut(FormatTarget, ResolvedStyle<'a>),
) {
    match target {
        FormatTarget::Cells { columns, rows } => {
            let (first, last) = rows.map_or((0, usize::MAX), |(a, b)| (a, b.unwrap_or(usize::MAX)));
            for &column in columns {
                let mut starts = vec![first];
                for range in &sheet.cell_formats {
                    let FormatTarget::Cells { columns, rows } = &range.target else { continue };
                    if !columns.contains(&column) {
                        continue;
                    }
                    let (a, b) = rows.map_or((0, usize::MAX), |(a, b)| (a, b.unwrap_or(usize::MAX)));
                    if a > first && a <= last {
                        starts.push(a);
                    }
                    if let Some(next) = b.checked_add(1)
                        && next > first
                        && next <= last
                    {
                        starts.push(next);
                    }
                }
                starts.sort_unstable();
                starts.dedup();
                for (i, &start) in starts.iter().enumerate() {
                    let end = starts.get(i + 1).map_or(last, |n| n - 1);
                    visitor(
                        FormatTarget::cells(vec![column], start, (end != usize::MAX).then_some(end)),
                        index.resolve(sheet, column, start),
                    );
                }
            }
        }
        FormatTarget::LabelRows { columns, rows } => {
            for &column in columns {
                for row in rows {
                    visitor(
                        FormatTarget::label_rows(vec![column], vec![row.clone()]),
                        index.resolve_label(sheet, column, row),
                    );
                }
            }
        }
    }
}

pub fn apply(doc: &mut Document, book: WindowId, sheet: usize, target: FormatTarget, change: Change) -> bool {
    let Some(ws) = doc.project().workbook(book).and_then(|b| b.sheets.get(sheet)) else { return false };
    if !valid(ws, &target) {
        return false;
    }
    let mut patch = CellFormat::default();
    let mut plans = Vec::new();
    match change {
        Change::Bold(value) => patch.bold = Some(value),
        Change::Italic(value) => patch.italic = Some(value),
        Change::Underline(value) => patch.underline = Some(value),
        Change::Fill(value) => patch.fill = Some(value.unwrap_or(Color::TRANSPARENT)),
        Change::Color(color) => patch.text_color = Some(color),
        Change::FollowPlotColor => return false,
        Change::Family(name) => {
            let name = name.trim();
            if name.len() > 256 || name.chars().any(char::is_control) {
                return false;
            }
            patch.font = Some(name.to_string());
        }
        Change::Size(size) => {
            if !size.is_finite() || (size != 0.0 && !(f64::from(MIN_CELL_PT)..=f64::from(MAX_CELL_PT)).contains(&size))
            {
                return false;
            }
            patch.size_pt = Some(size as f32);
        }
        Change::Scale(factor) => {
            if !factor.is_finite() || factor <= 0.0 {
                return false;
            }
            let index = ws.format_index();
            visit(ws, &target, &index, |band, style| {
                let before = style.size_pt.unwrap_or(9.0);
                let after = ((f64::from(before) * factor).round() as f32).clamp(MIN_CELL_PT, MAX_CELL_PT);
                if before != after {
                    plans.push((band, CellFormat { size_pt: Some(after), ..Default::default() }));
                }
            });
        }
    }
    if !patch.is_empty() {
        let index = ws.format_index();
        let mut changed = false;
        visit(ws, &target, &index, |_, style| {
            changed |= patch.bold.is_some_and(|v| v != style.bold)
                || patch.italic.is_some_and(|v| v != style.italic)
                || patch.underline.is_some_and(|v| v != style.underline)
                || patch.fill.is_some_and(|v| if v.a == 0 { style.fill.is_some() } else { style.fill != Some(v) })
                || patch.font.as_ref().is_some_and(|v| {
                    if v.is_empty() { style.font.is_some() } else { v.as_str() != style.font.unwrap_or("Arial") }
                })
                || patch.size_pt.is_some_and(|v| {
                    if v == 0.0 { style.size_pt.is_some() } else { v != style.size_pt.unwrap_or(9.0) }
                })
                || patch.text_color.is_some_and(|v| v != style.text_color.unwrap_or(Color::BLACK));
        });
        if !changed {
            return false;
        }
        plans.push((target, patch));
    }
    if plans.is_empty() {
        return false;
    }
    let before = doc.revision();
    doc.apply("Format cells", move |project| {
        if let Some(ws) = project.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet)) {
            for (target, patch) in plans {
                ws.apply_cell_format(target, patch);
            }
        }
    });
    doc.revision() != before
}
