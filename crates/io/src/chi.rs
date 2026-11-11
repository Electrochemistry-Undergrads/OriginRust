//! CH Instruments (CHI) potentiostat text exports.
//!
//! See `docs/spec/chi-text-format.md`. A file is a block of header lines
//! (date, technique, `name = value` parameters), a column header such as
//! `Potential/V, Current/A`, and numeric rows. Cyclic voltammetry files with
//! several cycles are split into one pair of columns per scan.

use crate::error::IoError;
use sheaf_core::column::default_short_name;
use sheaf_core::{Column, ColumnData, ColumnId, Designation, WindowId, Workbook, Worksheet};
use std::ops::Range;

/// Number of leading lines searched for the column header line.
const HEADER_SEARCH_LINES: usize = 200;
/// A current that stays at one value for this many rows is reported as clipped.
const CLIP_MIN_ROWS: usize = 20;
/// Relative tolerance for "the same value" when looking for clipping.
const CLIP_TOLERANCE: f64 = 1e-9;

/// What the CHI importer should do. The defaults follow the spec.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChiOptions {
    /// Split a multi-cycle CV file into scans (default on).
    pub split_scans: bool,
    /// Add a current column in mA after every current column in A (default on).
    pub add_milliamp: bool,
    /// For time-based files with a current column, add the running charge (default on).
    pub add_charge: bool,
    /// Multiply the current by -1 (default off).
    pub invert_current: bool,
}

impl Default for ChiOptions {
    fn default() -> Self {
        ChiOptions { split_scans: true, add_milliamp: true, add_charge: true, invert_current: false }
    }
}

/// The result of importing a CHI file.
#[derive(Clone, Debug)]
pub struct ChiImport {
    /// One workbook; its columns still carry placeholder identifiers (see
    /// `Project::adopt_workbook`).
    pub workbook: Workbook,
    /// The technique named on line 2 of the file, or empty when absent.
    pub technique: String,
    /// Number of scans the file was split into, 0 when it was not split.
    pub scans: usize,
    /// `Technique`, `Date`, `Instrument Model` (when present), then every
    /// `name = value` line of the header in file order.
    pub meta: Vec<(String, String)>,
    /// Things the user should look at, for example a clipped current.
    pub warnings: Vec<String>,
}

/// One column of a column-header line: `Potential/V` is ("Potential", "V").
#[derive(Clone, Debug, PartialEq)]
struct HeaderCol {
    long: String,
    unit: String,
}

#[derive(Debug)]
struct Block {
    header: Vec<HeaderCol>,
    /// One vector of values per header column.
    data: Vec<Vec<f64>>,
}

impl Block {
    fn rows(&self) -> usize {
        self.data.first().map_or(0, Vec::len)
    }
}

/// Parses a column-header line: at least two fields, each `Quantity/Unit`,
/// the first one a known X quantity.
fn parse_header_line(line: &str) -> Option<Vec<HeaderCol>> {
    let fields: Vec<&str> = line.split([',', '\t']).map(str::trim).filter(|f| !f.is_empty()).collect();
    if fields.len() < 2 {
        return None;
    }
    let mut cols = Vec::with_capacity(fields.len());
    for f in fields {
        let (long, unit) = f.rsplit_once('/')?;
        let (long, unit) = (long.trim(), unit.trim());
        if long.is_empty() {
            return None;
        }
        let unit = if unit.eq_ignore_ascii_case("sec") { "s" } else { unit };
        cols.push(HeaderCol { long: long.to_string(), unit: unit.to_string() });
    }
    let first = cols[0].long.to_ascii_lowercase();
    matches!(first.as_str(), "potential" | "time" | "freq" | "frequency").then_some(cols)
}

fn parse_row(line: &str) -> Option<Vec<f64>> {
    let mut values = Vec::new();
    for f in line.split([',', '\t']).map(str::trim).filter(|f| !f.is_empty()) {
        values.push(f.parse::<f64>().ok()?);
    }
    (values.len() >= 2).then_some(values)
}

