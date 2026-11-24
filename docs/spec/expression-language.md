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
- The column index `j` (Set Multiple Columns Values, T53). A worksheet host may supply `j`, the 1-based position in the sheet of the column whose formula is being calculated, as a variable (`col(A)*j`) and inside a column reference: `wcol(j-1)` is the column to the left of the column being set, `wcol(j+2)` two to the right, `col(j)` the column itself. In the parser `col(j+k)` and `col(j-k)` (a whole number `k`, no spaces needed) are a relative reference (`ColumnRef::Relative(offset)`, printed `col(j-1)`); a host that has no `j`, such as a fit function or a sheet filter, finds no column for it and reports an unknown column or variable. A relative reference outside the sheet (`wcol(j-1)` on column A) is an unknown column error. The sheet host defines `j` for every column formula, so the single-column Set Column Values window may use it too. Name rules, all **[choice]** until measured: the variable `j` follows the same host-wins rule as `i`; a bare `col(j)` or `wcol(j)` is a name first, as `col(A)` is: a column whose short name is j in any case (the tenth column of a default sheet is called J) or whose long name is exactly `j` is read, so formulas written before T53 keep their meaning; only on a sheet with no such column does a bare `col(j)` mean the position, where before it was an unknown column. The explicit form `col(j+0)` or `col(j-0)` is always the position. A formula that reads its own column (`col(j-0)` on column j) runs once, as for any self read. A layout change (insert, delete or move of a column) that changes the position of a column whose formula reads `j` (the variable or a position `col(j)`) recalculates it, or marks it pending in Manual mode, as for any other changed read. A user parameter row called `j` still wins in `col(A)[j]`, as it does for any name.
- The conditional `c ? a : b` evaluates only the branch it takes.
- Text strings `"abc"` (no escape sequences; cannot contain a double quote) and `col(A)$` / `col(A)[i]$` (the text of a cell) are text values; see the text section below. `col("Long Name")` also takes a quoted name.
- `col(A)[L]$` reads a column header rather than a data row; built-in header codes `G`, `L`, `U`, `C`, `O` are supported (see below).
- `col(A)[Temperature]$` reads a user parameter row. Use `value(col(A)[Temperature]$)` for a numeric calculation; `D1`, `D2`, ... refer to the displayed user parameter rows.

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
| Date and time | `date(y,m,d)` (impossible date: missing), `time(h,m,s)` (fraction of a day, not wrapped), `now()`, `today()`, `year`, `month`, `day`, `weekday` (0 = Sunday) of a date number; `hour`, `minute`, `second` of its time of day (T239); all seven give -1 for a missing input or a number without a date or time (T262, below) |
| Random | `rnd()` uniform in [0,1), `grnd()` standard normal; a new draw in every row; `Expr::eval_rows_seeded` / `compile_seeded` fix the seed for tests |
| Lookup | `lookup(x, xcol, ycol)` first exact match of `x` in `xcol`, the value of `ycol` in the same row, else missing; `cell(r, c)` the value at 1-based row `r` and column position `c` (both rounded; outside the sheet: missing) |
| Other | `distance(x1,y1,x2,y2)` Euclidean; `fact(n)` factorial of the whole part (negative, missing or above 170: missing) |

`cell(r, c)` is tracked as a dependency by the worksheet (`Expr::cell_reads`): a column given as a number is a source of the formula column, and a computed column number makes every other column of the sheet a source, so Auto recalculation and the circular-chain check see it. `cumsum`, `diff`, `shift`, `rank` take a column or a row range such as `cumsum(col(A)[2:4])` (the result starts at row 1). Each result is computed once per evaluation, not once per row. `now()` and `today()` read `Host::now()`, whose default is the system clock in UTC; the worksheet host (`SheetHost` in `crates/core/src/formula.rs`) overrides it with local time, as Origin does (`crates/core/src/localtime.rs` reads the offset from the system, daylight saving included).

