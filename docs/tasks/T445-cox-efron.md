# T445: Cox regression with Efron's ties

**Status:** done 2026-11-25 (Claude Windows "claude:gmail", statistics 52). T408's "Left": Statistics > Survival
Analysis > Cox Proportional Hazards... ("Tied times": Breslow or Efron) and `sheaf analyze cox --ties efron` use
Efron's approximation for tied failure times in the partial likelihood and the baseline survival. Breslow stays the
default. Not measured in Origin.
**Size:** S. **Spec:** statistics 52.

## Left

- Exact (discrete) ties, strata, time-varying covariates, residuals, the proportional-hazards check.

## Tests

- `crates/calc/tests/cox_reference.rs`: two Efron cases against an independent numpy/scipy fit (`tools/cox-ref.py`).
- `crates/calc/src/cox.rs`: Efron equals Breslow without ties, both null log-likelihoods by hand with a tie.
- `crates/cli/tests/analyze.rs` (`--ties`), `crates/app/src/app/ui_tests.rs` (`the_survival_menu_fits_a_cox_model`).