fn find_header(lines: &[&str]) -> Option<usize> {
    lines.iter().take(HEADER_SEARCH_LINES).position(|l| parse_header_line(l).is_some())
}

/// True when the text has a CHI column-header line within its first 200 lines.
pub fn looks_like_chi(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    find_header(&lines).is_some()
}

/// Splits a potential sweep into scans (spec section 2.2).
///
/// A *vertex* is a row where the sweep direction reverses. Every second
/// vertex (the 2nd, 4th, ...) is a cut point and the first row of the next
/// scan. Zero steps keep the previous direction. Ranges of 2 rows or fewer
/// are dropped. Returns half-open, 0-based row ranges.
pub fn split_scans(potential: &[f64]) -> Vec<Range<usize>> {
    let n = potential.len();
    if n < 2 {
        return Vec::new();
    }
    let mut dir: Vec<i8> = potential
        .windows(2)
        .map(|w| {
            let d = w[1] - w[0];
            if d > 0.0 {
                1
            } else if d < 0.0 {
                -1
            } else {
                0
            }
        })
        .collect();
    let Some(first_nonzero) = dir.iter().copied().find(|&d| d != 0) else {
        return if n > 2 { std::iter::once(0..n).collect() } else { Vec::new() };
    };
    let mut prev = first_nonzero;
    for d in &mut dir {
        if *d == 0 {
            *d = prev;
        } else {
            prev = *d;
        }
    }
    let vertices: Vec<usize> = (1..dir.len()).filter(|&k| dir[k] != dir[k - 1]).collect();
    let cuts = vertices.iter().copied().skip(1).step_by(2);
    let mut ranges = Vec::new();
    let mut start = 0;
    for cut in cuts.chain(std::iter::once(n)) {
        if cut - start > 2 {
            ranges.push(start..cut);
        }
        start = cut;
    }
    ranges
}

/// Reads a CHI text export (already decoded and newline-normalised).
///
/// `file_stem` becomes the workbook long name and is used in messages.
pub fn parse_chi(text: &str, file_stem: &str, opts: &ChiOptions) -> Result<ChiImport, IoError> {
    let lines: Vec<&str> = text.lines().collect();
    let header_index = find_header(&lines).ok_or_else(|| IoError::NotChi { source_name: file_stem.to_string() })?;
    let (technique, meta) = read_preamble(&lines[..header_index]);
    let blocks = read_blocks(&lines[header_index..]);
    if blocks.is_empty() {
        return Err(IoError::NoData {
            source_name: file_stem.to_string(),
            detail: "the column header is followed by no numeric rows".to_string(),
        });
    }

    let mut warnings = Vec::new();
    let mut scans = 0;
    let mut sheets: Vec<Worksheet> = Vec::new();
    let mut next_id = 1u64;
    for mut block in blocks {
        if opts.invert_current {
            for (h, col) in block.header.iter().zip(block.data.iter_mut()) {
                if is_current_a(h) {
                    col.iter_mut().for_each(|v| *v = -*v);
                }
            }
        }
        let (mut sheet, n_scans) = build_sheet(&block, &technique, opts, &mut warnings);
        scans += n_scans;
        let base = sheet.name.clone();
        let mut k = 2;
        while sheets.iter().any(|s| s.name == sheet.name) {
            sheet.name = format!("{base} {k}");
            k += 1;
        }
        for col in &mut sheet.columns {
            col.id = ColumnId(next_id);
            next_id += 1;
        }
        sheets.push(sheet);
    }

    let mut workbook = Workbook::new(WindowId(0), "Book1");
    workbook.long_name = file_stem.to_string();
    workbook.sheets = sheets;
    Ok(ChiImport { workbook, technique, scans, meta, warnings })
}

