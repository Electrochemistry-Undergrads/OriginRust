//! IIR filter design and application as Origin's Analysis > Signal Processing > IIR Filter
//! (`dfilter`, OriginPro; `docs/spec/analysis-tools.md` 17). Nothing here is measured in Origin:
//! the design chain follows Origin's documentation (an analog low pass prototype, the frequency
//! transform to the response type, the bilinear transform) and the numbers are checked against
//! `scipy.signal` (`crates/calc/tests/scipy_iir.rs`, made by `tools/iir-fir-ref.py`).
//!
//! Documented by Origin: the four methods (Butterworth, Chebyshev I, Chebyshev II, elliptic), the
//! four response types, the order (an integer, or the minimum that meets the pass and stop
//! frequencies), frequencies in Hz (with the sample frequency taken from the X column) or
//! normalized to the Nyquist frequency, a passband ripple (default 1 dB) and a stopband
//! attenuation (default 80 dB), forward-backward filtering (off by default), and the outputs
//! (SOS matrix, zeros, poles and gain, transfer function coefficients).
//!
//! Sheaf's choices, unmeasured (Origin does not say): the bilinear transform pre-warps the
//! frequencies (`tan(pi f / fs)`, as scipy does); the minimum order follows scipy's `buttord`,
//! `cheb1ord`, `cheb2ord` and `ellipord`; the zeros and poles are paired into second-order
//! sections by the closest pole to the unit circle last; one-way filtering is `sosfilt` with
//! zero initial state; forward-backward filtering is `sosfiltfilt` (odd extension of `3 *
//! ntaps` points at both ends, initial state of the step response), which needs more than that
//! many points. Bessel is not offered: Origin's documentation lists four methods and no Bessel.

use std::f64::consts::PI;

use rustfft::num_complex::Complex;
use serde::{Deserialize, Serialize};

type C = Complex<f64>;

/// Why a filter could not be designed or applied.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterError {
    /// A frequency is missing, out of range or out of order.
    BadFrequency(String),
    /// The order is not usable.
    BadOrder(String),
    /// The passband ripple or the stopband attenuation is not usable.
    BadMagnitude(String),
    /// Forward-backward filtering and the padding need more points.
    TooShort { need: usize, got: usize },
    /// The design produced numbers that are not finite.
    NotFinite,
}

impl std::fmt::Display for FilterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilterError::BadFrequency(s) | FilterError::BadOrder(s) | FilterError::BadMagnitude(s) => f.write_str(s),
            FilterError::TooShort { need, got } => {
                write!(f, "the filter needs more than {need} points, the signal has {got}")
            }
            FilterError::NotFinite => f.write_str("the filter design gave values that are not finite"),
        }
    }
}

impl std::error::Error for FilterError {}

/// Origin's Response Type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FilterResponse {
    #[default]
    LowPass,
    HighPass,
    BandPass,
    BandStop,
}

impl FilterResponse {
    pub fn is_band(self) -> bool {
        matches!(self, FilterResponse::BandPass | FilterResponse::BandStop)
    }
    pub fn label(self) -> &'static str {
        match self {
            FilterResponse::LowPass => "low pass",
            FilterResponse::HighPass => "high pass",
            FilterResponse::BandPass => "band pass",
            FilterResponse::BandStop => "band stop",
        }
    }
}

/// Origin's Unit of the frequency specification: Hz, or normalized so that 1 is the Nyquist
/// frequency (`f / (fs / 2)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FreqUnit {
    #[default]
    Hz,
    Normalized,
}

/// Origin's Method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum IirMethod {
    #[default]
    Butterworth,
    Chebyshev1,
    Chebyshev2,
    Elliptic,
}

impl IirMethod {
    pub fn label(self) -> &'static str {
        match self {
            IirMethod::Butterworth => "Butterworth",
            IirMethod::Chebyshev1 => "Chebyshev I",
            IirMethod::Chebyshev2 => "Chebyshev II",
            IirMethod::Elliptic => "elliptic",
        }
    }
}

/// The largest order Sheaf designs (the order of the low pass prototype).
pub const MAX_ORDER: usize = 200;

/// The options of the IIR Filter dialog, with Origin's defaults (`dfilter`: Butterworth, low
/// pass, order 10, ripple 1 dB, attenuation 80 dB, no forward-backward filtering).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct IirSpec {
    pub response: FilterResponse,
    pub method: IirMethod,
    /// The filter order; `None` is Origin's Minimum (from the pass and stop frequencies).
    pub order: Option<usize>,
    pub unit: FreqUnit,
    /// Sample frequency in Hz (Hz unit only); `None` takes it from the X column.
    pub sample_frequency: Option<f64>,
    /// Cutoff frequency (low and high pass) or the first cutoff (band pass and band stop); a
    /// fixed order only. `None` is a quarter of the Nyquist frequency.
    pub cutoff: Option<f64>,
    /// The second cutoff of the band responses; `None` is three quarters of the Nyquist.
    pub cutoff2: Option<f64>,
    /// Pass and stop frequencies, for the minimum order. `None` is Sheaf's default.
    pub pass: Option<f64>,
    pub pass2: Option<f64>,
    pub stop: Option<f64>,
    pub stop2: Option<f64>,
    /// Passband ripple in dB (Chebyshev I, elliptic; the passband edge of the minimum order).
    pub passband_ripple: f64,
    /// Stopband attenuation in dB (Chebyshev II, elliptic; the stop edge of the minimum order).
    pub stopband_attenuation: f64,
    /// Forward-Backward Filtering.
    pub forward_backward: bool,
}

impl Default for IirSpec {
    fn default() -> Self {
        IirSpec {
            response: FilterResponse::LowPass,
            method: IirMethod::Butterworth,
            order: Some(10),
            unit: FreqUnit::Hz,
            sample_frequency: None,
            cutoff: None,
            cutoff2: None,
            pass: None,
            pass2: None,
            stop: None,
            stop2: None,
            passband_ripple: 1.0,
            stopband_attenuation: 80.0,
            forward_backward: false,
        }
    }
}

/// A designed digital IIR filter.
#[derive(Debug, Clone, PartialEq)]
pub struct IirFilter {
    /// The order of the analog prototype (the filter has this many poles, twice as many for the
    /// band responses).
    pub order: usize,
    /// The cutoff frequencies as used, normalized so that 1 is the Nyquist frequency.
    pub wn: Vec<f64>,
    pub zeros: Vec<C>,
    pub poles: Vec<C>,
    pub gain: f64,
    /// Second-order sections `[b0, b1, b2, 1, a1, a2]`, the gain in the first.
    pub sos: Vec<[f64; 6]>,
}

