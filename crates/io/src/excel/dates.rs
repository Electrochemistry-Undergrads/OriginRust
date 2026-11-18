//! Excel dates and times: serial numbers to Sheaf's date numbers and back, and the display
//! patterns that go with them.

use calamine::ExcelDateTime;
use sheaf_core::datetime::{self, DatePattern, TimePattern};

/// What kind of value an Excel date cell holds.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum DateKind {
    /// A whole day.
    Date,
    /// A day with a time of day.
    DateTime,
    /// A time of day alone (the cell is from 0 to below one day).
    Time,
    /// A length of time: an elapsed-time format such as `[h]:mm:ss`, or a negative value, which
    /// is no date in Excel's calendar.
    Duration,
}

/// A date cell in Sheaf's numbers.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(super) struct Converted {
    /// A Sheaf date number for `Date` and `DateTime`; a fraction or a count of days for the others.
    pub value: f64,
    pub kind: DateKind,
    /// The cell was Excel's non-existent 1900-02-29 and was read as 1900-02-28.
    pub leap_bug: bool,
    /// The number of days the file stores (a count of days from the epoch for a date); `NaN` for
    /// a cell that was text in the file. A column of times reads it when a cell is over one day.
    pub days: f64,
    /// The Sheaf date number of a `Time` cell read as a moment on Excel's day 0 (1899-12-31).
    /// A column of dates reads it when a cell is under one day. `None` where that has no meaning.
    pub on_day_zero: Option<f64>,
}

impl Converted {
    /// A cell that was text in the file (an ISO date).
    pub(super) fn from_text(value: f64, kind: DateKind) -> Converted {
        Converted { value, kind, leap_bug: false, days: f64::NAN, on_day_zero: None }
    }

    /// The value as part of a column of times, `None` for a date cell that has no day count.
    pub(super) fn as_clock(&self) -> Option<f64> {
        match self.kind {
            DateKind::Time | DateKind::Duration => Some(self.value),
            DateKind::Date | DateKind::DateTime => self.days.is_finite().then_some(self.days),
        }
    }

    /// The value as part of a column of dates, with the kind that shows it; `None` for a length of
    /// time. A time of day is a moment on day 0, a whole day when it is exactly 0.
    pub(super) fn as_calendar(&self) -> Option<(f64, DateKind)> {
        match self.kind {
            DateKind::Date | DateKind::DateTime => Some((self.value, self.kind)),
            DateKind::Time => {
                self.on_day_zero.map(|d| (d, if self.value == 0.0 { DateKind::Date } else { DateKind::DateTime }))
            }
            DateKind::Duration => None,
        }
    }
}

/// Excel's serial numbers: day 1 is 1900-01-01 and, wrongly, 1900 is a leap year, so day 60 is a
/// 1900-02-29 that never existed. Days 61 and up are the real calendar from 1900-03-01.
const DAYS_1900_TO_EPOCH: f64 = 25_569.0;
/// Before the fake day the serial numbers sit one day lower.
const DAYS_1900_TO_EPOCH_EARLY: f64 = 25_568.0;
/// The Sheaf number of 1900-03-01 (Excel day 61).
const FIRST_REAL_DAY: f64 = 2_440_587.0 - 25_508.0;
/// The Sheaf number of 1900-01-01 (Excel day 1).
const FIRST_DAY: f64 = 2_440_587.0 - 25_567.0;

/// Reads an Excel date cell (1900 or 1904 system, the reader has applied the file's setting).
/// Dates are cut to milliseconds, which is all Excel shows. `None` for a value that is not a number.
///
/// The kind follows the value alone: below 0 and the cells the file marks as elapsed times are
/// durations, from 0 to below 1 a time of day, the rest dates. A column then settles the kind of
/// all its cells from its number format (see `build_column`).
pub(super) fn from_excel(dt: &ExcelDateTime) -> Option<Converted> {
    let v = dt.as_f64();
    if !v.is_finite() {
        return None;
    }
    if dt.is_duration() || v < 0.0 {
        return Some(Converted { value: v, kind: DateKind::Duration, leap_bug: false, days: v, on_day_zero: None });
    }
    let (year, month, day, hour, minute, second, milli) = dt.to_ymd_hms_milli();
    let exact = datetime::date_value(i64::from(year), u32::from(month), u32::from(day));
    let clock = u32::from(hour) * 3600 + u32::from(minute) * 60 + u32::from(second);
    let fraction = (f64::from(clock) + f64::from(milli) / 1000.0) / 86_400.0;
    if v < 1.0 {
        let on_day_zero = exact.map(|d| d + fraction);
        return Some(Converted { value: v, kind: DateKind::Time, leap_bug: false, days: v, on_day_zero });
    }
    let (date, leap_bug) = match exact {
        Some(d) => (d, false),
        None => (datetime::date_value(1900, 2, 28)?, true),
    };
    let kind = if clock == 0 && milli == 0 { DateKind::Date } else { DateKind::DateTime };
    Some(Converted { value: date + fraction, kind, leap_bug, days: v, on_day_zero: None })
}

