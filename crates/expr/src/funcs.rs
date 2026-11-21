//! The built-in function table and the scalar maths behind it.
//!
//! Everything here works on single numbers; the vector evaluator maps these over columns and the
//! compiled evaluator calls them directly, so both modes give identical results.

use crate::text::StrFn;
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
    /// Origin's `count`: the number of rows in the range, missing cells included; the column ends
    /// at its last value.
    Rows,
    Min,
    Max,
    First,
    Last,
    /// Population standard deviation (divisor n).
    Sdp,
    /// Standard error of the mean, `sd / sqrt(n)`.
    Sem,
}

/// Functions that turn a whole column into a column of the same length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VecFn {
    Cumsum,
    Diff,
    Shift,
    Rank,
}

pub(crate) type FnN = fn(&[f64]) -> f64;

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
    /// Statistic over a column or a row range of a column.
    Agg(Agg),
    /// Column in, column out (`cumsum`, `diff`, `shift`, `rank`).
    Vec(VecFn),
    /// Elementwise function of a fixed number of numbers (`date`, `time`, `distance`).
    N(FnN, usize),
    /// `now()` and `today()`.
    Clock { date_only: bool },
    /// `rnd()` (uniform) and `grnd()` (normal).
    Rand { gauss: bool },
    /// `lookup(x, xcol, ycol)`.
    Lookup,
    /// `cell(row, col)`.
    Cell,
    /// A string function (`left`, `mid`, `text`, ...).
    Str(StrFn),
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
            _ => Resolved::BadArity { name: "round", expected: "1 or 2 arguments".into() },
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
        Func::U(_) | Func::Agg(_) => (1, 1),
        Func::B(_) => (2, 2),
        Func::MinMax { .. } => (1, usize::MAX),
        Func::IfElse => (3, 3),
        Func::Na | Func::Clock { .. } | Func::Rand { .. } => (0, 0),
        Func::Vec(VecFn::Shift) => (2, 2),
        Func::Vec(_) => (1, 1),
        Func::N(_, k) => (k, k),
        Func::Lookup => (3, 3),
        Func::Cell => (2, 2),
        Func::Str(f) => f.arity(),
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
    ("left", Func::Str(StrFn::Left)),
    ("right", Func::Str(StrFn::Right)),
    ("mid", Func::Str(StrFn::Mid)),
    ("len", Func::Str(StrFn::Len)),
    ("upper", Func::Str(StrFn::Upper)),
    ("lower", Func::Str(StrFn::Lower)),
    ("trim", Func::Str(StrFn::Trim)),
    ("find", Func::Str(StrFn::Find)),
    ("search", Func::Str(StrFn::Search)),
    ("replace", Func::Str(StrFn::Replace)),
    ("substitute", Func::Str(StrFn::Substitute)),
    ("char", Func::Str(StrFn::Char)),
    ("code", Func::Str(StrFn::Code)),
    ("compare", Func::Str(StrFn::Compare)),
    ("exact", Func::Str(StrFn::Exact)),
    ("text", Func::Str(StrFn::Text)),
    ("format", Func::Str(StrFn::Format)),
    ("value", Func::Str(StrFn::Value)),
    ("if", Func::IfElse),
    // Origin's `sum(col)` in a column formula is the running sum (measured), `total` the total.
    ("sum", Func::Vec(VecFn::Cumsum)),
    ("total", Func::Agg(Agg::Sum)),
    ("mean", Func::Agg(Agg::Mean)),
    ("median", Func::Agg(Agg::Median)),
    ("sd", Func::Agg(Agg::Sd)),
    ("stddev", Func::Agg(Agg::Sd)),
    ("var", Func::Agg(Agg::Var)),
    ("count", Func::Agg(Agg::Rows)),
    ("first", Func::Agg(Agg::First)),
    ("last", Func::Agg(Agg::Last)),
    ("stddevp", Func::Agg(Agg::Sdp)),
    ("sem", Func::Agg(Agg::Sem)),
    ("cumsum", Func::Vec(VecFn::Cumsum)),
    ("diff", Func::Vec(VecFn::Diff)),
    ("shift", Func::Vec(VecFn::Shift)),
    ("rank", Func::Vec(VecFn::Rank)),
    ("fact", Func::U(fact)),
    ("date", Func::N(date_fn, 3)),
    ("time", Func::N(time_fn, 3)),
    ("now", Func::Clock { date_only: false }),
    ("today", Func::Clock { date_only: true }),
    ("year", Func::U(year)),
    ("month", Func::U(month)),
    ("day", Func::U(day)),
    ("weekday", Func::U(weekday)),
    ("hour", Func::U(hour)),
    ("minute", Func::U(minute)),
    ("second", Func::U(second)),
    ("rnd", Func::Rand { gauss: false }),
    ("grnd", Func::Rand { gauss: true }),
    ("lookup", Func::Lookup),
    ("cell", Func::Cell),
    ("distance", Func::N(distance, 4)),
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
        // A missing value equals another missing value (so `col(A)==1/0` finds the gaps).
        BinOp::Eq => f64::from(a == b || (a.is_nan() && b.is_nan())),
        BinOp::Ne => f64::from(!(a == b || (a.is_nan() && b.is_nan()))),
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
    if n.is_nan() {
        return NA;
    }
    if !x.is_finite() {
        return x;
    }
    // Infinite digit counts clamp like very large ones: +inf keeps x, -inf rounds to zero.
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
    if r != 0.0 && ((r < 0.0) != (b < 0.0)) { r + b } else { r }
}

