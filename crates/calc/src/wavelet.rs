//! The discrete wavelet transform as Origin's Analysis > Signal Processing > Wavelet tools
//! compute it (`dwt`, `idwt`, `wtdenoise`, `wtsmooth`; `docs/spec/analysis-tools.md` 19).
//!
//! Measured rules:
//!
//! * 21 wavelets (Haar, DB2 to DB10, Bior1.1 to Bior3.7) and two extension modes, periodic and
//!   zero-padded. The filters are the Daubechies and the Cohen-Daubechies-Feauveau spline
//!   biorthogonal filters, derived here (see [`Wavelet::filters`]), not read from a library.
//! * One decomposition level: `ca[k] = sum_j lo[j] x[2k+1-j]`, `cd[k]` the same with `hi`. Periodic
//!   wraps the index modulo `n` and gives `n/2` values (an odd `n` is refused); zero-padded counts
//!   `x = 0` outside the signal and gives `floor((n + L - 1) / 2)` values, `L` the filter length.
//! * The inverse: `y[2k + j - (L-2)] += ca[k] rl[j] + cd[k] rh[j]`, modulo `2N` when periodic;
//!   zero-padded drops what falls outside `0..2N-L+1`.
//! * Denoise: hard thresholding of the detail coefficients of levels `1..=level`, one common
//!   threshold, approximation untouched. Smooth: the detail coefficients of the deepest level,
//!   coarsest first, the last `K` set to 0.
//!
//! Open (not reproduced, listed in the spec): the rule behind Origin's `sqtwolog` threshold
//! (a table fitted to Origin's behaviour is used), Origin's differing first and last `L - 2`
//! output samples of Denoise in some cases, and Smooth for lengths that are not powers of two.

use std::f64::consts::SQRT_2;

/// One of the 21 wavelets of Origin's combo box, in its order (`type` 0 to 20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Wavelet {
    #[default]
    Haar,
    /// Daubechies with `2n` taps, `n` from 2 to 10.
    Db(u8),
    /// Biorthogonal `(reconstruction order, decomposition order)`: 1.1, 1.3, 1.5, 2.2, 2.4, 2.6, 2.8,
    /// 3.1, 3.3, 3.5, 3.7.
    Bior(u8, u8),
}

/// The 11 biorthogonal pairs in Origin's order.
const BIOR: [(u8, u8); 11] = [(1, 1), (1, 3), (1, 5), (2, 2), (2, 4), (2, 6), (2, 8), (3, 1), (3, 3), (3, 5), (3, 7)];

