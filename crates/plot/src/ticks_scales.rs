//! Limits, ticks and labels of the scales other than linear and log10 (ln, log2, the reciprocals
//! and the percent scales). The rules were measured in Origin (graphing spec section 3.4); the
//! limits of an automatic axis come from `sheaf_core::scales::auto_range`.

use crate::ticks::{ResolvedAxis, clean, decimal, linear_labels, linear_major, linear_minor, scientific};
use sheaf_core::graph::{AutoRange, Axis, ScaleKind, TickFormat};
use sheaf_core::scales::{self, ScaleRange};

/// Resolves an axis of any scale but linear and log10.
pub(crate) fn resolve_other(axis: &Axis, data: Option<(f64, f64)>) -> ResolvedAxis {
    let kind = axis.scale;
    let fallback = match kind {
        ScaleKind::OffsetReciprocal => (0.0, 100.0),
        k if k.is_percent() => (10.0, 90.0),
        _ => (1.0, 10.0),
    };
    let have = data.is_some_and(|(a, b)| kind.accepts(a) && kind.accepts(b));
    let (lo, hi) = match data {
        Some((a, b)) if have => (a.min(b), a.max(b)),
        _ => fallback,
    };
    let auto = scales::auto_range(kind, lo, hi, axis.rescale_margin_pct);
    let (mut min, mut max) = match (&auto, axis.auto_range, have) {
        (Some(r), AutoRange::Nice, true) => (r.from, r.to),
        (_, AutoRange::Tight, true) if hi > lo => (lo, hi),
        (Some(r), _, _) => (r.from, r.to),
        _ => fallback,
    };
    if let Some(a) = axis.min.filter(|v| kind.accepts(*v)) {
        min = a;
    }
    if let Some(b) = axis.max.filter(|v| kind.accepts(*v)) {
        max = b;
    }
    if max.is_nan() || min.is_nan() || max <= min {
        (min, max) = (fallback.0, fallback.1);
    }
    let range = auto.unwrap_or(ScaleRange { from: min, to: max, inc: 1.0 });
    let fixed_step = axis.major_step.filter(|s| s.is_finite() && *s > 0.0);
    let (major, minor, labels, step) = match kind {
        ScaleKind::Ln | ScaleKind::Log2 => log_like(axis, min, max, fixed_step),
        ScaleKind::Reciprocal | ScaleKind::OffsetReciprocal => {
            reciprocal(axis, min, max, fixed_step.unwrap_or(range.inc))
        }
        // Every whole probit is major, whatever the increment says (T263, measured).
        ScaleKind::Probit => probit(axis, min, max),
        _ => listed(
            axis,
            min,
            max,
            fixed_step.map(|s| s.round().max(1.0)).unwrap_or_else(|| list_step(kind, min, max, &range)),
        ),
    };
    ResolvedAxis {
        min,
        max,
        step,
        scale: kind,
        major,
        minor,
        labels,
        reversed: false,
        gaps: Vec::new(),
        minor_labels: Vec::new(),
    }
}

type Ticks = (Vec<f64>, Vec<f64>, Vec<String>, f64);

fn within(v: f64, min: f64, max: f64) -> bool {
    let eps = 1e-9 * (max - min).abs().max(v.abs());
    v >= min - eps && v <= max + eps
}

/// The label of a value that is not a plain tick of the linear kind: the automatic format is
/// `auto`; fixed decimals and scientific formats apply as on any axis.
fn label_with(axis: &Axis, v: f64, auto: impl Fn(f64) -> String) -> String {
    match axis.tick_format {
        TickFormat::Decimal(d) => decimal(v, usize::from(d.min(15))),
        TickFormat::Scientific(d) => scientific(v, Some(usize::from(d.min(15)))),
        _ => auto(v),
    }
}

/// Powers of the base: a tick at `b^e` for the exponents `floor(log_b min) + j * inc` that fall
/// inside the limits. The labels have up to five decimals, as Origin writes them. Powers skipped by
/// an increment above one carry minor ticks.
fn log_like(axis: &Axis, min: f64, max: f64, fixed: Option<f64>) -> Ticks {
    let kind = axis.scale;
    let b = kind.log_base().unwrap_or(10.0);
    let (lmin, lmax) = (min.ln() / b.ln(), max.ln() / b.ln());
    let inc = match fixed {
        Some(s) => s.round().max(1.0),
        None => ((lmax - lmin) / 12.0).ceil().max(1.0),
    };
    let e0 = (lmin + 1e-9).floor() as i64;
    let e1 = (lmax + 1e-9).floor() as i64;
    let (mut major, mut minor) = (Vec::new(), Vec::new());
    for e in e0..=e1 {
        let v = kind.inverse(e as f64);
        if !within(v, min, max) {
            continue;
        }
        if (e - e0) % inc as i64 == 0 {
            major.push(clean(v));
        } else if axis.minor_ticks > 0 {
            minor.push(clean(v));
        }
    }
    let labels = major
        .iter()
        .map(|v| {
            label_with(axis, *v, |v| {
                let s = format!("{v:.5}");
                if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
            })
        })
        .collect();
    (major, minor, labels, inc)
}

