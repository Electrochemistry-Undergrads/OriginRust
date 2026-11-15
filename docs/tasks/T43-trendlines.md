# T43: On-graph trendlines

**Status:** done 2026-11-15 (session "mac-multipdf", macOS, no Origin), branch `trendlines`: `PlotDecor::trendlines`, `sheaf_calc::trend`, plot drawing, plot menu + dialog, `plot --trendline`, MCP `style.trendline`, `.opj` warns (graphing 18.11). Left: Mini Toolbar, mouse-dragged label, Origin's fit method/R-squared/averaging unmeasured.
**Size:** M. **Source:** `origin-2026-2027-gaps.md` (Graphing: On-graph trendlines, Origin 2025b).

## Goal

A data plot can carry one or more trendlines: a fitted Linear, Polynomial, Power, Logarithmic or
Exponential curve, or a Moving Average, drawn over the plot, recomputed live from the plotted data,
with an optional equation / R² label and its own line style; added, edited and removed from the
plot's context menu.

## Origin facts (docs, not measured)

- 2025b: "Mini toolbar to add trendline for single or multiple plots"; types Linear, Polynomial,
  Power, Logarithmic, Exponential, Moving Average. [W:originlab.com/2025b]
- Mini Toolbar: Add Trend Line on the single-plot and group-plot tabs; Polynomial order 2-10; Moving
  Average "Points of Window" 2-10; can "show the fitted equation/trend line equation and/or R-Square
  value" (Pearson's R for linear); not for Moving Average; Label Position button; line colour,
  thickness (pt) and style; a hidden trendline cannot be brought back (re-add it).
  [W:origin-help/mini-toolbar; quick-help/add-trendlines]
- Not documented (Sheaf's choices, unmeasured): default type (Linear), default order 2, window 2,
  fitting method for Power/Log/Exponential (linearized least squares on ln-transformed data, as
  spreadsheets do; points that cannot be transformed are skipped), R² definition per type, label
  default position (top left inside the layer), default style (plot colour, dash, 1.5 pt), whether
  it updates with data (Sheaf: yes, computed at draw time), no worksheet output.

## Where

- `crates/core/src/graph.rs`: `Vec<Trendline>` on a plot (serde default empty); `.opj` warns.
- `crates/calc` (or `crates/fit`): the trend fits (reuse existing linear/polynomial least squares).
- `crates/plot`: draw the curve over the plot's X range (log/reversed axes through the plot
  transform), the label text (equation with significant digits, R²).
- `crates/app`: plot context menu "Add Trendline..." / "Edit Trendline..." / "Remove Trendline", a
  small dialog (type, order/window, show equation, show R², colour, width, dash), undo; CLI
  `sheaf plot --trendline linear|poly:N|power|log|exp|ma:N`, MCP style option.

## Done when

Trendlines draw and update with the data in the app, exports and CLI; fits match reference values
(numpy/scipy) in calc tests; menu/dialog/undo/save tests; graphing.md records Origin's facts and
Sheaf's choices.

## Don't

- Don't change NLFit, the existing Linear/Polynomial Fit tools' results, or graphs without
  trendlines (byte-identical exports).