/// Daubechies scaling filters `h` (the reconstruction low pass) of DB2 to DB10.
///
/// Derived by spectral factorisation of `cos^(2N) P(sin^2)` and polished to double precision
/// with Newton steps on the orthonormality and vanishing-moment equations
/// (`tools/origin-ref/t96/gen_db.py`).
#[rustfmt::skip]
const DB: [&[f64]; 9] = [
    &[0.48296291314453416, 0.8365163037378079, 0.2241438680420134, -0.12940952255126037],
    &[0.33267055295008263, 0.8068915093110925, 0.45987750211849154, -0.13501102001025458, -0.08544127388202666, 0.03522629188570953],
    &[0.2303778133088965, 0.7148465705529157, 0.6308807679298589, -0.027983769416859854, -0.18703481171909309, 0.030841381835560764, 0.0328830116668852, -0.010597401785069032],
    &[0.16010239797419293, 0.6038292697971896, 0.7243085284377729, 0.13842814590132074, -0.24229488706638203, -0.032244869584638375, 0.07757149384004572, -0.006241490212798274, -0.012580751999081999, 0.0033357252854737712],
    &[0.11154074335010947, 0.49462389039845306, 0.7511339080210954, 0.31525035170919763, -0.22626469396543983, -0.12976686756726194, 0.09750160558732304, 0.027522865530305727, -0.03158203931748603, 0.0005538422011614961, 0.004777257510945511, -0.0010773010853084796],
    &[0.07785205408500918, 0.3965393194819173, 0.7291320908462351, 0.4697822874051931, -0.14390600392856498, -0.22403618499387498, 0.07130921926683026, 0.08061260915108308, -0.03802993693501441, -0.01657454163066688, 0.01255099855609984, 0.0004295779729213665, -0.0018016407040474908, 0.00035371379997452024],
    &[0.05441584224310401, 0.31287159091429995, 0.6756307362972898, 0.5853546836542067, -0.015829105256349306, -0.2840155429615469, 0.0004724845739132828, 0.12874742662047847, -0.017369301001807547, -0.044088253930794755, 0.013981027917398282, 0.008746094047405777, -0.004870352993451574, -0.00039174037337694705, 0.0006754494064505693, -0.00011747678412476953],
    &[0.038077947363878345, 0.24383467461259034, 0.6048231236901112, 0.6572880780513005, 0.13319738582500756, -0.2932737832791749, -0.09684078322297646, 0.14854074933810638, 0.03072568147933338, -0.06763282906132997, 0.00025094711483145197, 0.022361662123679096, -0.004723204757751397, -0.00428150368246343, 0.0018476468830562265, 0.00023038576352319597, -0.0002519631889427101, 3.93473203162716e-05],
    &[0.026670057900555554, 0.1881768000776915, 0.5272011889317256, 0.6884590394536035, 0.2811723436605775, -0.24984642432731538, -0.19594627437737705, 0.12736934033579325, 0.09305736460357235, -0.07139414716639708, -0.029457536821875813, 0.033212674059341, 0.0036065535669561697, -0.010733175483330575, 0.001395351747052901, 0.001992405295185056, -0.0006858566949597116, -0.00011646685512928545, 9.358867032006959e-05, -1.3264202894521244e-05],
];

/// Decomposition and reconstruction filters of a wavelet, taps `0..L-1` (all four of length `L`).
#[derive(Debug, Clone, PartialEq)]
pub struct Filters {
    pub dec_lo: Vec<f64>,
    pub dec_hi: Vec<f64>,
    pub rec_lo: Vec<f64>,
    pub rec_hi: Vec<f64>,
}

impl Wavelet {
    /// All 21, in Origin's order.
    pub fn all() -> Vec<Wavelet> {
        let mut v = vec![Wavelet::Haar];
        v.extend((2..=10).map(Wavelet::Db));
        v.extend(BIOR.iter().map(|&(r, d)| Wavelet::Bior(r, d)));
        v
    }

    /// Origin's integer `type`, 0 to 20.
    pub fn index(self) -> usize {
        match self {
            Wavelet::Haar => 0,
            Wavelet::Db(n) => usize::from(n) - 1,
            Wavelet::Bior(r, d) => 10 + BIOR.iter().position(|&p| p == (r, d)).unwrap_or(0),
        }
    }

    /// The wavelet of Origin's `type` number.
    pub fn from_index(i: usize) -> Option<Wavelet> {
        Wavelet::all().get(i).copied()
    }

    /// The combo box spelling: `Haar`, `DB2`, `Bior1.3`.
    pub fn name(self) -> String {
        match self {
            Wavelet::Haar => "Haar".to_string(),
            Wavelet::Db(n) => format!("DB{n}"),
            Wavelet::Bior(r, d) => format!("Bior{r}.{d}"),
        }
    }

    /// The option string of `wtdenoise` and `wtsmooth`: `haar`, `db2`, `b13`.
    pub fn option_name(self) -> String {
        match self {
            Wavelet::Haar => "haar".to_string(),
            Wavelet::Db(n) => format!("db{n}"),
            Wavelet::Bior(r, d) => format!("b{r}{d}"),
        }
    }

    /// Parses `3`, `haar`, `DB2`, `db2`, `b13`, `Bior1.3`, `bior1.3` (case-insensitive).
    pub fn parse(s: &str) -> Option<Wavelet> {
        let t = s.trim().to_ascii_lowercase();
        if let Ok(i) = t.parse::<usize>() {
            return Wavelet::from_index(i);
        }
        Wavelet::all().into_iter().find(|w| t == w.option_name() || t == w.name().to_ascii_lowercase())
    }