/// The Excel serial number (1900 system) of a Sheaf date number, `None` before 1900-01-01.
pub(super) fn serial_from_date(v: f64) -> Option<f64> {
    if !v.is_finite() || v < FIRST_DAY {
        return None;
    }
    let day = v.floor();
    let offset = if day >= FIRST_REAL_DAY { DAYS_1900_TO_EPOCH } else { DAYS_1900_TO_EPOCH_EARLY };
    Some(day - datetime::UNIX_EPOCH_VALUE + offset + (v - day))
}

/// Drops quoted text, escaped characters, colours and the like from an Excel number format, lower
/// case. Elapsed-time brackets (`[h]`) keep their letter.
fn letters_of(code: &str) -> String {
    let first = code.split(';').next().unwrap_or("");
    let mut out = String::new();
    let mut chars = first.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                for q in chars.by_ref() {
                    if q == '"' {
                        break;
                    }
                }
            }
            '\\' | '_' | '*' => {
                chars.next();
            }
            '[' => {
                let inner: String = chars.by_ref().take_while(|c| *c != ']').collect();
                let l = inner.to_ascii_lowercase();
                if l.starts_with(['h', 'm', 's']) {
                    out.push_str(&l);
                }
            }
            c => out.push(c.to_ascii_lowercase()),
        }
    }
    out
}

/// The Sheaf patterns that best match an Excel number format code (`yyyy-mm-dd`, `h:mm:ss`,
/// `d/m/yy h:mm`). Either part is `None` when the code does not show it.
pub(super) fn patterns_from_code(code: &str) -> (Option<DatePattern>, Option<TimePattern>) {
    let mut c = letters_of(code);
    let ampm = c.contains("am/pm") || c.contains("a/p");
    c = c.replace("am/pm", "").replace("a/p", "");
    // Runs of one letter: (letter, length, byte position). A run of `m` beside `h` or `s` is
    // minutes, not a month.
    let mut runs: Vec<(char, usize, usize)> = Vec::new();
    let mut prev: Option<char> = None;
    for (pos, ch) in c.char_indices() {
        if ch.is_ascii_alphabetic() {
            match runs.last_mut() {
                Some(run) if prev == Some(ch) => run.1 += 1,
                _ => runs.push((ch, 1, pos)),
            }
        }
        prev = Some(ch);
    }
    let is_minutes = |i: usize| {
        runs[i].0 == 'm'
            && (i.checked_sub(1).is_some_and(|p| runs[p].0 == 'h') || runs.get(i + 1).is_some_and(|n| n.0 == 's'))
    };
    let letter =
        |l: char| runs.iter().enumerate().find(|(i, r)| r.0 == l && !(l == 'm' && is_minutes(*i))).map(|(_, r)| *r);
    let (year, month, day) = (letter('y'), letter('m'), letter('d'));
    let minutes = runs.iter().enumerate().any(|(i, _)| is_minutes(i));
    let hours = runs.iter().any(|r| r.0 == 'h');
    let seconds = runs.iter().any(|r| r.0 == 's');
    let date = if year.is_some() || month.is_some() || day.is_some() {
        Some(date_pattern(&c, year, month, day))
    } else {
        None
    };
    let time = if hours || seconds || minutes {
        let fraction = c.find("s.0").map_or(0, |p| c[p + 2..].chars().take_while(|ch| *ch == '0').count());
        Some(match (hours, seconds, ampm) {
            (true, _, true) if minutes => TimePattern::Hm12,
            (true, _, true) => TimePattern::H12,
            (false, true, _) if fraction > 0 => TimePattern::MsCs,
            (false, true, _) => TimePattern::Ms,
            (true, true, _) if fraction >= 3 => TimePattern::HmsMs,
            (true, true, _) if fraction > 0 => TimePattern::HmsCs,
            (true, true, _) => TimePattern::Hms,
            (true, false, _) if minutes => TimePattern::Hm,
            (true, false, _) => TimePattern::H,
            _ => TimePattern::Hms,
        })
    } else {
        None
    };
    (date, time)
}

