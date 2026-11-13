//! CH Instruments (CHI) binary files (`.bin`).
//!
//! See `docs/spec/chi-binary-format.md`. A file is ten length-prefixed
//! strings, a fixed block of 0x650 bytes of 4-byte words (point count,
//! channel count, date and time, experiment parameters), and then one array
//! of little-endian `f32` per channel. The potential of a cyclic voltammetry
//! run and the time of a time-based run are not stored; they are rebuilt from
//! the parameters. The result is handed to the same assembly code as the text
//! importer, so a file gives the same workbook whichever way it was saved.

use crate::chi::{Block, ChiImport, ChiOptions, HeaderCol, assemble};
use crate::error::IoError;

/// The four bytes every file starts with.
pub const MAGIC: [u8; 4] = [0x80, 0xF2, 0x1D, 0x00];

/// Number of length-prefixed strings after the magic number.
const STRING_COUNT: usize = 10;
/// Size of the fixed block after the strings.
const BLOCK_LEN: usize = 0x650;

// Offsets of the fields, counted from the end of the last string.
const POINTS: usize = 0x21c;
const DATE: usize = 0x228;
const MODEL: usize = 0x248;
const CHANNELS: usize = 0x2ec;
const E_INIT: usize = 0x3f8;
/// High E of a CV, E2 of an i-t run, High E limit of a CP run.
const E_HIGH: usize = 0x400;
const E_LOW: usize = 0x404;
const SCAN_RATE: usize = 0x408;
const POLARITY: usize = 0x410;
const SEGMENTS: usize = 0x414;
const INTERVAL: usize = 0x418;
const SENSITIVITY: usize = 0x420;
const QUIET_TIME: usize = 0x424;
const RUN_TIME: usize = 0x430;
const CP_CATHODIC_CURRENT: usize = 0x47c;
const CP_ANODIC_CURRENT: usize = 0x4ac;
const SENSITIVITY_2: usize = 0x4b4;
const CP_CATHODIC_TIME: usize = 0x4b8;
const CP_ANODIC_TIME: usize = 0x4bc;
/// Float 1.0 in the files of the newer programs, 0.0 in the files of the older ones that ship in
/// the `Test Data` folder of the CHI1040C program. Decides how several channels are stored.
const LAYOUT_FLAG: usize = 0x630;

/// Significant digits of a current in the text export; pinned-current detection compares at this precision.
const TEXT_DIGITS: usize = 4;

/// Most channels a file may claim before it is considered damaged.
const MAX_CHANNELS: usize = 16;

/// True when the bytes start like a CH Instruments binary file.
pub fn looks_like_chi_bin(bytes: &[u8]) -> bool {
    bytes.starts_with(&MAGIC)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Technique {
    CyclicVoltammetry,
    Amperometry,
    Chronopotentiometry,
}

struct Header<'a> {
    bytes: &'a [u8],
    /// Offset of the end of the last string.
    body: usize,
    stem: &'a str,
}

impl Header<'_> {
    fn bad(&self, detail: impl Into<String>) -> IoError {
        IoError::ChiBinary { source_name: self.stem.to_string(), detail: detail.into() }
    }

    fn u32_at(&self, rel: usize) -> u32 {
        let p = self.body + rel;
        u32::from_le_bytes([self.bytes[p], self.bytes[p + 1], self.bytes[p + 2], self.bytes[p + 3]])
    }

    fn f32_at(&self, rel: usize) -> f32 {
        f32::from_bits(self.u32_at(rel))
    }

    /// A parameter as the decimal number the user typed: `0.02` stays `0.02` instead of becoming
    /// 0.019999999552965164. The instrument computes some limits in single precision (0.995 is
    /// stored one step off the nearest `f32`), so a parameter is rounded to 6 significant digits.
    fn dec(&self, rel: usize) -> f64 {
        let v = f64::from(self.f32_at(rel));
        if v == 0.0 || !v.is_finite() {
            return v;
        }
        format!("{:.5e}", v).parse().unwrap_or(v)
    }

    /// A parameter as text, for the header list.
    fn text(&self, rel: usize) -> String {
        format!("{}", self.dec(rel))
    }
}

/// The shortest decimal that reads back as the same `f32`, as an `f64`. Used for the stored
/// values too, so that `0.0105` is shown as 0.0105 and not as 0.010499999858438969.
fn decimal(v: f32) -> f64 {
    if !v.is_finite() {
        return f64::from(v);
    }
    format!("{v}").parse().unwrap_or(f64::from(v))
}

fn latin1(b: &[u8]) -> String {
    b.iter().map(|&c| char::from(c)).collect()
}

