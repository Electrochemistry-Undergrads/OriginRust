//! FFT of a real signal as Origin's Analysis > Signal Processing > FFT > FFT (`fft1`, measured;
//! `docs/spec/analysis-tools.md` 15): frequency, real and imaginary parts, magnitude, amplitude,
//! phase, power and dB.

use rustfft::{FftPlanner, num_complex::Complex};

/// The window applied before the transform. All are symmetric over the `N` points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FftWindow {
    #[default]
    Rectangle,
    Welch,
    Triangular,
    Bartlett,
    Hanning,
    Hamming,
    Blackman,
}

/// The factor that makes up for the window (Origin's Window Correction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowCorrection {
    None,
    /// `N / sum(w)`: amplitudes as without a window.
    #[default]
    Amplitude,
    /// `sqrt(N / sum(w^2))`: power as without a window.
    Power,
}

/// How the power is normalized (Origin's power density normalization).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PowerNorm {
    /// Mean square amplitude: `|X|^2 / N^2`.
    #[default]
    Msa,
    /// Sum square amplitude: `|X|^2 / N`.
    Ssa,
    /// Time interval square amplitude: `|X|^2 / N` times the sampling interval.
    Tisa,
}

/// Options of [`fft_spectrum`], with Origin's defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FftOptions {
    /// Sampling interval: the frequency step is `1 / (N * interval)`.
    pub interval: f64,
    pub window: FftWindow,
    pub correction: WindowCorrection,
    /// Divide the real and imaginary parts and the magnitude by `N`.
    pub normalize: bool,
    /// All `N` frequencies instead of 0 to the Nyquist frequency.
    pub two_sided: bool,
    /// Two-sided only: negative frequencies first, so that 0 is in the middle.
    pub shift: bool,
    /// Unwrap the phase so that it changes by less than 180 degrees from row to row.
    pub unwrap: bool,
    /// The +1 (science) sign of the exponent instead of -1 (electrical engineering).
    pub science: bool,
    pub power: PowerNorm,
}

impl Default for FftOptions {
    fn default() -> Self {
        FftOptions {
            interval: 1.0,
            window: FftWindow::Rectangle,
            correction: WindowCorrection::Amplitude,
            normalize: false,
            two_sided: false,
            shift: true,
            unwrap: true,
            science: false,
            power: PowerNorm::Msa,
        }
    }
}

/// The columns of an FFT result, one value per frequency.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FftResult {
    pub frequency: Vec<f64>,
    pub real: Vec<f64>,
    pub imaginary: Vec<f64>,
    pub magnitude: Vec<f64>,
    pub amplitude: Vec<f64>,
    /// Degrees.
    pub phase: Vec<f64>,
    pub power: Vec<f64>,
    /// `20 log10(amplitude)`; missing where the amplitude is 0.
    pub db: Vec<f64>,
}

/// The window values for `n` points.
pub fn fft_window(window: FftWindow, n: usize) -> Vec<f64> {
    let m = n.saturating_sub(1) as f64;
    let c = m / 2.0;
    let cos = |k: f64, i: f64| if m > 0.0 { (k * std::f64::consts::PI * i / m).cos() } else { 1.0 };
    (0..n)
        .map(|i| {
            let i = i as f64;
            match window {
                FftWindow::Rectangle => 1.0,
                FftWindow::Welch => 1.0 - ((i - c) / ((m + 2.0) / 2.0)).powi(2),
                FftWindow::Triangular => 1.0 - ((i - c) / ((m + 1.0) / 2.0)).abs(),
                FftWindow::Bartlett if m > 0.0 => 1.0 - ((i - c) / c).abs(),
                FftWindow::Bartlett => 1.0,
                FftWindow::Hanning => 0.5 - 0.5 * cos(2.0, i),
                FftWindow::Hamming => 0.54 - 0.46 * cos(2.0, i),
                FftWindow::Blackman => 0.42 - 0.5 * cos(2.0, i) + 0.08 * cos(4.0, i),
            }
        })
        .collect()
}