/// `a` to the power `b`; zero to a negative power is a division by zero, so it is missing.
/// A missing base or exponent gives missing, even where IEEE `pow` would give 1 (`x^0`, `1^x`).
#[inline]
pub(crate) fn pow(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() || (a == 0.0 && b < 0.0) { NA } else { a.powf(b) }
}

fn sqr(x: f64) -> f64 {
    x * x
}

fn heaviside(x: f64) -> f64 {
    if x.is_nan() { NA } else { f64::from(x >= 0.0) }
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
// Factorial, dates, distance
// ---------------------------------------------------------------------------------------------

/// Factorial of the whole part. Negative, missing or above 170 (overflow) gives missing.
fn fact(x: f64) -> f64 {
    if x.is_nan() || x < 0.0 {
        return NA;
    }
    let n = x.trunc();
    if n > 170.0 {
        return NA;
    }
    let mut r = 1.0;
    let mut k = 2.0;
    while k <= n {
        r *= k;
        k += 1.0;
    }
    r
}

/// Date number of 1970-01-01 (the Julian day number; Origin's date numbers count days from it).
const UNIX_EPOCH_VALUE: f64 = 2_440_587.0;

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        2 if is_leap(y) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// `date(y, m, d)`: the date number at midnight; an impossible date is missing.
fn date_fn(a: &[f64]) -> f64 {
    if a.iter().any(|v| !v.is_finite()) || a[0].abs() > 1.0e6 {
        return NA;
    }
    let (y, m, d) = (a[0].trunc() as i64, a[1].trunc() as i64, a[2].trunc() as i64);
    if !(1..=12).contains(&m) || d < 1 || d > days_in_month(y, m) {
        return NA;
    }
    days_from_civil(y, m, d) as f64 + UNIX_EPOCH_VALUE
}

/// `time(h, m, s)`: the fraction of a day. Not wrapped: `time(25,0,0)` is above 1.
fn time_fn(a: &[f64]) -> f64 {
    (a[0] * 3600.0 + a[1] * 60.0 + a[2]) / 86_400.0
}

/// What `year`, `month`, `day`, `weekday`, `hour`, `minute` and `second` return for a number with no
/// date (or time) and for a missing input: Origin 2016 writes -1 (measured 2026-11-20, T262).
const NO_DATE: f64 = -1.0;
/// The first and the last date number Origin's date parts accept: 0099-12-31 and 9999-12-31 at
/// midnight (measured, T262: 1757582 and 5373483.25 already give -1).
const FIRST_DATE: f64 = 1_757_583.0;
const LAST_DATE: f64 = 5_373_483.0;
/// 1899-12-30 (day 0 of Windows' date numbers): Origin reads the time of a day before it backwards,
/// so that 2299159.25 is 18:00 (measured, T262).
const DATE_ZERO: f64 = 2_415_018.0;

fn civil_of(v: f64) -> Option<(i64, i64, i64)> {
    if !(FIRST_DATE..=LAST_DATE).contains(&v) {
        return None;
    }
    Some(civil_from_days((v - UNIX_EPOCH_VALUE).floor() as i64))
}

fn year(v: f64) -> f64 {
    civil_of(v).map_or(NO_DATE, |c| c.0 as f64)
}

fn month(v: f64) -> f64 {
    civil_of(v).map_or(NO_DATE, |c| c.1 as f64)
}

fn day(v: f64) -> f64 {
    civil_of(v).map_or(NO_DATE, |c| c.2 as f64)
}

/// The time of day of a date-time number in milliseconds, rounded to the nearest one so that a
/// value such as `date(2020,1,1) + 0.75` is 18:00 and not 17:59:59.999... (T239, Sheaf's rule: Origin
/// was not available to measure). A time that rounds up to midnight is the start of the next day.
///
/// Which numbers have a time (measured in Origin 2016, T262): a date in the range of the date
/// parts, and a time alone (a number below 1, also negative), which Origin reads as that time
/// today; a number from 1 up to the first date, or past the last, has none. A day before
/// 1899-12-30 reads its fraction backwards. Because of "today", Origin's answer for a large
/// negative time (below about -46000 the fraction turns, below about -704000 it is -1, in 2026)
/// moves with the current date; Sheaf does the same with the system date.
fn day_millis(v: f64) -> Option<i64> {
    let x = if v < 1.0 { v + system_now().floor() } else { v };
    civil_of(x)?;
    let mut frac = x - x.floor();
    if x < DATE_ZERO && frac > 0.0 {
        frac = 1.0 - frac;
    }
    Some((frac * 86_400_000.0).round() as i64 % 86_400_000)
}

/// Hour of the day of a date-time number, 0 to 23 (T239); -1 when it has no time (T262).
fn hour(v: f64) -> f64 {
    day_millis(v).map_or(NO_DATE, |ms| (ms / 3_600_000) as f64)
}

/// Minute of the hour, 0 to 59 (T239).
fn minute(v: f64) -> f64 {
    day_millis(v).map_or(NO_DATE, |ms| (ms / 60_000 % 60) as f64)
}

/// Second of the minute with its fraction, to the millisecond: 0 to 59.999 (T239).
fn second(v: f64) -> f64 {
    day_millis(v).map_or(NO_DATE, |ms| (ms % 60_000) as f64 / 1000.0)
}

/// Day of the week, 0 = Sunday (measured in Origin); -1 without a date (T262).
fn weekday(v: f64) -> f64 {
    if civil_of(v).is_none() {
        return NO_DATE;
    }
    // 1970-01-01 was a Thursday (4).
    ((v - UNIX_EPOCH_VALUE).floor() as i64 + 4).rem_euclid(7) as f64
}

/// A seed from the system clock.
pub(crate) fn system_seed() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64)
}

