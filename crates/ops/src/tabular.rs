//! Worksheets as delimited text and Excel workbooks.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sheaf_core::{Project, WindowId, Worksheet};
use sheaf_io::excel::{ExcelExportOptions, ExportOutcome};
use std::path::Path;

/// The delimiter of an exported sheet.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SheetFormat {
    Csv,
    Tsv,
}

impl SheetFormat {
    /// Tabs for `.tsv` and `.txt`, commas otherwise.
    pub fn from_path(path: &Path) -> Self {
        let tabs = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("tsv") || e.eq_ignore_ascii_case("txt"));
        if tabs { SheetFormat::Tsv } else { SheetFormat::Csv }
    }
}

/// A sheet as delimited text.
pub fn sheet_to_bytes(ws: &Worksheet, format: SheetFormat) -> Result<Vec<u8>> {
    let opts = match format {
        SheetFormat::Tsv => sheaf_io::export::AsciiExportOptions::default(),
        SheetFormat::Csv => sheaf_io::export::AsciiExportOptions::csv(),
    };
    let mut out = Vec::new();
    sheaf_io::export::write_delimited(ws, &mut out, &opts).context("could not write the sheet")?;
    Ok(out)
}

/// Writes a sheet as delimited text: tabs for .tsv and .txt, commas otherwise.
pub fn write_sheet(ws: &Worksheet, path: &Path) -> Result<()> {
    let opts = match SheetFormat::from_path(path) {
        SheetFormat::Tsv => sheaf_io::export::AsciiExportOptions::default(),
        SheetFormat::Csv => sheaf_io::export::AsciiExportOptions::csv(),
    };
    let file = std::fs::File::create(path).with_context(|| format!("could not create '{}'", path.display()))?;
    let mut out = std::io::BufWriter::new(file);
    sheaf_io::export::write_delimited(ws, &mut out, &opts)
        .with_context(|| format!("could not write '{}'", path.display()))?;
    Ok(())
}

/// True when the path asks for an Excel workbook (`.xlsx`).
pub fn is_xlsx_path(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("xlsx"))
}

/// What goes into an `.xlsx` file.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct XlsxRequest {
    /// Every sheet of the workbook, one Excel sheet each. Otherwise one sheet.
    pub all_sheets: bool,
    pub no_long_names: bool,
    pub no_units: bool,
    pub no_comments: bool,
    /// A first row of short names.
    pub short_names: bool,
}

impl XlsxRequest {
    pub fn options(&self) -> ExcelExportOptions {
        ExcelExportOptions {
            long_names: !self.no_long_names,
            units: !self.no_units,
            comments: !self.no_comments,
            short_names: self.short_names,
        }
    }
}

/// Writes a sheet of a workbook, or all its sheets, as an `.xlsx` file. `sheet` is a sheet name;
/// without it the workbook's active sheet is written, and with `all_sheets` every sheet.
pub fn write_xlsx(
    project: &Project,
    book: WindowId,
    sheet: Option<&str>,
    request: &XlsxRequest,
    path: &Path,
) -> Result<ExportOutcome> {
    let b = project.workbook(book).context("workbook not found")?;
    let outcome = if request.all_sheets && sheet.is_none() {
        sheaf_io::excel::write_workbook_xlsx(b, path, &request.options())
    } else {
        let picked = crate::pick::pick_sheet(project, book, sheet)?;
        let ws = b.sheets.get(picked).with_context(|| format!("workbook '{}' has no sheet", b.name))?;
        sheaf_io::excel::write_xlsx(&[ws], path, &request.options())
    };
    outcome.with_context(|| format!("could not write '{}'", path.display()))
}
