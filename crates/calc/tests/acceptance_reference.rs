//! Acceptance sampling against scipy, made by `tools/acceptance-ref.py` (T369; none of these numbers come from
//! Origin). Plans by brute force must agree exactly; probabilities, AOQ and ATI to 1e-10 relative; the AOQL to
//! 1e-9 and its fraction to 1e-6 (the maximum is flat).

#[path = "fixtures/acceptance_reference.rs"]
mod fixtures;

use sheaf_calc::acceptance::{SamplingPlan, accept_probability, aoq, aoql, ati, design};

fn close(a: f64, b: f64, rel: f64, what: &str) {
    assert!((a - b).abs() <= rel * b.abs().max(1e-300) || (a - b).abs() < 1e-300, "{what}: {a} vs {b}");
}

#[test]
fn designed_plans_are_the_smallest_brute_force_plans() {
    for &(aql, ltpd, alpha, beta, n, c) in fixtures::DESIGNS {
        assert_eq!(design(aql, ltpd, alpha, beta).unwrap(), SamplingPlan { n, c }, "{aql} {ltpd} {alpha} {beta}");
    }
}

#[test]
fn oc_aoq_and_ati_match_scipy() {
    for &(n, c, lot, p, pa, q, t) in fixtures::CURVES {
        let plan = SamplingPlan { n, c };
        let what = format!("n {n} c {c} lot {lot} p {p}");
        close(accept_probability(plan, p), pa, 1e-10, &format!("Pa {what}"));
        close(aoq(plan, lot, p), q, 1e-10, &format!("AOQ {what}"));
        close(ati(plan, lot, p), t, 1e-10, &format!("ATI {what}"));
    }
}

#[test]
fn the_aoql_matches_scipy() {
    for &(n, c, lot, q, p) in fixtures::AOQL {
        let (got_q, got_p) = aoql(SamplingPlan { n, c }, lot);
        close(got_q, q, 1e-9, &format!("AOQL n {n} c {c}"));
        assert!((got_p - p).abs() < 1e-6, "p at the AOQL n {n} c {c}: {got_p} vs {p}");
    }
}
