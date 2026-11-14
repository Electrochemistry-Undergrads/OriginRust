//! Tests with byte strings built here. No sample file is read except by the ignored test at the end.

use super::numbers::{MISSING, cut_implausible_tail, decode_strings, decode_values, read_varint};
use super::{read_opju_bytes, summarize};
use sheaf_core::ColumnData;
use std::path::Path;

fn varint(mut v: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            return out;
        }
        out.push(b | 0x80);
    }
}

/// The value coding of spec section 5, written the other way round.
fn encode_values(values: &[f64]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut prev = 0u64;
    let mut nibbles: Vec<(u8, Vec<u8>)> = Vec::new();
    for &v in values {
        let bits = if v.is_nan() { MISSING.to_bits() } else { v.to_bits() };
        let x = bits ^ prev;
        prev = bits;
        let used = 8 - (x.leading_zeros() / 8) as usize;
        let n = if x == 0 { 0 } else { used.max(2) };
        let nibble = match n {
            0 => 0,
            8 => 0x0f,
            n => (n - 1) as u8,
        };
        nibbles.push((nibble, x.to_le_bytes()[..n].to_vec()));
    }
    for pair in nibbles.chunks(2) {
        let lo = &pair[0];
        let hi = pair.get(1);
        out.push(lo.0 | hi.map_or(0, |h| h.0 << 4));
        out.extend_from_slice(&lo.1);
        if let Some(h) = hi {
            out.extend_from_slice(&h.1);
        }
    }
    out
}

#[test]
fn a_column_is_cut_at_the_first_value_that_cannot_be_data() {
    let mut v = vec![1.0, -2.5e3, f64::NAN, 0.0, 4.2e-12, 3.1e255, 1.0, 2.0];
    assert_eq!(cut_implausible_tail(&mut v), 3, "from the huge value on");
    assert_eq!(v.len(), 5);
    assert!(v[2].is_nan(), "a missing value is kept");
    let mut w = vec![1.0, f64::INFINITY];
    assert_eq!(cut_implausible_tail(&mut w), 1);
    let mut ok = vec![1.0, 2.0];
    assert_eq!(cut_implausible_tail(&mut ok), 0);
    assert_eq!(ok, [1.0, 2.0]);
}

#[test]
fn varint_reads_low_group_first() {
    let mut p = 0;
    assert_eq!(read_varint(&[0x29], &mut p), Some(41));
    let mut p = 0;
    assert_eq!(read_varint(&varint(1000), &mut p), Some(1000));
    assert_eq!(p, 2);
    let mut p = 0;
    assert_eq!(read_varint(&[0x80], &mut p), None, "a number that never ends");
}

#[test]
fn values_round_trip_through_the_xor_coding() {
    let values =
        [105.1, 101.9, 100.8, 100.8, -3.5, 0.0, 1e300, f64::NAN, 2.0, 3.0, 4.0, 123456.789, f64::NAN, f64::NAN, 7.25];
    let bytes = encode_values(&values);
    let (back, used) = decode_values(&bytes, 0, bytes.len(), values.len()).expect("decodes");
    assert_eq!(used, bytes.len());
    for (a, b) in values.iter().zip(&back) {
        assert!(a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan()), "{a} vs {b}");
    }
}

#[test]
fn odd_counts_use_half_a_tag() {
    let values = [1.0, 2.0, 3.0];
    let bytes = encode_values(&values);
    let (back, used) = decode_values(&bytes, 0, bytes.len(), 3).unwrap();
    assert_eq!(back, values);
    assert_eq!(used, bytes.len());
}

#[test]
fn eight_bytes_with_the_flag_are_a_change_and_without_it_the_value() {
    // Tag 0x7f: the first token (low nibble 0xf) is eight flagged bytes, the second (0x7) eight bytes
    // without the flag. With nothing before, the flagged one is the value itself as well.
    let mut bytes = vec![0x7f];
    bytes.extend_from_slice(&1.5f64.to_bits().to_le_bytes());
    let second = 2.5f64.to_bits();
    bytes.extend_from_slice(&second.to_le_bytes());
    let (vals, _) = decode_values(&bytes, 0, bytes.len(), 2).unwrap();
    assert_eq!(vals, [1.5, 2.5], "unflagged: the bytes are the value");
    // The same bytes with the second token flagged: the value is the first one xor the bytes.
    let mut flagged = vec![0xff];
    flagged.extend_from_slice(&1.5f64.to_bits().to_le_bytes());
    flagged.extend_from_slice(&(1.5f64.to_bits() ^ second).to_le_bytes());
    let (vals, _) = decode_values(&flagged, 0, flagged.len(), 2).unwrap();
    assert_eq!(vals, [1.5, 2.5], "flagged: the bytes are a change");
}

