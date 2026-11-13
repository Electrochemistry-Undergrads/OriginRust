//! Writing a worksheet as delimited text, as Origin's `expASC` does (measured, see
//! `docs/spec/import-ascii.md` section 13).

use crate::error::IoError;
use sheaf_core::format::format_full_precision;
use sheaf_core::{Column, ColumnData, Worksheet};
use std::io::Write;
use std::path::PathBuf;

/// What goes between two cells of a line (expASC `separator` and `csvsep`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Separator {
    #[default]
    Tab,
    Comma,
    Space,
    CommaTab,
    CommaSpace,
    Semicolon,
}

impl Separator {
    /// The text written between cells.
    pub fn text(self) -> &'static str {
        match self {
            Separator::Tab => "\t",
            Separator::Comma => ",",
            Separator::Space => " ",
            Separator::CommaTab => ",\t",
            Separator::CommaSpace => ", ",
            Separator::Semicolon => ";",
        }
    }
}

/// What to write besides the data rows. The defaults are Origin's: tab separated, the long name,
/// units and comment rows, no short names, no user parameters, no row labels, full precision, and
/// `--` for a missing value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AsciiExportOptions {
    pub separator: Separator,
    /// Origin's "csv" file type: a text cell that holds the separator or a quote is put in quotes,
    /// and with a semicolon as separator the numbers use a decimal comma.
    pub csv: bool,
    /// A row of the short (column) names.
    pub short_names: bool,
    /// A row of long names. On by default; left out when no column has one.
    pub long_names: bool,
    /// A row of units. On by default; left out when no column has one.
    pub units: bool,
    /// A row of comments. On by default; left out when no column has one.
    pub comments: bool,
    /// One row per user parameter. Off by default.
    pub user_params: bool,
    /// A first column with the row numbers, and the row names ("Long Name", "Units", ...) in the
    /// label rows.
    pub row_labels: bool,
    /// Every digit (15 significant) instead of the column's display format. On by default.
    pub full_precision: bool,
    /// Write `--` for a missing value inside a column (otherwise an empty field). On by default.
    pub missing_dashes: bool,
}

impl Default for AsciiExportOptions {
    fn default() -> Self {
        AsciiExportOptions {
            separator: Separator::Tab,
            csv: false,
            short_names: false,
            long_names: true,
            units: true,
            comments: true,
            user_params: false,
            row_labels: false,
            full_precision: true,
            missing_dashes: true,
        }
    }
}

impl AsciiExportOptions {
    /// Origin's "csv" type with a comma (what the app writes for a `.csv` file name).
    pub fn csv() -> Self {
        AsciiExportOptions { separator: Separator::Comma, csv: true, ..Self::default() }
    }
}

/// Writes `sheet` as delimited text, lines ending in CR LF as Origin writes them.
///
/// Label rows come first (short names, long names, units, comments, user parameters, each only
/// when its option is on and at least one column has something to say), then one line per row.
/// A missing value inside a column is `--`, past the end of a column the field is empty.
pub fn write_delimited(sheet: &Worksheet, out: &mut dyn Write, opts: &AsciiExportOptions) -> Result<(), IoError> {
    let io_err = |e: std::io::Error| IoError::Io { path: PathBuf::from(format!("<{}>", sheet.name)), source: e };
    let cols = &sheet.columns;

    let mut label_lines: Vec<(String, Vec<String>)> = Vec::new();
    if opts.short_names {
        label_lines.push(("Short Name".into(), cols.iter().map(|c| c.short_name.clone()).collect()));
    }
    if opts.long_names && cols.iter().any(|c| !c.long_name.is_empty()) {
        label_lines.push(("Long Name".into(), cols.iter().map(|c| c.long_name.clone()).collect()));
    }
    if opts.units && cols.iter().any(|c| !c.units.is_empty()) {
        label_lines.push(("Units".into(), cols.iter().map(|c| c.units.clone()).collect()));
    }
    if opts.comments && cols.iter().any(|c| !c.comments.is_empty()) {
        label_lines.push(("Comments".into(), cols.iter().map(|c| c.comments.clone()).collect()));
    }
    if opts.user_params {
        for name in sheet.parameter_names() {
            let line = cols.iter().map(|c| c.params.get(&name).cloned().unwrap_or_default()).collect();
            label_lines.push((name, line));
        }
    }
    let sep = opts.separator.text();
    let mut put_line = |first: Option<String>, fields: Vec<String>| -> std::io::Result<()> {
        let mut text = first.into_iter().chain(fields).collect::<Vec<_>>().join(sep);
        text.push_str("\r\n");
        out.write_all(text.as_bytes())
    };
    for (name, line) in label_lines {
        let first = opts.row_labels.then_some(name);
        put_line(first, line.iter().map(|f| quote(f, opts)).collect()).map_err(io_err)?;
    }
    let used: Vec<usize> = cols.iter().map(used_rows).collect();
    for row in 0..sheet.row_count() {
        let first = opts.row_labels.then(|| (row + 1).to_string());
        let fields = cols.iter().zip(&used).map(|(c, n)| cell(c, row, *n, opts)).collect();
        put_line(first, fields).map_err(io_err)?;
    }
    Ok(())
}

