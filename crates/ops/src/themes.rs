//! Saved import settings on the command line: `--theme NAME` and `--last-used` read the same
//! stores as the Dialog Theme row of the import windows (`sheaf_io::themes`) and turn the stored
//! settings into [`ReadSettings`]. The command-line flags are applied over them (see [`merge`]).
//! The rules are in `docs/spec/import-ascii.md`, section 17.

use crate::read::{DataFormat, NoFlagOptions, ReadSettings, SharedOptions, ThousandsSetting};
use anyhow::{Result, anyhow};
use sheaf_core::datetime::DateOrder;
use sheaf_io::ImportKind;
use sheaf_io::ascii::{AsciiOptions, Blocks, DateImport, Delimiter, Thousands};
use sheaf_io::chi::ChiOptions;
use sheaf_io::excel::ExcelOptions;
use sheaf_io::themes::{
    self, AsciiSettings, ChiSettings, DEFAULT_LABEL, ExcelSettings, LAST_USED_LABEL, Settings, ThemeError,
};
use std::path::{Path, PathBuf};

/// The environment variable that names the folder of the saved themes (tests set it, or give
/// `--themes-dir`, so that they never read the real settings folder).
pub const DIR_ENV: &str = "SHEAF_THEMES_DIR";

/// The theme list entry the flags ask for: the name of `--theme`, or `<last used>` for
/// `--last-used`; `None` when neither is given.
pub fn entry(theme: Option<&str>, last_used: bool) -> Option<String> {
    match (theme, last_used) {
        (Some(name), _) => Some(name.to_string()),
        (None, true) => Some(LAST_USED_LABEL.to_string()),
        (None, false) => None,
    }
}

/// The folder of the theme files: `--themes-dir`, else `$SHEAF_THEMES_DIR`, else the settings folder.
pub fn themes_dir(flag: Option<&Path>) -> Result<PathBuf> {
    if let Some(dir) = flag {
        return Ok(dir.to_path_buf());
    }
    if let Some(dir) = std::env::var_os(DIR_ENV).filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    themes::default_dir().ok_or_else(|| anyhow!("there is no settings folder; give --themes-dir"))
}

/// What a theme sets on the command line.
#[derive(Clone, Debug, PartialEq)]
pub struct Themed {
    /// The import options. Fields that no store has stay at the defaults.
    pub read: ReadSettings,
    /// The theme ticks "apply partial import to all files except the first".
    pub partial_except_first: bool,
}

/// How one store answers a theme list entry.
enum Found<S> {
    Settings(S),
    Absent,
}

fn look<S: Settings>(dir: &Path, entry: &str) -> Result<Found<S>, ThemeError> {
    match themes::resolve::<S>(dir, entry) {
        Ok(s) => Ok(Found::Settings(s)),
        Err(ThemeError::Unknown { .. } | ThemeError::NoLastUsed) => Ok(Found::Absent),
        Err(e) => Err(e),
    }
}

