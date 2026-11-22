# T345: Stale parity row "Generic analysis dialog frame" corrected

**Status:** done 2026-11-21 (Claude Windows "claude:3"), docs only. The row said Recalculate (Auto/Manual)
links only Smooth, Integrate and Find Peaks. `LinkTool::from_tool` in `crates/core/src/analysis_link.rs`
links 15 tools: Smooth, Integrate, Find Peaks, Average Multiple Curves, Intersect, Polygon Area, Reduce
Duplicate X, Reduce to Evenly Spaced X, Subtract Straight Line, Translate, Median Filter, IIR and FIR
Filter, Fourier Self-Deconvolution and Simulate Curve.
**Size:** S. **Spec:** `docs/spec/feature-parity.md`.
