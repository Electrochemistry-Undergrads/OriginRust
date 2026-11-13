//! CH Instruments (CHI) potentiostat text exports.
//!
//! See `docs/spec/chi-text-format.md`. A file is a block of header lines
//! (date, technique, `name = value` parameters, optional results), a column
//! header such as `Potential/V, Current/A`, and numeric rows. Cyclic
//! voltammetry files with several cycles are split into one pair of columns
//! per scan; chronopotentiometry (charge/discharge) files also get one group
//! of columns per segment. Times that the file rounds to 4 significant digits
//! are recomputed from the sampling interval in the header.

use crate::error::IoError;
use sheaf_core::column::default_short_name;
use sheaf_core::{Column, ColumnData, ColumnId, Designation, WindowId, Workbook, Worksheet};
use std::ops::Range;

/// Number of leading lines searched for the column header line.
/// A CV with many segments lists its peaks segment by segment before the column header (a real
/// export has the header at line 6906), so the window is large.
const HEADER_SEARCH_LINES: usize = 100_000;
/// A current that stays at one value for this many rows is reported as clipped.
const CLIP_MIN_ROWS: usize = 20;
/// Relative tolerance for "the same value" when looking for clipping.
const CLIP_TOLERANCE: f64 = 1e-9;
/// A chronopotentiometry segment ends where the potential has turned back by
/// this fraction of the whole potential range (spec section 2.4).
const SEGMENT_TURN_FRACTION: f64 = 0.05;
/// A turn counts as the instrument's switch at a potential limit when the
/// extreme is within this fraction of the limit span of the limit.
const SEGMENT_LIMIT_FRACTION: f64 = 0.02;

/// What the CHI importer should do. The defaults follow the spec.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChiOptions {
    /// Split a multi-cycle CV file into scans (default on).
    pub split_scans: bool,
    /// Add a sheet with one group of columns per charge or discharge segment
    /// to a chronopotentiometry file (default on).
    pub split_segments: bool,
    /// Add a current column in mA after every current column in A (default on).
    pub add_milliamp: bool,
    /// For time-based files with a current column, add the running charge (default on).
    pub add_charge: bool,
    /// Multiply the current by -1 (default off).
    pub invert_current: bool,
    /// Replace times that the file rounds to 4 significant digits by exact
    /// multiples of the sampling interval named in the header (default on).
    pub exact_time: bool,
}

