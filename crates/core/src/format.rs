//! Turning numbers into text for worksheet cells and tick labels.

use crate::datetime::{self, DatePattern, TimePattern};
use serde::{Deserialize, Serialize};

/// How a numeric column shows its values.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "kind", content = "digits")]
pub enum NumberFormat {
    /// Up to 9 significant digits, switching to scientific notation for very
    /// large or very small magnitudes. Trailing zeros are dropped.
    #[default]
    General,
    /// A fixed number of digits after the decimal point.
    Decimal(u8),
    /// Scientific notation with a fixed number of digits after the decimal point.
    Scientific(u8),
    /// A fixed number of significant digits.
    Significant(u8),
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
}
