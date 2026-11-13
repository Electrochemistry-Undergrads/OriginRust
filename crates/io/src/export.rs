//! Writing a worksheet as delimited text.

use crate::error::IoError;
use sheaf_core::{Column, ColumnData, Worksheet};
use std::io::Write;
use std::path::PathBuf;

/// What to write besides the data rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AsciiExportOptions {
    /// Field separator (default `,`).
    pub delimiter: char,
    /// Write a line of long names (the short name where a column has none). Default on.
    pub long_names: bool,
    /// Write a line of units. Default on.
    pub units: bool,
    /// Write a line of comments. Default off.
    pub comments: bool,
}

impl Default for AsciiExportOptions {
    fn default() -> Self {
        AsciiExportOptions { delimiter: ',', long_names: true, units: true, comments: false }
    }
}

/// Writes `sheet` as delimited text.
///
/// Label lines come first (long names, units, comments, each only when its
/// option is on and at least one column has something to say), then one line
/// per row. Numbers use Rust's shortest round-trip formatting, missing values
/// are empty fields, and text is quoted when it contains the delimiter, a
/// quote, a line break or leading or trailing spaces.
pub fn write_delimited(sheet: &Worksheet, out: &mut dyn Write, opts: &AsciiExportOptions) -> Result<(), IoError> {
    let io_err = |e: std::io::Error| IoError::Io { path: PathBuf::from(format!("<{}>", sheet.name)), source: e };
    let cols = &sheet.columns;

    let mut label_lines: Vec<Vec<String>> = Vec::new();
    if opts.long_names && cols.iter().any(|c| !c.long_name.is_empty()) {
        label_lines.push(cols.iter().map(|c| c.display_name().to_string()).collect());
    }
    if opts.units && cols.iter().any(|c| !c.units.is_empty()) {
        label_lines.push(cols.iter().map(|c| c.units.clone()).collect());
    }
    if opts.comments && cols.iter().any(|c| !c.comments.is_empty()) {
        label_lines.push(cols.iter().map(|c| c.comments.clone()).collect());
    }
    for line in label_lines {
        write_line(out, line.iter().map(|f| quote_if_needed(f, opts.delimiter)), opts.delimiter).map_err(io_err)?;
    }
    for row in 0..sheet.row_count() {
        write_line(out, cols.iter().map(|c| cell(c, row, opts.delimiter)), opts.delimiter).map_err(io_err)?;
    }
    Ok(())
}

fn write_line(out: &mut dyn Write, fields: impl Iterator<Item = String>, delimiter: char) -> std::io::Result<()> {
    let mut line = String::new();
    for (i, f) in fields.enumerate() {
        if i > 0 {
            line.push(delimiter);
        }
        line.push_str(&f);
    }
    line.push('\n');
    out.write_all(line.as_bytes())
}

fn cell(col: &Column, row: usize, delimiter: char) -> String {
    match &col.data {
        // Dates and times are written as shown in the worksheet, so another program reads them as dates.
        ColumnData::Numeric(v) if col.format.is_date_time() => v
            .get(row)
            .filter(|x| x.is_finite())
            .map_or_else(String::new, |x| quote_if_needed(&col.format.format(*x), delimiter)),
        ColumnData::Numeric(v) => v.get(row).map_or_else(String::new, |x| format_number(*x)),
        ColumnData::Text(v) => v.get(row).map_or_else(String::new, |s| quote_if_needed(s, delimiter)),
        // A Text & Numeric column: the text cells are written as text, the numbers as numbers.
        ColumnData::Mixed { numbers, .. } => match (col.data.text_cell(row), numbers.get(row)) {
            (Some(t), _) => quote_if_needed(t, delimiter),
            (None, Some(x)) if col.format.is_date_time() => {
                if x.is_finite() {
                    quote_if_needed(&col.format.format(*x), delimiter)
                } else {
                    String::new()
                }
            }
            (None, Some(x)) => format_number(*x),
            (None, None) => String::new(),
        },
    }
}

/// Shortest text that reads back as the same number. Very large and very
/// small magnitudes use exponent notation so the text stays short.
fn format_number(x: f64) -> String {
    if x.is_nan() {
        String::new()
    } else if x != 0.0 && x.is_finite() && !(1e-5..1e16).contains(&x.abs()) {
        format!("{x:e}")
    } else {
        format!("{x}")
    }
}

