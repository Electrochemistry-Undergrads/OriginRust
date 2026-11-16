//! Combine resolved worksheet styles with Excel number formats, without changing the sheet.
use rust_xlsxwriter::{Color as XlsxColor, Format, FormatAlign, FormatUnderline};
use sheaf_core::cellformat::{FormatIndex, FormatTarget, HAlign, LabelKind, ResolvedStyle, clamp_cell_pt};
use sheaf_core::color::Color;
use sheaf_core::condformat::{self, CellStyles};
use sheaf_core::{ColumnId, Worksheet};
use std::ops::Range;

pub(super) struct Formats<'a> {
    sheet: &'a Worksheet,
    manual: FormatIndex,
    conditional: CellStyles,
}

impl<'a> Formats<'a> {
    pub(super) fn new(sheet: &'a Worksheet) -> Self {
        Self { sheet, manual: sheet.format_index(), conditional: condformat::evaluate(sheet, &sheet.cond_formats) }
    }

    pub(super) fn data_style(&self, column: ColumnId, row: usize) -> ResolvedStyle<'_> {
        self.manual.resolve(self.sheet, column, row).with_conditional(self.conditional.get(column, row))
    }

    pub(super) fn data(&self, column: ColumnId, row: usize, number: Option<&Format>) -> Format {
        combine(self.data_style(column, row), number)
    }

    pub(super) fn label(&self, column: ColumnId, kind: Option<&LabelKind>) -> Format {
        combine(kind.map(|k| self.manual.resolve_label(self.sheet, column, k)).unwrap_or_default(), None)
    }

    /// Only scan styled intervals within the existing data rectangle. Open-ended styles must
    /// never manufacture a million blank rows, nor stretch the exported data to a remote range.
    pub(super) fn blank_ranges(&self, column: ColumnId) -> Vec<Range<usize>> {
        let limit = self.sheet.row_count();
        let mut ranges: Vec<_> = self
            .sheet
            .cell_formats
            .iter()
            .filter_map(|r| {
                let FormatTarget::Cells { columns, rows } = &r.target else { return None };
                if !columns.contains(&column) {
                    return None;
                }
                let (first, last) = rows.unwrap_or((0, None));
                let end = last.map_or(limit, |n| n.saturating_add(1).min(limit));
                (first < end).then_some(first..end)
            })
            .collect();
        ranges.sort_by_key(|r| r.start);
        let mut merged: Vec<Range<usize>> = Vec::new();
        for range in ranges {
            if let Some(last) = merged.last_mut().filter(|last| range.start <= last.end) {
                last.end = last.end.max(range.end);
            } else {
                merged.push(range);
            }
        }
        merged
    }

    pub(super) fn warn_omissions(&self, labels: &[Option<LabelKind>], warnings: &mut Vec<String>) {
        let rows = self.sheet.row_count();
        let mut outside = false;
        let mut omitted_labels = false;
        let mut alpha = false;
        for range in &self.sheet.cell_formats {
            match &range.target {
                FormatTarget::Cells { rows: span, .. } => {
                    let (first, last) = span.unwrap_or((0, None));
                    outside |= first >= rows || last.is_some_and(|last| last >= rows);
                }
                FormatTarget::LabelRows { rows, .. } => {
                    omitted_labels |= rows.iter().any(|r| !labels.iter().any(|l| l.as_ref() == Some(r)));
                }
            }
            alpha |= [range.style.text_color, range.style.fill].into_iter().flatten().any(|c| c.a != 0 && c.a != 255);
        }
        let name = &self.sheet.name;
        if outside {
            warnings.push(format!("sheet '{name}': cell formatting outside the exported data rows is not written"));
        }
        if omitted_labels {
            warnings.push(format!("sheet '{name}': formatting for label rows omitted from this export is not written"));
        }
        if !self.sheet.cond_formats.is_empty() {
            warnings.push(format!(
                "sheet '{name}': conditional colours are exported as a snapshot; the conditional rules are not written"
            ));
        }
        for rule in &self.sheet.cond_formats {
            let colours = match &rule.rule {
                condformat::Rule::MinMax { max, min, .. } => [*max, *min],
                condformat::Rule::TopBottom { colours, .. } => [*colours, *colours],
            };
            alpha |= colours.iter().flat_map(|c| [c.fill, c.font]).flatten().any(|c| c.a != 0 && c.a != 255);
        }
        if alpha {
            warnings
                .push(format!("sheet '{name}': partially transparent cell colours are written as opaque RGB in Excel"));
        }
    }
}

fn rgb(color: Color) -> XlsxColor {
    XlsxColor::RGB((u32::from(color.r) << 16) | (u32::from(color.g) << 8) | u32::from(color.b))
}

fn combine(style: ResolvedStyle<'_>, number: Option<&Format>) -> Format {
    let mut format = number
        .cloned()
        .unwrap_or_default()
        .set_font_name(style.font.unwrap_or("Arial"))
        .set_font_size(f64::from(clamp_cell_pt(style.size_pt.unwrap_or(9.0))));
    if style.bold {
        format = format.set_bold();
    }
    if style.italic {
        format = format.set_italic();
    }
    if style.underline {
        format = format.set_underline(FormatUnderline::Single);
    }
    if let Some(color) = style.text_color.filter(|c| c.a != 0) {
        format = format.set_font_color(rgb(color));
    }
    if let Some(color) = style.fill.filter(|c| c.a != 0) {
        format = format.set_background_color(rgb(color));
    }
    if let Some(align) = style.h_align {
        format = format.set_align(match align {
            HAlign::Left => FormatAlign::Left,
            HAlign::Center => FormatAlign::Center,
            HAlign::Right => FormatAlign::Right,
            HAlign::Auto => FormatAlign::General,
        });
    }
    format
}