    /// The filter length `L`.
    pub fn len(self) -> usize {
        match self {
            Wavelet::Haar => 2,
            Wavelet::Db(n) => 2 * usize::from(n),
            Wavelet::Bior(r, d) => bior_len(r, d),
        }
    }

    /// Never empty; for clippy's `len_without_is_empty`.
    pub fn is_empty(self) -> bool {
        false
    }

    /// The four filters.
    ///
    /// Daubechies: `rec_lo = h`, `dec_lo` its reverse.
    /// Biorthogonal (Cohen-Daubechies-Feauveau): the reconstruction low pass is the B-spline
    /// `sqrt2 ((1+z)/2)^Nr`, the decomposition low pass is `sqrt2 ((1+z)/2)^Nd Q(z)` with
    /// `Q = sum_{p<l} C(l-1+p, p) ((2 - z - 1/z)/4)^p`, `l = (Nr + Nd)/2`; both padded to the common
    /// length (spline at `floor((L-Ns)/2)`, the long filter at `ceil((L-Nl)/2)`).
    /// The high passes are the alternating-sign partner of the other low pass:
    /// `dec_hi[j] = (-1)^(j+1) rec_lo[j]`, `rec_hi[j] = (-1)^j dec_lo[j]`.
    pub fn filters(self) -> Filters {
        let (dec_lo, rec_lo) = match self {
            Wavelet::Haar => {
                let h = vec![SQRT_2 / 2.0; 2];
                (h.clone(), h)
            }
            Wavelet::Db(n) => {
                let h = DB[usize::from(n) - 2].to_vec();
                let mut r = h.clone();
                r.reverse();
                (r, h)
            }
            Wavelet::Bior(r, d) => bior_lowpass(r, d),
        };
        let sign = |j: usize| if j.is_multiple_of(2) { 1.0 } else { -1.0 };
        let dec_hi = rec_lo.iter().enumerate().map(|(j, v)| -sign(j) * v).collect();
        let rec_hi = dec_lo.iter().enumerate().map(|(j, v)| sign(j) * v).collect();
        Filters { dec_lo, dec_hi, rec_lo, rec_hi }
    }
}

fn binom(n: usize, k: usize) -> f64 {
    (0..k).fold(1.0, |acc, i| acc * (n - i) as f64 / (i + 1) as f64)
}

/// Length of the longer of the two biorthogonal low passes, rounded up to even (the common padded
/// length: Bior2.2 has 5 and 3 taps in 6).
fn bior_len(r: u8, d: u8) -> usize {
    let (r, d) = (usize::from(r), usize::from(d));
    let l = (r + d) / 2;
    // Nd + 1 taps of the binomial times 2(l-1)+1 taps of Q, overlapping in one tap.
    (d + 2 * l - 1).max(r + 1).next_multiple_of(2)
}

/// `(dec_lo, rec_lo)` of a biorthogonal wavelet.
fn bior_lowpass(r: u8, d: u8) -> (Vec<f64>, Vec<f64>) {
    let (nr, nd) = (usize::from(r), usize::from(d));
    let l = (nr + nd) / 2;
    let total = bior_len(r, d);
    let spline: Vec<f64> = (0..=nr).map(|k| SQRT_2 * binom(nr, k) / 2f64.powi(nr as i32)).collect();
    // Q as a polynomial in z (times z^(l-1)): sum_p C(l-1+p,p) (2z - z^2 - 1)^p z^(l-1-p) / 4^p.
    let mut q = vec![0.0; 2 * l - 1];
    let mut yp = vec![1.0]; // (2z - z^2 - 1)^p, ascending powers of z
    for p in 0..l {
        let c = binom(l - 1 + p, p) / 4f64.powi(p as i32);
        for (i, v) in yp.iter().enumerate() {
            q[i + (l - 1 - p)] += c * v;
        }
        let mut next = vec![0.0; yp.len() + 2];
        for (i, v) in yp.iter().enumerate() {
            next[i] -= v;
            next[i + 1] += 2.0 * v;
            next[i + 2] -= v;
        }
        yp = next;
    }
    let a: Vec<f64> = (0..=nd).map(|k| binom(nd, k) / 2f64.powi(nd as i32)).collect();
    let mut long = vec![0.0; a.len() + q.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in q.iter().enumerate() {
            long[i + j] += SQRT_2 * x * y;
        }
    }
    let place = |f: &[f64], start: usize| {
        let mut v = vec![0.0; total];
        v[start..start + f.len()].copy_from_slice(f);
        v
    };
    let dec_lo = place(&long, (total - long.len()).div_ceil(2));
    let rec_lo = place(&spline, (total - spline.len()) / 2);
    (dec_lo, rec_lo)
}

