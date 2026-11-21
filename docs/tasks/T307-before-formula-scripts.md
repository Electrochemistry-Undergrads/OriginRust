# T307: Before Formula Scripts in Set Column Values

**Status:** done 2026-11-21 (Claude Windows "claude:3"). feature-parity "Set Column Values" (P0, gap
"no before-formula script or multi-column set"; the multi-column set existed, T53). The read-only box
became usable: `name = expression` statements (with optional `double`/`int`/`const`/`range`, `//`
comments) define names the formula uses. They are substituted into the formula
(`sheaf_expr::Expr::parse_with_script`), so dependencies, Auto/Manual recalculation and the cycle check
work unchanged. The script is kept on the column (`Column::before_formula`), applied by Set Column
Values and Set Multiple Columns Values, cleared with the formula, and named in the `.opj` save warnings.
**Size:** M. **Spec:** `docs/spec/expression-language.md` 6.1.

## Tests

- `crates/expr/src/tests.rs`: `before_formula_scripts_define_names_for_the_formula`.
- `crates/core/src/formula.rs`: `before_formula_scripts_run_with_the_formula_and_follow_its_inputs`.
- `crates/app/src/app/ui_tests/set_multiple.rs`: `a_before_formula_script_defines_a_name_for_the_formula`.
- `crates/io/src/opj/write/feature_tests.rs`: `a_before_formula_script_is_named_in_the_warnings`.

## Left

- General LabTalk in the box (commands, loops, string variables); where Origin keeps the script in `.opj`.
