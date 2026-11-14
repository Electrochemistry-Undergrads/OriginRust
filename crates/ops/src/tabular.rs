//! Worksheets as delimited text.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sheaf_core::Worksheet;
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
