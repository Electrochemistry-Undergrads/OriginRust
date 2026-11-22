//! Process capability against numpy and scipy, made by `tools/capability-ref.py` (T365; none of these numbers come
//! from Origin). The reference integrates d2 itself, so values from the within deviation agree to the six decimals
//! of Sheaf's d2 table (1e-6 relative); the others to 1e-9 (1e-7 for the Weibull fit, solved another way).

#[path = "fixtures/capability_reference.rs"]
mod fixtures;

use sheaf_calc::capability::{CapModel, capability};

fn close(a: f64, b: f64, rel: f64, what: &str) {
    if b.is_nan() {
        assert!(a.is_nan(), "{what}: {a} vs NaN");
    } else {
        assert!((a - b).abs() <= rel * b.abs().max(1.0), "{what}: {a} vs {b}");
    }
}

#[test]
fn normal_capability_matches_scipy() {
    for (name, x, lsl, usl, n, r) in fixtures::NORMAL {
        let c = capability(x, *lsl, *usl, *n, CapModel::Normal).unwrap();
        let within = [
            (c.sd_within, r[0], "sd within"),
            (c.cp, r[2], "Cp"),
            (c.cpl, r[3], "CPL"),
            (c.cpu, r[4], "CPU"),
            (c.cpk, r[5], "Cpk"),
            (c.expected_within.below, r[12], "ppm below within"),
            (c.expected_within.above, r[13], "ppm above within"),
        ];
        for (got, want, what) in within {
            close(got, want, 1e-6, &format!("{name} {what}"));
        }
        let overall = [
            (c.sd_overall, r[1], "sd overall"),
            (c.pp, r[6], "Pp"),
            (c.ppl, r[7], "PPL"),
            (c.ppu, r[8], "PPU"),
            (c.ppk, r[9], "Ppk"),
            (c.expected_overall.below, r[10], "ppm below"),
            (c.expected_overall.above, r[11], "ppm above"),
        ];
        for (got, want, what) in overall {
            close(got, want, 1e-9, &format!("{name} {what}"));
        }
    }
}

#[test]
fn nonnormal_capability_matches_scipy() {
    for (name, x, lsl, usl, weibull, r) in fixtures::NONNORMAL {
        let model = if *weibull { CapModel::Weibull } else { CapModel::Lognormal };
        let c = capability(x, *lsl, *usl, 1, model).unwrap();
        let got =
            [c.params[0], c.params[1], c.pp, c.ppl, c.ppu, c.ppk, c.expected_overall.below, c.expected_overall.above];
        let names = ["param 1", "param 2", "Pp", "PPL", "PPU", "Ppk", "ppm below", "ppm above"];
        for i in 0..8 {
            close(got[i], r[i], 1e-7, &format!("{name} {}", names[i]));
        }
        assert!(c.cpk.is_nan() && c.sd_within.is_nan(), "{name}: no within indices");
    }
}