impl Default for ChiOptions {
    fn default() -> Self {
        ChiOptions {
            split_scans: true,
            split_segments: true,
            add_milliamp: true,
            add_charge: true,
            invert_current: false,
            exact_time: true,
        }
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
    /// Number of chronopotentiometry segments in the `Segments` sheet, 0 when
    /// there is no such sheet.
    pub segments: usize,
    /// `Technique`, `Date`, `Instrument Model` (when present), `Header` and `Note` (when the
    /// run has such a text), then every
    /// `name = value` line of the header in file order. Lines of the
    /// `Results:` block carry their heading, for example `Segment 2: Ep`.
    pub meta: Vec<(String, String)>,
    /// Things the user should look at, for example a clipped current.
    pub warnings: Vec<String>,
}

/// One column of a column-header line: `Potential/V` is ("Potential", "V").
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HeaderCol {
    pub(crate) long: String,
    pub(crate) unit: String,
}

impl HeaderCol {
    pub(crate) fn new(long: &str, unit: &str) -> Self {
        HeaderCol { long: long.to_string(), unit: unit.to_string() }
    }
}

#[derive(Debug)]
pub(crate) struct Block {
    pub(crate) header: Vec<HeaderCol>,
    /// One vector of values per header column.
    pub(crate) data: Vec<Vec<f64>>,
    /// For each row of a text file, the step of the last digit the file
    /// printed in the first column (`2.023e+3` gives 1, `742.6` gives 0.1).
    /// Empty for binary files, whose values are exact.
    pub(crate) x_step: Vec<f64>,
}

impl Block {
    pub(crate) fn new(header: Vec<HeaderCol>) -> Self {
        Block { data: vec![Vec::new(); header.len()], header, x_step: Vec::new() }
    }
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

/// The numbers of a data row and the printed step of its first field.
fn parse_row(line: &str) -> Option<(Vec<f64>, f64)> {
    let mut values = Vec::new();
    let mut step = f64::NAN;
    for f in line.split([',', '\t']).map(str::trim).filter(|f| !f.is_empty()) {
        let value = f.parse::<f64>().ok()?;
        if values.is_empty() {
            step = printed_step(f);
        }
        values.push(value);
    }
    (values.len() >= 2).then_some((values, step))
}

/// The value of one unit in the last printed digit of a number:
/// `2.023e+3` gives 1, `742.6` gives 0.1, `1200` gives 1. Only called on
/// text that parsed as a number; absurd exponents give 0 or infinity, never
/// an overflow.
fn printed_step(field: &str) -> f64 {
    let (mantissa, exponent) = match field.find(['e', 'E']) {
        Some(i) => {
            let digits = &field[i + 1..];
            // An exponent too long for i64 is still a valid f64 (0 or infinity): saturate it.
            let saturated = if digits.starts_with('-') { i64::MIN } else { i64::MAX };
            (&field[..i], digits.parse::<i64>().unwrap_or(saturated))
        }
        None => (field, 0),
    };
    let decimals = mantissa.split_once('.').map_or(0, |(_, frac)| frac.len()) as i64;
    let power = exponent.saturating_sub(decimals).clamp(-400, 400) as i32;
    10f64.powi(power)
}

fn find_header(lines: &[&str]) -> Option<usize> {
    lines.iter().take(HEADER_SEARCH_LINES).position(|l| parse_header_line(l).is_some())
}

/// True when the text has a CHI column-header line within its first 100000 lines.
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
    Ok(assemble(blocks, technique, meta, file_stem, opts, None))
}

/// Turns data blocks into a workbook: scan splitting, mA and charge columns,
/// clipping warnings. Shared by the text and the binary importer, so that a
/// file gives the same workbook whichever way it was saved.
///
/// `clip_digits` is `Some(4)` for binary files: the text export keeps 4 significant digits of a
/// current, so two stored values that agree to 4 digits count as the same value when looking for a
/// pinned current. `None` compares exactly (text files already are rounded).
pub(crate) fn assemble(
    blocks: Vec<Block>,
    technique: String,
    meta: Vec<(String, String)>,
    file_stem: &str,
    opts: &ChiOptions,
    clip_digits: Option<usize>,
) -> ChiImport {
    let mut warnings = Vec::new();
    let mut scans = 0;
    let mut segments = 0;
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
        if opts.exact_time
            && is_time(&block.header[0])
            && let Some(note) = recompute_times(&mut block, &meta)
        {
            warnings.push(note);
        }
        let built = build_sheet(&block, &technique, &meta, opts, clip_digits, &mut warnings);
        scans += built.scans;
        segments += built.segments;
        for mut sheet in built.sheets {
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
    }

    let mut workbook = Workbook::new(WindowId(0), "Book1");
    workbook.long_name = file_stem.to_string();
    workbook.sheets = sheets;
    ChiImport { workbook, technique, scans, segments, meta, warnings }
}

fn is_time(h: &HeaderCol) -> bool {
    h.long.eq_ignore_ascii_case("time") && h.unit == "s"
}

/// The sampling interval in seconds from a header line such as
/// `Data Storage Interval (s) = 0.2` or `Sample Interval (s) = 0.1`, with
/// the line's name.
fn sampling_interval(meta: &[(String, String)]) -> Option<(&str, f64)> {
    meta.iter().find_map(|(name, value)| {
        let lower = name.to_ascii_lowercase();
        let interval = lower.starts_with("sample interval") || lower.starts_with("data storage interval");
        let seconds = lower.ends_with("(s)") || lower.ends_with("(sec)");
        let dt = value.trim().parse::<f64>().ok()?;
        (interval && seconds && dt.is_finite() && dt > 0.0).then_some((name.as_str(), dt))
    })
}

/// CHI prints times with 4 significant digits, so above 1000 s they come in
/// whole seconds and repeat. When every time of the block is the rounded
/// value of a multiple of the header's sampling interval, the times are
/// replaced by those multiples (spec section 2.5). Returns the note for the
/// user, or `None` when nothing was changed.
fn recompute_times(block: &mut Block, meta: &[(String, String)]) -> Option<String> {
    let (name, dt) = sampling_interval(meta)?;
    let t = &block.data[0];
    let first = *t.first()?;
    let m = (first / dt).round();
    // Dividing by a whole number of samples per second (5 for 0.2 s) gives the
    // correctly rounded value (0.6, not 0.6000000000000001).
    let per_second = (1.0 / dt).round();
    let divide = per_second >= 1.0 && ((1.0 / dt) - per_second).abs() <= 1e-9 * per_second;
    let exact = |k: usize| {
        let n = m + k as f64;
        if divide { n / per_second } else { n * dt }
    };
    let mut changed = 0usize;
    let mut largest_step = 0.0f64;
    for (k, (&v, &step)) in t.iter().zip(&block.x_step).enumerate() {
        let e = exact(k);
        let off = (v - e).abs();
        let allowed = 0.5 * step * (1.0 + 1e-9) + 1e-12 * e.abs();
        // A NaN time or step is not within the allowance either.
        if off.is_nan() || allowed.is_nan() || off > allowed {
            return None;
        }
        if off > 1e-12 * e.abs().max(dt) {
            changed += 1;
            largest_step = largest_step.max(step);
        }
    }
    if changed == 0 {
        return None;
    }
    let rows = t.len();
    block.data[0] = (0..rows).map(exact).collect();
    Some(format!(
        "{changed} of {rows} times in the file are rounded (to steps of up to {largest_step} s); \
         all times were recomputed as multiples of {name} = {dt}"
    ))
}

/// A heading line such as `Results:`, `Channel 1:` or `Segment 2:` (a name
/// followed by a colon and nothing else). Returns the name.
fn heading(line: &str) -> Option<&str> {
    let name = line.strip_suffix(':')?.trim();
    (!name.is_empty() && !name.contains(':')).then_some(name)
}

fn is_channel(name: &str) -> bool {
    name.to_ascii_lowercase().starts_with("channel")
}

/// Reads date, technique, instrument model and `name = value` lines from the
/// lines before the column header. A `name = value` line inside the
/// `Results:` block is named after its headings (`Segment 2: Ep`; the
/// channel is added only when the block lists more than one).
fn read_preamble(lines: &[&str]) -> (String, Vec<(String, String)>) {
    let mut technique = String::new();
    let mut date = String::new();
    let mut instrument = String::new();
    let (mut header_text, mut note) = (String::new(), String::new());
    let mut params = Vec::new();
    let mut seen = 0;
    let channels = lines.iter().filter_map(|l| heading(l.trim())).filter(|h| is_channel(h)).count();
    let mut in_results = false;
    let (mut channel, mut section) = (String::new(), String::new());
    for raw in lines {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((name, value)) = line.split_once(" = ") {
            let (name, value) = (name.trim(), value.trim());
            if !name.is_empty() {
                let name = if in_results {
                    let mut label = String::new();
                    if channels > 1 && !channel.is_empty() {
                        label.push_str(&channel);
                    }
                    if !section.is_empty() {
                        if !label.is_empty() {
                            label.push_str(", ");
                        }
                        label.push_str(&section);
                    }
                    if label.is_empty() { name.to_string() } else { format!("{label}: {name}") }
                } else {
                    name.to_string()
                };
                params.push((name, value.to_string()));
            }
            seen += 1;
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            match key.trim().to_ascii_lowercase().as_str() {
                "instrument model" => instrument = value.trim().to_string(),
                "header" => header_text = value.trim().to_string(),
                "note" => note = value.trim().to_string(),
                _ => {}
            }
            if matches!(key.trim().to_ascii_lowercase().as_str(), "file" | "data source" | "header" | "note")
                || key.trim().eq_ignore_ascii_case("instrument model")
            {
                seen += 1;
                continue;
            }
        }
        if seen >= 2
            && let Some(name) = heading(line)
        {
            if name.eq_ignore_ascii_case("results") {
                in_results = true;
                channel.clear();
                section.clear();
            } else if in_results && is_channel(name) {
                channel = name.to_string();
                section.clear();
            } else if in_results {
                section = name.to_string();
            }
            seen += 1;
            continue;
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
    for (name, value) in [("Header", header_text), ("Note", note)] {
        if !value.is_empty() {
            meta.push((name.to_string(), value));
        }
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
                Some(last) if last.rows() == 0 => *last = Block::new(header),
                Some(last) if last.header == header => {}
                _ => blocks.push(Block::new(header)),
            }
        } else if let (Some((values, step)), Some(block)) = (parse_row(line), blocks.last_mut()) {
            for (i, col) in block.data.iter_mut().enumerate() {
                col.push(values.get(i).copied().unwrap_or(f64::NAN));
            }
            block.x_step.push(step);
        }
    }
    blocks.retain(|b| b.rows() > 0);
    blocks
}

/// A current column in A: `Current/A`, or `i1/A`, `i2/A`, ... in files with several channels.
fn is_current_a(h: &HeaderCol) -> bool {
    if h.unit != "A" {
        return false;
    }
    let long = h.long.to_ascii_lowercase();
    long == "current" || long.strip_prefix('i').is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
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

/// The sheets made from one data block.
struct Built {
    sheets: Vec<Worksheet>,
    scans: usize,
    segments: usize,
}

fn build_sheet(
    block: &Block,
    technique: &str,
    meta: &[(String, String)],
    opts: &ChiOptions,
    clip_digits: Option<usize>,
    warnings: &mut Vec<String>,
) -> Built {
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
            let sheet = scan_sheet(block, ci, &ranges, opts, clip_digits, warnings);
            return Built { sheets: vec![sheet], scans: ranges.len(), segments: 0 };
        }
        warnings.push("the potential does not sweep back and forth, so the file was not split into scans".to_string());
    }

