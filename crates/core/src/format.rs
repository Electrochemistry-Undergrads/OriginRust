//! Turning numbers into text for worksheet cells and tick labels.

use crate::datetime::{self, DatePattern, TimePattern};
use serde::{Deserialize, Serialize};

/// How a month or weekday name is written (Origin offers a short, a full and a one-letter form).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NameStyle {
    /// `Jan`, `Wed`.
    #[default]
    Short,
    /// `January`, `Wednesday`.
    Full,
    /// `J`, `W`.
    Letter,
}

impl NameStyle {
    pub const ALL: [NameStyle; 3] = [NameStyle::Short, NameStyle::Full, NameStyle::Letter];

    /// What Origin's Column Properties list shows for the style: a sample of the first month
    /// (`Jan`, `January`, `J`) or of Monday (`Mon`, `Monday`, `M`).
    pub fn sample(&self, month: bool) -> &'static str {
        match (self, month) {
            (NameStyle::Short, true) => "Jan",
            (NameStyle::Full, true) => "January",
            (NameStyle::Letter, true) => "J",
            (NameStyle::Short, false) => "Mon",
            (NameStyle::Full, false) => "Monday",
            (NameStyle::Letter, false) => "M",
        }
    }

    fn apply(&self, name: &str) -> String {
        match self {
            NameStyle::Short => name[..3].to_string(),
            NameStyle::Full => name.to_string(),
            NameStyle::Letter => name[..1].to_string(),
        }
    }
}

/// How a numeric column shows its values.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "kind", content = "digits")]
pub enum NumberFormat {
    /// Origin's General format. `NumberFormat::format` gives up to 9 significant digits,
    /// switching to scientific notation for very large or very small magnitudes, trailing zeros
    /// dropped; that is the text of cell editing and of results. What the worksheet grid and Copy
    /// show for this format is `Column::shown_text` (5 decimals as Origin's Options default), and
    /// what Export and Copy with full precision write is `format_full_precision` (15 digits).
    #[default]
    General,
    /// A fixed number of digits after the decimal point.
    Decimal(u8),
    /// Scientific notation with a fixed number of digits after the decimal point.
    Scientific(u8),
    /// A fixed number of significant digits.
    Significant(u8),
    /// Engineering notation: the exponent is a multiple of 3 (`15.0E3`), with this many digits
    /// after the point.
    Engineering(u8),
    /// Engineering notation with an SI prefix letter (`15.0k`, `2.50m`, `3.3M`), this many digits
    /// after the point. Outside the prefixes (below 1E-24 or from 1E27) the exponent form is used.
    EngineeringSi(u8),
    /// Decimal notation with a comma between thousands (`1,234,567.50`), this many digits after
    /// the point (Origin's `Decimal:1000`).
    DecimalGrouped(u8),
    /// The whole part of the number is a month (1 = January, repeating every 12), shown by name;
    /// zero, negative numbers and numbers beyond a 32-bit integer show nothing.
    Month(NameStyle),
    /// The whole part of the number is a day of the week (0 and 7 = Sunday, 1 = Monday, 6 =
    /// Saturday, repeating every 7), shown by name; negative numbers show nothing.
    Weekday(NameStyle),
    /// The number is a date (days, see `datetime`), shown as text such as `2026-11-11`.
    Date(DatePattern),
    /// The number is a time of day (a fraction of a day), shown as `13:45:10`.
    Time(TimePattern),
    /// The number is a date with a time of day, shown as `2026-11-11 13:45:10`.
    DateTime(DatePattern, TimePattern),
}