fn date_pattern(
    code: &str,
    year: Option<(char, usize, usize)>,
    month: Option<(char, usize, usize)>,
    day: Option<(char, usize, usize)>,
) -> DatePattern {
    let sep = |a: usize, b: usize| code[a.min(b)..a.max(b)].chars().find(|c| !c.is_ascii_alphabetic() && *c != ' ');
    match (year, month, day) {
        (_, _, Some((_, n, _))) if n >= 4 => DatePattern::LongDate,
        (Some(_), Some((_, n, _)), Some(_)) if n >= 3 => DatePattern::DMonY,
        (None, Some((_, n, _)), Some(_)) if n >= 3 => DatePattern::MonthDay,
        (None, Some((_, n, _)), None) if n >= 3 => DatePattern::MonthShort,
        (None, Some(_), Some(_)) => DatePattern::MonthDayNum,
        (Some((_, n, _)), None, None) => {
            if n >= 3 {
                DatePattern::YearOnly
            } else {
                DatePattern::Year2
            }
        }
        (None, None, Some((_, 3, _))) => DatePattern::WeekdayShort,
        (None, None, Some(_)) => DatePattern::DayOnly,
        (Some(y), Some(m), Some(d))
            if y.0 == 'y' && [y.1, m.1, d.1] == [2, 2, 2] && run_of_letters(code, [y, m, d]) =>
        {
            DatePattern::Yymmdd
        }
        (Some(y), Some(m), Some(d)) => {
            let first = [y, m, d].into_iter().min_by_key(|r| r.2).map(|r| r.0).unwrap_or('y');
            match first {
                'y' => DatePattern::Iso,
                'd' => match sep(d.2, m.2) {
                    Some('.') => DatePattern::DmyDot,
                    Some('-') => DatePattern::DmyDash,
                    _ => DatePattern::Dmy,
                },
                _ => {
                    if sep(m.2, d.2) == Some('-') {
                        DatePattern::MdyDash
                    } else {
                        DatePattern::Mdy
                    }
                }
            }
        }
        _ => DatePattern::Iso,
    }
}

/// True when the three runs `y`, `m` and `d` (in any order) sit together with only letters between,
/// as in `yymmdd`.
fn run_of_letters(code: &str, runs: [(char, usize, usize); 3]) -> bool {
    let start = runs.iter().map(|r| r.2).min().unwrap_or(0);
    let end = runs.iter().map(|r| r.2 + r.1).max().unwrap_or(0);
    code.get(start..end).is_some_and(|span| span.chars().all(|c| c.is_ascii_alphabetic()))
}

/// An Excel number format code for a Sheaf date pattern.
pub(super) fn date_code(p: DatePattern) -> &'static str {
    match p {
        DatePattern::Mdy => "m/d/yyyy",
        DatePattern::MdyDash => "m-d-yyyy",
        DatePattern::Dmy => "dd/mm/yyyy",
        DatePattern::DmyDash => "dd-mm-yyyy",
        DatePattern::DmyDot => "dd.mm.yyyy",
        DatePattern::Iso | DatePattern::Quarter => "yyyy-mm-dd",
        DatePattern::DMonY => "dd-mmm-yyyy",
        DatePattern::LongDate => "dddd, mmmm d, yyyy",
        DatePattern::MonthDay => "mmm d",
        DatePattern::MonthDayNum => "m/d",
        DatePattern::DayOnly => "d",
        DatePattern::WeekdayShort | DatePattern::WeekdayLetter => "ddd",
        DatePattern::YearOnly => "yyyy",
        DatePattern::Year2 => "yy",
        DatePattern::Yymmdd => "yymmdd",
        DatePattern::MonthShort => "mmm",
        DatePattern::MonthLetter => "mmmmm",
    }
}

/// An Excel number format code for a Sheaf time pattern. With `elapsed` the hours are counted
/// without wrapping at 24 (`[h]:mm:ss`), as Sheaf does below 240 days.
pub(super) fn time_code(p: TimePattern, elapsed: bool) -> &'static str {
    match (p, elapsed) {
        (TimePattern::Hm, false) => "hh:mm",
        (TimePattern::Hm, true) => "[hh]:mm",
        (TimePattern::H, false) => "hh",
        (TimePattern::H, true) => "[hh]",
        (TimePattern::Hms, false) => "hh:mm:ss",
        (TimePattern::Hms, true) => "[hh]:mm:ss",
        (TimePattern::HmsCs, false) => "hh:mm:ss.00",
        (TimePattern::HmsCs, true) => "[hh]:mm:ss.00",
        (TimePattern::HmsMs, false) => "hh:mm:ss.000",
        (TimePattern::HmsMs, true) => "[hh]:mm:ss.000",
        (TimePattern::H12, _) => "hh AM/PM",
        (TimePattern::Hm12, _) => "hh:mm AM/PM",
        (TimePattern::Ms, _) => "mm:ss",
        (TimePattern::MsCs, _) => "mm:ss.00",
        (TimePattern::Hhmm, _) => "hhmm",
        (TimePattern::Hhmmss, _) => "hhmmss",
        (TimePattern::ElapsedD, _) => "0",
        (TimePattern::ElapsedDh | TimePattern::ElapsedDhm, _) => "[hh]:mm",
        (TimePattern::ElapsedDhms, _) => "[hh]:mm:ss",
        (TimePattern::ElapsedDhmsCs, _) => "[hh]:mm:ss.00",
        (TimePattern::ElapsedDhmsMicro, _) => "[hh]:mm:ss.000",
    }
}
