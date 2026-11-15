//! Magnitude-squared coherence of two signals as Origin's Analysis > Signal Processing >
//! Coherence (`cohere`, measured; `docs/spec/analysis-tools.md` 13.3).

use crate::fft::{FftWindow, fft_window};
use rustfft::{FftPlanner, num_complex::Complex};

/// The window of each segment (Origin's seven FFT windows plus Gaussian and Kaiser).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum CoherenceWindow {
    #[default]
    Rectangle,
    Welch,
    Triangular,
    Bartlett,
    Hanning,
    Hamming,
    Blackman,
    /// `exp(-(alpha (i - c) / c)^2 / 2)` with `c = (n - 1) / 2`; `alpha = 0` is a flat window.
    Gaussian {
        alpha: f64,
    },
    /// `I0(beta sqrt(1 - (2 i / (n - 1) - 1)^2)) / I0(beta)`; `beta = 0` is a flat window.
    Kaiser {
        beta: f64,
    },
}

/// Options of [`coherence`], with Origin's defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoherenceOptions {
    /// FFT length: each segment is zero padded to it (Origin: 256).
    pub fft_len: usize,
    /// Segment length, 2 to `fft_len` (Origin: 256).
    pub win_len: usize,
    /// Points shared by consecutive segments, 0 to `win_len - 1` (Origin: 128).
    pub overlap: usize,
    pub window: CoherenceWindow,
    /// Sampling interval: the frequency step is `1 / (fft_len interval)` (Origin: 1).
    pub interval: f64,
}

impl Default for CoherenceOptions {
    fn default() -> Self {
        CoherenceOptions { fft_len: 256, win_len: 256, overlap: 128, window: CoherenceWindow::Rectangle, interval: 1.0 }
    }
}

/// The frequencies and the coherence at each, `fft_len / 2 + 1` rows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CoherenceResult {
    pub frequency: Vec<f64>,
    /// NaN where both spectra (or one of them) are zero.
    pub coherence: Vec<f64>,
}

/// The largest FFT length (the dialog's cap).
pub const MAX_FFT_LEN: usize = 1 << 24;

/// Modified Bessel function of the first kind, order 0 (power series).
fn bessel_i0(x: f64) -> f64 {
    let q = x * x / 4.0;
    let (mut term, mut sum) = (1.0, 1.0);
    for k in 1..5000 {
        term *= q / (k as f64 * k as f64);
        sum += term;
        if term < sum * 1e-17 {
            break;
        }
    }
    sum
}

/// The window values for `n` points.
pub fn coherence_window(window: CoherenceWindow, n: usize) -> Vec<f64> {
    let m = n.saturating_sub(1) as f64;
    let c = m / 2.0;
    match window {
        CoherenceWindow::Rectangle => fft_window(FftWindow::Rectangle, n),
        CoherenceWindow::Welch => fft_window(FftWindow::Welch, n),
        CoherenceWindow::Triangular => fft_window(FftWindow::Triangular, n),
        CoherenceWindow::Bartlett => fft_window(FftWindow::Bartlett, n),
        CoherenceWindow::Hanning => fft_window(FftWindow::Hanning, n),
        CoherenceWindow::Hamming => fft_window(FftWindow::Hamming, n),
        CoherenceWindow::Blackman => fft_window(FftWindow::Blackman, n),
        CoherenceWindow::Gaussian { alpha } => {
            (0..n).map(|i| (-0.5 * (alpha * (i as f64 - c) / c).powi(2)).exp()).collect()
        }
        CoherenceWindow::Kaiser { beta } => {
            let d = bessel_i0(beta);
            (0..n)
                .map(|i| {
                    let r = 2.0 * i as f64 / m - 1.0;
                    bessel_i0(beta * (1.0 - r * r).max(0.0).sqrt()) / d
                })
                .collect()
        }
    }
}

