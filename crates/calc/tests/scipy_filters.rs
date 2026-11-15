//! IIR and FIR filters against numbers from scipy.signal (scipy-checked; none of these numbers come from Origin,
//! whose documentation gives no formulas: `docs/spec/analysis-tools.md` 17).
//!
//! The inputs and the scipy numbers are in `fixtures/scipy_filters.rs`, made by `tools/iir-fir-ref.py`.
//!
//! Tolerances: zeros and poles 1e-11 (absolute, relative to 1 + the root's size) and the gain 1e-11 relative,
//! elliptic included (the elliptic functions are written out here with the AGM and the Landen transformation
//! instead of scipy's Cephes routines; the two agree to about 1e-13 on every case here); window values and FIR
//! coefficients 1e-10 relative to the largest value (firls 1e-8: it solves a linear system, and scipy builds
//! the same system another way); numerator and denominator 1e-9 relative to the largest coefficient (orders up
//! to 8: the coefficients of higher orders lose digits in either program); magnitude responses 1e-9; filtered
//! signals 1e-9 relative to the signal's range. Sheaf pairs the second-order sections its own way, so the
//! sections are compared through what they compute (magnitude response and filtered signals), not row by row.
//! The natural frequencies of a minimum-order band stop filter are compared to 1e-4: scipy moves the pass
//! edges with `fminbound` (xtol 1e-5), Sheaf with a golden section search to 1e-13, and only the order is exact
//! there.

#[path = "fixtures/scipy_filters.rs"]
mod fixtures;

use fixtures::*;
use sheaf_calc::{
    FilterResponse, FirFilter, FirMethod, FirSpec, FirWindow, FreqUnit, IirMethod, fir_window, firls, firwin,
    iir_filter, iir_order, kaiser_beta, kaiserord,
};

const METHODS: [IirMethod; 4] =
    [IirMethod::Butterworth, IirMethod::Chebyshev1, IirMethod::Chebyshev2, IirMethod::Elliptic];
const RESP: [FilterResponse; 4] =
    [FilterResponse::LowPass, FilterResponse::HighPass, FilterResponse::BandPass, FilterResponse::BandStop];

fn scale(want: &[f64]) -> f64 {
    want.iter().fold(0.0f64, |a, b| a.max(b.abs())).max(1e-300)
}

fn near_vec(what: &str, got: &[f64], want: &[f64], rel: f64) {
    assert_eq!(got.len(), want.len(), "{what}: length");
    let s = scale(want);
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        assert!((g - w).abs() <= rel * s, "{what}[{i}]: got {g:?}, scipy gave {w:?} (scale {s:e})");
    }
}

fn same_roots(what: &str, got: &[rustfft::num_complex::Complex<f64>], want: &[(f64, f64)], tol: f64) {
    assert_eq!(got.len(), want.len(), "{what}: count");
    let mut left: Vec<(f64, f64)> = got.iter().map(|c| (c.re, c.im)).collect();
    for w in want {
        let (i, d) = left
            .iter()
            .enumerate()
            .map(|(i, g)| (i, ((g.0 - w.0).powi(2) + (g.1 - w.1).powi(2)).sqrt()))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        assert!(d <= tol * (1.0 + (w.0 * w.0 + w.1 * w.1).sqrt()), "{what}: no root near {w:?} (closest off by {d:e})");
        left.remove(i);
    }
}

