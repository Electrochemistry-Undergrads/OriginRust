# T44: Ridgeline chart

**Status:** claimed 2026-11-15 (session "mac-multipdf", macOS, no Origin), branch `ridgeline`.
**Size:** M. **Source:** `origin-2026-2027-gaps.md` (Graphing: Sunburst and ridgeline plots, Origin
2022; the ridgeline half only).

## Goal

Plot > Statistical > Ridgeline Chart draws each selected Y column's distribution as a density curve
offset vertically by column index, with Origin's display types and the kernel bandwidth choices.

## Origin facts (docs, not measured)

- Menu "Plot > Statistical: Ridgeline Chart", template `ridgeline.OTPU`, "Select at least one Y
  column". [W:origin-help/ridgeline]
- Plot Details Display tab, Type: Distribution Curve, Distribution Curve with Data Points,
  Distribution Curve with Rug, Single Block Bar (histograms); Position: by default "the column
  index will be used to arrange the ridgelines"; Quantile Lines (check box, values 0-100, line
  style); Rug: Color, Thickness, Size(%), Gap Between Curve. [W:origin-help/pd-dialog-display-tab-ridgeline]
- Distribution tab: Curve Type Kernel Smooth by default for ridgelines; bandwidth Scott
  `0.9·A·n^(-1/5)`, Silverman `1.059·A·n^(-1/5)` with `A = min(SD, IQR/1.349)`, k×SD, Custom.
  [W:origin-help/pd-dialog-distribution-tab]
- Not documented (Sheaf's choices, unmeasured): default bandwidth method, number of evaluation
  points, ridge height/overlap, fill colours and transparency, baseline, Y-axis labels (column
  names at each ridge's baseline), histogram bins.

## Where

- Reuse `crates/calc/src/kde.rs` and the violin plot's bandwidth handling (`crates/plot/src/violin.rs`,
  T37 spec graphing 14.22) - share, don't duplicate; `crates/core` plot kind / template `Ridgeline`
  with per-plot ridgeline settings; `crates/plot` drawing (filled curves, baselines, points, rug,
  quantile lines, single block bars); Plot menu (Statistical); Plot Details controls only for the
  ridgeline settings; CLI `-t ridgeline`, MCP; `.opj` warns.
- `docs/spec/graphing.md` new 14.x section after the violin's.

## Done when

Ridgeline charts draw from the menu/CLI for one or more Y columns in all four display types, with
bandwidth choices; density values match the existing KDE (scipy-checked) code; tests and spec.

## Don't

- Don't change the violin plot's results or other plot kinds (byte-identical exports).
