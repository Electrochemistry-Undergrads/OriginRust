//! The worksheet corner menu's View switches (T92), as Origin 2016's: right-clicking the corner above
//! the row numbers gives View > Long Name, Units, Comments, Parameters, Sampling Interval, Sparklines,
//! Filter, F(x)=, Categories | Row Labels for Column Label Rows | Column Header, Row Header | Column Grid,
//! Row Grid, Page Break Preview Lines. Measured on 2026-11-16 (`docs/spec/workbook-model.md`, "Worksheet
//! corner menu"): a new sheet has Long Name, Units, Comments, F(x)=, Row Labels, both headers and both
//! grids ticked. The switches belong to the sheet.

use crate::{Column, Worksheet};
use serde::{Deserialize, Serialize};

/// One switch of the corner menu that Sheaf has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewSwitch {
    LongName,
    Units,
    Comments,
    Formula,
    SamplingInterval,
    /// The Sparklines row (T175): a small line of the values of each column marked for one.
    Sparklines,
    /// The titles ("Long Name", "Units", ...) to the left of the label rows.
    RowLabels,
    ColumnHeader,
    RowHeader,
    /// The vertical lines between the cells.
    ColumnGrid,
    /// The horizontal lines between the cells.
    RowGrid,
    /// Page Break Preview Lines (T185): dashed lines where the printed pages break.
    PageBreaks,
    /// The Categories row (T187): the categories of each categorical column.
    Categories,
}

impl ViewSwitch {
    pub const LABEL_ROWS: [ViewSwitch; 4] =
        [ViewSwitch::LongName, ViewSwitch::Units, ViewSwitch::Comments, ViewSwitch::Formula];

    /// The menu text, as Origin's.
    pub fn name(self) -> &'static str {
        match self {
            ViewSwitch::LongName => "Long Name",
            ViewSwitch::Units => "Units",
            ViewSwitch::Comments => "Comments",
            ViewSwitch::Formula => "F(x)=",
            ViewSwitch::SamplingInterval => "Sampling Interval",
            ViewSwitch::Sparklines => "Sparklines",
            ViewSwitch::RowLabels => "Row Labels for Column Label Rows",
            ViewSwitch::ColumnHeader => "Column Header",
            ViewSwitch::RowHeader => "Row Header",
            ViewSwitch::ColumnGrid => "Column Grid",
            ViewSwitch::RowGrid => "Row Grid",
            ViewSwitch::PageBreaks => "Page Break Preview Lines",
            ViewSwitch::Categories => "Categories",
        }
    }
}

fn yes() -> bool {
    true
}

/// Explicit grid header sizes as percentages of the 16 px row and 67 px column, and the top gap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeaderMeasurements {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_height: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_width: Option<u16>,
    /// Gap above the worksheet grid in points, independent of the Standard font.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub top_gap_pt: u8,
}

fn is_zero(value: &u8) -> bool {
    *value == 0
}

impl HeaderMeasurements {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub(crate) fn scale(&mut self, ratio: f32) {
        for percent in [&mut self.column_height, &mut self.row_width].into_iter().flatten() {
            *percent = (f32::from(*percent) * ratio).round() as u16;
        }
    }
}

/// Explicit sizes for body rows and default column widths, in grid percentages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellMeasurements {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_height: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_width: Option<u16>,
}

impl CellMeasurements {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// The sheet's display choices. Sampling Interval is automatic until toggled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheetView {
    #[serde(default = "yes")]
    pub long_name: bool,
    #[serde(default = "yes")]
    pub units: bool,
    #[serde(default = "yes")]
    pub comments: bool,
    #[serde(default = "yes")]
    pub formula: bool,
    /// `None` shows the row whenever a column has an interval.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sampling_interval: Option<bool>,
    /// The Sparklines row (T175); off on a new sheet, turned on by Add or Update Sparklines.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sparklines: bool,
    #[serde(default = "yes")]
    pub row_labels: bool,
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    pub column_header: bool,
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    pub row_header: bool,
    #[serde(default = "yes")]
    pub column_grid: bool,
    #[serde(default = "yes")]
    pub row_grid: bool,
    /// Page Break Preview Lines (T185); off on a new sheet.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub page_breaks: bool,
    /// The Categories row (T187); off on a new sheet.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub categories: bool,
    /// None uses Sheaf's original grid and print defaults (T59).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_missing_as_blank: Option<bool>,
    /// The row capacity chosen in Worksheet Properties. Data can extend beyond it (T59).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allocated_rows: Option<usize>,
    /// An explicit Miscellaneous tab setting. Older fixed allocations stay fixed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_add_rows: Option<bool>,
    #[serde(default, skip_serializing_if = "HeaderMeasurements::is_default")]
    pub header_measurements: HeaderMeasurements,
    #[serde(default, skip_serializing_if = "CellMeasurements::is_default")]
    pub cell_measurements: CellMeasurements,
}

