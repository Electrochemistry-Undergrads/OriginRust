# T261: Tukey p-values in the far tail

**Status:** done 2026-11-20 (Claude Windows "claude:2"), measured against Origin 2016 on 2026-11-20.
HANDOFF "Next candidates" A1. Running both programs showed Origin's Tukey p-values and interval ends
differing from Sheaf's while the differences, SEs and q agreed to 1e-14. Checking against SciPy's
`studentized_range` and an adaptive high-precision quadrature showed that Sheaf was the accurate one
and Origin 2016 is off by about 1e-7 absolute (3.4 % on a p of 1.3e-6), so Origin is not followed
here. Sheaf's own small p-values still lost up to ~1e-5 relative to the `1 - cdf` cancellation;
`ptukey_upper` now computes `P(Q > q)` directly to about 1e-9 relative.
**Size:** S. **Source:** HANDOFF.md "Next candidates" A1;
`re-workspace/scratch/origin-diff/stats-fit/REPORT.md`.
**Spec:** `docs/spec/statistics.md` 2 ("Accuracy of the Tukey p-value").

## Done

- `calc::ptukey_upper` (`crates/calc/src/anova.rs`): the range's upper tail
  `k * integral phi(z) Phi(z)^(k-1) (1 - (1 - Phi(z - w)/Phi(z))^(k-1))` through `expm1`/`ln_1p`,
  and the scale integral over the window where its integrand is within e^-46 of its peak. The Tukey
  comparisons use it.

## Tests

- `anova::tukey_tail_tests::tukey_tails_keep_their_digits`: eight reference values (2e-9 relative),
  p from 0.93 down to 3.9e-21.
- `anova::tukey_tail_tests::origin_2016_is_less_accurate_here`: the A1 case.

## Left

- `qtukey` still bisects `ptukey` (critical values agree with SciPy to 2e-10, enough for 0.05).
