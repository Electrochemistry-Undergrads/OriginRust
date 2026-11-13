//! Tests of the binary reader. The files are made by [`Builder`], a test-only
//! writer of the same layout, so no instrument data is needed. The last group
//! reads the owner's own files and skips quietly when they are not there.

use super::*;
use crate::chi::split_scans;
use sheaf_core::Designation;
use std::path::{Path, PathBuf};

/// Writes a file in the layout of the spec.
struct Builder {
    code: String,
    name: String,
    channels: usize,
    points: usize,
    words: Vec<(usize, u32)>,
    data: Vec<Vec<f32>>,
    /// The layout of the older programs (flag word 0.0).
    legacy: bool,
    header: String,
}

impl Builder {
    fn new(code: &str, name: &str) -> Self {
        let mut b = Builder {
            code: code.to_string(),
            name: name.to_string(),
            channels: 1,
            points: 0,
            words: Vec::new(),
            data: Vec::new(),
            legacy: false,
            header: String::new(),
        };
        b.f(E_INIT, 0.0).u(MODEL, 627).date([2026, 10, 28, 17, 45, 56]);
        b
    }

    fn u(&mut self, rel: usize, v: u32) -> &mut Self {
        self.words.push((rel, v));
        self
    }

    fn f(&mut self, rel: usize, v: f32) -> &mut Self {
        self.u(rel, v.to_bits())
    }

    /// The layout of the older programs: channel 1 alone, the others alternating after it.
    fn legacy(&mut self) -> &mut Self {
        self.legacy = true;
        self
    }

    fn date(&mut self, d: [u32; 6]) -> &mut Self {
        for (i, v) in d.into_iter().enumerate() {
            self.u(DATE + 4 * i, v);
        }
        self
    }

    /// Sets the channels; every array must have the same length.
    fn channels(&mut self, data: Vec<Vec<f32>>) -> &mut Self {
        self.channels = data.len();
        self.points = data[0].len();
        self.data = data;
        self
    }

    fn build(&self) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        let mut strings = vec![self.code.as_str(), self.name.as_str(), self.header.as_str()];
        strings.extend(["", "", "", "", "", ""]);
        strings.push("A4440");
        assert_eq!(strings.len(), STRING_COUNT);
        for s in strings {
            out.extend((s.len() as u32).to_le_bytes());
            out.extend(s.as_bytes());
        }
        let body = out.len();
        out.resize(body + BLOCK_LEN, 0);
        let mut put = |rel: usize, v: u32| out[body + rel..body + rel + 4].copy_from_slice(&v.to_le_bytes());
        put(POINTS, self.points as u32);
        put(POINTS + 8, self.points as u32);
        put(CHANNELS, self.channels as u32);
        for &(rel, v) in &self.words {
            put(rel, v);
        }
        if !self.legacy {
            put_f32(&mut out, body + LAYOUT_FLAG, 1.0);
        }
        let alone = usize::from(self.legacy && self.channels > 1);
        for ch in &self.data[..alone] {
            out.extend(ch.iter().flat_map(|v| v.to_le_bytes()));
        }
        for k in 0..self.points {
            for ch in &self.data[alone..] {
                out.extend(ch[k].to_le_bytes());
            }
        }
        out
    }
}

fn put_f32(out: &mut [u8], at: usize, v: f32) {
    out[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

/// A cyclic voltammetry file; the current of point `i` is `current(i)`.
pub(crate) fn cv_file(
    init: f32,
    high: f32,
    low: f32,
    step: f32,
    positive: bool,
    n: usize,
    current: impl Fn(usize) -> f32,
) -> Vec<u8> {
    let mut b = Builder::new("CV", "Cyclic Voltammetry");
    b.f(E_INIT, init).f(E_HIGH, high).f(E_LOW, low).f(INTERVAL, step).f(SCAN_RATE, 0.001);
    b.f(POLARITY, if positive { 1.0 } else { 0.0 }).f(SEGMENTS, 4.0).f(SENSITIVITY, 1e-3).f(QUIET_TIME, 2.0);
    b.channels(vec![(0..n).map(current).collect()]);
    b.build()
}

fn import(bytes: &[u8]) -> ChiImport {
    parse_chi_bin(bytes, "run1", &ChiOptions::default()).unwrap()
}

fn meta<'a>(imp: &'a ChiImport, key: &str) -> &'a str {
    &imp.meta.iter().find(|(k, _)| k == key).unwrap_or_else(|| panic!("no '{key}' in {:?}", imp.meta)).1
}