#[test]
fn a_zero_length_token_repeats_the_value_before() {
    // 1.0 as eight flagged bytes, then tag 0x88: two tokens without bytes.
    let mut bytes = vec![0x0f];
    bytes.extend_from_slice(&1.0f64.to_bits().to_le_bytes());
    bytes.push(0x88);
    let (vals, used) = decode_values(&bytes, 0, bytes.len(), 3).unwrap();
    assert_eq!(vals, [1.0, 1.0, 1.0]);
    assert_eq!(used, bytes.len());
}

#[test]
fn truncated_values_are_refused_not_guessed() {
    let bytes = encode_values(&[1.0, 2.0, 3.0, 4.0]);
    assert!(decode_values(&bytes, 0, bytes.len() - 1, 4).is_none());
    assert!(decode_values(&[], 0, 0, 1).is_none());
}

#[test]
fn strings_are_a_length_and_bytes() {
    let mut bytes = Vec::new();
    for s in ["", "abc", "h\u{e9}llo"] {
        bytes.extend(varint(s.len() as u64));
        bytes.extend_from_slice(s.as_bytes());
    }
    assert_eq!(decode_strings(&bytes, 0, bytes.len(), 3).unwrap(), ["", "abc", "h\u{e9}llo"]);
    assert!(decode_strings(&bytes, 0, bytes.len() - 1, 3).is_none());
}

/// A column record: name field, the 24-byte block header, the data block and the sentinel.
fn numeric_record(name: &str, values: &[f64]) -> Vec<u8> {
    let tokens = encode_values(values);
    let count_code = (2 * values.len() - 1) as u64;
    let mut block = vec![0x0a, 5];
    block.extend(varint(32));
    block.extend([0xff, 0xff]);
    block.extend(varint(values.len() as u64));
    block.push(0);
    block.extend(varint(count_code));
    block.push(0x0c);
    block.extend(tokens);
    block.push(0xce);
    let mut out = vec![0x82, name.len() as u8];
    out.extend_from_slice(name.as_bytes());
    out.extend([0x8e, 0x02, 0xca, 0x10]);
    out.extend([0x9b, 0x18]);
    out.extend(24u64.to_le_bytes());
    out.extend((block.len() as u64).to_le_bytes());
    out.extend(0x1122_3344_5566_7788u64.to_le_bytes());
    out.extend(block);
    out.extend([0xfa, 0, 0, 0, 0]);
    out
}

fn text_record(name: &str, strings: &[&str]) -> Vec<u8> {
    let mut block = vec![0x0a, 5];
    block.extend(varint(strings.len() as u64 + 2));
    for s in strings {
        block.push(s.len() as u8);
        block.extend_from_slice(s.as_bytes());
    }
    block.extend([0, 0]);
    let mut out = vec![0x97, name.len() as u8];
    out.extend_from_slice(name.as_bytes());
    out.extend([0x98, 0x18]);
    out.extend(24u64.to_le_bytes());
    out.extend(0x2222_2222_2222_2222u64.to_le_bytes());
    out.extend(0x2222_2222_2222_2222u64.to_le_bytes());
    out.extend(block);
    out.extend([0xce, 0xfa, 0, 0, 0, 0]);
    out
}

/// A window record as it follows a preview picture: workbook with sheets, or a graph.
fn window_record(name: &str, long_name: &str, sheets: &[&str], origin_mark: bool) -> Vec<u8> {
    let mut out = b"IEND\xaeB`\x82\x0a\x80".to_vec();
    out.extend([0x70, 0x08, 0, 0]);
    out.extend_from_slice(name.as_bytes());
    out.extend([0x90, 0x0c, 0x1b, 0x00]);
    if origin_mark {
        out.extend_from_slice(b"ORIGIN");
    }
    if !long_name.is_empty() {
        out.extend([0x81, 0x01, 0xfc, 0xc0, 0xff, 0x02, 0x00, 0x39, 0x9f, long_name.len() as u8 + 1]);
        out.extend_from_slice(long_name.as_bytes());
        out.push(0);
    }
    for s in sheets {
        out.extend([0x94, s.len() as u8]);
        out.extend_from_slice(s.as_bytes());
        out.extend([0x0a, 0x00, 0x10, 0x01, 0x09]);
        out.extend_from_slice(b"_SEPD");
        out.extend([4, 2, 4, 0, 0]);
    }
    out
}