/// Multiples of the step in data units, with a minor tick in the middle of each gap (in data
/// units, not on the reversed scale).
fn reciprocal(axis: &Axis, min: f64, max: f64, step: f64) -> Ticks {
    let (major, step) = linear_major(min, max, step, false);
    let minor = linear_minor(min, max, step, axis.minor_ticks);
    let labels = linear_labels(axis, &major, step);
    (major, minor, labels, step)
}

/// Probit axis: ticks at whole probits (normal quantile plus 5), labelled with the probit.
/// A probit axis (T263, measured in Origin 2016 on 2026-11-20): a major tick and label at every whole
/// probit and no minor ticks, with the increment 3, 1 or 2 alike (Origin keeps 3 as the increment
/// but does not use it).
fn probit(axis: &Axis, min: f64, max: f64) -> Ticks {
    let kind = ScaleKind::Probit;
    let (pmin, pmax) = (kind.forward(min), kind.forward(max));
    let (probits, step) = linear_major(pmin, pmax, 1.0, false);
    let major: Vec<f64> = probits.iter().map(|p| clean(kind.inverse(*p))).collect();
    let minor = Vec::new();
    let labels = linear_labels(axis, &probits, step);
    (major, minor, labels, step)
}

/// The default increment, in entries of the tick list, of an axis that does not fix it.
fn list_step(kind: ScaleKind, min: f64, max: f64, range: &ScaleRange) -> f64 {
    if axis_follows_range(min, max, range) {
        return range.inc;
    }
    match kind {
        ScaleKind::Probability => 3.0,
        ScaleKind::Logit => 1.0,
        _ => {
            let span = max - min;
            if span < 10.0 {
                1.0
            } else if span < 60.0 {
                2.0
            } else {
                3.0
            }
        }
    }
}

fn axis_follows_range(min: f64, max: f64, r: &ScaleRange) -> bool {
    (min - r.from).abs() <= 1e-12 * r.from.abs() && (max - r.to).abs() <= 1e-12 * r.to.abs()
}

