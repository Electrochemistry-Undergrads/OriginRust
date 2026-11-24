# T365: Process capability (normal and nonnormal)

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", statistics 36). HANDOFF C (Origin 2027 Pro Quality
improvement apps, "assess nonnormal process capability"): Statistics > Process Capability... and `sheaf analyze
capability` give Cp, CPL, CPU, Cpk (within deviation from subgroup ranges or moving ranges), Pp, PPL, PPU, Ppk and
observed and expected parts per million for a normal model, and the percentile-method Pp/Ppk and expected parts per
million for a lognormal or Weibull fit, with a histogram, the fitted curve and the limits. Not measured in Origin.
**Size:** S. **Spec:** statistics 36.

## Left

- Box-Cox and Cpm with a target are T419. Johnson transformations, confidence intervals of the indices, a pooled-deviation
  within estimate.

## Tests

- `crates/calc/tests/capability_reference.rs`: three normal and three nonnormal cases against numpy/scipy
  (`tools/capability-ref.py`).
- `crates/calc/src/capability.rs`: moving ranges, subgroups with a short last one, one limit, refusals.
- `crates/core/src/analysis/capability.rs`: a missing value, Cpk, the Weibull model, no limits refused.
- `crates/cli/tests/analyze.rs`: `process_capability_runs_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `the_statistics_menu_computes_process_capability`.