impl NumberFormat {
    /// Formats one value. NaN (a missing value) is shown as `--`.
    pub fn format(&self, v: f64) -> String {
        if v.is_nan() {
            return "--".to_string();
        }
        if v.is_infinite() {
            return if v > 0.0 { "inf".into() } else { "-inf".into() };
        }
        match *self {
            NumberFormat::General => format_general(v, 9),
            NumberFormat::Decimal(d) => format!("{:.*}", usize::from(d), v),
            NumberFormat::Scientific(d) => format_scientific(v, usize::from(d)),
            NumberFormat::Significant(d) => format_general(v, usize::from(d.max(1))),
            NumberFormat::Engineering(d) => format_engineering(v, usize::from(d), false),
            NumberFormat::EngineeringSi(d) => format_engineering(v, usize::from(d), true),
            NumberFormat::DecimalGrouped(d) => group_thousands(&format!("{:.*}", usize::from(d), v)),
            // Origin's measured rule: the whole part counts, 1 is January and 13 is January again;
            // zero, negative numbers and numbers beyond a 32-bit integer show nothing.
            NumberFormat::Month(style) => match whole_part(v) {
                Some(m) if m >= 1 => style.apply(datetime::MONTHS[((m - 1) % 12) as usize]),
                _ => String::new(),
            },
            // 0 and 7 are Sunday, 1 is Monday and 6 is Saturday, repeating every 7; negative
            // numbers show nothing (`WEEKDAYS` starts with Monday).
            NumberFormat::Weekday(style) => match whole_part(v) {
                Some(d) if d >= 0 => style.apply(datetime::WEEKDAYS[((d + 6) % 7) as usize]),
                _ => String::new(),
            },
            NumberFormat::Time(t) if t.is_elapsed() => datetime::format_time(v, t),
            NumberFormat::Date(_) | NumberFormat::DateTime(..) if !datetime::in_range(v) => format_general(v, 9),
            NumberFormat::Date(d) => datetime::format_date(v, d),
            NumberFormat::Time(t) => datetime::format_time(datetime::time_only_fraction(v), t),
            NumberFormat::DateTime(d, t) => datetime::format_date_time(v, d, t),
        }
    }

    /// True for the date, time and date-time formats.
    pub fn is_date_time(&self) -> bool {
        matches!(self, NumberFormat::Date(_) | NumberFormat::Time(_) | NumberFormat::DateTime(..))
    }
}

/// The whole part of `v` (cut toward zero) when it fits a 32-bit signed integer, as in Origin's
/// Month and Day of Week columns.
fn whole_part(v: f64) -> Option<i64> {
    let t = v.trunc();
    (t >= f64::from(i32::MIN) && t <= f64::from(i32::MAX)).then_some(t as i64)
}

/// Puts a comma between groups of three digits of the integer part of a plain decimal text.
fn group_thousands(s: &str) -> String {
    let (sign, rest) = match s.strip_prefix('-') {
        Some(r) => ("-", r),
        None => ("", s),
    };
    let (int, frac) = rest.split_once('.').map_or((rest, None), |(a, b)| (a, Some(b)));
    let mut out = String::from(sign);
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if let Some(f) = frac {
        out.push('.');
        out.push_str(f);
    }
    out
}

/// SI prefixes from 1E-24 (yocto) to 1E24 (yotta), in steps of 3 in the exponent.
const SI_PREFIXES: [&str; 17] =
    ["y", "z", "a", "f", "p", "n", "\u{b5}", "m", "", "k", "M", "G", "T", "P", "E", "Z", "Y"];

/// Engineering notation: the mantissa is from 1 up to 1000 and the exponent a multiple of 3. With
/// `si` the exponent is the SI prefix letter where one exists.
pub fn format_engineering(v: f64, digits: usize, si: bool) -> String {
    if v == 0.0 {
        return format!("{:.*}{}", digits, 0.0, if si { "" } else { "E0" });
    }
    // Round to the digits first so that 999.96 with 1 digit becomes 1.0k, not 1000.0.
    let mut exp = (v.abs().log10().floor() as i32).div_euclid(3) * 3;
    let mut mantissa = v / 10f64.powi(exp);
    let scale = 10f64.powi(digits as i32);
    if (mantissa.abs() * scale).round() / scale >= 1000.0 {
        exp += 3;
        mantissa = v / 10f64.powi(exp);
    }
    let text = format!("{mantissa:.digits$}");
    let idx = exp / 3 + 8;
    if si && (0..SI_PREFIXES.len() as i32).contains(&idx) {
        format!("{text}{}", SI_PREFIXES[idx as usize])
    } else {
        format!("{text}E{exp}")
    }
}

