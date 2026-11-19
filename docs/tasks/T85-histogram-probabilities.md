# T85: Histogram + Probabilities (B4 slice)

**Status:** done 2026-11-19 (Claude Windows "t30-origin", branch `hist-prob-2`, taking over the stale `hist-prob` claim of session fe4b37eb, which held only this brief); graphing "Histogram + Probabilities". Left: the Results Log summary (Mean, SD, Maximum, Minimum, Size), the right axis title and the exact start of the step line (not measured).
**Size:** S-M. **Inventory:** Plot > Statistics > Histogram + Probabilities (1).

## Origin facts (measured 2026-11-16, Origin 2016, scratch `re-workspace/scratch/t85`)

With one Y column of 50 values (2.2 .. 6.4) selected, Plot > Statistics > Histogram + Probabilities
makes a graph with **two layers** (lock icon: the data is made by the command):
- layer 1: the histogram of the column, bars with a red outline and red diagonal hatching on white,
  bins of 0.5 here, X 1..7 (major 2), Y 0..12.5 titled "Counts";
- layer 2: linked to layer 1 (`layer.link` 1, X linked straight, Y not), Y 0..100 on the right, the
  cumulative percentage of the same bins as a blue step line (flat across each bin at the percentage
  reached at its right edge, rising at the bin edges).
- The Results Log gets "Histogram and Probabilities for Book1 col(B)" with Mean, SD, Maximum, Minimum,
  Size. Not measured: the right axis title (hidden in the capture), the exact start of the step line.

## Where

A plot kind for the cumulative curve sharing the histogram's bins (`HistogramStyle`), a template that
builds the two layers (the histogram plot and a linked right-Y layer, as Double-Y does), menu item,
CLI template name, tests, graphing spec.