/// The FFT of `y` with Origin's outputs.
///
/// One-sided (the default) gives the frequencies 0 to `N/2` (integer division); there the
/// amplitude is `2|X|/N` except for the first and the last row, and the power twice the two-sided
/// power except for the first row (and four times in the last row for an odd `N`), all as measured
/// in Origin. Two-sided gives all `N`, with `|X|/N` and the plain power; shifted, the negative
/// frequencies come first, else the frequencies run up to `(N-1) / (N * interval)`. The phase is
/// unwrapped in row order.
pub fn fft_spectrum(y: &[f64], o: &FftOptions) -> FftResult {
    let n = y.len();
    if n == 0 {
        return FftResult::default();
    }
    let w = fft_window(o.window, n);
    let corr = match o.correction {
        WindowCorrection::None => 1.0,
        WindowCorrection::Amplitude => n as f64 / w.iter().sum::<f64>(),
        WindowCorrection::Power => (n as f64 / w.iter().map(|v| v * v).sum::<f64>()).sqrt(),
    };
    let corr = if corr.is_finite() { corr } else { 1.0 };
    let mut buf: Vec<Complex<f64>> = y.iter().zip(&w).map(|(v, w)| Complex::new(v * w * corr, 0.0)).collect();
    FftPlanner::<f64>::new().plan_fft_forward(n).process(&mut buf);
    let nf = n as f64;
    let order: Vec<usize> = if !o.two_sided {
        (0..=n / 2).collect()
    } else if o.shift {
        let first = n / 2 + 1;
        (first..n).chain(0..first).collect()
    } else {
        (0..n).collect()
    };
    let last = order.len() - 1;
    let mut r = FftResult::default();
    for (row, &k) in order.iter().enumerate() {
        let x = buf[k];
        let im = if o.science { -x.im } else { x.im };
        let mag = x.norm();
        let scale = if o.normalize { nf } else { 1.0 };
        let signed = if o.two_sided && o.shift && k > n / 2 { k as f64 - nf } else { k as f64 };
        r.frequency.push(signed / (nf * o.interval));
        r.real.push(x.re / scale);
        r.imaginary.push(im / scale);
        r.magnitude.push(mag / scale);
        let doubled = !o.two_sided && row != 0;
        let amp = mag / nf * if doubled && row != last { 2.0 } else { 1.0 };
        r.amplitude.push(amp);
        r.phase.push(im.atan2(x.re).to_degrees());
        // Measured: for an odd N Origin's last one-sided power row is four times the plain power.
        let factor = match (doubled, row == last && !n.is_multiple_of(2)) {
            (false, _) => 1.0,
            (true, false) => 2.0,
            (true, true) => 4.0,
        };
        let msa = mag * mag / (nf * nf) * factor;
        r.power.push(match o.power {
            PowerNorm::Msa => msa,
            PowerNorm::Ssa => msa * nf,
            PowerNorm::Tisa => msa * nf * o.interval,
        });
        let db = 20.0 * amp.log10();
        r.db.push(if db.is_finite() { db } else { f64::NAN });
    }
    if o.unwrap {
        unwrap_degrees(&mut r.phase);
    }
    r
}

/// Origin's phase unwrapping in row order: a step above 180 degrees loses 360, a step of -180 or
/// less gains 360 (measured: 0, 180, 0 becomes 0, 180, 360).
fn unwrap_degrees(phase: &mut [f64]) {
    // SIMD FFT backends and atan2 can place a half-turn a few ULPs on either side of
    // 180. Treat that roundoff as the measured positive half-turn tie, without changing
    // genuinely nearby phases. Reduction also avoids repeated loops for accumulated turns.
    const HALF_TURN_ROUNDOFF: f64 = 8.0 * f64::EPSILON * 180.0;
    for i in 1..phase.len() {
        let prev = phase[i - 1];
        let p = phase[i];
        if !prev.is_finite() || !p.is_finite() {
            continue;
        }
        let delta = (p - prev).rem_euclid(360.0);
        let step = if (delta - 180.0).abs() <= HALF_TURN_ROUNDOFF {
            180.0
        } else if delta > 180.0 {
            delta - 360.0
        } else {
            delta
        };
        phase[i] = prev + step;
    }
}

