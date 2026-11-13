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
- Text strings `"abc"` (no escape sequences; cannot contain a double quote) and `col(A)$` / `col(A)[i]$` (the text of a cell) are text values; see the text section below. `col("Long Name")` also takes a quoted name.

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
| Statistics of a column | `total`, `mean`, `median`, `sd` (`stddev`; sample, n-1), `var`, `count`, `min`, `max`, `first`, `last` (`sum` and `count` as Origin, see below) |

Statistics take one argument that must be a column or a row range: `mean(col(A))`, `sd(col(A)[1:10])`, `max(col(B))`. `min`/`max` with exactly one column argument are statistics; with one plain number they return it. A statistic whose bounds do not depend on the row is computed once, so `col(B) - mean(col(B)[1:10])` costs one pass. Moving windows such as `mean(col(A)[i-3:i+3])` are recomputed per row (cost proportional to the window). Missing values are skipped by statistics. `stddevp` is the population standard deviation (divisor n; Origin `stddevp` agrees to 1e-12) and `sem` is `sd/sqrt(n)` (**[choice]**: Origin has no `sem` formula function).

| Group | Functions |
|---|---|
| Whole column in, column out | `cumsum(col(A))` running sum; `diff(col(A))` forward difference (row i holds A[i+1]-A[i], the last row is missing); `shift(col(A), n)` row i holds A[i-n], rows that fall off the ends are missing; `rank(col(A))` ascending rank, ties share the average rank, missing stays missing |
| Date and time | `date(y,m,d)` (impossible date: missing), `time(h,m,s)` (fraction of a day, not wrapped), `now()`, `today()`, `year`, `month`, `day`, `weekday` (0 = Sunday) of a date number |
| Random | `rnd()` uniform in [0,1), `grnd()` standard normal; a new draw in every row; `Expr::eval_rows_seeded` / `compile_seeded` fix the seed for tests |
| Lookup | `lookup(x, xcol, ycol)` first exact match of `x` in `xcol`, the value of `ycol` in the same row, else missing; `cell(r, c)` the value at 1-based row `r` and column position `c` (both rounded; outside the sheet: missing) |
| Other | `distance(x1,y1,x2,y2)` Euclidean; `fact(n)` factorial of the whole part (negative, missing or above 170: missing) |

`cell(r, c)` is tracked as a dependency by the worksheet (`Expr::cell_reads`): a column given as a number is a source of the formula column, and a computed column number makes every other column of the sheet a source, so Auto recalculation and the circular-chain check see it. `cumsum`, `diff`, `shift`, `rank` take a column or a row range such as `cumsum(col(A)[2:4])` (the result starts at row 1). Each result is computed once per evaluation, not once per row. `now()` and `today()` read `Host::now()`, whose default is the system clock in UTC; the worksheet host (`SheetHost` in `crates/core/src/formula.rs`) overrides it with local time, as Origin does (`crates/core/src/localtime.rs` reads the offset from the system, daylight saving included).

Measured in Origin (`tools/origin-ref`, probes `re-workspace/scratch/expr-funcs/g1e..g4`): `diff` is the forward difference with the missing value last; `date(2020,1,1)` is 2458849; `weekday` is 0 for Sunday; `time(25,0,0)` is 1.0417; `fact(1.5)` is 1, `fact(171)` and `fact(-1)` are missing, `fact(20)` is 2.43290200817664E18; `lookup` needs an exact match; `cell(1.6,1)` rounds to row 2; `distance` takes four arguments. Origin does not accept `cumsum`, `shift`, `rank` or `sem` as formula functions (the column stays empty), so those four are Sheaf's own and are **not checked against Origin**. `sum` and `count` follow Origin since branch `expr-funcs2` (probes `re-workspace/scratch/expr-funcs2/a1`, `a2`, `a3`, local only). With A = 3 1 4 1 5 (missing) 9 2 6: `sum(col(A))` is the running sum, `3 4 8 9 14 . 23 25 31` (it skips the missing value and leaves its own row missing), so it is the same as `cumsum`; `sum(col(A)[5:7])` is `5 . 14` in rows 1 to 3 (the running sum of the range, from row 1); `total(col(A))` is 31 (the total of the values); `count(col(A))` is 9, **counting the missing cell**, and `count(col(A)[5:7])` is 3, `count(col(A)[0:3])` is 3 (the range is clamped), `count` of an empty range is 0; a column ends at its last value (trailing missing cells are not stored), so with A = . 1 4 1 5 . 9 2 . `count(col(A)[1:20])` is 8. `mean` (3.875), `max` (9), `min` (1) skip missing values as before. Earlier Sheaf versions returned the total for `sum` and the number of values for `count`; the only tests that relied on that were in `crates/expr/src/tests.rs`, and the Set Values dialog's descriptions were corrected. Origin also accepts an expression such as `sum(col(A)*2)` (still the running sum); Sheaf requires a column or a column range inside `sum`.