/// The magnitude-squared coherence `|Sxy|^2 / (Sxx Syy)` of `x` and `y`, the spectra summed over
/// the segments as Welch's method, all as measured in Origin.
///
/// The shorter signal is padded with zeros to the longer one's length, and a missing value (NaN)
/// counts as 0. Segments of `win_len` points start every `win_len - overlap` points; only whole
/// segments are used (a signal shorter than one segment is one zero padded segment, so the
/// coherence is 1). Each segment is multiplied by the window, zero padded to `fft_len` (no
/// detrending), and transformed; the rows are the frequencies `k / (fft_len interval)` for
/// `k = 0 ..= fft_len / 2`. Origin refuses a `win_len` above `fft_len` or below 2 and an overlap
/// of `win_len` or more; so does this function (`Err`).
pub fn coherence(x: &[f64], y: &[f64], o: &CoherenceOptions) -> Result<CoherenceResult, String> {
    if o.win_len < 2 || o.win_len > o.fft_len {
        return Err(format!("the window length must be between 2 and the FFT length {}", o.fft_len));
    }
    if o.fft_len > MAX_FFT_LEN {
        return Err(format!("the FFT length must be at most {MAX_FFT_LEN}"));
    }
    if o.overlap >= o.win_len {
        return Err("the overlap must be less than the window length".to_string());
    }
    if !(o.interval.is_finite() && o.interval > 0.0) {
        return Err(format!("{} is not a usable interval", o.interval));
    }
    let n = x.len().max(y.len());
    if n == 0 {
        return Err("no data".to_string());
    }
    let at = |v: &[f64], i: usize| v.get(i).copied().filter(|a| !a.is_nan()).unwrap_or(0.0);
    let step = o.win_len - o.overlap;
    let segments = if n < o.win_len { 1 } else { (n - o.win_len) / step + 1 };
    if let CoherenceWindow::Gaussian { alpha: v } | CoherenceWindow::Kaiser { beta: v } = o.window
        && !v.is_finite()
    {
        return Err(format!("{v} is not a usable window parameter"));
    }
    let w = coherence_window(o.window, o.win_len);
    if !w.iter().all(|v| v.is_finite()) {
        return Err("the window parameter gives a window that is not finite".to_string());
    }
    let half = o.fft_len / 2 + 1;
    let fft = FftPlanner::<f64>::new().plan_fft_forward(o.fft_len);
    let (mut sxx, mut syy) = (vec![0.0; half], vec![0.0; half]);
    let mut sxy = vec![Complex::new(0.0, 0.0); half];
    let segment = |v: &[f64], s: usize| -> Vec<Complex<f64>> {
        let mut b = vec![Complex::new(0.0, 0.0); o.fft_len];
        for (i, wi) in w.iter().enumerate() {
            if s + i < n {
                b[i] = Complex::new(at(v, s + i) * wi, 0.0);
            }
        }
        fft.process(&mut b);
        b
    };
    for k in 0..segments {
        let s = k * step;
        let (a, b) = (segment(x, s), segment(y, s));
        for j in 0..half {
            sxx[j] += a[j].norm_sqr();
            syy[j] += b[j].norm_sqr();
            sxy[j] += a[j] * b[j].conj();
        }
    }
    let df = 1.0 / (o.fft_len as f64 * o.interval);
    Ok(CoherenceResult {
        frequency: (0..half).map(|j| j as f64 * df).collect(),
        coherence: (0..half)
            .map(|j| if sxx[j] > 0.0 && syy[j] > 0.0 { sxy[j].norm_sqr() / sxx[j] / syy[j] } else { f64::NAN })
            .collect(),
    })
}

#[cfg(test)]
#[allow(clippy::excessive_precision)]
mod tests {
    use super::*;

    const X: [f64; 64] = [
        -1.179, -1.148, 0.669, -2.294, -0.143, -2.256, 1.101, 0.203, 1.356, -0.504, 0.398, -0.286, -0.738, 0.145,
        -1.257, -0.355, 0.697, 0.058, -0.41, 2.189, 0.058, -0.587, 0.16, -0.523, -0.385, -0.35, 2.025, 0.023, 0.176,
        0.674, 2.017, -0.224, -0.624, 2.475, -1.46, -0.367, 0.667, 2.284, -0.95, -2.428, 0.662, -0.522, -0.387, 0.461,
        0.224, 0.291, -0.431, 1.289, 1.505, 0.032, -0.452, 0.733, 0.478, -1.04, -0.46, 1.049, -0.096, -0.329, 0.212,
        -0.034, -0.002, -1.931, 1.749, 0.199,
    ];
    const Y: [f64; 64] = [
        -1.264, -1.257, -0.296, -1.062, -1.411, -0.563, -0.757, 1.815, 2.19, 0.222, -0.386, 0.506, -1.599, -0.623,
        -0.117, -0.733, 0.494, -0.349, 0.935, 1.48, 2.175, -0.117, -1.216, -0.956, -1.296, 0.406, 0.722, 1.515, 0.014,
        0.898, 2.201, 1.591, -1.247, -0.336, 1.041, -1.291, 0.789, 2.081, 0.857, -2.254, -2.256, -0.357, -1.192, 0.157,
        1.571, 0.765, 0.502, 0.677, 2.224, 1.002, -0.431, -1.063, 0.32, -0.989, -0.735, 0.698, 1.343, 0.471, -0.267,
        0.674, 0.949, -1.358, 0.223, 1.456,
    ];