/// Reads date, technique, instrument model and `name = value` lines from the
/// lines before the column header.
fn read_preamble(lines: &[&str]) -> (String, Vec<(String, String)>) {
    let mut technique = String::new();
    let mut date = String::new();
    let mut instrument = String::new();
    let mut params = Vec::new();
    let mut seen = 0;
    for raw in lines {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((name, value)) = line.split_once(" = ") {
            let (name, value) = (name.trim(), value.trim());
            if !name.is_empty() {
                params.push((name.to_string(), value.to_string()));
            }
            seen += 1;
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            if key.trim().eq_ignore_ascii_case("instrument model") {
                instrument = value.trim().to_string();
            }
            if matches!(key.trim().to_ascii_lowercase().as_str(), "file" | "data source" | "header" | "note")
                || key.trim().eq_ignore_ascii_case("instrument model")
            {
                seen += 1;
                continue;
            }
        }
        match seen {
            0 => date = line.to_string(),
            1 => technique = line.to_string(),
            _ => {}
        }
        seen += 1;
    }
    let mut meta = Vec::new();
    if !technique.is_empty() {
        meta.push(("Technique".to_string(), technique.clone()));
    }
    if !date.is_empty() {
        meta.push(("Date".to_string(), date));
    }
    if !instrument.is_empty() {
        meta.push(("Instrument Model".to_string(), instrument));
    }
    meta.extend(params);
    (technique, meta)
}

/// Collects the data blocks. A new column-header line directly after rows
/// continues the block when it is identical and starts a new one otherwise.
fn read_blocks(lines: &[&str]) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    for raw in lines {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(header) = parse_header_line(line) {
            match blocks.last_mut() {
                Some(last) if last.rows() == 0 => {
                    last.data = vec![Vec::new(); header.len()];
                    last.header = header;
                }
                Some(last) if last.header == header => {}
                _ => blocks.push(Block { data: vec![Vec::new(); header.len()], header }),
            }
        } else if let (Some(values), Some(block)) = (parse_row(line), blocks.last_mut()) {
            for (i, col) in block.data.iter_mut().enumerate() {
                col.push(values.get(i).copied().unwrap_or(f64::NAN));
            }
        }
    }
    blocks.retain(|b| b.rows() > 0);
    blocks
}

fn is_current_a(h: &HeaderCol) -> bool {
    h.long.eq_ignore_ascii_case("current") && h.unit == "A"
}

/// A column that is being assembled; `milliamp` marks a column whose formula
/// refers to the column before it.
struct Draft {
    column: Column,
    milliamp: bool,
}

fn draft(designation: Designation, long: &str, unit: &str, comments: &str, data: Vec<f64>) -> Draft {
    Draft {
        column: Column::new(ColumnId(0), "", designation, ColumnData::numeric(data)).with_labels(long, unit, comments),
        milliamp: false,
    }
}

fn milliamp_draft(a: &[f64], comments: &str) -> Draft {
    let mut d = draft(Designation::Y, "Current", "mA", comments, a.iter().map(|v| v * 1000.0).collect());
    d.milliamp = true;
    d
}

/// Gives the drafts their short names and formulas and puts them in a sheet.
fn finish_sheet(name: &str, drafts: Vec<Draft>) -> Worksheet {
    let mut sheet = Worksheet::new(name);
    for (i, mut d) in drafts.into_iter().enumerate() {
        d.column.short_name = default_short_name(i);
        if d.milliamp && i > 0 {
            d.column.formula = Some(format!("col({})*1000", default_short_name(i - 1)));
        }
        sheet.columns.push(d.column);
    }
    sheet
}

