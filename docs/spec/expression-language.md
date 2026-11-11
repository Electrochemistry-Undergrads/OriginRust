# Expression language

Summary. One small expression language serves two jobs: worksheet column formulas ("Set Column Values", for example `col(B)*1000` or `sin(col(A)) + i/10`) and user-defined fit functions (the right-hand side of `y0 + A*exp(-x/t)`). An expression is parsed once into a tree (`Expr::parse`) and then evaluated against a *host*, which supplies the values of variables and worksheet cells. Everything is a double-precision number; truth values are 1 and 0; a missing value is NaN and flows through arithmetic. Errors are reported in plain English with the character position. The implementation is the `sheaf-expr` crate. Where Origin's own formula language (LabTalk) was documented in `workbook-model.md` section 9 to 11 the same spelling is used; where it was not, the choice below is what a scientist used to Origin would expect and is marked **[choice]**.

## 1. Syntax

- Numbers: `42`, `0.5`, `.5`, `5.`, `1e-3`, `2.5E+2`. No digit separators, no hex.
- Names: letters, digits and `_`, starting with a letter or `_`. Function names and the constants `pi` and `e` ignore case (`SIN`, `Pi`). Variable names are case-sensitive, because fit parameters are (`Z0` and `z0` differ in the shipped fit functions).
- Whitespace between tokens is free. There are no comments and no statements: one expression, no `;`.
- Column references:
  - `col(A)`: by short name. The host decides how names match; the ready-made host ignores case.
  - `col("Long Name")`: by long name (quoted, no escape sequences; the name cannot contain a double quote).
  - `col(3)` and `wcol(3)`: by 1-based position in the sheet. Both spellings mean the same thing.
  - `col(A)[i-1]`: the value at an explicit 1-based row; the subscript is any expression. A row that does not exist (below 1 or past the end) is missing. A non-integer subscript is rounded.
  - `col(A)[a:b]`: rows `a` to `b` inclusive (1-based), allowed only as the argument of a statistic such as `mean(col(A)[1:10])`. Bounds are clamped to the column; an empty range gives the statistic of nothing (see section 5).
  - A bare `col(A)` is the value in the row being calculated.
  - Only column references take a subscript; `(x+1)[2]` is an error.
- Variables: any other name is a variable looked up in the host (fit parameters, `x`, and so on). The name `i` is the 1-based number of the row being calculated, unless the host defines a variable called `i` itself. **[choice: the host wins, so a fit function may use `i` as a parameter]**
- The conditional `c ? a : b` evaluates only the branch it takes.
- Text strings exist only inside `col("...")`. There is no string arithmetic.

## 2. Operators, tightest first

| Level | Operators | Notes |
|---|---|---|
| 1 | `( )`, calls, `col(..)[..]` | |
| 2 | `^` | right associative: `2^3^2` is 512. The exponent may carry a sign: `2^-1` is 0.5 |
| 3 | unary `-` `+` `!` | binds looser than `^`: `-2^2` is -4 |
| 4 | `*` `/` `%` | left associative. `%` keeps the sign of the dividend, like `mod(a,b)` |
| 5 | `+` `-` | left associative |
| 6 | `<` `<=` `>` `>=` `==` `!=` | give 1 or 0; same level, left associative |
| 7 | `&&` | 1 or 0 |
| 8 | `\|\|` | 1 or 0 |
| 9 | `? :` | right associative: `a ? b : c ? d : e` |

A single `=`, `&` or `\|` is an error with a hint, not a silent alternative spelling. Assignment is not part of the language (the fit-function importer strips the `y =` prefix before parsing).

Truth: a number is true when it is neither 0 nor missing.

## 3. Functions

Arguments are numbers. Names are case-insensitive.

| Group | Functions |
|---|---|
| Powers, logs | `sqrt`, `exp`, `ln` (natural), `log` and `log10` (base 10, as in Origin), `log2`, `pow(a,b)` (same as `a^b`), `sqr` (square) |
| Trigonometry | `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2(y,x)`, `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`, `degrees`, `radians` (radians in and out for the trigonometric functions) |
| Rounding, sign | `abs` (`fabs`), `int` (truncate toward zero), `nint` (nearest, halves away from zero), `round(x)` (same as `nint`), `round(x, digits)`, `floor`, `ceil`, `sign` |
| Remainder | `mod(a,b)` (sign of `a`, like `%`), `rmod(a,b)` (sign of `b`) |
| Several values | `min(a,b,...)`, `max(a,b,...)`: two or more arguments, elementwise |
| Special | `erf`, `erfc`, `gamma`, `lgamma` (log of the absolute value), `beta(a,b)`, `j0`, `j1` (Bessel, first kind), `heaviside` (1 for x >= 0 else 0) |
| Missing, branching | `na()` (a missing value), `isna(x)` (1 or 0), `ifelse(c,a,b)` (also `if`) |
| Statistics of a column | `sum` (`total`), `mean`, `median`, `sd` (`stddev`; sample, n-1), `var`, `count`, `min`, `max`, `first`, `last` |