    /// Origin's printed output (`cohere`; `name` is the probe sheet listed in the spec) against the
    /// function, to 1e-9.
    fn check(name: &str, x: &[f64], y: &[f64], o: &CoherenceOptions, want: &[f64]) {
        let r = coherence(x, y, o).unwrap();
        assert_eq!(r.coherence.len(), want.len(), "{name}");
        for (i, (g, w)) in r.coherence.iter().zip(want).enumerate() {
            if w.is_nan() {
                assert!(g.is_nan(), "{name} row {i}: {g}");
            } else {
                assert!((g - w).abs() < 1e-9, "{name} row {i}: {g} vs {w}");
            }
        }
    }

    #[test]
    fn windows_match_origin() {
        use CoherenceWindow::*;
        // Wrect
        check(
            "Wrect",
            &X,
            &Y,
            &CoherenceOptions {
                fft_len: 16,
                win_len: 16,
                overlap: 8,
                window: Rectangle,
                ..CoherenceOptions::default()
            },
            &[
                0.795800126404888,
                0.7778404419864591,
                0.7485758555543445,
                0.6105148633864366,
                0.9009128033737633,
                0.5967596640959499,
                0.23941788390935453,
                0.29586698051326876,
                0.011495276987301918,
            ],
        );
        // Wwelch
        check(
            "Wwelch",
            &X,
            &Y,
            &CoherenceOptions { fft_len: 16, win_len: 16, overlap: 8, window: Welch, ..CoherenceOptions::default() },
            &[
                0.8779352149567771,
                0.8179514558198947,
                0.8961697872592204,
                0.6240805436504822,
                0.8893997715346845,
                0.8151680861038991,
                0.21908802662221594,
                0.4916609532459406,
                0.37753549883068377,
            ],
        );
        // Wtri
        check(
            "Wtri",
            &X,
            &Y,
            &CoherenceOptions {
                fft_len: 16,
                win_len: 16,
                overlap: 8,
                window: Triangular,
                ..CoherenceOptions::default()
            },
            &[
                0.8362753817522542,
                0.8412792557513477,
                0.9015371852661309,
                0.6861248335388324,
                0.8216495747197962,
                0.8258915202409245,
                0.2718685908017399,
                0.4890223096892323,
                0.39209828132509,
            ],
        );
        // Wbartlett
        check(
            "Wbartlett",
            &X,
            &Y,
            &CoherenceOptions { fft_len: 16, win_len: 16, overlap: 8, window: Bartlett, ..CoherenceOptions::default() },
            &[
                0.8369254699397793,
                0.8355470922087116,
                0.9056281588258122,
                0.6960165870878998,
                0.7905531295677826,
                0.8357980795223606,
                0.29014216284142097,
                0.4771539798104317,
                0.3763151912893184,
            ],
        );
        // Whanning
        check(
            "Whanning",
            &X,
            &Y,
            &CoherenceOptions { fft_len: 16, win_len: 16, overlap: 8, window: Hanning, ..CoherenceOptions::default() },
            &[
                0.8222672716643267,
                0.8306914080839694,
                0.9031047814740493,
                0.7082571488591533,
                0.7568108269685078,
                0.842095832217385,
                0.28434879325427764,
                0.4871373008895487,
                0.55400801176573,
            ],
        );
        // Whamming
        check(
            "Whamming",
            &X,
            &Y,
            &CoherenceOptions { fft_len: 16, win_len: 16, overlap: 8, window: Hamming, ..CoherenceOptions::default() },
            &[
                0.8195054443079967,
                0.8366233059732125,
                0.9007594478516961,
                0.6952030767987739,
                0.8010748079386805,
                0.8283726867669928,
                0.25198735330731015,
                0.5110683066105209,
                0.5530585617150043,
            ],
        );
        // Wblackman
        check(
            "Wblackman",
            &X,
            &Y,
            &CoherenceOptions { fft_len: 16, win_len: 16, overlap: 8, window: Blackman, ..CoherenceOptions::default() },
            &[
                0.8197146618097313,
                0.8481634713337667,
                0.8837439053180075,
                0.7148325506243146,
                0.7359459172240677,
                0.8237092942394566,
                0.35645017121550276,
                0.3656932756209651,
                0.49283121545573005,
            ],
        );
        // Wgauss
        check(
            "Wgauss",
            &X,
            &Y,
            &CoherenceOptions {
                fft_len: 16,
                win_len: 16,
                overlap: 8,
                window: Gaussian { alpha: 2.5 },
                ..CoherenceOptions::default()
            },
            &[
                0.8135485090982855,
                0.8449691916991693,
                0.8943677545245472,
                0.7038824852766634,
                0.7804201839451322,
                0.828329413742685,
                0.2946543749163591,
                0.46914616153561695,
                0.5152761426510356,
            ],
        );
        // Wkaiser
        check(
            "Wkaiser",
            &X,
            &Y,
            &CoherenceOptions {
                fft_len: 16,
                win_len: 16,
                overlap: 8,
                window: Kaiser { beta: 4.0 },
                ..CoherenceOptions::default()
            },
            &[
                0.8454063744523842,
                0.828918300808075,
                0.9047332654052395,
                0.6707781896789461,
                0.8271020221650203,
                0.8338543166899018,
                0.22888460150066506,
                0.5248586620728315,
                0.4953746454678752,
            ],
        );
        // Wgauss0
        check(
            "Wgauss0",
            &X,
            &Y,
            &CoherenceOptions {
                fft_len: 16,
                win_len: 16,
                overlap: 8,
                window: Gaussian { alpha: 0.0 },
                ..CoherenceOptions::default()
            },
            &[
                0.795800126404888,
                0.7778404419864591,
                0.7485758555543445,
                0.6105148633864366,
                0.9009128033737633,
                0.5967596640959499,
                0.23941788390935453,
                0.29586698051326876,
                0.011495276987301918,
            ],
        );
        // Wkaiser0
        check(
            "Wkaiser0",
            &X,
            &Y,
            &CoherenceOptions {
                fft_len: 16,
                win_len: 16,
                overlap: 8,
                window: Kaiser { beta: 0.0 },
                ..CoherenceOptions::default()
            },
            &[
                0.795800126404888,
                0.7778404419864591,
                0.7485758555543445,
                0.6105148633864366,
                0.9009128033737633,
                0.5967596640959499,
                0.23941788390935453,
                0.29586698051326876,
                0.011495276987301918,
            ],
        );
    }

