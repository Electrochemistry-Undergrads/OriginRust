//! Reading one data file with the import options.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sheaf_io::ascii::{AsciiOptions, Delimiter, Thousands};
use sheaf_io::chi::ChiOptions;
use sheaf_io::excel::{ExcelOptions, SheetRef};
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

/// Import options that have no command-line flag. They stay at their defaults unless a saved import
/// theme (`--theme`) carries other values, so a theme brings back the whole window.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NoFlagOptions {
    /// The first column is the X column.
    pub first_column_x: bool,
    /// A parameter row is named after the first cell of its header line.
    pub param_name_from_first_cell: bool,
    /// Legacy: rows to skip before the row selection.
    pub skip_rows: usize,
    /// Legacy: most rows to read.
    pub max_rows: Option<usize>,
}

impl Default for NoFlagOptions {
    fn default() -> Self {
        Self { first_column_x: true, param_name_from_first_cell: true, skip_rows: 0, max_rows: None }
    }
}

/// The options the text reader and the Excel reader share: header lines, names, row and column
/// selection, dates. A saved Excel theme can carry its own copy (see [`ReadSettings::excel_shared`]).
#[derive(Clone, Debug, PartialEq)]
pub struct SharedOptions {
    pub header_lines: Option<usize>,
    pub long_name_line: Option<usize>,
    pub units_line: Option<usize>,
    pub comments_line: Option<usize>,
    pub param_lines: Vec<usize>,
    pub short_name_line: Option<usize>,
    pub no_rename_columns: bool,
    pub no_rename_book: bool,
    pub first_row: usize,
    pub last_row: usize,
    pub read_rows: usize,
    pub skip_rows: usize,
    pub first_column: usize,
    pub last_column: usize,
    pub read_columns: usize,
    pub skip_columns: usize,
    pub dates_as_text: bool,
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
    /// Excel: the sheets to read, as names or 1-based positions (empty: every sheet).
    pub sheets: Vec<String>,
    /// Excel: read hidden sheets when `sheets` is empty (also the default, unless
    /// `skip_hidden_sheets` is set).
    pub hidden_sheets: bool,
    /// Excel: leave hidden sheets out when `sheets` is empty and `hidden_sheets` is off.
    pub skip_hidden_sheets: bool,
    /// The text options without a flag (only a saved theme sets them).
    #[serde(skip)]
    pub text_no_flag: NoFlagOptions,
    /// The Excel options without a flag (only a saved theme sets them).
    #[serde(skip)]
    pub excel_no_flag: NoFlagOptions,
    /// The delimiter exactly as a saved theme holds it. `delimiter` is read as a keyword (`tab`,
    /// `comma`, ...), so a whole-string delimiter that spells a keyword, or a single character
    /// the keywords take, is carried here instead. Only a saved theme sets it.
    #[serde(skip)]
    pub delimiter_exact: Option<Delimiter>,
    /// The shared options of a saved Excel theme. When set, Excel files are read with these and
    /// text files with the fields above, so a name that both windows saved gives each kind of
    /// file the settings of its own window. Only a saved theme sets it.
    #[serde(skip)]
    pub excel_shared: Option<SharedOptions>,
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
            sheets: Vec::new(),
            hidden_sheets: false,
            skip_hidden_sheets: false,
            text_no_flag: NoFlagOptions::default(),
            excel_no_flag: NoFlagOptions::default(),
            delimiter_exact: None,
            excel_shared: None,
        }
    }
}

impl ReadSettings {
    /// The shared options as the fields above hold them.
    pub fn shared(&self) -> SharedOptions {
        SharedOptions {
            header_lines: self.header_lines,
            long_name_line: self.long_name_line,
            units_line: self.units_line,
            comments_line: self.comments_line,
            param_lines: self.param_lines.clone(),
            short_name_line: self.short_name_line,
            no_rename_columns: self.no_rename_columns,
            no_rename_book: self.no_rename_book,
            first_row: self.first_row,
            last_row: self.last_row,
            read_rows: self.read_rows,
            skip_rows: self.skip_rows,
            first_column: self.first_column,
            last_column: self.last_column,
            read_columns: self.read_columns,
            skip_columns: self.skip_columns,
            dates_as_text: self.dates_as_text,
        }
    }

    /// Sets the fields that hold the shared options.
    pub fn set_shared(&mut self, s: SharedOptions) {
        self.header_lines = s.header_lines;
        self.long_name_line = s.long_name_line;
        self.units_line = s.units_line;
        self.comments_line = s.comments_line;
        self.param_lines = s.param_lines;
        self.short_name_line = s.short_name_line;
        self.no_rename_columns = s.no_rename_columns;
        self.no_rename_book = s.no_rename_book;
        self.first_row = s.first_row;
        self.last_row = s.last_row;
        self.read_rows = s.read_rows;
        self.skip_rows = s.skip_rows;
        self.first_column = s.first_column;
        self.last_column = s.last_column;
        self.read_columns = s.read_columns;
        self.skip_columns = s.skip_columns;
        self.dates_as_text = s.dates_as_text;
    }

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
        if let Some(d) = &r.delimiter_exact {
            o.delimiter = d.clone();
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
        let extra = r.text_no_flag;
        o.first_column_x = extra.first_column_x;
        o.param_name_from_first_cell = extra.param_name_from_first_cell;
        o.skip_rows = extra.skip_rows;
        o.max_rows = extra.max_rows;
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

    /// The options for Excel files. The header, row and column settings are those of the Excel
    /// theme when one was loaded (`excel_shared`), else those of text files; the separator, number
    /// and block settings do not apply to a cell grid and are left out.
    pub fn excel_options(&self) -> ExcelOptions {
        let r = self;
        let s = r.excel_shared.clone().unwrap_or_else(|| r.shared());
        let mut o = ExcelOptions {
            sheets: r.sheets.iter().map(|s| SheetRef::parse(s)).collect(),
            include_hidden: r.hidden_sheets || !r.skip_hidden_sheets,
            ..ExcelOptions::default()
        };
        let extra = r.excel_no_flag;
        o.first_column_x = extra.first_column_x;
        o.param_name_from_first_cell = extra.param_name_from_first_cell;
        o.skip_rows = extra.skip_rows;
        o.max_rows = extra.max_rows;
        o.header_lines = s.header_lines;
        o.long_name_line = s.long_name_line;
        o.units_line = s.units_line;
        o.comments_line = s.comments_line;
        o.rename_columns = !s.no_rename_columns;
        o.short_name_line = s.short_name_line.unwrap_or(0);
        o.rename_book = !s.no_rename_book;
        o.param_lines = s.param_lines;
        o.rows = sheaf_io::ascii::ImportSelection {
            first: s.first_row,
            last: s.last_row,
            read: s.read_rows,
            skip: s.skip_rows,
        };
        o.columns = sheaf_io::ascii::ImportSelection {
            first: s.first_column,
            last: s.last_column,
            read: s.read_columns,
            skip: s.skip_columns,
        };
        if s.dates_as_text {
            o.dates = sheaf_io::ascii::DateImport::AsText;
        }
        o
    }
}

/// Reads one data file. The import warnings are in the result.
pub fn read_data_file(path: &Path, r: &ReadSettings) -> Result<Imported> {
    sheaf_io::import_file_with_excel(path, r.format.kind(), &r.chi_options(), &r.ascii_options()?, &r.excel_options())
        .with_context(|| format!("could not import '{}'", path.display()))
}