/// The potential of a sweep found by walking it: step, and mirror whatever
/// passes a limit. A different method from the folding formula of the reader.
fn simulated_potential(init: f64, high: f64, low: f64, step: f64, positive: bool, n: usize) -> Vec<f64> {
    let mut e = init;
    let mut dir = if positive { 1.0 } else { -1.0 };
    let mut out = Vec::new();
    for k in 0..n {
        if k > 0 {
            e += dir * step;
            if e > high + 1e-9 {
                e = 2.0 * high - e;
                dir = -1.0;
            } else if e < low - 1e-9 {
                e = 2.0 * low - e;
                dir = 1.0;
            }
        }
        out.push(e);
    }
    out
}

#[test]
fn the_waveform_formula_agrees_with_walking_the_sweep() {
    // (init, high, low, step, positive): limits that are a whole number of steps apart, and limits
    // that are not (which repeats the extreme potential once).
    let cases = [
        (1.2, 1.2, 0.2, 0.001, false),
        (0.4, 1.3, -0.35, 0.02, true),
        (0.0, 0.5, -0.5, 0.005, false),
        (0.25, 0.8, 0.1, 0.01, true),
    ];
    for (init, high, low, step, positive) in cases {
        let n = 5000;
        let a = cv_potential(init, high, low, step, positive, n);
        let b = simulated_potential(init, high, low, step, positive, n);
        let worst = a.iter().zip(&b).map(|(x, y)| (x - y).abs()).fold(0.0, f64::max);
        assert!(worst < 1e-8, "case {init} {high} {low} {step}: worst difference {worst}");
        assert!(a.iter().all(|&e| e >= low - 1e-9 && e <= high + 1e-9), "potential left its limits");
    }
}

#[test]
fn a_cv_potential_starts_at_init_e_and_runs_between_its_limits() {
    let up = cv_potential(0.4, 1.3, -0.35, 0.02, true, 200);
    assert!((up[0] - 0.4).abs() < 1e-9, "the first row is Init E: {}", up[0]);
    assert!((up[45] - 1.3).abs() < 1e-9 && up[46] < up[45], "High E reached at row 45");
    // The next limit is 82.5 steps away, so the extreme is visited twice in a row.
    assert!((up[127] - -0.34).abs() < 1e-9 && (up[128] - -0.34).abs() < 1e-9, "{} {}", up[127], up[128]);
    let down = cv_potential(1.2, 1.2, 0.2, 0.001, false, 4001);
    assert!((down[0] - 1.2).abs() < 1e-9 && (down[1] - 1.199).abs() < 1e-9);
    assert!((down[1000] - 0.2).abs() < 1e-9 && (down[1001] - 0.201).abs() < 1e-9);
    assert!((down[2000] - 1.2).abs() < 1e-9 && (down[2001] - 1.199).abs() < 1e-9);
    assert!((down[4000] - 1.2).abs() < 1e-9);
}

#[test]
fn a_four_segment_cv_gives_two_scans_with_the_text_importers_layout() {
    // 1.2 V down to 0.2 V and back, twice: 4 segments of 1000 points.
    let bytes = cv_file(1.2, 1.2, 0.2, 0.001, false, 4000, |i| (i as f32 * 1e-6) - 2e-3);
    let imp = import(&bytes);
    assert_eq!(imp.technique, "Cyclic Voltammetry");
    assert_eq!(imp.scans, 2);
    let sheet = &imp.workbook.sheets[0];
    assert_eq!(sheet.name, "Scans");
    assert_eq!(sheet.columns.len(), 6, "potential, A and mA for each scan");
    let c = &sheet.columns;
    assert_eq!(c[0].designation, Designation::X);
    assert_eq!((c[0].long_name.as_str(), c[0].units.as_str(), c[0].comments.as_str()), ("Potential", "V", "Scan 1"));
    assert_eq!((c[1].long_name.as_str(), c[1].units.as_str()), ("Current", "A"));
    assert_eq!((c[2].long_name.as_str(), c[2].units.as_str()), ("Current", "mA"));
    assert_eq!(c[2].formula.as_deref(), Some("col(B)*1000"));
    assert_eq!(c[3].comments, "Scan 2");
    // The cut is at the second vertex, the 1.2 V row 2000, which opens scan 2: 2000 + 2000 rows as
    // in the text export of such a run.
    assert_eq!((c[0].len(), c[3].len()), (2000, 2000));
    assert!((c[0].numeric().unwrap()[0] - 1.2).abs() < 1e-9 && (c[3].numeric().unwrap()[0] - 1.2).abs() < 1e-9);
    // The sweep reaches both limits and never leaves them.
    let e: Vec<f64> = c[0].numeric().unwrap().iter().chain(c[3].numeric().unwrap()).copied().collect();
    let (lo, hi) = e.iter().fold((f64::MAX, f64::MIN), |(l, h), &v| (l.min(v), h.max(v)));
    assert!((lo - 0.2).abs() < 1e-9 && (hi - 1.2).abs() < 1e-9, "{lo} {hi}");
    // Cut where the text importer cuts: at every second vertex of the stored potential.
    assert_eq!(split_scans(c[0].numeric().unwrap()).len(), 1, "scan 1 alone has one turning point");
    assert_eq!(imp.warnings, Vec::<String>::new());
}

