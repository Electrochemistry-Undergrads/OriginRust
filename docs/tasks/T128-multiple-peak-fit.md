# T128: Analysis > Peaks and Baseline > Multiple Peak Fit

**Status:** done 2026-11-17 (Claude Windows, scheduled session "cond-heatmap", branch `multi-peak-fit`; analysis-tools.md 11.3), not measured (Origin COM unavailable). Individual peak curves added the same day. Left: Open NLFit, Use Peak Marker, the results table on the graph.
**Size:** M. **Source:** greyed inventory package B8 (Multiple Peak Fit; Batch Peak Analysis stays B8).
**Spec:** `docs/spec/analysis-tools.md` (a new subsection next to the Peak Analyzer, 11).

## Goal

Fit several peaks of the active data plot: choose the peak function, pick the peak centres on the
graph, and fit them together, as Origin's Multiple Peak Fit tool does.

## Origin facts (docs.originlab.com/origin-help/fit-multipeakfit-tool and x-function/ref/nlfitpeaks; not measured, Origin COM unavailable)

- Analysis: Peak and Baseline: Multiple Peak Fit opens the `nlfitpeaks` dialog: input data (the
  active data plot from a graph) and the peak function; OK activates the graph (from a worksheet a
  graph is made first).
- Then the Get Points dialog: move the cursor with the mouse or arrow keys; double-click or Enter
  picks a peak centre; the dialog shows the X and Y and the number of centres picked.
- **Fit** fits the data with the picked centres as starting points and adds a table of results to
  the graph; **Open NLFit** opens the NLFit dialog with peak heights and centres initialised.
- No baseline treatment (the Peak Analyzer does that); Use Peak Marker fits existing peak markers.

## Steps

1. Calc/core: the Peak Analyzer chain with given centres instead of a search (`peakchain`,
   `PeakChainSpec`), tests.
2. App: the nlfitpeaks window, the Get Points window on the graph's Data Reader, Fit writing the
   results and drawing the fitted curve on the graph; menu item; UI test.
3. Spec subsection, inventory, index.

## Done when

The item fits picked peaks of a plot as one undo step with the result table and the fitted curve;
Open NLFit, Use Peak Marker and the results table on the graph may be left (stated in Status).
fmt, strict Clippy and the full tests pass.