/// Options of [`ifft_spectrum`], with Origin's defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IfftOptions {
    /// Sampling interval of the input (its frequency step): the time step is `1 / (N * interval)`.
    pub interval: f64,
    /// The input is a shifted spectrum (negative frequencies first, as [`fft_spectrum`] writes it):
    /// put it back in FFT order first.
    pub shift: bool,
    pub unwrap: bool,
    /// The +1 (science) sign of the forward transform, so the inverse uses -1.
    pub science: bool,
}

impl Default for IfftOptions {
    fn default() -> Self {
        IfftOptions { interval: 1.0, shift: true, unwrap: true, science: false }
    }
}

/// The columns of an inverse FFT result.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IfftResult {
    pub time: Vec<f64>,
    pub real: Vec<f64>,
    pub imaginary: Vec<f64>,
    /// `|x|`.
    pub amplitude: Vec<f64>,
    /// Degrees.
    pub phase: Vec<f64>,
}

/// The inverse FFT of the spectrum `re + i im` (`im` missing: 0), as Origin's FFT > IFFT
/// (`ifft1`, measured): `x_n = (1/N) sum X_k e^(2 pi i k n / N)`.
pub fn ifft_spectrum(re: &[f64], im: Option<&[f64]>, o: &IfftOptions) -> IfftResult {
    let n = re.len();
    if n == 0 {
        return IfftResult::default();
    }
    let at = |k: usize| Complex::new(re[k], im.and_then(|v| v.get(k)).copied().unwrap_or(0.0));
    // A shifted spectrum holds frequency k at row (k - (N/2 + 1)) mod N.
    let mut buf: Vec<Complex<f64>> =
        (0..n).map(|k| if o.shift { at((k + n - (n / 2 + 1) % n) % n) } else { at(k) }).collect();
    let mut planner = FftPlanner::<f64>::new();
    if o.science {
        planner.plan_fft_forward(n).process(&mut buf);
    } else {
        planner.plan_fft_inverse(n).process(&mut buf);
    }
    let nf = n as f64;
    let mut r = IfftResult::default();
    for (i, c) in buf.iter().enumerate() {
        let c = c / nf;
        r.time.push(i as f64 / (nf * o.interval));
        r.real.push(c.re);
        r.imaginary.push(c.im);
        r.amplitude.push(c.norm());
        r.phase.push(c.im.atan2(c.re).to_degrees());
    }
    if o.unwrap {
        unwrap_degrees(&mut r.phase);
    }
    r
}

/// The filters of Origin's Signal Processing > FFT Filters (`fft_filters`, measured;
/// analysis-tools.md 15.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FftFilterType {
    #[default]
    LowPass,
    HighPass,
    BandPass,
    BandBlock,
    /// Keeps the frequencies whose magnitude `|X|` reaches the threshold.
    Threshold,
    /// Low pass with a parabolic edge between the pass and the stop frequency.
    LowPassParabolic,
}

/// Options of [`fft_filter`]; a `None` frequency or threshold is Origin's automatic value.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FftFilterOptions {
    pub kind: FftFilterType,
    /// Low and high pass (automatic: a quarter of the Nyquist frequency).
    pub cutoff: Option<f64>,
    /// Band pass and band block (automatic: a quarter and three quarters of the Nyquist frequency).
    pub freq1: Option<f64>,
    pub freq2: Option<f64>,
    /// Threshold (automatic: a tenth of the largest `|X|`, the zero frequency included).
    pub threshold: Option<f64>,
    /// Parabolic low pass (automatic: as `freq1` and `freq2`).
    pub pass: Option<f64>,
    pub stop: Option<f64>,
    /// High pass, band pass, band block: keep the zero frequency (the mean; Origin's "Keep DC
    /// offset", on by default).
    pub keep_offset: bool,
}