#[test]
fn the_header_parameters_are_listed_like_the_text_header() {
    let bytes = cv_file(0.4, 1.3, -0.35, 0.02, true, 300, |_| 1e-5);
    let imp = import(&bytes);
    let names: Vec<&str> = imp.meta.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(
        names,
        [
            "Technique",
            "Date",
            "Instrument Model",
            "Init E (V)",
            "High E (V)",
            "Low E (V)",
            "Init P/N",
            "Scan Rate (V/s)",
            "Segment",
            "Sample Interval (V)",
            "Quiet Time (sec)",
            "Sensitivity (A/V)",
            "Instrument Code",
            "Data Points",
            "Channels"
        ]
    );
    assert_eq!(meta(&imp, "Date"), "Oct. 28, 2026   17:45:56");
    assert_eq!(meta(&imp, "Instrument Model"), "CHI627");
    assert_eq!(meta(&imp, "Init E (V)"), "0.4");
    assert_eq!(meta(&imp, "Low E (V)"), "-0.35");
    assert_eq!(meta(&imp, "Init P/N"), "P");
    assert_eq!(meta(&imp, "Sample Interval (V)"), "0.02");
    assert_eq!(meta(&imp, "Sensitivity (A/V)"), "0.001");
    assert_eq!(meta(&imp, "Quiet Time (sec)"), "2");
    assert_eq!(meta(&imp, "Data Points"), "300");
}

#[test]
fn a_long_cv_with_a_repeated_extreme_has_vertices_83_and_82_rows_apart() {
    let n = 1000;
    let bytes = cv_file(0.4, 1.3, -0.35, 0.02, true, n, |i| i as f32 * 1e-7);
    let imp = import(&bytes);
    let lens: Vec<usize> = imp.workbook.sheets[0].columns.iter().step_by(3).map(|c| c.len()).collect();
    // Two sweeps are 165 rows, so every scan holds 165 rows, except the short first one.
    assert_eq!(lens[0], 128, "0.4 up to 1.3 and down to the first -0.34 and its repeat: 46 + 82");
    assert!(lens[1..lens.len() - 1].iter().all(|&l| l == 165), "{lens:?}");
    assert_eq!(lens.iter().sum::<usize>(), n);
}

#[test]
fn the_charge_of_an_i_t_run_follows_the_sample_interval() {
    // One channel: the current of 2 mA per second of time is linear, so the trapezoid rule is exact.
    let n = 101;
    let mut b = Builder::new("i-t", "Amperometric i-t Curve");
    b.f(E_INIT, 0.5).f(INTERVAL, 0.5).f(RUN_TIME, 50.0).f(SENSITIVITY, 1e-3).f(QUIET_TIME, 0.0);
    b.channels(vec![(0..n).map(|k| 2e-3 * (k as f32 * 0.5)).collect()]);
    let imp = import(&b.build());
    let s = &imp.workbook.sheets[0];
    assert_eq!(s.name, "Data");
    assert_eq!(s.columns.len(), 4, "time, current in A, current in mA, charge");
    assert_eq!((s.columns[0].long_name.as_str(), s.columns[0].units.as_str()), ("Time", "s"));
    assert_eq!(s.columns[1].long_name, "Current");
    assert_eq!(s.columns[3].long_name, "Charge");
    let t = s.columns[0].numeric().unwrap();
    assert!(t[0] == 0.0 && (t[100] - 50.0).abs() < 1e-9, "time starts at 0 and is k times the sample interval");
    assert!(t.windows(2).all(|w| ((w[1] - w[0]) - 0.5).abs() < 1e-9));
    let q = s.columns[3].numeric().unwrap();
    assert!((q[100] - 1e-3 * 50.0f64.powi(2)).abs() < 1e-6, "{}", q[100]);
    assert_eq!(meta(&imp, "Run Time (sec)"), "50");
    assert!(imp.warnings.is_empty(), "{:?}", imp.warnings);
}