`gamma`, `erf`, `erfc`, `j0`, `j1` use short in-house implementations with relative error near 1e-14 for moderate arguments (`gamma` is exact for whole numbers up to 171; gamma at 0 and negative whole numbers is missing).

### Text values and string functions

Origin measured with `tools/origin-ref/run-labtalk-lines.ps1` (probes `re-workspace/scratch/expr-funcs2/c1` to `c7`, local only): a Text & Numeric column A filled with `Hello`, `  World  `, `abcabc`, 12.5, an empty text, `Hello World`, 42, a missing value and `-1e3`, then `col(X)=<formula>` in other columns; the results were read back cell by cell. What came out, and what Sheaf does:

| Formula | Origin gave | Sheaf |
|---|---|---|
| `col(A)$` | the text of the cell; a number as its text (42 as `42`); a missing value as `--` | same; a number is written like `text(x)` (see below) |
| `col(A)$ + col(B)$`, `"pre-" + col(A)$` | joined text | same |
| `col(A)$ + "n=" + len(col(A)$)` | `Hellon=`: a number joined to a text adds nothing | same |
| `col(A) + col(B)`, `col(A) + 1` on text cells | missing (text is missing in arithmetic) | same |
| `col(A)` alone (or as a `?:` branch) | copies the cell, text included | same: the column becomes Text & Numeric |
| `col(A)$ == col(B)$`, `col(A) == "Hello"` | 1 or 0 | same; a text never equals a number |
| `col(A)$ < col(B)$` | missing | same |
| `left(s,n)`, `right(s,n)` | first and last n characters; n = 0 or negative gives an empty text | same |
| `mid(s,start,n)`, `mid(s,start)` | 1-based; start below 1 counts as 1 (`mid(s,0,3)` is `mid(s,1,3)`); without n, to the end | same |
| `len(s)`, `upper(s)`, `lower(s)`, `trim(s)` | `len` of an empty text is 0; `trim` cuts both ends | same |
| `find(s,part[,start])` | 1-based position, -1 when absent, case matters, `start` 1-based and inclusive | same |
| `search(s,part[,start])` | the same ignoring case | same |
| `replace(s,pos,n,new)` | `replace("Hello",2,2,"ZZ")` is `HZZlo`; a `pos` past the end changes nothing | same |
| `compare(s,t[,case])` | **1 when equal**, else 0 (not an ordering); case matters unless `case` is 0 | same |
| `exact(s,t)` | 1 when equal with case | same |
| `value(s)` | the number in a text, missing for `Hello` | same (missing when there is no number) |
| `text(x)`, `text(x,fmt)` | `fmt` `*` or none: general; `*n`: n significant digits; `.n`: n decimals; `E*n`: engineering with a unit prefix; a missing value is `--` | same for these four forms; other format codes give the general form (Origin returned the code itself or an odd number) |

How `text(x)` writes numbers (the general form, no format given or `*`): up to five decimals with trailing zeros dropped (`3.14159`, `1234.5678`, `0.00123`), `E` form from the exponent -4 downward and for seven or more integer digits that do not round-trip (`1.23456E-4`, `1.23457E6`, `1E15`; 1000000 stays `1000000`). `*3`: `3.14`, `1.23E+03`, `-42.0`, `1.00E+10`, `0.00`. `.2`: `1234.57`, `1.23E-04`, `-42.00`. `E*3`: `1.23k`, `12.3k`, `1.23m`, `-500m`, `10.0G`. Halves round away from zero. Where the exponent thresholds between the measured points are unknown they were chosen to reproduce every measured value (`crates/expr/src/text.rs`, test `text_of_numbers_matches_origin_formats`).