`hour`, `minute` and `second` (T239; measured in Origin 2016 on 2026-11-22 on a worksheet column, probe
`re-workspace/scratch/t239/hms.lines.txt`, local only): the time of day rounded to the nearest
millisecond, so `date(2020,1,1)+0.75` is 18:00 and not 17:59:59.999. `hour` is 0 to 23, `minute` 0 to
59, and `second` keeps its fraction: Origin gave 12:34:56.789 for `date(2020,1,1)+time(12,34,56.789)`,
23:59:59.991 for `+0.9999999`, 0:00:59.6 for `+59.6/86400`, 13:30:15 for `time(13,30,15)` and 18:00 for
-0.25; 1.5 and a missing cell give -1 (the rules for numbers without a date are T262's, below). A
probe of the bounds on 2026-11-22 (`re-workspace/scratch/t367`, local only) agreed with T262: the
time parts of a negative time are -1 from -703784 on (today 2461366 minus 703784 is 1757582, the
day before the first date), the date window is 1757583 to 5373483, and before 1899-12-30 the time
of day is read backwards (2415017.25 is 18:00).

Measured in Origin (`tools/origin-ref`, probes `re-workspace/scratch/expr-funcs/g1e..g4`): `diff` is the forward difference with the missing value last; `date(2020,1,1)` is 2458849; `weekday` is 0 for Sunday; `time(25,0,0)` is 1.0417; `fact(1.5)` is 1, `fact(171)` and `fact(-1)` are missing, `fact(20)` is 2.43290200817664E18; `lookup` needs an exact match; `cell(1.6,1)` rounds to row 2; `distance` takes four arguments. Origin does not accept `cumsum`, `shift`, `rank` or `sem` as formula functions (the column stays empty), so those four are Sheaf's own and are **not checked against Origin**. `sum` and `count` follow Origin since branch `expr-funcs2` (probes `re-workspace/scratch/expr-funcs2/a1`, `a2`, `a3`, local only). With A = 3 1 4 1 5 (missing) 9 2 6: `sum(col(A))` is the running sum, `3 4 8 9 14 . 23 25 31` (it skips the missing value and leaves its own row missing), so it is the same as `cumsum`; `sum(col(A)[5:7])` is `5 . 14` in rows 1 to 3 (the running sum of the range, from row 1); `total(col(A))` is 31 (the total of the values); `count(col(A))` is 9, **counting the missing cell**, and `count(col(A)[5:7])` is 3, `count(col(A)[0:3])` is 3 (the range is clamped), `count` of an empty range is 0; a column ends at its last value (trailing missing cells are not stored), so with A = . 1 4 1 5 . 9 2 . `count(col(A)[1:20])` is 8. `mean` (3.875), `max` (9), `min` (1) skip missing values as before. Earlier Sheaf versions returned the total for `sum` and the number of values for `count`; the only tests that relied on that were in `crates/expr/src/tests.rs`, and the Set Values dialog's descriptions were corrected. Origin also accepts an expression such as `sum(col(A)*2)` (still the running sum); Sheaf requires a column or a column range inside `sum`.

**Masked cells (T264, measured in Origin 2016 on 2026-11-20).** A formula reads a masked cell as a
missing value: with A = 1, 4, ..., 36 (row 3 masked) and B = 10, ..., 60 (row 5 masked later),
`col(A)+col(B)` is missing in row 3 (and in row 5 once B's row 5 is masked) and `col(A)*col(B)` too.
Sheaf does the same for every read of a cell, row by row and in whole-column functions such as
`mean(col(A))` (the latter not measured). Test: `a_formula_reads_a_masked_cell_as_missing_as_origin`
(`crates/core/src/formula.rs`).

**Date and time parts without a date (T262, measured in Origin 2016 on 2026-11-20, probes
`re-workspace/scratch/t262/range*.lines.txt` on a worksheet column).** Dates run from 1757583
(0099-12-31) to 5373483 (9999-12-31 at midnight); outside that (1757582, 5373483.25, -0.25, 0.999,
1, 1721426, 4e7 were probed) `year`, `month`, `day` and `weekday` return -1, and so do all seven
functions for an empty cell. `hour`, `minute` and `second` also work on a time alone (a number below
1, negative too: -0.25 is 18:00, -2.75 is 06:00, -100 is 00:00), but give -1 from 1 up to the first
date and after the last. Two Windows-date habits show through: a day before 1899-12-30 (date number
2415018, day 0 of Windows' date numbers) reads its fraction backwards (2299159.25 is 18:00,
2415016.9 is 02:24, 1757583.99 is 00:14:24), and a time alone is taken as that time today, so for a
large negative time the same rules apply after adding today's date number (in November 2026 the
fraction turns below about -46346 and -1 comes below about -703781; both move with the date).
Sheaf copies all of this, using the system date (UTC) for "today". Test:
`date_and_time_parts_follow_origins_range_and_give_minus_one_without_a_date` (`crates/expr`).

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
| `NumExtract(s[,count[,decimal]])` | Origin 2026 SR1 (T265, from OriginLab's reference page; not in Origin 2016, unmeasured): the `count`-th run of number characters as text (1 the first, the default; 0 the last; empty text when there is none). Digits, and the decimal separator between two digits: the period, or the comma when `decimal` is 1. A `-` is not a sign: `NumExtract("Jan-12")` is `12`, `NumExtract("1-413-586-2013",2)` is `413`, `NumExtract("abcd12,343",1,1)` is `12,343` | wrap in `value()` for the number |
| `token(s, n)` | Sheaf's own (T430, for Flash Fill; not an Origin function): word `n` of `s`, a word being a run of letters and digits; a negative `n` counts from the end (-1 the last); empty text for 0, a fraction or no such word. `token("Smith, John A.", 2)` is `John`, `token("2024-03-17", -1)` is `17` | used by Flash Fill's formulas |
| `NumBreak(s[,offset[,decimal]])` | Origin 2026 SR1 (T265, as above): from the 1-based `offset` (default 1), the 1-based position of the first character whose kind (number or not) differs from the one at `offset`; 0 when none, or when `offset` is past the end. `NumBreak("Club45North")` 5, with offset 5: 7, with offset 7: 0. OriginLab's page also shows -1 for one no-change case while its Return section says 0; Sheaf gives 0 | same |
| `value(s)` | the number in a text, missing for `Hello` | same (missing when there is no number) |
| `text(x)`, `text(x,fmt)` | in measured General columns, `fmt` `*` or none: general; empty: `@SD` significant digits (default 14); `*n`: n significant digits; `.n`: n decimals; `E*n`: engineering with a unit prefix; a missing value is `--` | same for these forms at default settings; an omitted format on a direct numeric column/cell uses its column format; other format codes give the general form (Origin returned the code itself or an odd number) |

How general-format `text(x)` writes numbers (General source column, literal/arithmetic default, or explicit `*`): up to five decimals with trailing zeros dropped (`3.14159`, `1234.5678`, `0.00123`), `E` form from the exponent -4 downward and for seven or more integer digits that do not round-trip (`1.23456E-4`, `1.23457E6`, `1E15`; 1000000 stays `1000000`). `*3`: `3.14`, `1.23E+03`, `-42.0`, `1.00E+10`, `0.00`. `.2`: `1234.57`, `1.23E-04`, `-42.00`. `E*3`: `1.23k`, `12.3k`, `1.23m`, `-500m`, `10.0G`. Halves round away from zero. Where the exponent thresholds between the measured points are unknown they were chosen to reproduce every measured value (`crates/expr/src/text.rs`, test `text_of_numbers_matches_origin_formats`).

How a bare column read decides between number and text: the result of the whole formula, a branch of `?:` or `ifelse`, an argument that a string function reads as text, and the operand next to a string in `+`, `==` and `!=` are read as text when the cell is a text cell; every other read of a column is the number, and a text cell is then missing. This reproduces `len(col(A))` (counts the text), `text(col(A),"*")` (formats the number of the cell, `--` for text) and `col(A)+1` (missing) as measured. A formula with text in it gives a Text & Numeric column when any row comes out as text (`Expr::eval_values`, `Value::Text`); the rows that come out as numbers stay numbers, and an empty text is a missing cell. `Expr::eval_rows` of such a formula gives numbers only, with text as missing. `compile()` and `Expr::eval` do not return text.

`substitute(s,new,find[,n])$` follows [Origin's documented signature](https://docs.originlab.com/labtalk/ref/substitute-func/): the replacement is the second argument and the search text is the third. With no `n`, or `n = 0`, it replaces all matches; a positive integer chooses the 1-based occurrence. The documented example `substitute("abcdefabcdef",12,"bcd",0)$` gives `a12efa12ef` and is a regression fixture. This addition is based on the reference documentation, not a new installed-Origin probe.

Sheaf uses literal, case-sensitive, non-overlapping matches, preserving Unicode text. An absent occurrence leaves the text unchanged. An empty search or an invalid occurrence (negative, fractional, missing or nonfinite) also leaves it unchanged; these are Sheaf's guard choices for cases the reference does not specify. The first three arguments accept text columns or numbers written as text; the occurrence is numeric, so formulas can read it from another column. Text-returning function calls accept an optional `$` suffix (`upper(substitute(col(A)$,"clean","raw")$)$`); numeric functions do not. Unsuffixed calls remain accepted. Set Column Values lists `substitute` under Function > Text, and Auto recalculation tracks every source column.

Origin's `ifelse(c,"a","b")` and `c ? "a" : "b"` came out empty in the probe (a column formula set from a script); Sheaf accepts text branches. String-expression `%()` substitution is described below; user-configured global/`@SD` format settings remain unsupported (fresh defaults below). For a number cell `col(A)$` is Origin's displayed text (a `-1e3` typed into a cell reads back as typed); Sheaf has no per-cell typed text, so it writes the number as `text(x)` does.

### Percent string-expression substitution

T08 (2026-11-14, `codex/formula-string-substitution`) supports `%(expression)` inside a quoted
formula string, for example `"name=%(col(A)[L]$), unit=%(col(A)[U]$)"` or
`"run:%([Book1]Sheet1!col(A)[i]$)"`. Existing text expressions, including nested function
calls with quoted arguments, can supply the text. Numeric results use ordinary string
conversion. `%()` outside a quoted string converts the result to a number as `value(...)`
does: `%(col(A)[Scale]$)*i`. It retains the existing empty-text and nonnumeric-text rules.
The infix `%` remainder operator is unchanged.

Within quoted strings, `%(TAB)` inserts a tab, `%(CRLF)` inserts carriage return and line feed,
and `%%(` leaves a literal `%(`. Keyword matching ignores ASCII case and surrounding whitespace.
These are based on Origin's [string substitution reference](https://docs.originlab.com/labtalk/guide/string-substitution/)
and the installed Origin 2016 probe `scratch/formula-string-substitution/strings.lines.txt`.
Measured scalar results: `value:12.5`, `name=Voltage, unit=mV`, `VOLTAGE`, and numeric 13.5
from a substituted `12.5` plus 1. Control characters and the literal escape are also measured.
The row-by-row worksheet behavior is Sheaf's extension: the scripted Origin column assignment
did not populate its target, so no direct formula/recalculation parity is claimed.

Substitution uses the existing parsed expression tree. Cell and header dependencies, user
parameters, cross-sheet references, Auto/Manual/None, cycles, explicit recalculation and undo
follow the existing formula rules. Native saves and `.opj` companions preserve the formula
and its links. Original UTF-8 byte offsets remain in diagnostics. Empty or malformed `%()`
and unsupported multi-argument forms are errors; nested substitutions are bounded along with
the existing expression depth limits.

Cell/metadata text is copied as data and never reparsed. In Origin the substituted string
`2+3` can become executable expression text and produce 5; Sheaf's standalone conversion
returns missing for that text. Dynamic reference-name construction, session string variables,
range-variable names, graph/legend `@` forms, script macros
and general LabTalk statement substitution remain unsupported. Regressions in the expression,
core and I/O `string_substitution` tests cover the supported subset and this data boundary.

### Worksheet cell substitution

T08 (2026-11-15, `codex/formula-worksheet-substitution`) adds the static
`%([Book]Sheet,column,row[,format])` cell form, based on the same
[Origin reference](https://docs.originlab.com/labtalk/guide/string-substitution/) and fresh
Origin 2016 scalar probes in `scratch/formula-worksheet-substitution/` (`probe`, `control`,
`ties-names` and `row-boundaries`). For example,
`"full:%([Src]Sheet1,A,i); shown:%([Src]Sheet1,A,i,W)"` reads a remote cell in each formula row.
Outside quotes, the substituted text goes through the existing numeric-only conversion.

Book, sheet and column selectors are fixed in the formula. They use the qualified-column
lookup rules below: book short name or unambiguous long name, sheet name or one-based index,
column short name (then long-name fallback), quoted long name, or one-based index. Quoted
names follow ordinary expression quoting. Origin's short/long collision control returns
10 for bare `A` and 99 for `"A"` when another column's long name is A. The numeric row can be
an expression, including `i` or another column; it remains a data read even when a user
parameter has the same name. Rows below 1 give `###` before rounding; valid fractional rows
round half up (`1.499999` selects row 1, `1.5` selects row 2, `2.5` selects row 3).

Measured numeric output (A1 = pi, column set to two decimal places):

| Format | Substituted text |
|---|---|
| omitted or quoted empty | `3.14159265358979` |
| `W` | `3.14` |
| `.2` or `".2"` | `3.14` |
| `*4` | `3.142` |
| `E*3` | `3.14` |

Omitted/empty cell formats use 15 significant digits, independently of Text/Format's empty
format (`@SD=14`). Fractional trailing zeros are dropped. Further measured controls give
`12.3456789012346`, `1.23456789012346E-10`, `1000000000000000` for 1e15, and `1E-3` for
0.0009999999999999999. `W` uses the numeric column display format; other explicit codes
use the existing formatter. Text cells are copied unchanged, including Unicode and `2+3`,
even with `W` or `.2`. A stored numeric missing value gives `--`; a cell past the data gives
empty text. Infinite/out-of-range row expressions also give `###` in Sheaf.

Unknown columns/books/sheets retain Sheaf's normal diagnostic instead of Origin's `###` or
unresolved literal. Names containing commas use Sheaf's ordinary quoted-name parser; Origin's
corresponding script controls either failed execution or returned `###`, so that is an
expression-language extension. No double-leading-quote syntax is added. Book-only active-sheet
notation, dataset/range substitution, `@WL`, generated selectors/expressions, session variables
and dynamic format expressions remain unsupported. Cell text is never executed: Origin can
turn `2+3` into 5, whereas standalone Sheaf substitution returns missing. Formula evaluation,
row-by-row filling, dependency/cycle rules, Auto/Manual/None, Undo/Redo and native/companion
save behavior use Sheaf's existing machinery; scalar probes do not establish Origin parity
for that machinery. The `worksheet_substitution` expression/core/I/O tests cover these rules.

### Format compatibility name

`format(x[,fmt])$` uses the existing Text formatter for the explicit formats `*`, `*n`, `.n` and `E*n`. The [Origin Text/Format reference](https://docs.originlab.com/labtalk/ref/text-format-func/) documents equivalent explicit formatting; its `format(123000,"E*3")$` example gives `123k` and is a regression fixture. The first argument is numeric; the format argument can be a literal or a text column, including a different format in each row. Both `text` and `format` track that formatting column as a data dependency.

With no explicit format, `text(col(A))$` (or an indexed cell such as `text(col(A)[2])$`) uses the numeric source column's format. Decimal places, scientific/engineering, date/time, Month and Day of Week formats are available through `Host::formatted_numeric_cell`; General retains the existing expression formatter. Format-only edits invalidate Auto/Manual dependents in the same undo step as the edit. Text cells remain numeric missing values (`--`), and an explicit format still overrides the column.

An explicit empty format in either function uses Origin's default `@SD=14`: up to 14 significant
digits, with fractional trailing zeros dropped. This differs from omitted formats on
`format` or literal/arithmetic `text`, and explicit `"*"`, which use the existing general
formatter (matching the measured fresh Numeric Format settings). Thus
`text(12.345678901234567,"")$` is `12.345678901235`, whereas omitting the format gives
`12.34568`; empty formatting of `0.00012345678901234567` gives `1.2345678901235E-4`.
An empty format also overrides a source column's display settings. Missing remains `--`.

The [Text/Format reference](https://docs.originlab.com/labtalk/ref/text-format-func/) distinguishes
these cases; the [numeric conversion guide](https://docs.originlab.com/labtalk/guide/numeric-string-conversion/)
documents the default precision. Origin 2016 scalar probes confirmed fresh `@SD=14` and 16
numeric cases through Text/Format with omitted, empty and `"*"` formats (80 strings); a temporary
`@SD=6` control changed the empty-format result to `12.3457`. The probe restored its original
value; no Numeric Format preference was changed or saved. Eight independent boundary inputs
also match: `1e14` stays decimal, `1.2345678901234567e14` uses `E14`, and a value just below
`0.001` retains scientific notation after rounding (`1E-3`). Local evidence:
`scratch/formula-format-defaults/{probe,holdout}.lines.txt` and `.out.txt`.
Regressions in the new expression/core `format_defaults` fixtures cover precision,
row-dependent formats, column-format override, recalculation and Undo.

Compatibility limit: Sheaf uses fixed fresh-session defaults, with no mutable `@SD` or global
Numeric Format setting. Format is accepted when typed into Set Column Values; the picker keeps
the recommended Text name rather than adding a duplicate alias.

### Character codes

`char(n)$` and `code(s)` follow the published [Char](https://docs.originlab.com/labtalk/ref/char-func/) and [Code](https://docs.originlab.com/labtalk/ref/code-func/) references. `char(65)$` gives `A`; `code("abc")` is 97 and `code("")` is 0. Char accepts integer code points from 1 through 65535, including `char(181)$` = `µ` and `char(956)$` = `μ`. Numeric source columns work with Char; Code reads text cells and returns a number. Both appear in Set Column Values > Function > Text.

Sheaf returns empty text for invalid Char input (zero, negative, fractional, nonfinite, missing, above 65535, or a surrogate code point); these are explicit guard choices where the reference does not define a usable character. Code reads the first Unicode scalar value, not the first UTF-8 byte, consistently with Sheaf's other text functions. Supplementary characters can therefore produce a code above 65535, although Char retains its documented upper bound. The Unicode behavior and guard cases have unit coverage; no new installed-Origin probe is claimed.

### Column label values

The built-in header reads follow Origin's [Col reference](https://docs.originlab.com/labtalk/ref/col-func/) and [label-row table](https://docs.originlab.com/labtalk/ref/column-label-row-characters/). This support is based on the published reference, not a new installed-Origin probe.

| Code | Field | Example |
|---|---|---|
| `G` | Short Name | `col(A)[G]$` |
| `L` | Long Name | `col(A)[L]$` |
| `U` | Units | `col(A)[U]$` |
| `C` | Comments | `col(A)[C]$` |
| `O` | F(x)= formula text | `col(A)[O]$` |

The result is a scalar repeated across the requested output rows. Column short names, quoted long names, column numbers and `wcol(1)` work as for data reads. `$` returns the header text; without `$`, a built-in header is parsed as a number (missing for empty or nonnumeric text). An empty Long Name stays empty rather than falling back to the short name. A standalone built-in label code takes precedence over a variable with that name; an expression such as `col(A)[L+0]$` still indexes data rows using the variable `L`. Sampling intervals, categories and filter headers are not implemented.

`Host::column_label` supplies the metadata; its default is empty. An unknown column is still an error. `Expr::columns()` lists all sources; `data_columns()` and `label_columns()` separate reads of cells and headers. Only cell reads create calculation-order or cycle edges, so formulas reading their own headers remain Auto, and mutual header reads do not form a false cycle. Header edits (including units, comments and formula text) invalidate dependent Auto or Manual results within the same undo step. Header-only formulas use the current worksheet row count when recalculated; headers do not extend a formula's data-input range.

### User parameter label rows

Measured in installed Origin on 2026-11-14 through the shared reference lock (local probes
`re-workspace/scratch/formula-parameter-labels/labels.out.txt` and `scalars.out.txt`), following
Origin's [metadata guide](https://docs.originlab.com/labtalk/guide/accessing-metadata) and
[column label row reference](https://docs.originlab.com/labtalk/ref/column-label-row-characters/):

- `col(A)[Temperature]$`, lowercase `temperature`, and `col(A)[Bath Temp]$` return the named
  parameter text. Missing and empty labels return empty text. A parameter wins over a variable
  of the same name; a known variable without a matching parameter still indexes a data row
  (`j=2`, `col(A)[j]$` returned `2` in the synthetic column 1, 2, 3).
- With Temperature = `2.5`, `value(col(A)[Temperature]$)` returned 2.5 and multiplying it by
  row 3 returned 7.5. Direct `col(A)[Temperature]` was missing: user parameters are text, and
  callers must use `value(...$)` to convert. Nonnumeric parameter text converted to missing;
  empty text converted to zero. Sheaf's `value("")` now follows that measured zero result.
- `D1` and `D2` select the first and second displayed user parameter rows. Origin's probe used
  creation order (Temperature, Sample). **Existing Sheaf model difference:** its worksheet
  displays the union of parameter names alphabetically; aliases follow that same displayed
  order, including rows empty in the referenced column. Use names when matching an Origin
  sheet whose parameter rows have a different order. No new row-order field is introduced.

`Host::column_parameter` provides parameter text and distinguishes an existing empty row from
an unknown row. `Expr::resolve_parameters(host)` resolves label-versus-variable precedence
before inspecting `variables()`, `data_columns()` or `label_columns()`; evaluation resolves it
automatically. Worksheet dependencies use that resolved form, so self/mutual parameter reads
do not create false data cycles. Parameter edits update Auto results and mark Manual results
pending within the same undo step; None remains a one-time calculation. Adding/removing a row,
even by deleting the only column containing it, refreshes aliases on other columns without
marking unrelated data-only formulas pending.

The current parser supports identifier names and names containing spaces and numbers, such as
`[Bath Temp]` and `[Group 1]`. Names containing operator punctuation and names parsed as expression
constants (`e`, `pi`) require a `Dn` alias. Quotes around a parameter name are not supported
(the Origin probe also returned empty text for `["Temperature"]$`). An empty text assigned to
a numeric worksheet column is stored as a missing cell under the existing column rules.
Native saves and `.opj.sheaf` companions retain parameters and their formula links; the `.opj`
writer still warns that user parameter rows themselves require the companion.

Regressions: `crates/expr/tests/parameter_labels.rs`, `crates/core/tests/parameter_labels.rs`
and `crates/io/tests/parameter_labels.rs`. These use only generated metadata and cell values.

### Cross-sheet and cross-book columns

T08 group 6 (2026-11-14, `codex/formula-cross-sheet`) accepts explicit references such as
`[Book1]Sheet2!col(A)`, `Sheet2!col(A)` within the target workbook, and
`[Book1]2!wcol(1)` using a 1-based worksheet position. A column still uses its short name,
quoted long name, or 1-based position. Double quotes allow spaces in workbook and worksheet
names: `["My Book"]"Raw Data"!col("Current µA")`. Workbook references prefer the actual short
name, then accept a unique display title; ambiguous titles are refused. Workbook and worksheet
names ignore ASCII case. Short-name precedence and ambiguity refusal are Sheaf's explicit rules.
Ordinary unqualified columns still use the target worksheet. No active-window fallback or
file access occurs. A missing book, sheet or column is an error.

Qualified columns support the existing arithmetic, statistics, vector functions, row indices,
text reads, display-format conversion, built-in headers and user parameter rows. `i` continues
to refer to the output row. An initial fill without an explicit row range covers the longer of
the target worksheet and its data sources, so a remote column can fill an empty target sheet.
Later recalculation follows source lengths under the existing shrink/empty-source rule.
Header-only formulas need target rows or an explicit range.

The worksheet host resolves the complete `ColumnRef::Scoped`; `WorksheetRef` selects a sheet
by name or position. Recalculation now orders data dependencies across the entire project.
Manual columns retain their values and mark Pending until explicitly recalculated, then their
remote dependents update. Header and parameter dependencies remain separate from data cycles;
changing a source sheet's parameter-row order invalidates its `Dn` readers in other sheets.
Names and positions are resolved anew after structural edits. A moved indexed sheet can change
the source; adding an unrelated book leaves Manual formulas Current. Renaming/deleting a source
marks its readers Broken and retains their values; restoring it repairs the link. Formula text
is not rewritten when names change. Document edits, Undo and Redo include the entire chain.

Reference basis: Origin's published [range notation](https://docs.originlab.com/labtalk/guide/range-notation/)
and [Set Values reference rules](https://docs.originlab.com/origin-help/wks-setcolval-quickstart/).
The latter dates direct cross-sheet formulas to Origin 2017; earlier versions require a range
variable. Installed Origin 2016 probes in `scratch/formula-cross-sheet/ranges*.lines.txt`
confirm `[Src]Sheet1!col(A)` via a range variable gives `6,12,24` when multiplied by 3,
and `[src]other!col(A)` gives `11,21,31,41` after adding 1 to `10,20,30,40`.
The follow-up confirms an unqualified `Sheet1!col(A)` within the same book, sheet position 2,
its second value 20 and mean 25. It also records that `[Probe]` resolves a book whose actual
short name is `Book1` and display title is `Probe`.
The direct qualified-`col()` expression used here is Sheaf's extension of range notation;
direct-formula syntax, recalculation, structural edits and quoted-name details are not claimed
as measured Origin 2016 behavior. No range-variable declarations, spreadsheet `A1` shorthand,
multi-sheet ranges are added. The supported subset of `%()` substitution is described above.

Native saves retain these formulas. The `.opj` writer stores text and values but cannot write
their cross-sheet recalculation records: its existing warning says the external file uses mode
None, and the `.opj.sheaf` companion preserves the full mode and linkage for reopening in Sheaf.
Expression, model and native/companion round-trip regressions are in the `cross_sheet` and
`cross_sheet_formulas` test files. The sheet-only host used for filters and analysis conditions
has no project scope and continues to reject qualified references.

### Named cells (T396, T400, Origin 2022 feature, not measured)

A data cell's right-click Define Name... names the cell (`Project::define_name`, kept in `Workbook::names` and saved
in `.sheaf`). A formula of any column of that workbook reads the name as the cell's value: `col(B) * Rate`. Names
are matched without regard to letter case; a name starts with a letter, holds letters, digits and underscores, is
at most 32 characters, and `i`, `j` and `pi` are refused (formulas already use them). Defining a name that exists
moves it to the new cell. A text, empty or out-of-range cell reads as missing; a name whose column was deleted no
longer resolves. A formula that reads a name depends on the named cell's column, so Auto and Manual columns are
updated when that column changes. One undo step ("Define Name Rate"). The window prefills the cell's current name.

Named ranges (T400, Origin 2022 feature, one column only, not measured): with several cells of one column selected,
right-click inside the selection, Define Name... names those rows (`Project::define_range`; the same `Workbook::names`
entry with a last row). A formula uses the name where a column with a row range goes: `mean(Upper)` is
`mean(col(B)[2:4])`, and so are `total`, `count`, `sd`, `max(Upper)`, `cumsum(Upper)` and the other statistics
and column functions. The range's column may be on another sheet of the workbook. On its own (`Upper * 2`) the name
is refused: it is not one value. Rows past the column's end are not read. A selection over several columns names
only the right-clicked cell. Defining a one-row range names a single cell again.
Not done: blocks of several columns, project-wide names, a name manager, the `.opj` and Excel round trip.

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

- Only a single expression, with Before Formula Scripts limited to `name = expression` statements (T307, below); no other assignments, session string variables, graph/dataset `%()` substitution, generated expression text or user-defined functions. The supported string-expression and static worksheet-cell substitution subsets are described above.
- No spreadsheet cell notation (`A1`, bare cross-sheet `Sheet!A`), range-variable declarations, or `x`/`y` as built-in column shorthands (a host may supply `x` as a variable). Explicit cross-sheet `col()` references are described above. The `[a:b]` range is accepted only inside statistics. Parameter-row ordering and the supported parameter-name syntax are described above.
- Bare column names, measured in Origin 2016 (T338, 2026-11-21, `re-workspace/scratch/formula-shorthand/`:
  `csetvalue col:=col(C) formula:="..."` with A = 1..4, B = 10..40, A's long name `Long`): `A*2`,
  `A+B`, `B-A`, `a*2`, `Long*2`, `A1`, `A[2]`, `A$` and `i*A` all fill the column with missing values
  (an unknown name is an undefined LabTalk variable, not a column), `x*2` and `y*2` give 0 (`x` and `y`
  are LabTalk system variables, 0 unless set), and only `col(A)*2` reads the column. So Origin 2016 has
  no bare-name, `A1` or `x`/`y` column shorthand either. Sheaf refuses such a formula instead of writing
  missing values; when the unknown name is a column's short name (any case) the message says
  "to read column A, write col(A)" (`Host::names_column`).
- A single `=` is not a comparison here; use `==`.
- `e` is the constant 2.718...; a parameter literally named `e` in a fit function cannot be used (rename it).
- `i` is read-only and always the 1-based row number of the output row.
- `j` exists only where the host supplies the column position (a worksheet column formula); it is the position in the sheet, not the position within a multi-column selection **[choice, not measured]**. See section 7.
- Names of variables are case-sensitive; function names are not.
- Unknown names are errors at evaluation time, not silently zero.

### 6.1 Before Formula Scripts (T307, Sheaf's subset of Origin's LabTalk box)

Set Column Values' Before Formula Scripts box takes statements `name = expression`, one per line or
separated by `;`; a leading `double`, `int`, `const` or `range` is ignored and a line starting with `//` is
a comment. Every use of a name in the formula, and in later statements, stands for its expression, as if
it had been typed in place (`Expr::parse_with_script`): `double m = mean(col(B));` with the formula
`col(A) - m` gives `col(A) - (mean(col(B)))`, so the column's sources, Auto/Manual recalculation and the
cycle check see the columns the script reads. A name defined twice takes its last definition. Anything
else (LabTalk commands, loops, string variables, `if`) is refused with a message naming the line. The
script is kept with the formula (`Column::before_formula`, saved in `.sheaf`) and runs whenever the
formula runs; setting the formula without a script drops it. The `.opj` written here does not hold the
script; the save names such columns. Origin's own script language is far wider; this is not measured.

## 7. Set Multiple Columns Values (T53)

Origin's Set Multiple Columns Values (Column menu, Ctrl+Shift+Q) opens the Set Values window for the selected columns at once: one formula, `i` the row and `j` the column. Measured on Windows on 2026-11-15 (7.1); what is still marked **[choice]** is Sheaf's and not Origin's, or not measured. Originally everything marked **[choice]** was Sheaf's guess from Origin's public documentation and must be checked on Windows.

Core API (`sheaf-core`, `crates/core/src/formula.rs`):

- `Project::set_columns_values_with(&[ColumnId], formula, rows, mode)` sets the same formula on each column as `set_column_values_with` would, from left to right in sheet order whatever the order of the slice; `Document::set_columns_values(...)` does it as one undo step ("Set multiple columns values"). The columns must be on one sheet. Any failure (unknown column, cycle, text outside the rows) undoes the whole call and names the column when the error comes from the formula.
- Each column keeps the formula text exactly as typed, `j` included, and recalculates with its own `j`. **[choice]** Origin keeps the literal text too, but only on the first column of the range (7.1).
- Recalculate (None, Manual, Auto) and the row range apply to every column. For Auto and Manual the dependencies are resolved per column, so `wcol(j-1)` over B to E is a chain B reads A, C reads B, and so on, calculated in dependency order, and a change in A reaches all of them. A loop (`wcol(j+1)+wcol(j-1)` over B and C) is refused with the chain named; a column that reads itself runs once. A reorder or insertion of columns changes `j`, so recalculating columns re-resolve their reads (the same trigger as a renamed column).
- `j` is the position in the sheet, so selecting only column D gives `j = 4`, as in Origin (7.1).

To measure on Windows with `tools/origin-ref`:

1. The window title, and whether the column navigation buttons are shown or hidden.
2. Whether `j` is the sheet position or the position in the selection (select C and E and set `j`).
3. What each column's stored formula (the F(x) label row) shows afterwards: the literal text with `j`, or `j` substituted.
4. How Auto recalculation treats a `wcol(j-1)` chain, and a selection that makes a loop.
5. Whether a selection that includes a locked, text or non-numeric column is refused or silently skipped.
6. `col(j)`, `wcol(j)` and `col(J)` on a sheet with a column named J and on one without, and whether `wcol(j+k)` outside the sheet is an error or missing values.
7. Whether one undo step reverts every column (Sheaf: yes).

### 7.1 Measured in Origin 2016 (2026-11-15)

Run with `tools/origin-ref` (`re-workspace/scratch/t53`, local only): `csetvalue` from LabTalk for one
column at a time, and the window itself opened with Ctrl+Shift+Q after dragging across the column heads
(the watcher's `G:` drag), a formula typed and OK pressed. A selection made from a script
(`wks.colSel`, `worksheet -s`) is not taken by the window: Origin answers "The current selection will be
ignored. The column will be added to the end of the worksheet."

- **Window**: titled "Set Values - Multiple Columns"; menus Formula, wcol(1), Col(A), Function,
  Variables, Options; `Row ( i ): From <auto> To <auto>`; a second row `Col ( j ): From 3 To 5` (the
  selected columns C to E as **sheet positions**, editable); the formula label `wcol(j) = `; the four
  column buttons shown but greyed; Recalculate Manual by default; OK, Cancel, Apply (greyed) and the
  Before Formula Scripts box. Sheaf copies the title, the `Col (j)` row (editing it sets the columns at
  those positions; left as opened it sets exactly the selected columns), the label and the greyed
  buttons.
- **`j` is the sheet position**, also for a single column: `csetvalue` of `j` on column E gives 5, of
  `j*100+i` on D gives 402 in row 2. As Sheaf.
- **Order**: `wcol(j-1)*2+j` over C to E (B = 10 to 50) gives C = 23, D = 50, E = 105 in row 1: the
  columns are computed left to right, each reading the new values of the one before. As Sheaf.
- **Stored formula**: Origin keeps the formula **once, on the first column** of the range (C reads back
  `wcol(j-1)*2+j`, D and E read back empty), and the F(x)= cell is drawn across C to E; C gets the
  recalculation lock of the formula and D and E another one. **Not copied** [choice]: Sheaf keeps the
  literal text on every column, so each column shows and recalculates its own copy (same values; a
  column moved out of the range keeps its formula).
- **Outside the sheet**: `wcol(j+1)+1` over E and F (no column G) writes no values in either column and
  puts two messages in the Messages Log; the formula is kept on E. Sheaf refuses the call and changes
  nothing.
- Not measured: a selection that is not side by side (only a drag selection could be made), locked or
  text columns in the range, a column named J, undo.