#[test]
fn two_channel_i_t_data_alternates_point_by_point() {
    let n = 40;
    let mut b = Builder::new("i-t", "Amperometric i-t Curve");
    b.f(E_INIT, 1.0).f(E_HIGH, 0.9).f(INTERVAL, 0.1).f(RUN_TIME, 4.0).f(SENSITIVITY, 1e-3).f(SENSITIVITY_2, 1e-4);
    b.channels(vec![
        (0..n).map(|k| 1e-3 + k as f32 * 1e-5).collect(),
        (0..n).map(|k| -2e-3 - k as f32 * 1e-5).collect(),
    ]);
    let imp = import(&b.build());
    let s = &imp.workbook.sheets[0];
    let names: Vec<(&str, &str)> = s.columns.iter().map(|c| (c.long_name.as_str(), c.units.as_str())).collect();
    assert_eq!(
        names,
        [
            ("Time", "s"),
            ("i1", "A"),
            ("Current", "mA"),
            ("i2", "A"),
            ("Current", "mA"),
            ("Charge", "C"),
            ("Charge", "C")
        ]
    );
    assert_eq!(s.columns[2].comments, "i1");
    assert_eq!(s.columns[6].comments, "i2");
    let (i1, i2) = (s.columns[1].numeric().unwrap(), s.columns[3].numeric().unwrap());
    assert!(
        i1[0] > 0.0 && i2[0] < 0.0 && (i1[39] - (1e-3 + 39e-5)).abs() < 1e-9 && (i2[39] + (2e-3 + 39e-5)).abs() < 1e-9
    );
    assert_eq!(meta(&imp, "E2"), "0.9");
    assert_eq!(meta(&imp, "Sens2"), "0.0001");
    assert_eq!(meta(&imp, "Channels"), "2");
}

#[test]
fn the_bytes_of_a_two_channel_run_alternate_and_an_old_three_channel_file_keeps_channel_1_apart() {
    let n = 5;
    let ch = |c: usize| -> Vec<f32> { (0..n).map(|k| (c * 100 + k) as f32).collect() };
    let mut b = Builder::new("i-t", "Amperometric i-t Curve");
    b.f(INTERVAL, 0.1).f(SENSITIVITY, 1e-3).channels(vec![ch(1), ch(2)]);
    let bytes = b.build();
    let raw: Vec<f32> =
        bytes[bytes.len() - 8 * n..].as_chunks::<4>().0.iter().map(|w| f32::from_le_bytes(*w)).collect();
    assert_eq!(raw, [100.0, 200.0, 101.0, 201.0, 102.0, 202.0, 103.0, 203.0, 104.0, 204.0]);
    let cols = |imp: &ChiImport| -> Vec<Vec<f64>> {
        let s = &imp.workbook.sheets[0];
        ["i1", "i2", "i3"]
            .iter()
            .filter_map(|name| s.columns.iter().find(|c| c.long_name == *name && c.units == "A"))
            .map(|c| c.numeric().unwrap().to_vec())
            .collect()
    };
    let got = cols(&import(&bytes));
    assert_eq!(got[0], [100.0, 101.0, 102.0, 103.0, 104.0]);
    assert_eq!(got[1], [200.0, 201.0, 202.0, 203.0, 204.0]);

    // Old layout, three channels: channel 1, then channels 2 and 3 alternating.
    let mut b = Builder::new("i-t", "Amperometric i-t Curve");
    b.f(INTERVAL, 0.1).f(SENSITIVITY, 1e-3).legacy().channels(vec![ch(1), ch(2), ch(3)]);
    let bytes = b.build();
    let raw: Vec<f32> =
        bytes[bytes.len() - 12 * n..].as_chunks::<4>().0.iter().map(|w| f32::from_le_bytes(*w)).collect();
    assert_eq!(&raw[..7], [100.0, 101.0, 102.0, 103.0, 104.0, 200.0, 300.0]);
    let got = cols(&import(&bytes));
    assert_eq!(got.len(), 3);
    assert_eq!(got[0], [100.0, 101.0, 102.0, 103.0, 104.0]);
    assert_eq!(got[1], [200.0, 201.0, 202.0, 203.0, 204.0]);
    assert_eq!(got[2], [300.0, 301.0, 302.0, 303.0, 304.0]);
}

