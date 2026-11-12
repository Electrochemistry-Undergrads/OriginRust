//! Dates and times as numbers, the way Origin stores them.
//!
//! A worksheet cell in a date or date-time column holds one number: the whole
//! part counts days and the fraction is the time of day. Day 2440587 is
//! 1970-01-01 (so the value is the number of days since that date plus 2440587,
//! which is the astronomical day number minus one, counted from midnight). A
//! column that holds only times of day stores the fraction alone. The numbers
//! were checked against Origin 2016 (see `docs/spec/workbook-model.md`).
//!
//! This module turns such numbers into text ([`format_date`], [`format_time`],
//! [`format_date_time`]) and reads the common text patterns back ([`parse`]).
//! No time zones and no leap seconds: the time of day is exactly the fraction.

use crate::format::NumberFormat;
use serde::{Deserialize, Serialize};

/// The value of 1970-01-01 00:00 in Origin's date numbers.
pub const UNIX_EPOCH_VALUE: f64 = 2_440_587.0;
const MS_PER_DAY: i64 = 86_400_000;

/// How the date part is written.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DatePattern {
    /// `11/5/2026` (month, day, year; Origin's default in a US setup).
    #[default]
    Mdy,
    /// `11-5-2026`.
    MdyDash,
    /// `05/11/2026` (day, month, year).
    Dmy,
    /// `05-11-2026`.
    DmyDash,
    /// `05.11.2026`.
    DmyDot,
    /// `2026-11-05`.
    Iso,
    /// `05-Nov-2026`.
    DMonY,
}

impl DatePattern {
    pub const ALL: [DatePattern; 7] = [
        DatePattern::Mdy,
        DatePattern::MdyDash,
        DatePattern::Dmy,
        DatePattern::DmyDash,
        DatePattern::DmyDot,
        DatePattern::Iso,
        DatePattern::DMonY,
    ];

    /// The pattern in the usual letters, for menus.
    pub fn label(&self) -> &'static str {
        match self {
            DatePattern::Mdy => "M/d/yyyy",
            DatePattern::MdyDash => "M-d-yyyy",
            DatePattern::Dmy => "dd/MM/yyyy",
            DatePattern::DmyDash => "dd-MM-yyyy",
            DatePattern::DmyDot => "dd.MM.yyyy",
            DatePattern::Iso => "yyyy-MM-dd",
            DatePattern::DMonY => "dd-MMM-yyyy",
        }
    }
}

/// How the time of day is written. All use 24 hours.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimePattern {
    /// `13:45`.
    Hm,
    /// `13:45:10`.
    #[default]
    Hms,
    /// `13:45:10.250`.
    HmsMs,
}

impl TimePattern {
    pub const ALL: [TimePattern; 3] = [TimePattern::Hm, TimePattern::Hms, TimePattern::HmsMs];

    pub fn label(&self) -> &'static str {
        match self {
            TimePattern::Hm => "HH:mm",
            TimePattern::Hms => "HH:mm:ss",
            TimePattern::HmsMs => "HH:mm:ss.fff",
        }
    }
}

/// The calendar fields of a date number.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Parts {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub milli: u32,
}

// ----- calendar arithmetic ----------------------------------------------------------

/// Days since 1970-01-01 of a proleptic Gregorian date.
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (i64::from(m) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if is_leap(y) {
                29
            } else {
                28
            }
        }
    }
}

/// The number for a calendar date at midnight, or `None` for an impossible date.
pub fn date_value(year: i64, month: u32, day: u32) -> Option<f64> {
    if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
        return None;
    }
    Some((days_from_civil(year, month, day) as f64) + UNIX_EPOCH_VALUE)
}

/// The fraction of a day for a time of day (`seconds` may have a fraction),
/// or `None` when a field is out of range.
pub fn time_value(hour: u32, minute: u32, seconds: f64) -> Option<f64> {
    if hour > 23 || minute > 59 || !(0.0..60.0).contains(&seconds) {
        return None;
    }
    Some((f64::from(hour) * 3600.0 + f64::from(minute) * 60.0 + seconds) / 86_400.0)
}

/// Date numbers outside this range (years about -100000 to 100000) are shown as plain numbers.
const MIN_VALUE: f64 = -3.0e7;
const MAX_VALUE: f64 = 3.7e7;

/// True when `v` is a date number the calendar code can show.
pub fn in_range(v: f64) -> bool {
    v.is_finite() && (MIN_VALUE..=MAX_VALUE).contains(&v)
}