fn project(parts: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"CPYUA 4.3295 377\n".to_vec();
    out.extend([0x30, 0x01, 0x6c, 0xc0, 0x11, 0x01, 0x01, 0x0a]);
    for p in parts {
        out.extend_from_slice(p);
    }
    out
}

fn numbers(c: &ColumnData) -> Vec<f64> {
    match c {
        ColumnData::Numeric(v) => v.to_vec(),
        other => panic!("not numeric: {other:?}"),
    }
}

#[test]
fn a_workbook_with_two_sheets_is_read_with_its_values() {
    let file = project(&[
        numeric_record("Book1_A", &[1.0, 2.0, 3.0]),
        numeric_record("Book1_B", &[0.5, f64::NAN, 2.5]),
        text_record("Book1_C", &["x", "", "z"]),
        numeric_record("Book1_A@2", &[10.0, 20.0]),
        window_record("Book1", "My data", &["Sheet1", "Counts"], true),
    ]);
    let import = read_opju_bytes(&file, Path::new("t.opju")).unwrap();
    assert_eq!(import.version, "4.3295");
    assert_eq!(import.workbooks.len(), 1);
    let wb = &import.workbooks[0];
    assert_eq!((wb.name.as_str(), wb.long_name.as_str()), ("Book1", "My data"));
    assert_eq!(wb.sheets.len(), 2);
    assert_eq!(wb.sheets[0].name, "Sheet1");
    assert_eq!(wb.sheets[1].name, "Counts");
    let cols = &wb.sheets[0].columns;
    assert_eq!(cols.iter().map(|c| c.short_name.as_str()).collect::<Vec<_>>(), ["A", "B", "C"]);
    assert_eq!(numbers(&cols[0].data), [1.0, 2.0, 3.0]);
    let b = numbers(&cols[1].data);
    assert_eq!((b[0], b[2]), (0.5, 2.5));
    assert!(b[1].is_nan(), "the missing marker becomes NaN");
    match &cols[2].data {
        ColumnData::Text(t) => assert_eq!(t.as_slice(), ["x", "", "z", "", ""]),
        other => panic!("{other:?}"),
    }
    assert_eq!(numbers(&wb.sheets[1].columns[0].data), [10.0, 20.0]);
}

#[test]
fn graphs_matrices_and_unreadable_blocks_are_named_in_the_warnings() {
    let mut odd = numeric_record("Book2_A", &[1.0]);
    // Another data type (4) instead of 5.
    let at = odd.iter().position(|&c| c == 0x0a).unwrap();
    odd[at + 1] = 4;
    let file = project(&[
        numeric_record("Book2_B", &[4.0, 5.0]),
        odd,
        numeric_record("CS_1", &[1.0, 2.0]),
        numeric_record("MBook1_1", &[1.0]),
        window_record("Book2", "", &["Sheet1"], true),
        window_record("Graph1", "A graph", &[], false),
        window_record("MBook1", "", &["MSheet1"], true),
    ]);
    let import = read_opju_bytes(&file, Path::new("t.opju")).unwrap();
    let text = import.warnings.join("\n");
    assert!(text.contains("1 graph window"), "{text}");
    assert!(text.contains("1 matrix window"), "{text}");
    assert!(text.contains("another type"), "{text}");
    assert!(text.contains("belong to no workbook"), "{text}");
    assert!(text.contains("Long names"), "{text}");
    assert_eq!(import.workbooks.len(), 1);
    assert_eq!(import.workbooks[0].sheets[0].columns.len(), 1);
    let s = summarize(&file);
    assert_eq!((s.windows, s.workbook_windows, s.matrix_windows, s.graph_windows), (3, 1, 1, 1));
    assert_eq!((s.column_records, s.columns_read, s.columns_unmatched), (4, 1, 2));
}