#[test]
fn iir_designs_match_scipy() {
    for c in IIR {
        let f = iir_filter(c.order, c.wn, RESP[c.response], METHODS[c.method], c.rp, c.rs).unwrap();
        let tol = 1e-12;
        // the poles of high orders are sensitive to the last digits of the transform: compare to a scale
        same_roots(&format!("{} zeros", c.name), &f.zeros, c.zeros, tol * 10.0);
        same_roots(&format!("{} poles", c.name), &f.poles, c.poles, tol * 10.0);
        assert!(
            (f.gain - c.gain).abs() <= tol * c.gain.abs().max(1e-12) * 10.0,
            "{}: gain {} vs {}",
            c.name,
            f.gain,
            c.gain
        );
        let (b, a) = f.coefficients();
        let order = c.order.min(8);
        if c.order <= order {
            near_vec(&format!("{} b", c.name), &b, c.b, 1e-9);
            near_vec(&format!("{} a", c.name), &a, c.a, 1e-9);
        }
        let got: Vec<f64> = MAG_W.iter().map(|w| f.magnitude(*w)).collect();
        near_vec(&format!("{} magnitude", c.name), &got, c.magnitude, 1e-9);
        let range = scale(SIGNAL);
        let one = f.apply(SIGNAL, false).unwrap();
        let zero = f.apply(SIGNAL, true).unwrap();
        for (i, (g, w)) in one.iter().zip(c.one_way).enumerate() {
            assert!((g - w).abs() <= 1e-9 * range.max(w.abs()), "{} one way [{i}]: {g} vs {w}", c.name);
        }
        for (i, (g, w)) in zero.iter().zip(c.zero_phase).enumerate() {
            assert!((g - w).abs() <= 1e-9 * range.max(w.abs()), "{} zero phase [{i}]: {g} vs {w}", c.name);
        }
    }
}

#[test]
fn minimum_orders_match_scipy() {
    for c in ORD {
        let (n, wn) = iir_order(METHODS[c.method], RESP[c.response], c.wp, c.ws, c.gpass, c.gstop).unwrap();
        assert_eq!(n, c.order, "{} order", c.name);
        let tol = if c.response == 3 { 1e-4 } else { 1e-9 };
        assert_eq!(wn.len(), c.wn.len(), "{}", c.name);
        for (g, w) in wn.iter().zip(c.wn) {
            assert!((g - w).abs() <= tol * w.abs().max(1e-3), "{} wn: {g} vs {w}", c.name);
        }
    }
}

#[test]
fn windows_match_scipy() {
    for c in WINDOWS {
        let got = fir_window(FirWindow::ALL[c.window], c.m, None).unwrap();
        near_vec(&format!("{} {}", c.name, c.m), &got, c.values, 1e-10);
    }
}

#[test]
fn firwin_matches_scipy() {
    for c in FIRWIN {
        let got = firwin(c.numtaps, c.cutoff, RESP[c.response], FirWindow::ALL[c.window], None, c.scale).unwrap();
        near_vec(c.name, &got, c.h, 1e-10);
    }
}

#[test]
fn firls_matches_scipy() {
    for c in FIRLS {
        let bands: Vec<(f64, f64)> = c.bands.chunks(2).map(|b| (b[0], b[1])).collect();
        let got = firls(c.numtaps, &bands, c.desired, c.weight).unwrap();
        near_vec(c.name, &got, c.h, 1e-8);
    }
}

#[test]
fn kaiser_order_matches_scipy() {
    for &(a, w, n, beta) in KAISER {
        match kaiserord(a, w) {
            Ok((got_n, got_beta)) => {
                assert_eq!(got_n, n, "kaiserord({a}, {w})");
                assert!((got_beta - beta).abs() < 1e-12, "beta {got_beta} vs {beta}");
            }
            Err(_) => assert!(a < 8.0, "kaiserord({a}, {w}) refused"),
        }
        assert!((kaiser_beta(a) - beta).abs() < 1e-12);
    }
}

#[test]
fn fir_application_matches_scipy() {
    for c in APPLY {
        let f = FirFilter { coefficients: c.h.to_vec(), edges: vec![] };
        let one = f.apply(SIGNAL, false).unwrap();
        let zero = f.apply(SIGNAL, true).unwrap();
        near_vec(&format!("{} one way", c.name), &one, c.one_way, 1e-10);
        near_vec(&format!("{} zero phase", c.name), &zero, c.zero_phase, 1e-10);
    }
}

#[test]
fn the_spec_designs_what_firwin_does() {
    // a Hamming low pass of order 40 at Fs = 1000 Hz, 150 Hz
    let spec = FirSpec {
        order: Some(40),
        cutoff: Some(150.0),
        sample_frequency: Some(1000.0),
        method: FirMethod::Window,
        unit: FreqUnit::Hz,
        ..FirSpec::default()
    };
    let f = spec.design(None).unwrap();
    let want = firwin(41, &[0.3], FilterResponse::LowPass, FirWindow::Hamming, None, true).unwrap();
    near_vec("spec", &f.coefficients, &want, 1e-15);
}
