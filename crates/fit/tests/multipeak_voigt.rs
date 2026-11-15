//! Voigt peaks in the multi-peak fit, checked against Origin 2016 (analysis-tools.md 11.2).
//!
//! Origin's numbers come from `re-workspace/scratch/t04v/v1.lines.txt` (local only): the data are
//! made by Origin's own `nlf_Voigt`, then fitted with `nlbegin func:=Voigt; nlfit`.

use sheaf_fit::library::voigt;
use sheaf_fit::multipeak::{FitBaseline, PeakShape, PeakStart, multi_peak_function, peak_summaries, voigt_fwhm};
use sheaf_fit::nlsf::{FitOptions, fit};

fn close(got: f64, want: f64, rel: f64) -> bool {
    (got - want).abs() <= rel * want.abs().max(1e-300)
}

#[test]
fn the_voigt_function_is_exact_and_within_origins_approximation() {
    // Sheaf evaluates the Faddeeva function: scipy's `wofz` gives the same digits.
    for (x, y0, xc, a, wg, wl, exact) in [
        (0.0, 0.2, 5.0, 4.0, 1.2, 0.8, 0.220_897_721_635_662_65),
        (2.0, 0.2, 5.0, 4.0, 1.2, 0.8, 0.260_994_918_233_220_74),
        (4.5, 0.2, 5.0, 4.0, 1.2, 0.8, 1.623_306_341_901_244_8),
        (5.0, 0.2, 5.0, 4.0, 1.2, 0.8, 2.042_945_314_137_422_3),
        (5.0, 0.0, 5.0, 1.0, 0.3, 2.5, 0.252_081_210_340_471_4),
        (6.5, 0.0, 5.0, 1.0, 2.5, 0.05, 0.138_992_389_442_277_15),
    ] {
        let got = voigt(x, y0, xc, a, wg, wl);
        assert!(close(got, exact, 1e-12), "x = {x}: {got} against {exact}");
    }
    // Origin's nlf_Voigt is a numerical approximation, within 2e-6 of the peak height of the
    // exact values above (the same six points, in order).
    let origin = [
        0.220_897_922_727_964_35,
        0.260_995_282_248_682_46,
        1.623_304_004_348_845_4,
        2.042_942_100_309_095_3,
        0.252_081_001_048_125_7,
        0.138_992_224_606_982_18,
    ];
    let cases = [
        (0.0, 0.2, 5.0, 4.0, 1.2, 0.8),
        (2.0, 0.2, 5.0, 4.0, 1.2, 0.8),
        (4.5, 0.2, 5.0, 4.0, 1.2, 0.8),
        (5.0, 0.2, 5.0, 4.0, 1.2, 0.8),
        (5.0, 0.0, 5.0, 1.0, 0.3, 2.5),
        (6.5, 0.0, 5.0, 1.0, 2.5, 0.05),
    ];
    for ((x, y0, xc, a, wg, wl), want) in cases.into_iter().zip(origin) {
        let height = voigt(xc, 0.0, xc, a, wg, wl);
        let got = voigt(x, y0, xc, a, wg, wl);
        assert!((got - want).abs() <= 2e-6 * height, "x = {x}: {got} against Origin's {want}");
    }
}

/// Origin's probe data as Origin computed them (`tests/data/origin_voigt_probe.tsv`):
/// `nlf_Voigt` plus a fixed ripple, x = 0 to 10 in steps of 0.05.
fn probe_data() -> (Vec<f64>, Vec<f64>) {
    let text = include_str!("data/origin_voigt_probe.tsv");
    let rows: Vec<(f64, f64)> = text
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| {
            let mut f = l.split('\t').map(|v| v.parse::<f64>().unwrap());
            (f.next().unwrap(), f.next().unwrap())
        })
        .collect();
    assert_eq!(rows.len(), 201);
    rows.into_iter().unzip()
}

