# T338: Bare column names in Set Column Values, measured

**Status:** done 2026-11-21 (Claude Windows "claude:2"), measured.
Feature-parity "Reference notation in formulas" (P0) gaps "no x/y shorthand, no A1 shorthand". Origin
2016's `csetvalue` has none of them: `A*2`, `A1`, `Long*2` give missing values and `x*2`, `y*2` give 0
(LabTalk variables). The gaps belong to later Origin versions. Sheaf keeps refusing the formula, and now
says "to read column A, write col(A)" when the name is a column's short name.
**Size:** S. **Spec:** `docs/spec/expression-language.md` 6. Probe: `re-workspace/scratch/formula-shorthand/`.

## Tests

- `formula::tests::a_bare_column_name_is_refused_with_the_col_form` (core).

## Left

- Range variables inside the formula itself (later Origin versions' shorthands are not 2016 parity).
