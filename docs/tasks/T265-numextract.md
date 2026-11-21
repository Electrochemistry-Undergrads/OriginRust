# T265: NumExtract and NumBreak

**Status:** done 2026-11-20 (Claude Windows "claude:2"), **not measured** (the functions came with
Origin 2026 SR1; the reference install is Origin 2016).
HANDOFF "Next candidates" C, `docs/tasks/origin-2026-2027-gaps.md`: Origin's text-to-number
functions for moving Excel formulas. Built from OriginLab's LabTalk reference pages
(`docs.originlab.com/labtalk/ref/numextract-func/` and `numbreak-func/`); every example on them is
a test.
**Size:** S. **Spec:** `docs/spec/expression-language.md` 3 (string functions table).

## Done

- `StrFn::NumExtract` and `StrFn::NumBreak` in `crates/expr/src/text.rs` (one rule for which
  characters are a number: digits, and the decimal separator between two digits), the function
  names `numextract` and `numbreak`, and both in the Set Column Values function list.

## Tests

- `tests::numextract_and_numbreak_follow_origins_reference` (sheaf-expr).

## Choices

- OriginLab's NumBreak page says 0 when nothing changes but shows -1 for one example; Sheaf gives 0.
- NumExtract returns text, as the page's syntax says; `value()` makes it a number.