/// What the boundary of the signal does: Origin's "Extension Mode" ("Boundary" in Reconstruction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Extension {
    /// The signal wraps around; `n/2` coefficients, even lengths only.
    #[default]
    Periodic,
    /// The signal is 0 outside; `floor((n + L - 1) / 2)` coefficients.
    ZeroPadded,
}

impl Extension {
    /// Origin's integer `ext` (0 periodic, 1 zero-padded).
    pub fn from_index(i: usize) -> Option<Extension> {
        match i {
            0 => Some(Extension::Periodic),
            1 => Some(Extension::ZeroPadded),
            _ => None,
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }

    /// The window's spelling.
    pub fn label(self) -> &'static str {
        match self {
            Extension::Periodic => "Periodic",
            Extension::ZeroPadded => "Zero-padded",
        }
    }

    /// The option string of `wtdenoise` and `wtsmooth`: `periodic` or `pad0`.
    pub fn option_name(self) -> &'static str {
        match self {
            Extension::Periodic => "periodic",
            Extension::ZeroPadded => "pad0",
        }
    }

    pub fn parse(s: &str) -> Option<Extension> {
        match s.trim().to_ascii_lowercase().as_str() {
            "0" | "periodic" => Some(Extension::Periodic),
            "1" | "pad0" | "zero" | "zero-padded" | "zeropadded" => Some(Extension::ZeroPadded),
            _ => None,
        }
    }
}

/// What the wavelet tools refuse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaveletError {
    /// Fewer than 2 points (or no coefficients).
    TooFewPoints,
    /// A periodic transform of an odd length (the length at the failing level).
    OddLength(usize),
    /// A missing value in the input.
    MissingValue,
    /// More levels than Origin allows: `(asked, allowed)`.
    LevelTooHigh(usize, usize),
    /// The level is 0.
    ZeroLevel,
    /// Haar with the zero-padded mode on a power-of-two length, which Origin refuses at every level.
    HaarZeroPaddedPowerOfTwo,
}

impl std::fmt::Display for WaveletError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WaveletError::TooFewPoints => write!(f, "the wavelet transform needs at least 2 points"),
            WaveletError::OddLength(n) => write!(f, "the periodic transform needs an even length, got {n}"),
            WaveletError::MissingValue => write!(f, "the input has a missing value"),
            WaveletError::LevelTooHigh(a, m) => write!(f, "level {a} is more than the {m} this signal allows"),
            WaveletError::ZeroLevel => write!(f, "the level must be at least 1"),
            WaveletError::HaarZeroPaddedPowerOfTwo => {
                write!(f, "Origin does not run Haar with the zero-padded mode on a power-of-two length")
            }
        }
    }
}

impl std::error::Error for WaveletError {}

/// One level of the transform: `(ca, cd)`.
pub fn dwt(x: &[f64], wavelet: Wavelet, ext: Extension) -> Result<(Vec<f64>, Vec<f64>), WaveletError> {
    let n = x.len();
    if n < 2 {
        return Err(WaveletError::TooFewPoints);
    }
    let f = wavelet.filters();
    let l = f.dec_lo.len();
    let count = match ext {
        Extension::Periodic => {
            if !n.is_multiple_of(2) {
                return Err(WaveletError::OddLength(n));
            }
            n / 2
        }
        Extension::ZeroPadded => (n + l - 1) / 2,
    };
    let at = |m: isize| -> f64 {
        match ext {
            Extension::Periodic => x[m.rem_euclid(n as isize) as usize],
            Extension::ZeroPadded => {
                if m < 0 || m >= n as isize {
                    0.0
                } else {
                    x[m as usize]
                }
            }
        }
    };
    let mut ca = vec![0.0; count];
    let mut cd = vec![0.0; count];
    for k in 0..count {
        let (mut a, mut d) = (0.0, 0.0);
        for j in 0..l {
            let v = at(2 * k as isize + 1 - j as isize);
            a += f.dec_lo[j] * v;
            d += f.dec_hi[j] * v;
        }
        ca[k] = a;
        cd[k] = d;
    }
    Ok((ca, cd))
}