/// Reads the ten strings; returns them and the offset after the last one.
fn read_strings(bytes: &[u8], stem: &str) -> Result<(Vec<String>, usize), IoError> {
    let bad = |detail: String| IoError::ChiBinary { source_name: stem.to_string(), detail };
    let mut pos = MAGIC.len();
    let mut strings = Vec::with_capacity(STRING_COUNT);
    for k in 0..STRING_COUNT {
        let Some(len_bytes) = bytes.get(pos..pos + 4) else {
            return Err(bad(format!("the file ends inside the text header (string {} of {STRING_COUNT})", k + 1)));
        };
        let len = u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;
        let Some(text) = bytes.get(pos + 4..pos + 4 + len).filter(|_| len <= 255) else {
            return Err(bad(format!("string {} of the text header has an impossible length ({len})", k + 1)));
        };
        strings.push(latin1(text));
        pos += 4 + len;
    }
    Ok((strings, pos))
}

fn technique_of(code: &str, name: &str) -> Option<Technique> {
    match code.trim() {
        "CV" => return Some(Technique::CyclicVoltammetry),
        "i-t" => return Some(Technique::Amperometry),
        "CP" => return Some(Technique::Chronopotentiometry),
        _ => {}
    }
    match name.trim().to_ascii_lowercase().as_str() {
        "cyclic voltammetry" => Some(Technique::CyclicVoltammetry),
        "amperometric i-t curve" => Some(Technique::Amperometry),
        "chronopotentiometry" => Some(Technique::Chronopotentiometry),
        _ => None,
    }
}

const MONTHS: [&str; 12] =
    ["Jan.", "Feb.", "March", "Apr.", "May", "June", "July", "Aug.", "Sept.", "Oct.", "Nov.", "Dec."];

/// The date line in the style of the text exports: `Sept. 3, 2024   09:05:07`.
fn date_line(h: &Header) -> Option<String> {
    let [year, month, day, hour, minute, second] = std::array::from_fn(|i| h.u32_at(DATE + 4 * i));
    let ok = (1990..=2100).contains(&year)
        && (1..=12).contains(&month)
        && (1..=31).contains(&day)
        && hour < 24
        && minute < 60
        && second < 61;
    ok.then(|| format!("{} {day}, {year}   {hour:02}:{minute:02}:{second:02}", MONTHS[month as usize - 1]))
}

/// The potential of a cyclic voltammetry run, point by point.
///
/// Point `k` (counted from 0, so the first row is Init E itself, as in the
/// text export) is the starting potential moved by `k` steps
/// in the initial direction, folded back and forth between the two limits
/// like a triangle wave. A limit is reflected, not clamped: when the step
/// would pass it, the overshoot is mirrored, so the potential can sit at the
/// extreme twice in a row when the limits are not a whole number of steps
/// apart. Values are rounded to 1e-9 V so that equal potentials compare equal.
pub(crate) fn cv_potential(init: f64, high: f64, low: f64, step: f64, positive: bool, n: usize) -> Vec<f64> {
    let span = high - low;
    let period = 2.0 * span;
    let sign = if positive { 1.0 } else { -1.0 };
    (0..n)
        .map(|k| {
            let x = init + sign * k as f64 * step;
            let mut u = (x - low).rem_euclid(period);
            if u > span {
                u = period - u;
            }
            ((low + u) * 1e9).round() / 1e9
        })
        .collect()
}

/// Time of point `k` (counted from 1): `k` sample intervals, free of rounding noise.
fn grid(k: usize, step: f64) -> f64 {
    (k as f64 * step * 1e9).round() / 1e9
}

/// Splits the data block into channels. In the files of the newer programs the values of the
/// channels alternate point by point (`i1, i2, i1, i2, ...`). In the older files (layout flag 0.0)
/// the first channel comes alone as one array and the others alternate after it, which for the
/// three-channel sample is `i1 ... i1, i2, i3, i2, i3, ...`. A single channel is just its array.
fn split_channels(data: &[u8], n: usize, channels: usize, old_layout: bool) -> Vec<Vec<f64>> {
    let values: Vec<f64> = data.as_chunks::<4>().0.iter().map(|w| decimal(f32::from_le_bytes(*w))).collect();
    let (alone, rest) = if old_layout && channels > 1 { (1, &values[n..]) } else { (0, &values[..]) };
    let stride = channels - alone;
    let mut out: Vec<Vec<f64>> = Vec::with_capacity(channels);
    if alone == 1 {
        out.push(values[..n].to_vec());
    }
    out.extend((0..stride).map(|c| rest.iter().skip(c).step_by(stride).copied().collect::<Vec<f64>>()));
    out
}

