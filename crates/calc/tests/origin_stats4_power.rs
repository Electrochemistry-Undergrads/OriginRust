//! Power and sample size of the t tests and the one-way ANOVA against numbers from Origin.
//!
//! Reference scripts in `tools/origin-ref/stats4/` (`stats4-pss.c` calls Origin's statistics library; `g2.py`
//! writes the runs, `make-fixture2.py` the fixture `fixtures/stats4_power_origin.rs`). All inputs were generated
//! by us. Origin's powers carry a numerical error of up to 2.5e-5 (one sample, n = 5: 0.2113505 where the exact
//! value is 0.2113345; equal means give 0.0500001 instead of 0.05), so powers are compared to 3e-5; sample sizes are
//! whole numbers and must be equal, except above 10000, where Origin's own search is off by up to 1e-4 relative
//! (353200 where the smallest size is 353184).

#[path = "fixtures/stats4_power_origin.rs"]
mod fixtures;

use fixtures::*;
use sheaf_calc::*;

fn test_of(kind: u8) -> PowerTest {
    [PowerTest::OneSample, PowerTest::TwoSample, PowerTest::Paired][kind as usize - 1]
}

fn same(name: &str, what: &str, got: f64, want: f64, tol: f64) {
    if want.is_nan() {
        assert!(got.is_nan(), "{name}: {what}: got {got:?}, Origin leaves it empty");
    } else {
        assert!((got - want).abs() <= tol + tol * want.abs(), "{name}: {what}: got {got:?}, Origin gave {want:?}");
    }
}

#[test]
fn powers_and_sample_sizes_match_origin() {
    assert!(POWER.len() > 80);
    let (mut powers, mut sizes) = (0, 0);
    for c in POWER {
        let two_sided = c.tail == 0;
        let effect = c.b - c.a;
        if c.mode == 0 {
            let got = if c.kind == 4 {
                anova_powers(c.inputs, c.a, c.b, c.sd, c.alpha)
            } else {
                t_powers(test_of(c.kind), c.inputs, effect, c.sd, c.alpha, two_sided)
            };
            match c.rc {
                0 => {
                    let got = got.unwrap_or_else(|e| panic!("{}: {e}", c.name));
                    for (i, (g, w)) in got.iter().zip(c.outputs).enumerate() {
                        same(c.name, &format!("power {i}"), *g, *w, 3e-5);
                        powers += 1;
                    }
                }
                -151 => assert_eq!(got, Err(PowerError::BadInput), "{}", c.name),
                rc => panic!("{}: unexpected return code {rc}", c.name),
            }
        } else {
            let got = if c.kind == 4 {
                anova_sample_sizes(c.inputs, c.a, c.b, c.sd, c.alpha)
            } else {
                t_sample_sizes(test_of(c.kind), c.inputs, effect, c.sd, c.alpha, two_sided)
            };
            match c.rc {
                0 | -186 => {
                    let got = got.unwrap_or_else(|e| panic!("{}: {e}", c.name));
                    assert_eq!(got.below_lowest.iter().any(|b| *b), c.rc == -186, "{}: below the lowest power", c.name);
                    for (i, (g, w)) in got.sizes.iter().zip(c.outputs).enumerate() {
                        let tol = if *w > 10000.0 { 1e-4 } else { 0.0 };
                        same(c.name, &format!("size {i}"), *g, *w, tol);
                        sizes += 1;
                    }
                    match got.lowest {
                        Some(l) => same(c.name, "lowest power", l, c.lowest, 3e-5),
                        None => assert!(c.lowest.is_nan(), "{}: Origin gave a lowest power", c.name),
                    }
                }
                -151 => assert_eq!(got.map(|_| ()), Err(PowerError::BadInput), "{}", c.name),
                -187 => assert_eq!(got.map(|_| ()), Err(PowerError::NoEffect), "{}", c.name),
                -204 => assert_eq!(got.map(|_| ()), Err(PowerError::BadPower), "{}", c.name),
                rc => panic!("{}: unexpected return code {rc}", c.name),
            }
        }
    }
    assert!(powers > 350 && sizes > 200, "{powers} powers and {sizes} sizes were compared");
}
