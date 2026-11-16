//! Origin 2016 runs of `cwt` (Continuous Wavelet; `docs/spec/analysis-tools.md` 19.6). The numbers are in
//! `fixtures/cwt_origin.rs`, generated from `tools/origin-ref/t96/t96-origin.json`; every column there was
//! read from Origin (impulses and a generic signal, Morlet wave numbers 5 to 20, DGauss orders 1 to 8, Mexican
//! hat, integer and fractional scales, signals of 1 to 65 samples). Tolerance 1e-12 (the agreement is 1e-14).

#![allow(clippy::approx_constant)] // Origin's outputs contain sqrt 2 as printed

#[path = "fixtures/cwt_origin.rs"]
mod data;

use sheaf_calc::cwt::{CwtError, CwtWavelet, cwt};

const TOL: f64 = 1e-12;

#[test]
fn every_origin_run_is_reproduced() {
    let mut columns = 0;
    for c in data::CWT {
        let w = CwtWavelet::from_index(c.wavelet).unwrap();
        let got = cwt(c.signal, c.scales, w, c.param).unwrap();
        assert_eq!(got.len(), c.cols.len(), "{}: columns", c.name);
        for (k, (g, want)) in got.iter().zip(c.cols).enumerate() {
            assert_eq!(g.len(), want.len(), "{} scale {}: length", c.name, c.scales[k]);
            for (i, (a, b)) in g.iter().zip(*want).enumerate() {
                assert!((a - b).abs() <= TOL, "{} scale {} row {i}: got {a}, want {b}", c.name, c.scales[k]);
                columns += 1;
            }
        }
    }
    assert!(columns > 5_000, "{columns} values compared");
}

#[test]
fn every_run_origin_refused_is_refused() {
    for (w, param, scales) in data::CWT_REFUSED {
        let w = CwtWavelet::from_index(*w).unwrap();
        let x = [0.0, 1.0, 0.0, 0.0, 2.0, 0.0, 1.0, 0.0];
        assert!(cwt(&x, scales, w, *param).is_err(), "{} {param} {scales:?}", w.name());
    }
}

#[test]
fn a_fractional_scale_is_cut_to_its_integer_part_as_in_origin() {
    // Origin gives the same column for scale 1.5 as for 1 (case `CM`).
    let c = data::CWT.iter().find(|c| c.name == "CM").unwrap();
    assert_eq!(c.scales[0], 1.5);
    let one = cwt(c.signal, &[1.0], CwtWavelet::Morlet, 5.0).unwrap();
    for (a, b) in one[0].iter().zip(c.cols[0]) {
        assert!((a - b).abs() <= TOL);
    }
    assert_eq!(cwt(c.signal, &[0.99999], CwtWavelet::Morlet, 5.0), Err(CwtError::ScaleBelowOne(0.99999)));
}