impl IirFilter {
    /// Numerator and denominator of the transfer function (polynomials in `z^-1`).
    pub fn coefficients(&self) -> (Vec<f64>, Vec<f64>) {
        let b: Vec<f64> = poly(&self.zeros).iter().map(|c| c.re * self.gain).collect();
        let a: Vec<f64> = poly(&self.poles).iter().map(|c| c.re).collect();
        (b, a)
    }

    /// Magnitude of the frequency response at `w` (normalized, 1 is Nyquist).
    pub fn magnitude(&self, w: f64) -> f64 {
        let zinv = C::from_polar(1.0, -PI * w);
        let zinv2 = zinv * zinv;
        let mut h = C::new(1.0, 0.0);
        for s in &self.sos {
            let num = C::new(s[0], 0.0) + zinv * s[1] + zinv2 * s[2];
            let den = C::new(s[3], 0.0) + zinv * s[4] + zinv2 * s[5];
            h = h * num / den;
        }
        h.norm()
    }

    /// Filters `y`: one way (`sosfilt`, zero initial state), or forward and backward
    /// (`sosfiltfilt`, zero phase).
    pub fn apply(&self, y: &[f64], forward_backward: bool) -> Result<Vec<f64>, FilterError> {
        if forward_backward { sosfiltfilt(&self.sos, y) } else { Ok(sosfilt(&self.sos, y, None)) }
    }
}

// ---------------------------------------------------------------------------------------------
// Elliptic functions (Landen / AGM, after Cephes), needed by the elliptic prototype and order.

fn agm(mut a: f64, mut b: f64) -> f64 {
    for _ in 0..64 {
        let next = (a + b) / 2.0;
        b = (a * b).sqrt();
        a = next;
        if (a - b).abs() <= 1e-16 * a.abs() {
            break;
        }
    }
    a
}

/// Complete elliptic integral of the first kind, parameter `m < 1`.
pub(crate) fn ellipk(m: f64) -> f64 {
    PI / (2.0 * agm(1.0, (1.0 - m).sqrt()))
}

/// `K(1 - p)`, accurate for small `p`.
fn ellipkm1(p: f64) -> f64 {
    PI / (2.0 * agm(1.0, p.sqrt()))
}

/// Jacobi elliptic functions `(sn, cn, dn)` of real `u`, parameter `0 <= m <= 1`.
fn ellipj(u: f64, m: f64) -> (f64, f64, f64) {
    if m < 1e-9 {
        let (t, b) = (u.sin(), u.cos());
        let ai = 0.25 * m * (u - t * b);
        return (t - ai * b, b + ai * t, 1.0 - 0.5 * m * t * t);
    }
    if m >= 1.0 - 1e-9 {
        let ai = 0.25 * (1.0 - m);
        let b = u.cosh();
        let t = u.tanh();
        let phi = 1.0 / b;
        let twon = b * u.sinh();
        let sn = t + ai * (twon - u) / (b * b);
        let ai2 = ai * t * phi;
        let cn = phi - ai2 * (twon - u);
        let dn = phi + ai2 * (twon + u);
        return (sn, cn, dn);
    }
    let mut a = [0.0f64; 9];
    let mut c = [0.0f64; 9];
    a[0] = 1.0;
    let mut b = (1.0 - m).sqrt();
    c[0] = m.sqrt();
    let mut twon = 1.0;
    let mut i = 0usize;
    while (c[i] / a[i]).abs() > f64::EPSILON && i < 8 {
        let ai = a[i];
        i += 1;
        c[i] = (ai - b) / 2.0;
        let t = (ai * b).sqrt();
        a[i] = (ai + b) / 2.0;
        b = t;
        twon *= 2.0;
    }
    let mut phi = twon * a[i] * u;
    let mut prev = phi;
    while i > 0 {
        let t = c[i] * phi.sin() / a[i];
        prev = phi;
        phi = (t.asin() + phi) / 2.0;
        i -= 1;
    }
    let sn = phi.sin();
    let cn = phi.cos();
    let dnfac = (phi - prev).cos();
    let dn = if dnfac.abs() < 0.1 { (1.0 - m * sn * sn).sqrt() } else { cn / dnfac };
    (sn, cn, dn)
}

/// The parameter `m` of the elliptic function of degree `n` with the complementary `m1`.
fn ellipdeg(n: usize, m1: f64) -> f64 {
    let k1 = ellipk(m1);
    let k1p = ellipkm1(m1);
    let q1 = (-PI * k1p / k1).exp();
    let q = q1.powf(1.0 / n as f64);
    let num: f64 = (0..=7).map(|m| q.powi(m * (m + 1))).sum();
    let den = 1.0 + 2.0 * (1..=8).map(|m| q.powi(m * m)).sum::<f64>();
    16.0 * q * (num / den).powi(4)
}

/// Inverse of the Jacobi `sn` for a complex argument (descending Landen transformation).
fn arc_jac_sn(w: C, m: f64) -> C {
    let k = m.sqrt();
    if k >= 1.0 {
        return w.atanh();
    }
    let mut ks = vec![k];
    for _ in 0..12 {
        let k_ = *ks.last().unwrap();
        if k_ == 0.0 {
            break;
        }
        let kp = (1.0 - k_ * k_).sqrt();
        ks.push((1.0 - kp) / (1.0 + kp));
    }
    let big_k: f64 = ks[1..].iter().map(|k| 1.0 + k).product::<f64>() * PI / 2.0;
    let mut wn = w;
    for pair in ks.windows(2) {
        let (kn, knext) = (pair[0], pair[1]);
        wn = wn * 2.0 / ((1.0 + knext) * (C::new(1.0, 0.0) + (C::new(1.0, 0.0) - wn * wn * (kn * kn)).sqrt()));
    }
    wn.asin() * (2.0 / PI) * big_k
}

// ---------------------------------------------------------------------------------------------
// Analog low pass prototypes (cutoff 1 rad/s), as zeros, poles and gain.

type Zpk = (Vec<C>, Vec<C>, f64);

fn neg_prod(v: &[C]) -> C {
    v.iter().fold(C::new(1.0, 0.0), |a, b| a * (-b))
}

fn odd_range(n: usize) -> impl Iterator<Item = f64> {
    (0..n).map(move |i| -(n as f64) + 1.0 + 2.0 * i as f64)
}

fn buttap(n: usize) -> Zpk {
    // p = -exp(i pi m / 2n)
    let p = odd_range(n).map(|m| -C::from_polar(1.0, PI * m / (2.0 * n as f64))).collect::<Vec<_>>();
    (vec![], p, 1.0)
}