#[test]
fn a_single_voigt_peak_fits_to_origins_parameters() {
    let (x, y) = probe_data();
    // The Peak Analyzer starts a peak at the found centre with its height and FWHM.
    let start = PeakStart { center: 5.0, fwhm: 1.6, height: 1.8 };
    let func = multi_peak_function(PeakShape::Voigt, FitBaseline::Constant, &[start]).with_initial("y0", 0.22);
    let r = fit(&func, &x, &y, &FitOptions::default()).unwrap();
    let v = |n: &str| r.param(n).unwrap().value;
    // Origin: y0 0.199772863732606, xc 4.99999967995525, A 4.00225088185448, wG 1.19892425718239,
    // wL 0.801615491802901, reduced chi-square 7.8673486183435684e-05. Origin fits with its
    // approximate Voigt (within 2e-6 of the height, above) and Sheaf with the exact one, so the
    // parameters agree to about 6e-6 relative (measured 2026-11-15: y0 3.7e-6, A 3.0e-6, wG 3.4e-6,
    // wL 5.5e-6, xc 2.3e-9 absolute) and the reduced chi-square to 2e-7.
    for (name, want, rel) in [
        ("y0", 0.199_772_863_732_606, 1e-5),
        ("xc1", 4.999_999_679_955_25, 1e-8),
        ("A1", 4.002_250_881_854_48, 1e-5),
        ("wG1", 1.198_924_257_182_39, 1e-5),
        ("wL1", 0.801_615_491_802_901, 1e-5),
    ] {
        assert!(close(v(name), want, rel), "{name} = {} against Origin's {want}", v(name));
    }
    assert!(close(r.stats.reduced_chi_sqr, 7.867_348_618_343_568e-5, 1e-6), "{}", r.stats.reduced_chi_sqr);
    // Origin's standard errors of xc and A: 3.4e-4 and 5.3e-3 relative apart, which the
    // derivatives of Origin's approximate Voigt would explain (not measured further).
    assert!(close(r.param("xc1").unwrap().std_error, 0.001_029_423_895_714_3, 1e-3));
    assert!(close(r.param("A1").unwrap().std_error, 0.017_788_521_852_850_4, 1e-2));

    let params: Vec<f64> = r.params.iter().map(|p| p.value).collect();
    let peak = peak_summaries(PeakShape::Voigt, FitBaseline::Constant, &params)[0];
    assert_eq!(peak.area, v("A1"));
    assert!(close(peak.fwhm, voigt_fwhm(v("wG1"), v("wL1")), 1e-15));
    assert!(close(peak.height, voigt(v("xc1"), 0.0, v("xc1"), v("A1"), v("wG1"), v("wL1")), 1e-15));
}

#[test]
fn two_overlapping_voigt_peaks_on_a_slope_are_recovered() {
    let x: Vec<f64> = (0..500).map(|i| i as f64 * 0.04).collect();
    let truth = |x: f64| 0.1 + 0.01 * x + voigt(x, 0.0, 8.0, 2.0, 1.0, 0.6) + voigt(x, 0.0, 10.0, 1.2, 0.7, 1.1);
    let y: Vec<f64> = x.iter().map(|&x| truth(x)).collect();
    let starts =
        [PeakStart { center: 7.8, fwhm: 1.2, height: 1.0 }, PeakStart { center: 10.2, fwhm: 1.5, height: 0.5 }];
    let func = multi_peak_function(PeakShape::Voigt, FitBaseline::Linear, &starts)
        .with_initial("y0", 0.0)
        .with_initial("m", 0.0);
    let r = fit(&func, &x, &y, &FitOptions::default()).unwrap();
    let names = ["y0", "m", "xc1", "A1", "wG1", "wL1", "xc2", "A2", "wG2", "wL2"];
    let got: Vec<String> = r.params.iter().map(|p| p.name.clone()).collect();
    assert_eq!(got, names);
    for (name, want) in names.iter().zip([0.1, 0.01, 8.0, 2.0, 1.0, 0.6, 10.0, 1.2, 0.7, 1.1]) {
        let v = r.param(name).unwrap().value;
        assert!((v - want).abs() < 1e-6, "{name} = {v}, want {want}");
    }
}

#[test]
fn a_voigt_start_has_the_found_height_and_fwhm() {
    let start = PeakStart { center: 3.0, fwhm: 2.0, height: 5.0 };
    let func = multi_peak_function(PeakShape::Voigt, FitBaseline::Constant, &[start]).with_initial("y0", 0.0);
    let p: Vec<f64> = func.params.iter().map(|p| p.initial).collect();
    let peak = peak_summaries(PeakShape::Voigt, FitBaseline::Constant, &p)[0];
    assert!(close(peak.height, 5.0, 1e-12) && close(peak.fwhm, 2.0, 1e-12), "{peak:?}");
    // Equal Gaussian and Lorentzian widths, as Origin's Voigt initialisation.
    assert_eq!(p[3], p[4]);
}
