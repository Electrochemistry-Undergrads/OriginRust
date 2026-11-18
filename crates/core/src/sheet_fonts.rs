//! Worksheet-wide category fonts, below per-range cell formatting (T59).
use crate::Worksheet;
use crate::cellformat::{LabelKind, ResolvedStyle, clamp_cell_pt};
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_name_size_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub units_size_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comments_size_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_name_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub units_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comments_family: Option<String>,
}

impl WorksheetFonts {
    pub fn is_empty(&self) -> bool {
        self.data.is_none()
            && self.long_name_size_pt.is_none()
            && self.units_size_pt.is_none()
            && self.comments_size_pt.is_none()
            && self.long_name_family.is_none()
            && self.units_family.is_none()
            && self.comments_family.is_none()
    }

    pub fn label_size(&self, kind: &LabelKind) -> Option<f32> {
        match kind {
            LabelKind::LongName => self.long_name_size_pt,
            LabelKind::Units => self.units_size_pt,
            LabelKind::Comments => self.comments_size_pt,
            _ => None,
        }
    }

    pub fn label_family(&self, kind: &LabelKind) -> Option<&str> {
        match kind {
            LabelKind::LongName => self.long_name_family.as_deref(),
            LabelKind::Units => self.units_family.as_deref(),
            LabelKind::Comments => self.comments_family.as_deref(),
            _ => None,
        }
        .filter(|name| !name.is_empty())
    }

