//! The fills against Origin 2016's `patternN`, `patternD` and `patternT` (probes
//! `re-workspace/scratch/t74/f1`, `f2`, local only; workbook-model.md "Fill Column With patterns").

use super::*;
use crate::column::Designation;

fn numbers(p: &FillPattern) -> Vec<f64> {
    match generate(p, 0, 7).unwrap() {
        FillValues::Numbers(v, _) => v,
        FillValues::Texts(_) => panic!("numbers expected"),
    }
}

fn n(from: f64, to: f64, inc: f64, each: usize, sequence: usize) -> FillPattern {
    FillPattern::Numbers { from, to, inc, mode: FillMode::Repeat, each, sequence, size: 10 }
}

#[test]
fn number_runs_as_origin() {
    assert_eq!(numbers(&n(1.0, 5.0, 1.0, 1, 1)), [1.0, 2.0, 3.0, 4.0, 5.0]);
    let b = numbers(&n(1.0, 3.0, 1.0, 2, 3));
    assert_eq!(b.len(), 18);
    assert_eq!(&b[..8], [1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 1.0, 1.0]);
    assert_eq!(numbers(&n(1.0, 2.0, 0.25, 1, 1)), [1.0, 1.25, 1.5, 1.75, 2.0]);
    // 5 to 1 by 1 is empty in Origin (nothing written); by -1 it counts down.
    assert_eq!(generate(&n(5.0, 1.0, 1.0, 1, 1), 0, 1).err(), Some(FillError::Empty));
    assert_eq!(numbers(&n(5.0, 1.0, -1.0, 1, 1)), [5.0, 4.0, 3.0, 2.0, 1.0]);
    assert_eq!(numbers(&n(1.0, 10.0, 3.0, 1, 1)), [1.0, 4.0, 7.0, 10.0]);
    // The set size is not used in Repeat mode.
    let p =
        FillPattern::Numbers { from: 1.0, to: 3.0, inc: 1.0, mode: FillMode::Repeat, each: 1, sequence: 1, size: 8 };
    assert_eq!(numbers(&p), [1.0, 2.0, 3.0]);
}

#[test]
fn random_numbers_draw_size_values_from_the_run_with_replacement() {
    let p =
        FillPattern::Numbers { from: 1.0, to: 4.0, inc: 1.0, mode: FillMode::Random, each: 1, sequence: 1, size: 7 };
    let v = numbers(&p);
    assert_eq!(v.len(), 7);
    assert!(v.iter().all(|x| [1.0, 2.0, 3.0, 4.0].contains(x)), "{v:?}");
    let many =
        FillPattern::Numbers { from: 1.0, to: 4.0, inc: 1.0, mode: FillMode::Random, each: 1, sequence: 1, size: 400 };
    let v = numbers(&many);
    for k in 1..=4 {
        assert!(v.contains(&(k as f64)), "{k} never drawn");
    }
}

fn dates(unit: DateUnit, from: f64, to: f64, inc: f64) -> Vec<f64> {
    let p = FillPattern::Dates { time: false, from, to, inc, unit, mode: FillMode::Repeat };
    numbers(&p)
}

#[test]
fn dates_step_by_fixed_lengths_as_origin() {
    // 2026-01-01 is 2461041, 2024-01-01 is 2460310 (Origin's numbers).
    assert_eq!(
        dates(DateUnit::Day, 2461041.0, 2461050.0, 2.0),
        [2461041.0, 2461043.0, 2461045.0, 2461047.0, 2461049.0]
    );
    // A month is 31 days: from Jan 31 to Jun 30.
    assert_eq!(
        dates(DateUnit::Month, 2461071.0, 2461221.0, 1.0),
        [2461071.0, 2461102.0, 2461133.0, 2461164.0, 2461195.0]
    );
    // A year is 365 days, also over a leap year.
    assert_eq!(
        dates(DateUnit::Year, 2460310.0, 2461924.0, 1.0),
        [2460310.0, 2460675.0, 2461040.0, 2461405.0, 2461770.0]
    );
    assert_eq!(
        dates(DateUnit::Week, 2461041.0, 2461072.0, 1.0),
        [2461041.0, 2461048.0, 2461055.0, 2461062.0, 2461069.0]
    );
    let hours = dates(DateUnit::Hour, 2461041.0, 2461043.0, 6.0);
    assert_eq!(hours.len(), 9);
    assert_eq!((hours[1], hours[8]), (2461041.25, 2461043.0));
    // Times: 0.25 to 0.5 by 90 minutes.
    let p = FillPattern::Dates {
        time: true,
        from: 0.25,
        to: 0.5,
        inc: 90.0,
        unit: DateUnit::Minute,
        mode: FillMode::Repeat,
    };
    match generate(&p, 0, 1).unwrap() {
        FillValues::Numbers(v, f) => {
            assert_eq!(v.len(), 5);
            assert!((v[1] - 0.3125).abs() < 1e-12 && (v[4] - 0.5).abs() < 1e-12, "{v:?}");
            assert!(matches!(f, Some(NumberFormat::Time(_))));
        }
        FillValues::Texts(_) => panic!(),
    }
    let secs = dates(DateUnit::Second, 0.0, 0.0001, 2.0);
    assert_eq!(secs.len(), 5);
    assert!((secs[1] - 2.3148148148148147e-5).abs() < 1e-18);
}

