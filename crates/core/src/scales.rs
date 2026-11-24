//! The axis scales other than linear and log10, and the limits and tick lists Origin gives them
//! (graphing spec section 3.1; measured in Origin, see `docs/spec/graphing.md` section 3.4).
//!
//! Every scale is a monotonic map `forward` from data values to a straight "transformed" line; the
//! axis draws that line. The map increases with the data value on every scale (so a reciprocal axis
//! still has larger values at the top).

use crate::graph::ScaleKind;

/// The offset of the offset reciprocal scale, `1 / (x + OFFSET)` (Origin's value, for kelvin).
pub const RECIPROCAL_OFFSET: f64 = 273.14;

/// Percent values at which a probability axis can carry ticks.
pub const PROBABILITY_TICKS: [f64; 21] = [
    0.01, 0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 95.0, 98.0, 99.0, 99.5, 99.9,
    99.99,
];
/// The same for a logit axis (its limits stay within 0.1 and 99.9).
pub const LOGIT_TICKS: [f64; 13] = [0.01, 0.1, 1.0, 5.0, 10.0, 25.0, 50.0, 75.0, 90.0, 95.0, 99.0, 99.9, 99.99];
/// The same for a Weibull axis.
pub const WEIBULL_TICKS: [f64; 11] = [0.001, 0.01, 0.1, 0.5, 1.0, 5.0, 10.0, 50.0, 90.0, 99.0, 99.9];

impl ScaleKind {
    /// Built-in scales in the order of the axis dialog's list.
    pub const ALL: [ScaleKind; 10] = [
        ScaleKind::Linear,
        ScaleKind::Log10,
        ScaleKind::Ln,
        ScaleKind::Log2,
        ScaleKind::Reciprocal,
        ScaleKind::OffsetReciprocal,
        ScaleKind::Probability,
        ScaleKind::Probit,
        ScaleKind::Logit,
        ScaleKind::Weibull,
    ];