/// Formats with at most `sig` significant digits and no trailing zeros.
/// Magnitudes below 1e-4 or at or above 1e`sig` use scientific notation
/// such as `1.5E-5`.
pub fn format_general(v: f64, sig: usize) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    if !v.is_finite() {
        return v.to_string();
    }
    let sig = sig.clamp(1, 17);
    // Round to `sig` significant digits first, then decide on the notation,
    // so that 9.9999999999 with sig = 3 becomes "10" and not "1E1".
    let sci = format!("{:.*e}", sig - 1, v);
    let (mantissa, exp) = sci.split_once('e').expect("exponent form");
    let exp: i32 = exp.parse().expect("integer exponent");
    if exp < -4 || exp >= sig as i32 {
        let m = trim_zeros(mantissa);
        format!("{m}E{exp}")
    } else {
        let decimals = (sig as i32 - 1 - exp).max(0) as usize;
        trim_zeros(&format!("{v:.decimals$}")).to_string()
    }
}

/// The text Origin writes for a number when it exports with full precision (measured with `expASC`,
/// `docs/spec/import-ascii.md` section 13): 15 significant digits, trailing zeros dropped, plain
/// notation from 1E-3 up to 1E15 (inclusive), scientific notation such as `1.5E-4` or `1.5E16`
/// (capital E, no plus sign, no leading zeros in the exponent) outside that range.
pub fn format_full_precision(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    if !v.is_finite() {
        return if v.is_nan() {
            String::new()
        } else if v > 0.0 {
            "inf".into()
        } else {
            "-inf".into()
        };
    }
    let sci = format!("{v:.14e}");
    let (mantissa, exp) = sci.split_once('e').expect("exponent form");
    let exp: i32 = exp.parse().expect("integer exponent");
    let mantissa = trim_zeros(mantissa);
    let digits_is_one = mantissa.trim_start_matches('-') == "1";
    if !(-3..=15).contains(&exp) || (exp == 15 && !digits_is_one) {
        format!("{mantissa}E{exp}")
    } else {
        let decimals = (14 - exp).max(0) as usize;
        trim_zeros(&format!("{v:.decimals$}")).to_string()
    }
}

/// Scientific notation with `digits` decimals, written like `1.50E-5`.
pub fn format_scientific(v: f64, digits: usize) -> String {
    let s = format!("{v:.digits$e}");
    match s.split_once('e') {
        Some((m, e)) => format!("{m}E{e}"),
        None => s,
    }
}