fn cheb1ap(n: usize, rp: f64) -> Zpk {
    let eps = (10f64.powf(0.1 * rp) - 1.0).sqrt();
    let mu = (1.0 / eps).asinh() / n as f64;
    let p: Vec<C> = odd_range(n)
        .map(|m| {
            let theta = PI * m / (2.0 * n as f64);
            // -sinh(mu + i theta) = -(sinh mu cos theta + i cosh mu sin theta)
            C::new(-mu.sinh() * theta.cos(), -mu.cosh() * theta.sin())
        })
        .collect();
    let mut k = neg_prod(&p).re;
    if n.is_multiple_of(2) {
        k /= (1.0 + eps * eps).sqrt();
    }
    (vec![], p, k)
}

fn cheb2ap(n: usize, rs: f64) -> Zpk {
    let de = 1.0 / (10f64.powf(0.1 * rs) - 1.0).sqrt();
    let mu = (1.0 / de).asinh() / n as f64;
    let nf = n as f64;
    let m: Vec<f64> = if n % 2 == 1 {
        let mut v: Vec<f64> = (0..(n / 2)).map(|i| -nf + 1.0 + 2.0 * i as f64).collect();
        v.extend((0..).map(|i| 2.0 + 2.0 * i as f64).take_while(|&x| x < nf));
        v
    } else {
        odd_range(n).collect()
    };
    let z: Vec<C> = m
        .iter()
        .map(|&m| {
            let s = (m * PI / (2.0 * nf)).sin();
            // conj(i / s) = -i / s, negated: +i / s
            C::new(0.0, 1.0 / s)
        })
        .collect();
    let p: Vec<C> = odd_range(n)
        .map(|m| {
            let theta = PI * m / (2.0 * nf);
            // p = -exp(i theta), then sinh(mu) Re + i cosh(mu) Im, then 1 / p
            let p = C::new(-mu.sinh() * theta.cos(), -mu.cosh() * theta.sin());
            C::new(1.0, 0.0) / p
        })
        .collect();
    let k = (neg_prod(&p) / neg_prod(&z)).re;
    (z, p, k)
}

fn ellipap(n: usize, rp: f64, rs: f64) -> Zpk {
    if n == 1 {
        let p = -(1.0 / (10f64.powf(0.1 * rp) - 1.0)).sqrt();
        return (vec![], vec![C::new(p, 0.0)], p.abs());
    }
    let eps_sq = 10f64.powf(0.1 * rp) - 1.0;
    let eps = eps_sq.sqrt();
    let ck1_sq = eps_sq / (10f64.powf(0.1 * rs) - 1.0);
    let val0 = ellipk(ck1_sq);
    let m = ellipdeg(n, ck1_sq);
    let capk = ellipk(m);
    let nf = n as f64;
    let js: Vec<f64> = (0..).map(|i| (1 - (n % 2) as i64) as f64 + 2.0 * i as f64).take_while(|&j| j < nf).collect();
    let sj: Vec<(f64, f64, f64)> = js.iter().map(|&j| ellipj(j * capk / nf, m)).collect();
    let mut z: Vec<C> =
        sj.iter().filter(|(s, _, _)| s.abs() > 2e-16).map(|&(s, _, _)| C::new(0.0, 1.0 / (m.sqrt() * s))).collect();
    let zc: Vec<C> = z.iter().map(|c| c.conj()).collect();
    z.extend(zc);
    let r = arc_jac_sn(C::new(0.0, 1.0 / eps), ck1_sq).im;
    let v0 = capk * r / (nf * val0);
    let (sv, cv, dv) = ellipj(v0, 1.0 - m);
    let mut p: Vec<C> = sj
        .iter()
        .map(|&(s, c, d)| {
            let den = 1.0 - (d * sv) * (d * sv);
            C::new(-(c * d * sv * cv) / den, -(s * dv) / den)
        })
        .collect();
    if n % 2 == 1 {
        let norm = p.iter().map(|c| c.norm_sqr()).sum::<f64>().sqrt();
        let newp: Vec<C> = p.iter().filter(|c| c.im.abs() > 2e-16 * norm).map(|c| c.conj()).collect();
        p.extend(newp);
    } else {
        let pc: Vec<C> = p.iter().map(|c| c.conj()).collect();
        p.extend(pc);
    }
    let mut k = (neg_prod(&p) / neg_prod(&z)).re;
    if n.is_multiple_of(2) {
        k /= (1.0 + eps_sq).sqrt();
    }
    (z, p, k)
}

// ---------------------------------------------------------------------------------------------
// Frequency transforms (scipy's lp2lp_zpk and friends) and the bilinear transform.

fn lp2lp(z: &[C], p: &[C], k: f64, wo: f64) -> Zpk {
    let degree = p.len() - z.len();
    (z.iter().map(|c| c * wo).collect(), p.iter().map(|c| c * wo).collect(), k * wo.powi(degree as i32))
}

fn lp2hp(z: &[C], p: &[C], k: f64, wo: f64) -> Zpk {
    let degree = p.len() - z.len();
    let mut zh: Vec<C> = z.iter().map(|c| C::new(wo, 0.0) / c).collect();
    let ph: Vec<C> = p.iter().map(|c| C::new(wo, 0.0) / c).collect();
    zh.extend(std::iter::repeat_n(C::new(0.0, 0.0), degree));
    let kh = k * (neg_prod(z) / neg_prod(p)).re;
    (zh, ph, kh)
}

fn lp2bp(z: &[C], p: &[C], k: f64, wo: f64, bw: f64) -> Zpk {
    let degree = p.len() - z.len();
    let split = |v: &[C]| -> Vec<C> {
        let lp: Vec<C> = v.iter().map(|c| c * (bw / 2.0)).collect();
        let mut out: Vec<C> = lp.iter().map(|c| *c + (c * c - wo * wo).sqrt()).collect();
        out.extend(lp.iter().map(|c| *c - (c * c - wo * wo).sqrt()));
        out
    };
    let mut zb = split(z);
    zb.extend(std::iter::repeat_n(C::new(0.0, 0.0), degree));
    (zb, split(p), k * bw.powi(degree as i32))
}

fn lp2bs(z: &[C], p: &[C], k: f64, wo: f64, bw: f64) -> Zpk {
    let degree = p.len() - z.len();
    let split = |v: &[C]| -> Vec<C> {
        let hp: Vec<C> = v.iter().map(|c| C::new(bw / 2.0, 0.0) / c).collect();
        let mut out: Vec<C> = hp.iter().map(|c| *c + (c * c - wo * wo).sqrt()).collect();
        out.extend(hp.iter().map(|c| *c - (c * c - wo * wo).sqrt()));
        out
    };
    let mut zb = split(z);
    for _ in 0..degree {
        zb.push(C::new(0.0, wo));
        zb.push(C::new(0.0, -wo));
    }
    let kb = k * (neg_prod(z) / neg_prod(p)).re;
    (zb, split(p), kb)
}