    /// The name in the axis dialog.
    pub fn label(self) -> &'static str {
        match self {
            ScaleKind::Linear => "Linear",
            ScaleKind::Log10 => "Log10",
            ScaleKind::Ln => "Ln",
            ScaleKind::Log2 => "Log2",
            ScaleKind::Reciprocal => "Reciprocal",
            ScaleKind::OffsetReciprocal => "Offset Reciprocal",
            ScaleKind::Probability => "Probability",
            ScaleKind::Probit => "Probit",
            ScaleKind::Logit => "Logit",
            ScaleKind::Weibull => "Weibull",
            ScaleKind::CustomFormula => "Custom Formula",
        }
    }

    /// The scale type byte of the `.opj` layer header (opj-format.md section 10.2); one less than
    /// the number `layer.x.type` has in LabTalk for the built-in scales.
    /// An axis with custom formulas is written as Linear in a bare OPJ.
    pub fn file_code(self) -> u8 {
        match self {
            ScaleKind::Linear | ScaleKind::CustomFormula => 0,
            ScaleKind::Log10 => 1,
            ScaleKind::Probability => 2,
            ScaleKind::Probit => 3,
            ScaleKind::Reciprocal => 4,
            ScaleKind::OffsetReciprocal => 5,
            ScaleKind::Logit => 6,
            ScaleKind::Ln => 7,
            ScaleKind::Log2 => 8,
            ScaleKind::Weibull => 9,
        }
    }

    /// The inverse of [`ScaleKind::file_code`].
    pub fn from_file_code(code: u8) -> Option<ScaleKind> {
        ScaleKind::ALL.into_iter().find(|k| k.file_code() == code)
    }

    /// The base of a logarithmic scale.
    pub fn log_base(self) -> Option<f64> {
        match self {
            ScaleKind::Log10 => Some(10.0),
            ScaleKind::Ln => Some(std::f64::consts::E),
            ScaleKind::Log2 => Some(2.0),
            _ => None,
        }
    }

    /// The scales of percent values (0 to 100, exclusive).
    pub fn is_percent(self) -> bool {
        matches!(self, ScaleKind::Probability | ScaleKind::Probit | ScaleKind::Logit | ScaleKind::Weibull)
    }

    /// The tick values of a scale whose ticks come from a fixed list of percent values.
    pub fn tick_list(self) -> Option<&'static [f64]> {
        match self {
            ScaleKind::Probability => Some(&PROBABILITY_TICKS),
            ScaleKind::Logit => Some(&LOGIT_TICKS),
            ScaleKind::Weibull => Some(&WEIBULL_TICKS),
            _ => None,
        }
    }

    /// Whether `v` can be placed on the scale.
    pub fn accepts(self, v: f64) -> bool {
        if !v.is_finite() {
            return false;
        }
        match self {
            ScaleKind::Linear | ScaleKind::CustomFormula => true,
            ScaleKind::Log10 | ScaleKind::Ln | ScaleKind::Log2 | ScaleKind::Reciprocal => v > 0.0,
            ScaleKind::OffsetReciprocal => v + RECIPROCAL_OFFSET > 0.0,
            _ => v > 0.0 && v < 100.0,
        }
    }

    /// The position of `v` on the straight line the axis draws (increasing with `v`); NaN where the
    /// scale does not accept `v`.
    pub fn forward(self, v: f64) -> f64 {
        if !self.accepts(v) {
            return f64::NAN;
        }
        match self {
            ScaleKind::Linear | ScaleKind::CustomFormula => v,
            ScaleKind::Log10 => v.log10(),
            ScaleKind::Ln => v.ln(),
            ScaleKind::Log2 => v.log2(),
            ScaleKind::Reciprocal => -1.0 / v,
            ScaleKind::OffsetReciprocal => -1.0 / (v + RECIPROCAL_OFFSET),
            ScaleKind::Probability => norm_inv(v / 100.0),
            ScaleKind::Probit => norm_inv(v / 100.0) + 5.0,
            ScaleKind::Logit => (v / (100.0 - v)).ln(),
            ScaleKind::Weibull => (-(1.0 - v / 100.0).ln()).ln(),
        }
    }

    /// The inverse of [`ScaleKind::forward`].
    pub fn inverse(self, t: f64) -> f64 {
        match self {
            ScaleKind::Linear | ScaleKind::CustomFormula => t,
            ScaleKind::Log10 => 10f64.powf(t),
            ScaleKind::Ln => t.exp(),
            ScaleKind::Log2 => t.exp2(),
            ScaleKind::Reciprocal => -1.0 / t,
            ScaleKind::OffsetReciprocal => -1.0 / t - RECIPROCAL_OFFSET,
            ScaleKind::Probability => 100.0 * norm_cdf(t),
            ScaleKind::Probit => 100.0 * norm_cdf(t - 5.0),
            ScaleKind::Logit => 100.0 / (1.0 + (-t).exp()),
            ScaleKind::Weibull => 100.0 * (1.0 - (-t.exp()).exp()),
        }
    }
}

/// The value of a tick label formula (an expression of `x`) at `x`; `None` when it does not parse or
/// does not give a finite number.
pub fn eval_tick_formula(formula: &str, x: f64) -> Option<f64> {
    let host = sheaf_expr::SimpleHost::from_vars([("x", x)]);
    sheaf_expr::Expr::parse(formula).ok()?.eval(&host).ok().filter(|v| v.is_finite())
}

