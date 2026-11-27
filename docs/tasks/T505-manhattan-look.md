# T505: Manhattan plot colours, gaps and hit labels

**Status:** done 2026-11-27 (Claude Windows "claude:3", statistics 33). Left item of T362 (claude:gmail's); Origin's
Manhattan Plot app was not run, so this is Sheaf's own.
**Size:** S. **Source:** T362 "Left".

## What it does

Statistics > Manhattan Plot... colours the groups in two alternating colours (default; `--palette` for the colour
list), can leave a gap between groups (`--gap PCT` of the summed group spans) and can label the points at or above the
line with their position (`--label-hits`). Analysis graph plans now shift their data-label columns with the rest.

## Tests

- `crates/core/src/analysis/manhattan.rs`: `groups_are_laid_end_to_end_and_p_becomes_minus_log10`.
- `crates/cli/tests/analyze.rs`: the Manhattan test (`--gap`, `--label-hits`, a gap above 100 % refused).

## Left

- Group names at the group centres on the X axis (needs text tick labels on axes).