/// The settings of a theme list entry: `<default>`, `<last used>` or a saved name, looked up in the
/// text, CH Instruments and Excel stores. Each store that has the entry sets its part: the text
/// store the text options and the options the readers share, the CH Instruments store the CHI
/// options, the Excel store the hidden-sheet choice and the shared options of Excel files
/// (`ReadSettings::excel_shared`; the text files take them too when the text store has no theme of
/// that name). Import mode and the graph tick belong to the window and are not applied.
/// An entry that no store has is an error that lists the saved themes of all three.
pub fn load(dir: &Path, entry: &str) -> Result<Themed> {
    let entry = entry.trim();
    let ascii = look::<AsciiSettings>(dir, entry)?;
    let chi = look::<ChiSettings>(dir, entry)?;
    let excel = look::<ExcelSettings>(dir, entry)?;
    if matches!((&ascii, &chi, &excel), (Found::Absent, Found::Absent, Found::Absent)) {
        return Err(if entry.eq_ignore_ascii_case(LAST_USED_LABEL) {
            ThemeError::NoLastUsed.into()
        } else {
            let mut available: Vec<String> = Vec::new();
            for names in [
                themes::names::<AsciiSettings>(dir),
                themes::names::<ChiSettings>(dir),
                themes::names::<ExcelSettings>(dir),
            ] {
                for n in names {
                    if !available.iter().any(|a| a.to_lowercase() == n.to_lowercase()) {
                        available.push(n);
                    }
                }
            }
            ThemeError::Unknown { name: entry.to_string(), available }.into()
        });
    }
    let mut read = ReadSettings::default();
    let mut partial_except_first = false;
    if entry.eq_ignore_ascii_case(DEFAULT_LABEL) {
        return Ok(Themed { read, partial_except_first });
    }
    if let Found::Settings(c) = &chi {
        apply_chi(&mut read, &c.chi);
    }
    let have_text = matches!(ascii, Found::Settings(_));
    if let Found::Settings(a) = &ascii {
        apply_ascii(&mut read, &a.ascii);
        read.format = match a.data_type {
            ImportKind::Auto => DataFormat::Auto,
            ImportKind::Chi => DataFormat::Chi,
            ImportKind::Ascii => DataFormat::Ascii,
        };
        partial_except_first = a.partial_except_first;
    }
    if let Found::Settings(x) = &excel {
        let shared = shared_of_excel(&x.options);
        if !have_text {
            read.set_shared(shared.clone());
            partial_except_first = x.partial_except_first;
        }
        read.excel_shared = Some(shared);
        read.hidden_sheets = false;
        read.skip_hidden_sheets = !x.options.include_hidden;
        read.excel_no_flag = no_flag(
            x.options.first_column_x,
            x.options.param_name_from_first_cell,
            x.options.skip_rows,
            x.options.max_rows,
        );
    }
    Ok(Themed { read, partial_except_first })
}

fn no_flag(
    first_column_x: bool,
    param_name_from_first_cell: bool,
    skip_rows: usize,
    max_rows: Option<usize>,
) -> NoFlagOptions {
    NoFlagOptions { first_column_x, param_name_from_first_cell, skip_rows, max_rows }
}

fn apply_chi(r: &mut ReadSettings, o: &ChiOptions) {
    r.no_split_scans = !o.split_scans;
    r.no_split_segments = !o.split_segments;
    r.no_milliamp = !o.add_milliamp;
    r.no_charge = !o.add_charge;
    r.invert_current = o.invert_current;
    r.keep_file_times = !o.exact_time;
}

/// The delimiter as the flags say it. A character or a whole string is carried as it is
/// (`delimiter_exact`): the `--delimiter` text would read `tab`, `comma` and a single `,` or space as
/// a name or as white space.
fn apply_delimiter(r: &mut ReadSettings, d: &Delimiter) {
    r.delimiter = None;
    r.delimiter_exact = None;
    r.delimiter_chars = None;
    r.combine_delimiters = false;
    r.fixed_width = Vec::new();
    match d {
        Delimiter::Auto => {}
        Delimiter::Comma => r.delimiter = Some("comma".into()),
        Delimiter::Tab => r.delimiter = Some("tab".into()),
        Delimiter::Semicolon => r.delimiter = Some("semicolon".into()),
        Delimiter::Whitespace => r.delimiter = Some("space".into()),
        Delimiter::Other(_) | Delimiter::Multi(_) => r.delimiter_exact = Some(d.clone()),
        Delimiter::AnyOf { chars, combine } => {
            r.delimiter_chars = Some(chars.clone());
            r.combine_delimiters = *combine;
        }
        Delimiter::Fixed(w) => r.fixed_width = w.clone(),
    }
}

fn shared_of_ascii(o: &AsciiOptions) -> SharedOptions {
    SharedOptions {
        header_lines: o.header_lines,
        long_name_line: o.long_name_line,
        units_line: o.units_line,
        comments_line: o.comments_line,
        param_lines: o.param_lines.clone(),
        short_name_line: (o.short_name_line > 0).then_some(o.short_name_line),
        no_rename_columns: !o.rename_columns,
        no_rename_book: !o.rename_book,
        first_row: o.rows.first,
        last_row: o.rows.last,
        read_rows: o.rows.read,
        skip_rows: o.rows.skip,
        first_column: o.columns.first,
        last_column: o.columns.last,
        read_columns: o.columns.read,
        skip_columns: o.columns.skip,
        dates_as_text: o.dates == DateImport::AsText,
    }
}