/// Bilinear transform with sample frequency `fs` (`s = 2 fs (1 - z^-1) / (1 + z^-1)`).
fn bilinear(z: &[C], p: &[C], k: f64, fs: f64) -> Zpk {
    let degree = p.len() - z.len();
    let fs2 = 2.0 * fs;
    let map = |v: &[C]| -> Vec<C> { v.iter().map(|c| (fs2 + c) / (fs2 - c)).collect() };
    let mut zz = map(z);
    zz.extend(std::iter::repeat_n(C::new(-1.0, 0.0), degree));
    let num = z.iter().fold(C::new(1.0, 0.0), |a, c| a * (fs2 - c));
    let den = p.iter().fold(C::new(1.0, 0.0), |a, c| a * (fs2 - c));
    (zz, map(p), k * (num / den).re)
}

/// Coefficients (highest power first) of the polynomial with the given roots.
fn poly(roots: &[C]) -> Vec<C> {
    let mut a = vec![C::new(1.0, 0.0)];
    for r in roots {
        let mut next = vec![C::new(0.0, 0.0); a.len() + 1];
        for (i, c) in a.iter().enumerate() {
            next[i] += c;
            next[i + 1] -= c * r;
        }
        a = next;
    }
    a
}

// ---------------------------------------------------------------------------------------------
// Second-order sections.

/// Splits roots into one of each complex conjugate pair (positive imaginary part) and the real ones.
fn cplx_real(v: &[C]) -> (Vec<C>, Vec<f64>) {
    let mut cplx = Vec::new();
    let mut real = Vec::new();
    for c in v {
        if c.im.abs() <= 100.0 * f64::EPSILON * c.norm().max(1e-300) {
            real.push(c.re);
        } else if c.im > 0.0 {
            cplx.push(*c);
        }
    }
    (cplx, real)
}

fn nearest(list: &[C], to: C) -> usize {
    let mut best = 0;
    for (i, c) in list.iter().enumerate() {
        if (c - to).norm() < (list[best] - to).norm() {
            best = i;
        }
    }
    best
}

fn nearest_real(list: &[f64], to: f64) -> usize {
    let mut best = 0;
    for (i, c) in list.iter().enumerate() {
        if (c - to).abs() < (list[best] - to).abs() {
            best = i;
        }
    }
    best
}

/// Pairs the zeros and poles into second-order sections `[b0 b1 b2 1 a1 a2]`: the pole closest
/// to the unit circle goes last, each with the nearest zeros; the gain goes into the first.
fn zpk2sos(z: &[C], p: &[C], k: f64) -> Vec<[f64; 6]> {
    let mut z = z.to_vec();
    let mut p = p.to_vec();
    while z.len() < p.len() {
        z.push(C::new(0.0, 0.0));
    }
    while p.len() < z.len() {
        p.push(C::new(0.0, 0.0));
    }
    if p.len() % 2 == 1 {
        p.push(C::new(0.0, 0.0));
        z.push(C::new(0.0, 0.0));
    }
    if p.is_empty() {
        return vec![[k, 0.0, 0.0, 1.0, 0.0, 0.0]];
    }
    let (mut pc, mut pr) = cplx_real(&p);
    let (mut zc, mut zr) = cplx_real(&z);
    let mut sections: Vec<[f64; 6]> = Vec::new();
    while !pc.is_empty() || !pr.is_empty() {
        let dc = pc.iter().enumerate().map(|(i, c)| (i, (1.0 - c.norm()).abs())).min_by(|a, b| a.1.total_cmp(&b.1));
        let dr = pr.iter().enumerate().map(|(i, c)| (i, (1.0 - c.abs()).abs())).min_by(|a, b| a.1.total_cmp(&b.1));
        let use_complex = match (dc, dr) {
            (Some(c), Some(r)) => c.1 <= r.1,
            (Some(_), None) => true,
            _ => false,
        };
        let (a1, a2, p_for_zero) = if use_complex {
            let c = pc.remove(dc.unwrap().0);
            (-2.0 * c.re, c.norm_sqr(), (c, c))
        } else {
            let r1 = pr.remove(dr.unwrap().0);
            // the other real pole: the next closest to the unit circle
            let j = pr
                .iter()
                .enumerate()
                .min_by(|a, b| (1.0 - a.1.abs()).abs().total_cmp(&(1.0 - b.1.abs()).abs()))
                .map(|x| x.0)
                .unwrap_or(0);
            let r2 = if pr.is_empty() { 0.0 } else { pr.remove(j) };
            (-(r1 + r2), r1 * r2, (C::new(r1, 0.0), C::new(r2, 0.0)))
        };
        let (b1, b2) = if use_complex {
            if !zc.is_empty() {
                let c = zc.remove(nearest(&zc, p_for_zero.0));
                (-2.0 * c.re, c.norm_sqr())
            } else {
                let i = nearest_real(&zr, p_for_zero.0.re);
                let z1 = zr.remove(i);
                let j = nearest_real(&zr, p_for_zero.0.re);
                let z2 = zr.remove(j);
                (-(z1 + z2), z1 * z2)
            }
        } else if zr.len() >= 2 {
            let i = nearest_real(&zr, p_for_zero.0.re);
            let z1 = zr.remove(i);
            let j = nearest_real(&zr, p_for_zero.1.re);
            let z2 = zr.remove(j);
            (-(z1 + z2), z1 * z2)
        } else {
            let c = zc.remove(nearest(&zc, p_for_zero.0));
            (-2.0 * c.re, c.norm_sqr())
        };
        sections.push([1.0, b1, b2, 1.0, a1, a2]);
    }
    sections.reverse();
    for v in sections[0].iter_mut().take(3) {
        *v *= k;
    }
    sections
}

// ---------------------------------------------------------------------------------------------
// Design.

