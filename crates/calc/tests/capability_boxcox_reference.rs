//! Box-Cox process capability and Cpm against numpy and scipy, made by `tools/capability-boxcox-ref.py` (T419; none of
//! these numbers come from Origin). The fitted lambda to 1e-6; at the reference's lambda, values from the within
//! deviation and the parts per million to 1e-6 (Sheaf's six-decimal d2 table against the reference's integrated d2),
//! the others to 1e-9.

#[path = "fixtures/capability_boxcox_reference.rs"]
mod fixtures;

use sheaf_calc::capability::{CapModel, CapOptions, capability_with};

fn close(a: f64, b: f64, rel: f64, what: &str) {
    if b.is_nan() {
        assert!(a.is_nan(), "{what}: {a} vs NaN");
    } else {
        assert!((a - b).abs() <= rel * b.abs().max(1.0), "{what}: {a} vs {b}");
    }
}

#[test]
fn box_cox_capability_matches_scipy() {
    for (name, x, lsl, usl, n, lambda, target, r) in fixtures::CASES {
        // The fitted lambda on its own (the likelihood is flat at its top: both find it to about 1e-7), then every
        // index at the reference's lambda, so its last digits do not blur them.
        let fitted = capability_with(
            x,
            *lsl,
            *usl,
            *n,
            CapModel::BoxCox,
            CapOptions { lambda: *lambda, target: None, ..Default::default() },
        );
        close(fitted.unwrap().lambda, r[0], 1e-6, &format!("{name} lambda"));
        let opts = CapOptions { lambda: Some(r[0]), target: *target, ..Default::default() };
        let c = capability_with(x, *lsl, *usl, *n, CapModel::BoxCox, opts).unwrap();
        let within = [
            (c.sd_within, r[2], "sd within"),
            (c.cp, r[4], "Cp"),
            (c.cpl, r[5], "CPL"),
            (c.cpu, r[6], "CPU"),
            (c.cpk, r[7], "Cpk"),
        ];
        let tails = [(c.expected_overall.below, r[12], "ppm below"), (c.expected_overall.above, r[13], "ppm above")];
        for (got, want, what) in within.into_iter().chain(tails) {
            close(got, want, 1e-6, &format!("{name} {what}"));
        }
        let overall = [
            (c.mean, r[1], "mean"),
            (c.sd_overall, r[3], "sd overall"),
            (c.pp, r[8], "Pp"),
            (c.ppl, r[9], "PPL"),
            (c.ppu, r[10], "PPU"),
            (c.ppk, r[11], "Ppk"),
            (c.cpm, r[14], "Cpm"),
        ];
        for (got, want, what) in overall {
            close(got, want, 1e-9, &format!("{name} {what}"));
        }
    }
}
