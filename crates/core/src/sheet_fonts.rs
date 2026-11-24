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

/// A worksheet category font whose family and point size can inherit independently.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ColumnLabelFont {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub family: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_pt: Option<f32>,
}

impl ColumnLabelFont {
    pub fn is_empty(&self) -> bool {
        self.family.is_empty() && self.size_pt.is_none()
    }

    pub fn style(&self) -> ResolvedStyle<'_> {
        ResolvedStyle {
            font: (!self.family.is_empty()).then_some(self.family.as_str()),
            size_pt: self
                .size_pt
                .map(|size| clamp_cell_pt(if size.is_finite() && size > 0.0 { size } else { default_size() })),
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorksheetFonts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<ColumnLabelFont>,
    #[serde(default, skip_serializing_if = "ColumnLabelFont::is_empty")]
    pub standard: ColumnLabelFont,
    #[serde(default, skip_serializing_if = "ColumnLabelFont::is_empty")]
    pub column_labels: ColumnLabelFont,
    #[serde(default, skip_serializing_if = "ColumnLabelFont::is_empty")]
    pub column_header: ColumnLabelFont,
    #[serde(default, skip_serializing_if = "ColumnLabelFont::is_empty")]
    pub row_header: ColumnLabelFont,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_name_size_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub units_size_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comments_size_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula_size_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_name_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub units_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comments_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula_family: Option<String>,
}

impl WorksheetFonts {
    pub fn is_empty(&self) -> bool {
        !self.has_cell_fonts() && self.column_header.is_empty() && self.row_header.is_empty()
    }

    pub fn has_data_fonts(&self) -> bool {
        self.data.is_some() || !self.standard.is_empty()
    }

    pub fn has_cell_fonts(&self) -> bool {
        self.has_data_fonts()
            || !self.column_labels.is_empty()
            || self.long_name_size_pt.is_some()
            || self.units_size_pt.is_some()
            || self.comments_size_pt.is_some()
            || self.formula_size_pt.is_some()
            || self.long_name_family.is_some()
            || self.units_family.is_some()
            || self.comments_family.is_some()
            || self.formula_family.is_some()
    }

    /// The Standard point size sets the sheet's base row height and character width.
    pub fn size_scale(&self) -> f32 {
        self.standard.style().size_pt.unwrap_or_else(default_size) / default_size()
    }

    pub fn column_header_style(&self) -> ResolvedStyle<'_> {
        inherit_font(self.column_header.style(), self.standard.style())
    }

    pub fn row_header_style(&self) -> ResolvedStyle<'_> {
        inherit_font(self.row_header.style(), self.standard.style())
    }

    pub fn label_size(&self, kind: &LabelKind) -> Option<f32> {
        match kind {
            LabelKind::LongName => self.long_name_size_pt,
            LabelKind::Units => self.units_size_pt,
            LabelKind::Comments => self.comments_size_pt,
            LabelKind::Formula => self.formula_size_pt,
            _ => None,
        }
        .or_else(|| self.column_labels_for(kind).and_then(|font| font.size_pt))
        .or_else(|| self.standard_for_label(kind).and_then(|font| font.size_pt))
    }

    pub fn label_family(&self, kind: &LabelKind) -> Option<&str> {
        match kind {
            LabelKind::LongName => self.long_name_family.as_deref(),
            LabelKind::Units => self.units_family.as_deref(),
            LabelKind::Comments => self.comments_family.as_deref(),
            LabelKind::Formula => self.formula_family.as_deref(),
            _ => None,
        }
        .filter(|name| !name.is_empty())
        .or_else(|| self.column_labels_for(kind).and_then(|font| font.style().font))
        .or_else(|| self.standard_for_label(kind).and_then(|font| font.style().font))
    }

    fn standard_for_label(&self, kind: &LabelKind) -> Option<&ColumnLabelFont> {
        (!matches!(kind, LabelKind::Sparklines)).then_some(&self.standard)
    }

    fn column_labels_for(&self, kind: &LabelKind) -> Option<&ColumnLabelFont> {
        kind.uses_column_label_defaults().then_some(&self.column_labels)
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

    pub(crate) fn data_style<'a>(&'a self, manual: ResolvedStyle<'a>) -> ResolvedStyle<'a> {
        let data = self.data.as_ref().map(ColumnLabelFont::style).unwrap_or_default();
        inherit_font(inherit_font(manual, data), self.standard.style())
    }
}