#[test]
fn a_workbook_without_a_window_record_is_rebuilt_from_its_column_names() {
    let file = project(&[numeric_record("Lost_A", &[1.0, 2.0]), numeric_record("Lost_B", &[3.0, 4.0])]);
    let import = read_opju_bytes(&file, Path::new("t.opju")).unwrap();
    assert_eq!(import.workbooks.len(), 1);
    assert_eq!(import.workbooks[0].name, "Lost");
    assert_eq!(import.workbooks[0].sheets[0].name, "Sheet1");
    assert!(import.warnings.iter().any(|w| w.contains("no window record")));
}

#[test]
fn damaged_input_is_an_error_or_a_warning_never_a_panic() {
    let err = read_opju_bytes(b"PK\x03\x04 zip", Path::new("t.opju")).unwrap_err().to_string();
    assert!(err.contains("CPYUA"), "{err}");
    assert!(read_opju_bytes(b"CPYUA 4.3295 377", Path::new("t.opju")).is_err(), "no end of line");
    let good = project(&[
        numeric_record("Book1_A", &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
        window_record("Book1", "", &["Sheet1"], true),
    ]);
    for cut in (0..good.len()).step_by(3) {
        let _ = read_opju_bytes(&good[..cut], Path::new("t.opju"));
    }
    for i in (17..good.len()).step_by(5) {
        let mut copy = good.clone();
        copy[i] ^= 0xa5;
        let _ = read_opju_bytes(&copy, Path::new("t.opju"));
    }
}

#[test]
fn open_project_routes_an_opju_name_to_the_reader() {
    let file = project(&[numeric_record("Book1_A", &[1.0, 2.0]), window_record("Book1", "", &["Sheet1"], true)]);
    let dir = std::env::temp_dir().join(format!("sheaf-opju-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sample.opju");
    std::fs::write(&path, file).unwrap();
    assert!(crate::is_project_file(&path));
    let opened = crate::open_project(&path).unwrap();
    assert!(opened.foreign);
    assert_eq!(opened.save_path, None);
    assert_eq!(opened.project.workbooks().count(), 1);
    assert!(opened.warnings[0].contains("Unicode"));
    std::fs::remove_dir_all(&dir).ok();
}

/// Opens every file of the folder of OriginLab sample projects, if it is there. The files are never
/// copied into the repository. Run with `cargo test -p sheaf-io --lib opju -- --ignored --nocapture`.
#[test]
#[ignore = "needs the folder of sample .opju files next to the repository"]
fn every_sample_file_imports_with_sensible_counts() {
    let dir = std::env::var_os("SHEAF_OPJU_SAMPLES").map(std::path::PathBuf::from).or_else(|| {
        let here = std::env::current_dir().ok()?;
        here.ancestors().map(|a| a.join(".opju files for testing")).find(|p| p.is_dir())
    });
    let Some(dir) = dir else {
        eprintln!("no sample folder found; skipped");
        return;
    };
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "opju"))
        .collect();
    files.sort();
    assert!(!files.is_empty());
    for f in files {
        let bytes = std::fs::read(&f).unwrap();
        let s = summarize(&bytes);
        let import = read_opju_bytes(&bytes, &f).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        let columns: usize = import.workbooks.iter().flat_map(|w| &w.sheets).map(|s| s.columns.len()).sum();
        println!(
            "{}: windows {}, workbooks {}, columns found {}, read {}, skipped {}, unmatched {}",
            f.file_name().unwrap().to_string_lossy(),
            s.windows,
            import.workbooks.len(),
            s.column_records,
            columns,
            s.columns_skipped.values().sum::<usize>(),
            s.columns_unmatched
        );
        assert!(s.windows > 0, "{}: no window record", f.display());
        assert!(!import.workbooks.is_empty(), "{}: no workbook", f.display());
        assert_eq!(columns, s.columns_read);
        assert!(
            columns * 3 >= s.column_records - s.columns_unmatched,
            "{}: fewer than a third of the workbook columns were read",
            f.display()
        );
        let rows: usize =
            import.workbooks.iter().flat_map(|w| &w.sheets).flat_map(|s| &s.columns).map(|c| c.len()).sum();
        assert!(rows > 0, "{}: no values", f.display());
    }
}