/// Filters `y`, sampled every `interval`, in the frequency domain as Origin's FFT Filters: each
/// frequency `f = k / (N interval)` (as `k` times the step, both signs) is kept, removed or, for the
/// parabolic low pass, weighted by `1 - ((f - pass) / (stop - pass))^2`, and the real part of the
/// inverse transform returned. Low pass keeps `f <= cutoff`, high pass `f >= cutoff`, band pass
/// `freq1 <= f <= freq2`; band block removes `freq1 <= f < freq2 + step` (measured: one step more
/// than band pass keeps).
pub fn fft_filter(y: &[f64], interval: f64, o: &FftFilterOptions) -> Vec<f64> {
    let n = y.len();
    if n == 0 {
        return Vec::new();
    }
    let mut buf: Vec<Complex<f64>> = y.iter().map(|&v| Complex::new(v, 0.0)).collect();
    let mut planner = FftPlanner::<f64>::new();
    planner.plan_fft_forward(n).process(&mut buf);
    let step = 1.0 / (n as f64 * interval);
    let nyquist = 1.0 / (2.0 * interval);
    let (low, high) = (nyquist / 4.0, nyquist * 3.0 / 4.0);
    let max = buf.iter().map(|c| c.norm()).fold(0.0, f64::max);
    let weights: Vec<f64> = (0..n)
        .map(|k| {
            let f = k.min(n - k) as f64 * step;
            let keep = |b: bool| if b { 1.0 } else { 0.0 };
            match o.kind {
                FftFilterType::LowPass => keep(f <= o.cutoff.unwrap_or(low)),
                FftFilterType::HighPass if k == 0 => keep(o.keep_offset),
                FftFilterType::HighPass => keep(f >= o.cutoff.unwrap_or(low)),
                FftFilterType::BandPass | FftFilterType::BandBlock if k == 0 => keep(o.keep_offset),
                FftFilterType::BandPass => keep(f >= o.freq1.unwrap_or(low) && f <= o.freq2.unwrap_or(high)),
                FftFilterType::BandBlock => keep(!(f >= o.freq1.unwrap_or(low) && f < o.freq2.unwrap_or(high) + step)),
                FftFilterType::Threshold => keep(buf[k].norm() >= o.threshold.unwrap_or(max / 10.0)),
                FftFilterType::LowPassParabolic => {
                    let (p, s) = (o.pass.unwrap_or(low), o.stop.unwrap_or(high));
                    if f <= p {
                        1.0
                    } else if f >= s {
                        0.0
                    } else {
                        1.0 - ((f - p) / (s - p)).powi(2)
                    }
                }
            }
        })
        .collect();
    for (c, w) in buf.iter_mut().zip(&weights) {
        *c *= *w;
    }
    planner.plan_fft_inverse(n).process(&mut buf);
    buf.iter().map(|c| c.re / n as f64).collect()
}

#[cfg(test)]
#[allow(clippy::excessive_precision)] // the digits as Origin printed them
mod tests {
    use super::*;

    const X: [f64; 8] = [1.0, 3.0, -2.0, 4.0, 0.0, 5.0, 2.0, -1.0];

    fn close(got: &[f64], want: &[f64]) {
        assert_eq!(got.len(), want.len(), "{got:?} vs {want:?}");
        for (g, w) in got.iter().zip(want) {
            assert!((g - w).abs() < 1e-9 * w.abs().max(1.0), "{got:?} vs {want:?}");
        }
    }