/// Rows up to and including the last one that holds a value (Origin keeps a missing value only
/// between values; a column does not end in `--`).
fn used_rows(col: &Column) -> usize {
    match &col.data {
        ColumnData::Numeric(v) => v.iter().rposition(|x| !x.is_nan()).map_or(0, |i| i + 1),
        ColumnData::Text(v) => v.iter().rposition(|s| !s.is_empty()).map_or(0, |i| i + 1),
        ColumnData::Mixed { numbers, texts } => {
            let last_number = numbers.iter().rposition(|x| !x.is_nan()).map_or(0, |i| i + 1);
            let last_text = texts.keys().next_back().map_or(0, |r| r + 1);
            last_number.max(last_text)
        }
    }
}

fn cell(col: &Column, row: usize, used: usize, opts: &AsciiExportOptions) -> String {
    if row >= used {
        return String::new();
    }
    let missing = || if opts.missing_dashes { "--".to_string() } else { String::new() };
    let number = |x: f64| {
        if x.is_nan() {
            return missing();
        }
        let text = if col.format.is_date_time() || !x.is_finite() {
            col.format.format(x)
        } else if opts.full_precision {
            format_full_precision(x)
        } else {
            col.shown_text(row)
        };
        if opts.csv && opts.separator == Separator::Semicolon { text.replace('.', ",") } else { text }
    };
    match &col.data {
        ColumnData::Numeric(v) => match v.get(row) {
            Some(&x) => number(x),
            None => String::new(),
        },
        ColumnData::Text(v) => match v.get(row) {
            Some(s) if !s.is_empty() => quote(s, opts),
            _ => missing(),
        },
        // A Text & Numeric column: its text cells are written as text, its numbers as numbers.
        ColumnData::Mixed { numbers, .. } => match (col.data.text_cell(row), numbers.get(row)) {
            (Some(t), _) => quote(t, opts),
            (None, Some(&x)) => number(x),
            (None, None) => missing(),
        },
    }
}

