# T485: LaTeX math in worksheet label rows

**Status:** done 2026-11-26 (Claude Windows "claude:3", graphing "LaTeX math in graph text"). Origin 2023b renders LaTeX in column
labels; Origin 2016 has none, so the look is Sheaf's own.
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Built-in equation rendering": metadata equations remain.

## What it does

A label-row cell (Long Name, Units, Comments, parameter rows) whose text holds a `$...$` math span is drawn as the parsed runs, as a
Rich Text cell is: italic letters, Greek, scripts. Stacked fractions, limits and matrices read flat in the cell. Plain labels keep their
drawing; editing shows the typed text.

## Tests

- `crates/app/src/app/ui_tests.rs`: `a_label_with_latex_math_shows_its_letters_and_scripts`.