impl crate::graph::ScaleLink {
    /// Evaluate the child endpoints from the parent's X and Y limits in From/To order.
    /// Returns no limits for a non-formula link or a range that cannot be drawn on the child scale.
    pub fn formula_limits(&self, x: (f64, f64), y: (f64, f64), scale: ScaleKind) -> Option<(f64, f64)> {
        let Self::Custom { from, to } = self else { return None };
        let host = sheaf_expr::SimpleHost::from_vars([
            ("X1", x.0),
            ("X2", x.1),
            ("Y1", y.0),
            ("Y2", y.1),
            ("x1", x.0),
            ("x2", x.1),
            ("y1", y.0),
            ("y2", y.1),
        ]);
        let eval = |source| sheaf_expr::Expr::parse(source).ok()?.eval(&host).ok();
        let (from, to) = (eval(from)?, eval(to)?);
        let span = to - from;
        (scale.accepts(from) && scale.accepts(to) && span.is_finite() && span != 0.0).then_some((from, to))
    }
}

/// The one-dimensional transform and its inverse for a Custom Formula axis.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CustomScale {
    pub direct: String,
    pub inverse: String,
}

impl Default for CustomScale {
    fn default() -> Self {
        Self { direct: "x".into(), inverse: "x".into() }
    }
}

impl CustomScale {
    /// Parse both expressions once for layout and coordinate conversion.
    pub fn compile(&self) -> Option<FormulaTransform> {
        let direct = sheaf_expr::Expr::parse(&self.direct).ok()?;
        let inverse = sheaf_expr::Expr::parse(&self.inverse).ok()?;
        direct
            .variables()
            .into_iter()
            .chain(inverse.variables())
            .all(|variable| variable.eq_ignore_ascii_case("x"))
            .then_some(FormulaTransform { direct, inverse })
    }
}

/// Parsed formulas for an axis transform, which can be shared through a layout pass.
#[derive(Clone, Debug)]
pub struct FormulaTransform {
    direct: sheaf_expr::Expr,
    inverse: sheaf_expr::Expr,
}

impl PartialEq for FormulaTransform {
    fn eq(&self, other: &Self) -> bool {
        self.direct.source() == other.direct.source() && self.inverse.source() == other.inverse.source()
    }
}

impl FormulaTransform {
    /// Move from data values into the space in which the axis is drawn.
    pub fn forward(&self, x: f64) -> Option<f64> {
        Self::value(&self.direct, x)
    }

    /// Move from axis space back to a data value.
    pub fn inverse(&self, x: f64) -> Option<f64> {
        Self::value(&self.inverse, x)
    }

    fn value(expr: &sheaf_expr::Expr, x: f64) -> Option<f64> {
        if !x.is_finite() {
            return None;
        }
        expr.eval(&AxisVariable(x)).ok().filter(|v| v.is_finite())
    }

    /// Transform the two limits and check monotonicity and inverse agreement at points across
    /// the interval. Decreasing formulas give decreasing transformed limits.
    pub fn limits(&self, min: f64, max: f64) -> Option<(f64, f64)> {
        let width = max - min;
        if !width.is_finite() || width <= 0.0 {
            return None;
        }
        let (lo, hi) = (self.forward(min)?, self.forward(max)?);
        let span = hi - lo;
        if span == 0.0 || !span.is_finite() {
            return None;
        }
        let agrees = |x: f64, t: f64| self.inverse(t).is_some_and(|again| (again - x).abs() <= 1e-9 * x.abs().max(1.0));
        if !agrees(min, lo) || !agrees(max, hi) {
            return None;
        }
        let mut prev_x = min;
        let mut prev_t = lo;
        for i in 1..32 {
            let x = min + width * (f64::from(i) / 32.0);
            if x <= prev_x || x >= max {
                continue;
            }
            let t = self.forward(x)?;
            let increasing = prev_t < t && t < hi;
            let decreasing = hi < t && t < prev_t;
            if !(if span > 0.0 { increasing } else { decreasing }) || !agrees(x, t) {
                return None;
            }
            prev_x = x;
            prev_t = t;
        }
        Some((lo, hi))
    }
}

struct AxisVariable(f64);

impl sheaf_expr::Host for AxisVariable {
    fn variable(&self, name: &str) -> Option<f64> {
        name.eq_ignore_ascii_case("x").then_some(self.0)
    }

