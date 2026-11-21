# T281: Axis-title token options measured in Origin

**Status:** done 2026-11-21 (Claude Windows "claude:3"). HANDOFF next candidates B (T246's left item).
Origin 2016's `%(?Y,option)` in an axis title, set with `yl.text$` and exported as pictures
(`re-workspace/scratch/t281`, local) for four label sets: `@LL`/`@L` long name, `@LU` units, `@LC`
comments, `@LS`/`@C` short name, `@LA` long else short name, `@LN` comments else long else short name
with units, `@LG`/`@U`/none the built title, `@D` Book_Column, `@W`/`@WL`/`@WS` book and sheet names,
`@LP` empty, unknown `###`. Sheaf now draws the same (`token_option_text` in `crates/plot/src/layout.rs`).
Origin's `xl.text$` could not be set from the script in this run; the X tokens use the same code.
**Size:** S. **Spec:** `docs/spec/graphing.md` 3.4 (Tokens inside a title).

## Tests

- `crates/plot/src/offset_axis_tests.rs`: `axis_token_options_give_what_origin_2016_draws`.