/// Text as Origin writes it. A quote becomes two quotes; in the csv type a text that holds the
/// separator or a quote is then put in quotes and its quotes doubled once more (Origin does so, a
/// text `q"x` comes out as `"q""""x"`). Spaces at the ends are kept as they are.
fn quote(s: &str, opts: &AsciiExportOptions) -> String {
    let doubled = s.replace('"', "\"\"");
    if opts.csv && (s.contains('"') || s.contains(opts.separator.text())) {
        format!("\"{}\"", doubled.replace('"', "\"\""))
    } else {
        doubled
    }
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use super::*;
    use sheaf_core::format::NumberFormat;
    use sheaf_core::{ColumnId, Designation};

    fn column(id: u64, name: &str, data: ColumnData, long: &str, unit: &str, comment: &str) -> Column {
        let d = if id == 1 { Designation::X } else { Designation::Y };
        Column::new(ColumnId(id), name, d, data).with_labels(long, unit, comment)
    }

    /// The sheet that was exported from Origin for the first probe (`re-workspace/scratch/
    /// sheet-export/out/d_*.csv`): General columns, a missing value, a text with a comma.
    fn origin_sheet() -> Worksheet {
        let mut s = Worksheet::new("S");
        s.columns = vec![
            column(1, "A", ColumnData::numeric(vec![123456.789, 0.123456789012345, f64::NAN, 1e-7]), "Pot", "V", "cmt"),
            column(2, "B", ColumnData::numeric(vec![3.14159, 2.5, 1234567.891, -0.000123456]), "Cur", "A", ""),
            column(3, "C", ColumnData::text(vec!["hello".into(), "a,b".into()]), "Txt", "", ""),
        ];
        s
    }

    fn export(s: &Worksheet, opts: &AsciiExportOptions) -> String {
        let mut buf = Vec::new();
        write_delimited(s, &mut buf, opts).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn csv_equals_the_file_origin_wrote() {
        let want = "Pot,Cur,Txt\r\nV,A,\r\ncmt,,\r\n123456.789,3.14159,hello\r\n0.123456789012345,2.5,\"a,b\"\r\n\
                    --,1234567.891,\r\n1E-7,-1.23456E-4,\r\n";
        assert_eq!(export(&origin_sheet(), &AsciiExportOptions::csv()), want);
    }

    #[test]
    fn tab_type_does_not_quote_and_defaults_are_origins() {
        let want = "Pot\tCur\tTxt\r\nV\tA\t\r\ncmt\t\t\r\n123456.789\t3.14159\thello\r\n0.123456789012345\t2.5\ta,b\r\n\
                    --\t1234567.891\t\r\n1E-7\t-1.23456E-4\t\r\n";
        assert_eq!(export(&origin_sheet(), &AsciiExportOptions::default()), want);
    }

    #[test]
    fn short_names_and_missing_option() {
        let opts = AsciiExportOptions {
            short_names: true,
            long_names: false,
            units: false,
            comments: false,
            missing_dashes: false,
            ..AsciiExportOptions::csv()
        };
        let text = export(&origin_sheet(), &opts);
        assert!(text.starts_with("A,B,C\r\n123456.789,3.14159,hello\r\n"), "{text}");
        assert!(text.contains("\r\n,1234567.891,\r\n"), "{text}");
    }

    /// Sheet of the last probe: a user parameter, texts with the separator, a space and a quote.
    fn texts_sheet() -> Worksheet {
        let mut s = Worksheet::new("S");
        let mut a = column(1, "A", ColumnData::numeric(vec![1.5]), "Pot", "V", "cmt");
        a.params.insert("Sample".into(), "s1".into());
        let b = column(2, "B", ColumnData::numeric(vec![2.5]), "Cur", "", "");
        let c = column(
            3,
            "C",
            ColumnData::text(vec!["a,b".into(), "a;b".into(), "a b".into(), " p ".into(), "q\"x".into()]),
            "Txt",
            "",
            "",
        );
        s.columns = vec![a, b, c];
        s
    }

    #[test]
    fn quoting_follows_the_separator_and_doubles_quotes_twice() {
        let comma = export(&texts_sheet(), &AsciiExportOptions::csv());
        let rows: Vec<&str> = comma.split("\r\n").skip(3).collect();
        assert_eq!(rows, ["1.5,2.5,\"a,b\"", ",,a;b", ",,a b", ",,\u{20}p\u{20}", ",,\"q\"\"\"\"x\"", ""]);
        let semi = export(
            &texts_sheet(),
            &AsciiExportOptions { separator: Separator::Semicolon, csv: true, ..AsciiExportOptions::default() },
        );
        let rows: Vec<&str> = semi.split("\r\n").skip(3).collect();
        assert_eq!(rows[0], "1,5;2,5;a,b");
        assert_eq!(rows[1], ";;\"a;b\"");
        let tab = export(&texts_sheet(), &AsciiExportOptions::default());
        assert!(tab.ends_with("\r\n\t\tq\"\"x\r\n"), "{tab:?}");
    }

    #[test]
    fn separators_row_labels_and_user_parameters() {
        let opts = AsciiExportOptions {
            separator: Separator::CommaTab,
            short_names: true,
            user_params: true,
            row_labels: true,
            ..AsciiExportOptions::default()
        };
        let text = export(&texts_sheet(), &opts);
        let lines: Vec<&str> = text.split("\r\n").collect();
        assert_eq!(lines[0], "Short Name,\tA,\tB,\tC");
        assert_eq!(lines[1], "Long Name,\tPot,\tCur,\tTxt");
        assert_eq!(lines[2], "Units,\tV,\t,\t");
        assert_eq!(lines[3], "Comments,\tcmt,\t,\t");
        assert_eq!(lines[4], "Sample,\ts1,\t,\t");
        assert_eq!(lines[5], "1,\t1.5,\t2.5,\ta,b");
        assert_eq!(lines[6], "2,\t,\t,\ta;b");
        let plain = export(&texts_sheet(), &AsciiExportOptions::csv());
        assert!(!plain.contains("s1"), "user parameters are off by default");
    }

    #[test]
    fn display_precision_uses_the_column_format() {
        let mut s = Worksheet::new("S");
        let mut c = column(1, "A", ColumnData::numeric(vec![3.14159, 1234.5]), "", "", "");
        c.format = NumberFormat::Decimal(2);
        s.columns = vec![c];
        let shown = AsciiExportOptions { full_precision: false, ..AsciiExportOptions::default() };
        assert_eq!(export(&s, &shown), "3.14\r\n1234.50\r\n");
        assert_eq!(export(&s, &AsciiExportOptions::default()), "3.14159\r\n1234.5\r\n");
    }

    #[test]
    fn date_columns_are_written_as_shown() {
        use sheaf_core::datetime::{DatePattern, TimePattern};
        let mut s = Worksheet::new("S");
        let mut t =
            column(1, "A", ColumnData::numeric(vec![2_461_355.573_032_407_6, f64::NAN, 2_461_356.0]), "", "", "");
        t.format = NumberFormat::DateTime(DatePattern::Iso, TimePattern::Hms);
        let y = column(2, "B", ColumnData::numeric(vec![1.0, 2.0, 3.0]), "", "", "");
        s.columns = vec![t, y];
        let text = export(&s, &AsciiExportOptions::csv());
        assert_eq!(text, "2026-11-11 13:45:10,1\r\n--,2\r\n2026-11-12 00:00:00,3\r\n");
    }

    #[test]
    fn text_cells_of_a_text_and_numeric_column_are_written_and_read_back() {
        let mut s = Worksheet::new("S");
        let texts = [(1, "<LOD".to_string()), (3, "a,b".to_string())].into_iter().collect();
        let c = Column::new(
            ColumnId(1),
            "A",
            Designation::Y,
            ColumnData::mixed(vec![1.5, f64::NAN, 3.0, f64::NAN, f64::NAN, 4.0], texts),
        );
        s.columns.push(c);
        let text = export(&s, &AsciiExportOptions::csv());
        assert_eq!(text, "1.5\r\n<LOD\r\n3\r\n\"a,b\"\r\n--\r\n4\r\n");
        let book = crate::ascii::parse_ascii(&text, "S", &crate::ascii::AsciiOptions::default()).unwrap().workbook;
        let back = &book.sheets[0].columns[0];
        assert_eq!(back.numeric().unwrap()[0], 1.5);
        assert_eq!((back.cell_text(1), back.cell_text(3)), ("<LOD".to_string(), "a,b".to_string()));
        use sheaf_core::datetime::{DatePattern, TimePattern};
        let mut s = Worksheet::new("S");
        let mut t =
            column(1, "A", ColumnData::numeric(vec![2_461_355.573_032_407_6, f64::NAN, 2_461_356.0]), "", "", "");
        t.format = NumberFormat::DateTime(DatePattern::Iso, TimePattern::Hms);
        let y = column(2, "B", ColumnData::numeric(vec![1.0, 2.0, 3.0]), "", "", "");
        s.columns = vec![t, y];
        let text = export(&s, &AsciiExportOptions::csv());
        assert_eq!(text, "2026-11-11 13:45:10,1\r\n--,2\r\n2026-11-12 00:00:00,3\r\n");
    }

    #[test]
    fn unlabelled_sheet_has_no_label_lines_and_short_columns_end_empty() {
        let mut s = Worksheet::new("S");
        s.columns.push(column(1, "A", ColumnData::numeric(vec![1.0, 2.0, f64::NAN]), "", "", ""));
        s.columns.push(column(2, "B", ColumnData::numeric(vec![1.0, 2.0, 3.0]), "", "", ""));
        assert_eq!(export(&s, &AsciiExportOptions::default()), "1\t1\r\n2\t2\r\n\t3\r\n");
    }
}