/// The current date number from the system clock, in UTC.
pub(crate) fn system_now() -> f64 {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64());
    secs / 86_400.0 + UNIX_EPOCH_VALUE
}

/// `distance(x1, y1, x2, y2)`: Euclidean distance.
fn distance(a: &[f64]) -> f64 {
    (a[2] - a[0]).hypot(a[3] - a[1])
}

// ---------------------------------------------------------------------------------------------
// Whole-column functions and random numbers
// ---------------------------------------------------------------------------------------------

/// Applies a column-in, column-out function. `n` is the shift for [`VecFn::Shift`].
pub(crate) fn vec_fn(kind: VecFn, x: &[f64], n: f64) -> Vec<f64> {
    match kind {
        VecFn::Cumsum => {
            // The running sum skips missing values; a missing input row stays missing.
            let mut acc = 0.0;
            let mut comp = 0.0;
            x.iter()
                .map(|&v| {
                    if v.is_nan() {
                        return NA;
                    }
                    let t = acc + v;
                    if !t.is_finite() {
                        comp = 0.0;
                    } else if acc.abs() >= v.abs() {
                        comp += (acc - t) + v;
                    } else {
                        comp += (v - t) + acc;
                    }
                    acc = t;
                    acc + comp
                })
                .collect()
        }
        // Forward difference: row i holds x[i+1] - x[i]; the last row is missing.
        VecFn::Diff => (0..x.len()).map(|i| x.get(i + 1).map_or(NA, |next| next - x[i])).collect(),
        VecFn::Shift => {
            if n.is_nan() {
                return vec![NA; x.len()];
            }
            let k = n.round().clamp(-4.0e15, 4.0e15) as i64;
            (0..x.len() as i64)
                .map(|i| {
                    let src = i - k;
                    if src >= 0 && (src as usize) < x.len() { x[src as usize] } else { NA }
                })
                .collect()
        }
        VecFn::Rank => {
            // Ascending; tied values share the average of their ranks; missing stays missing.
            let mut idx: Vec<usize> = (0..x.len()).filter(|&i| !x[i].is_nan()).collect();
            idx.sort_by(|&a, &b| x[a].total_cmp(&x[b]));
            let mut out = vec![NA; x.len()];
            let mut s = 0;
            while s < idx.len() {
                let mut e = s;
                while e + 1 < idx.len() && x[idx[e + 1]] == x[idx[s]] {
                    e += 1;
                }
                let r = (s + e) as f64 / 2.0 + 1.0;
                for &i in &idx[s..=e] {
                    out[i] = r;
                }
                s = e + 1;
            }
            out
        }
    }
}

