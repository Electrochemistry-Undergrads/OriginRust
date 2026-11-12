# ADR 0006: Fitting in the app and the command-line program, and where fit results go

Status: accepted (2026-11-11)

## Context

`sheaf-fit` computes linear, polynomial and nonlinear fits and returns one `FitReport` for all of
them (parameters with standard errors, t-values and confidence limits; statistics; an ANOVA table;
fitted values and residuals). The app and the command-line program need one rule for how a fit is
requested and where its results appear. ADR 0005 settles this for the other analysis tools, but
its `Tool` type is a small copyable option set and a fit carries more: a function definition,
initial values, bounds, fixed parameters and weights.

Origin 2016 writes a fit as a hierarchical report sheet (`FitLinear1`, `FitNL1`), a sheet with the
fitted curve and the residuals, and adds the fitted curve to the graph of the data.

## Decision

1. **One entry point.** `Project::fit(&FitRequest)` in `sheaf-core` (module `fitting`) is used by
   both the app and `sheaf fit`. A request names one Y column (its X column follows from the
   designations, as in ADR 0005), an optional y-error column for instrumental weighting, the rows
   (`RowRange` from ADR 0005), and the model: linear (with optional fixed intercept or slope),
   polynomial of a given order, a built-in function by name, a user-defined formula with its
   parameter list, or a function imported from an `.FDF` file. Per parameter: initial value,
   fixed flag, lower and upper bound.
2. **Report sheet.** A new sheet in the source workbook, named after the model family as Origin
   does (`FitLinear1`, `FitPolynomial1`, `FitNL1`, counting up), holds one row per parameter:
   name (text), value, standard error, t-value, Prob>|t|, lower and upper confidence limit,
   dependency, and whether it was fixed. A second sheet with the same name plus `Stats` holds the
   statistics as name and value rows (number of points, degrees of freedom, residual sum of
   squares, reduced chi-square, R-square, adjusted R-square, Pearson's r where defined, iterations,
   status) followed by the ANOVA rows. Both are plain numbers that can be plotted, exported and
   used in column formulas, and they are saved with the project like any sheet.
3. **Fitted values and residuals** (one value per input row) are new Y columns directly after the
   source column, following ADR 0005 rule 3: "Fitted Current" and "Residual of Current", with the
   source unit, and missing values for rows outside the range or left out of the fit. Both
   columns carry the flag `fit_result` (a column property saved with the project; projects saved
   before it existed load with it unset). A fit that is not told which columns to use (`sheaf fit`
   without `--y`, and the default column of the app's Fit window) takes the Y columns that a plot
   would show, except those flagged `fit_result`, so refitting a project does not fit the earlier
   results. They can still be fitted by naming them. Name matching was rejected because users
   rename columns.
4. **Fitted curve.** A sheet `FitCurve1` (counting up) holds an X column of equally spaced points
   over the fitted X range and the model evaluated there (default 1000 points, so that a curve
   fitted to a few points is still smooth on the graph).
5. **Graph.** If the source column is plotted in a graph layer, the fitted curve is added to that
   layer as a line plot in a colour that differs from the data. When the request comes from the
   app and no graph shows the data, the app makes one (data as symbols, fit as a line). The
   command-line program does the same when it is asked for an image (`--plot out.png`).
6. **Undo.** Each fit is one undo step named "Fit". On error nothing changes.
7. **The command-line program** prints the report as text tables (parameters, statistics,
   ANOVA) and can export the sheets like other analysis results.

## Reasons

- The same split as ADR 0005 (per-row values next to the source, result tables as sheets) keeps
  one mental model for all analysis results and needs no new element in the project file.
- Two report sheets instead of Origin's hierarchical sheet: our sheets are flat tables of equal
  length columns; parameters and statistics have different shapes.
- A dense fitted curve separate from the fitted values at the data points: the first is for
  drawing, the second for residual analysis and for formulas.

## Not decided here

- Refitting automatically when the data change (Origin's "Recalculate: Auto").
- Fitting several columns at once into one combined report, global fits with shared parameters,
  and batch fitting across workbooks (milestone 6, batch processing).
- Choosing the fit range by dragging on the graph.