/// Splits a date number into calendar fields, rounding to the millisecond.
pub fn parts(v: f64) -> Parts {
    let v = if v.is_finite() { v.clamp(MIN_VALUE, MAX_VALUE) } else { 0.0 };
    let days_f = v - UNIX_EPOCH_VALUE;
    let mut day = days_f.floor();
    let mut ms = ((days_f - day) * MS_PER_DAY as f64).round() as i64;
    if ms >= MS_PER_DAY {
        ms -= MS_PER_DAY;
        day += 1.0;
    }
    let (year, month, d) = civil_from_days(day as i64);
    Parts {
        year,
        month,
        day: d,
        hour: (ms / 3_600_000) as u32,
        minute: (ms / 60_000 % 60) as u32,
        second: (ms / 1000 % 60) as u32,
        milli: (ms % 1000) as u32,
    }
}

/// Day of the week of a date number, 0 = Monday.
pub fn weekday(v: f64) -> u32 {
    let day = (v - UNIX_EPOCH_VALUE).floor() as i64;
    // 1970-01-01 was a Thursday.
    (day + 3).rem_euclid(7) as u32
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

// ----- formatting ---------------------------------------------------------------------

/// Writes the date part of `v`.
pub fn format_date(v: f64, pat: DatePattern) -> String {
    let p = parts(v);
    let (y, m, d) = (p.year, p.month, p.day);
    match pat {
        DatePattern::Mdy => format!("{m}/{d}/{y}"),
        DatePattern::MdyDash => format!("{m}-{d}-{y}"),
        DatePattern::Dmy => format!("{d:02}/{m:02}/{y}"),
        DatePattern::DmyDash => format!("{d:02}-{m:02}-{y}"),
        DatePattern::DmyDot => format!("{d:02}.{m:02}.{y}"),
        DatePattern::Iso => format!("{y:04}-{m:02}-{d:02}"),
        DatePattern::DMonY => format!("{d:02}-{}-{y}", &MONTHS[m as usize - 1][..3]),
    }
}

/// Writes the time of day of `v` (the fraction of the day).
pub fn format_time(v: f64, pat: TimePattern) -> String {
    let p = parts(v);
    match pat {
        TimePattern::Hm => format!("{:02}:{:02}", p.hour, p.minute),
        TimePattern::Hms => format!("{:02}:{:02}:{:02}", p.hour, p.minute, p.second),
        TimePattern::HmsMs => format!("{:02}:{:02}:{:02}.{:03}", p.hour, p.minute, p.second, p.milli),
    }
}

/// Writes date and time of day separated by a space.
pub fn format_date_time(v: f64, date: DatePattern, time: TimePattern) -> String {
    format!("{} {}", format_date(v, date), format_time(v, time))
}

/// A time-only column holds the fraction of a day; a stored whole-day offset is ignored.
pub(crate) fn time_only_fraction(v: f64) -> f64 {
    v - v.floor()
}

// ----- parsing -------------------------------------------------------------------------

/// Which number comes first in `a/b/yyyy`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum DateOrder {
    /// Month first (`11/12/2026` is 12 November).
    #[default]
    Mdy,
    /// Day first (`11/12/2026` is 11 December).
    Dmy,
}

/// What [`parse`] found in one string.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Parsed {
    pub value: f64,
    /// The display format that matches how the text was written.
    pub format: NumberFormat,
    /// The text reads as a different date with the other [`DateOrder`] (both numbers are 12 or less and differ).
    pub ambiguous: bool,
}

