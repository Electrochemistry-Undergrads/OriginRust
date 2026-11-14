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
    for i in 1..phase.len() {
        let prev = phase[i - 1];
        let mut p = phase[i];
        while p - prev > 180.0 {
            p -= 360.0;
        }
        while p - prev <= -180.0 {
            p += 360.0;
        }
        phase[i] = p;
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
