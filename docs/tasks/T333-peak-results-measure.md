# T333: Measure the Peak Analyzer's Integrate Peaks output and limits

**Status:** measured 2026-11-21 (Claude Windows "claude:2"), docs only.
Feature-parity "Peak results layout" (P0). Origin 2016's Integrate goal writes Integration_Result1
(Index, Area, AreaIntgP(%), Row Index, Beginning X, Ending X, FWHM, centre, Height),
Integrated_Curve_Data1 and Plot_Data1 into the source book. Its limits stop at the data's local minima
(valley, and the right end of the last peak over a sloping background), where Sheaf walks on to the
baseline; on a rising ramp at the left edge Origin stops at a row that shrinks with the X step (rule not
found). **Size:** S. **Spec:** `docs/spec/analysis-tools.md` 11.8.

## Left

- Stop the limits at the local minimum as Origin does (needs a noise-proof rule and the
  `origin_peakchain` integ1 reference re-run over the new rows); the left-edge rule.
- Write Origin's three sheets (names, columns) for the Integrate goal.
