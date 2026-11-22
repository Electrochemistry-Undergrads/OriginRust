# T351: Peak Analyzer anchors picked on the graph

**Status:** done 2026-11-21 (Claude Windows "claude:gmail", analysis-tools 11.6a). The parity row "Peak Analyzer
wizard" (P0) said the baseline anchors could not be picked on a graph. The Baseline Mode page's User Defined anchor
finding now has "Pick on the graph (Sheaf's own)": Pick on Graph turns on the Data Reader of a graph of the input,
double-click or Enter adds the reader's point as an anchor, Remove Last / Clear / Stop Picking edit the list, and the
anchors are the baseline's anchor points (`ChainBaseline::Anchors`) for every goal.
**Size:** S. **Spec:** analysis-tools 11.6a.

## Left

- Moving or deleting the anchors found by the 2nd derivative on the graph (Origin's Modify/Add), free (not snapped)
  anchors, drawing the anchors and the baseline on the graph while picking.

## Tests

- `crates/app/src/app/ui_tests.rs`: `peak_analyzer_anchors_are_picked_on_the_graph` (the reader turns on and off,
  Enter picks without pressing OK, the baseline is the line through the picked anchors).