    #[test]
    fn segments_and_padding_match_origin() {
        use CoherenceWindow::*;
        // hanning, fft 32, window 16, overlap 4
        check(
            "Pad",
            &X,
            &Y,
            &CoherenceOptions { fft_len: 32, win_len: 16, overlap: 4, window: Hanning, ..CoherenceOptions::default() },
            &[
                0.7629396225673024,
                0.7713651047790082,
                0.8570796959210824,
                0.8922116809965654,
                0.824284482054165,
                0.7395272000794287,
                0.7571711778861011,
                0.7936428274867747,
                0.8983925351990463,
                0.8898376168344991,
                0.7947138661426386,
                0.5846107977570788,
                0.3066711324116043,
                0.34208067763117184,
                0.5374193964700036,
                0.6269563973867511,
                0.5373421015749251,
            ],
        );
        // no overlap
        check(
            "Ov0",
            &X,
            &Y,
            &CoherenceOptions { fft_len: 16, win_len: 16, overlap: 0, ..CoherenceOptions::default() },
            &[
                0.8489490434891392,
                0.8236530977713884,
                0.8495926891094789,
                0.7321224725674079,
                0.9261200938042669,
                0.6158148139796861,
                0.47119593318627556,
                0.17637652061817083,
                0.03215343990477565,
            ],
        );
        // overlap 15
        check(
            "Ov15",
            &X,
            &Y,
            &CoherenceOptions { fft_len: 16, win_len: 16, overlap: 15, ..CoherenceOptions::default() },
            &[
                0.8025911755222954,
                0.832375694804499,
                0.7891098574391888,
                0.5955144871627818,
                0.8489522389428602,
                0.7658204645749276,
                0.22921460381884765,
                0.4490843657411029,
                0.0025973051525724385,
            ],
        );
        // a partial last segment is dropped
        check(
            "Tail",
            &X,
            &Y,
            &CoherenceOptions { fft_len: 32, win_len: 20, overlap: 7, ..CoherenceOptions::default() },
            &[
                0.9121834780958183,
                0.9256429145925265,
                0.9236720424265379,
                0.867847596180161,
                0.8383999425455152,
                0.8826608408872333,
                0.7276054439652505,
                0.5586362274621498,
                0.8064305398704537,
                0.8212777057128064,
                0.6816759568885459,
                0.8963391734862313,
                0.3593314566890335,
                0.13561555976565276,
                0.7504958090429024,
                0.46847305899756364,
                0.1302515224225099,
            ],
        );
        // odd lengths
        check(
            "Odd",
            &X,
            &Y,
            &CoherenceOptions { fft_len: 15, win_len: 15, overlap: 5, ..CoherenceOptions::default() },
            &[
                0.8060807187752401,
                0.8621456471164904,
                0.7646429297304972,
                0.2606649126012803,
                0.8710089698253679,
                0.5418944296295795,
                0.1355575988199325,
                0.4340125267574723,
            ],
        );
        // odd lengths, hanning, padded
        check(
            "Odd2",
            &X,
            &Y,
            &CoherenceOptions { fft_len: 31, win_len: 15, overlap: 5, window: Hanning, ..CoherenceOptions::default() },
            &[
                0.8494203174126322,
                0.8880774748520573,
                0.9231728748750692,
                0.9319141438630711,
                0.9524364789817369,
                0.9601675188917256,
                0.9100574852802062,
                0.8728348988432398,
                0.8654475891958349,
                0.8370220202243472,
                0.6726494830622644,
                0.13941919296312985,
                0.34044497344888375,
                0.5537620981137198,
                0.6053491930807661,
                0.6571343214102542,
            ],
        );
        // window of 2
        check(
            "W2",
            &X,
            &Y,
            &CoherenceOptions { fft_len: 4, win_len: 2, overlap: 0, ..CoherenceOptions::default() },
            &[0.42088449320251864, 0.295147634007124, 0.07882595928379144],
        );
        // window of 2, overlap 1
        check(
            "Ov1",
            &X,
            &Y,
            &CoherenceOptions { fft_len: 16, win_len: 2, overlap: 1, ..CoherenceOptions::default() },
            &[
                0.5321812181435015,
                0.5190947093354942,
                0.48114427320174147,
                0.4213535563500923,
                0.3430355001709457,
                0.2504667479587516,
                0.15349943340247452,
                0.0739517930264709,
                0.04217205768240291,
            ],
        );
        // gaussian, window 9
        check(
            "Gauss3",
            &X,
            &Y,
            &CoherenceOptions {
                fft_len: 16,
                win_len: 9,
                overlap: 3,
                window: Gaussian { alpha: 3.0 },
                ..CoherenceOptions::default()
            },
            &[
                0.7739298052420932,
                0.7126444873209616,
                0.4597010449558619,
                0.23110654937342992,
                0.5210481571144157,
                0.6792000390205895,
                0.577011389870135,
                0.4165436360208377,
                0.35978365243674837,
            ],
        );
        // defaults (256, 256, 128) on 64 points: one segment, coherence 1
        check(
            "Dflt",
            &X,
            &Y,
            &CoherenceOptions { ..CoherenceOptions::default() },
            &[
                1.0,
                1.0,
                0.9999999999999999,
                1.0000000000000002,
                1.0000000000000002,
                0.9999999999999999,
                0.9999999999999999,
                1.0,
                1.0,
                0.9999999999999997,
                1.0000000000000002,
                1.0,
                1.0,
                0.9999999999999999,
                0.9999999999999996,
                1.0000000000000004,
                1.0000000000000002,
                0.9999999999999999,
                1.0,
                1.0,
                1.0000000000000002,
                0.9999999999999997,
                0.9999999999999998,
                1.0,
                1.0000000000000002,
                1.0,
                1.0000000000000004,
                1.0,
                0.9999999999999999,
                1.0000000000000002,
                0.9999999999999998,
                1.0000000000000002,
                1.0,
                1.0000000000000002,
                0.9999999999999997,
                1.0,
                0.9999999999999998,
                1.0,
                0.9999999999999998,
                1.0000000000000002,
                1.0000000000000002,
                1.0000000000000002,
                0.9999999999999998,
                1.0000000000000002,
                0.9999999999999999,
                1.0000000000000002,
                1.0,
                1.0000000000000002,
                0.9999999999999999,
                0.9999999999999998,
                1.0000000000000002,
                1.0,
                1.0000000000000002,
                1.0,
                1.0,
                1.0000000000000002,
                1.0,
                0.9999999999999998,
                1.0,
                0.9999999999999999,
                0.9999999999999998,
                1.0000000000000002,
                0.9999999999999998,
                1.0,
                1.0,
                0.9999999999999997,
                1.0000000000000002,
                1.0,
                0.9999999999999998,
                1.0000000000000002,
                1.0000000000000002,
                1.0,
                0.9999999999999999,
                0.9999999999999998,
                0.9999999999999999,
                0.9999999999999998,
                1.0000000000000002,
                1.0,
                1.0,
                1.0000000000000002,
                1.0,
                0.9999999999999999,
                1.0000000000000002,
                1.0000000000000002,
                0.9999999999999999,
                1.0000000000000002,
                0.9999999999999999,
                0.9999999999999999,
                1.0,
                1.0,
                0.9999999999999996,
                1.0,
                1.0000000000000002,
                1.0000000000000002,
                1.0,
                1.0000000000000002,
                1.0,
                1.0000000000000002,
                0.9999999999999999,
                0.9999999999999998,
                1.0000000000000002,
                1.0,
                1.0,
                1.0,
                0.9999999999999998,
                1.0,
                1.0000000000000002,
                1.0,
                0.9999999999999999,
                0.9999999999999997,
                0.9999999999999999,
                0.9999999999999999,
                1.0,
                1.0,
                1.0000000000000002,
                0.9999999999999999,
                1.0,
                1.0,
                0.9999999999999996,
                1.0,
                1.0000000000000002,
                1.0,
                0.9999999999999998,
                1.0000000000000002,
                1.0000000000000002,
                0.9999999999999999,
                0.9999999999999998,
                1.0000000000000002,
                1.0000000000000002,
            ],
        );
    }