fn trim_zeros(s: &str) -> &str {
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.') } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn huge_values_in_a_date_format_show_as_plain_numbers() {
        for v in [1e300, -1e300, 1e19, 1e15, -1e15] {
            let a = NumberFormat::Date(DatePattern::Iso).format(v);
            let b = NumberFormat::DateTime(DatePattern::Iso, TimePattern::Hms).format(v);
            let c = NumberFormat::Time(TimePattern::Hms).format(v);
            assert_eq!(a, format_general(v, 9));
            assert_eq!(b, a);
            assert!(!c.is_empty());
        }
        assert_eq!(datetime::parts(1e300).month, datetime::parts(3.7e7).month);
    }

    #[test]
    fn general_format_drops_noise_and_trailing_zeros() {
        assert_eq!(format_general(0.0, 9), "0");
        assert_eq!(format_general(1.0, 9), "1");
        assert_eq!(format_general(0.1 + 0.2, 9), "0.3");
        assert_eq!(format_general(-12.5, 9), "-12.5");
        assert_eq!(format_general(1.2345e-5, 9), "1.2345E-5");
        assert_eq!(format_general(1.5e12, 9), "1.5E12");
        assert_eq!(format_general(123456789.0, 9), "123456789");
        assert_eq!(format_general(9.9999999999, 3), "10");
        assert_eq!(format_general(0.0001, 9), "0.0001");
    }

    #[test]
    fn full_precision_matches_origin_export() {
        // Values and texts measured with expASC in Origin.
        let cases: [(f64, &str); 20] = [
            (123456.789, "123456.789"),
            (0.123456789012345, "0.123456789012345"),
            (1e-7, "1E-7"),
            (-1.23456e-4, "-1.23456E-4"),
            (1e15, "1000000000000000"),
            (1e16, "1E16"),
            (123456789012345678.0, "1.23456789012346E17"),
            (1.0 / 3.0, "0.333333333333333"),
            (0.001, "0.001"),
            (0.00012, "1.2E-4"),
            (123456789012345.0, "123456789012345"),
            (1234567890123456.0, "1.23456789012346E15"),
            (1.5e15, "1.5E15"),
            (12345678.9, "12345678.9"),
            (-1e-10, "-1E-10"),
            (123456.7891234567, "123456.789123457"),
            (2e15, "2E15"),
            (1e300, "1E300"),
            (100.0, "100"),
            (-0.0, "0"),
        ];
        for (v, want) in cases {
            assert_eq!(format_full_precision(v), want, "{v:e}");
        }
    }

    #[test]
    fn fixed_formats() {
        assert_eq!(NumberFormat::Decimal(2).format(7.126), "7.13");
        assert_eq!(NumberFormat::Scientific(2).format(0.000015), "1.50E-5");
        assert_eq!(NumberFormat::Significant(3).format(1234.5), "1.23E3");
        assert_eq!(NumberFormat::General.format(f64::NAN), "--");
    }

    #[test]
    fn date_time_formats_show_text_and_missing_stays_dashes() {
        let v = 2_461_355.573_032_407_6;
        assert_eq!(NumberFormat::DateTime(DatePattern::Iso, TimePattern::Hms).format(v), "2026-11-11 13:45:10");
        assert_eq!(NumberFormat::Date(DatePattern::Mdy).format(v), "11/11/2026");
        assert_eq!(NumberFormat::Time(TimePattern::Hm).format(0.572_916_666_666_666_6), "13:45");
        // A time-only cell ignores a whole-day part.
        assert_eq!(NumberFormat::Time(TimePattern::Hms).format(v), "13:45:10");
        assert_eq!(NumberFormat::Date(DatePattern::Iso).format(f64::NAN), "--");
        assert!(NumberFormat::Time(TimePattern::Hm).is_date_time());
        assert!(!NumberFormat::General.is_date_time());
    }

    #[test]
    fn old_project_files_and_new_formats_serialize() {
        let old: NumberFormat = serde_json::from_str(r#"{"kind":"decimal","digits":2}"#).unwrap();
        assert_eq!(old, NumberFormat::Decimal(2));
        let none: NumberFormat = serde_json::from_str(r#"{"kind":"general"}"#).unwrap();
        assert_eq!(none, NumberFormat::General);
        for f in [
            NumberFormat::Date(DatePattern::DMonY),
            NumberFormat::Time(TimePattern::HmsMs),
            NumberFormat::DateTime(DatePattern::Iso, TimePattern::Hm),
        ] {
            let json = serde_json::to_string(&f).unwrap();
            assert_eq!(serde_json::from_str::<NumberFormat>(&json).unwrap(), f, "{json}");
        }
    }

    #[test]
    fn engineering_formats_use_exponents_in_steps_of_three() {
        assert_eq!(NumberFormat::Engineering(1).format(15000.0), "15.0E3");
        assert_eq!(NumberFormat::Engineering(2).format(0.00123), "1.23E-3");
        assert_eq!(NumberFormat::Engineering(0).format(123456.0), "123E3");
        assert_eq!(NumberFormat::Engineering(1).format(999.96), "1.0E3");
        assert_eq!(NumberFormat::Engineering(1).format(-4.2), "-4.2E0");
        assert_eq!(NumberFormat::Engineering(1).format(0.0), "0.0E0");
        assert_eq!(NumberFormat::EngineeringSi(1).format(15000.0), "15.0k");
        assert_eq!(NumberFormat::EngineeringSi(2).format(2.5e-3), "2.50m");
        assert_eq!(NumberFormat::EngineeringSi(1).format(3.3e6), "3.3M");
        assert_eq!(NumberFormat::EngineeringSi(1).format(2e-9), "2.0n");
        assert_eq!(NumberFormat::EngineeringSi(1).format(47.0), "47.0");
        assert_eq!(NumberFormat::EngineeringSi(1).format(1e30), "1.0E30");
        assert_eq!(NumberFormat::EngineeringSi(1).format(f64::NAN), "--");
    }

    #[test]
    fn decimal_with_thousands_separators() {
        let f = NumberFormat::DecimalGrouped(2);
        assert_eq!(f.format(1234567.5), "1,234,567.50");
        assert_eq!(f.format(-1000.0), "-1,000.00");
        assert_eq!(f.format(999.0), "999.00");
        assert_eq!(NumberFormat::DecimalGrouped(0).format(1000.0), "1,000");
        assert_eq!(NumberFormat::DecimalGrouped(1).format(100000.0), "100,000.0");
    }

    #[test]
    fn month_and_weekday_formats_show_names() {
        assert_eq!(NumberFormat::Month(NameStyle::Short).format(1.0), "Jan");
        assert_eq!(NumberFormat::Month(NameStyle::Full).format(11.0), "November");
        assert_eq!(NumberFormat::Month(NameStyle::Letter).format(12.0), "D");
        assert!(!NumberFormat::Weekday(NameStyle::Full).is_date_time());
        assert!(!NumberFormat::Month(NameStyle::Short).is_date_time());
        let json = serde_json::to_string(&NumberFormat::Weekday(NameStyle::Full)).unwrap();
        assert_eq!(serde_json::from_str::<NumberFormat>(&json).unwrap(), NumberFormat::Weekday(NameStyle::Full));
    }

    #[test]
    fn month_and_weekday_numbers_follow_origins_measured_rules() {
        // Measured in Origin 2016 (column format Month and Day of Week, docs/spec/workbook-model.md).
        let month = |v: f64| NumberFormat::Month(NameStyle::Short).format(v);
        assert_eq!(
            [1.0, 2.0, 9.0, 12.0, 13.0, 14.0, 15.0].map(month),
            ["Jan", "Feb", "Sep", "Dec", "Jan", "Feb", "Mar"]
        );
        // The whole part counts: 1.5 is January, 6.9 is June, 0.5 is nothing.
        assert_eq!([1.5, 2.5, 6.9, 12.99].map(month), ["Jan", "Feb", "Jun", "Dec"]);
        for v in [0.0, 0.5, -0.5, -1.0, -8.0] {
            assert_eq!(month(v), "", "{v}");
        }
        // The cycle goes on to large numbers: 100 and 1E6 and 1E9 are April, 2147483647 is July.
        assert_eq!([100.0, 1e6, 1e9, 2_147_483_647.0].map(month), ["Apr", "Apr", "Apr", "Jul"]);
        // Past a 32-bit integer nothing is shown.
        for v in [2_147_483_648.0, 4_294_967_295.0, 3e9, 1e10, 1e300] {
            assert_eq!(month(v), "", "{v}");
        }
        assert_eq!(NumberFormat::Month(NameStyle::Full).format(8.0), "August");
        assert_eq!(NumberFormat::Month(NameStyle::Letter).format(9.0), "S");

        let day = |v: f64| NumberFormat::Weekday(NameStyle::Short).format(v);
        // 0 and 7 are Sunday, 1 is Monday and 6 is Saturday, repeating every 7.
        assert_eq!(
            [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0].map(day),
            ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
        );
        assert_eq!(
            [8.0, 9.0, 13.0, 14.0, 15.0, 100.0, 1e6].map(day),
            ["Mon", "Tue", "Sat", "Sun", "Mon", "Tue", "Mon"]
        );
        assert_eq!(
            [0.5, 1.5, 2.5, 6.9, 7.5, -0.5, -0.9999].map(day),
            ["Sun", "Mon", "Tue", "Sat", "Sun", "Sun", "Sun"]
        );
        for v in [-1.0, -1.5, -7.0, 2_147_483_648.0, 1e10, 1e19] {
            assert_eq!(day(v), "", "{v}");
        }
        assert_eq!(NumberFormat::Weekday(NameStyle::Full).format(3.0), "Wednesday");
        assert_eq!(NumberFormat::Weekday(NameStyle::Letter).format(4.0), "T");
        assert_eq!(NumberFormat::Weekday(NameStyle::Letter).format(6.0), "S");
    }

    #[test]
    fn every_date_pattern_shows_origins_text_for_a_fixed_value() {
        // 2026-11-11 13:45:10 (a Wednesday); the texts are Origin's subformats (workbook-model.md).
        let v = 2_461_355.573_032_407_6;
        let want = [
            (DatePattern::Mdy, "11/11/2026"),
            (DatePattern::MdyDash, "11-11-2026"),
            (DatePattern::Dmy, "11/11/2026"),
            (DatePattern::DmyDash, "11-11-2026"),
            (DatePattern::DmyDot, "11.11.2026"),
            (DatePattern::Iso, "2026-11-11"),
            (DatePattern::DMonY, "11-Nov-2026"),
            (DatePattern::LongDate, "Wednesday, November 11, 2026"),
            (DatePattern::MonthDay, "Nov 11"),
            (DatePattern::MonthDayNum, "11/11"),
            (DatePattern::DayOnly, "11"),
            (DatePattern::WeekdayShort, "Wed"),
            (DatePattern::WeekdayLetter, "W"),
            (DatePattern::YearOnly, "2026"),
            (DatePattern::Year2, "26"),
            (DatePattern::Yymmdd, "261111"),
            (DatePattern::MonthShort, "Nov"),
            (DatePattern::MonthLetter, "N"),
            (DatePattern::Quarter, "Q4"),
        ];
        assert_eq!(want.len(), DatePattern::ALL.len());
        for (p, text) in want {
            assert_eq!(NumberFormat::Date(p).format(v), text, "{p:?}");
        }
        // Single digits and other quarters.
        let feb5 = datetime::date_value(2026, 2, 5).unwrap();
        assert_eq!(NumberFormat::Date(DatePattern::Mdy).format(feb5), "2/5/2026");
        assert_eq!(NumberFormat::Date(DatePattern::MonthDayNum).format(feb5), "2/5");
        assert_eq!(NumberFormat::Date(DatePattern::Quarter).format(feb5), "Q1");
        assert_eq!(NumberFormat::Date(DatePattern::Year2).format(datetime::date_value(2005, 1, 1).unwrap()), "05");
    }

    #[test]
    fn every_time_pattern_shows_origins_text_for_a_fixed_value() {
        // 13:45:10.256 as a fraction of a day.
        let v = (13.0 * 3600.0 + 45.0 * 60.0 + 10.256) / 86_400.0;
        let want = [
            (TimePattern::Hm, "13:45"),
            (TimePattern::Hms, "13:45:10"),
            (TimePattern::HmsMs, "13:45:10.256"),
            (TimePattern::H, "13"),
            (TimePattern::HmsCs, "13:45:10.25"),
            (TimePattern::H12, "01 PM"),
            (TimePattern::Hm12, "01:45 PM"),
            (TimePattern::Ms, "45:10"),
            (TimePattern::MsCs, "45:10.25"),
            (TimePattern::Hhmm, "1345"),
            (TimePattern::Hhmmss, "134510"),
            (TimePattern::Millis, "256"),
            (TimePattern::ElapsedDh, "000:13"),
            (TimePattern::ElapsedDhm, "000:13:45"),
        ];
        assert_eq!(want.len(), TimePattern::ALL.len());
        for (p, text) in want {
            assert_eq!(NumberFormat::Time(p).format(v), text, "{p:?}");
        }
        // Midnight and noon on the 12-hour clock; elapsed time counts whole days.
        assert_eq!(NumberFormat::Time(TimePattern::Hm12).format(0.0), "12:00 AM");
        assert_eq!(NumberFormat::Time(TimePattern::H12).format(0.5), "12 PM");
        assert_eq!(NumberFormat::Time(TimePattern::ElapsedDh).format(2.5), "002:12");
        // A calendar date never shows the serial number as an elapsed day count.
        let s = NumberFormat::DateTime(DatePattern::Iso, TimePattern::ElapsedDh).format(2461355.5);
        assert!(s.ends_with(" 12"), "{s}");
    }
}
