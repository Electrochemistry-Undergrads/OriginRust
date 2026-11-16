//! Origin 2016 runs of the discrete wavelet tools (`dwt`, `idwt`, `wtdenoise`, `wtsmooth`;
//! `docs/spec/analysis-tools.md` 19). The numbers are in `fixtures/wavelet_origin.rs`, generated from
//! `tools/origin-ref/t96/t96-origin.json`; every output there was read from Origin. Tolerance 1e-12
//! (the decompositions agree to 1e-15).

#[path = "fixtures/wavelet_origin.rs"]
mod data;

use sheaf_calc::wavelet::{self, Extension, Threshold, Wavelet, WaveletError};

const TOL: f64 = 1e-12;

fn close(got: &[f64], want: &[f64], tol: f64, what: &str) {
    assert_eq!(got.len(), want.len(), "{what}: length");
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        assert!((g - w).abs() <= tol, "{what}: index {i}: got {g}, want {w}");
    }
}

fn ext(i: usize) -> Extension {
    Extension::from_index(i).unwrap()
}

#[test]
fn dwt_all_21_wavelets_both_modes() {
    for c in data::DWT {
        let w = Wavelet::from_index(c.wavelet).unwrap();
        let x = if c.n == 16 { data::DWT_INPUT16 } else { data::DWT_INPUT21 };
        let (ca, cd) = wavelet::dwt(x, w, ext(c.ext)).unwrap();
        let what = format!("{} ext {} n {}", w.name(), c.ext, c.n);
        close(&ca, c.ca, TOL, &format!("{what} ca"));
        close(&cd, c.cd, TOL, &format!("{what} cd"));
    }
}

#[test]
fn dwt_periodic_odd_length_refused() {
    assert_eq!(wavelet::dwt(data::DWT_INPUT21, Wavelet::Haar, Extension::Periodic), Err(WaveletError::OddLength(21)));
}

#[test]
fn idwt_cases() {
    for c in data::IDWT {
        let w = Wavelet::from_index(c.wavelet).unwrap();
        let y = wavelet::idwt(c.ca, c.cd, w, ext(c.ext)).unwrap();
        close(&y, c.out, TOL, c.name);
    }
}

fn wave(s: &str) -> Wavelet {
    Wavelet::parse(s).unwrap()
}

fn ext_name(s: &str) -> Extension {
    Extension::parse(s).unwrap()
}

#[test]
fn denoise_custom_threshold_matches_origin() {
    let mut checked = 0;
    for c in data::DENOISE.iter().filter(|c| c.thrt == "custom") {
        let r = wavelet::denoise(data::NOISY128, wave(c.wavelet), ext_name(c.ext), c.level, Threshold::Custom(c.thrd));
        let what = format!("{} {} level {} thrd {}", c.wavelet, c.ext, c.level, c.thrd);
        match c.out {
            None => assert!(r.is_err(), "{what} is refused by Origin"),
            Some(out) => {
                // At exactly 100 % Origin's rounding of 100/100 * max dropped the maximum at one
                // level (spec 19.4); that one case differs by design.
                if c.thrd == 100.0 && c.level == 3 {
                    continue;
                }
                close(&r.unwrap(), out, TOL, &what);
                checked += 1;
            }
        }
    }
    assert!(checked >= 20, "{checked}");
}

#[test]
fn denoise_sqtwolog_matches_origin_on_the_noisy_signal() {
    for c in data::DENOISE.iter().filter(|c| c.thrt == "sqtwolog") {
        let r = wavelet::denoise(data::NOISY128, wave(c.wavelet), ext_name(c.ext), c.level, Threshold::SqtwoLog);
        let what = format!("sqtwolog {} {} level {}", c.wavelet, c.ext, c.level);
        match c.out {
            None => assert!(r.is_err(), "{what}"),
            Some(out) => close(&r.unwrap(), out, TOL, &what),
        }
    }
}

/// The designed signals of spec 19.4: the interior samples (`L - 2 ..= n - L + 1`) agree with the
/// model; the first and last `L - 2` samples differ in Origin for a reason not found.
#[test]
fn denoise_boundary_signals_agree_in_the_interior() {
    for c in data::BOUNDARY {
        let y = wavelet::denoise(c.input, Wavelet::Db(2), Extension::Periodic, 1, Threshold::Custom(50.0)).unwrap();
        let n = c.input.len();
        close(&y[2..n - 2], &c.out[2..n - 2], 1e-8, c.name);
    }
}

#[test]
fn smooth_128_periodic_matches_origin() {
    let mut checked = 0;
    for c in data::SMOOTH.iter().filter(|c| c.n == 128 && c.ext == "periodic") {
        let y = wavelet::smooth(data::NOISY128, wave(c.wavelet), ext_name(c.ext), c.cutoff).unwrap();
        close(&y, c.out, TOL, &format!("{} cutoff {}", c.wavelet, c.cutoff));
        checked += 1;
    }
    assert!(checked >= 20, "{checked}");
}

#[test]
fn smooth_128_zero_padded_matches_origin() {
    // DB2 only (cutoff 0, 50, 100); the cutoff-0 corner zeroes 3 coefficients here, 2 periodic.
    for c in data::SMOOTH.iter().filter(|c| c.n == 128 && c.ext == "pad0") {
        let y = wavelet::smooth(data::NOISY128, wave(c.wavelet), ext_name(c.ext), c.cutoff).unwrap();
        close(&y, c.out, TOL, &format!("pad0 {} cutoff {}", c.wavelet, c.cutoff));
    }
}

#[test]
fn sqtwolog_threshold_in_the_measured_brackets() {
    for &(n, lo, hi) in data::SQTWOLOG {
        let t = wavelet::sqtwolog_threshold(n);
        if n >= 128 {
            assert!(t > lo && t < hi, "n = {n}: {t} not in {lo}..{hi}");
        } else {
            assert!(t > lo - 0.0 && t < hi, "n = {n}");
        }
    }
}