#[test]
fn the_time_of_an_i_t_run_starts_at_zero_in_either_layout() {
    for legacy in [false, true] {
        let mut b = Builder::new("i-t", "Amperometric i-t Curve");
        b.f(INTERVAL, 0.1).f(RUN_TIME, 0.5).f(SENSITIVITY, 1e-3);
        if legacy {
            b.legacy();
        }
        b.channels(vec![vec![1e-3; 5]]);
        let imp = import(&b.build());
        assert_eq!(imp.workbook.sheets[0].columns[0].numeric().unwrap(), [0.0, 0.1, 0.2, 0.3, 0.4]);
    }
}

#[test]
fn the_header_text_of_the_run_is_kept_in_the_metadata() {
    let mut b = Builder::new("CV", "Cyclic Voltammetry");
    b.header = "CV".to_string();
    b.f(E_INIT, 0.4).f(E_HIGH, 1.3).f(E_LOW, -0.35).f(INTERVAL, 0.02).f(POLARITY, 1.0);
    b.channels(vec![vec![1e-6; 300]]);
    let imp = import(&b.build());
    let names: Vec<&str> = imp.meta.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(names[..4], ["Technique", "Date", "Instrument Model", "Header"]);
    assert_eq!(meta(&imp, "Header"), "CV");
    let empty = import(&cv_file(0.4, 1.3, -0.35, 0.02, true, 300, |_| 1e-6));
    assert!(empty.meta.iter().all(|(k, _)| k != "Header"));
}

#[test]
fn a_current_pinned_at_the_range_limit_is_reported_with_its_channel() {
    let n = 200;
    let mut b = Builder::new("i-t", "Amperometric i-t Curve");
    b.f(INTERVAL, 0.1).f(RUN_TIME, 20.0).f(SENSITIVITY, 1e-3);
    let pinned: Vec<f32> = (0..n).map(|k| if k < 30 { 0.0105 } else { 0.004 - k as f32 * 1e-6 }).collect();
    b.channels(vec![(0..n).map(|k| 1e-3 + k as f32 * 1e-6).collect(), pinned]);
    let imp = import(&b.build());
    assert_eq!(imp.warnings.len(), 1, "{:?}", imp.warnings);
    let w = &imp.warnings[0];
    assert!(w.contains("i2") && w.contains("positive") && w.contains("10.5 mA") && w.contains("30 consecutive"), "{w}");
}

#[test]
fn a_run_that_stops_early_is_reported() {
    let mut b = Builder::new("i-t", "Amperometric i-t Curve");
    b.f(INTERVAL, 0.1).f(RUN_TIME, 16000.0).f(SENSITIVITY, 1e-3);
    b.channels(vec![vec![1e-3; 50]]);
    let imp = import(&b.build());
    assert!(imp.warnings.iter().any(|w| w.contains("16000") && w.contains("5")), "{:?}", imp.warnings);
}