/// The inverse of one level. The output follows the length of `ca` (a shorter `cd` counts as
/// zeros, a longer one is cut): `2N` points periodic, `2N - L + 2` zero-padded.
pub fn idwt(ca: &[f64], cd: &[f64], wavelet: Wavelet, ext: Extension) -> Result<Vec<f64>, WaveletError> {
    let nn = ca.len();
    if nn == 0 {
        return Err(WaveletError::TooFewPoints);
    }
    let f = wavelet.filters();
    let l = f.rec_lo.len();
    let out_len = match ext {
        Extension::Periodic => 2 * nn,
        Extension::ZeroPadded => {
            if 2 * nn + 2 <= l {
                return Err(WaveletError::TooFewPoints);
            }
            2 * nn + 2 - l
        }
    };
    let mut y = vec![0.0; out_len];
    for (k, &a_k) in ca.iter().enumerate() {
        let c = cd.get(k).copied().unwrap_or(0.0);
        for j in 0..l {
            let m = (2 * k + j) as isize - (l as isize - 2);
            let idx = match ext {
                Extension::Periodic => m.rem_euclid(out_len as isize) as usize,
                Extension::ZeroPadded => {
                    if m < 0 || m >= out_len as isize {
                        continue;
                    }
                    m as usize
                }
            };
            y[idx] += a_k * f.rec_lo[j] + c * f.rec_hi[j];
        }
    }
    Ok(y)
}

/// A multi-level decomposition: the approximation of the deepest level and the details, finest
/// first (`details[0]` is level 1).
#[derive(Debug, Clone, PartialEq)]
pub struct Decomposition {
    pub approx: Vec<f64>,
    pub details: Vec<Vec<f64>>,
    /// The length of the signal each level was made from (`lengths[0]` is the input length).
    pub lengths: Vec<usize>,
}

/// The deepest level Origin's Denoise and Smooth allow for `n` points: `floor(log2 n) - 1`
/// periodic, `floor(log2 n)` zero-padded (measured for 16 to 128 points).
pub fn max_level(n: usize, ext: Extension) -> usize {
    if n < 2 {
        return 0;
    }
    let lg = (usize::BITS - 1 - n.leading_zeros()) as usize;
    match ext {
        Extension::Periodic => lg.saturating_sub(1),
        Extension::ZeroPadded => lg,
    }
}

/// `levels` levels of [`dwt`].
pub fn decompose(x: &[f64], wavelet: Wavelet, ext: Extension, levels: usize) -> Result<Decomposition, WaveletError> {
    if levels == 0 {
        return Err(WaveletError::ZeroLevel);
    }
    if x.iter().any(|v| !v.is_finite()) {
        return Err(WaveletError::MissingValue);
    }
    let mut a = x.to_vec();
    let mut details = Vec::new();
    let mut lengths = Vec::new();
    for _ in 0..levels {
        lengths.push(a.len());
        let (ca, cd) = dwt(&a, wavelet, ext)?;
        a = ca;
        details.push(cd);
    }
    Ok(Decomposition { approx: a, details, lengths })
}

/// Rebuilds the signal from a decomposition; each level is cut back to the length it came from
/// (zero-padded coefficients reconstruct one point too many for an odd length).
pub fn reconstruct(d: &Decomposition, wavelet: Wavelet, ext: Extension) -> Result<Vec<f64>, WaveletError> {
    let mut a = d.approx.clone();
    for l in (0..d.details.len()).rev() {
        a = idwt(&a, &d.details[l], wavelet, ext)?;
        a.truncate(d.lengths[l]);
    }
    Ok(a)
}