    fn cell(&self, _col: &sheaf_expr::ColumnRef, _row: usize) -> Option<f64> {
        None
    }

    fn has_column(&self, _col: &sheaf_expr::ColumnRef) -> bool {
        false
    }
}

/// The standard normal distribution function (West's double-precision algorithm, error about 1e-15).
pub fn norm_cdf(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    let xa = x.abs();
    let r = if xa > 37.0 {
        0.0
    } else {
        let e = (-xa * xa / 2.0).exp();
        if xa < 7.071_067_811_865_47 {
            let mut b = 3.526_249_659_989_11e-2 * xa + 0.700_383_064_443_688;
            for c in [
                6.373_962_203_531_65,
                33.912_866_078_383,
                112.079_291_497_871,
                221.213_596_169_931,
                220.206_867_912_376,
            ] {
                b = b * xa + c;
            }
            let mut d = 8.838_834_764_831_84e-2 * xa + 1.755_667_163_182_64;
            for c in [
                16.064_177_579_207,
                86.780_732_202_946_1,
                296.564_248_779_674,
                637.333_633_378_831,
                793.826_512_519_948,
                440.413_735_824_752,
            ] {
                d = d * xa + c;
            }
            e * b / d
        } else {
            let mut b = xa + 0.65;
            for c in [4.0, 3.0, 2.0, 1.0] {
                b = xa + c / b;
            }
            e / b / 2.506_628_274_631
        }
    };
    if x > 0.0 { 1.0 - r } else { r }
}

/// The standard normal quantile function: `p` in (0, 1) gives `z` with `norm_cdf(z) = p`.
/// Acklam's rational approximation followed by one Halley step (error about 1e-15).
pub fn norm_inv(p: f64) -> f64 {
    if !(p > 0.0 && p < 1.0) {
        return if p == 0.0 {
            f64::NEG_INFINITY
        } else if p == 1.0 {
            f64::INFINITY
        } else {
            f64::NAN
        };
    }
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] =
        [7.784_695_709_041_462e-3, 3.224_671_290_700_398e-1, 2.445_134_137_142_996, 3.754_408_661_907_416];
    let low = 0.02425;
    let x = if p < low {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= 1.0 - low {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    };
    // One Halley step against the distribution function.
    let e = norm_cdf(x) - p;
    let u = e * (2.0 * std::f64::consts::PI).sqrt() * (x * x / 2.0).exp();
    x - u / (1.0 + x * u / 2.0)
}

/// Limits and major step an automatic axis gets (Origin's Normal rescale), in data units. For a list
/// scale the step counts entries of the tick list; for a probit axis it is in standard deviations;
/// for a logarithmic scale it counts powers of the base.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ScaleRange {
    pub from: f64,
    pub to: f64,
    pub inc: f64,
}

fn tidy(v: f64) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    format!("{v:.11e}").parse().unwrap_or(v)
}

/// The step among 1, 2 and 5 times a power of ten that is nearest to `x` on a log scale.
pub fn nearest_nice(x: f64) -> f64 {
    if !(x.is_finite() && x > 0.0) {
        return 1.0;
    }
    let e = x.log10().floor() as i32;
    let base = 10f64.powi(e);
    [1.0, 2.0, 5.0, 10.0]
        .iter()
        .map(|m| m * base)
        .min_by(|a, b| (x / a).ln().abs().total_cmp(&(x / b).ln().abs()))
        .map(tidy)
        .unwrap_or(1.0)
}

/// A 1/2/5 increment rounded upward, used when the projected reciprocal range exceeds
/// Origin's default tick limit. Keep the raw comparison at decade boundaries.
fn reciprocal_promoted_step(high: f64) -> f64 {
    let raw = high / 10.0;
    if !(raw.is_finite() && raw > 0.0) {
        return 1.0;
    }
    let base = 10f64.powi(raw.log10().floor() as i32);
    [1.0, 2.0, 5.0, 10.0].into_iter().map(|m| m * base).find(|v| *v >= raw).map(tidy).unwrap_or(1.0)
}