fn build_sheet(block: &Block, technique: &str, opts: &ChiOptions, warnings: &mut Vec<String>) -> (Worksheet, usize) {
    let first = &block.header[0];
    let is_potential = first.long.eq_ignore_ascii_case("potential");
    let is_time = first.long.eq_ignore_ascii_case("time");
    let current_idx = block.header.iter().position(is_current_a);
    let cv_technique = technique.is_empty() || technique.to_ascii_lowercase().contains("cyclic voltammetry");

    if opts.split_scans
        && is_potential
        && cv_technique
        && let Some(ci) = current_idx
    {
        let ranges = split_scans(&block.data[0]);
        if !ranges.is_empty() {
            if block.header.len() > 2 {
                warnings.push(
                    "the file has columns other than potential and current; they are not part of the scan sheet"
                        .to_string(),
                );
            }
            return (scan_sheet(block, ci, &ranges, opts, warnings), ranges.len());
        }
        warnings.push("the potential does not sweep back and forth, so the file was not split into scans".to_string());
    }

    let mut drafts = Vec::new();
    for (j, (h, values)) in block.header.iter().zip(&block.data).enumerate() {
        let designation = if j == 0 { Designation::X } else { Designation::Y };
        drafts.push(draft(designation, &h.long, &h.unit, "", values.clone()));
        if is_current_a(h) && opts.add_milliamp {
            drafts.push(milliamp_draft(values, ""));
        }
    }
    if let Some(ci) = current_idx {
        if is_time && opts.add_charge {
            let charge = running_charge(&block.data[0], &block.data[ci]);
            drafts.push(draft(Designation::Y, "Charge", "C", "", charge));
        }
        warnings.extend(clipping_warnings(None, first, &block.data[0], &block.data[ci]));
    }
    (finish_sheet("Data", drafts), 0)
}

fn scan_sheet(
    block: &Block,
    ci: usize,
    ranges: &[Range<usize>],
    opts: &ChiOptions,
    warnings: &mut Vec<String>,
) -> Worksheet {
    let mut drafts = Vec::new();
    for (k, range) in ranges.iter().enumerate() {
        let comment = format!("Scan {}", k + 1);
        let potential = &block.data[0][range.clone()];
        let current = &block.data[ci][range.clone()];
        drafts.push(draft(Designation::X, "Potential", "V", &comment, potential.to_vec()));
        drafts.push(draft(Designation::Y, "Current", "A", &comment, current.to_vec()));
        if opts.add_milliamp {
            drafts.push(milliamp_draft(current, &comment));
        }
        warnings.extend(clipping_warnings(Some(&comment), &block.header[0], potential, current));
    }
    finish_sheet("Scans", drafts)
}

/// Running trapezoidal integral of `current` over `time`, starting at 0.
fn running_charge(time: &[f64], current: &[f64]) -> Vec<f64> {
    let mut q = Vec::with_capacity(time.len());
    let mut total = 0.0;
    for i in 0..time.len() {
        if i > 0 {
            total += (time[i] - time[i - 1]) * (current[i] + current[i - 1]) / 2.0;
        }
        q.push(total);
    }
    q
}

