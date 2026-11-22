# T371: Origin's vinc and vinc_check measured (HANDOFF B, follows T228)

**Status:** done 2026-11-22 (Claude Windows "claude:1"). Origin 2016's `vinc` (mean and sample standard
deviation of the steps) and `vinc_check` (default tolerance 0.05: the largest deviation of a step from the
mean step, relative to the step) were measured on 14 runs. `sheaf_core::sampling::check_increment` already
used the same test; it now also gives the sample standard deviation (`sd`), and `ORIGIN_TOLERANCE` names
Origin's default. Set Sampling Interval's From X Column keeps its 1e-6 tolerance (Sheaf's choice).
**Size:** S. **Spec:** `docs/spec/workbook-model.md` 13d.

## Tests

- `crates/core/src/sampling.rs`: `vinc_and_vinc_check_match_origin` (the measured cases).

## Left

- Origin reads a missing cell as about 0 inside `vinc`; Sheaf refuses it (kept).
