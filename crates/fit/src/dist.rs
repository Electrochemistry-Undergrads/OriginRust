//! Probabilities of the t and F distributions used by the fit reports.

use statrs::distribution::{ContinuousCDF, FisherSnedecor, StudentsT};

/// Two-sided p-value of a t statistic with `dof` degrees of freedom: `P(|T| >= |t|)`.
/// NaN if `t` is NaN or `dof` is zero.
pub fn t_two_sided_p(t: f64, dof: usize) -> f64 {
    if t.is_nan() || dof == 0 {
        return f64::NAN;
    }
    if t.is_infinite() {
        return 0.0;
    }
    match StudentsT::new(0.0, 1.0, dof as f64) {
        Ok(d) => (2.0 * d.sf(t.abs())).min(1.0),
        Err(_) => f64::NAN,
    }
}

/// The t value that leaves `(1 - level) / 2` in each tail, used for confidence limits
/// (`value ± t * std_error`). NaN if `dof` is zero or `level` is not inside (0, 1).
pub fn t_critical(level: f64, dof: usize) -> f64 {
    if dof == 0 || !(level > 0.0 && level < 1.0) {
        return f64::NAN;
    }
    match StudentsT::new(0.0, 1.0, dof as f64) {
        Ok(d) => d.inverse_cdf(0.5 + level / 2.0),
        Err(_) => f64::NAN,
    }
}

/// Upper-tail probability of an F statistic: `P(F >= f)` with `(df1, df2)` degrees of freedom.
pub fn f_upper_p(f: f64, df1: usize, df2: usize) -> f64 {
    if f.is_nan() || df1 == 0 || df2 == 0 {
        return f64::NAN;
    }
    if f.is_infinite() {
        return 0.0;
    }
    match FisherSnedecor::new(df1 as f64, df2 as f64) {
        Ok(d) => d.sf(f.max(0.0)),
        Err(_) => f64::NAN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_values_match_tables() {
        // Two-sided 95 % critical values from standard tables.
        assert!((t_critical(0.95, 10) - 2.228_138_851_986_272).abs() < 1e-9);
        assert!((t_critical(0.95, 1) - 12.706_204_736_174_7).abs() < 1e-6);
        assert!((t_two_sided_p(2.228_138_851_986_272, 10) - 0.05).abs() < 1e-9);
        assert!(t_two_sided_p(f64::NAN, 3).is_nan());
        assert!(t_critical(0.95, 0).is_nan());
    }

    #[test]
    fn f_upper_tail() {
        // F(1, 10) at 4.964602743730711 leaves 5 % in the upper tail.
        assert!((f_upper_p(4.964_602_743_730_711, 1, 10) - 0.05).abs() < 1e-9);
        assert_eq!(f_upper_p(f64::INFINITY, 2, 5), 0.0);
    }
}
