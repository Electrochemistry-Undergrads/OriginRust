//! Power and sample size of the proportion and variance tests against scipy, made by `tools/power-rate-ref.py` (T415;
//! none of these numbers come from Origin). Powers to 1e-9, sample sizes exactly.

#[path = "fixtures/power_rate_reference.rs"]
mod fixtures;

use sheaf_calc::{PowerError, rate_powers, rate_sample_sizes};

#[test]
fn rate_powers_and_sizes_match_scipy() {
    for c in fixtures::CASES {
        let what = format!("{:?} {} {} alpha {} two-sided {}", c.test, c.a, c.b, c.alpha, c.two_sided);
        let p = rate_powers(c.test, c.sizes, c.a, c.b, c.alpha, c.two_sided).unwrap();
        for (got, want) in p.iter().zip(c.powers) {
            assert!((got - want).abs() < 1e-9, "{what}: {p:?} vs {:?}", c.powers);
        }
        let found = rate_sample_sizes(c.test, c.wanted, c.a, c.b, c.alpha, c.two_sided);
        match c.needed {
            None => assert_eq!(found, Err(PowerError::NoEffect), "{what}"),
            Some(needed) => {
                let found = found.unwrap();
                for (got, want) in found.sizes.iter().zip(needed) {
                    assert!(got == want || (got.is_nan() && want.is_nan()), "{what}: {:?} vs {needed:?}", found.sizes);
                }
            }
        }
    }
}