/// A scale whose ticks come from a fixed list of percent values: every `inc`-th entry of the list
/// inside the limits is a major tick, counted from the first entry; the lower limit is labelled
/// too; the entries in between are minor ticks.
fn listed(axis: &Axis, min: f64, max: f64, inc: f64) -> Ticks {
    let list = axis.scale.tick_list().unwrap_or(&[]);
    let inc = inc.max(1.0) as usize;
    let (mut major, mut minor) = (Vec::new(), Vec::new());
    for (i, v) in list.iter().enumerate() {
        if !within(*v, min, max) {
            continue;
        }
        if i % inc == 0 {
            major.push(*v);
        } else if axis.minor_ticks > 0 {
            minor.push(*v);
        }
    }
    if !major.iter().any(|v| within(min, *v, *v)) {
        major.insert(0, min);
        minor.retain(|v| !within(min, *v, *v));
    }
    let labels = major.iter().map(|v| label_with(axis, *v, |v| format!("{}", clean(v)))).collect();
    (major, minor, labels, inc as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn axis_of(kind: ScaleKind) -> Axis {
        Axis { scale: kind, ..crate::ticks::tests_axis() }
    }

    fn resolve(kind: ScaleKind, data: (f64, f64)) -> ResolvedAxis {
        crate::ticks::resolve_axis(&axis_of(kind), Some(data))
    }

    #[test]
    fn ln_ticks_are_powers_of_e() {
        // Origin showed 0.36788 .. 1096.63316 (every power of e) for data 1..1000.
        let r = resolve(ScaleKind::Ln, (1.0, 1000.0));
        assert_eq!(r.labels.first().map(String::as_str), Some("0.36788"));
        assert_eq!(r.labels.last().map(String::as_str), Some("1096.63316"));
        assert_eq!(r.labels.len(), 9);
        assert!(r.labels.contains(&"2.71828".to_string()) && r.labels.contains(&"1".to_string()));
    }

    #[test]
    fn log2_ticks_are_powers_of_two() {
        let r = resolve(ScaleKind::Log2, (1.0, 1000.0));
        assert_eq!(r.labels.first().map(String::as_str), Some("0.5"));
        assert_eq!(r.labels.last().map(String::as_str), Some("1024"));
        assert_eq!(r.labels.len(), 12);
        // An increment of two: odd exponents, the powers between are minor ticks (measured on 0.01..5000).
        let mut a = axis_of(ScaleKind::Log2);
        a.min = Some(0.01);
        a.max = Some(5000.0);
        a.major_step = Some(2.0);
        let r = crate::ticks::resolve_axis(&a, Some((1.0, 9.0)));
        assert_eq!(r.labels[..3], ["0.03125", "0.125", "0.5"]);
        assert_eq!(r.labels.last().map(String::as_str), Some("2048"));
        assert!(r.minor.contains(&0.0625));
    }

    #[test]
    fn reciprocal_ticks_are_multiples_of_the_step() {
        let r = resolve(ScaleKind::Reciprocal, (1.0, 9.0));
        assert_eq!((r.min, r.max, r.step), (1.0, 25.0, 2.0));
        assert_eq!(r.major[0], 2.0);
        assert!(r.to_unit(2.0) > 0.4 && r.to_unit(2.0) < 0.6, "1/x: 2 sits a little above the middle");
        let r = resolve(ScaleKind::OffsetReciprocal, (0.0, 100.0));
        assert_eq!(r.major, vec![0.0, 50.0, 100.0]);
        // Measured picture: 0, 50 and 100 stand at 0.25 + 0.0..: larger values higher up.
        assert!(r.to_unit(100.0) > r.to_unit(50.0) && r.to_unit(50.0) > r.to_unit(0.0));
    }

    #[test]
    fn probability_ticks_follow_the_list() {
        // Origin: data 1..99 gives 0.1 .. 99.9; the ticks are every third entry of the list.
        let r = resolve(ScaleKind::Probability, (1.0, 99.0));
        assert_eq!((r.min, r.max), (0.1, 99.9));
        assert_eq!(r.labels, ["0.1", "1", "10", "40", "70", "95", "99.5"]);
        let r = resolve(ScaleKind::Probability, (10.0, 90.0));
        assert_eq!(r.labels, ["5", "10", "40", "70", "95"]);
        let mut a = axis_of(ScaleKind::Probability);
        a.major_step = Some(1.0);
        let r = crate::ticks::resolve_axis(&a, Some((1.0, 99.0)));
        assert_eq!(r.labels.len(), 19, "every entry of the list inside 0.1 .. 99.9");
        // The normal quantile is the position: 50 is in the middle, symmetric about it.
        let r = resolve(ScaleKind::Probability, (1.0, 99.0));
        assert!((r.to_unit(50.0) - 0.5).abs() < 1e-9);
        assert!((r.to_unit(10.0) + r.to_unit(90.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn logit_and_weibull_ticks() {
        let r = resolve(ScaleKind::Logit, (1.0, 99.0));
        assert_eq!(r.labels, ["0.1", "1", "5", "10", "25", "50", "75", "90", "95", "99", "99.9"]);
        let mut a = axis_of(ScaleKind::Logit);
        a.major_step = Some(2.0);
        let r = crate::ticks::resolve_axis(&a, Some((1.0, 99.0)));
        assert_eq!(r.labels, ["0.1", "1", "10", "50", "90", "99"]);
        let r = resolve(ScaleKind::Weibull, (0.1, 0.9));
        assert_eq!((r.min, r.max), (0.01, 5.0));
        assert_eq!(r.labels, ["0.01", "0.1", "0.5", "1", "5"]);
    }

    #[test]
    fn probit_ticks_are_whole_probits() {
        let r = resolve(ScaleKind::Probit, (1.0, 99.0));
        assert_eq!(r.labels, ["2", "3", "4", "5", "6", "7", "8"]);
        assert!((r.major[3] - 50.0).abs() < 1e-9);
        assert!(r.minor.is_empty(), "no minor ticks on a probit axis");
    }

    #[test]
    fn a_probit_axis_ignores_its_increment_as_origin_does() {
        // Origin 2016: data 0.5..80 gives 0.135..84.13 % with inc 3, labelled 2..6 at every whole
        // probit; inc 1 and inc 2 draw the same axis.
        let auto = sheaf_core::scales::auto_range(ScaleKind::Probit, 0.5, 80.0, 8.0).unwrap();
        assert_eq!(auto.inc, 3.0);
        for step in [None, Some(1.0), Some(2.0), Some(3.0)] {
            let mut a = axis_of(ScaleKind::Probit);
            a.major_step = step;
            let r = crate::ticks::resolve_axis(&a, Some((0.5, 80.0)));
            assert_eq!(r.labels, ["2", "3", "4", "5", "6"], "inc {step:?}");
        }
    }
}
