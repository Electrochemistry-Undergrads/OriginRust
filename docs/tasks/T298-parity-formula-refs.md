# T298: Stale parity row "Reference notation in formulas" corrected

**Status:** done 2026-11-21 (Claude Windows "claude:3"), docs only. The feature-parity row said there
were no other-sheet or other-book references, no `cell()` and no label access. The code has explicit
`[Book]Sheet!col(A)` references (T08, `docs/spec/expression-language.md` "Cross-sheet and cross-book
columns"), `cell(r, c)` (lookup functions) and header reads such as `col(A)[L]$`. Left in the row: the
`x`/`y` shorthand, range variables and `A1` shorthand.
**Size:** S. **Spec:** `docs/spec/feature-parity.md`.