    #[test]
    fn interval_sets_the_frequency_step() {
        let o = CoherenceOptions { fft_len: 16, win_len: 16, overlap: 8, interval: 0.5, ..Default::default() };
        let r = coherence(&X, &Y, &o).unwrap();
        let f = col_int();
        for (g, w) in r.frequency.iter().zip(f) {
            assert!((g - w).abs() < 1e-12, "{g} vs {w}");
        }
    }

    fn col_int() -> Vec<f64> {
        [0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875, 1.0].to_vec()
    }

    #[test]
    fn missing_values_and_short_signals_count_as_zero() {
        // 20 points with a missing value in each signal.
        let n = f64::NAN;
        let b = [1.0, 3.0, 2.0, 5.0, 4.0, 7.0, n, 8.0, 2.0, 9.0, 3.0, 4.0, 6.0, 2.0, 5.0, 1.0, 3.0, 7.0, 2.0, 4.0];
        let c = [2.0, 1.0, 4.0, 3.0, 6.0, 2.0, 7.0, 3.0, 8.0, 1.0, 5.0, 2.0, n, 6.0, 3.0, 2.0, 5.0, 1.0, 4.0, 3.0];
        let o = CoherenceOptions { fft_len: 8, win_len: 8, overlap: 4, ..Default::default() };
        check(
            "M1",
            &b,
            &c,
            &o,
            &[0.9963868903231117, 0.1980585582815266, 0.18792576932111818, 0.7829852410081815, 0.9037940816773005],
        );
        // A missing start.
        let b =
            [n, n, 1.0, 3.0, 2.0, 5.0, 4.0, 7.0, 6.0, 8.0, 2.0, 9.0, 3.0, 4.0, 6.0, 2.0, 5.0, 1.0, 3.0, 7.0, 2.0, 4.0];
        let c = [
            2.0, 3.0, 2.0, 1.0, 4.0, 3.0, 6.0, 2.0, 7.0, 3.0, 8.0, 1.0, 5.0, 2.0, 4.0, 6.0, 3.0, 2.0, 5.0, 1.0, 4.0,
            3.0,
        ];
        check(
            "N1",
            &b,
            &c,
            &o,
            &[0.992307896869137, 0.43401699112260916, 0.2673400673400674, 0.7940023270807384, 0.7245119305856833],
        );
        // The shorter signal (41 points) is padded with zeros to 64.
        let o = CoherenceOptions { fft_len: 16, win_len: 16, overlap: 8, ..Default::default() };
        check(
            "Len",
            &X,
            &Y[..41],
            &o,
            &[
                0.5635037018972613,
                0.7808474764422662,
                0.5579802192680294,
                0.7194776592909115,
                0.45129007077530503,
                0.48448702636106267,
                0.21312452667639878,
                0.4494343461423544,
                0.14360804663159762,
            ],
        );
        check(
            "Len2",
            &X[..41],
            &Y,
            &o,
            &[
                0.7903517297126117,
                0.5321875528262312,
                0.5998809893920968,
                0.5816482950273517,
                0.6411931214754154,
                0.5574930614114443,
                0.19254598821808974,
                0.36315660469939265,
                0.19381802380242483,
            ],
        );
        // Shorter than a segment: one segment, coherence 1.
        check(
            "Short",
            &X[..10],
            &Y[..10],
            &o,
            &[1.0, 0.9999999999999998, 1.0, 1.0, 1.0, 0.9999999999999996, 0.9999999999999998, 0.9999999999999997, 1.0],
        );
    }

