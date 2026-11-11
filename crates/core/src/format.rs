//! Turning numbers into text for worksheet cells and tick labels.

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
        }
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
}