    #[test]
    fn default_fft_matches_origin() {
        let r = fft_spectrum(&X, &FftOptions::default());
        close(&r.frequency, &[0.0, 0.125, 0.25, 0.375, 0.5]);
        close(&r.real, &[12.0, -3.9497474683058327, 1.0, 5.9497474683058327, -10.0]);
        close(&r.imaginary, &[0.0, 1.8786796564403572, -5.0, -6.1213203435596428, 0.0]);
        close(&r.magnitude, &[12.0, 4.3737789513087186, 5.0990195135927845, 8.53639605952587, 10.0]);
        close(&r.amplitude, &[1.5, 1.0934447378271797, 1.2747548783981961, 2.1340990148814676, 1.25]);
        close(&r.phase, &[0.0, 154.56211291297552, 281.30993247402023, 314.18567769913909, 180.0]);
        close(&r.power, &[2.25, 0.59781069734097481, 0.81249999999999989, 2.2771893026590253, 3.125]);
        close(
            &r.db,
            &[3.5218251811136247, 0.77593677786764281, 2.1085336531489314, 6.5842913066337685, 1.9382002601611283],
        );
        // Seven points: frequencies k/7, the last amplitude not doubled, its power doubled.
        let r = fft_spectrum(&X[..7], &FftOptions::default());
        close(&r.frequency, &[0.0, 1.0 / 7.0, 2.0 / 7.0, 3.0 / 7.0]);
        close(&r.amplitude, &[1.857142857142857, 1.2313945124171892, 0.26897322716568739, 1.44659122001745]);
        close(&r.power, &[3.4489795918367343, 0.75816622260558342, 0.036173298465962241, 8.3705046313262983]);
        close(&r.phase, &[0.0, 92.047577099547581, 250.02770849687278, 255.56383428069648]);
        let r = fft_spectrum(&[2.0, -1.0, 4.0, 3.0, 0.5], &FftOptions::default());
        close(&r.amplitude, &[1.7000000000000002, 1.5634763340709921, 0.9843197845212619]);
        close(&r.power, &[2.89, 1.222229123600034, 3.8755417527999332]);
    }

    #[test]
    fn two_sided_shift_and_other_options_match_origin() {
        let o = FftOptions { two_sided: true, ..Default::default() };
        let r = fft_spectrum(&X, &o);
        close(&r.frequency, &[-0.375, -0.25, -0.125, 0.0, 0.125, 0.25, 0.375, 0.5]);
        close(
            &r.phase,
            &[
                45.814322300860894,
                78.690067525979785,
                205.43788708702448,
                360.0,
                514.56211291297552,
                641.30993247402023,
                674.18567769913909,
                540.0,
            ],
        );
        close(&r.amplitude[3..], &[1.5, 0.54672236891358983, 0.63737743919909806, 1.0670495074407338, 1.25]);
        close(&r.power[3..], &[2.25, 0.2989053486704874, 0.40624999999999994, 1.1385946513295127, 1.5625]);
        let r = fft_spectrum(&X, &FftOptions { shift: false, ..o });
        close(&r.frequency, &[0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875]);
        close(&r.phase[4..], &[180.0, 45.814322300860894, 78.690067525979785, 205.43788708702448]);
        let r = fft_spectrum(&X, &FftOptions { normalize: true, ..Default::default() });
        close(&r.real, &[1.5, -0.49371843353822908, 0.125, 0.74371843353822908, -1.25]);
        close(&r.amplitude, &[1.5, 1.0934447378271797, 1.2747548783981961, 2.1340990148814676, 1.25]);
        let r = fft_spectrum(&X, &FftOptions { science: true, ..Default::default() });
        close(&r.phase, &[0.0, -154.56211291297552, -281.30993247402023, -314.18567769913909, -180.0]);
        let r = fft_spectrum(&X, &FftOptions { unwrap: false, ..Default::default() });
        close(&r.phase, &[0.0, 154.56211291297552, -78.690067525979785, -45.814322300860894, 180.0]);
        let r = fft_spectrum(&X, &FftOptions { interval: 0.5, power: PowerNorm::Tisa, ..Default::default() });
        close(&r.frequency, &[0.0, 0.25, 0.5, 0.75, 1.0]);
        close(&r.power, &[9.0, 2.3912427893638992, 3.2499999999999996, 9.1087572106361012, 12.5]);
        let r = fft_spectrum(&X, &FftOptions { power: PowerNorm::Ssa, ..Default::default() });
        close(&r.power, &[18.0, 4.7824855787277984, 6.4999999999999991, 18.217514421272202, 25.0]);
    }