fn inherit_font<'a>(mut style: ResolvedStyle<'a>, fallback: ResolvedStyle<'a>) -> ResolvedStyle<'a> {
    style.font = style.font.or(fallback.font);
    style.size_pt = style.size_pt.or(fallback.size_pt);
    style
}

impl Worksheet {
    pub fn set_standard_font(&mut self, font: ColumnLabelFont) -> bool {
        let previous_scale = self.font_defaults.size_scale();
        if !set_category_font(&mut self.font_defaults.standard, font) {
            return false;
        }
        self.view.header_measurements.scale(self.font_defaults.size_scale() / previous_scale);
        true
    }

    /// Changes the common font under the independent column-label categories.
    pub fn set_column_labels_font(&mut self, font: ColumnLabelFont) -> bool {
        set_category_font(&mut self.font_defaults.column_labels, font)
    }

    pub fn set_column_header_font(&mut self, font: ColumnLabelFont) -> bool {
        set_category_font(&mut self.font_defaults.column_header, font)
    }

    pub fn set_row_header_font(&mut self, font: ColumnLabelFont) -> bool {
        set_category_font(&mut self.font_defaults.row_header, font)
    }

    /// Changes one label category's family. Empty names restore the automatic face;
    /// point sizes and manual properties remain independent.
    pub fn set_label_font_family(&mut self, kind: &LabelKind, family: &str) -> bool {
        let slot = match kind {
            LabelKind::LongName => &mut self.font_defaults.long_name_family,
            LabelKind::Units => &mut self.font_defaults.units_family,
            LabelKind::Comments => &mut self.font_defaults.comments_family,
            LabelKind::Formula => &mut self.font_defaults.formula_family,
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
        self.set_label_font_size_override(kind, Some(size_pt))
    }

    /// `None` makes the category inherit the shared Column Labels size again.
    pub fn set_label_font_size_override(&mut self, kind: &LabelKind, size_pt: Option<f32>) -> bool {
        if size_pt.is_some_and(|size| !size.is_finite() || size <= 0.0) {
            return false;
        }
        let slot = match kind {
            LabelKind::LongName => &mut self.font_defaults.long_name_size_pt,
            LabelKind::Units => &mut self.font_defaults.units_size_pt,
            LabelKind::Comments => &mut self.font_defaults.comments_size_pt,
            LabelKind::Formula => &mut self.font_defaults.formula_size_pt,
            _ => return false,
        };
        let size = size_pt.map(clamp_cell_pt);
        if *slot == size {
            return false;
        }
        *slot = size;
        true
    }

    /// Changes the Data font of this sheet, including rows/columns subsequently added.
    /// Per-cell/range fonts remain above this layer; this does not create per-row records.
    pub fn set_data_font(&mut self, font: WorksheetFont) -> bool {
        self.set_data_font_override(ColumnLabelFont { family: font.family, size_pt: Some(font.size_pt) })
    }

    /// Properties left empty inherit Standard independently.
    pub fn set_data_font_override(&mut self, font: ColumnLabelFont) -> bool {
        let Some(font) = normalized_font(font) else { return false };
        let next = (!font.is_empty()).then_some(font);
        if self.font_defaults.data == next {
            return false;
        }
        self.font_defaults.data = next;
        true
    }
}

fn normalized_font(mut font: ColumnLabelFont) -> Option<ColumnLabelFont> {
    if font.size_pt.is_some_and(|size| !size.is_finite() || size <= 0.0) {
        return None;
    }
    font.family = font.family.trim().to_owned();
    font.size_pt = font.size_pt.map(clamp_cell_pt);
    Some(font)
}

fn set_category_font(slot: &mut ColumnLabelFont, font: ColumnLabelFont) -> bool {
    let Some(font) = normalized_font(font) else { return false };
    if *slot == font {
        return false;
    }
    *slot = font;
    true
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
    fn standard_font_is_inherited_property_by_property_and_sets_base_geometry() {
        use crate::sheet_print::{PrintStyle, SheetTable};
        let mut s = sheet();
        s.columns[0].long_name = "Alpha".into();
        let before = SheetTable::new(&s, PrintStyle::default()).column_widths();
        let bold = CellFormat { bold: Some(true), size_pt: Some(10.0), ..Default::default() };
        s.apply_cell_format(FormatTarget::cells(vec![ColumnId(1)], 0, Some(0)), bold);
        assert!(s.set_column_header_font(ColumnLabelFont { family: String::new(), size_pt: Some(12.0) }));
        assert!(s.set_standard_font(ColumnLabelFont { family: "Courier New".into(), size_pt: Some(18.0) }));
        assert!(s.set_column_labels_font(ColumnLabelFont { family: String::new(), size_pt: Some(11.0) }));
        assert!(s.set_data_font_override(ColumnLabelFont { family: "Arial".into(), size_pt: None }));
        s.columns.push(Column::new(ColumnId(2), "B", Designation::Y, ColumnData::default()));

        let saved = serde_json::to_value(&s).unwrap();
        assert!(saved["font_defaults"]["data"].get("size_pt").is_none());
        let back: Worksheet = serde_json::from_value(saved).unwrap();
        assert_eq!(back.font_defaults, s.font_defaults);

        let manual = s.cell_format_at(ColumnId(1), 0);
        assert_eq!((manual.font, manual.size_pt, manual.bold), (Some("Arial"), Some(10.0), true));
        let future = s.cell_format_at(ColumnId(2), 999_999);
        assert_eq!((future.font, future.size_pt), (Some("Arial"), Some(18.0)));
        let label = s.format_index().resolve_label(&s, ColumnId(2), &LabelKind::LongName);
        assert_eq!((label.font, label.size_pt), (Some("Courier New"), Some(11.0)));
        let header = s.font_defaults.column_header_style();
        assert_eq!((header.font, header.size_pt), (Some("Courier New"), Some(12.0)));
        let rows = s.font_defaults.row_header_style();
        assert_eq!((rows.font, rows.size_pt), (Some("Courier New"), Some(18.0)));
        assert_eq!(s.font_defaults.size_scale(), 2.0);
        let table = SheetTable::new(&s, PrintStyle::default());
        assert_eq!(table.column_widths(), [before[0] * 2.0; 2]);

        assert!(s.set_standard_font(ColumnLabelFont::default()));
        assert_eq!(s.cell_format_at(ColumnId(2), 1).size_pt, None);
        assert_eq!(s.font_defaults.column_header_style().size_pt, Some(12.0));
        assert_eq!(s.font_defaults.size_scale(), 1.0);
    }

    #[test]
    fn header_fonts_are_sparse_and_style_only_the_printed_headings() {
        use crate::sheet_print::{Align, PrintOp, PrintStyle, SheetTable};
        let mut s = sheet();
        s.columns[0].long_name = "Alpha".into();
        assert!(s.set_column_header_font(ColumnLabelFont { family: String::new(), size_pt: Some(18.0) }));
        assert!(s.set_row_header_font(ColumnLabelFont { family: " Courier New ".into(), size_pt: Some(13.5) }));
        let back: Worksheet = serde_json::from_value(serde_json::to_value(&s).unwrap()).unwrap();
        assert_eq!(back.font_defaults, s.font_defaults);
        assert!(s.format_index().is_empty(), "header fonts are independent of cell fonts");

        let table = SheetTable::new(&s, PrintStyle::default());
        let (pages, _) = table.pages_for(&crate::page_setup::PageSetup::default());
        let ops = table.draw_page(&pages[0], pages.len());
        assert!(table.head_height() > 24.0);
        for (label, align, expected_size, family) in [
            ("A(Y1)", Align::Center, 16.0, None),
            ("Long Name", Align::Right, 12.0, Some("Courier New")),
            ("2", Align::Right, 12.0, Some("Courier New")),
            ("Alpha", Align::Left, 8.0, None),
        ] {
            assert!(ops.iter().any(|op| matches!(op, PrintOp::Text { text, align: view, size, fmt, .. }
                if text == label && *view == align && *size == expected_size && fmt.family.as_deref() == family)));
        }
        assert!(s.set_row_header_font(ColumnLabelFont::default()));
        assert!(s.set_column_header_font(ColumnLabelFont::default()));
        assert!(serde_json::to_value(s).unwrap().get("font_defaults").is_none());
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
    fn bulk_label_fonts_cascade_below_specific_and_manual_formats_and_roundtrip() {
        let mut s = sheet();
        let a = ColumnId(1);
        let b = ColumnId(9);
        assert!(s.set_label_font_size(&LabelKind::LongName, 24.0));
        assert!(s.set_label_font_family(&LabelKind::Units, "Courier New"));
        let common = ColumnLabelFont { family: "Times New Roman".into(), size_pt: Some(20.0) };
        assert!(s.set_column_labels_font(common.clone()));
        assert!(!s.set_column_labels_font(common));
        s.apply_cell_format(
            FormatTarget::label_rows(vec![a], vec![LabelKind::LongName]),
            CellFormat { size_pt: Some(12.0), bold: Some(true), ..Default::default() },
        );
        s.columns.push(Column::new(b, "B", Designation::Y, ColumnData::default()));
        let mut back: Worksheet = serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
        assert_eq!(back.font_defaults, s.font_defaults);
        let index = back.format_index();
        let manual = index.resolve_label(&back, a, &LabelKind::LongName);
        assert_eq!((manual.size_pt, manual.font, manual.bold), (Some(12.0), Some("Times New Roman"), true));
        let specific = index.resolve_label(&back, b, &LabelKind::LongName);
        assert_eq!((specific.size_pt, specific.font), (Some(24.0), Some("Times New Roman")));
        let units = index.resolve_label(&back, b, &LabelKind::Units);
        assert_eq!((units.size_pt, units.font), (Some(20.0), Some("Courier New")));
        for kind in [LabelKind::Comments, LabelKind::Formula, LabelKind::Param("Sample".into())] {
            let inherited = index.resolve_label(&back, b, &kind);
            assert_eq!((inherited.size_pt, inherited.font), (Some(20.0), Some("Times New Roman")));
        }
        assert!(index.resolve(&back, b, 0).is_default());
        assert!(index.resolve_label(&back, b, &LabelKind::Sparklines).is_default());

        assert!(back.set_label_font_size_override(&LabelKind::LongName, None));
        assert_eq!(back.font_defaults.label_size(&LabelKind::LongName), Some(20.0));
        assert!(back.set_column_labels_font(ColumnLabelFont::default()));
        assert_eq!(back.font_defaults.label_size(&LabelKind::LongName), None);
        assert_eq!(back.font_defaults.label_family(&LabelKind::Units), Some("Courier New"));
    }

    #[test]
    fn formula_font_inherits_independently_and_respects_manual_formatting() {
        let mut s = sheet();
        let a = ColumnId(1);
        let b = ColumnId(2);
        s.set_column_labels_font(ColumnLabelFont { family: "Times New Roman".into(), size_pt: Some(12.0) });
        assert!(s.set_label_font_size(&LabelKind::Formula, 18.0));
        assert!(s.set_label_font_family(&LabelKind::Formula, "Courier New"));
        s.apply_cell_format(
            FormatTarget::label_rows(vec![a], vec![LabelKind::Formula]),
            CellFormat { size_pt: Some(10.0), ..Default::default() },
        );
        s.columns.push(Column::new(b, "B", Designation::Y, ColumnData::default()));

        let mut back: Worksheet = serde_json::from_value(serde_json::to_value(&s).unwrap()).unwrap();
        assert_eq!(back.font_defaults, s.font_defaults);
        let index = back.format_index();
        let manual = index.resolve_label(&back, a, &LabelKind::Formula);
        assert_eq!((manual.size_pt, manual.font), (Some(10.0), Some("Courier New")));
        let future = index.resolve_label(&back, b, &LabelKind::Formula);
        assert_eq!((future.size_pt, future.font), (Some(18.0), Some("Courier New")));
        let units = index.resolve_label(&back, b, &LabelKind::Units);
        assert_eq!((units.size_pt, units.font), (Some(12.0), Some("Times New Roman")));
        assert!(index.resolve(&back, b, 0).is_default());

        assert!(back.set_label_font_family(&LabelKind::Formula, ""));
        assert_eq!(back.font_defaults.label_family(&LabelKind::Formula), Some("Times New Roman"));
        assert_eq!(back.font_defaults.label_size(&LabelKind::Formula), Some(18.0));
        assert!(back.set_label_font_size_override(&LabelKind::Formula, None));
        assert_eq!(back.font_defaults.label_size(&LabelKind::Formula), Some(12.0));
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
        assert!(!s.set_label_font_size(&LabelKind::Param("Sample".into()), 18.0));
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
        assert!(!s.set_label_font_family(&LabelKind::Param("Sample".into()), "Arial"));
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
