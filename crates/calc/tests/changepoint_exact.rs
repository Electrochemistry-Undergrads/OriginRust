//! PELT change points against exact optimal partitioning (an O(n^2) dynamic programme over every split, in
//! `tools/changepoint-ref.py`, T346): the same change points, and the objective to 1e-9 relative. None of these
//! numbers come from Origin.

#[path = "fixtures/changepoint_exact.rs"]
mod fixtures;

use sheaf_calc::changepoint::{ChangeKind, Penalty, change_points};

#[test]
fn pelt_finds_the_optimal_partition() {
    for series in fixtures::SERIES {
        for run in series.runs {
            let kind = match run.kind {
                "Mean" => ChangeKind::Mean,
                "Variance" => ChangeKind::Variance,
                _ => ChangeKind::MeanVariance,
            };
            let penalty = match run.penalty {
                "Mbic" => Penalty::Mbic,
                "Bic" => Penalty::Bic,
                "Aic" => Penalty::Aic,
                "HannanQuinn" => Penalty::HannanQuinn,
                _ => Penalty::Manual(run.manual),
            };
            let got = change_points(series.x, kind, penalty, run.min_segment).unwrap();
            let what = format!("{} {} {} m={}", series.name, run.kind, run.penalty, run.min_segment);
            assert_eq!(got.changes, run.changes, "{what}");
            assert!(
                (got.objective - run.objective).abs() <= 1e-9 * run.objective.abs().max(1.0),
                "{what}: {} vs {}",
                got.objective,
                run.objective
            );
        }
    }
}