    #[test]
    fn inverse_fft_matches_origin() {
        let spec = fft_spectrum(&X, &FftOptions { two_sided: true, shift: false, ..Default::default() });
        let unshifted = IfftOptions { shift: false, ..Default::default() };
        let r = ifft_spectrum(&spec.real, Some(&spec.imaginary), &unshifted);
        close(&r.time, &[0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875]);
        close(&r.real, &X);
        close(&r.imaginary, &[0.0; 8]);
        close(&r.amplitude, &[1.0, 3.0, 2.0, 4.0, 0.0, 5.0, 2.0, 1.0]);
        close(&r.phase, &[0.0, 0.0, 180.0, 360.0, 360.0, 360.0, 360.0, 540.0]);
        // Shift on an unshifted spectrum: Origin rotates it as if it were shifted.
        let r = ifft_spectrum(&spec.real, Some(&spec.imaginary), &IfftOptions::default());
        close(
            &r.real,
            &[1.0, -2.1213203435596428, 0.0, 2.82842712474619, 0.0, 3.5355339059327378, 0.0, 0.70710678118654746],
        );
        close(
            &r.imaginary,
            &[0.0, -2.1213203435596428, -2.0, -2.82842712474619, 0.0, 3.5355339059327378, -2.0, -0.70710678118654746],
        );
        close(&r.phase, &[0.0, -135.0, -90.0, -45.0, 0.0, 45.0, -90.0, -45.0]);
        let r = ifft_spectrum(&spec.real, Some(&spec.imaginary), &IfftOptions { science: true, ..unshifted });
        close(&r.real, &[1.0, -1.0, 2.0, 5.0, 0.0, 4.0, -2.0, 3.0]);
        close(&r.phase, &[0.0, 180.0, 360.0, 360.0, 360.0, 360.0, 540.0, 720.0]);
        // The real part alone, interval 0.25.
        let r = ifft_spectrum(&spec.real, None, &IfftOptions { interval: 0.25, ..unshifted });
        close(&r.time, &[0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5]);
        close(&r.real, &[1.0, 1.0, 0.0, 4.5, 0.0, 4.5, 0.0, 1.0]);
        // A shifted FFT comes back unchanged with Shift on.
        let shifted = fft_spectrum(&X, &FftOptions { two_sided: true, ..Default::default() });
        close(&ifft_spectrum(&shifted.real, Some(&shifted.imaginary), &IfftOptions::default()).real, &X);
        let odd = fft_spectrum(&X[..7], &FftOptions { two_sided: true, ..Default::default() });
        close(&ifft_spectrum(&odd.real, Some(&odd.imaginary), &IfftOptions::default()).real, &X[..7]);
    }