#[test]
fn random_dates_fill_as_many_rows_as_the_sheet_has() {
    let p = FillPattern::Dates {
        time: false,
        from: 2461041.0,
        to: 2461060.0,
        inc: 1.0,
        unit: DateUnit::Day,
        mode: FillMode::Random,
    };
    match generate(&p, 9, 3).unwrap() {
        FillValues::Numbers(v, _) => {
            assert_eq!(v.len(), 9);
            assert!(v.iter().all(|d| (2461041.0..=2461060.0).contains(d) && d.fract() == 0.0), "{v:?}");
        }
        FillValues::Texts(_) => panic!(),
    }
}

#[test]
fn text_items_repeat_as_numbers_do() {
    assert_eq!(text_items(r#"Boston "New York" Chicago"#), ["Boston", "New York", "Chicago"]);
    assert_eq!(text_items("  a   b "), ["a", "b"]);
    let p = FillPattern::Text { text: "a b c".into(), mode: FillMode::Repeat, each: 2, sequence: 2, size: 10 };
    match generate(&p, 0, 1).unwrap() {
        FillValues::Texts(v) => assert_eq!(v, ["a", "a", "b", "b", "c", "c", "a", "a", "b", "b", "c", "c"]),
        FillValues::Numbers(..) => panic!(),
    }
    let empty = FillPattern::Text { text: "   ".into(), mode: FillMode::Repeat, each: 1, sequence: 1, size: 3 };
    assert_eq!(generate(&empty, 0, 1).err(), Some(FillError::NoItems));
}

#[test]
fn a_fill_writes_from_the_first_row_and_keeps_the_rows_below() {
    let mut p = Project::new();
    let book = p.add_workbook("B");
    let x = p.add_column(book, 0, Designation::X, ColumnData::numeric((1..=8).map(f64::from).collect())).unwrap();
    let written = p.fill_pattern(x, &n(1.0, 3.0, 1.0, 1, 1), 1).unwrap();
    assert_eq!(written, 3);
    assert_eq!(p.numeric(x).unwrap(), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![9.0; 5])).unwrap();
    let t = FillPattern::Text { text: "x 2 y".into(), mode: FillMode::Repeat, each: 1, sequence: 1, size: 3 };
    p.fill_pattern(y, &t, 1).unwrap();
    let col = p.column(y).unwrap();
    assert_eq!(col.data.text_cell(0), Some("x"));
    assert_eq!(col.numeric().unwrap()[1], 2.0, "a number item is a number");
    assert_eq!(col.data.text_cell(2), Some("y"));
    assert_eq!(&col.numeric().unwrap()[3..], [9.0, 9.0], "the rows below keep their values");
    let d = p.add_column(book, 0, Designation::Y, ColumnData::numeric(Vec::new())).unwrap();
    let dp = FillPattern::Dates {
        time: false,
        from: 2461041.0,
        to: 2461043.0,
        inc: 1.0,
        unit: DateUnit::Day,
        mode: FillMode::Repeat,
    };
    p.fill_pattern(d, &dp, 1).unwrap();
    assert!(matches!(p.column(d).unwrap().format, NumberFormat::Date(_)));
}

#[test]
fn quick_fills_cover_every_row_of_the_sheet() {
    let mut p = Project::new();
    let book = p.add_workbook("B");
    let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![5.0; 6])).unwrap();
    let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    assert_eq!(p.fill_quick(y, QuickFill::RowNumbers, 1).unwrap(), 6, "the sheet's rows, not the column's");
    assert_eq!(p.numeric(y).unwrap(), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    p.fill_quick(x, QuickFill::Uniform, 3).unwrap();
    let u = p.numeric(x).unwrap().to_vec();
    assert!(u.iter().all(|v| (0.0..1.0).contains(v)), "{u:?}");
    assert_ne!(u, QuickFill::Uniform.values(6, 4), "another seed, other numbers");
    let empty = p.add_workbook("E");
    let e = p.add_column(empty, 0, Designation::Y, ColumnData::numeric(Vec::new())).unwrap();
    assert_eq!(p.fill_quick(e, QuickFill::Normal, 9).unwrap(), 32, "an empty sheet gets 32 rows");
    let big = QuickFill::Normal.values(20_000, 11);
    let mean = big.iter().sum::<f64>() / big.len() as f64;
    let sd = (big.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (big.len() - 1) as f64).sqrt();
    assert!(mean.abs() < 0.03 && (sd - 1.0).abs() < 0.03, "mean {mean}, sd {sd}");
    let big = QuickFill::Uniform.values(20_000, 12);
    let mean = big.iter().sum::<f64>() / big.len() as f64;
    assert!((mean - 0.5).abs() < 0.01, "mean {mean}");
}
