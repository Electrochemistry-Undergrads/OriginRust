# T08: Missing Set Column Values functions in `sheaf-expr`

**Status:** groups 1–5 implemented (`04a8a80`, `d8d1416`), including text-valued formulas,
local `now()`/`today()`, measured running `sum` and row-counting `count`, and `cell()` dependencies.
Further completed: `substitute(s,new,find[,n])$`, optional `$` on text functions, built-in column
headers `G/L/U/C/O` with separate metadata dependencies, `char`/`code`, explicit `format`
compatibility, formatting columns, and direct-column Text defaults that follow format edits.
Reference sources, unmeasured edge choices and regressions are in expression-language.md section 3.
**Left:** cross-sheet/book references (group 6), parameter label rows, `%()` substitution,
and Origin global/`@SD` formatting defaults. **Size:** M. **Branch:** `expr-funcs2`. **Model:** Sonnet.
**Detail:** audit E "Missing" (formula functions). **Spec:** `docs/spec/expression-language.md`.

## Where

- Function table: `crates/expr/src/funcs.rs` (search the table near the top; `round` is handled
  before the table; `col()`/`wcol()` in `crates/expr/src/parser.rs`).
- Evaluation: `crates/expr/src/eval.rs`; tests in `crates/expr/src/tests.rs`.
- Date numbers: Origin epoch, Julian day 2440587 = 1970-01-01 (`crates/core/src/datetime.rs`).

## Missing (do in this order; each its own commit)

1. Statistics over a column: `cumsum`, `diff`, `shift`, `rank`, `stddevp`, `sem`.
2. Date/time: `date(...)`, `time(...)`, `now()`, `today()`, `year`, `month`, `day`, `weekday`.
3. Random: `rnd`, `grnd` (seeded per evaluation for reproducible tests).
4. String functions and `col(A)$` text access (needs text values in the evaluator; check how text
   cells are represented in `crates/core/src/column.rs` first).
5. `cell(r,c)`, `lookup`, `distance`, `fact`.
6. Cross-sheet and cross-book references `[Book]Sheet!col(A)`; `%()` substitution. These touch
   recalculation dependencies (only same-sheet inputs today); leave for last.

## Origin facts

Get reference values for each function with `tools/origin-ref/run-labtalk.ps1`: a sheet with known
inputs, `col(B)=f(col(A))`, dump the result. Remember `log` is base 10 in LabTalk (LESSONS).
Edge cases to measure: missing inputs, `diff` length (does Origin put the missing value first or
last?), `rank` ties.

## Done when

Each function matches Origin on its fixture to 1e-12 (or exactly for integers/strings); listed in
expression-language.md; checks pass.