impl Default for SheetView {
    fn default() -> Self {
        SheetView {
            long_name: true,
            units: true,
            comments: true,
            formula: true,
            sampling_interval: None,
            sparklines: false,
            row_labels: true,
            column_header: true,
            row_header: true,
            column_grid: true,
            row_grid: true,
            page_breaks: false,
            categories: false,
            show_missing_as_blank: None,
            allocated_rows: None,
            auto_add_rows: None,
            header_measurements: HeaderMeasurements::default(),
            cell_measurements: CellMeasurements::default(),
        }
    }
}

impl SheetView {
    pub fn is_default(&self) -> bool {
        *self == SheetView::default()
    }

    pub fn automatic_rows(&self) -> bool {
        self.auto_add_rows.unwrap_or(self.allocated_rows.is_none())
    }

    /// The presented value of a data cell. Literal text survives a missing-value display change.
    pub fn data_text(&self, column: &Column, row: usize, blank_by_default: bool) -> String {
        let text = column.shown_text(row);
        if self.show_missing_as_blank.unwrap_or(blank_by_default)
            && text == "--"
            && column.data.is_numeric()
            && column.data.text_cell(row).is_none()
        {
            String::new()
        } else {
            text
        }
    }

    fn slot(&mut self, s: ViewSwitch) -> Option<&mut bool> {
        Some(match s {
            ViewSwitch::LongName => &mut self.long_name,
            ViewSwitch::Units => &mut self.units,
            ViewSwitch::Comments => &mut self.comments,
            ViewSwitch::Formula => &mut self.formula,
            ViewSwitch::SamplingInterval => return None,
            ViewSwitch::Sparklines => &mut self.sparklines,
            ViewSwitch::RowLabels => &mut self.row_labels,
            ViewSwitch::ColumnHeader => &mut self.column_header,
            ViewSwitch::RowHeader => &mut self.row_header,
            ViewSwitch::ColumnGrid => &mut self.column_grid,
            ViewSwitch::RowGrid => &mut self.row_grid,
            ViewSwitch::PageBreaks => &mut self.page_breaks,
            ViewSwitch::Categories => &mut self.categories,
        })
    }
}

fn is_yes(value: &bool) -> bool {
    *value
}

impl Worksheet {
    pub fn shows_view_switch(&self, s: ViewSwitch) -> bool {
        let mut view = self.view;
        view.slot(s).map(|shown| *shown).unwrap_or_else(|| {
            self.view.sampling_interval.unwrap_or_else(|| self.columns.iter().any(|c| c.sampling.is_some()))
        })
    }