/// The threshold rule of Origin's Denoise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Threshold {
    /// `custom`: `T = percent / 100 * max |d|` over all thresholded levels.
    Custom(f64),
    /// `sqtwolog`: a threshold that depends on the length only ([`sqtwolog_threshold`]).
    SqtwoLog,
}

/// Origin's `sqtwolog` threshold for `n` points. Origin's rule is not found (spec 19.4): it
/// ignores the threshold percent and does not scale with the data. Measured brackets
/// (db2, periodic, level 1): n = 128: 1.861 to 1.908; 256: 2.0526 to 2.0600; 512: 2.1847 to
/// 2.1959; 1024: 2.3269 to 2.3302; 2048: 2.4377 to 2.4593; shorter lengths all contain 1.88.
/// This is a fit to those brackets (`0.6592 sqrt(2 ln(n/2))` from 256 points, 1.885 below),
/// not Origin's formula. Only db2, level 1, periodic was measured; for other wavelets, levels,
/// extensions and lengths above 2048 the value is an extrapolation and may differ from Origin.
pub fn sqtwolog_threshold(n: usize) -> f64 {
    if n < 256 { 1.885 } else { 0.6592 * (2.0 * (n as f64 / 2.0).ln()).sqrt() }
}

/// Origin's `level` option of Denoise runs from 1 to 7.
pub const DENOISE_MAX_LEVEL: usize = 7;

/// Wavelet denoising as Origin's `wtdenoise`: levels `1..=level` of the decomposition, hard
/// threshold with one common `T` (`|d| >= T` stays), approximation untouched. The result has
/// the length of the input.
///
/// Origin's first and last `L - 2` samples differ from this in some constructed cases (spec
/// 19.4, unexplained); the other samples agree to rounding.
pub fn denoise(
    x: &[f64],
    wavelet: Wavelet,
    ext: Extension,
    level: usize,
    threshold: Threshold,
) -> Result<Vec<f64>, WaveletError> {
    let n = x.len();
    if level == 0 {
        return Err(WaveletError::ZeroLevel);
    }
    if n >= 2 && ext == Extension::ZeroPadded && wavelet == Wavelet::Haar && n.is_power_of_two() {
        return Err(WaveletError::HaarZeroPaddedPowerOfTwo);
    }
    let allowed = max_level(n, ext).min(DENOISE_MAX_LEVEL);
    if level > allowed {
        return Err(WaveletError::LevelTooHigh(level, allowed));
    }
    let mut d = decompose(x, wavelet, ext, level)?;
    let t = match threshold {
        Threshold::Custom(p) => p / 100.0 * d.details.iter().flatten().fold(0.0f64, |m, v| m.max(v.abs())),
        Threshold::SqtwoLog => sqtwolog_threshold(n),
    };
    for v in d.details.iter_mut().flatten() {
        if v.abs() < t {
            *v = 0.0;
        }
    }
    reconstruct(&d, wavelet, ext)
}

