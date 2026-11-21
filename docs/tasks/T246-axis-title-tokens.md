# T246: `%(?X)` and `%(?Y)` inside axis titles

**Status:** done 2026-11-20 (Claude Windows "claude:3"). An axis title with Origin's `%(?X)` or
`%(?Y)` tokens (options after a comma ignored) is drawn with each token replaced by the automatic
title of that axis; the `.opj` reader keeps text around a token instead of dropping it.
**Size:** S. **Source:** feature-parity "Axis titles" (P0) gap.
**Spec:** `docs/spec/graphing.md` 3.4.

## Tests

- `crates/plot/src/offset_axis_tests.rs`: `axis_title_tokens_are_replaced_by_the_built_titles`.
- `crates/io/src/opj/graph.rs`: `only_a_lone_axis_token_is_the_automatic_title`.

## Left

- Title rotation and offset. The token options were measured and copied in T281.
