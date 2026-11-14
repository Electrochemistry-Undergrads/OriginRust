//! Peak chain against Origin `integ1` (reference script `tools/origin-ref/peaks-integ.lines.txt`).
//!
//! The data are generated: a sloped baseline `0.05 + 0.01 x`, one separate peak and two
//! overlapping peaks, and a small deterministic ripple. Sheaf finds the peaks and their limits
//! (the limit rules are Sheaf's own, see `docs/spec/analysis-tools.md`); Origin then integrated the
//! baseline-corrected curve over exactly the rows Sheaf chose. Compared: peak position and height
//! (exact), width at half height (where the half height is reached inside the limits) and area.
//! Sheaf's area also includes the sliver from the last data point to the interpolated baseline
//! crossing, which `integ1` over whole rows cannot contain, hence the absolute tolerance of 2e-4
//! (the sliver is below 1.1e-4 here). For the second peak the half height is not reached on the
//! valley side; Origin then extends the width beyond the rows (2.47), Sheaf stops at its limit.

use sheaf_calc::*;

fn data() -> (Vec<f64>, Vec<f64>) {
    let x: Vec<f64> = (0..401).map(|i| i as f64 * 0.05).collect();
    let g = |x: f64, c: f64, s: f64, h: f64| h * (-(x - c).powi(2) / (2.0 * s * s)).exp();
    let y = x
        .iter()
        .map(|&x| {
            0.05 + 0.01 * x
                + g(x, 4.0, 0.4, 1.0)
                + g(x, 9.0, 0.5, 0.7)
                + g(x, 10.2, 0.5, 0.55)
                + 0.004 * (37.0 * x).sin()
                + 0.003 * (91.0 * x).cos()
        })
        .collect();
    (x, y)
}

#[test]
fn integration_figures_match_integ1_over_the_same_rows() {
    let (x, y) = data();
    let o = ChainOptions {
        baseline: ChainBaseline::Anchors(vec![(0.0, 0.05), (20.0, 0.25)], InterpMethod::Linear),
        find: FindOptions { filter: PeakFilter::Height(0.2), ..Default::default() },
        ..Default::default()
    };
    let r = analyze_peaks(&x, &y, &o).unwrap();
    assert_eq!(r.peaks.len(), 3);
    // rows (1-based) given to integ1, then Origin's area, x0, y0, dx
    let origin: [((usize, usize), f64, f64, f64, Option<f64>); 3] = [
        ((55, 108), 1.00185707337143, 4.0, 1.00138009447625, Some(0.939286498296096)),
        ((144, 197), 0.975270740928194, 9.05, 0.742114131064976, None),
        ((197, 237), 0.590948974587407, 10.05, 0.603790298626747, Some(0.953319203542334)),
    ];
    for (p, (rows, area, x0, y0, dx)) in r.peaks.iter().zip(origin) {
        assert_eq!((p.left_index + 1, p.right_index + 1), rows);
        assert!((p.area - area).abs() < 2e-4, "area {} vs {}", p.area, area);
        assert_eq!(p.center, x0);
        assert!((p.height - y0).abs() < 1e-12);
        if let Some(dx) = dx {
            assert!((p.fwhm - dx).abs() < 1e-9, "fwhm {} vs {}", p.fwhm, dx);
        }
    }
    // Area % is the share of the whole corrected curve's area (Origin's AreaIntgP), the same
    // whole for every peak, so the peaks together hold less than 100 %.
    let whole: Vec<f64> = r.peaks.iter().map(|p| 100.0 * p.area / p.area_percent).collect();
    assert!(whole.windows(2).all(|w| (w[0] - w[1]).abs() < 1e-9), "{whole:?}");
    let total: f64 = r.peaks.iter().map(|p| p.area_percent).sum();
    assert!(total < 100.0, "{total}");
}
