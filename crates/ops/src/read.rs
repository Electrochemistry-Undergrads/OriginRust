//! Reading one data file with the import options.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sheaf_io::ascii::{AsciiOptions, Delimiter, Thousands};
use sheaf_io::chi::ChiOptions;
use sheaf_io::{ImportKind, Imported};
use std::path::Path;

/// Which reader to use.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataFormat {
    #[default]
    Auto,
    Chi,
    Ascii,
}

impl DataFormat {
    pub fn kind(self) -> ImportKind {
        match self {
            DataFormat::Auto => ImportKind::Auto,
            DataFormat::Chi => ImportKind::Chi,
            DataFormat::Ascii => ImportKind::Ascii,
        }
    }
}

/// The thousands separator of numbers in a text file.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThousandsSetting {
    #[default]
    Auto,
    None,
    Comma,
    Dot,
    Space,
    Apostrophe,
}

/// The import options of both readers. The defaults are those of the command line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReadSettings {
    pub format: DataFormat,
    pub no_split_scans: bool,
    pub no_milliamp: bool,
    pub no_charge: bool,
    pub invert_current: bool,
    pub no_split_segments: bool,
    pub keep_file_times: bool,
    pub delimiter: Option<String>,
    pub delimiter_chars: Option<String>,
    pub combine_delimiters: bool,
    pub fixed_width: Vec<usize>,
    pub header_lines: Option<usize>,
    pub first_row: usize,
    pub last_row: usize,
    pub read_rows: usize,
    pub skip_rows: usize,
    pub first_column: usize,
    pub last_column: usize,
    pub read_columns: usize,
    pub skip_columns: usize,
    pub decimal_comma: bool,
    pub dates_as_text: bool,
    pub day_first: bool,
    pub thousands: ThousandsSetting,
    pub long_name_line: Option<usize>,
    pub units_line: Option<usize>,
    pub comments_line: Option<usize>,
    pub param_lines: Vec<usize>,
    pub short_name_line: Option<usize>,
    pub no_rename_columns: bool,
    pub no_rename_sheet: bool,
    pub no_rename_book: bool,
    pub blocks_as_sheets: bool,
}

impl Default for ReadSettings {
    fn default() -> Self {
        Self {
            format: DataFormat::Auto,
            no_split_scans: false,
            no_milliamp: false,
            no_charge: false,
            invert_current: false,
            no_split_segments: false,
            keep_file_times: false,
            delimiter: None,
            delimiter_chars: None,
            combine_delimiters: false,
            fixed_width: Vec::new(),
            header_lines: None,
            first_row: 1,
            last_row: 0,
            read_rows: 1,
            skip_rows: 0,
            first_column: 1,
            last_column: 0,
            read_columns: 1,
            skip_columns: 0,
            decimal_comma: false,
            dates_as_text: false,
            day_first: false,
            thousands: ThousandsSetting::Auto,
            long_name_line: None,
            units_line: None,
            comments_line: None,
            param_lines: Vec::new(),
            short_name_line: None,
            no_rename_columns: false,
            no_rename_sheet: false,
            no_rename_book: false,
            blocks_as_sheets: false,
        }
    }
}

impl ReadSettings {
    pub fn chi_options(&self) -> ChiOptions {
        ChiOptions {
            split_scans: !self.no_split_scans,
            split_segments: !self.no_split_segments,
            add_milliamp: !self.no_milliamp,
            add_charge: !self.no_charge,
            invert_current: self.invert_current,
            exact_time: !self.keep_file_times,
        }
    }

    pub fn ascii_options(&self) -> Result<AsciiOptions> {
        let r = self;
        let mut o = AsciiOptions::default();
        if let Some(d) = &r.delimiter {
            o.delimiter = match d.to_ascii_lowercase().as_str() {
                "comma" | "," => Delimiter::Comma,
                "tab" | "\\t" => Delimiter::Tab,
                "semicolon" | ";" => Delimiter::Semicolon,
                "space" | "whitespace" | " " => Delimiter::Whitespace,
                _ => match Delimiter::from_text(d) {
                    Some(delimiter) => delimiter,
                    None => bail!("--delimiter must not be empty"),
                },
            };
        }
        if !r.fixed_width.is_empty() {
            if r.fixed_width.contains(&0) {
                bail!("--fixed-width: every width must be above 0");
            }
            o.delimiter = Delimiter::Fixed(r.fixed_width.clone());
        }
        if let Some(chars) = &r.delimiter_chars {
            if chars.is_empty() {
                bail!("--delimiter-chars must not be empty");
            }
            o.delimiter = Delimiter::AnyOf { chars: chars.clone(), combine: r.combine_delimiters };
        }
        o.short_name_line = r.short_name_line.unwrap_or(0);
        o.rename_columns = !r.no_rename_columns;
        o.rename_sheet = !r.no_rename_sheet;
        o.rename_book = !r.no_rename_book;
        o.header_lines = r.header_lines;
        o.rows = sheaf_io::ascii::ImportSelection {
            first: r.first_row,
            last: r.last_row,
            read: r.read_rows,
            skip: r.skip_rows,
        };
        o.columns = sheaf_io::ascii::ImportSelection {
            first: r.first_column,
            last: r.last_column,
            read: r.read_columns,
            skip: r.skip_columns,
        };
        o.decimal_comma = r.decimal_comma;
        o.thousands = match r.thousands {
            ThousandsSetting::Auto => Thousands::Auto,
            ThousandsSetting::None => Thousands::Off,
            ThousandsSetting::Comma => Thousands::Comma,
            ThousandsSetting::Dot => Thousands::Dot,
            ThousandsSetting::Space => Thousands::Space,
            ThousandsSetting::Apostrophe => Thousands::Apostrophe,
        };
        o.long_name_line = r.long_name_line;
        o.units_line = r.units_line;
        o.comments_line = r.comments_line;
        o.param_lines = r.param_lines.clone();
        if r.blocks_as_sheets {
            o.blocks = sheaf_io::ascii::Blocks::Sheets;
        }
        if r.dates_as_text {
            o.dates = sheaf_io::ascii::DateImport::AsText;
        }
        if r.day_first {
            o.date_order = sheaf_core::datetime::DateOrder::Dmy;
        }
        Ok(o)
    }
}

/// Reads one data file. The import warnings are in the result.
pub fn read_data_file(path: &Path, r: &ReadSettings) -> Result<Imported> {
    sheaf_io::import_file(path, r.format.kind(), &r.chi_options(), &r.ascii_options()?)
        .with_context(|| format!("could not import '{}'", path.display()))
}