/// Reads a CHI binary file. `file_stem` becomes the workbook long name and is
/// used in messages.
pub fn parse_chi_bin(bytes: &[u8], file_stem: &str, opts: &ChiOptions) -> Result<ChiImport, IoError> {
    let bad = |detail: String| IoError::ChiBinary { source_name: file_stem.to_string(), detail };
    if !looks_like_chi_bin(bytes) {
        return Err(bad("it does not start with the marker bytes 80 F2 1D 00".to_string()));
    }
    let (strings, body) = read_strings(bytes, file_stem)?;
    let (code, name) = (strings[0].as_str(), strings[1].as_str());
    let Some(technique) = technique_of(code, name) else {
        return Err(bad(format!(
            "its technique ('{}', code '{}') is not supported yet; only Cyclic Voltammetry, Amperometric i-t \
             Curve and Chronopotentiometry are. Save the run as a text file in the CH Instruments program \
             and import that",
            name.trim(),
            code.trim()
        )));
    };
    if bytes.len() < body + BLOCK_LEN {
        return Err(bad(format!(
            "it is {} bytes long, but the parameter block alone needs {}",
            bytes.len(),
            body + BLOCK_LEN
        )));
    }
    let h = Header { bytes, body, stem: file_stem };

    let channels = h.u32_at(CHANNELS) as usize;
    if channels == 0 || channels > MAX_CHANNELS {
        return Err(h.bad(format!("it claims {channels} data channels")));
    }
    let claimed = h.u32_at(POINTS) as usize;
    let data = &bytes[body + BLOCK_LEN..];
    let present = data.len() / (4 * channels);
    let mut warnings = Vec::new();
    if claimed == 0 || (present == 0 && channels > 1) {
        return Err(IoError::NoData {
            source_name: file_stem.to_string(),
            detail: "the file holds no data points".to_string(),
        });
    }
    // The data are `claimed` values for each of the channels (see `split_channels`). A cut-short
    // file can only be read when there is a single channel.
    let needed = claimed.saturating_mul(4 * channels);
    let n = if data.len() >= needed {
        claimed
    } else if channels == 1 && present > 0 {
        warnings.push(format!(
            "the header announces {claimed} points, but the file holds only {present}; the run was probably cut short or the file is damaged"
        ));
        present
    } else {
        return Err(h.bad(format!(
            "the header announces {claimed} points in each of {channels} channels, but the file ends after {} bytes of data",
            data.len()
        )));
    };
    let extra = data.len() - n * 4 * channels;
    if extra > 0 {
        warnings.push(format!("{extra} bytes after the data were ignored"));
    }
    let channel_data = split_channels(&data[..n * 4 * channels], n, channels, h.f32_at(LAYOUT_FLAG) == 0.0);
    let not_finite: usize = channel_data.iter().flatten().filter(|v| !v.is_finite()).count();
    if not_finite > 0 {
        warnings.push(format!("{not_finite} stored values are not finite numbers"));
    }

    let technique_name = match technique {
        Technique::CyclicVoltammetry => "Cyclic Voltammetry",
        Technique::Amperometry => "Amperometric i-t Curve",
        Technique::Chronopotentiometry => "Chronopotentiometry",
    };
    let mut meta = vec![("Technique".to_string(), technique_name.to_string())];
    if let Some(date) = date_line(&h) {
        meta.push(("Date".to_string(), date));
    }
    let model = h.u32_at(MODEL);
    if (1..10_000).contains(&model) {
        meta.push(("Instrument Model".to_string(), format!("CHI{model}")));
    }
    let current_cols = |names_from: usize| -> Vec<HeaderCol> {
        (names_from..channels)
            .map(|c| HeaderCol::new(&if channels == 1 { "Current".to_string() } else { format!("i{}", c + 1) }, "A"))
            .collect()
    };

    let block = match technique {
        Technique::CyclicVoltammetry => {
            let (init, high, low, step) = (h.dec(E_INIT), h.dec(E_HIGH), h.dec(E_LOW), h.dec(INTERVAL));
            if !(high > low && step > 0.0 && step.is_finite()) {
                return Err(h.bad("the stored High E, Low E and Sample Interval do not describe a potential sweep"));
            }
            let eps = step * 1e-6;
            if init < low - eps || init > high + eps {
                return Err(h.bad("the stored Init E lies outside the range between Low E and High E"));
            }
            let positive = h.f32_at(POLARITY) > 0.5;
            meta.extend([
                ("Init E (V)".to_string(), h.text(E_INIT)),
                ("High E (V)".to_string(), h.text(E_HIGH)),
                ("Low E (V)".to_string(), h.text(E_LOW)),
                ("Init P/N".to_string(), if positive { "P" } else { "N" }.to_string()),
                ("Scan Rate (V/s)".to_string(), h.text(SCAN_RATE)),
                ("Segment".to_string(), h.text(SEGMENTS)),
                ("Sample Interval (V)".to_string(), h.text(INTERVAL)),
                ("Quiet Time (sec)".to_string(), h.text(QUIET_TIME)),
                ("Sensitivity (A/V)".to_string(), h.text(SENSITIVITY)),
            ]);
            let mut header = vec![HeaderCol::new("Potential", "V")];
            header.extend(current_cols(0));
            let mut data = vec![cv_potential(init, high, low, step, positive, n)];
            data.extend(channel_data);
            Block { header, data, x_step: Vec::new() }
        }
        Technique::Amperometry => {
            let step = h.dec(INTERVAL);
            if !(step > 0.0 && step.is_finite()) {
                return Err(h.bad("the stored Sample Interval is not a positive number"));
            }
            meta.extend([
                ("Init E (V)".to_string(), h.text(E_INIT)),
                ("Sample Interval (s)".to_string(), h.text(INTERVAL)),
                ("Run Time (sec)".to_string(), h.text(RUN_TIME)),
                ("Quiet Time (sec)".to_string(), h.text(QUIET_TIME)),
                ("Sensitivity (A/V)".to_string(), h.text(SENSITIVITY)),
            ]);
            if channels >= 2 {
                meta.push(("E2".to_string(), h.text(E_HIGH)));
                meta.push(("Sens2".to_string(), h.text(SENSITIVITY_2)));
            }
            let run = f64::from(h.f32_at(RUN_TIME));
            if run > 0.0 && (n as f64) * step < 0.99 * run {
                warnings.push(format!(
                    "the run was set to {} s but the file covers only {} s",
                    h.text(RUN_TIME),
                    ((n as f64) * step * 1e6).round() / 1e6
                ));
            }
            let mut header = vec![HeaderCol::new("Time", "s")];
            header.extend(current_cols(0));
            let mut data = vec![(1..=n).map(|k| grid(k, step)).collect::<Vec<_>>()];
            data.extend(channel_data);
            Block { header, data, x_step: Vec::new() }
        }
        Technique::Chronopotentiometry => {
            let step = h.dec(INTERVAL);
            if !(step > 0.0 && step.is_finite()) {
                return Err(h.bad("the stored Data Storage Interval is not a positive number"));
            }
            meta.extend([
                ("Cathodic Current (A)".to_string(), h.text(CP_CATHODIC_CURRENT)),
                ("Anodic Current (A)".to_string(), h.text(CP_ANODIC_CURRENT)),
                ("High E Limit (V)".to_string(), h.text(E_HIGH)),
                ("Low E Limit (V)".to_string(), h.text(E_LOW)),
                ("Cathodic Time (sec)".to_string(), h.text(CP_CATHODIC_TIME)),
                ("Anodic Time (sec)".to_string(), h.text(CP_ANODIC_TIME)),
                (
                    "Initial Polarity".to_string(),
                    if h.f32_at(POLARITY) > 0.5 { "Anodic" } else { "Cathodic" }.to_string(),
                ),
                ("Segment".to_string(), h.text(SEGMENTS)),
                ("Data Storage Interval (sec)".to_string(), h.text(INTERVAL)),
                ("Quiet Time (sec)".to_string(), h.text(QUIET_TIME)),
            ]);
            if channels > 1 {
                warnings.push(format!(
                    "only the potential channel was imported; {} more channels were skipped",
                    channels - 1
                ));
            }
            let (high, low) = (h.dec(E_HIGH), h.dec(E_LOW));
            let potential = channel_data.into_iter().next().unwrap_or_default();
            let margin = 1e-3 * (high - low).abs().max(1e-3);
            if potential.iter().any(|&v| v.is_finite() && (v < low - margin || v > high + margin)) {
                warnings.push("the potential leaves the range between the stored Low E and High E limits".to_string());
            }
            Block {
                header: vec![HeaderCol::new("Time", "s"), HeaderCol::new("Potential", "V")],
                data: vec![(1..=n).map(|k| grid(k, step)).collect(), potential],
                x_step: Vec::new(),
            }
        }
    };
    meta.push(("Instrument Code".to_string(), strings[STRING_COUNT - 1].trim().to_string()));
    meta.push(("Data Points".to_string(), n.to_string()));
    meta.push(("Channels".to_string(), channels.to_string()));

    let mut imp = assemble(vec![block], technique_name.to_string(), meta, file_stem, opts, Some(TEXT_DIGITS));
    warnings.append(&mut imp.warnings);
    imp.warnings = warnings;
    Ok(imp)
}

#[cfg(test)]
pub(crate) mod tests;