fn shared_of_excel(o: &ExcelOptions) -> SharedOptions {
    SharedOptions {
        header_lines: o.header_lines,
        long_name_line: o.long_name_line,
        units_line: o.units_line,
        comments_line: o.comments_line,
        param_lines: o.param_lines.clone(),
        short_name_line: (o.short_name_line > 0).then_some(o.short_name_line),
        no_rename_columns: !o.rename_columns,
        no_rename_book: !o.rename_book,
        first_row: o.rows.first,
        last_row: o.rows.last,
        read_rows: o.rows.read,
        skip_rows: o.rows.skip,
        first_column: o.columns.first,
        last_column: o.columns.last,
        read_columns: o.columns.read,
        skip_columns: o.columns.skip,
        dates_as_text: o.dates == DateImport::AsText,
    }
}

fn apply_ascii(r: &mut ReadSettings, o: &AsciiOptions) {
    apply_delimiter(r, &o.delimiter);
    r.set_shared(shared_of_ascii(o));
    r.no_rename_sheet = !o.rename_sheet;
    r.decimal_comma = o.decimal_comma;
    r.thousands = match o.thousands {
        Thousands::Auto => ThousandsSetting::Auto,
        Thousands::Off => ThousandsSetting::None,
        Thousands::Comma => ThousandsSetting::Comma,
        Thousands::Dot => ThousandsSetting::Dot,
        Thousands::Space => ThousandsSetting::Space,
        Thousands::Apostrophe => ThousandsSetting::Apostrophe,
    };
    r.blocks_as_sheets = o.blocks == Blocks::Sheets;
    r.sparklines = o.sparklines;
    r.day_first = o.date_order == DateOrder::Dmy;
    r.text_no_flag = no_flag(o.first_column_x, o.param_name_from_first_cell, o.skip_rows, o.max_rows);
}

/// The settings with the command-line flags applied over a theme. `explicit` says whether the
/// option of that id (the field name of `ReadOptions`) was given on the command line; such an
/// option is taken from `flags`, every other one from `themed`. The four delimiter options act as
/// one: any of them given replaces the theme's delimiter whole.
pub fn merge(themed: &ReadSettings, flags: &ReadSettings, explicit: &dyn Fn(&str) -> bool) -> ReadSettings {
    let mut out = themed.clone();
    macro_rules! take {
        ($($field:ident),* $(,)?) => {
            $(if explicit(stringify!($field)) { out.$field = flags.$field.clone(); })*
        };
    }
    take!(
        format,
        no_split_scans,
        no_milliamp,
        no_charge,
        invert_current,
        no_split_segments,
        keep_file_times,
        header_lines,
        first_row,
        last_row,
        read_rows,
        skip_rows,
        first_column,
        last_column,
        read_columns,
        skip_columns,
        decimal_comma,
        dates_as_text,
        day_first,
        thousands,
        long_name_line,
        units_line,
        comments_line,
        param_lines,
        short_name_line,
        no_rename_columns,
        no_rename_sheet,
        no_rename_book,
        blocks_as_sheets,
        sparklines,
        sheets,
    );
    if ["delimiter", "delimiter_chars", "combine_delimiters", "fixed_width"].iter().any(|id| explicit(id)) {
        out.delimiter = flags.delimiter.clone();
        out.delimiter_exact = flags.delimiter_exact.clone();
        out.delimiter_chars = flags.delimiter_chars.clone();
        out.combine_delimiters = flags.combine_delimiters;
        out.fixed_width = flags.fixed_width.clone();
    }
    if let Some(x) = &mut out.excel_shared {
        macro_rules! take_shared {
            ($($field:ident),* $(,)?) => {
                $(if explicit(stringify!($field)) { x.$field = flags.$field.clone(); })*
            };
        }
        take_shared!(
            header_lines,
            long_name_line,
            units_line,
            comments_line,
            param_lines,
            short_name_line,
            no_rename_columns,
            no_rename_book,
            first_row,
            last_row,
            read_rows,
            skip_rows,
            first_column,
            last_column,
            read_columns,
            skip_columns,
            dates_as_text,
        );
    }
    if explicit("hidden_sheets") || explicit("skip_hidden_sheets") {
        out.hidden_sheets = flags.hidden_sheets;
        out.skip_hidden_sheets = flags.skip_hidden_sheets;
    }
    out
}

#[cfg(test)]
mod tests;