    pub(crate) fn label_style<'a>(&'a self, kind: &LabelKind, mut manual: ResolvedStyle<'a>) -> ResolvedStyle<'a> {
        if manual.font.is_none() {
            manual.font = self.label_family(kind);
        }
        if manual.size_pt.is_none() {
            manual.size_pt = self
                .label_size(kind)
                .map(|size| clamp_cell_pt(if size.is_finite() && size > 0.0 { size } else { default_size() }));
        }
        manual
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
    /// Changes one label category's family. Empty names restore the automatic face;
    /// point sizes and manual properties remain independent.
    pub fn set_label_font_family(&mut self, kind: &LabelKind, family: &str) -> bool {
        let slot = match kind {
            LabelKind::LongName => &mut self.font_defaults.long_name_family,
            LabelKind::Units => &mut self.font_defaults.units_family,
            LabelKind::Comments => &mut self.font_defaults.comments_family,
            _ => return false,
        };
        let name = family.trim();
        let next = (!name.is_empty()).then(|| name.to_owned());
        if *slot == next {
            return false;
        }
        *slot = next;
        true
    }

    /// Changes only one supported label category's point size, including future columns.
    pub fn set_label_font_size(&mut self, kind: &LabelKind, size_pt: f32) -> bool {
        if !size_pt.is_finite() || size_pt <= 0.0 {
            return false;
        }
        let slot = match kind {
            LabelKind::LongName => &mut self.font_defaults.long_name_size_pt,
            LabelKind::Units => &mut self.font_defaults.units_size_pt,
            LabelKind::Comments => &mut self.font_defaults.comments_size_pt,
            _ => return false,
        };
        let size = clamp_cell_pt(size_pt);
        if *slot == Some(size) {
            return false;
        }
        *slot = Some(size);
        true
    }

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

    #[test]
    fn label_categories_reach_future_columns_independently_and_roundtrip_sparsely() {
        let mut s = sheet();
        for (kind, size) in [(LabelKind::LongName, 18.0), (LabelKind::Units, 12.0), (LabelKind::Comments, 24.0)] {
            assert!(s.set_label_font_size(&kind, size));
            assert!(!s.set_label_font_size(&kind, size));
        }
        s.columns.push(Column::new(ColumnId(9), "B", Designation::Y, ColumnData::default()));
        let back: Worksheet = serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
        assert_eq!(back.font_defaults, s.font_defaults);
        assert_eq!(back.clone().font_defaults, s.font_defaults);
        assert!(back.cell_formats.is_empty());
        assert!(!back.format_index().is_empty());
        for col in [ColumnId(1), ColumnId(9)] {
            let index = back.format_index();
            for (kind, size) in [(LabelKind::LongName, 18.0), (LabelKind::Units, 12.0), (LabelKind::Comments, 24.0)] {
                assert_eq!(index.resolve_label(&back, col, &kind).size_pt, Some(size));
            }
            assert!(index.resolve_label(&back, col, &LabelKind::Formula).is_default());
            assert!(index.resolve_label(&back, col, &LabelKind::Param("Sample".into())).is_default());
            assert!(index.resolve(&back, col, 0).is_default());
        }
        assert_eq!(s.columns[0].numeric().unwrap(), &[1.0, 2.0]);
        assert!(serde_json::to_string(&s.font_defaults).unwrap().len() < 100);
    }

    #[test]
    fn label_defaults_keep_manual_properties_above_independent_category_sizes() {
        let mut s = sheet();
        s.set_label_font_size(&LabelKind::LongName, 18.0);
        s.set_label_font_size(&LabelKind::Units, 24.0);
        s.apply_cell_format(
            FormatTarget::label_rows(vec![ColumnId(1)], vec![LabelKind::LongName]),
            CellFormat {
                size_pt: Some(12.0),
                font: Some("Courier New".into()),
                bold: Some(true),
                ..Default::default()
            },
        );
        s.set_label_font_size(&LabelKind::LongName, 20.0);
        let index = s.format_index();
        let manual = index.resolve_label(&s, ColumnId(1), &LabelKind::LongName);
        assert_eq!((manual.size_pt, manual.font, manual.bold), (Some(12.0), Some("Courier New"), true));
        assert_eq!(index.resolve_label(&s, ColumnId(1), &LabelKind::Units).size_pt, Some(24.0));
        assert!(index.resolve_label(&s, ColumnId(1), &LabelKind::Comments).is_default());
    }

    #[test]
    fn invalid_and_unsupported_label_defaults_do_not_write_and_sizes_are_bounded() {
        let mut s = sheet();
        for size in [f32::NAN, f32::INFINITY, 0.0, -1.0] {
            assert!(!s.set_label_font_size(&LabelKind::LongName, size));
        }
        for kind in [LabelKind::Formula, LabelKind::Param("Sample".into())] {
            assert!(!s.set_label_font_size(&kind, 18.0));
        }
        assert!(s.font_defaults.is_empty());
        assert!(s.set_label_font_size(&LabelKind::Units, 200.0));
        assert_eq!(s.font_defaults.label_size(&LabelKind::Units), Some(96.0));
        s.font_defaults.comments_size_pt = Some(-1.0);
        assert_eq!(s.format_index().resolve_label(&s, ColumnId(1), &LabelKind::Comments).size_pt, Some(9.0));
    }

    #[test]
    fn label_families_are_independent_of_sizes_data_and_manual_properties() {
        let mut s = sheet();
        s.set_label_font_size(&LabelKind::LongName, 18.0);
        for (kind, name) in [
            (LabelKind::LongName, "Times New Roman"),
            (LabelKind::Units, "Courier New"),
            (LabelKind::Comments, "Arial"),
        ] {
            assert!(s.set_label_font_family(&kind, name));
        }
        s.apply_cell_format(
            FormatTarget::label_rows(vec![ColumnId(1)], vec![LabelKind::LongName]),
            CellFormat {
                font: Some("Courier New".into()),
                size_pt: Some(12.0),
                bold: Some(true),
                ..Default::default()
            },
        );
        s.columns.push(Column::new(ColumnId(9), "B", Designation::Y, ColumnData::default()));
        let index = s.format_index();
        let manual = index.resolve_label(&s, ColumnId(1), &LabelKind::LongName);
        assert_eq!((manual.font, manual.size_pt, manual.bold), (Some("Courier New"), Some(12.0), true));
        let future = index.resolve_label(&s, ColumnId(9), &LabelKind::LongName);
        assert_eq!((future.font, future.size_pt), (Some("Times New Roman"), Some(18.0)));
        for col in [ColumnId(1), ColumnId(9)] {
            assert_eq!(index.resolve_label(&s, col, &LabelKind::Units).font, Some("Courier New"));
            assert_eq!(index.resolve_label(&s, col, &LabelKind::Comments).font, Some("Arial"));
            assert!(index.resolve(&s, col, 0).is_default());
            assert!(index.resolve_label(&s, col, &LabelKind::Formula).is_default());
        }
        let back: Worksheet = serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
        assert_eq!(back.font_defaults, s.font_defaults);
        assert_eq!(back.cell_formats, s.cell_formats);
        assert_eq!(s.columns[0].numeric().unwrap(), &[1.0, 2.0]);
    }

    #[test]
    fn automatic_family_restores_the_size_only_schema_without_extra_records() {
        let mut s = sheet();
        assert!(!s.set_label_font_family(&LabelKind::LongName, "  "));
        assert!(!s.set_label_font_family(&LabelKind::Formula, "Arial"));
        assert!(!s.set_label_font_family(&LabelKind::Param("Sample".into()), "Arial"));
        assert!(s.font_defaults.is_empty());
        s.set_label_font_size(&LabelKind::Units, 12.0);
        let size_only = serde_json::to_value(&s).unwrap();
        assert!(s.set_label_font_family(&LabelKind::Units, " Courier New "));
        assert!(!s.set_label_font_family(&LabelKind::Units, "Courier New"));
        assert_eq!(s.font_defaults.units_family.as_deref(), Some("Courier New"));
        assert_eq!(s.font_defaults.units_size_pt, Some(12.0));
        assert!(s.set_label_font_family(&LabelKind::Units, ""));
        assert!(!s.set_label_font_family(&LabelKind::Units, ""));
        assert_eq!(serde_json::to_value(&s).unwrap(), size_only);
        let back: Worksheet = serde_json::from_value(size_only).unwrap();
        assert_eq!(back.font_defaults, s.font_defaults);
        assert!(s.cell_formats.is_empty());
    }

    #[test]
    fn family_only_labels_reach_print_and_leave_other_text_automatic() {
        use crate::sheet_print::{PrintOp, PrintStyle, SheetTable};
        let mut s = sheet();
        s.columns[0].long_name = "Alpha".into();
        s.columns[0].units = "V".into();
        s.columns[0].width = Some(20);
        s.set_label_font_family(&LabelKind::LongName, "Times New Roman");
        s.set_label_font_family(&LabelKind::Units, "Courier New");
        assert!(!s.format_index().is_empty());
        assert!(s.font_defaults.long_name_size_pt.is_none());
        let table = SheetTable::new(&s, PrintStyle::default());
        let (pages, _) = table.pages_for(&crate::page_setup::PageSetup::default());
        let ops = table.draw_page(&pages[0], pages.len());
        for (text, face) in [("Alpha", "Times New Roman"), ("V", "Courier New")] {
            assert!(ops.iter().any(|op| matches!(op, PrintOp::Text { text: t, size, fmt, .. }
                if t == text && *size == 8.0 && fmt.family.as_deref() == Some(face))));
        }
        assert!(ops.iter().any(|op| matches!(op, PrintOp::Text { text, size, fmt, .. }
            if text == "1" && *size == 8.0 && fmt.family.is_none())));
    }
}