#[test]
fn chronopotentiometry_keeps_the_potential_inside_its_limits_and_the_set_current() {
    // A charge-discharge between 0.4 V and 0.995 V: a triangle of 4 segments.
    let (low, high, n_seg) = (0.4f32, 0.995f32, 500usize);
    let potential: Vec<f32> = (0..4 * n_seg)
        .map(|k| {
            let (seg, i) = (k / n_seg, ((k % n_seg) + 1) as f32 / n_seg as f32);
            if seg % 2 == 0 { high - (high - low) * i } else { low + (high - low) * i }
        })
        .collect();
    let mut b = Builder::new("CP", "Chronopotentiometry");
    b.f(E_HIGH, high).f(E_LOW, low).f(CP_CATHODIC_CURRENT, 0.0012).f(CP_ANODIC_CURRENT, 0.0012);
    b.f(CP_CATHODIC_TIME, 6800.0).f(CP_ANODIC_TIME, 6800.0).f(SEGMENTS, 4.0).f(INTERVAL, 0.2);
    b.channels(vec![potential]);
    let imp = import(&b.build());
    assert_eq!(imp.technique, "Chronopotentiometry");
    assert!(imp.warnings.is_empty(), "{:?}", imp.warnings);
    assert_eq!(imp.scans, 0);
    let s = &imp.workbook.sheets[0];
    assert_eq!(s.columns.len(), 2, "time and potential only");
    assert_eq!((s.columns[0].long_name.as_str(), s.columns[0].units.as_str()), ("Time", "s"));
    assert_eq!((s.columns[1].long_name.as_str(), s.columns[1].units.as_str()), ("Potential", "V"));
    assert_eq!(s.columns[1].designation, Designation::Y);
    let t = s.columns[0].numeric().unwrap();
    assert!((t[1] - t[0] - 0.2).abs() < 1e-9 && (t[1999] - 399.8).abs() < 1e-9 && t[0] == 0.0);
    let e = s.columns[1].numeric().unwrap();
    assert!(e.iter().all(|v| (0.4 - 1e-6..=0.995 + 1e-6).contains(v)));
    assert_eq!(meta(&imp, "Cathodic Current (A)"), "0.0012");
    assert_eq!(meta(&imp, "High E Limit (V)"), "0.995");
    assert_eq!(meta(&imp, "Segment"), "4");
    // Three turning points for four segments.
    let vertices = e.windows(3).filter(|w| (w[1] - w[0]) * (w[2] - w[1]) < 0.0).count();
    assert_eq!(vertices, 3);
}

#[test]
fn a_chronopotentiometry_potential_beyond_its_limit_is_reported() {
    let mut b = Builder::new("CP", "Chronopotentiometry");
    b.f(E_HIGH, 0.9).f(E_LOW, 0.4).f(INTERVAL, 0.1);
    b.channels(vec![vec![0.5, 0.6, 1.2, 0.7]]);
    let imp = import(&b.build());
    assert!(imp.warnings.iter().any(|w| w.contains("leaves the range")), "{:?}", imp.warnings);
}

#[test]
fn unknown_techniques_and_damaged_files_are_refused_with_a_message() {
    let mut b = Builder::new("LSV", "Linear Sweep Voltammetry");
    b.channels(vec![vec![1.0; 10]]);
    let e = parse_chi_bin(&b.build(), "lsv", &ChiOptions::default()).unwrap_err().to_string();
    assert!(e.contains("lsv") && e.contains("Linear Sweep Voltammetry") && e.contains("not supported"), "{e}");

    let good = cv_file(1.2, 1.2, 0.2, 0.001, false, 100, |_| 1e-3);
    assert!(parse_chi_bin(b"not a binary", "x", &ChiOptions::default()).is_err());
    // A multi-channel file cut short cannot be read; a one-channel file can, with a warning.
    let short = &good[..good.len() - 40];
    let imp = parse_chi_bin(short, "short", &ChiOptions::default()).unwrap();
    assert!(
        imp.warnings.iter().any(|w| w.contains("announces 100 points") && w.contains("only 90")),
        "{:?}",
        imp.warnings
    );
    assert!(imp.warnings.iter().all(|w| !w.contains("  ")), "message holds a run of spaces: {:?}", imp.warnings);
    let mut two = Builder::new("i-t", "Amperometric i-t Curve");
    two.f(INTERVAL, 0.1).channels(vec![vec![1.0; 20], vec![2.0; 20]]);
    let bytes = two.build();
    let err = parse_chi_bin(&bytes[..bytes.len() - 4], "two", &ChiOptions::default()).unwrap_err().to_string();
    assert!(!err.contains("  "), "message holds a run of spaces: {err}");
    // Bytes after the data are ignored with a warning.
    let mut long = good.clone();
    long.extend([0u8; 7]);
    assert!(import(&long).warnings.iter().any(|w| w.contains("7 bytes")));
    // A sweep whose parameters do not make sense is an error, not a wrong curve.
    let bad = cv_file(5.0, 1.2, 0.2, 0.001, false, 100, |_| 1e-3);
    assert!(parse_chi_bin(&bad, "bad", &ChiOptions::default()).unwrap_err().to_string().contains("Init E"));
}

