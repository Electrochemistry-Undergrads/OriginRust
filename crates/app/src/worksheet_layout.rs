//! Screen row geometry derived from cell formats. Only visible rows are laid out as widgets.

use crate::actions::LabelRow;
use eframe::egui::{self, FontId};
use sheaf_core::Worksheet;
use sheaf_core::cellformat::{FormatIndex, LabelKind, ResolvedStyle};
use sheaf_core::sheet_print::RowHeights;

pub struct Layout {
    pub rows: RowHeights,
    pub labels: Vec<f32>,
}

fn height(ctx: &egui::Context, f: ResolvedStyle<'_>) -> f64 {
    if f.size_pt.is_none() && f.font.is_none() {
        return 16.0;
    }
    let font = crate::worksheet_fonts::cell_font(ctx, &f);
    let id = FontId::new(f.size_pt.map_or(12.0, |p| sheaf_core::cellformat::clamp_cell_pt(p) * 4.0 / 3.0), font.family);
    f64::from(ctx.fonts_mut(|fonts| fonts.row_height(&id)).ceil() + 2.0).max(16.0)
}

impl Layout {
    pub fn new(
        ctx: &egui::Context,
        sheet: &Worksheet,
        index: &FormatIndex,
        labels: &[LabelRow],
        shown: Option<&[usize]>,
    ) -> Self {
        let rows = RowHeights::new(sheet, index, 16.0, |f| height(ctx, f));
        let rows = shown.map_or_else(|| rows.clone(), |s| rows.shown(s));
        let labels = labels
            .iter()
            .map(|l| {
                let kind = match l {
                    LabelRow::LongName => LabelKind::LongName,
                    LabelRow::Units => LabelKind::Units,
                    LabelRow::Comments => LabelKind::Comments,
                    LabelRow::Formula => LabelKind::Formula,
                    LabelRow::Param(n) => LabelKind::Param(n.clone()),
                };
                sheet
                    .columns
                    .iter()
                    .filter(|c| !c.hidden)
                    .map(|c| height(ctx, index.resolve_label(sheet, c.id, &kind)))
                    .fold(16.0, f64::max) as f32
            })
            .collect();
        Self { rows, labels }
    }

    pub fn label_top(&self, row: usize) -> f32 {
        16.0 + self.labels[..row].iter().sum::<f32>()
    }
    pub fn head_height(&self) -> f32 {
        self.label_top(self.labels.len())
    }
}