/// A time of day such as `13:45`, `1:45:10 PM`, `13:45:10.25`.
fn parse_clock(s: &str) -> Option<(f64, TimePattern)> {
    let mut t = s.trim();
    let mut pm: Option<bool> = None;
    let upper = t.to_ascii_uppercase();
    if let Some(rest) = upper.strip_suffix("PM") {
        pm = Some(true);
        t = t[..rest.len()].trim_end();
    } else if let Some(rest) = upper.strip_suffix("AM") {
        pm = Some(false);
        t = t[..rest.len()].trim_end();
    }
    let fields: Vec<&str> = t.split(':').collect();
    if !(2..=3).contains(&fields.len()) || fields.iter().any(|f| f.is_empty() || f.len() > 12) {
        return None;
    }
    let int =
        |f: &str| -> Option<u32> { (f.len() <= 2 && f.bytes().all(|b| b.is_ascii_digit())).then(|| f.parse().ok())? };
    let mut hour = int(fields[0])?;
    // Clock minutes are always written with two digits; this keeps ratios such as 1:2 out.
    if fields[1].len() != 2 {
        return None;
    }
    let minute = int(fields[1])?;
    let (seconds, pat) = match fields.get(2) {
        None => (0.0, TimePattern::Hm),
        Some(f) => {
            let (whole, frac) = f.split_once('.').map_or((*f, None), |(a, b)| (a, Some(b)));
            if whole.len() != 2 || !whole.bytes().all(|b| b.is_ascii_digit()) || whole.is_empty() {
                return None;
            }
            if let Some(fr) = frac
                && (fr.is_empty() || !fr.bytes().all(|b| b.is_ascii_digit()))
            {
                return None;
            }
            let secs: f64 = f.parse().ok()?;
            (secs, if frac.is_some() { TimePattern::HmsMs } else { TimePattern::Hms })
        }
    };
    if let Some(is_pm) = pm {
        if !(1..=12).contains(&hour) {
            return None;
        }
        hour %= 12;
        if is_pm {
            hour += 12;
        }
    }
    Some((time_value(hour, minute, seconds)?, pat))
}

fn month_from_name(s: &str) -> Option<u32> {
    let lower = s.to_ascii_lowercase();
    if lower.len() < 3 {
        return None;
    }
    MONTHS
        .iter()
        .position(|m| {
            let m = m.to_ascii_lowercase();
            m == lower || (lower.len() == 3 && m.starts_with(&lower)) || (lower == "sept" && m == "september")
        })
        .map(|i| i as u32 + 1)
}

/// The date part: returns the value, the pattern and the ambiguity flag.
fn parse_date_part(s: &str, order: DateOrder) -> Option<(f64, DatePattern, bool)> {
    let sep = s.chars().find(|c| matches!(c, '/' | '-' | '.' | ' '))?;
    let fields: Vec<&str> = s.split(sep).map(str::trim).collect();
    if fields.len() != 3 || fields.iter().any(|f| f.is_empty()) {
        return None;
    }
    let digits = |f: &str, max: usize| f.len() <= max && f.bytes().all(|b| b.is_ascii_digit());
    // Month written as a name: d-Mon-yyyy or d Mon yyyy.
    if let Some(m) = month_from_name(fields[1]) {
        if !digits(fields[0], 2) || !(digits(fields[2], 4) && fields[2].len() == 4) {
            return None;
        }
        let v = date_value(fields[2].parse().ok()?, m, fields[0].parse().ok()?)?;
        return Some((v, DatePattern::DMonY, false));
    }
    if sep == ' ' {
        return None;
    }
    // Year first: yyyy-MM-dd (also with / or .).
    if fields[0].len() == 4 && digits(fields[0], 4) {
        if !digits(fields[1], 2) || !digits(fields[2], 2) {
            return None;
        }
        let v = date_value(fields[0].parse().ok()?, fields[1].parse().ok()?, fields[2].parse().ok()?)?;
        return Some((v, DatePattern::Iso, false));
    }
    // Year last: the first two numbers are day and month in some order.
    if !(fields[2].len() == 4 && digits(fields[2], 4)) || !digits(fields[0], 2) || !digits(fields[1], 2) {
        return None;
    }
    let (a, b): (u32, u32) = (fields[0].parse().ok()?, fields[1].parse().ok()?);
    let year: i64 = fields[2].parse().ok()?;
    let day_first = sep == '.' || order == DateOrder::Dmy;
    let (day, month) = if day_first { (a, b) } else { (b, a) };
    let v = date_value(year, month, day)?;
    let ambiguous = sep != '.' && a <= 12 && b <= 12 && a != b;
    let pat = match (sep, day_first) {
        ('.', _) => DatePattern::DmyDot,
        ('-', true) => DatePattern::DmyDash,
        ('-', false) => DatePattern::MdyDash,
        (_, true) => DatePattern::Dmy,
        (_, false) => DatePattern::Mdy,
    };
    Some((v, pat, ambiguous))
}