Statistics take one argument that must be a column or a row range: `mean(col(A))`, `sd(col(A)[1:10])`, `max(col(B))`. `min`/`max` with exactly one column argument are statistics; with one plain number they return it. A statistic whose bounds do not depend on the row is computed once, so `col(B) - mean(col(B)[1:10])` costs one pass. Moving windows such as `mean(col(A)[i-3:i+3])` are recomputed per row (cost proportional to the window). Missing values are skipped by statistics. Vector-valued functions (running sums, differences, shifts) are not part of the language; they are worksheet operations.

`gamma`, `erf`, `erfc`, `j0`, `j1` use short in-house implementations with relative error near 1e-14 for moderate arguments (`gamma` is exact for whole numbers up to 171; gamma at 0 and negative whole numbers is missing).

## 4. Calling it from the program

`Expr::parse(text)`, then `eval(&host)` for one value, `eval_rows(&host, n)` for rows `0..n`, or `compile(&host)` to get an object that can be run row by row. Variables and column slices are resolved once at compile time and constant sub-expressions are folded, so a million-row column evaluates without parsing and without per-row allocation. `variables()` and `columns()` list what the formula depends on, in order of first use, so the worksheet can decide when to recalculate.

## 5. Missing values and errors

- Missing is NaN. Arithmetic and every function propagate it: `na()+1`, `sin(na())`, `max(1, na())` are missing.
- Results that are mathematically undefined are missing instead of infinite or an error: division by zero (`1/0` is missing, matching the shipped Origin example that uses `1/0` to mean "missing"), `0^-1`, `sqrt` of a negative number, `ln` or `log` of zero or a negative number, `mod(a,0)`, `asin(2)`. **[choice for the less obvious ones: they follow the same idea]** Overflow to infinity from ordinary arithmetic (`exp(1000)`) is left as infinity.
- Comparison `==` and `!=` treat missing as an ordinary value that equals itself, so `col(A)==1/0 ? mean(col(A)) : col(A)` fills gaps with the mean (an example shipped with Origin). Ordered comparisons (`<` and the rest) with a missing operand give 0. `&&`, `||`, `!` and the condition of `?:` treat missing as false. **[choice: this keeps filter-style formulas such as `col(A) > 0 ? ... : ...` working on columns with gaps]**
- Reading a cell the host does not have (a row past the end of a shorter column) gives missing.
- A statistic over no values: `sum` and `count` give 0, the others missing.
- Real errors are only: syntax errors, an unknown function, a wrong number of arguments, a row range outside a statistic, and (when evaluating against a host) an unknown variable or an unknown column. Each message names the problem and ends with `at position N`, the zero-based byte offset in the formula text; `ExprError::position()` returns the same number. Nesting beyond 64 levels of parentheses, signs or `?:`, or a flat chain longer than about 1000 operators, is rejected with an error instead of risking a stack overflow. No input makes the parser or evaluator panic.

## 6. Differences from Origin's LabTalk

- Only a single expression; no `before formula` scripts, assignments, string variables, `%()` substitution, user-defined functions or string functions. Those are separate features and are not part of this crate.
- No dataset-valued formulas (a formula returning a whole vector, `cumsum`, `diff`, `shift`) and no `cell(r,c)`, `col(A)$`, `col(A)[L]$` label access, `x`/`y` as built-in column shorthands (a host may supply `x` as a variable). The `[a:b]` range is accepted only inside statistics.
- A single `=` is not a comparison here; use `==`.
- `e` is the constant 2.718...; a parameter literally named `e` in a fit function cannot be used (rename it).
- `i` is read-only and always the 1-based row number of the output row.
- Names of variables are case-sensitive; function names are not.
- Unknown names are errors at evaluation time, not silently zero.