/// Limits and step of an automatic axis for data `lo..hi` on `kind` (every scale but the linear and
/// log10 ones, whose rules are in [`crate::autorange`]); `margin_pct` is the Rescale Margin. `None`
/// for those two. `lo` and `hi` must be accepted by the scale.
pub fn auto_range(kind: ScaleKind, lo: f64, hi: f64, margin_pct: f64) -> Option<ScaleRange> {
    let m = margin_pct.max(0.0) / 100.0;
    let (lo, hi) = (lo.min(hi), lo.max(hi));
    let (tlo, thi) = (kind.forward(lo), kind.forward(hi));
    let pad = m * (thi - tlo);
    match kind {
        ScaleKind::Linear | ScaleKind::Log10 | ScaleKind::CustomFormula => None,
        ScaleKind::Ln | ScaleKind::Log2 => {
            let b = kind.log_base()?;
            if m == 0.0 {
                return Some(ScaleRange { from: lo, to: hi, inc: 1.0 });
            }
            let (xl, xh) = (kind.inverse(tlo - pad), kind.inverse(thi + pad));
            let (from, mut to) = (base_floor(b, xl), base_ceil(b, xh));
            if to <= from {
                to = from * b;
            }
            Some(ScaleRange { from, to, inc: 1.0 })
        }
        ScaleKind::Reciprocal | ScaleKind::OffsetReciprocal => {
            let low_raw = kind.inverse(tlo - pad);
            let high_t = thi + pad;
            // A margin that reaches the point at infinity: let the axis end well beyond the data.
            let high_raw = if high_t < 0.0 { kind.inverse(high_t) } else { hi * 100.0 };
            let finite_reciprocal =
                kind == ScaleKind::Reciprocal && hi > lo && low_raw > 0.0 && high_raw.is_finite() && high_t < 0.0;
            let initial_step = if finite_reciprocal { crate::autorange::linear_step(hi - lo) } else { 0.0 };
            let measured_step = finite_reciprocal && initial_step / 2.0 > 0.0 && initial_step.is_finite();
            let mut inc = if measured_step {
                initial_step
            } else if kind == ScaleKind::Reciprocal {
                nearest_nice(high_raw / 12.0)
            } else {
                nearest_nice((high_raw - low_raw) / 3.0)
            };
            let bounds = |step: f64| {
                let h = step / 2.0;
                let mut from = tidy(((low_raw / h) + 1e-9).floor() * h);
                let to = tidy(((high_raw / h) - 1e-9).ceil() * h);
                if kind == ScaleKind::Reciprocal && from <= 0.0 {
                    from = 1.0;
                }
                (from, to)
            };
            let (mut from, mut to) = bounds(inc);
            // The cap is checked before rounding the bounds. The promoted increment aims
            // for ten ticks from zero, rounding upward through 1/2/5 times a power of ten.
            if measured_step && (high_raw - low_raw) / inc > 20.0 {
                inc = reciprocal_promoted_step(high_raw);
                (from, to) = bounds(inc);
            }
            Some(ScaleRange { from, to, inc })
        }
        ScaleKind::Probit => {
            let (zl, zh) = (tlo - 5.0 - pad, thi - 5.0 + pad);
            let (a, mut b) = (zl.round(), zh.round());
            if b <= a {
                b = a + 1.0;
            }
            // Origin stores 3 as a probit axis's increment (as for probability) but draws every
            // whole probit whatever it is (T263, measured 2026-11-20); the ticks ignore it.
            Some(ScaleRange { from: kind.inverse(a + 5.0), to: kind.inverse(b + 5.0), inc: 3.0 })
        }
        ScaleKind::Probability | ScaleKind::Logit | ScaleKind::Weibull => {
            let list = kind.tick_list()?;
            // The limits of a logit axis stay within 0.1 and 99.9.
            let usable: &[f64] = if kind == ScaleKind::Logit { &list[1..list.len() - 1] } else { list };
            let (xl, xh) = (kind.inverse(tlo - pad), kind.inverse(thi + pad));
            let from = usable.iter().rev().find(|v| **v <= xl * (1.0 + 1e-9)).copied().unwrap_or(usable[0]);
            let to = usable.iter().find(|v| **v >= xh * (1.0 - 1e-9)).copied().unwrap_or(usable[usable.len() - 1]);
            let inc = match kind {
                ScaleKind::Probability => 3.0,
                ScaleKind::Logit => 1.0,
                _ => {
                    let span = to - from;
                    if span < 10.0 {
                        1.0
                    } else if span < 60.0 {
                        2.0
                    } else {
                        3.0
                    }
                }
            };
            Some(ScaleRange { from, to, inc })
        }
    }
}

