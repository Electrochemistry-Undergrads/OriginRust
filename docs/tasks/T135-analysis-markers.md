# T135: Data > Analysis Markers

**Status:** done 2026-11-18 (Claude Windows "t30-origin", graphing "Analysis markers"), **not measured**
(Origin COM unregistered on the test machine). Left: Lock Position; measure Origin's marker sizes and
which markers each choice shows, and match them.
**Size:** S. **Source:** greyed inventory package B3 (graph Data > Analysis Markers: Hide, Show Partial
Ranges, Show All, Tiny, Small, Medium, Large).
**Spec:** `docs/spec/graphing.md` ("Analysis markers").

## Done when

The seven items change how the data markers of T118 are drawn, the inventory no longer lists them, and
fmt, strict Clippy and the full tests pass.
