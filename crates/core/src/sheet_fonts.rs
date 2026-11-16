//! Worksheet-wide category fonts, below per-range cell formatting (T59).
use crate::Worksheet;
use crate::cellformat::{ResolvedStyle, clamp_cell_pt};
use serde::{Deserialize, Serialize};

/// Empty family is the automatic face. Sizes use the existing worksheet point-size bounds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorksheetFont {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub family: String,
    #[serde(default = "default_size")]
    pub size_pt: f32,
}

fn default_size() -> f32 {
    9.0
}

impl Default for WorksheetFont {
    fn default() -> Self {
        Self { family: String::new(), size_pt: default_size() }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorksheetFonts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<WorksheetFont>,
}

impl WorksheetFonts {
    pub fn is_empty(&self) -> bool {
        self.data.is_none()
    }

    pub(crate) fn data_style<'a>(&'a self, mut manual: ResolvedStyle<'a>) -> ResolvedStyle<'a> {
        if let Some(font) = &self.data {
            if manual.font.is_none() && !font.family.is_empty() {
                manual.font = Some(&font.family);
            }
            if manual.size_pt.is_none() {
                let size = if font.size_pt.is_finite() && font.size_pt > 0.0 { font.size_pt } else { default_size() };
                manual.size_pt = Some(clamp_cell_pt(size));
            }
        }
        manual
    }
}

impl Worksheet {
    /// Changes the Data font of this sheet, including rows/columns subsequently added.
    /// Per-cell/range fonts remain above this layer; this does not create per-row records.
    pub fn set_data_font(&mut self, mut font: WorksheetFont) -> bool {
        if !font.size_pt.is_finite() || font.size_pt <= 0.0 {
            return false;
        }
        font.family = font.family.trim().to_owned();
        font.size_pt = clamp_cell_pt(font.size_pt);
        if self.font_defaults.data.as_ref() == Some(&font) {
            return false;
        }
        self.font_defaults.data = Some(font);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cellformat::{CellFormat, FormatTarget, LabelKind};
    use crate::color::Color;
    use crate::{Column, ColumnData, ColumnId, Designation};

    fn sheet() -> Worksheet {
        Worksheet {
            columns: vec![Column::new(ColumnId(1), "A", Designation::Y, ColumnData::numeric(vec![1.0, 2.0]))],
            ..Worksheet::new("S")
        }
    }

    #[test]
    fn old_sheets_keep_their_format_and_serialized_shape() {
        let s = sheet();
        let value = serde_json::to_value(&s).unwrap();
        assert!(value.get("font_defaults").is_none());
        let back: Worksheet = serde_json::from_value(value.clone()).unwrap();
        // Worksheet JSON is metadata; the IO container stores column arrays separately.
        assert_eq!(serde_json::to_value(&back).unwrap(), value);
        assert!(back.format_index().is_empty());
        assert!(back.cell_format_at(ColumnId(1), 0).is_default());
    }

    #[test]
    fn data_size_reaches_future_columns_and_rows_without_format_records() {
        let mut s = sheet();
        assert!(s.set_data_font(WorksheetFont { size_pt: 18.0, ..Default::default() }));
        assert!(s.cell_formats.is_empty());
        assert!(!s.format_index().is_empty());
        s.columns.push(Column::new(ColumnId(9), "B", Designation::Y, ColumnData::default()));
        let index = s.format_index();
        for col in [ColumnId(1), ColumnId(9)] {
            for row in [0, 1, 999_999] {
                assert_eq!(index.resolve(&s, col, row).size_pt, Some(18.0));
            }
        }
        assert!(index.resolve_label(&s, ColumnId(1), &LabelKind::LongName).is_default());
        assert_eq!(s.columns[0].numeric().unwrap(), &[1.0, 2.0]);
        assert!(serde_json::to_string(&s.font_defaults).unwrap().len() < 100);
    }

    #[test]
    fn manual_size_twelve_survives_data_twenty_and_other_properties_are_retained() {
        let mut s = sheet();
        s.set_data_font(WorksheetFont { size_pt: 18.0, ..Default::default() });
        s.apply_cell_format(
            FormatTarget::cells(vec![ColumnId(1)], 0, Some(0)),
            CellFormat {
                size_pt: Some(12.0),
                bold: Some(true),
                text_color: Some(Color::rgb(17, 34, 51)),
                ..Default::default()
            },
        );
        s.set_data_font(WorksheetFont { size_pt: 20.0, ..Default::default() });
        let index = s.format_index();
        let a = index.resolve(&s, ColumnId(1), 0);
        assert_eq!(a.size_pt, Some(12.0));
        assert!(a.bold);
        assert_eq!(a.text_color, Some(Color::rgb(17, 34, 51)));
        assert_eq!(index.resolve(&s, ColumnId(1), 1).size_pt, Some(20.0));
    }

    #[test]
    fn named_defaults_clone_and_roundtrip_with_independent_manual_faces() {
        let mut s = sheet();
        s.set_data_font(WorksheetFont { family: "Times New Roman".into(), size_pt: 18.0 });
        s.apply_cell_format(
            FormatTarget::cells(vec![ColumnId(1)], 1, Some(1)),
            CellFormat { font: Some("Courier New".into()), ..Default::default() },
        );
        let back: Worksheet = serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
        assert_eq!(s.clone(), s);
        assert_eq!(serde_json::to_value(&back).unwrap(), serde_json::to_value(&s).unwrap());
        assert_eq!(back.font_defaults, s.font_defaults);
        assert_eq!(back.cell_formats, s.cell_formats);
        let index = back.format_index();
        assert_eq!(index.resolve(&back, ColumnId(1), 0).font, Some("Times New Roman"));
        let manual = index.resolve(&back, ColumnId(1), 1);
        assert_eq!((manual.font, manual.size_pt), (Some("Courier New"), Some(18.0)));
    }

    #[test]
    fn invalid_sizes_and_repeated_values_leave_the_model_unchanged() {
        let mut s = sheet();
        for size_pt in [f32::NAN, f32::INFINITY, -1.0, 0.0] {
            assert!(!s.set_data_font(WorksheetFont { size_pt, ..Default::default() }));
        }
        assert!(s.font_defaults.is_empty());
        let font = WorksheetFont { family: " Arial ".into(), size_pt: 200.0 };
        assert!(s.set_data_font(font.clone()));
        assert!(!s.set_data_font(font));
        let f = s.cell_format_at(ColumnId(1), 0);
        assert_eq!((f.font, f.size_pt), (Some("Arial"), Some(96.0)));
    }

    #[test]
    fn print_uses_effective_fonts_and_compact_derived_geometry() {
        use crate::sheet_print::{PrintOp, PrintStyle, RowHeights, SheetTable};
        let mut s = sheet();
        s.set_data_font(WorksheetFont { family: "Times New Roman".into(), size_pt: 18.0 });
        s.apply_cell_format(
            FormatTarget::cells(vec![ColumnId(1)], 0, Some(0)),
            CellFormat { size_pt: Some(12.0), ..Default::default() },
        );
        let index = s.format_index();
        let rows = RowHeights::new(&s, &index, 16.0, |f| f64::from(f.size_pt.unwrap_or(9.0)) * 2.0);
        assert_eq!(rows.height(0), 24.0);
        assert_eq!(rows.height(999_999), 36.0);
        assert_eq!(rows.row_at(rows.offset(999_999) + 18.0), 999_999);
        let table = SheetTable::new(&s, PrintStyle::default());
        let (pages, _) = table.pages_for(&crate::page_setup::PageSetup::default());
        let ops = table.draw_page(&pages[0], pages.len());
        for (value, pt) in [("1", 12.0), ("2", 18.0)] {
            assert!(ops.iter().any(|op| matches!(op, PrintOp::Text { text, size, fmt, .. }
                if text == value && *size == pt * 8.0 / 9.0 && fmt.family.as_deref() == Some("Times New Roman"))));
        }
    }
}