fn quote_if_needed(s: &str, delimiter: char) -> String {
    let needs = s.contains(delimiter)
        || s.contains(['"', '\n', '\r'])
        || s.starts_with(char::is_whitespace)
        || s.ends_with(char::is_whitespace);
    if needs { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ascii::{AsciiOptions, parse_ascii};
    use sheaf_core::{ColumnId, Designation};

    fn sheet() -> Worksheet {
        let mut s = Worksheet::new("S");
        let mk = |id, short: &str, d, data, long: &str, unit: &str| {
            Column::new(ColumnId(id), short, d, data).with_labels(long, unit, "")
        };
        s.columns = vec![
            mk(
                1,
                "A",
                Designation::X,
                ColumnData::numeric(vec![0.0, 0.1, 1e-7, 123456.789, -2.5e20]),
                "Potential",
                "V",
            ),
            mk(
                2,
                "B",
                Designation::Y,
                ColumnData::numeric(vec![1.0, f64::NAN, 3.0, 1.0 / 3.0, f64::INFINITY]),
                "Current",
                "mA",
            ),
            mk(
                3,
                "C",
                Designation::Label,
                ColumnData::text(vec!["a,b".into(), "plain".into(), "say \"x\"".into(), "".into(), " pad ".into()]),
                "Note",
                "",
            ),
        ];
        s
    }

    fn export(s: &Worksheet, opts: &AsciiExportOptions) -> String {
        let mut buf = Vec::new();
        write_delimited(s, &mut buf, opts).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn writes_labels_numbers_and_quotes() {
        let text = export(&sheet(), &AsciiExportOptions::default());
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "Potential,Current,Note");
        assert_eq!(lines[1], "V,mA,");
        assert_eq!(lines[2], "0,1,\"a,b\"");
        assert_eq!(lines[3], "0.1,,plain");
        assert_eq!(lines[4], "1e-7".to_string() + ",3,\"say \"\"x\"\"\"");
        assert_eq!(lines[5], "123456.789,0.3333333333333333,");
        assert_eq!(lines[6], "-2.5e20,inf,\" pad \"");
    }

    #[test]
    fn options_choose_lines_and_delimiter() {
        let mut s = sheet();
        s.columns[0].comments = "c1".into();
        let opts = AsciiExportOptions { delimiter: '\t', long_names: false, units: false, comments: true };
        let text = export(&s, &opts);
        assert!(text.starts_with("c1\t\t\n0\t1\ta,b\n"), "{text}");
    }

    #[test]
    fn write_then_import_equals_the_original() {
        let s = sheet();
        let text = export(&s, &AsciiExportOptions::default());
        let back = parse_ascii(&text, "S", &AsciiOptions::default()).unwrap().workbook;
        let t = &back.sheets[0];
        assert_eq!(t.columns.len(), 3);
        for (a, b) in s.columns.iter().zip(&t.columns) {
            assert_eq!(a.long_name, b.long_name);
            assert_eq!(a.units, b.units);
            assert_eq!(a.designation, b.designation, "{}", a.short_name);
            assert_eq!(a.short_name, b.short_name);
            match (&a.data, &b.data) {
                (ColumnData::Numeric(x), ColumnData::Numeric(y)) => {
                    assert_eq!(x.len(), y.len());
                    for (p, q) in x.iter().zip(y.iter()) {
                        assert!((p.is_nan() && q.is_nan()) || p == q, "{p} vs {q}");
                    }
                }
                (ColumnData::Text(x), ColumnData::Text(y)) => assert_eq!(x, y),
                // Imported numbers are Text & Numeric columns (Origin's format 7).
                (ColumnData::Numeric(x), ColumnData::Mixed { numbers: y, .. }) => {
                    assert_eq!(x.len(), y.len());
                    for (p, q) in x.iter().zip(y.iter()) {
                        assert!((p.is_nan() && q.is_nan()) || p == q, "{p} vs {q}");
                    }
                }
                _ => panic!("column kinds differ"),
            }
        }
    }

    #[test]
    fn text_cells_of_a_text_and_numeric_column_are_written_and_read_back() {
        let mut s = Worksheet::new("S");
        let texts = [(1, "<LOD".to_string()), (3, "a,b".to_string())].into_iter().collect();
        let c =
            Column::new(ColumnId(1), "A", Designation::Y, ColumnData::mixed(vec![1.5, f64::NAN, 3.0, f64::NAN], texts));
        s.columns.push(c);
        let text = export(&s, &AsciiExportOptions { long_names: false, units: false, ..Default::default() });
        assert_eq!(text, "1.5\n<LOD\n3\n\"a,b\"\n");
        let back = parse_ascii(&text, "S", &AsciiOptions::default()).unwrap().workbook;
        let t = &back.sheets[0].columns[0];
        assert_eq!(t.numeric().unwrap()[0], 1.5);
        assert_eq!((t.cell_text(1), t.cell_text(3)), ("<LOD".to_string(), "a,b".to_string()));
    }

    #[test]
    fn date_columns_are_written_as_shown_and_read_back_equal() {
        use sheaf_core::datetime::{DatePattern, TimePattern};
        use sheaf_core::format::NumberFormat;
        let mut s = Worksheet::new("S");
        let mut t = Column::new(
            ColumnId(1),
            "A",
            Designation::X,
            ColumnData::numeric(vec![2_461_355.573_032_407_6, f64::NAN, 2_461_356.0]),
        );
        t.format = NumberFormat::DateTime(DatePattern::Iso, TimePattern::Hms);
        let y = Column::new(ColumnId(2), "B", Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0]));
        s.columns = vec![t, y];
        let text = export(&s, &AsciiExportOptions::default());
        assert_eq!(text, "2026-11-11 13:45:10,1\n,2\n2026-11-12 00:00:00,3\n");
        let back = parse_ascii(&text, "S", &AsciiOptions::default()).unwrap().workbook;
        let c = &back.sheets[0].columns[0];
        assert_eq!(c.format, s.columns[0].format);
        assert!((c.numeric().unwrap()[0] - 2_461_355.573_032_407_6).abs() < 1e-9);
    }

    #[test]
    fn unlabelled_sheet_has_no_label_lines() {
        let mut s = Worksheet::new("S");
        s.columns.push(Column::new(ColumnId(1), "A", Designation::X, ColumnData::numeric(vec![1.0, 2.0])));
        assert_eq!(export(&s, &AsciiExportOptions::default()), "1\n2\n");
    }
}
