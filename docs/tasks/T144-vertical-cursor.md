# T144: Gadgets > Vertical Cursor

**Status:** done 2026-11-18 (Claude Windows "t30-origin", graphing "Vertical Cursor gadget"), **not
measured** (Origin COM unregistered on the test machine). Taken from the stale T101 slice; Differentiate and
Interpolate stay with T101. Left: Origin's settings, its output, cursors across layers; measure and match.
**Size:** S. **Source:** greyed inventory package B9 (Gadgets > Vertical Cursor...).
**Spec:** `docs/spec/graphing.md` ("Vertical Cursor gadget").

## Done when

The menu item opens the cursor, it reads every visible plot of the layer at its X live, the inventory no
longer lists the item, and fmt, strict Clippy and the full tests pass.