    let mut drafts = Vec::new();
    let several = block.header.iter().filter(|h| is_current_a(h)).count() > 1;
    for (j, (h, values)) in block.header.iter().zip(&block.data).enumerate() {
        let designation = if j == 0 { Designation::X } else { Designation::Y };
        drafts.push(draft(designation, &h.long, &h.unit, "", values.clone()));
        if is_current_a(h) && opts.add_milliamp {
            drafts.push(milliamp_draft(values, if several { &h.long } else { "" }));
        }
    }
    // With several current channels the charge columns and the warnings name the channel.
    let currents: Vec<usize> = (0..block.header.len()).filter(|&j| is_current_a(&block.header[j])).collect();
    for &ci in &currents {
        let channel = if currents.len() > 1 { block.header[ci].long.as_str() } else { "" };
        if is_time && opts.add_charge {
            let charge = running_charge(&block.data[0], &block.data[ci]);
            drafts.push(draft(Designation::Y, "Charge", "C", channel, charge));
        }
        let who = (!channel.is_empty()).then_some(channel);
        warnings.extend(clipping_warnings(who, first, &block.data[0], &block.data[ci], clip_digits));
    }
    let mut built = Built { sheets: vec![finish_sheet("Data", drafts)], scans: 0, segments: 0 };

    let potential_idx = block.header.iter().position(|h| h.long.eq_ignore_ascii_case("potential") && h.unit == "V");
    if opts.split_segments
        && is_time
        && technique.to_ascii_lowercase().contains("chronopotentiometry")
        && let Some(pi) = potential_idx
    {
        let segments = chrono_segments(&block.data[0], &block.data[pi], meta);
        if segments.len() >= 2 {
            built.segments = segments.len();
            built.sheets.push(segment_sheet(&block.data[0], &block.data[pi], &segments, meta));
        }
    }
    built
}

/// One charge or discharge segment of a chronopotentiometry run.
#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    /// Half-open row range.
    pub rows: Range<usize>,
    /// True when the current was cathodic (the potential falls).
    pub cathodic: bool,
    /// The time at which the segment's current started, when the header
    /// gives the switching times; otherwise `None`.
    pub start_time: Option<f64>,
}