    /// Turns a switch over, as clicking its menu item does.
    pub fn toggle_view_switch(&mut self, s: ViewSwitch) {
        let shown = !self.shows_view_switch(s);
        if let Some(slot) = self.view.slot(s) {
            *slot = shown;
        } else {
            self.view.sampling_interval = Some(shown);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_measurements_roundtrip_and_stay_out_of_print_geometry() {
        use crate::sheet_print::{PrintStyle, SheetTable};

        let mut sheet = Worksheet::new("Sheet1");
        let json = serde_json::to_string(&sheet).unwrap();
        assert!(!json.contains("header_measurements"));
        let print_extent = SheetTable::new(&sheet, PrintStyle::default()).heading_extent();
        sheet.view.header_measurements =
            HeaderMeasurements { column_height: Some(320), row_width: Some(0), top_gap_pt: 20 };

        let json = serde_json::to_string(&sheet).unwrap();
        let saved: Worksheet = serde_json::from_str(&json).unwrap();
        assert_eq!(saved.view.header_measurements, sheet.view.header_measurements);
        assert!(saved.view.row_header && saved.view.column_header);
        assert_eq!(SheetTable::new(&saved, PrintStyle::default()).heading_extent(), print_extent);
    }

    #[test]
    fn auto_add_rows_defaults_respect_existing_allocations() {
        let mut view: SheetView = serde_json::from_str("{}").unwrap();
        assert!(view.automatic_rows());
        view.allocated_rows = Some(3);
        assert!(!view.automatic_rows());
        view.auto_add_rows = Some(true);
        let saved: SheetView = serde_json::from_str(&serde_json::to_string(&view).unwrap()).unwrap();
        assert!(saved.automatic_rows());
        assert_eq!(saved.allocated_rows, Some(3));
    }

    #[test]
    fn allocated_rows_roundtrip_without_counting_as_data() {
        let mut sheet = Worksheet::new("Sheet1");
        let before = serde_json::to_string(&sheet).unwrap();
        assert!(!before.contains("allocated_rows"));
        sheet.view.allocated_rows = Some(1_000_000);
        let saved = serde_json::to_string(&sheet).unwrap();
        let back: Worksheet = serde_json::from_str(&saved).unwrap();
        assert_eq!(back.view.allocated_rows, Some(1_000_000));
        assert_eq!(back.row_count(), 0);
    }

    #[test]
    fn everything_shows_on_a_new_sheet_and_toggles() {
        let mut sheet = Worksheet::new("Sheet1");
        for s in [ViewSwitch::RowLabels, ViewSwitch::ColumnGrid, ViewSwitch::RowGrid]
            .into_iter()
            .chain(ViewSwitch::LABEL_ROWS)
        {
            assert!(sheet.shows_view_switch(s), "{s:?}");
        }
        sheet.toggle_view_switch(ViewSwitch::Units);
        assert!(!sheet.shows_view_switch(ViewSwitch::Units) && !sheet.view.is_default());
        let json = serde_json::to_string(&sheet.view).unwrap();
        let back: SheetView = serde_json::from_str(&json).unwrap();
        assert_eq!(back, sheet.view);
        // A file written before a switch existed reads it as on.
        let old: SheetView = serde_json::from_str("{}").unwrap();
        assert!(old.is_default());
        sheet.toggle_view_switch(ViewSwitch::Units);
        assert!(sheet.view.is_default());
    }

    #[test]
    fn header_switches_default_to_visible_and_persist_independently() {
        let mut sheet = Worksheet::new("Sheet1");
        sheet.view = serde_json::from_str("{}").unwrap();
        let (column, row) = (ViewSwitch::ColumnHeader, ViewSwitch::RowHeader);
        assert!(sheet.shows_view_switch(column) && sheet.shows_view_switch(row));
        sheet.toggle_view_switch(column);
        let json = serde_json::to_string(&sheet).unwrap();
        let mut opened: Worksheet = serde_json::from_str(&json).unwrap();
        assert!(!opened.shows_view_switch(column) && opened.shows_view_switch(row));
        assert!(opened.view.row_labels && !opened.view.is_default());
        opened.toggle_view_switch(row);
        opened.toggle_view_switch(column);
        assert!(opened.shows_view_switch(column) && !opened.shows_view_switch(row));
    }

    #[test]
    fn sampling_interval_visibility_defaults_to_automatic_and_persists_an_override() {
        use crate::{ColumnData, Designation, Project};
        let mut project = Project::new();
        let book = project.add_workbook("Book1");
        let column = project.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![3.0])).unwrap();
        let switch = ViewSwitch::SamplingInterval;
        assert!(!project.locate(column).unwrap().1.shows_view_switch(switch));
        project.set_sampling_interval(&[column], 0.0, 0.5, "Time", "s").unwrap();
        let sheet = &mut project.workbook_mut(book).unwrap().sheets[0];
        assert!(sheet.shows_view_switch(switch) && sheet.view.is_default());

        sheet.toggle_view_switch(switch);
        let saved = serde_json::to_string(sheet).unwrap();
        let mut back: Worksheet = serde_json::from_str(&saved).unwrap();
        assert_eq!(back.view.sampling_interval, Some(false));
        assert!(!back.shows_view_switch(switch));
        assert!(!back.view.is_default());
        back.toggle_view_switch(switch);
        back.columns[0].sampling = None;
        assert!(back.shows_view_switch(switch), "an explicitly displayed empty row stays visible");
        back.view = serde_json::from_str("{}").unwrap();
        assert!(!back.shows_view_switch(switch));
        back.columns[0].sampling = sheet.columns[0].sampling.clone();
        assert!(back.shows_view_switch(switch), "legacy view settings use automatic visibility");
    }
}