/// A digital filter from the order, the cutoffs (normalized, Nyquist = 1) and the method, as
/// `scipy.signal.iirfilter(..., output="zpk")` and `zpk2sos`.
pub fn iir_filter(
    order: usize,
    wn: &[f64],
    response: FilterResponse,
    method: IirMethod,
    rp: f64,
    rs: f64,
) -> Result<IirFilter, FilterError> {
    if order == 0 || order > MAX_ORDER {
        return Err(FilterError::BadOrder(format!("the filter order must be from 1 to {MAX_ORDER}, not {order}")));
    }
    let want = if response.is_band() { 2 } else { 1 };
    if wn.len() != want {
        return Err(FilterError::BadFrequency(format!("{} needs {want} cutoff frequencies", response.label())));
    }
    for &w in wn {
        if !(w.is_finite() && w > 0.0 && w < 1.0) {
            return Err(FilterError::BadFrequency(format!(
                "a cutoff frequency must be above 0 and below the Nyquist frequency (normalized {w})"
            )));
        }
    }
    if want == 2 && wn[0] >= wn[1] {
        return Err(FilterError::BadFrequency("the first cutoff frequency must be below the second".into()));
    }
    if matches!(method, IirMethod::Chebyshev1 | IirMethod::Elliptic) && !(rp.is_finite() && rp > 0.0) {
        return Err(FilterError::BadMagnitude(format!("the passband ripple must be above 0 dB, not {rp}")));
    }
    if matches!(method, IirMethod::Chebyshev2 | IirMethod::Elliptic) && !(rs.is_finite() && rs > 0.0) {
        return Err(FilterError::BadMagnitude(format!("the stopband attenuation must be above 0 dB, not {rs}")));
    }
    if method == IirMethod::Elliptic && rs <= rp {
        return Err(FilterError::BadMagnitude(format!(
            "the stopband attenuation ({rs} dB) must exceed the passband ripple ({rp} dB)"
        )));
    }
    let (z, p, k) = match method {
        IirMethod::Butterworth => buttap(order),
        IirMethod::Chebyshev1 => cheb1ap(order, rp),
        IirMethod::Chebyshev2 => cheb2ap(order, rs),
        IirMethod::Elliptic => ellipap(order, rp, rs),
    };
    let fs = 2.0;
    let warped: Vec<f64> = wn.iter().map(|w| 2.0 * fs * (PI * w / fs).tan()).collect();
    let (z, p, k) = match response {
        FilterResponse::LowPass => lp2lp(&z, &p, k, warped[0]),
        FilterResponse::HighPass => lp2hp(&z, &p, k, warped[0]),
        FilterResponse::BandPass => lp2bp(&z, &p, k, (warped[0] * warped[1]).sqrt(), warped[1] - warped[0]),
        FilterResponse::BandStop => lp2bs(&z, &p, k, (warped[0] * warped[1]).sqrt(), warped[1] - warped[0]),
    };
    let (z, p, k) = bilinear(&z, &p, k, fs);
    if !k.is_finite() || z.iter().chain(p.iter()).any(|c| !c.re.is_finite() || !c.im.is_finite()) {
        return Err(FilterError::NotFinite);
    }
    let sos = zpk2sos(&z, &p, k);
    if sos.iter().flatten().any(|v| !v.is_finite()) {
        return Err(FilterError::NotFinite);
    }
    Ok(IirFilter { order, wn: wn.to_vec(), zeros: z, poles: p, gain: k, sos })
}

fn warp(w: f64) -> f64 {
    (PI * w / 2.0).tan()
}

/// The ratio of stop to pass edges of the equivalent low pass (the "natural frequency" of
/// scipy's order functions; above 1).
fn natural(response: FilterResponse, passb: &[f64], stopb: &[f64]) -> f64 {
    match response {
        FilterResponse::LowPass => stopb[0] / passb[0],
        FilterResponse::HighPass => passb[0] / stopb[0],
        FilterResponse::BandStop => stopb
            .iter()
            .map(|s| (s * (passb[0] - passb[1]) / (s * s - passb[0] * passb[1])).abs())
            .fold(f64::INFINITY, f64::min),
        FilterResponse::BandPass => stopb
            .iter()
            .map(|s| ((s * s - passb[0] * passb[1]) / (s * (passb[0] - passb[1]))).abs())
            .fold(f64::INFINITY, f64::min),
    }
}

/// The order (a real number) for a given natural frequency.
fn order_for(method: IirMethod, nat: f64, gpass: f64, gstop: f64) -> f64 {
    let gs = 10f64.powf(0.1 * gstop.abs());
    let gp = 10f64.powf(0.1 * gpass.abs());
    match method {
        IirMethod::Butterworth => ((gs - 1.0) / (gp - 1.0)).log10() / (2.0 * nat.log10()),
        IirMethod::Chebyshev1 | IirMethod::Chebyshev2 => ((gs - 1.0) / (gp - 1.0)).sqrt().acosh() / nat.acosh(),
        IirMethod::Elliptic => {
            let arg1_sq = (gp - 1.0) / (gs - 1.0);
            let arg0 = 1.0 / nat;
            let d0 = (ellipk(arg0 * arg0), ellipk(1.0 - arg0 * arg0));
            let d1 = (ellipk(arg1_sq), ellipk(1.0 - arg1_sq));
            d0.0 * d1.1 / (d0.1 * d1.0)
        }
    }
}

/// Minimizes `f` on `[lo, hi]` (golden section; `scipy.optimize.fminbound` for the band stop).
fn minimize(f: impl Fn(f64) -> f64, mut lo: f64, mut hi: f64) -> f64 {
    let g = (5f64.sqrt() - 1.0) / 2.0;
    let (mut x1, mut x2) = (hi - g * (hi - lo), lo + g * (hi - lo));
    let (mut f1, mut f2) = (f(x1), f(x2));
    for _ in 0..200 {
        if (hi - lo).abs() < 1e-13 * hi.abs().max(1.0) {
            break;
        }
        if f1 < f2 {
            hi = x2;
            x2 = x1;
            f2 = f1;
            x1 = hi - g * (hi - lo);
            f1 = f(x1);
        } else {
            lo = x1;
            x1 = x2;
            f1 = f2;
            x2 = lo + g * (hi - lo);
            f2 = f(x2);
        }
    }
    (lo + hi) / 2.0
}

