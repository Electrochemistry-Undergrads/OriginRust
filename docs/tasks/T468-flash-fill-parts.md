# T468: Flash Fill reads parts of words (rest of T430)

**Status:** done 2026-11-25 (Claude Windows "claude:work", workbook-model.md 13r). Flash Fill pieces can be the first
or last one to four characters of a word (`Jo` from `John`, `17` from `2017`), kept as `left(token(...), n)` and
`right(token(...), n)` in a formula. The search budget is 2000 steps.
**Size:** S. **Source:** T430 "Left".

## Tests

- `crates/core/src/flash_fill.rs`: `parts_of_words_are_read`.
