//! Screen row geometry derived from cell formats. Only visible rows are laid out as widgets.

use crate::actions::LabelRow;
use eframe::egui::{self, FontId};
use sheaf_core::Worksheet;
use sheaf_core::cellformat::{FormatIndex, LabelKind, ResolvedStyle};
use sheaf_core::sheet_print::RowHeights;

pub struct Layout {
    pub rows: RowHeights,
    pub labels: Vec<f32>,
    column_header_height: f32,
}

fn height(ctx: &egui::Context, f: ResolvedStyle<'_>, minimum: f64) -> f64 {
    if f.size_pt.is_none() && f.font.is_none() {
        return minimum;
    }
    let font = crate::worksheet_fonts::cell_font(ctx, &f);
    let id = FontId::new(f.size_pt.map_or(12.0, |p| sheaf_core::cellformat::clamp_cell_pt(p) * 4.0 / 3.0), font.family);
    f64::from(ctx.fonts_mut(|fonts| fonts.row_height(&id)).ceil() + 2.0).max(minimum)
}

impl Layout {
    pub fn new(
        ctx: &egui::Context,
        sheet: &Worksheet,
        index: &FormatIndex,
        labels: &[LabelRow],
        shown: Option<&[usize]>,
    ) -> Self {
        let base = f64::from(crate::grid::ROW_HEIGHT * sheet.font_defaults.size_scale());
        let measure = |font: ResolvedStyle<'_>| height(ctx, font, base);
        let row_header_height =
            if sheet.view.row_header { measure(sheet.font_defaults.row_header_style()) } else { base };
        let rows = RowHeights::new(sheet, index, row_header_height, measure);
        let rows = shown.map_or_else(|| rows.clone(), |s| rows.shown(s));
        let labels = labels
            .iter()
            .map(|l| {
                let kind = match l {
                    LabelRow::LongName => LabelKind::LongName,
                    LabelRow::Units => LabelKind::Units,
                    LabelRow::Comments => LabelKind::Comments,
                    LabelRow::Formula => LabelKind::Formula,
                    LabelRow::SamplingInterval => LabelKind::SamplingInterval,
                    LabelRow::Sparklines => LabelKind::Sparklines,
                    LabelRow::Categories => LabelKind::Categories,
                    LabelRow::Param(n) => LabelKind::Param(n.clone()),
                };
                // The Sparklines row (T175) is twice as tall as a text row, Sheaf's choice.
                let least = if matches!(l, LabelRow::Sparklines) { 2.0 * base } else { base };
                let least =
                    if sheet.view.row_header && sheet.view.row_labels { least.max(row_header_height) } else { least };
                sheet
                    .columns
                    .iter()
                    .filter(|c| !c.hidden)
                    .map(|c| measure(index.resolve_label(sheet, c.id, &kind)))
                    .fold(least, f64::max) as f32
            })
            .collect();
        let column_header_height =
            if sheet.view.column_header { measure(sheet.font_defaults.column_header_style()) as f32 } else { 0.0 };
        Self { rows, labels, column_header_height }
    }

    pub fn column_header_height(&self) -> f32 {
        self.column_header_height
    }

    pub fn label_top(&self, row: usize) -> f32 {
        self.column_header_height + self.labels[..row].iter().sum::<f32>()
    }
    pub fn head_height(&self) -> f32 {
        self.label_top(self.labels.len())
    }
}
