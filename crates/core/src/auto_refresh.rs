//! File > Import > Auto Refresh... (T442; Origin 2019b's Data Connectors and Origin 2026's periodic refresh, Sheaf's
//! own design): an imported workbook keeps its link to the source file and re-imports it on a timer, as
//! File > Import > Re-Import Directly does (T98), optionally only when the file has changed since the last read. The
//! timer can be paused; the setting is saved with the workbook.
//!
//! Sheaf's choices (Origin 2026 not measured here, Origin 2016 has none): 5 seconds to 1 day between checks, 60 by
//! default; "only when changed" on by default (the file's modification time); a refresh that fails pauses the
//! timer so a missing file is not read every few seconds.

use crate::ids::WindowId;
use crate::project::{Project, ProjectError};
use serde::{Deserialize, Serialize};

/// The shortest and longest time between two checks, in seconds.
pub const MIN_SECONDS: u32 = 5;
pub const MAX_SECONDS: u32 = 86_400;

/// A workbook's auto refresh (in [`crate::Workbook::refresh`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoRefresh {
    /// Seconds between two checks of the source.
    pub every_seconds: u32,
    /// Re-import only when the file's modification time differs from the last read.
    #[serde(default = "yes")]
    pub only_when_changed: bool,
    #[serde(default)]
    pub paused: bool,
}

fn yes() -> bool {
    true
}

impl Default for AutoRefresh {
    fn default() -> Self {
        AutoRefresh { every_seconds: 60, only_when_changed: true, paused: false }
    }
}

impl Project {
    /// Sets (or with `None` stops) workbook `book`'s auto refresh; the interval is kept within
    /// [`MIN_SECONDS`]..=[`MAX_SECONDS`]. Refused for a workbook that was not imported from a file.
    pub fn set_auto_refresh(&mut self, book: WindowId, refresh: Option<AutoRefresh>) -> Result<(), ProjectError> {
        let b = self.workbook_mut(book).ok_or(ProjectError::NoSuchWindow(book))?;
        if refresh.is_some() && b.source.is_none() {
            return Err(ProjectError::NotImported);
        }
        b.refresh =
            refresh.map(|r| AutoRefresh { every_seconds: r.every_seconds.clamp(MIN_SECONDS, MAX_SECONDS), ..r });
        Ok(())
    }

    /// The workbooks whose auto refresh runs (not paused), with their settings.
    pub fn refreshing_books(&self) -> Vec<(WindowId, AutoRefresh)> {
        self.workbooks()
            .filter_map(|b| b.refresh.filter(|r| !r.paused && b.source.is_some()).map(|r| (b.id, r)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};
    use crate::worksheet::ImportSource;

    #[test]
    fn only_imported_books_refresh_and_the_interval_is_kept_in_range() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap();
        assert!(p.set_auto_refresh(b, Some(AutoRefresh::default())).is_err(), "not imported");
        p.workbook_mut(b).unwrap().source = Some(ImportSource { path: "run.csv".into(), importer: "ascii".into() });
        p.set_auto_refresh(b, Some(AutoRefresh { every_seconds: 1, ..Default::default() })).unwrap();
        assert_eq!(p.workbook(b).unwrap().refresh.unwrap().every_seconds, MIN_SECONDS);
        assert_eq!(p.refreshing_books().len(), 1);
        p.set_auto_refresh(b, Some(AutoRefresh { paused: true, ..Default::default() })).unwrap();
        assert!(p.refreshing_books().is_empty(), "paused");
        let back: Project = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(back.workbook(b).unwrap().refresh, p.workbook(b).unwrap().refresh);
        p.set_auto_refresh(b, None).unwrap();
        assert!(p.workbook(b).unwrap().refresh.is_none());
    }
}