How a bare column read decides between number and text: the result of the whole formula, a branch of `?:` or `ifelse`, an argument that a string function reads as text, and the operand next to a string in `+`, `==` and `!=` are read as text when the cell is a text cell; every other read of a column is the number, and a text cell is then missing. This reproduces `len(col(A))` (counts the text), `text(col(A),"*")` (formats the number of the cell, `--` for text) and `col(A)+1` (missing) as measured. A formula with text in it gives a Text & Numeric column when any row comes out as text (`Expr::eval_values`, `Value::Text`); the rows that come out as numbers stay numbers, and an empty text is a missing cell. `Expr::eval_rows` of such a formula gives numbers only, with text as missing. `compile()` and `Expr::eval` do not return text.

Not copied: `substitute(s,old,new[,n])` returned its text unchanged in every probe, so it is not offered. Origin's `ifelse(c,"a","b")` and `c ? "a" : "b"` came out empty in the probe (a column formula set from a script); Sheaf accepts text branches. `col(A)[L]$` label rows, `char`, `code`, `format` and `%()` substitution are not done. For a number cell `col(A)$` is Origin's displayed text (a `-1e3` typed into a cell reads back as typed); Sheaf has no per-cell typed text, so it writes the number as `text(x)` does.

## 4. Calling it from the program

`Expr::parse(text)`, then `eval(&host)` for one value, `eval_rows(&host, n)` for rows `0..n`, or `compile(&host)` to get an object that can be run row by row. Variables and column slices are resolved once at compile time and constant sub-expressions are folded, so a million-row column evaluates without parsing and without per-row allocation. `variables()` and `columns()` list what the formula depends on, in order of first use, so the worksheet can decide when to recalculate.

## 5. Missing values and errors

- Missing is NaN. Arithmetic and every function propagate it: `na()+1`, `sin(na())`, `max(1, na())` are missing.
- Results that are mathematically undefined are missing instead of infinite or an error: division by zero (`1/0` is missing, matching the shipped Origin example that uses `1/0` to mean "missing"), `0^-1`, `sqrt` of a negative number, `ln` or `log` of zero or a negative number, `mod(a,0)`, `asin(2)`. **[choice for the less obvious ones: they follow the same idea]** Overflow to infinity from ordinary arithmetic (`exp(1000)`) is left as infinity.
- Comparison `==` and `!=` treat missing as an ordinary value that equals itself, so `col(A)==1/0 ? mean(col(A)) : col(A)` fills gaps with the mean (an example shipped with Origin). Ordered comparisons (`<` and the rest) with a missing operand give 0. `&&`, `||`, `!` and the condition of `?:` treat missing as false. **[choice: this keeps filter-style formulas such as `col(A) > 0 ? ... : ...` working on columns with gaps]**
- Reading a cell the host does not have (a row past the end of a shorter column) gives missing.
- A statistic over no values: `sum` and `count` give 0, the others missing.
- Real errors are only: syntax errors, an unknown function, a wrong number of arguments, a row range outside a statistic, and (when evaluating against a host) an unknown variable or an unknown column. Each message names the problem and ends with `at position N`, the zero-based byte offset in the formula text; `ExprError::position()` returns the same number and `ExprError::message()` gives the text without that ending. The place is named once only: an unclosed parenthesis says where it was opened as "opened at offset N" and ends with the position where `)` was expected. Nesting beyond 64 levels of parentheses, signs or `?:`, or a flat chain longer than about 1000 operators, is rejected with an error instead of risking a stack overflow. No input makes the parser or evaluator panic.

## 6. Differences from Origin's LabTalk

- Only a single expression; no `before formula` scripts, assignments, string variables, `%()` substitution or user-defined functions. Those are separate features and are not part of this crate.
- No `col(A)[L]$` label access, no cross-sheet references `[Book]Sheet!col(A)` (the cells of other sheets are not visible to the evaluator), `x`/`y` as built-in column shorthands (a host may supply `x` as a variable). The `[a:b]` range is accepted only inside statistics.
- A single `=` is not a comparison here; use `==`.
- `e` is the constant 2.718...; a parameter literally named `e` in a fit function cannot be used (rename it).
- `i` is read-only and always the 1-based row number of the output row.
- Names of variables are case-sensitive; function names are not.
- Unknown names are errors at evaluation time, not silently zero.