#[test]
fn no_prefix_or_scramble_of_a_valid_file_makes_the_reader_panic() {
    let good = cv_file(1.2, 1.2, 0.2, 0.001, false, 60, |i| i as f32 * 1e-5);
    for cut in 0..good.len() {
        let _ = parse_chi_bin(&good[..cut], "cut", &ChiOptions::default());
    }
    for pos in (0..BLOCK_LEN + 120).step_by(3) {
        let mut broken = good.clone();
        broken[pos] ^= 0xFF;
        let _ = parse_chi_bin(&broken, "broken", &ChiOptions::default());
    }
}

#[test]
fn the_current_can_be_inverted_like_in_the_text_importer() {
    let bytes = cv_file(1.2, 1.2, 0.2, 0.001, false, 2000, |_| 2e-3);
    let opts = ChiOptions { invert_current: true, add_milliamp: false, ..ChiOptions::default() };
    let imp = parse_chi_bin(&bytes, "inv", &opts).unwrap();
    assert!(imp.workbook.sheets[0].columns[1].numeric().unwrap().iter().all(|&v| (v + 2e-3).abs() < 1e-9));
}

// ---- local files: skipped when there are none on this computer -----------------------------
//
// These tests read whatever CH Instruments binary files are present locally (the owner's data,
// which is never committed) and check only the rules of the layout. They name no file and state
// no number taken from the owner's data.

/// The folders searched for local files: `$SHEAF_CHI_BIN_DIR`, the folders from this crate up to
/// the main checkout (a work tree sits below it), and the scratch folders of the
/// reverse-engineering workspace.
fn local_dirs() -> Vec<PathBuf> {
    let mut places: Vec<PathBuf> = Vec::new();
    if let Some(dir) = std::env::var_os("SHEAF_CHI_BIN_DIR") {
        places.push(PathBuf::from(dir));
    }
    let mut dir = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    for _ in 0..6 {
        places.push(dir.clone());
        for sub in ["pair", "vendor"] {
            places.push(dir.join("re-workspace/scratch/chibin").join(sub));
        }
        if !dir.pop() {
            break;
        }
    }
    places
}

/// Every local file with the binary marker, at most a few dozen.
fn local_bin_files() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    for dir in local_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            let is_bin = p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bin"));
            if is_bin && p.is_file() && !found.contains(&p) {
                found.push(p);
            }
        }
    }
    found.sort();
    found.truncate(40);
    found
}

fn number(imp: &ChiImport, key: &str) -> f64 {
    meta(imp, key).parse().unwrap_or_else(|_| panic!("{key} is not a number"))
}