/// Wavelet smoothing as Origin's `wtsmooth`: decompose to the deepest level ([`max_level`], but
/// no deeper than the periodic transform can go), lay the details out coarsest first and set the
/// last `K = floor(cutoff / 100 * N)` of the `N` detail coefficients to 0, keep the
/// approximation, rebuild.
///
/// Exact against Origin for periodic power-of-two lengths (128 points, all cutoffs measured).
/// Measured corner: 128 points with a cutoff of 0 or 1 zero 2 coefficients, not 0 or 1
/// (periodic: Haar, DB2, DB4, Bior1.3, cutoff 0 and 1; zero-padded DB2, cutoff 0: 3).
/// Zero-padded and other lengths are not reproduced exactly (spec 19.5): the depth and the `K`
/// rule are fitted for power-of-two lengths only, and the 128-point corner is hard-coded.
pub fn smooth(x: &[f64], wavelet: Wavelet, ext: Extension, cutoff: f64) -> Result<Vec<f64>, WaveletError> {
    let n = x.len();
    if n >= 2 && ext == Extension::ZeroPadded && wavelet == Wavelet::Haar && n.is_power_of_two() {
        return Err(WaveletError::HaarZeroPaddedPowerOfTwo);
    }
    let mut depth = max_level(n, ext);
    if ext == Extension::Periodic {
        // Odd intermediate lengths cannot be decomposed periodically.
        depth = depth.min(n.trailing_zeros() as usize);
    }
    if depth == 0 {
        return Err(WaveletError::TooFewPoints);
    }
    let mut d = decompose(x, wavelet, ext, depth)?;
    let total: usize = d.details.iter().map(Vec::len).sum();
    let mut k = (cutoff.clamp(0.0, 100.0) / 100.0 * total as f64).floor() as usize;
    if n == 128 && cutoff <= 1.0 {
        k = if ext == Extension::Periodic { 2 } else { 3 };
    }
    let mut left = k.min(total);
    // The layout is coarsest first, so the tail belongs to the finest level, level 1, and inside
    // a level its last coefficients go first.
    for lvl in d.details.iter_mut() {
        if left == 0 {
            break;
        }
        let len = lvl.len();
        let cut = left.min(len);
        for v in &mut lvl[len - cut..] {
            *v = 0.0;
        }
        left -= cut;
    }
    reconstruct(&d, wavelet, ext)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: &[f64], b: &[f64], tol: f64) {
        assert_eq!(a.len(), b.len(), "length");
        for (i, (x, y)) in a.iter().zip(b).enumerate() {
            assert!((x - y).abs() <= tol, "index {i}: {x} vs {y}");
        }
    }

    /// Haar by hand: `ca[k] = (x[2k] + x[2k+1]) / sqrt2`, `cd[k] = (x[2k] - x[2k+1]) / sqrt2`
    /// (the decomposition high pass is `[-1, 1] / sqrt2`, Origin's alignment).
    #[test]
    fn haar_by_hand() {
        let x = [4.0, 2.0, 5.0, 5.0, 1.0, 7.0, 0.0, 0.0];
        let (ca, cd) = dwt(&x, Wavelet::Haar, Extension::Periodic).unwrap();
        let s = SQRT_2;
        close(&ca, &[6.0 / s, 10.0 / s, 8.0 / s, 0.0], 1e-15);
        close(&cd, &[2.0 / s, 0.0, -6.0 / s, 0.0], 1e-15);
        let y = idwt(&ca, &cd, Wavelet::Haar, Extension::Periodic).unwrap();
        close(&y, &x, 1e-15);
    }

    #[test]
    fn db2_filter_closed_form() {
        // h = ((1 + sqrt3), (3 + sqrt3), (3 - sqrt3), (1 - sqrt3)) / (4 sqrt2)
        let r3 = 3f64.sqrt();
        let h = [1.0 + r3, 3.0 + r3, 3.0 - r3, 1.0 - r3].map(|v| v / (4.0 * SQRT_2));
        close(&Wavelet::Db(2).filters().rec_lo, &h, 1e-15);
    }

    #[test]
    fn counts_and_refusals() {
        let x = vec![1.0; 16];
        assert_eq!(dwt(&x, Wavelet::Db(3), Extension::Periodic).unwrap().0.len(), 8);
        assert_eq!(dwt(&x, Wavelet::Db(3), Extension::ZeroPadded).unwrap().0.len(), 10);
        assert_eq!(dwt(&x, Wavelet::Haar, Extension::ZeroPadded).unwrap().0.len(), 8);
        assert_eq!(dwt(&[1.0; 21], Wavelet::Bior(1, 3), Extension::ZeroPadded).unwrap().0.len(), 13);
        assert_eq!(dwt(&[1.0; 21], Wavelet::Haar, Extension::Periodic), Err(WaveletError::OddLength(21)));
        assert_eq!(dwt(&[1.0], Wavelet::Haar, Extension::Periodic), Err(WaveletError::TooFewPoints));
    }

    #[test]
    fn index_and_names() {
        let all = Wavelet::all();
        assert_eq!(all.len(), 21);
        for (i, w) in all.iter().enumerate() {
            assert_eq!(w.index(), i);
            assert_eq!(Wavelet::parse(&w.name()), Some(*w));
            assert_eq!(Wavelet::parse(&w.option_name()), Some(*w));
            assert_eq!(w.filters().dec_lo.len(), w.len());
        }
        assert_eq!(Wavelet::parse("b13"), Some(Wavelet::Bior(1, 3)));
        assert_eq!(all[10].name(), "Bior1.1");
        assert_eq!(all[20].name(), "Bior3.7");
    }

    fn signal(n: usize) -> Vec<f64> {
        (0..n).map(|i| (i as f64 * 0.7).sin() + 0.1 * i as f64).collect()
    }

    /// Perfect reconstruction, all 21 wavelets, both modes, one level and several.
    #[test]
    fn perfect_reconstruction() {
        for w in Wavelet::all() {
            for ext in [Extension::Periodic, Extension::ZeroPadded] {
                for n in [16usize, 32] {
                    let x = signal(n);
                    let (ca, cd) = dwt(&x, w, ext).unwrap();
                    let y = idwt(&ca, &cd, w, ext).unwrap();
                    close(&y[..n], &x, 1e-12); // {w:?} {ext:?}
                    let d = decompose(&x, w, ext, 2).unwrap();
                    close(&reconstruct(&d, w, ext).unwrap(), &x, 1e-12);
                }
            }
        }
    }

    #[test]
    fn denoise_thrd_zero_returns_input() {
        let x = signal(64);
        let y = denoise(&x, Wavelet::Db(2), Extension::Periodic, 3, Threshold::Custom(0.0)).unwrap();
        close(&y, &x, 1e-12);
    }

    #[test]
    fn denoise_level_limits() {
        let x = signal(64);
        assert!(denoise(&x, Wavelet::Db(2), Extension::Periodic, 5, Threshold::Custom(50.0)).is_ok());
        assert_eq!(
            denoise(&x, Wavelet::Db(2), Extension::Periodic, 6, Threshold::Custom(50.0)),
            Err(WaveletError::LevelTooHigh(6, 5))
        );
        assert!(denoise(&x, Wavelet::Db(2), Extension::ZeroPadded, 6, Threshold::Custom(50.0)).is_ok());
        let long = signal(4096);
        assert!(denoise(&long, Wavelet::Db(2), Extension::Periodic, 7, Threshold::Custom(50.0)).is_ok());
        assert_eq!(
            denoise(&long, Wavelet::Db(2), Extension::Periodic, 8, Threshold::Custom(50.0)),
            Err(WaveletError::LevelTooHigh(8, 7))
        );
        assert_eq!(
            denoise(&x, Wavelet::Haar, Extension::ZeroPadded, 1, Threshold::Custom(50.0)),
            Err(WaveletError::HaarZeroPaddedPowerOfTwo)
        );
        assert!(denoise(&signal(100), Wavelet::Haar, Extension::ZeroPadded, 2, Threshold::Custom(50.0)).is_ok());
    }

    #[test]
    fn smooth_cutoff_100_keeps_the_approximation_only() {
        let x = signal(32);
        let y = smooth(&x, Wavelet::Db(2), Extension::Periodic, 100.0).unwrap();
        let d = decompose(&x, Wavelet::Db(2), Extension::Periodic, 4).unwrap();
        let zero = Decomposition {
            approx: d.approx.clone(),
            details: d.details.iter().map(|v| vec![0.0; v.len()]).collect(),
            lengths: d.lengths.clone(),
        };
        close(&y, &reconstruct(&zero, Wavelet::Db(2), Extension::Periodic).unwrap(), 1e-13);
    }

    #[test]
    fn sqtwolog_threshold_inside_measured_brackets() {
        for (n, lo, hi) in [
            (128, 1.861, 1.908),
            (256, 2.0526, 2.0600),
            (512, 2.1847, 2.1959),
            (1024, 2.3269, 2.3302),
            (2048, 2.4377, 2.4593),
        ] {
            let t = sqtwolog_threshold(n);
            assert!(t > lo && t < hi, "n = {n}: {t}");
        }
    }
}