/// Next value of a splitmix64 generator.
pub(crate) fn next_u64(state: &std::cell::Cell<u64>) -> u64 {
    let s = state.get().wrapping_add(0x9E37_79B9_7F4A_7C15);
    state.set(s);
    let mut z = s;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Uniform in [0, 1).
pub(crate) fn uniform(state: &std::cell::Cell<u64>) -> f64 {
    (next_u64(state) >> 11) as f64 / (1u64 << 53) as f64
}

/// Standard normal by the Box-Muller transform.
pub(crate) fn normal(state: &std::cell::Cell<u64>) -> f64 {
    let u1 = 1.0 - uniform(state);
    let u2 = uniform(state);
    (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos()
}

// ---------------------------------------------------------------------------------------------
// Aggregates and cumulative functions
// ---------------------------------------------------------------------------------------------

/// Sum of the non-missing values (Neumaier compensated summation) and their count.
/// With an infinite value the plain sum is returned (infinite, or missing for `inf - inf`),
/// because the compensation term would turn it into a missing value.
fn sum_count(data: &[f64]) -> (f64, usize) {
    if data.iter().any(|x| x.is_infinite()) {
        let valid = data.iter().filter(|x| !x.is_nan());
        return (valid.clone().sum::<f64>(), valid.count());
    }
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
        Agg::Rows => data.len() as f64,
        Agg::Mean => {
            let (s, n) = sum_count(data);
            if n == 0 { NA } else { s / n as f64 }
        }
        Agg::Var | Agg::Sd | Agg::Sdp | Agg::Sem => {
            let (s, n) = sum_count(data);
            if n < 2 && !(kind == Agg::Sdp && n == 1) {
                return NA;
            }
            let mean = s / n as f64;
            if !mean.is_finite() {
                // An infinite value makes the spread undefined, not zero.
                return NA;
            }
            let dev: Vec<f64> = data.iter().filter(|x| !x.is_nan()).map(|x| (x - mean) * (x - mean)).collect();
            let ss = sum_count(&dev).0;
            match kind {
                Agg::Var => ss / (n - 1) as f64,
                Agg::Sd => (ss / (n - 1) as f64).sqrt(),
                Agg::Sdp => (ss / n as f64).sqrt(),
                _ => (ss / (n - 1) as f64).sqrt() / (n as f64).sqrt(),
            }
        }
        Agg::Median => {
            let mut v: Vec<f64> = data.iter().copied().filter(|x| !x.is_nan()).collect();
            if v.is_empty() {
                return NA;
            }
            v.sort_by(f64::total_cmp);
            let m = v.len() / 2;
            if v.len() % 2 == 1 { v[m] } else { 0.5 * (v[m - 1] + v[m]) }
        }
        Agg::Min => data.iter().copied().filter(|x| !x.is_nan()).reduce(f64::min).unwrap_or(NA),
        Agg::Max => data.iter().copied().filter(|x| !x.is_nan()).reduce(f64::max).unwrap_or(NA),
        Agg::First => data.iter().copied().find(|x| !x.is_nan()).unwrap_or(NA),
        Agg::Last => data.iter().rev().copied().find(|x| !x.is_nan()).unwrap_or(NA),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) {
        let scale = b.abs().max(1e-300);
        assert!((a - b).abs() <= tol * scale.max(if b == 0.0 { 1.0 } else { 0.0 }), "{a} vs {b}");
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
