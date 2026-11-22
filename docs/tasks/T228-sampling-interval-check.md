# T228: Sampling interval check (parity "Sampling interval check", vinc)

**Status:** done 2026-11-20 (Claude Windows "claude:1"); the rule was **measured** against Origin's `vinc` / `vinc_check` on 2026-11-22 (T371). `sheaf_core::sampling::check_increment`
gives the start, mean step and largest step deviation of a run of X values and whether they are evenly
spaced; Set Sampling Interval's "From X Column" button fills Initial X Value and X Increment from the
selected column's evenly spaced X column, or says why it cannot.
**Size:** S. **Source:** feature-parity "Sampling interval check" (vinc, vinc_check).
**Spec:** `docs/spec/workbook-model.md` 13d.

## Tests

- `crates/core/src/sampling.rs` (`increment_tests`): even, uneven, too short, missing value, zero step,
  steps of 0.1.
- `crates/app/src/app/ui_tests/sampling.rs`: From X Column sets 0 and 0.5 from an X column of 0, 0.5, 1,
  1.5; an uneven X column gives the message and nothing changes.

## Left

- Origin's own tolerance and its `vinc` output were not measured; no `vinc` command or LabTalk function.
