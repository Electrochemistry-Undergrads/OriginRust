# T137: Text label Bring to the front / Push to the back

**Status:** done 2026-11-18 (Claude Windows "t30-origin", graphing "Text label order"), **not measured**
(Origin COM unregistered on the test machine). Left: order across object kinds (labels, drawn shapes,
legend, axis titles) and label Cut/Copy (with T125).
**Size:** S. **Source:** greyed inventory package B2 (graph text label right-click: Bring to the front,
Push to the back).
**Spec:** `docs/spec/graphing.md` ("Text label order").

## Done when

Both items reorder the label, the label stays selected, one undo step each, the inventory no longer lists
them, and fmt, strict Clippy and the full tests pass.