    #[test]
    fn zero_and_identical_signals() {
        let o = CoherenceOptions { fft_len: 8, win_len: 8, overlap: 4, ..Default::default() };
        let ramp: Vec<f64> = (1..=20).map(f64::from).collect();
        let r = coherence(&ramp, &[0.0; 20], &o).unwrap();
        assert!(r.coherence.iter().all(|v| v.is_nan()), "a zero signal: NaN as Origin");
        let r = coherence(&ramp, &ramp, &o).unwrap();
        assert!(r.coherence.iter().all(|v| (v - 1.0).abs() < 1e-12));
    }

    #[test]
    fn bad_options_are_refused_as_origin() {
        let base = CoherenceOptions { fft_len: 16, win_len: 16, overlap: 8, ..Default::default() };
        for o in [
            CoherenceOptions { win_len: 32, ..base },
            CoherenceOptions { win_len: 1, overlap: 0, ..base },
            CoherenceOptions { overlap: 16, ..base },
            CoherenceOptions { interval: 0.0, ..base },
            CoherenceOptions { fft_len: (1 << 24) + 2, win_len: 16, ..base },
            CoherenceOptions { window: CoherenceWindow::Gaussian { alpha: f64::NAN }, ..base },
            CoherenceOptions { window: CoherenceWindow::Gaussian { alpha: f64::INFINITY }, ..base },
            CoherenceOptions { window: CoherenceWindow::Kaiser { beta: f64::NAN }, ..base },
            CoherenceOptions { window: CoherenceWindow::Kaiser { beta: 1e4 }, ..base },
        ] {
            assert!(coherence(&X, &Y, &o).is_err(), "{o:?}");
        }
    }

    #[test]
    fn extreme_amplitudes_do_not_overflow_the_ratio() {
        let o = CoherenceOptions { fft_len: 8, win_len: 8, overlap: 4, ..Default::default() };
        let x: Vec<f64> = X[..16].iter().map(|v| v * 1e60).collect();
        let r = coherence(&x, &x, &o).unwrap();
        assert!(r.coherence.iter().all(|v| v.is_nan() || (v - 1.0).abs() < 1e-9));
        assert!(r.coherence[0].is_finite());
    }
}