#[test]
fn local_binary_files_follow_the_layout_rules() {
    let files = local_bin_files();
    if files.is_empty() {
        eprintln!("no local CH Instruments binary files; local test skipped");
        return;
    }
    for path in files {
        let bytes = std::fs::read(&path).unwrap();
        if !looks_like_chi_bin(&bytes) {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let imp = match parse_chi_bin(&bytes, &name, &ChiOptions::default()) {
            Ok(imp) => imp,
            // Techniques the reader does not know are refused with a message naming them.
            Err(e) if e.to_string().contains("not supported") => continue,
            Err(e) => panic!("{name}: {e}"),
        };
        let sheet = &imp.workbook.sheets[0];
        match imp.technique.as_str() {
            "Cyclic Voltammetry" => {
                // The rebuilt sweep stays between the header's limits.
                let (hi, lo) = (number(&imp, "High E (V)"), number(&imp, "Low E (V)"));
                for c in sheet.columns.iter().filter(|c| c.units == "V") {
                    assert!(c.numeric().unwrap().iter().all(|&v| v >= lo - 1e-9 && v <= hi + 1e-9), "{name}");
                }
            }
            "Amperometric i-t Curve" => {
                // The time grows by the sample interval, starting at 0.
                let t = sheet.columns[0].numeric().unwrap();
                let dt = number(&imp, "Sample Interval (s)");
                assert!(t.windows(2).all(|w| ((w[1] - w[0]) - dt).abs() < 1e-6), "{name}");
                assert!(t[0].abs() < 1e-9, "{name}");
            }
            "Chronopotentiometry" => {
                // The potential stays between the two limits (reached to within a few mV).
                let e = sheet.columns[1].numeric().unwrap();
                let (hi, lo) = (number(&imp, "High E Limit (V)"), number(&imp, "Low E Limit (V)"));
                assert!(e.iter().all(|&v| v >= lo - 5e-3 && v <= hi + 5e-3), "{name}");
            }
            other => panic!("{name}: unexpected technique {other}"),
        }
    }
}

/// Reads the same run saved both ways (a binary file and the text export of it) and checks that
/// the two workbooks have the same shape and the same numbers up to the rounding of the text.
fn same_run(bin: &Path, txt: &Path) {
    let stem = bin.file_stem().unwrap().to_string_lossy().to_string();
    let from_bin = parse_chi_bin(&std::fs::read(bin).unwrap(), &stem, &ChiOptions::default()).unwrap();
    let from_txt = crate::chi::parse_chi(&crate::read_text_file(txt).unwrap(), &stem, &ChiOptions::default()).unwrap();
    assert_eq!(from_bin.technique, from_txt.technique);
    assert_eq!(from_bin.meta[1], from_txt.meta[1], "date");
    let header_of = |imp: &ChiImport| imp.meta.iter().find(|(k, _)| k == "Header").cloned();
    assert_eq!(header_of(&from_bin), header_of(&from_txt), "Header text");
    // The CHI1040C program writes the axis of its exports one step in (the first potential is
    // Init E plus a step, the first time one sample interval); the CHI627E program starts at Init E and
    // at 0. The reader follows the second. Such an export is only compared on its currents.
    let one_step_in = from_txt.meta.iter().any(|(k, v)| k == "Instrument Model" && v.contains("1040C"));
    if one_step_in && from_txt.technique == "Cyclic Voltammetry" {
        eprintln!("{stem}: CV exported by the CHI1040C starts one step in; not compared");
        return;
    }
    assert_eq!(from_bin.scans, from_txt.scans);
    let (a, b) = (&from_bin.workbook.sheets[0], &from_txt.workbook.sheets[0]);
    assert_eq!(a.name, b.name);
    assert_eq!(a.columns.len(), b.columns.len());
    for (ca, cb) in a.columns.iter().zip(&b.columns) {
        let (la, ua, ma, da) = (&ca.long_name, &ca.units, &ca.comments, ca.designation);
        assert_eq!((la, ua, ma, da), (&cb.long_name, &cb.units, &cb.comments, cb.designation));
        if one_step_in && ca.units != "A" {
            continue;
        }
        assert_eq!(ca.len(), cb.len(), "{stem}: {la} {ua} {ma}");
        let (xa, xb) = (ca.numeric().unwrap(), cb.numeric().unwrap());
        let scale = xb.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        let worst = xa.iter().zip(xb).map(|(p, q)| (p - q).abs()).fold(0.0, f64::max);
        // The text keeps 4 significant digits of a current, and 3 decimals of a potential.
        assert!(
            worst <= 6e-4 * scale.max(1e-12) + 6e-4 * (ca.units == "V") as u8 as f64,
            "{la} {ua}: worst difference {worst}, scale {scale}"
        );
    }
    // The binary reader also says when a run stopped before its set run time, and the text
    // importer says when it had to repair the rounded times of the text; neither applies to the other.
    let same: Vec<&String> = from_bin.warnings.iter().filter(|w| !w.contains("was set to")).collect();
    let text: Vec<&String> = from_txt.warnings.iter().filter(|w| !w.contains("are rounded")).collect();
    // A warning quotes the axis range in brackets, which moves with the one-step-in convention.
    let unbracket = |w: &&String| w.split(" (").next().unwrap_or("").to_string();
    if one_step_in {
        assert_eq!(same.iter().map(unbracket).collect::<Vec<_>>(), text.iter().map(unbracket).collect::<Vec<_>>());
    } else {
        assert_eq!(same, text);
    }
}

#[test]
fn local_binary_files_with_a_text_export_give_the_same_workbook() {
    let pairs: Vec<(PathBuf, PathBuf)> = local_bin_files()
        .into_iter()
        .filter_map(|bin| {
            let txt = bin.with_extension("txt");
            txt.is_file().then_some((bin, txt))
        })
        .collect();
    if pairs.is_empty() {
        eprintln!("no local binary file with a text export beside it; local test skipped");
    }
    for (bin, txt) in pairs {
        same_run(&bin, &txt);
    }
}