/// A number from a `name = value` header line, matched case-insensitively.
fn header_number(meta: &[(String, String)], name: &str) -> Option<f64> {
    meta.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .and_then(|(_, v)| v.trim().parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

/// Splits a chronopotentiometry run into charge and discharge segments
/// (spec section 2.4). A run whose header says the current switches on time
/// is cut at the switching times; any other run is cut where the potential
/// turns back at a potential limit (or after the segment's maximum time).
pub fn chrono_segments(time: &[f64], potential: &[f64], meta: &[(String, String)]) -> Vec<Segment> {
    let by_time = meta
        .iter()
        .any(|(k, v)| k.to_ascii_lowercase().starts_with("current switching") && v.trim().eq_ignore_ascii_case("time"));
    // Text exports write `Init P/N = N`; the binary reader writes `Initial Polarity = Cathodic`.
    let cathodic_first = meta
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Init P/N") || k.eq_ignore_ascii_case("Initial Polarity"))
        .and_then(|(_, v)| match v.trim().to_ascii_lowercase().as_str() {
            "n" | "cathodic" => Some(true),
            "p" | "anodic" => Some(false),
            _ => None,
        });
    let seconds = |name: &str| {
        header_number(meta, &format!("{name} (s)"))
            .or_else(|| header_number(meta, &format!("{name} (sec)")))
            .filter(|t| *t > 0.0)
    };
    let cathodic_time = seconds("Cathodic Time");
    let anodic_time = seconds("Anodic Time");
    if by_time && let (Some(first), Some(tc), Some(ta)) = (cathodic_first, cathodic_time, anodic_time) {
        return segments_by_time(time, first, tc, ta);
    }

    let high = header_number(meta, "High E Limit (V)");
    let low = header_number(meta, "Low E Limit (V)");
    let tolerance = match (high, low) {
        (Some(h), Some(l)) if h > l => Some(SEGMENT_LIMIT_FRACTION * (h - l)),
        _ => None,
    };
    let step = if time.len() > 1 { (time[1] - time[0]).abs() } else { 0.0 };
    // A turn at row `ext` after a rising (or falling) stretch that began at row `start` is a real
    // switch when the extreme is at the potential limit, or when the stretch lasted the segment's
    // maximum time. Without limits in the header every turn counts.
    let accept = |ext: usize, rising: bool, start: usize| {
        let Some(tol) = tolerance else { return true };
        let limit = if rising { high } else { low };
        let at_limit = limit.is_some_and(|l| (potential[ext] - l).abs() <= tol);
        let max_time = if rising { anodic_time } else { cathodic_time };
        let origin = if start == 0 { 0.0 } else { time[start - 1] };
        let timed_out = max_time.is_some_and(|m| time[ext] - origin >= m - 1.5 * step);
        at_limit || timed_out
    };
    let turns = split_at_turns(potential, accept);
    let mut segments: Vec<Segment> = Vec::new();
    for (rows, rising) in turns {
        segments.push(Segment { rows, cathodic: !rising, start_time: None });
    }
    // A run whose direction never became clear is one segment; Init P/N names its current.
    if segments.len() == 1
        && let Some(first) = cathodic_first
    {
        segments[0].cathodic = first;
    }
    segments
}

/// Cuts a time-switched run at `tc`, `tc + ta`, `2 tc + ta`, ... (or starting
/// with `ta` when the first current is anodic). A row belongs to the segment
/// whose switching time it has not passed.
fn segments_by_time(time: &[f64], cathodic_first: bool, tc: f64, ta: f64) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut start_row = 0;
    let mut start_time = 0.0;
    let mut cathodic = cathodic_first;
    let n = time.len();
    while start_row < n {
        let end_time = start_time + if cathodic { tc } else { ta };
        let tolerance = 1e-9 * end_time.abs().max(1.0);
        // A missing time (NaN) is not past the switch, so it stays in the current segment.
        let mut end_row = start_row;
        while end_row < n && (time[end_row].is_nan() || time[end_row] <= end_time + tolerance) {
            end_row += 1;
        }
        if end_row > start_row {
            segments.push(Segment { rows: start_row..end_row, cathodic, start_time: Some(start_time) });
        } else {
            let t = time[start_row];
            if !t.is_finite() {
                // An infinite time cannot be placed; it joins the segment before it.
                match segments.last_mut() {
                    Some(last) => last.rows.end = start_row + 1,
                    None => segments.push(Segment { rows: start_row..start_row + 1, cathodic, start_time: Some(0.0) }),
                }
                start_row += 1;
                continue;
            }
            // The next row lies beyond this segment (a gap in the data). The next segment is of the
            // other kind and starts at `end_time`; whole cycles later it starts again, so skip them
            // at once instead of one empty segment at a time.
            let cycles = ((t - end_time) / (tc + ta)).floor();
            if cycles >= 1.0 {
                start_time = end_time + cycles * (tc + ta);
                cathodic = !cathodic;
                continue;
            }
        }
        start_row = end_row;
        start_time = end_time;
        cathodic = !cathodic;
    }
    segments
}

/// Splits a potential trace at its turning points.
///
/// The direction (rising or falling) is known once the potential has moved
/// by [`SEGMENT_TURN_FRACTION`] of its whole range. A stretch ends at its
/// extreme row (the later one on ties) once the potential has come back by
/// that much and `accept(extreme, rising, stretch_start)` agrees; a turn that
/// is not accepted is ignored. Returns each stretch's row range and whether
/// it was rising. A stretch of 2 rows or fewer is joined to the next one.
fn split_at_turns(potential: &[f64], accept: impl Fn(usize, bool, usize) -> bool) -> Vec<(Range<usize>, bool)> {
    let n = potential.len();
    let finite = || potential.iter().copied().filter(|v| v.is_finite());
    let (Some(lo), Some(hi)) = (finite().reduce(f64::min), finite().reduce(f64::max)) else {
        return Vec::new();
    };
    let first = potential.iter().position(|v| v.is_finite()).unwrap_or(0);
    let last = potential.iter().rposition(|v| v.is_finite()).unwrap_or(0);
    let overall_rising = potential[last] >= potential[first];
    let threshold = (hi - lo) * SEGMENT_TURN_FRACTION;
    // Index of the largest (rising) or smallest (falling) finite value in `from..=to`, the later
    // one on ties.
    let extreme = |from: usize, to: usize, rising: bool| {
        (from..=to).filter(|&i| potential[i].is_finite()).fold(None, |best: Option<usize>, i| match best {
            Some(b) if (rising && potential[i] < potential[b]) || (!rising && potential[i] > potential[b]) => Some(b),
            _ => Some(i),
        })
    };
    // (row range, rising) of each stretch; the cut rows are where the next stretch starts.
    let mut stretches: Vec<(Range<usize>, bool)> = Vec::new();
    let mut start = 0usize;
    // `None` until the direction is known; then true for rising.
    let mut rising: Option<bool> = None;
    if threshold > 0.0 {
        let (mut lowest, mut highest) = (None::<usize>, None::<usize>);
        let mut ext = 0usize;
        for (k, &v) in potential.iter().enumerate() {
            if !v.is_finite() {
                continue;
            }
            match rising {
                None => {
                    if lowest.is_none_or(|i| v <= potential[i]) {
                        lowest = Some(k);
                    }
                    if highest.is_none_or(|i| v >= potential[i]) {
                        highest = Some(k);
                    }
                    let (lo_i, hi_i) = (lowest.unwrap_or(k), highest.unwrap_or(k));
                    if v - potential[lo_i] >= threshold {
                        rising = Some(true);
                        ext = extreme(lo_i, k, true).unwrap_or(k);
                    } else if potential[hi_i] - v >= threshold {
                        rising = Some(false);
                        ext = extreme(hi_i, k, false).unwrap_or(k);
                    }
                }
                Some(up) => {
                    let beyond = if up { v >= potential[ext] } else { v <= potential[ext] };
                    let back = if up { potential[ext] - v } else { v - potential[ext] };
                    if beyond {
                        ext = k;
                    } else if back >= threshold && accept(ext, up, start) {
                        stretches.push((start..ext + 1, up));
                        start = ext + 1;
                        rising = Some(!up);
                        ext = extreme(start, k, !up).unwrap_or(k);
                    }
                }
            }
        }
    }
    stretches.push((start..n, rising.unwrap_or(overall_rising)));
    // Join stretches of 2 rows or fewer to the next one (the last to the one before it), so that
    // no row is lost.
    let mut joined: Vec<(Range<usize>, bool)> = Vec::new();
    let mut carry: Option<usize> = None;
    for (rows, up) in stretches {
        let rows = carry.take().unwrap_or(rows.start)..rows.end;
        if rows.len() <= 2 {
            carry = Some(rows.start);
        } else {
            joined.push((rows, up));
        }
    }
    if let Some(from) = carry {
        match joined.last_mut() {
            Some(last) => last.0.end = n.max(last.0.end),
            None if n > 0 => joined.push((from..n, overall_rising)),
            None => {}
        }
    }
    joined
}

/// The sheet with one group of columns per chronopotentiometry segment: time
/// since the segment's current started, potential and, when the header gives
/// both currents, the capacity passed so far in mAh.
fn segment_sheet(time: &[f64], potential: &[f64], segments: &[Segment], meta: &[(String, String)]) -> Worksheet {
    let currents = header_number(meta, "Cathodic Current (A)").zip(header_number(meta, "Anodic Current (A)"));
    let mut drafts = Vec::new();
    for (k, seg) in segments.iter().enumerate() {
        let range = seg.rows.clone();
        let comment = format!("Segment {} ({})", k + 1, if seg.cathodic { "cathodic" } else { "anodic" });
        // Without switching times, a segment's clock starts at the last row of the segment before
        // it; the first segment's starts at 0 when its first sample is about one interval after 0.
        let origin = match seg.start_time {
            Some(t) => t,
            None if range.start > 0 => time[range.start - 1],
            None if time.len() > 1 && time[0] >= 0.0 && time[0] <= 1.5 * (time[1] - time[0]) => 0.0,
            None => time[0],
        };
        let since: Vec<f64> = time[range.clone()].iter().map(|t| t - origin).collect();
        let capacity = currents.map(|(cathodic, anodic)| {
            let amps = if seg.cathodic { cathodic } else { anodic }.abs();
            since.iter().map(|t| amps * t / 3.6).collect::<Vec<f64>>()
        });
        drafts.push(draft(Designation::X, "Time", "s", &comment, since));
        drafts.push(draft(Designation::Y, "Potential", "V", &comment, potential[range].to_vec()));
        if let Some(capacity) = capacity {
            drafts.push(draft(Designation::Y, "Capacity", "mAh", &comment, capacity));
        }
    }
    finish_sheet("Segments", drafts)
}

fn scan_sheet(
    block: &Block,
    ci: usize,
    ranges: &[Range<usize>],
    opts: &ChiOptions,
    clip_digits: Option<usize>,
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
        warnings.extend(clipping_warnings(Some(&comment), &block.header[0], potential, current, clip_digits));
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

/// `v` rounded to `digits` significant digits (what a text export would show).
fn round_significant(v: f64, digits: usize) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    format!("{:.*e}", digits.saturating_sub(1), v).parse().unwrap_or(v)
}

/// Looks for a current that is stuck at its largest positive or negative
/// value for [`CLIP_MIN_ROWS`] or more consecutive rows.
fn clipping_warnings(
    scan: Option<&str>,
    x: &HeaderCol,
    xs: &[f64],
    current: &[f64],
    clip_digits: Option<usize>,
) -> Vec<String> {
    let rounded: Vec<f64>;
    let current = match clip_digits {
        Some(d) => {
            rounded = current.iter().map(|&v| round_significant(v, d)).collect();
            &rounded[..]
        }
        None => current,
    };
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
    fn a_pinned_current_is_found_in_every_numbered_current_column_of_an_i_t_text() {
        let mut text = String::from(
            "Sept. 17, 2026   13:52:07
Amperometric i-t Curve
File: x
Data Source:  Experiment
             Instrument Model:  CHI627E
Header: 
Note: 

Init E (V) = 1
Sample Interval (s) = 0.1

             Results:

Time/s, i1/A, i2/A

",
        );
        for k in 0..100 {
            let t = k as f64 * 0.1;
            let i1 = if k < 30 { 1.05e-2 } else { 1e-3 + k as f64 * 1e-5 };
            let i2 = if (40..70).contains(&k) { -1.05e-2 } else { 1e-3 };
            text.push_str(&format!("{t:.3e}, {i1:.3e}, {i2:.3e}
"));
        }
        let w = parse_chi(&text, "it", &ChiOptions::default()).unwrap().warnings;
        let (a, b) = (w.iter().find(|m| m.starts_with("i1:")), w.iter().find(|m| m.starts_with("i2: negative")));
        assert!(a.is_some_and(|m| m.contains("positive") && m.contains("30 consecutive")), "{w:?}");
        assert!(b.is_some_and(|m| m.contains("30 consecutive")), "{w:?}");
    }

    #[test]
    fn header_and_note_text_of_a_text_export_are_kept() {
        let text = "Oct. 28, 2026   17:45:56
Cyclic Voltammetry
File: x
Data Source:  Experiment
                    Instrument Model:  CHI627E
Header: CV
Note: after cycling

Init E (V) = 1.2

                    Potential/V, Current/A

1.200, 1.0e-3
1.199, 2.0e-3
";
        let imp = parse_chi(text, "x", &ChiOptions::default()).unwrap();
        let names: Vec<(&str, &str)> = imp.meta.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        assert_eq!(names[2..5], [("Instrument Model", "CHI627E"), ("Header", "CV"), ("Note", "after cycling")]);
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

    /// A number as CHI prints it: 4 significant digits, `e+3` style exponent.
    fn chi_e(x: f64) -> String {
        let s = format!("{x:.3e}");
        match s.split_once('e') {
            Some((m, e)) if !e.starts_with('-') => format!("{m}e+{e}"),
            _ => s,
        }
    }

    /// A chronopotentiometry file in the layout of a CHI627E export, sampled
    /// every 0.2 s, with the given potentials.
    fn cp_text(potential: &[f64], with_currents: bool) -> String {
        let mut s = String::from(
            "Oct. 2, 2026   10:00:00\nChronopotentiometry\nFile: run\nData Source:  Experiment\n\
             Instrument Model:  CHI627E\nHeader: \nNote: \n\n",
        );
        if with_currents {
            s.push_str("Cathodic Current (A) = 0.002\nAnodic Current (A) = 0.001\n");
        }
        s.push_str(
            "Init P/N = N\nData Storage Interval (s) = 0.2\nHigh E Limit (V) = 1\nLow E Limit (V) = 0.4\n\
             Segment = 4\n\nTime/sec, Potential/V\n\n",
        );
        for (k, e) in potential.iter().enumerate() {
            s.push_str(&format!("{}, {}\n", chi_e((k + 1) as f64 * 0.2), chi_e(*e)));
        }
        s
    }

    #[test]
    fn printed_step_reads_the_last_digit() {
        assert_eq!(printed_step("2.023e+3"), 1.0);
        assert_eq!(printed_step("742.6"), 0.1f64.powi(1));
        assert_eq!(printed_step("1200"), 1.0);
        assert!((printed_step("5.000e-1") - 1e-4).abs() < 1e-18);
        assert_eq!(chi_e(2023.4), "2.023e+3");
        assert_eq!(chi_e(0.2), "2.000e-1");
        // Absurd exponents neither overflow nor panic.
        assert_eq!(printed_step("1.5e-2147483648"), 0.0);
        assert!(printed_step("1e99999999999999999999").is_infinite());
        let text = "Oct. 2, 2026\nAmperometric i-t Curve\n\nTime/sec, Current/A\n\n1.5e-2147483648, 1.0e-3\n\
                    2.0e-1, 1.0e-3\nnot a number at all\n4.0e-1, 1.0e-3\n";
        let imp = parse_chi(text, "odd", &ChiOptions::default()).unwrap();
        assert_eq!(imp.workbook.sheets[0].row_count(), 3);
    }

    #[test]
    fn results_block_lines_carry_their_segment() {
        let e = path(&[1.2, 0.2, 1.2], &[50, 50]);
        let text = cv_text("Cyclic Voltammetry", &e, |_, v| v * 1e-4, ",").replace(
            "Sensitivity (A/V) = 1e-3\n",
            "Sensitivity (A/V) = 1e-3\n\nResults:\n\nChannel 1:\nSegment 1:\nEp = 0.812V\nip = -6.5e-4A\n\n\
             Segment 2:\nEp = 0.731V\nip = 8.1e-4A\n",
        );
        let imp = parse_chi(&text, "cv", &ChiOptions::default()).unwrap();
        let tail: Vec<(&str, &str)> =
            imp.meta.iter().rev().take(5).rev().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        assert_eq!(
            tail,
            [
                ("Sensitivity (A/V)", "1e-3"),
                ("Segment 1: Ep", "0.812V"),
                ("Segment 1: ip", "-6.5e-4A"),
                ("Segment 2: Ep", "0.731V"),
                ("Segment 2: ip", "8.1e-4A")
            ]
        );
        assert_eq!(imp.scans, 1);
        assert_eq!(imp.technique, "Cyclic Voltammetry");
        // With several channels, the channel is named too.
        let two = text.replace("Segment 2:\n", "Channel 2:\nSegment 1:\n");
        let imp = parse_chi(&two, "cv", &ChiOptions::default()).unwrap();
        assert!(imp.meta.iter().any(|(k, _)| k == "Channel 2, Segment 1: Ep"), "{:?}", imp.meta);
        assert!(imp.meta.iter().any(|(k, _)| k == "Channel 1, Segment 1: Ep"), "{:?}", imp.meta);
    }

    #[test]
    fn rounded_times_are_recomputed_from_the_interval() {
        // 6000 samples at 0.2 s: above 1000 s the file prints whole seconds.
        let e: Vec<f64> = (0..6000).map(|k| 0.9 - k as f64 * 1e-5).collect();
        let text = cp_text(&e, false);
        assert!(text.contains("\n1.000e+3, ") && text.contains("\n1.200e+3, "));
        let imp = parse_chi(&text, "cp", &ChiOptions::default()).unwrap();
        let t = imp.workbook.sheets[0].columns[0].numeric().unwrap();
        assert_eq!(t.len(), 6000);
        assert!(t.iter().enumerate().all(|(k, &v)| v == (k + 1) as f64 / 5.0), "times are exact multiples of 0.2 s");
        assert_eq!(imp.warnings.len(), 1, "{:?}", imp.warnings);
        assert!(imp.warnings[0].contains("Data Storage Interval (s) = 0.2"), "{}", imp.warnings[0]);
        assert!(imp.warnings[0].contains("steps of up to 1 s"), "{}", imp.warnings[0]);

        let kept = parse_chi(&text, "cp", &ChiOptions { exact_time: false, ..ChiOptions::default() }).unwrap();
        let t = kept.workbook.sheets[0].columns[0].numeric().unwrap();
        assert_eq!((t[5000], t[5001]), (1000.0, 1000.0), "the file's rounded times are kept when asked");
        assert!(kept.warnings.is_empty());
    }

    #[test]
    fn times_that_do_not_fit_the_interval_are_kept() {
        let e: Vec<f64> = (0..100).map(|k| 0.9 - k as f64 * 1e-3).collect();
        let text = cp_text(&e, false).replace("\n2.000e+0, ", "\n2.500e+0, ");
        let imp = parse_chi(&text, "cp", &ChiOptions::default()).unwrap();
        let t = imp.workbook.sheets[0].columns[0].numeric().unwrap();
        assert_eq!(t[9], 2.5, "a time off the sampling grid leaves the column as the file has it");
        assert!(imp.warnings.is_empty(), "{:?}", imp.warnings);
        // A file whose times are already exact is not touched and gets no note.
        let exact = cp_text(&e, false);
        assert!(parse_chi(&exact, "cp", &ChiOptions::default()).unwrap().warnings.is_empty());
    }

    #[test]
    fn charge_of_an_it_file_uses_the_recomputed_times() {
        // Constant 1 mA for 2000 s sampled every 0.1 s: Q = 2 C exactly. The file's rounded times
        // would still integrate to 2 C overall, so check a running value inside the rounded part.
        let mut text = String::from(
            "Aug. 1, 2026\nAmperometric i-t Curve\nInit E (V) = 0.5\nSample Interval (s) = 0.1\n\nTime/sec, Current/A\n\n",
        );
        for k in 1..=20000 {
            text.push_str(&format!("{}, 1.000e-3\n", chi_e(k as f64 * 0.1)));
        }
        let imp = parse_chi(&text, "it", &ChiOptions::default()).unwrap();
        assert!(imp.warnings.iter().any(|w| w.contains("Sample Interval (s) = 0.1")), "{:?}", imp.warnings);
        let s = &imp.workbook.sheets[0];
        let q = s.columns.iter().find(|c| c.long_name == "Charge").unwrap().numeric().unwrap();
        // Row 15004 is t = 1500.5 s; the charge since the first row (t = 0.1 s) is 1e-3 * 1500.4 C.
        assert!((q[15004] - 1e-3 * 1500.4).abs() < 1e-12, "{}", q[15004]);
    }

    /// Fall from 0.85 V to the 0.4 V limit, jump, rise to 1.0 V, jump, fall to 0.4 V, jump, and a
    /// partial rise (the run was stopped): four segments, with a small wiggle on every row.
    fn gcd_trace() -> (Vec<f64>, Vec<usize>) {
        let mut e = Vec::new();
        let mut ends = Vec::new();
        let legs: [(f64, f64, usize); 4] = [(0.85, 0.4, 3000), (0.55, 1.0, 1500), (0.9, 0.4, 2000), (0.5, 0.8, 1000)];
        for (a, b, n) in legs {
            for k in 0..n {
                let wiggle = 0.0005 * ((e.len() % 7) as f64 - 3.0) / 3.0;
                e.push(a + (b - a) * k as f64 / (n - 1) as f64 + wiggle);
            }
            ends.push(e.len());
        }
        (e, ends)
    }

    /// Header lines as `(name, value)` pairs.
    fn meta(lines: &[(&str, &str)]) -> Vec<(String, String)> {
        lines.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    fn times(n: usize) -> Vec<f64> {
        (1..=n).map(|k| k as f64 / 5.0).collect()
    }

    const LIMITS: [(&str, &str); 3] = [("Init P/N", "N"), ("High E Limit (V)", "1"), ("Low E Limit (V)", "0.4")];

    #[test]
    fn segments_end_where_the_potential_turns_at_a_limit() {
        let (e, ends) = gcd_trace();
        let s = chrono_segments(&times(e.len()), &e, &meta(&LIMITS));
        assert_eq!(s.len(), 4, "{s:?}");
        // Each segment ends at its extreme row, within the wiggle of the last few rows.
        for (seg, &end) in s.iter().zip(&ends) {
            assert!(seg.rows.end.abs_diff(end) <= 7, "{seg:?} vs {end}");
        }
        assert_eq!(s.iter().map(|s| s.cathodic).collect::<Vec<_>>(), [true, false, true, false]);
        assert_eq!(s[0].rows.start, 0);
        assert_eq!(s[3].rows.end, e.len());
        assert!(s.windows(2).all(|w| w[0].rows.end == w[1].rows.start), "no row is lost or repeated");
        // A monotone trace with noise is one segment; a flat one too, named by Init P/N.
        let ramp: Vec<f64> = (0..500).map(|k| k as f64 * 1e-3 + 1e-4 * (k % 3) as f64).collect();
        let one = chrono_segments(&times(500), &ramp, &meta(&LIMITS));
        assert_eq!(one.iter().map(|s| (s.rows.clone(), s.cathodic)).collect::<Vec<_>>(), [(0..500, true)]);
        assert_eq!(chrono_segments(&times(10), &[0.5; 10], &[]).len(), 1);
        assert!(chrono_segments(&[], &[], &[]).is_empty());
    }

    #[test]
    fn a_spike_that_relaxes_away_from_the_limits_is_not_a_segment() {
        // Falling towards 0.4 V with a 60 mV dip-and-recovery at 0.7 V (more than 5% of the range),
        // then the real switch at the limit and a rise to 1.0 V.
        let mut e: Vec<f64> = (0..1000).map(|k| 0.85 - 0.15 * k as f64 / 999.0).collect();
        e.extend((0..20).map(|k| 0.70 - 0.06 * (1.0 - (k as f64 - 10.0).abs() / 10.0)));
        e.extend((0..1000).map(|k| 0.70 - 0.30 * k as f64 / 999.0));
        e.extend((0..1000).map(|k| 0.55 + 0.45 * k as f64 / 999.0));
        let s = chrono_segments(&times(e.len()), &e, &meta(&LIMITS));
        assert_eq!(
            s.iter().map(|s| (s.rows.clone(), s.cathodic)).collect::<Vec<_>>(),
            [(0..2020, true), (2020..3020, false)]
        );
        // Without limits in the header every turn counts, so the dip makes extra segments.
        assert!(chrono_segments(&times(e.len()), &e, &meta(&[("Init P/N", "N")])).len() > 2);
    }

    #[test]
    fn a_time_switched_run_is_cut_at_the_switching_times() {
        // 500 s per segment at 0.2 s: each half rises (or falls) for 100 s, then drifts back 2 mV.
        let mut e = Vec::new();
        for half in 0..4 {
            let (a, b) = if half % 2 == 0 { (0.75, 0.95) } else { (0.95, 0.75) };
            for k in 0..2500 {
                let ramp = (k as f64 / 499.0).min(1.0);
                let drift = if k >= 500 { 0.002 * (k - 500) as f64 / 1999.0 } else { 0.0 };
                e.push(a + (b - a) * ramp - drift * (b - a).signum());
            }
        }
        let header = meta(&[
            ("Init P/N", "P"),
            ("Current Switching Polarity", "Time"),
            ("Cathodic Time (s)", "500"),
            ("Anodic Time (s)", "500"),
            ("High E Limit (V)", "1"),
            ("Low E Limit (V)", "0.7"),
        ]);
        let s = chrono_segments(&times(e.len()), &e, &header);
        let got: Vec<_> = s.iter().map(|s| (s.rows.clone(), s.cathodic, s.start_time)).collect();
        assert_eq!(
            got,
            [
                (0..2500, false, Some(0.0)),
                (2500..5000, true, Some(500.0)),
                (5000..7500, false, Some(1000.0)),
                (7500..10000, true, Some(1500.0))
            ]
        );
        // The capacity of every segment ends at 1 mA * 500 s.
        let mut text = cp_text(&e, false);
        text = text.replace(
            "Init P/N = N\n",
            "Cathodic Current (A) = 0.001\nAnodic Current (A) = 0.001\nInit P/N = P\nCurrent Switching Polarity = Time\n\
             Cathodic Time (s) = 500\nAnodic Time (s) = 500\n",
        );
        let imp = parse_chi(&text, "t", &ChiOptions::default()).unwrap();
        let c = &imp.workbook.sheets[1].columns;
        for k in 0..4 {
            let q = c[3 * k + 2].numeric().unwrap();
            assert!((q[q.len() - 1] - 0.001 * 500.0 / 3.6).abs() < 1e-12, "segment {k}: {}", q[q.len() - 1]);
        }
    }

    #[test]
    fn capacity_uses_the_magnitude_and_allows_zero() {
        let (e, _) = gcd_trace();
        let text = cp_text(&e, true)
            .replace("Anodic Current (A) = 0.001", "Anodic Current (A) = 0")
            .replace("Cathodic Current (A) = 0.002", "Cathodic Current (A) = -0.002");
        let imp = parse_chi(&text, "gcd", &ChiOptions::default()).unwrap();
        let c = &imp.workbook.sheets[1].columns;
        assert_eq!(c.len(), 12, "a Capacity column for every segment");
        let t1 = c[0].numeric().unwrap();
        let q1 = c[2].numeric().unwrap();
        assert!((q1[10] - 0.002 * t1[10] / 3.6).abs() < 1e-15, "magnitude of a negative current");
        assert!(c[5].numeric().unwrap().iter().all(|&q| q == 0.0), "zero anodic current gives zero capacity");
    }

    #[test]
    fn labels_come_from_the_direction_not_the_end_rows() {
        let (mut e, _) = gcd_trace();
        e.push(f64::NAN);
        e[0] = f64::NAN;
        let s = chrono_segments(&times(e.len()), &e, &meta(&LIMITS));
        assert_eq!(s.iter().map(|s| s.cathodic).collect::<Vec<_>>(), [true, false, true, false]);
    }

    #[test]
    fn a_short_first_stretch_joins_the_next_segment() {
        // One stray row at 0.40 V, then a fall from 0.85 V: the 1-row stretch is not dropped.
        let (mut e, _) = gcd_trace();
        e.insert(0, 0.40);
        let s = chrono_segments(&times(e.len()), &e, &meta(&LIMITS));
        assert_eq!(s[0].rows.start, 0, "{s:?}");
        assert_eq!(s.iter().map(|s| s.rows.len()).sum::<usize>(), e.len());
    }

    #[test]
    fn chronopotentiometry_gets_a_segment_sheet_with_capacity() {
        let (e, _) = gcd_trace();
        let imp = parse_chi(&cp_text(&e, true), "gcd", &ChiOptions::default()).unwrap();
        assert_eq!(imp.segments, 4);
        assert_eq!(imp.scans, 0);
        let names: Vec<&str> = imp.workbook.sheets.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Data", "Segments"]);
        assert_eq!(imp.workbook.sheets[0].columns.len(), 2, "the whole run stays in the Data sheet");
        let seg = &imp.workbook.sheets[1];
        assert_eq!(seg.columns.len(), 12);
        let c = &seg.columns;
        assert_eq!(
            (c[0].designation, c[0].long_name.as_str(), c[0].units.as_str(), c[0].comments.as_str()),
            (Designation::X, "Time", "s", "Segment 1 (cathodic)")
        );
        assert_eq!((c[2].long_name.as_str(), c[2].units.as_str()), ("Capacity", "mAh"));
        assert_eq!(c[3].comments, "Segment 2 (anodic)");
        // Segment 1 starts at t = 0: its first row is 0.2 s and its capacity is 2 mA * t.
        let t1 = c[0].numeric().unwrap();
        let q1 = c[2].numeric().unwrap();
        assert_eq!(t1[0], 0.2);
        let last = t1.len() - 1;
        assert!((q1[last] - 0.002 * t1[last] / 3.6).abs() < 1e-15);
        // Segment 2 restarts its clock and uses the anodic current.
        let t2 = c[3].numeric().unwrap();
        let q2 = c[5].numeric().unwrap();
        assert!((t2[0] - 0.2).abs() < 1e-9, "{}", t2[0]);
        assert!((q2[9] - 0.001 * t2[9] / 3.6).abs() < 1e-15);
        let rows: usize = (0..4).map(|k| c[3 * k].len()).sum();
        assert_eq!(rows, e.len());

        // Without the currents in the header there is no capacity column.
        let imp = parse_chi(&cp_text(&e, false), "gcd", &ChiOptions::default()).unwrap();
        assert_eq!(imp.workbook.sheets[1].columns.len(), 8);
        // The option turns the sheet off.
        let opts = ChiOptions { split_segments: false, ..ChiOptions::default() };
        let imp = parse_chi(&cp_text(&e, true), "gcd", &opts).unwrap();
        assert_eq!((imp.workbook.sheets.len(), imp.segments), (1, 0));
    }

    #[test]
    fn detection() {
        assert!(looks_like_chi("a\nb\nPotential/V, Current/A\n1,2\n"));
        assert!(looks_like_chi("time/s\tcurrent/a\n1\t2\n"));
        assert!(!looks_like_chi("x,y\n1,2\n"));
    }
}
