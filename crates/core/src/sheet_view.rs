//! The worksheet corner menu's View switches (T92), as Origin 2016's: right-clicking the corner above
//! the row numbers gives View > Long Name, Units, Comments, Parameters, Sampling Interval, Sparklines,
//! Filter, F(x)=, Categories | Row Labels for Column Label Rows | Column Header, Row Header | Column Grid,
//! Row Grid, Page Break Preview Lines. Measured on 2026-11-16 (`docs/spec/workbook-model.md`, "Worksheet
//! corner menu"): a new sheet has Long Name, Units, Comments, F(x)=, Row Labels, both headers and both
//! grids ticked. The switches belong to the sheet.

use serde::{Deserialize, Serialize};

/// One switch of the corner menu that Sheaf has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewSwitch {
    LongName,
    Units,
    Comments,
    Formula,
    /// The titles ("Long Name", "Units", ...) to the left of the label rows.
    RowLabels,
    /// The vertical lines between the cells.
    ColumnGrid,
    /// The horizontal lines between the cells.
    RowGrid,
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
            ViewSwitch::RowLabels => "Row Labels for Column Label Rows",
            ViewSwitch::ColumnGrid => "Column Grid",
            ViewSwitch::RowGrid => "Row Grid",
        }
    }
}

fn yes() -> bool {
    true
}

/// What the sheet shows; everything on for a new sheet.
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
    #[serde(default = "yes")]
    pub row_labels: bool,
    #[serde(default = "yes")]
    pub column_grid: bool,
    #[serde(default = "yes")]
    pub row_grid: bool,
}

impl Default for SheetView {
    fn default() -> Self {
        SheetView {
            long_name: true,
            units: true,
            comments: true,
            formula: true,
            row_labels: true,
            column_grid: true,
            row_grid: true,
        }
    }
}

impl SheetView {
    pub fn is_default(&self) -> bool {
        *self == SheetView::default()
    }

    fn slot(&mut self, s: ViewSwitch) -> &mut bool {
        match s {
            ViewSwitch::LongName => &mut self.long_name,
            ViewSwitch::Units => &mut self.units,
            ViewSwitch::Comments => &mut self.comments,
            ViewSwitch::Formula => &mut self.formula,
            ViewSwitch::RowLabels => &mut self.row_labels,
            ViewSwitch::ColumnGrid => &mut self.column_grid,
            ViewSwitch::RowGrid => &mut self.row_grid,
        }
    }

    pub fn shows(&self, s: ViewSwitch) -> bool {
        let mut copy = *self;
        *copy.slot(s)
    }

    /// Turns a switch over, as clicking its menu item does.
    pub fn toggle(&mut self, s: ViewSwitch) {
        let v = self.slot(s);
        *v = !*v;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_shows_on_a_new_sheet_and_toggles() {
        let mut v = SheetView::default();
        for s in [ViewSwitch::RowLabels, ViewSwitch::ColumnGrid, ViewSwitch::RowGrid]
            .into_iter()
            .chain(ViewSwitch::LABEL_ROWS)
        {
            assert!(v.shows(s), "{s:?}");
        }
        v.toggle(ViewSwitch::Units);
        assert!(!v.shows(ViewSwitch::Units) && !v.is_default());
        let json = serde_json::to_string(&v).unwrap();
        let back: SheetView = serde_json::from_str(&json).unwrap();
        assert_eq!(back, v);
        // A file written before a switch existed reads it as on.
        let old: SheetView = serde_json::from_str("{}").unwrap();
        assert!(old.is_default());
        v.toggle(ViewSwitch::Units);
        assert!(v.is_default());
    }
}
