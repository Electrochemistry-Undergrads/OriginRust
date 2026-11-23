# T397: qPCR relative quantification and standard curves

**Status:** done 2026-11-23 (Claude Windows "claude:gmail", statistics 50). HANDOFF C (Origin 2027 Pro assay apps,
"analyze qPCR experiments"): Statistics > Quantitative PCR > Relative Quantification... / `sheaf analyze qpcr` give dCt,
ddCt and the Livak or Pfaffl expression ratio per group against a control, with a Welch interval and test and a graph;
Standard Curve... / `sheaf analyze qpcr-curve` give the amplification efficiency of a dilution series. Not measured in
Origin.
**Size:** S. **Spec:** statistics 50.

## Left

- Several reference genes, plate layouts, melt curves, outlier wells, Western blot densitometry.

## Tests

- `crates/calc/tests/qpcr_reference.rs`: Livak and Pfaffl on three groups and a standard curve against numpy/scipy
  (`tools/qpcr-ref.py`).
- `crates/calc/src/qpcr.rs`: Livak and Pfaffl agree at full efficiency, a perfect doubling series, refusals.
- `crates/core/src/analysis/qpcr.rs`: text groups, the ratio, the plotted columns, the standard curve.
- `crates/cli/tests/analyze.rs`: `qpcr_runs_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `the_statistics_menu_quantifies_qpcr` (the window takes the imported text group,
  a Label column).