    const LOW_02: [f64; 16] = [
        2.4815835878369148,
        1.5999835694299871,
        2.0371060052525296,
        3.3957971502100106,
        4.9264088062138107,
        5.8429331919632554,
        5.7155095874189294,
        4.7827932939138149,
        3.8970960686034424,
        3.9858028681969171,
        5.3415736511878276,
        7.3113096309765373,
        8.6949115373458312,
        8.5712803704098413,
        6.9058107561407125,
        4.5100999248996372,
    ];
    const HIGH_02_NO_OFFSET: [f64; 16] = [
        0.51841641216308543,
        -0.59998356942998765,
        1.9628939947474706,
        -2.3957971502100097,
        0.073591193786188613,
        3.1570668080367454,
        -3.71550958741893,
        1.2172067060861855,
        1.1029039313965574,
        -0.98580286819691787,
        -0.34157365118782779,
        0.68869036902346248,
        0.30508846265416856,
        -1.5712803704098397,
        2.0941892438592871,
        -1.5100999248996381,
    ];
    const BAND_NO_OFFSET: [f64; 16] = [
        -1.5408029288227159,
        -1.8131844165320048,
        -1.4703640057367258,
        0.028740719440290441,
        1.9131048661742769,
        2.4189855107139122,
        1.1790387201241934,
        -0.47482910172299597,
        -1.580517414736927,
        -2.1010291458410912,
        -1.6509563378229171,
        0.17836606174625713,
        2.2082154773853659,
        2.4952280516591836,
        0.9422816234354493,
        -0.73227767946355149,
    ];
    const BLOCK: [f64; 16] = [
        6.1213203435596419,
        1.6893398282201781,
        4.75,
        2.6464466094067265,
        2.5251262658470841,
        5.335786437626906,
        2.3357864376269051,
        6.5606601717798219,
        5.0,
        6.2248737341529159,
        7.3713203435596419,
        6.1464466094067252,
        7.353553390593274,
        5.75,
        6.542893218813453,
        3.646446609406726,
    ];
    const THRESHOLD_15: [f64; 16] = [
        2.9801427570428944,
        0.85607763534545622,
        3.9097038339569048,
        1.074812671152984,
        5.147555305605545,
        9.0381212704726366,
        1.8816214516556276,
        5.8712757111297229,
        5.0198572429571051,
        3.1439223646545429,
        5.0902961660430952,
        7.9251873288470165,
        8.852444694394455,
        6.9618787295273643,
        9.1183785483443724,
        3.1287242888702771,
    ];
    const LOW_AUTO: [f64; 16] = [
        2.4617263448798088,
        1.4560612047754438,
        1.9468098392094344,
        3.4706098213629941,
        5.0739641118193557,
        5.8810544624358911,
        5.597131039074557,
        4.6540690050435369,
        3.9169533115605484,
        4.12972523285146,
        5.4318698172309228,
        7.2364969598235538,
        8.5473562317402862,
        8.5331590999372047,
        7.0241893044850849,
        4.6388242137699152,
    ];
    const Y20: [f64; 20] = [
        -1.43, 3.16, 0.55, 4.06, 4.39, -4.02, -4.8, 7.56, -1.11, -1.49, 9.93, 2.05, 7.55, 2.15, 4.59, -2.74, 4.52,
        8.02, 2.85, 6.12,
    ];
    const PARABOLIC_005_04: [f64; 20] = [
        1.5465863735257743,
        1.1522023969398165,
        1.8306380882712177,
        3.9673429652217829,
        2.4926135044769464,
        -2.960441837378776,
        -1.8585904134936619,
        2.7489631324789405,
        0.96632778321659107,
        0.84227699267154432,
        5.5403619233094954,
        5.8872133012846,
        4.5609019179482448,
        4.6170135786637347,
        1.7677827721951744,
        0.33874824866822129,
        3.6804581759344872,
        6.2183876446138839,
        5.4279198746157338,
        3.143293576836252,
    ];
    const BLOCK_012_032: [f64; 20] = [
        1.4042205398774246,
        4.4835324359398872,
        1.56637694101251,
        1.1595457646276555,
        -0.30102376568214118,
        0.46644345223742756,
        -1.6430433056099392,
        2.1425744483452269,
        0.75476393202250025,
        2.8137003533473255,
        5.8046912095575518,
        1.0601532955150703,
        8.7698039589421519,
        -0.43322968366719722,
        6.9538953151271929,
        1.3126577808748214,
        2.8922353348400636,
        5.1045534421425574,
        0.83807983991268675,
        6.760068710637225,
    ];
    const THRESHOLD_AUTO_20: [f64; 20] = [
        -1.5384999999999998,
        3.2684999999999995,
        0.441500000000001,
        4.1685,
        4.2815000000000012,
        -3.9114999999999993,
        -4.9085,
        7.6685,
        -1.2184999999999995,
        -1.3814999999999997,
        9.8215,
        2.158500000000001,
        7.4415,
        2.2585000000000006,
        4.4814999999999987,
        -2.6315,
        4.4115000000000011,
        8.1284999999999989,
        2.7415000000000003,
        6.2285000000000013,
    ];