/// Looks for a current that is stuck at its largest positive or negative
/// value for [`CLIP_MIN_ROWS`] or more consecutive rows.
fn clipping_warnings(scan: Option<&str>, x: &HeaderCol, xs: &[f64], current: &[f64]) -> Vec<String> {
    let mut out = Vec::new();
    for positive in [true, false] {
        let extreme = current
            .iter()
            .copied()
            .filter(|v| if positive { *v > 0.0 } else { *v < 0.0 })
            .max_by(|a, b| a.abs().total_cmp(&b.abs()));
        let Some(m) = extreme else { continue };
        let pinned = |v: f64| (v - m).abs() <= CLIP_TOLERANCE * m.abs();
        let (mut best_start, mut best_len) = (0, 0);
        let mut run_start = 0;
        let mut run_len = 0;
        for (i, &v) in current.iter().enumerate() {
            if pinned(v) {
                if run_len == 0 {
                    run_start = i;
                }
                run_len += 1;
                if run_len > best_len {
                    best_len = run_len;
                    best_start = run_start;
                }
            } else {
                run_len = 0;
            }
        }
        if best_len >= CLIP_MIN_ROWS {
            let (x0, x1) = (xs[best_start], xs[best_start + best_len - 1]);
            let who = scan.map(|s| format!("{s}: ")).unwrap_or_default();
            out.push(format!(
                "{who}{} current is pinned at {} mA for {best_len} consecutive rows ({} {x0} {} to {x1} {}); \
                 the current range of the instrument was probably exceeded",
                if positive { "positive" } else { "negative" },
                m * 1000.0,
                x.long,
                x.unit,
                x.unit
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Potential values that move through the given corner points in steps
    /// of `steps[i]` rows between corner `i` and `i+1` (corner included once).
    fn path(corners: &[f64], rows_between: &[usize]) -> Vec<f64> {
        let mut e = vec![corners[0]];
        for (w, &n) in corners.windows(2).zip(rows_between) {
            for k in 1..=n {
                e.push(w[0] + (w[1] - w[0]) * k as f64 / n as f64);
            }
        }
        e
    }

    fn cv_text(technique: &str, potential: &[f64], current: impl Fn(usize, f64) -> f64, sep: &str) -> String {
        let mut s = format!(
            "Sept. 12, 2026   14:03:22\r\n{technique}\r\nFile: C:\\data\\run1.bin\r\nData Source: Experiment\r\n\
             Instrument Model:  CHI627E\r\nHeader:\r\nNote:\r\n\r\nInit E (V) = 1.2\r\nHigh E (V) = 1.2\r\nLow E (V) = 0\r\n\
             Init P/N = N\r\nScan Rate (V/s) = 0.001\r\nSegment = 6\r\nSample Interval (V) = 0.001\r\n\
             Quiet Time (sec) = 2\r\nSensitivity (A/V) = 1e-3\r\n\r\nPotential/V{sep} Current/A\r\n\r\n"
        );
        for (i, &e) in potential.iter().enumerate() {
            s.push_str(&format!("{e:.3}{sep} {:e}\r\n", current(i, e)));
        }
        crate::text::normalize_newlines(&s)
    }

    fn range_1based(r: &Range<usize>) -> (usize, usize) {
        (r.start + 1, r.end)
    }

    #[test]
    fn three_scans_of_a_six_segment_run() {
        // 1.2 V down to 0 and back, three times: 6 segments of 1200 steps.
        let e = path(&[1.2, 0.0, 1.2, 0.0, 1.2, 0.0, 1.2], &[1200; 6]);
        assert_eq!(e.len(), 7201);
        let r = split_scans(&e);
        // Vertices are rows 1200, 2400, ..., 6000; every second one cuts.
        assert_eq!(r, vec![0..2400, 2400..4800, 4800..7201]);
        // The vertex row 2400 (the 1.2 V extreme) opens scan 2 and is not repeated.
        assert!((e[2400] - 1.2).abs() < 1e-12 && e[2399] < e[2400] && e[2401] < e[2400]);
    }

    #[test]
    fn owner_check_value_7000_rows() {
        // Start at 1.0 V, sweep up to 1.2 V (1000 rows), then 1.2 -> 0 -> 1.2 ... with 1200-row segments.
        let mut e = path(&[1.0, 1.2], &[1000]);
        let mut corners = vec![1.2];
        for i in 0..5 {
            corners.push(if i % 2 == 0 { 0.0 } else { 1.2 });
        }
        e.extend_from_slice(&path(&corners, &[1200; 5])[1..]);
        e.truncate(7000);
        assert_eq!(e.len(), 7000);
        let r = split_scans(&e);
        let one_based: Vec<_> = r.iter().map(range_1based).collect();
        assert_eq!(one_based, vec![(1, 2200), (2201, 4600), (4601, 7000)]);
        assert_eq!(r.iter().map(|r| r.len()).collect::<Vec<_>>(), vec![2200, 2400, 2400]);
    }

    #[test]
    fn flat_steps_keep_direction_and_short_tails_are_dropped() {
        let e = [0.0, 0.0, 1.0, 1.0, 2.0, 1.0, 1.0, 0.0, 0.0, 1.0];
        // Vertices at rows 4 and 8; the cut at 8 leaves a 2-row tail that is dropped.
        assert_eq!(split_scans(&e), vec![0..8]);
        assert_eq!(split_scans(&[1.0, 1.0, 1.0, 1.0]), vec![0..4], "no movement: one scan");
        assert!(split_scans(&[]).is_empty() && split_scans(&[1.0]).is_empty());
    }

    #[test]
    fn single_sweep_is_one_scan() {
        let e: Vec<f64> = (0..500).map(|k| k as f64 * 0.001).collect();
        assert_eq!(split_scans(&e), vec![0..500]);
    }

    #[test]
    fn imports_a_cv_with_scans_milliamp_and_meta() {
        let e = path(&[1.2, 0.0, 1.2, 0.0, 1.2], &[60; 4]);
        let text = cv_text("Cyclic Voltammetry", &e, |_, v| v * 1e-5, ",");
        let imp = parse_chi(&text, "run1", &ChiOptions::default()).unwrap();
        assert_eq!(imp.scans, 2);
        assert_eq!(imp.technique, "Cyclic Voltammetry");
        let names: Vec<&str> = imp.meta.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(&names[..3], ["Technique", "Date", "Instrument Model"]);
        assert_eq!(
            &names[3..],
            [
                "Init E (V)",
                "High E (V)",
                "Low E (V)",
                "Init P/N",
                "Scan Rate (V/s)",
                "Segment",
                "Sample Interval (V)",
                "Quiet Time (sec)",
                "Sensitivity (A/V)"
            ]
        );
        assert_eq!(imp.meta.iter().find(|(k, _)| k == "Segment").unwrap().1, "6");
        assert_eq!(imp.meta[2].1, "CHI627E");
        assert!(imp.warnings.is_empty(), "{:?}", imp.warnings);

        let sheet = &imp.workbook.sheets[0];
        assert_eq!(sheet.name, "Scans");
        assert_eq!(sheet.columns.len(), 6);
        let c = &sheet.columns;
        assert_eq!(c[0].designation, Designation::X);
        assert_eq!(
            (c[0].long_name.as_str(), c[0].units.as_str(), c[0].comments.as_str()),
            ("Potential", "V", "Scan 1")
        );
        assert_eq!((c[2].long_name.as_str(), c[2].units.as_str()), ("Current", "mA"));
        assert_eq!(c[2].formula.as_deref(), Some("col(B)*1000"));
        assert_eq!(c[5].formula.as_deref(), Some("col(E)*1000"));
        assert_eq!(c[3].comments, "Scan 2");
        let a = c[1].numeric().unwrap();
        let ma = c[2].numeric().unwrap();
        assert!(a.iter().zip(ma).all(|(a, m)| (a * 1000.0 - m).abs() < 1e-12));
        assert_eq!(c[0].len() + c[3].len(), e.len());
        assert_eq!(imp.workbook.long_name, "run1");
    }

    #[test]
    fn tab_separated_variant_and_options() {
        let e = path(&[0.0, 1.0], &[100]);
        let text = cv_text("Linear Sweep Voltammetry", &e, |_, v| v, "\t");
        let opts = ChiOptions { add_milliamp: false, invert_current: true, ..ChiOptions::default() };
        let imp = parse_chi(&text, "lsv", &opts).unwrap();
        assert_eq!(imp.scans, 0, "LSV technique is not split");
        let sheet = &imp.workbook.sheets[0];
        assert_eq!(sheet.name, "Data");
        assert_eq!(sheet.columns.len(), 2);
        assert_eq!(sheet.row_count(), 101);
        let i = sheet.columns[1].numeric().unwrap();
        assert!((i[100] + 1.0).abs() < 1e-12, "current inverted");
    }

    #[test]
    fn missing_technique_with_one_sweep_gives_one_scan() {
        let e = path(&[0.0, 1.0], &[100]);
        let text =
            "Potential/V, Current/A\n".to_string() + &e.iter().map(|v| format!("{v:.3}, {v:e}\n")).collect::<String>();
        let imp = parse_chi(&text, "x", &ChiOptions::default()).unwrap();
        assert_eq!(imp.scans, 1);
        assert_eq!(imp.technique, "");
    }

    #[test]
    fn it_file_gets_a_charge_column() {
        // i(t) = 2e-3 * t is linear, so the trapezoid rule is exact: Q = 1e-3 * t^2.
        let mut text =
            String::from("Aug. 1, 2026\nAmperometric i-t Curve\nInit E (V) = 0.5\n\nTime/sec, Current/A\n\n");
        for k in 0..=100 {
            let t = k as f64 * 0.5;
            text.push_str(&format!("{t}, {:e}\n", 2e-3 * t));
        }
        let imp = parse_chi(&text, "it", &ChiOptions::default()).unwrap();
        let s = &imp.workbook.sheets[0];
        assert_eq!(s.columns[0].units, "s", "sec is shown as s");
        assert_eq!(s.columns.len(), 4);
        let q = &s.columns[3];
        assert_eq!((q.long_name.as_str(), q.units.as_str()), ("Charge", "C"));
        let q = q.numeric().unwrap();
        assert_eq!(q[0], 0.0);
        assert!((q[100] - 1e-3 * 50.0 * 50.0).abs() < 1e-12, "{}", q[100]);
        assert!(imp.warnings.is_empty());
    }

    #[test]
    fn clipping_needs_twenty_identical_rows() {
        let e = path(&[0.0, 1.0, 0.0], &[200, 200]);
        let make = |run: usize| {
            cv_text(
                "Cyclic Voltammetry",
                &e,
                move |i, v| if (100..100 + run).contains(&i) { 1.5e-3 } else { v * 1e-4 },
                ",",
            )
        };
        let w20 = parse_chi(&make(20), "clip", &ChiOptions::default()).unwrap().warnings;
        assert_eq!(w20.len(), 1, "{w20:?}");
        assert!(
            w20[0].contains("Scan 1") && w20[0].contains("1.5 mA") && w20[0].contains("20 consecutive"),
            "{}",
            w20[0]
        );
        assert!(parse_chi(&make(19), "clip", &ChiOptions::default()).unwrap().warnings.is_empty());
        assert!(parse_chi(&make(5), "clip", &ChiOptions::default()).unwrap().warnings.is_empty());
    }

    #[test]
    fn repeated_headers_append_or_start_a_new_sheet() {
        let text = "Time/sec, Current/A\n0, 1\n1, 2\nTime/sec, Current/A\n2, 3\nfree text\nFreq/Hz, Z'/ohm\n10, 5\n";
        let imp = parse_chi(text, "multi", &ChiOptions::default()).unwrap();
        assert_eq!(imp.workbook.sheets.len(), 2);
        assert_eq!(imp.workbook.sheets[0].row_count(), 3);
        assert_eq!(imp.workbook.sheets[1].name, "Data 2");
        assert_eq!(imp.workbook.sheets[1].columns[1].long_name, "Z'");
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        for text in ["", "hello\nworld\n", "\u{1}\u{2}\u{ffff}", "Potential/V, Current/A\n", "1,2\n3,4\n"] {
            assert!(!looks_like_chi(text) || parse_chi(text, "bad", &ChiOptions::default()).is_err());
            assert!(parse_chi(text, "bad", &ChiOptions::default()).is_err(), "{text:?}");
        }
        let msg = parse_chi("nonsense", "bad", &ChiOptions::default()).unwrap_err().to_string();
        assert!(msg.contains("bad"));
    }

    #[test]
    fn detection() {
        assert!(looks_like_chi("a\nb\nPotential/V, Current/A\n1,2\n"));
        assert!(looks_like_chi("time/s\tcurrent/a\n1\t2\n"));
        assert!(!looks_like_chi("x,y\n1,2\n"));
    }
}
