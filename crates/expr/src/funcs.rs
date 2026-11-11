//! The built-in function table and the scalar maths behind it.
//!
//! Everything here works on single numbers; the vector evaluator maps these over columns and the
//! compiled evaluator calls them directly, so both modes give identical results.

use std::f64::consts::PI;

/// The missing value.
pub(crate) const NA: f64 = f64::NAN;

pub(crate) type F1 = fn(f64) -> f64;
pub(crate) type F2 = fn(f64, f64) -> f64;

/// Aggregates reduce a vector to one number, ignoring missing values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Agg {
    Sum,
    Mean,
    Median,
    Sd,
    Var,
    Count,
    Min,
    Max,
    First,
    Last,
}

/// Vector-to-vector functions that look at neighbouring rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cum {
    Sum,
    Diff,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Func {
    /// Elementwise function of one number.
    U(F1),
    /// Elementwise function of two numbers.
    B(F2),
    /// `min`/`max` with two or more arguments (elementwise); one argument is an aggregate.
    MinMax { max: bool },
    /// `ifelse(c, a, b)`.
    IfElse,
    /// `na()`.
    Na,
    Agg(Agg),
    Cum(Cum),
    Shift,
}

pub(crate) enum Resolved {
    Found(Func, &'static str),
    Unknown,
    BadArity { name: &'static str, expected: String },
}

/// Looks up a function by (case-insensitive) name and checks the argument count.
pub(crate) fn resolve(name: &str, argc: usize) -> Resolved {
    if name.eq_ignore_ascii_case("round") {
        return match argc {
            1 => Resolved::Found(Func::U(nint), "round"),
            2 => Resolved::Found(Func::B(round_digits), "round"),
            _ => Resolved::BadArity {
                name: "round",
                expected: "1 or 2 arguments".into(),
            },
        };
    }
    for (n, f) in TABLE {
        if name.eq_ignore_ascii_case(n) {
            let (lo, hi) = arity(*f);
            if argc < lo || argc > hi {
                let expected = if hi == usize::MAX {
                    format!("at least {lo} argument{}", if lo == 1 { "" } else { "s" })
                } else {
                    match lo {
                        0 => "no arguments".to_string(),
                        1 => "1 argument".to_string(),
                        k => format!("{k} arguments"),
                    }
                };
                return Resolved::BadArity { name: n, expected };
            }
            return Resolved::Found(*f, n);
        }
    }
    Resolved::Unknown
}

fn arity(f: Func) -> (usize, usize) {
    match f {
        Func::U(_) | Func::Agg(_) | Func::Cum(_) => (1, 1),
        Func::B(_) | Func::Shift => (2, 2),
        Func::MinMax { .. } => (1, usize::MAX),
        Func::IfElse => (3, 3),
        Func::Na => (0, 0),
    }
}

static TABLE: &[(&str, Func)] = &[
    ("abs", Func::U(f64::abs)),
    ("fabs", Func::U(f64::abs)),
    ("sqrt", Func::U(sqrt)),
    ("exp", Func::U(f64::exp)),
    ("ln", Func::U(ln)),
    ("log", Func::U(log10)),
    ("log10", Func::U(log10)),
    ("log2", Func::U(log2)),
    ("sin", Func::U(f64::sin)),
    ("cos", Func::U(f64::cos)),
    ("tan", Func::U(f64::tan)),
    ("asin", Func::U(f64::asin)),
    ("acos", Func::U(f64::acos)),
    ("atan", Func::U(f64::atan)),
    ("atan2", Func::B(f64::atan2)),
    ("sinh", Func::U(f64::sinh)),
    ("cosh", Func::U(f64::cosh)),
    ("tanh", Func::U(f64::tanh)),
    ("asinh", Func::U(f64::asinh)),
    ("acosh", Func::U(f64::acosh)),
    ("atanh", Func::U(f64::atanh)),
    ("degrees", Func::U(f64::to_degrees)),
    ("radians", Func::U(f64::to_radians)),
    ("int", Func::U(f64::trunc)),
    ("nint", Func::U(nint)),
    ("floor", Func::U(f64::floor)),
    ("ceil", Func::U(f64::ceil)),
    ("sign", Func::U(sign)),
    ("mod", Func::B(rem)),
    ("rmod", Func::B(rmod)),
    ("min", Func::MinMax { max: false }),
    ("max", Func::MinMax { max: true }),
    ("pow", Func::B(pow)),
    ("sqr", Func::U(sqr)),
    ("erf", Func::U(erf)),
    ("erfc", Func::U(erfc)),
    ("gamma", Func::U(gamma)),
    ("lgamma", Func::U(lgamma)),
    ("beta", Func::B(beta)),
    ("j0", Func::U(bessel_j0)),
    ("j1", Func::U(bessel_j1)),
    ("heaviside", Func::U(heaviside)),
    ("isna", Func::U(isna)),
    ("na", Func::Na),
    ("ifelse", Func::IfElse),
    ("if", Func::IfElse),
    ("sum", Func::Agg(Agg::Sum)),
    ("total", Func::Agg(Agg::Sum)),
    ("mean", Func::Agg(Agg::Mean)),
    ("median", Func::Agg(Agg::Median)),
    ("sd", Func::Agg(Agg::Sd)),
    ("stddev", Func::Agg(Agg::Sd)),
    ("var", Func::Agg(Agg::Var)),
    ("count", Func::Agg(Agg::Count)),
    ("first", Func::Agg(Agg::First)),
    ("last", Func::Agg(Agg::Last)),
    ("cumsum", Func::Cum(Cum::Sum)),
    ("diff", Func::Cum(Cum::Diff)),
    ("shift", Func::Shift),
];

// ---------------------------------------------------------------------------------------------
// Operators
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    And,
    Or,
}

/// True for any number that is neither zero nor missing.
#[inline(always)]
pub(crate) fn truthy(x: f64) -> bool {
    x != 0.0 && !x.is_nan()
}

#[inline(always)]
pub(crate) fn apply_bin(op: BinOp, a: f64, b: f64) -> f64 {
    match op {
        BinOp::Add => a + b,
        BinOp::Sub => a - b,
        BinOp::Mul => a * b,
        BinOp::Div => {
            if b == 0.0 {
                NA
            } else {
                a / b
            }
        }
        BinOp::Rem => rem(a, b),
        BinOp::Pow => pow(a, b),
        BinOp::Lt => f64::from(a < b),
        BinOp::Le => f64::from(a <= b),
        BinOp::Gt => f64::from(a > b),
        BinOp::Ge => f64::from(a >= b),
        BinOp::Eq => f64::from(a == b),
        BinOp::Ne => f64::from(!a.is_nan() && !b.is_nan() && a != b),
        BinOp::And => f64::from(truthy(a) && truthy(b)),
        BinOp::Or => f64::from(truthy(a) || truthy(b)),
    }
}

/// `min`/`max` of two numbers; a missing value on either side gives missing.
#[inline]
pub(crate) fn min2(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() { NA } else { a.min(b) }
}

#[inline]
pub(crate) fn max2(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() { NA } else { a.max(b) }
}

// ---------------------------------------------------------------------------------------------
// Elementary functions with Origin-style missing-value rules
// ---------------------------------------------------------------------------------------------

fn sqrt(x: f64) -> f64 {
    if x < 0.0 { NA } else { x.sqrt() }
}

fn ln(x: f64) -> f64 {
    if x <= 0.0 { NA } else { x.ln() }
}

fn log10(x: f64) -> f64 {
    if x <= 0.0 { NA } else { x.log10() }
}

fn log2(x: f64) -> f64 {
    if x <= 0.0 { NA } else { x.log2() }
}

fn nint(x: f64) -> f64 {
    x.round()
}

fn round_digits(x: f64, n: f64) -> f64 {
    if !x.is_finite() || !n.is_finite() {
        return x;
    }
    let n = n.trunc().clamp(-300.0, 300.0);
    let scale = 10f64.powf(n);
    let r = (x * scale).round() / scale;
    if r.is_finite() { r } else { x }
}

fn sign(x: f64) -> f64 {
    if x.is_nan() {
        NA
    } else if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Remainder with the sign of the dividend; a zero divisor gives missing.
fn rem(a: f64, b: f64) -> f64 {
    if b == 0.0 { NA } else { a % b }
}

/// Remainder with the sign of the divisor (floored modulus); a zero divisor gives missing.
fn rmod(a: f64, b: f64) -> f64 {
    if b == 0.0 {
        return NA;
    }
    let r = a % b;
    if r != 0.0 && ((r < 0.0) != (b < 0.0)) {
        r + b
    } else {
        r
    }
}

/// `a` to the power `b`; zero to a negative power is a division by zero, so it is missing.
#[inline]
pub(crate) fn pow(a: f64, b: f64) -> f64 {
    if a == 0.0 && b < 0.0 { NA } else { a.powf(b) }
}

fn sqr(x: f64) -> f64 {
    x * x
}

fn heaviside(x: f64) -> f64 {
    if x.is_nan() {
        NA
    } else {
        f64::from(x >= 0.0)
    }
}

fn isna(x: f64) -> f64 {
    f64::from(x.is_nan())
}

// ---------------------------------------------------------------------------------------------
// Special functions
// ---------------------------------------------------------------------------------------------

const FRAC_2_SQRT_PI: f64 = std::f64::consts::FRAC_2_SQRT_PI;

/// Error function. Series for small arguments, continued fraction for the tail.
/// Accuracy: relative error below 1e-14 everywhere.
pub(crate) fn erf(x: f64) -> f64 {
    if x.is_nan() {
        return NA;
    }
    let a = x.abs();
    let v = if a < 1.5 {
        erf_series(a)
    } else if a > 6.0 {
        1.0
    } else {
        1.0 - erfc_cf(a)
    };
    if x < 0.0 { -v } else { v }
}

/// Complementary error function `1 - erf(x)`, accurate in the tail (relative error below 1e-14).
pub(crate) fn erfc(x: f64) -> f64 {
    if x.is_nan() {
        return NA;
    }
    if x < 0.0 {
        return 2.0 - erfc(-x);
    }
    if x < 1.5 { 1.0 - erf_series(x) } else { erfc_cf(x) }
}

/// `erf(x) = 2/sqrt(pi) * exp(-x^2) * sum 2^n x^(2n+1) / (2n+1)!!`, all terms positive.
fn erf_series(x: f64) -> f64 {
    let x2 = x * x;
    let mut term = x;
    let mut sum = x;
    let mut k = 0.0;
    while term > 1e-18 * sum && k < 200.0 {
        k += 1.0;
        term *= 2.0 * x2 / (2.0 * k + 1.0);
        sum += term;
    }
    FRAC_2_SQRT_PI * (-x2).exp() * sum
}

/// Continued fraction `erfc(x) = exp(-x^2)/sqrt(pi) / (x + (1/2)/(x + (2/2)/(x + ...)))`, x >= 1.5.
fn erfc_cf(x: f64) -> f64 {
    if x > 27.0 {
        return 0.0;
    }
    let depth = if x > 4.0 { 40 } else { 140 };
    let mut t = x;
    for k in (1..=depth).rev() {
        t = x + (f64::from(k) / 2.0) / t;
    }
    (-x * x).exp() / (PI.sqrt() * t)
}

const LANCZOS_G: f64 = 7.0;
const LANCZOS: [f64; 9] = [
    0.999_999_999_999_809_9,
    676.520_368_121_885_1,
    -1_259.139_216_722_402_8,
    771.323_428_777_653_1,
    -176.615_029_162_140_6,
    12.507_343_278_686_905,
    -0.138_571_095_265_720_12,
    9.984_369_578_019_572e-6,
    1.505_632_735_149_311_6e-7,
];

fn lanczos_sum(x: f64) -> f64 {
    // x here is (argument - 1).
    let mut a = LANCZOS[0];
    for (i, c) in LANCZOS.iter().enumerate().skip(1) {
        a += c / (x + i as f64);
    }
    a
}

/// Gamma function. Whole numbers up to 171 are exact products; elsewhere a Lanczos approximation
/// with reflection for negative arguments (relative error about 1e-15 for moderate arguments).
/// Poles (zero and negative whole numbers) give missing; overflow gives infinity.
pub(crate) fn gamma(x: f64) -> f64 {
    if x.is_nan() {
        return NA;
    }
    if x == x.floor() {
        if x <= 0.0 {
            return NA;
        }
        if x > 171.0 {
            return f64::INFINITY;
        }
        let mut r = 1.0;
        let mut k = 2.0;
        while k < x {
            r *= k;
            k += 1.0;
        }
        return r;
    }
    if x < 0.5 {
        return PI / ((PI * x).sin() * gamma(1.0 - x));
    }
    let z = x - 1.0;
    let t = z + LANCZOS_G + 0.5;
    // Split the power in two halves so that large arguments do not overflow early.
    let half = t.powf((z + 0.5) / 2.0);
    (2.0 * PI).sqrt() * lanczos_sum(z) * half * (-t).exp() * half
}

/// Natural log of the absolute value of the gamma function.
/// Poles give missing. Absolute error about 1e-15 near the zeros at 1 and 2.
pub(crate) fn lgamma(x: f64) -> f64 {
    if x.is_nan() {
        return NA;
    }
    if x == x.floor() && x <= 0.0 {
        return NA;
    }
    if x == 1.0 || x == 2.0 {
        return 0.0;
    }
    if x < 0.5 {
        return (PI / (PI * x).sin().abs()).ln() - lgamma(1.0 - x);
    }
    let z = x - 1.0;
    let t = z + LANCZOS_G + 0.5;
    0.5 * (2.0 * PI).ln() + (z + 0.5) * t.ln() - t + lanczos_sum(z).ln()
}

/// Beta function `gamma(a) gamma(b) / gamma(a+b)` for positive `a`, `b`; otherwise missing.
fn beta(a: f64, b: f64) -> f64 {
    if !(a > 0.0 && b > 0.0) {
        return NA;
    }
    (lgamma(a) + lgamma(b) - lgamma(a + b)).exp()
}

/// Bessel function of the first kind of order 0 or 1.
///
/// Below |x| = 50 the periodic trapezoid rule on `J_n(x) = 1/(2 pi) * integral of
/// cos(n t - x sin t)`, which converges geometrically; above, the Hankel asymptotic series.
fn bessel(order: u32, x: f64) -> f64 {
    if x.is_nan() {
        return NA;
    }
    let a = x.abs();
    let v = if a < 50.0 {
        let n = a as usize + 48;
        let step = 2.0 * PI / n as f64;
        let nu = f64::from(order);
        let mut s = 0.0;
        for k in 0..n {
            let t = step * k as f64;
            s += (nu * t - a * t.sin()).cos();
        }
        s / n as f64
    } else if a.is_infinite() {
        0.0
    } else {
        let mu = 4.0 * f64::from(order * order);
        let mut p = 1.0;
        let mut q = 0.0;
        let mut term = 1.0;
        for k in 1..=14u32 {
            let odd = f64::from(2 * k - 1);
            term *= (mu - odd * odd) / (f64::from(k) * 8.0 * a);
            // Terms alternate in the pattern: q +, p -, q -, p +, ...
            match k % 4 {
                1 => q += term,
                2 => p -= term,
                3 => q -= term,
                _ => p += term,
            }
        }
        let chi = a - (f64::from(order) / 2.0 + 0.25) * PI;
        (2.0 / (PI * a)).sqrt() * (p * chi.cos() - q * chi.sin())
    };
    if order == 1 && x < 0.0 { -v } else { v }
}

fn bessel_j0(x: f64) -> f64 {
    bessel(0, x)
}

fn bessel_j1(x: f64) -> f64 {
    bessel(1, x)
}

// ---------------------------------------------------------------------------------------------
// Aggregates and cumulative functions
// ---------------------------------------------------------------------------------------------

/// Sum of the non-missing values (Neumaier compensated summation) and their count.
fn sum_count(data: &[f64]) -> (f64, usize) {
    let mut sum = 0.0;
    let mut comp = 0.0;
    let mut n = 0;
    for &x in data {
        if x.is_nan() {
            continue;
        }
        n += 1;
        let t = sum + x;
        if sum.abs() >= x.abs() {
            comp += (sum - t) + x;
        } else {
            comp += (x - t) + sum;
        }
        sum = t;
    }
    (sum + comp, n)
}

/// Reduces `data` to one number, ignoring missing values.
pub(crate) fn aggregate(kind: Agg, data: &[f64]) -> f64 {
    match kind {
        Agg::Sum => sum_count(data).0,
        Agg::Count => sum_count(data).1 as f64,
        Agg::Mean => {
            let (s, n) = sum_count(data);
            if n == 0 { NA } else { s / n as f64 }
        }
        Agg::Var | Agg::Sd => {
            let (s, n) = sum_count(data);
            if n < 2 {
                return NA;
            }
            let mean = s / n as f64;
            let dev: Vec<f64> = data
                .iter()
                .filter(|x| !x.is_nan())
                .map(|x| (x - mean) * (x - mean))
                .collect();
            let var = sum_count(&dev).0 / (n - 1) as f64;
            if kind == Agg::Sd { var.sqrt() } else { var }
        }
        Agg::Median => {
            let mut v: Vec<f64> = data.iter().copied().filter(|x| !x.is_nan()).collect();
            if v.is_empty() {
                return NA;
            }
            v.sort_by(f64::total_cmp);
            let m = v.len() / 2;
            if v.len() % 2 == 1 {
                v[m]
            } else {
                0.5 * (v[m - 1] + v[m])
            }
        }
        Agg::Min => data
            .iter()
            .copied()
            .filter(|x| !x.is_nan())
            .reduce(f64::min)
            .unwrap_or(NA),
        Agg::Max => data
            .iter()
            .copied()
            .filter(|x| !x.is_nan())
            .reduce(f64::max)
            .unwrap_or(NA),
        Agg::First => data.iter().copied().find(|x| !x.is_nan()).unwrap_or(NA),
        Agg::Last => data
            .iter()
            .rev()
            .copied()
            .find(|x| !x.is_nan())
            .unwrap_or(NA),
    }
}

/// Running sum (a missing row stays missing but does not stop the running total) or first
/// differences (`v[k+1] - v[k]`, last row missing).
pub(crate) fn cumulative(kind: Cum, data: &[f64]) -> Vec<f64> {
    match kind {
        Cum::Sum => {
            let mut total = 0.0;
            data.iter()
                .map(|&x| {
                    if x.is_nan() {
                        NA
                    } else {
                        total += x;
                        total
                    }
                })
                .collect()
        }
        Cum::Diff => {
            let mut out: Vec<f64> = data.windows(2).map(|w| w[1] - w[0]).collect();
            if !data.is_empty() {
                out.push(NA);
            }
            out
        }
    }
}

/// `out[k] = data[k - n]` (so a positive `n` moves values down); rows without a source are missing.
pub(crate) fn shift(data: &[f64], n: f64) -> Vec<f64> {
    if n.is_nan() || n.abs() > 1e15 {
        return vec![NA; data.len()];
    }
    let n = n.trunc() as i64;
    (0..data.len() as i64)
        .map(|k| {
            let src = k - n;
            if src >= 0 && (src as usize) < data.len() {
                data[src as usize]
            } else {
                NA
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) {
        let scale = b.abs().max(1e-300);
        assert!(
            (a - b).abs() <= tol * scale.max(if b == 0.0 { 1.0 } else { 0.0 }),
            "{a} vs {b}"
        );
    }

    #[test]
    fn erf_reference_values() {
        close(erf(0.5), 0.520_499_877_813_046_5, 1e-14);
        close(erf(1.0), 0.842_700_792_949_714_9, 1e-14);
        close(erf(2.0), 0.995_322_265_018_952_7, 1e-14);
        close(erfc(2.0), 0.004_677_734_981_047_266, 1e-13);
        close(erfc(3.0), 2.209_049_699_858_544e-5, 1e-13);
        close(erf(-1.0), -0.842_700_792_949_714_9, 1e-14);
        assert_eq!(erf(0.0), 0.0);
    }

    #[test]
    fn gamma_reference_values() {
        close(gamma(5.0), 24.0, 1e-15);
        close(gamma(0.5), PI.sqrt(), 1e-14);
        close(gamma(1.5), 0.886_226_925_452_758, 1e-14);
        close(gamma(-0.5), -2.0 * PI.sqrt(), 1e-13);
        close(lgamma(10.0), 12.801_827_480_081_469, 1e-14);
        close(lgamma(0.5), 0.572_364_942_924_700_1, 1e-14);
        assert!(gamma(0.0).is_nan());
    }

    #[test]
    fn bessel_reference_values() {
        close(bessel_j0(1.0), 0.765_197_686_557_966_6, 1e-13);
        close(bessel_j1(1.0), 0.440_050_585_744_933_5, 1e-13);
        close(bessel_j0(10.0), -0.245_935_764_451_348_3, 1e-13);
        close(bessel_j1(10.0), 0.043_472_746_168_861_44, 1e-12);
        close(bessel_j0(100.0), 0.019_985_850_304_223_12, 1e-11);
        close(bessel_j1(100.0), -0.077_145_352_014_112_16, 1e-11);
    }
}