    #[test]
    fn fft_filters_match_origin() {
        let y = [3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0, 5.0, 3.0, 5.0, 8.0, 9.0, 7.0, 9.0, 3.0];
        let run = |o: FftFilterOptions| fft_filter(&y, 1.0, &o);
        let base = FftFilterOptions { keep_offset: true, ..Default::default() };
        close(&run(FftFilterOptions { cutoff: Some(0.2), ..base }), &LOW_02);
        close(&run(base), &LOW_AUTO);
        let high = FftFilterOptions { kind: FftFilterType::HighPass, cutoff: Some(0.2), keep_offset: false, ..base };
        close(&run(high), &HIGH_02_NO_OFFSET);
        let band = FftFilterOptions {
            kind: FftFilterType::BandPass,
            freq1: Some(0.1),
            freq2: Some(0.3),
            keep_offset: false,
            ..base
        };
        close(&run(band), &BAND_NO_OFFSET);
        close(&run(FftFilterOptions { kind: FftFilterType::BandBlock, keep_offset: true, ..band }), &BLOCK);
        close(&run(FftFilterOptions { kind: FftFilterType::Threshold, threshold: Some(1.5), ..base }), &THRESHOLD_15);
        // Twenty points: the parabolic edge, band block one step past freq2, the automatic threshold.
        let run20 = |o: FftFilterOptions| fft_filter(&Y20, 1.0, &o);
        let parabolic =
            FftFilterOptions { kind: FftFilterType::LowPassParabolic, pass: Some(0.05), stop: Some(0.4), ..base };
        close(&run20(parabolic), &PARABOLIC_005_04);
        let block = FftFilterOptions { kind: FftFilterType::BandBlock, freq1: Some(0.12), freq2: Some(0.32), ..base };
        close(&run20(block), &BLOCK_012_032);
        close(&run20(FftFilterOptions { kind: FftFilterType::Threshold, ..base }), &THRESHOLD_AUTO_20);
    }

    #[test]
    fn phase_unwrap_is_consistent_at_half_turn_roundoff_boundaries() {
        // The science IFFT on Linux produces -179.99999999999997 instead of -180.
        // A one-ULP difference must not change every subsequent phase by a full turn.
        for half in [-180.0_f64, (-180.0_f64).next_up(), (-180.0_f64).next_down(), 180.0_f64.next_up()] {
            let mut phase = [0.0, half, 0.0, 3.4215324074876726e-15, 0.0, 3.674471194299201e-15, 180.0, 0.0];
            unwrap_degrees(&mut phase);
            close(&phase, &[0.0, 180.0, 360.0, 360.0, 360.0, 360.0, 540.0, 720.0]);
        }
        let mut below = [0.0, -180.0 + 1e-8];
        unwrap_degrees(&mut below);
        assert!(below[1] < 0.0, "a real step below the tie threshold is not snapped");
        let mut above = [0.0, 180.0 + 1e-8];
        unwrap_degrees(&mut above);
        assert!(above[1] < 0.0, "a real step past the positive boundary wraps down");
    }

    #[test]
    fn windows_and_corrections_match_origin() {
        let with = |window, correction| fft_spectrum(&X, &FftOptions { window, correction, ..Default::default() });
        let r = with(FftWindow::Hanning, WindowCorrection::Amplitude);
        close(
            &r.real,
            &[15.033130587639945, -10.171791290415573, 1.9337388247201091, 10.171791290415573, -18.90060823708016],
        );
        close(&r.imaginary, &[0.0, 1.5370009425854507, 0.41341737989063376, -5.7728556612892046, 0.0]);
        close(&r.phase, &[0.0, 171.40736752886789, 12.067694022412185, -29.576461790731706, -180.0]);
        close(&with(FftWindow::Hanning, WindowCorrection::None).real[..2], &[6.5769946320924761, -4.4501586895568135]);
        close(&with(FftWindow::Hanning, WindowCorrection::Power).real[..2], &[11.481743141669336, -7.76883392360365]);
        let firsts = [
            (FftWindow::Welch, 13.600000000000001, -5.8777681494246981),
            (FftWindow::Triangular, 14.5, -7.70495128834866),
            (FftWindow::Bartlett, 15.333333333333337, -8.9566858950296027),
            (FftWindow::Hamming, 14.530228106787726, -9.140157081568363),
            (FftWindow::Blackman, 14.996654331450792, -10.978932376235395),
        ];
        for (window, a, b) in firsts {
            close(&with(window, WindowCorrection::Amplitude).real[..2], &[a, b]);
        }
        let r = with(FftWindow::Blackman, WindowCorrection::Amplitude);
        close(&r.phase, &[0.0, -178.9729496679906, -303.49999289330384, -389.37206401461259, -540.0]);
    }
}