/// Reads a date, a time of day or both from text. Recognised: `2026-11-11`,
/// `2026-11-11 13:45:10`, `2026-11-11T13:45:10.5`, `11/11/2026`, `11/11/2026 1:45:10 PM`,
/// `11.11.2026`, `11-Nov-2026`, `11 November 2026`, `13:45`, `13:45:10.250`, `1:45 PM`.
/// `order` decides `a/b/yyyy` and `a-b-yyyy`.
pub fn parse(text: &str, order: DateOrder) -> Option<Parsed> {
    let s = text.trim();
    if s.is_empty() || s.len() > 40 {
        return None;
    }
    // Time of day only.
    if (!s.contains(['/', '.']) || s.contains(':'))
        && let Some((frac, tp)) = parse_clock(s)
    {
        return Some(Parsed { value: frac, format: NumberFormat::Time(tp), ambiguous: false });
    }
    // A date, with a clock after it when there is a space or `T`. A month name with
    // spaces ("11 Nov 2026") has two spaces inside the date part, so try the whole text first.
    if let Some((v, dp, amb)) = parse_date_part(s, order) {
        return Some(Parsed { value: v, format: NumberFormat::Date(dp), ambiguous: amb });
    }
    let (date_text, clock) = match s.rfind(['T', ' ']) {
        Some(_) => {
            // The clock is the part after the last separator, or the last two words with AM or PM.
            let words: Vec<&str> = s.split([' ', 'T']).collect();
            let has_ampm = words.last().is_some_and(|w| matches!(w.to_ascii_uppercase().as_str(), "AM" | "PM"));
            let n = if has_ampm { 2 } else { 1 };
            if words.len() <= n {
                return None;
            }
            let clock_words = &words[words.len() - n..];
            let clock_start = s.len() - clock_words.iter().map(|w| w.len()).sum::<usize>() - (n - 1);
            (s[..clock_start].trim_end_matches(['T', ' ']), s[clock_start..].to_string())
        }
        None => return None,
    };
    let (v, dp, amb) = parse_date_part(date_text.trim(), order)?;
    let (frac, tp) = parse_clock(&clock)?;
    Some(Parsed { value: v + frac, format: NumberFormat::DateTime(dp, tp), ambiguous: amb })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_matches_origin_numbers() {
        // Numbers produced by Origin 2016's date() and time() (see docs/spec/workbook-model.md).
        assert_eq!(date_value(2026, 11, 11), Some(2_461_355.0));
        assert_eq!(date_value(1970, 1, 1), Some(2_440_587.0));
        assert_eq!(date_value(1900, 1, 1), Some(2_415_020.0));
        assert_eq!(date_value(2000, 1, 1), Some(2_451_544.0));
        let t = time_value(13, 45, 10.0).unwrap();
        assert!((t - 0.573_032_407_407_407_4).abs() < 1e-15);
        let v = date_value(2026, 11, 11).unwrap() + t;
        assert!((v - 2_461_355.573_032_407_6).abs() < 1e-9);
    }

    #[test]
    fn impossible_dates_are_refused() {
        assert_eq!(date_value(2026, 2, 29), None);
        assert!(date_value(2024, 2, 29).is_some());
        assert_eq!(date_value(2026, 13, 1), None);
        assert_eq!(date_value(2026, 4, 31), None);
        assert_eq!(time_value(24, 0, 0.0), None);
    }

    #[test]
    fn ratios_are_not_clock_times() {
        for t in ["1:1", "1:2", "2:3", "1:30:5"] {
            assert!(parse_clock(t).is_none(), "{t}");
        }
        assert!(parse_clock("9:05").is_some());
        assert!(parse_clock("1:45:10 PM").is_some());
    }

    #[test]
    fn parts_round_trip_and_round_to_the_millisecond() {
        let v = date_value(2026, 2, 5).unwrap() + time_value(1, 5, 9.123).unwrap();
        let p = parts(v);
        assert_eq!((p.year, p.month, p.day, p.hour, p.minute, p.second, p.milli), (2026, 2, 5, 1, 5, 9, 123));
        // A value a hair below midnight rolls into the next day.
        let v = date_value(2026, 11, 11).unwrap() + time_value(23, 59, 59.0).unwrap() + 0.9999 / 86_400.0;
        let p = parts(v);
        assert_eq!((p.day, p.hour, p.minute, p.second, p.milli), (12, 0, 0, 0, 0));
        assert_eq!(weekday(date_value(2026, 11, 11).unwrap()), 2); // Wednesday
        assert_eq!(weekday(date_value(1970, 1, 1).unwrap()), 3); // Thursday
    }

    #[test]
    fn formats_match_origin_output() {
        // Origin showed 11/11/2026 13:45:10 and 2/5/2026 01:05:09 for these (subformats 11).
        let v = date_value(2026, 11, 11).unwrap() + time_value(13, 45, 10.0).unwrap();
        assert_eq!(format_date_time(v, DatePattern::Mdy, TimePattern::Hms), "11/11/2026 13:45:10");
        let w = date_value(2026, 2, 5).unwrap() + time_value(1, 5, 9.0).unwrap();
        assert_eq!(format_date_time(w, DatePattern::Mdy, TimePattern::Hms), "2/5/2026 01:05:09");
        assert_eq!(format_date(w, DatePattern::MdyDash), "2-5-2026");
        assert_eq!(format_date(w, DatePattern::Iso), "2026-02-05");
        assert_eq!(format_date(w, DatePattern::Dmy), "05/02/2026");
        assert_eq!(format_date(w, DatePattern::DmyDot), "05.02.2026");
        assert_eq!(format_date(w, DatePattern::DMonY), "05-Feb-2026");
        assert_eq!(format_time(v, TimePattern::Hm), "13:45");
        assert_eq!(format_time(v, TimePattern::HmsMs), "13:45:10.000");
    }

    fn p(s: &str, o: DateOrder) -> Parsed {
        parse(s, o).unwrap_or_else(|| panic!("{s} did not parse"))
    }

    #[test]
    fn parses_the_common_patterns() {
        let o = DateOrder::Mdy;
        let a = p("2026-11-11 13:45:10", o);
        assert_eq!(a.format, NumberFormat::DateTime(DatePattern::Iso, TimePattern::Hms));
        assert!((a.value - 2_461_355.573_032_407_6).abs() < 1e-9);
        assert_eq!(p("2026-11-11T13:45:10", o).value, a.value);
        let b = p("11/11/2026 1:45:10 PM", o);
        assert_eq!(b.format, NumberFormat::DateTime(DatePattern::Mdy, TimePattern::Hms));
        assert!((b.value - a.value).abs() < 1e-12);
        let c = p("11-Nov-2026", o);
        assert_eq!(c.value, 2_461_355.0);
        assert_eq!(c.format, NumberFormat::Date(DatePattern::DMonY));
        assert_eq!(p("11 November 2026", o).value, 2_461_355.0);
        let d = p("13:45", o);
        assert_eq!(d.format, NumberFormat::Time(TimePattern::Hm));
        assert!((d.value - 0.572_916_666_666_666_6).abs() < 1e-15);
        let e = p("13:45:10.250", o);
        assert_eq!(e.format, NumberFormat::Time(TimePattern::HmsMs));
        assert!((e.value - 0.573_035_300_925_925_9).abs() < 1e-12);
        assert!((p("12:00 AM", o).value).abs() < 1e-15);
        assert!((p("12:30 PM", o).value - 12.5 / 24.0).abs() < 1e-12);
        assert_eq!(p("2026-11-11", o).format, NumberFormat::Date(DatePattern::Iso));
        assert_eq!(p("11.11.2026", o).format, NumberFormat::Date(DatePattern::DmyDot));
    }

    #[test]
    fn day_month_order_follows_the_option_and_is_flagged() {
        let m = p("02/12/2026", DateOrder::Mdy);
        let d = p("02/12/2026", DateOrder::Dmy);
        assert_eq!(m.value, date_value(2026, 2, 12).unwrap());
        assert_eq!(d.value, date_value(2026, 12, 2).unwrap());
        assert!(m.ambiguous && d.ambiguous);
        assert_eq!(m.format, NumberFormat::Date(DatePattern::Mdy));
        assert_eq!(d.format, NumberFormat::Date(DatePattern::Dmy));
        // 13 cannot be a month, so only day-first fits.
        assert!(parse("13/11/2026", DateOrder::Mdy).is_none());
        let x = p("13/11/2026", DateOrder::Dmy);
        assert!(!x.ambiguous);
        assert!(!p("11/11/2026", DateOrder::Mdy).ambiguous);
        // Dotted dates are always day first.
        assert_eq!(p("02.12.2026", DateOrder::Mdy).value, date_value(2026, 12, 2).unwrap());
    }

    #[test]
    fn rejects_text_that_only_looks_like_a_date() {
        for s in [
            "",
            "hello",
            "12",
            "1.5",
            "3/4",
            "2026",
            "25:00",
            "12:75",
            "1:2:3:4",
            "2026-13-01",
            "2026-02-30",
            "a:b",
            "10:30 XM",
        ] {
            assert!(parse(s, DateOrder::Mdy).is_none(), "{s:?} should not be a date");
        }
    }
}