/// The minimum order and the cutoff frequencies (normalized) that meet a specification: the
/// gain is within `gpass` dB in the passband and at least `gstop` dB down in the stopband
/// (`scipy.signal.buttord`, `cheb1ord`, `cheb2ord`, `ellipord`).
pub fn iir_order(
    method: IirMethod,
    response: FilterResponse,
    wp: &[f64],
    ws: &[f64],
    gpass: f64,
    gstop: f64,
) -> Result<(usize, Vec<f64>), FilterError> {
    let want = if response.is_band() { 2 } else { 1 };
    if wp.len() != want || ws.len() != want {
        return Err(FilterError::BadFrequency(format!(
            "{} needs {want} pass and {want} stop frequencies",
            response.label()
        )));
    }
    for &w in wp.iter().chain(ws) {
        if !(w.is_finite() && w > 0.0 && w < 1.0) {
            return Err(FilterError::BadFrequency(
                "pass and stop frequencies must be above 0 and below the Nyquist frequency".into(),
            ));
        }
    }
    let ok = match response {
        FilterResponse::LowPass => wp[0] < ws[0],
        FilterResponse::HighPass => ws[0] < wp[0],
        FilterResponse::BandPass => ws[0] < wp[0] && wp[0] < wp[1] && wp[1] < ws[1],
        FilterResponse::BandStop => wp[0] < ws[0] && ws[0] < ws[1] && ws[1] < wp[1],
    };
    if !ok {
        return Err(FilterError::BadFrequency(match response {
            FilterResponse::LowPass => {
                "for a low pass filter the pass frequency must be below the stop frequency".into()
            }
            FilterResponse::HighPass => {
                "for a high pass filter the stop frequency must be below the pass frequency".into()
            }
            FilterResponse::BandPass => "for a band pass filter the stop band must enclose the pass band".into(),
            FilterResponse::BandStop => "for a band stop filter the pass band must enclose the stop band".into(),
        }));
    }
    if !(gpass.is_finite() && gpass > 0.0 && gstop.is_finite() && gstop > gpass) {
        return Err(FilterError::BadMagnitude(format!(
            "the stopband attenuation ({gstop} dB) must exceed the passband ripple ({gpass} dB), both above 0"
        )));
    }
    let mut passb: Vec<f64> = wp.iter().map(|&w| warp(w)).collect();
    let stopb: Vec<f64> = ws.iter().map(|&w| warp(w)).collect();
    if response == FilterResponse::BandStop {
        // scipy moves the pass edges toward the stop band to the place of the smallest order.
        let obj = |w: f64, ind: usize, passb: &[f64]| {
            let mut pb = passb.to_vec();
            pb[ind] = w;
            order_for(method, natural(response, &pb, &stopb), gpass, gstop)
        };
        let base = passb.clone();
        let w0 = minimize(|w| obj(w, 0, &base), base[0], stopb[0] - 1e-12);
        passb[0] = w0;
        let base = passb.clone();
        let w1 = minimize(|w| obj(w, 1, &base), stopb[1] + 1e-12, base[1]);
        passb[1] = w1;
    }
    let nat = natural(response, &passb, &stopb);
    let ord_f = order_for(method, nat, gpass, gstop);
    if !ord_f.is_finite() {
        return Err(FilterError::BadOrder("the minimum order is not finite for these frequencies".into()));
    }
    let n = ord_f.ceil().max(1.0);
    if n > MAX_ORDER as f64 {
        return Err(FilterError::BadOrder(format!("the minimum order is {n}, above the largest ({MAX_ORDER})")));
    }
    let n = n as usize;
    let gs = 10f64.powf(0.1 * gstop.abs());
    let gp = 10f64.powf(0.1 * gpass.abs());
    let back = |v: &[f64]| -> Vec<f64> { v.iter().map(|x| 2.0 / PI * x.atan()).collect() };
    let wn: Vec<f64> = match method {
        IirMethod::Chebyshev1 | IirMethod::Elliptic => back(&passb),
        IirMethod::Butterworth => {
            let w0 = (gp - 1.0).powf(-1.0 / (2.0 * n as f64));
            match response {
                FilterResponse::LowPass => back(&[w0 * passb[0]]),
                FilterResponse::HighPass => back(&[passb[0] / w0]),
                FilterResponse::BandStop => {
                    let d = passb[1] - passb[0];
                    let discr = (d * d + 4.0 * w0 * w0 * passb[0] * passb[1]).sqrt();
                    let mut v = [((d + discr) / (2.0 * w0)).abs(), ((d - discr) / (2.0 * w0)).abs()];
                    v.sort_by(f64::total_cmp);
                    back(&v)
                }
                FilterResponse::BandPass => {
                    let d = passb[1] - passb[0];
                    let mut v =
                        [-w0, w0].map(|w| (-w * d / 2.0 + (w * w / 4.0 * d * d + passb[0] * passb[1]).sqrt()).abs());
                    v.sort_by(f64::total_cmp);
                    back(&v)
                }
            }
        }
        IirMethod::Chebyshev2 => {
            let new_freq = 1.0 / ((1.0 / n as f64) * ((gs - 1.0) / (gp - 1.0)).sqrt().acosh()).cosh();
            let nat: Vec<f64> = match response {
                FilterResponse::LowPass => vec![passb[0] / new_freq],
                FilterResponse::HighPass => vec![passb[0] * new_freq],
                FilterResponse::BandStop => {
                    let d = passb[1] - passb[0];
                    let a = new_freq / 2.0 * (passb[0] - passb[1])
                        + (new_freq * new_freq * d * d / 4.0 + passb[1] * passb[0]).sqrt();
                    let mut v = [a, passb[1] * passb[0] / a];
                    v.sort_by(f64::total_cmp);
                    v.to_vec()
                }
                FilterResponse::BandPass => {
                    let d = passb[1] - passb[0];
                    let a = 1.0 / (2.0 * new_freq) * (passb[0] - passb[1])
                        + (d * d / (4.0 * new_freq * new_freq) + passb[1] * passb[0]).sqrt();
                    let mut v = [a, passb[0] * passb[1] / a];
                    v.sort_by(f64::total_cmp);
                    v.to_vec()
                }
            };
            back(&nat)
        }
    };
    Ok((n, wn))
}

/// The frequencies of a design in normalized units, with Sheaf's defaults.
struct Edges {
    nyq: f64,
    unit: FreqUnit,
}

impl Edges {
    fn new(unit: FreqUnit, sample_frequency: Option<f64>, auto: Option<f64>) -> Result<Edges, FilterError> {
        match unit {
            FreqUnit::Normalized => Ok(Edges { nyq: 1.0, unit }),
            FreqUnit::Hz => {
                let fs = sample_frequency.or(auto);
                match fs {
                    Some(fs) if fs.is_finite() && fs > 0.0 => Ok(Edges { nyq: fs / 2.0, unit }),
                    Some(fs) => Err(FilterError::BadFrequency(format!("the sample frequency {fs} is not usable"))),
                    None => Err(FilterError::BadFrequency(
                        "the sample frequency is unknown: give Fs or use a signal with an X column".into(),
                    )),
                }
            }
        }
    }

    /// `value` (or the default, a fraction of the Nyquist frequency) as a normalized frequency.
    fn norm(&self, name: &str, value: Option<f64>, default: f64) -> Result<f64, FilterError> {
        let w = match value {
            Some(v) => v / self.nyq,
            None => default,
        };
        if w.is_finite() && w > 0.0 && w < 1.0 {
            Ok(w)
        } else {
            let unit = if self.unit == FreqUnit::Hz { " Hz" } else { "" };
            Err(FilterError::BadFrequency(format!(
                "{name} ({}{unit}) must be above 0 and below the Nyquist frequency ({}{unit})",
                value.unwrap_or(f64::NAN),
                self.nyq
            )))
        }
    }
}