/// `floor(x / b^k) * b^k` with `k = floor(log_b x)`: the next whole multiple of a power of the base
/// at or below `x`.
fn base_floor(b: f64, x: f64) -> f64 {
    let k = (x.ln() / b.ln() + 1e-12).floor();
    let p = b.powf(k);
    tidy(((x / p) + 1e-9).floor() * p)
}

/// The upper limit rule of Origin's log-like axes: with `k = floor(log_b x) + 1`, the next tenth of
/// `b^k` at or above `x`.
fn base_ceil(b: f64, x: f64) -> f64 {
    let k = (x.ln() / b.ln() + 1e-12).floor() + 1.0;
    let p = b.powf(k);
    tidy((((x / p) * 10.0) - 1e-9).ceil() / 10.0 * p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * b.abs().max(1e-3)
    }

    #[test]
    fn normal_functions() {
        assert!(close(norm_cdf(0.0), 0.5));
        assert!(close(norm_cdf(-1.0), 0.158_655_253_931_457));
        assert!(close(norm_cdf(-2.0), 0.022_750_131_948_179));
        for p in [1e-6, 0.001, 0.02, 0.3, 0.5, 0.7, 0.975, 0.999] {
            assert!(close(norm_cdf(norm_inv(p)), p), "p = {p}");
        }
    }

    #[test]
    fn nonlinear_custom_scale_checks_inverse_agreement_inside_the_limits() {
        let definition = CustomScale { direct: "x*x".into(), inverse: "x".into() };
        let formula = definition.compile().unwrap();
        assert_eq!(formula.inverse(0.0), Some(0.0));
        assert_eq!(formula.inverse(1.0), Some(1.0));
        assert!(formula.limits(0.0, 1.0).is_none());

        let definition = CustomScale { inverse: "sqrt(x)".into(), ..definition };
        assert_eq!(definition.compile().unwrap().limits(0.0, 1.0), Some((0.0, 1.0)));
    }

    #[test]
    fn forward_and_inverse_agree() {
        for kind in ScaleKind::ALL {
            let v = if kind.is_percent() { 37.0 } else { 12.5 };
            assert!(close(kind.inverse(kind.forward(v)), v), "{kind:?}");
        }
        for k in [ScaleKind::Reciprocal, ScaleKind::OffsetReciprocal] {
            assert!(k.forward(2.0) < k.forward(3.0));
        }
    }

    #[test]
    fn file_codes_round_trip() {
        for k in ScaleKind::ALL {
            assert_eq!(ScaleKind::from_file_code(k.file_code()), Some(k));
        }
        assert_eq!(ScaleKind::from_file_code(40), None);
    }

    // Limits and steps Origin reported for the same data (docs/spec/graphing.md section 3.4); data
    // `lo..hi` run along a ramp, Rescale Margin 8 %.
    #[test]
    fn log_like_limits_follow_origin() {
        for (kind, lo, hi, from, to) in [
            (ScaleKind::Ln, 1.0, 9.0, 0.735_758_882_342_884_7, 12.051_322_153_912_6),
            (ScaleKind::Ln, 1.0, 1000.0, 0.367_879_441_171_442_33, 1_788.574_792_225_025),
            (ScaleKind::Ln, 0.01, 50.0, 0.004_957_504_353_332_717, 103.889_211_371_803_6),
            (ScaleKind::Ln, 3.0, 300.0, 2.0, 438.653_263_371_383_4),
            (ScaleKind::Log2, 1.0, 9.0, 0.5, 11.2),
            (ScaleKind::Log2, 1.0, 1000.0, 0.5, 1843.2),
            (ScaleKind::Log2, 0.01, 50.0, 0.003_906_25, 102.4),
            (ScaleKind::Log2, 3.0, 300.0, 2.0, 460.8),
        ] {
            let r = auto_range(kind, lo, hi, 8.0).unwrap();
            assert!(close(r.from, from) && close(r.to, to), "{kind:?} {lo}..{hi}: {r:?}");
            assert_eq!(r.inc, 1.0);
        }
    }

    #[test]
    fn reciprocal_limits_follow_origin() {
        for (kind, lo, hi, from, to, inc) in [
            (ScaleKind::Reciprocal, 1.0, 9.0, 1.0, 25.0, 2.0),
            (ScaleKind::Reciprocal, 10.0, 100.0, 1.0, 360.0, 20.0),
            (ScaleKind::OffsetReciprocal, 0.0, 100.0, -25.0, 125.0, 50.0),
            (ScaleKind::OffsetReciprocal, 20.0, 300.0, 0.0, 350.0, 100.0),
            (ScaleKind::OffsetReciprocal, 5.0, 40.0, 0.0, 45.0, 10.0),
        ] {
            let r = auto_range(kind, lo, hi, 8.0).unwrap();
            assert_eq!((r.from, r.to, r.inc), (from, to, inc), "{kind:?} {lo}..{hi}");
        }
    }

    #[test]
    fn percent_scale_limits_follow_origin() {
        for (kind, lo, hi, from, to) in [
            (ScaleKind::Probability, 1.0, 99.0, 0.1, 99.9),
            (ScaleKind::Probability, 5.0, 60.0, 2.0, 70.0),
            (ScaleKind::Probability, 10.0, 90.0, 5.0, 95.0),
            (ScaleKind::Probability, 0.5, 99.5, 0.1, 99.9),
            (ScaleKind::Probability, 0.1, 99.9, 0.01, 99.99),
            (ScaleKind::Logit, 1.0, 99.0, 0.1, 99.9),
            (ScaleKind::Logit, 5.0, 60.0, 1.0, 75.0),
            (ScaleKind::Logit, 10.0, 90.0, 5.0, 95.0),
            (ScaleKind::Logit, 0.1, 99.9, 0.1, 99.9),
            (ScaleKind::Weibull, 1.0, 9.0, 0.5, 50.0),
            (ScaleKind::Weibull, 0.1, 0.9, 0.01, 5.0),
            (ScaleKind::Weibull, 5.0, 99.0, 1.0, 99.9),
            (ScaleKind::Probit, 1.0, 99.0, 0.134_989_803_162_5, 99.865_010_196_837_5),
            (ScaleKind::Probit, 5.0, 60.0, 2.275_013_194_809_2, 50.0),
            (ScaleKind::Probit, 10.0, 90.0, 15.865_525_393_275_9, 84.134_474_606_724_1),
        ] {
            let r = auto_range(kind, lo, hi, 8.0).unwrap();
            assert!(close(r.from, from) && close(r.to, to), "{kind:?} {lo}..{hi}: {r:?}");
        }
        assert_eq!(auto_range(ScaleKind::Probability, 1.0, 99.0, 8.0).unwrap().inc, 3.0);
        assert_eq!(auto_range(ScaleKind::Logit, 1.0, 99.0, 8.0).unwrap().inc, 1.0);
    }
}
