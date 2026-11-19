//! Short-time Fourier transform, as Origin's Analysis > Signal Processing > STFT (`stft`,
//! documented, not measured; `docs/spec/analysis-tools.md` 20.2): the amplitude spectrum of
//! overlapping windowed segments of a signal.

use crate::fft::{FftWindow, fft_window};
use rustfft::{FftPlanner, num_complex::Complex};

/// What each spectrum value is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StftOutput {
    /// One-sided amplitude: `2|X| / sum(w)`, `|X| / sum(w)` at zero and at the Nyquist frequency,
    /// so a sine of amplitude A inside a segment shows a peak of about A.
    #[default]
    Amplitude,
    /// `20 log10(amplitude)`; missing where the amplitude is 0.
    Db,
}

/// Options of [`stft`] (Sheaf's defaults; Origin's are not measured).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StftOptions {
    /// Sampling interval: the frequency step is `1 / (fft_len * interval)`.
    pub interval: f64,
    pub window: FftWindow,
    /// Points in each segment.
    pub window_len: usize,
    /// Points of each transform; a segment shorter than this is padded with zeros.
    pub fft_len: usize,
    /// Points shared by neighbouring segments (less than `window_len`).
    pub overlap: usize,
    pub output: StftOutput,
}

impl Default for StftOptions {
    fn default() -> Self {
        StftOptions {
            interval: 1.0,
            window: FftWindow::Hanning,
            window_len: 64,
            fft_len: 64,
            overlap: 32,
            output: StftOutput::Amplitude,
        }
    }
}

/// The result: one spectrum per segment.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StftResult {
    /// 0 to the Nyquist frequency, `fft_len / 2 + 1` values.
    pub frequency: Vec<f64>,
    /// The time of each segment's centre, from the first sample (`start + (window_len - 1) / 2`
    /// samples, times the interval).
    pub time: Vec<f64>,
    /// `spectra[s][k]`: segment `s`, frequency `k`.
    pub spectra: Vec<Vec<f64>>,
}

/// The short-time Fourier transform of `y`.
///
/// Segments of `window_len` points start every `window_len - overlap` points while they fit in
/// the signal (a remainder at the end is left out). Each segment is multiplied by the window
/// (symmetric over its points, as FFT's windows), padded with zeros to `fft_len` and transformed.
pub fn stft(y: &[f64], o: &StftOptions) -> Result<StftResult, String> {
    let (n, w, nfft) = (y.len(), o.window_len, o.fft_len);
    if w < 2 {
        return Err("the window needs at least 2 points".into());
    }
    if o.overlap >= w {
        return Err(format!("the overlap ({}) must be less than the window length ({w})", o.overlap));
    }
    if nfft < w {
        return Err(format!("the FFT length ({nfft}) must be at least the window length ({w})"));
    }
    if n < w {
        return Err(format!("{n} points are fewer than the window length ({w})"));
    }
    if !(o.interval.is_finite() && o.interval > 0.0) {
        return Err(format!("{} is not a usable interval", o.interval));
    }
    let win = fft_window(o.window, w);
    let sum: f64 = win.iter().sum();
    let sum = if sum > 0.0 { sum } else { 1.0 };
    let step = w - o.overlap;
    let fft = FftPlanner::<f64>::new().plan_fft_forward(nfft);
    let half = nfft / 2;
    let mut r = StftResult {
        frequency: (0..=half).map(|k| k as f64 / (nfft as f64 * o.interval)).collect(),
        ..Default::default()
    };
    let mut start = 0;
    while start + w <= n {
        let mut buf = vec![Complex::new(0.0, 0.0); nfft];
        for (i, b) in buf.iter_mut().take(w).enumerate() {
            b.re = y[start + i] * win[i];
        }
        fft.process(&mut buf);
        let spectrum = (0..=half)
            .map(|k| {
                let edge = k == 0 || (nfft.is_multiple_of(2) && k == half);
                let amp = buf[k].norm() / sum * if edge { 1.0 } else { 2.0 };
                match o.output {
                    StftOutput::Amplitude => amp,
                    StftOutput::Db => {
                        let db = 20.0 * amp.log10();
                        if db.is_finite() { db } else { f64::NAN }
                    }
                }
            })
            .collect();
        r.spectra.push(spectrum);
        r.time.push((start as f64 + (w - 1) as f64 / 2.0) * o.interval);
        start += step;
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sine_peaks_at_its_frequency_with_its_amplitude() {
        // 3 s at 64 samples per second: a 8 Hz sine of amplitude 2 in every segment.
        let y: Vec<f64> = (0..192).map(|i| 2.0 * (2.0 * std::f64::consts::PI * 8.0 * i as f64 / 64.0).sin()).collect();
        let o = StftOptions { interval: 1.0 / 64.0, window: FftWindow::Rectangle, ..Default::default() };
        let r = stft(&y, &o).unwrap();
        assert_eq!(r.frequency.len(), 33);
        assert_eq!(r.frequency[8], 8.0);
        // Segments start at 0, 32, ..., 128: five of them.
        assert_eq!(r.spectra.len(), 5);
        assert_eq!(r.time[0], 31.5 / 64.0);
        for s in &r.spectra {
            let peak = (0..s.len()).max_by(|a, b| s[*a].total_cmp(&s[*b])).unwrap();
            assert_eq!(peak, 8);
            assert!((s[8] - 2.0).abs() < 1e-12, "{}", s[8]);
        }
    }

    #[test]
    fn segments_match_numpy() {
        // numpy: w = np.hanning(8); X = np.fft.rfft(y[s:s+8] * w, 16); a = 2*abs(X)/w.sum(),
        // a[0] and a[-1] halved, for s = 0, 5 and y = cos(0.7 i) + 0.1 i.
        let y: Vec<f64> = (0..20).map(|i| (0.7 * i as f64).cos() + 0.1 * i as f64).collect();
        let o = StftOptions { window_len: 8, fft_len: 16, overlap: 3, ..Default::default() };
        let r = stft(&y, &o).unwrap();
        assert_eq!(r.spectra.len(), 3, "starts 0, 5, 10");
        let want0 =
            [0.16207127047972847, 0.44315379375231495, 0.5865386107335955, 0.5873479006437073, 0.4386056273810831];
        let want1 = [1.4782659883539615, 2.7168328760212765, 2.092174289767207, 1.3143326374642754, 0.6313002000461246];
        for (got, want) in [(&r.spectra[0], want0), (&r.spectra[1], want1)] {
            for (g, w) in got.iter().zip(want) {
                assert!((g - w).abs() < 1e-12, "{got:?}");
            }
        }
    }

    #[test]
    fn bad_options_are_refused() {
        let y = vec![1.0; 10];
        for o in [
            StftOptions { window_len: 8, overlap: 8, fft_len: 8, ..Default::default() },
            StftOptions { window_len: 8, overlap: 2, fft_len: 4, ..Default::default() },
            StftOptions { window_len: 16, overlap: 2, fft_len: 16, ..Default::default() },
            StftOptions { window_len: 1, overlap: 0, fft_len: 4, ..Default::default() },
        ] {
            assert!(stft(&y, &o).is_err(), "{o:?}");
        }
    }
}