/// Sheaf's default pass and stop frequencies (fractions of the Nyquist frequency) for the
/// minimum order, and the default cutoffs. Unmeasured.
pub(crate) fn default_cutoffs(response: FilterResponse) -> (f64, f64) {
    let _ = response;
    (0.25, 0.75)
}

pub(crate) fn default_pass_stop(response: FilterResponse) -> ([f64; 2], [f64; 2]) {
    match response {
        FilterResponse::LowPass => ([0.2, 0.0], [0.3, 0.0]),
        FilterResponse::HighPass => ([0.3, 0.0], [0.2, 0.0]),
        FilterResponse::BandPass => ([0.3, 0.5], [0.2, 0.6]),
        FilterResponse::BandStop => ([0.2, 0.6], [0.3, 0.5]),
    }
}

/// The normalized pass and stop frequency lists of a specification (`pass`, `pass2`, `stop`,
/// `stop2`, with the defaults), shared with the FIR design.
pub(crate) struct PassStop {
    pub wp: Vec<f64>,
    pub ws: Vec<f64>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn pass_stop(
    response: FilterResponse,
    unit: FreqUnit,
    sample_frequency: Option<f64>,
    auto_fs: Option<f64>,
    pass: [Option<f64>; 2],
    stop: [Option<f64>; 2],
) -> Result<PassStop, FilterError> {
    let e = Edges::new(unit, sample_frequency, auto_fs)?;
    let (dp, ds) = default_pass_stop(response);
    let two = response.is_band();
    let mut wp = vec![e.norm(if two { "Pass Frequency 1" } else { "Pass Frequency" }, pass[0], dp[0])?];
    let mut ws = vec![e.norm(if two { "Stop Frequency 1" } else { "Stop Frequency" }, stop[0], ds[0])?];
    if two {
        wp.push(e.norm("Pass Frequency 2", pass[1], dp[1])?);
        ws.push(e.norm("Stop Frequency 2", stop[1], ds[1])?);
    }
    Ok(PassStop { wp, ws })
}

/// The normalized cutoffs of a specification (`cutoff`, `cutoff2`, with the defaults).
pub(crate) fn cutoffs(
    response: FilterResponse,
    unit: FreqUnit,
    sample_frequency: Option<f64>,
    auto_fs: Option<f64>,
    cutoff: Option<f64>,
    cutoff2: Option<f64>,
) -> Result<Vec<f64>, FilterError> {
    let e = Edges::new(unit, sample_frequency, auto_fs)?;
    let (d1, d2) = default_cutoffs(response);
    if response.is_band() {
        Ok(vec![e.norm("Cutoff Frequency 1", cutoff, d1)?, e.norm("Cutoff Frequency 2", cutoff2, d2)?])
    } else {
        Ok(vec![e.norm("Cutoff Frequency", cutoff, d1)?])
    }
}

impl IirSpec {
    /// Designs the filter. `auto_fs` is the sample frequency taken from the X column, used when
    /// the unit is Hz and no sample frequency is given.
    pub fn design(&self, auto_fs: Option<f64>) -> Result<IirFilter, FilterError> {
        match self.order {
            Some(n) => {
                let wn = cutoffs(self.response, self.unit, self.sample_frequency, auto_fs, self.cutoff, self.cutoff2)?;
                iir_filter(n, &wn, self.response, self.method, self.passband_ripple, self.stopband_attenuation)
            }
            None => {
                let ps = pass_stop(
                    self.response,
                    self.unit,
                    self.sample_frequency,
                    auto_fs,
                    [self.pass, self.pass2],
                    [self.stop, self.stop2],
                )?;
                let (n, wn) = iir_order(
                    self.method,
                    self.response,
                    &ps.wp,
                    &ps.ws,
                    self.passband_ripple,
                    self.stopband_attenuation,
                )?;
                iir_filter(n, &wn, self.response, self.method, self.passband_ripple, self.stopband_attenuation)
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Application.

/// Cascade of second-order sections in transposed direct form II (`scipy.signal.sosfilt`).
/// `zi` is the state of each section (two values), zero if `None`.
pub fn sosfilt(sos: &[[f64; 6]], x: &[f64], zi: Option<&[[f64; 2]]>) -> Vec<f64> {
    let mut y = x.to_vec();
    for (i, s) in sos.iter().enumerate() {
        let (b0, b1, b2, a1, a2) = (s[0], s[1], s[2], s[4], s[5]);
        let [mut z0, mut z1] = zi.map(|z| z[i]).unwrap_or([0.0, 0.0]);
        for v in y.iter_mut() {
            let xv = *v;
            let out = b0 * xv + z0;
            z0 = b1 * xv - a1 * out + z1;
            z1 = b2 * xv - a2 * out;
            *v = out;
        }
    }
    y
}

/// Initial state of a section for a constant input of 1, scaled through the cascade
/// (`scipy.signal.sosfilt_zi`).
fn sosfilt_zi(sos: &[[f64; 6]]) -> Vec<[f64; 2]> {
    let mut scale = 1.0;
    let mut out = Vec::new();
    for s in sos {
        let (b0, b1, b2, a1, a2) = (s[0], s[1], s[2], s[4], s[5]);
        // (I - A) zi = B with A the transposed companion matrix of the denominator.
        let (m00, m01, m10, m11) = (1.0 + a1, -1.0, a2, 1.0);
        let (r0, r1) = (b1 - a1 * b0, b2 - a2 * b0);
        let det = m00 * m11 - m01 * m10;
        let zi = [(r0 * m11 - m01 * r1) / det, (m00 * r1 - m10 * r0) / det];
        out.push([scale * zi[0], scale * zi[1]]);
        scale *= (b0 + b1 + b2) / (1.0 + a1 + a2);
    }
    out
}

/// `2 * x[0] - x[n..=1]`, `x`, `2 * x[last] - x[last-1..]`: the odd extension of `filtfilt`.
pub(crate) fn odd_ext(x: &[f64], n: usize) -> Vec<f64> {
    let len = x.len();
    let mut out = Vec::with_capacity(len + 2 * n);
    for i in (1..=n).rev() {
        out.push(2.0 * x[0] - x[i]);
    }
    out.extend_from_slice(x);
    for i in 1..=n {
        out.push(2.0 * x[len - 1] - x[len - 1 - i]);
    }
    out
}

/// Zero-phase filtering with second-order sections (`scipy.signal.sosfiltfilt`, odd padding of
/// `3 * ntaps` points).
pub fn sosfiltfilt(sos: &[[f64; 6]], x: &[f64]) -> Result<Vec<f64>, FilterError> {
    let mut ntaps = 2 * sos.len() + 1;
    let b2_zero = sos.iter().filter(|s| s[2] == 0.0).count();
    let a2_zero = sos.iter().filter(|s| s[5] == 0.0).count();
    ntaps -= b2_zero.min(a2_zero);
    let edge = 3 * ntaps;
    if x.len() <= edge {
        return Err(FilterError::TooShort { need: edge, got: x.len() });
    }
    let ext = odd_ext(x, edge);
    let zi = sosfilt_zi(sos);
    let scaled = |v: f64| -> Vec<[f64; 2]> { zi.iter().map(|z| [z[0] * v, z[1] * v]).collect() };
    let y = sosfilt(sos, &ext, Some(&scaled(ext[0])));
    let y0 = *y.last().unwrap();
    let mut rev: Vec<f64> = y.into_iter().rev().collect();
    rev = sosfilt(sos, &rev, Some(&scaled(y0)));
    rev.reverse();
    Ok(rev[edge..rev.len() - edge].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(got: f64, want: f64) {
        assert!((got - want).abs() <= 1e-9 * want.abs().max(1.0), "{got} vs {want}");
    }

    #[test]
    fn butterworth_low_pass_has_half_power_at_the_cutoff() {
        let f = iir_filter(4, &[0.3], FilterResponse::LowPass, IirMethod::Butterworth, 1.0, 40.0).unwrap();
        close(f.magnitude(0.3), 0.5f64.sqrt());
        close(f.magnitude(1e-9), 1.0);
        assert!(f.magnitude(0.9) < 1e-4);
    }

    #[test]
    fn the_four_responses_pass_and_stop_the_right_places() {
        for method in [IirMethod::Butterworth, IirMethod::Chebyshev1, IirMethod::Chebyshev2, IirMethod::Elliptic] {
            let lp = iir_filter(5, &[0.3], FilterResponse::LowPass, method, 1.0, 50.0).unwrap();
            assert!(lp.magnitude(0.01) > 0.85 && lp.magnitude(0.9) < 0.05, "{method:?} low pass");
            let hp = iir_filter(5, &[0.3], FilterResponse::HighPass, method, 1.0, 50.0).unwrap();
            assert!(hp.magnitude(0.99) > 0.85 && hp.magnitude(0.05) < 0.05, "{method:?} high pass");
            let bp = iir_filter(4, &[0.3, 0.5], FilterResponse::BandPass, method, 1.0, 50.0).unwrap();
            assert!(
                bp.magnitude(0.4) > 0.8 && bp.magnitude(0.05) < 0.05 && bp.magnitude(0.95) < 0.05,
                "{method:?} band pass"
            );
            let bs = iir_filter(4, &[0.3, 0.5], FilterResponse::BandStop, method, 1.0, 50.0).unwrap();
            assert!(
                bs.magnitude(0.4) < 0.2 && bs.magnitude(0.05) > 0.85 && bs.magnitude(0.95) > 0.85,
                "{method:?} band stop"
            );
        }
    }

    #[test]
    fn minimum_order_meets_the_specification() {
        for method in [IirMethod::Butterworth, IirMethod::Chebyshev1, IirMethod::Chebyshev2, IirMethod::Elliptic] {
            for (resp, wp, ws) in [
                (FilterResponse::LowPass, vec![0.2], vec![0.3]),
                (FilterResponse::HighPass, vec![0.3], vec![0.2]),
                (FilterResponse::BandPass, vec![0.3, 0.5], vec![0.2, 0.6]),
                (FilterResponse::BandStop, vec![0.2, 0.6], vec![0.3, 0.5]),
            ] {
                let (n, wn) = iir_order(method, resp, &wp, &ws, 1.0, 40.0).unwrap();
                let f = iir_filter(n, &wn, resp, method, 1.0, 40.0).unwrap();
                let pass_floor = 10f64.powf(-1.0 / 20.0) - 1e-6;
                let stop_ceiling = 10f64.powf(-40.0 / 20.0) + 1e-6;
                for w in &wp {
                    assert!(f.magnitude(*w) >= pass_floor, "{method:?} {resp:?} pass edge {w}: {}", f.magnitude(*w));
                }
                for w in &ws {
                    assert!(f.magnitude(*w) <= stop_ceiling, "{method:?} {resp:?} stop edge {w}: {}", f.magnitude(*w));
                }
            }
        }
    }

    #[test]
    fn forward_backward_has_no_phase_shift_and_one_way_has() {
        let f = iir_filter(2, &[0.1], FilterResponse::LowPass, IirMethod::Butterworth, 1.0, 40.0).unwrap();
        let x: Vec<f64> = (0..400).map(|i| (2.0 * PI * 0.004 * i as f64).sin()).collect();
        let zero = f.apply(&x, true).unwrap();
        let one = f.apply(&x, false).unwrap();
        let err = |y: &[f64]| (100..300).map(|i| (y[i] - x[i]).abs()).fold(0.0, f64::max);
        assert!(err(&zero) < 0.02, "{}", err(&zero));
        assert!(err(&one) > 0.05, "{}", err(&one));
    }

    #[test]
    fn short_signals_and_bad_frequencies_are_errors() {
        let f = iir_filter(4, &[0.3], FilterResponse::LowPass, IirMethod::Butterworth, 1.0, 40.0).unwrap();
        assert!(matches!(f.apply(&[1.0; 10], true), Err(FilterError::TooShort { .. })));
        assert!(f.apply(&[1.0; 10], false).is_ok());
        assert!(iir_filter(4, &[1.0], FilterResponse::LowPass, IirMethod::Butterworth, 1.0, 40.0).is_err());
        assert!(iir_filter(4, &[0.5, 0.4], FilterResponse::BandPass, IirMethod::Butterworth, 1.0, 40.0).is_err());
        assert!(iir_filter(0, &[0.5], FilterResponse::LowPass, IirMethod::Butterworth, 1.0, 40.0).is_err());
        assert!(iir_order(IirMethod::Butterworth, FilterResponse::LowPass, &[0.4], &[0.3], 1.0, 40.0).is_err());
        let spec = IirSpec { cutoff: Some(600.0), sample_frequency: Some(1000.0), ..IirSpec::default() };
        assert!(spec.design(None).is_err());
        assert!(IirSpec::default().design(None).is_err(), "Hz without a sample frequency");
    }

    #[test]
    fn the_spec_designs_in_hz() {
        let spec = IirSpec { cutoff: Some(50.0), sample_frequency: Some(1000.0), order: Some(4), ..IirSpec::default() };
        let f = spec.design(None).unwrap();
        close(f.wn[0], 0.1);
        let auto = IirSpec { cutoff: Some(50.0), order: Some(4), ..IirSpec::default() }.design(Some(1000.0)).unwrap();
        assert_eq!(f, auto);
    }
}
