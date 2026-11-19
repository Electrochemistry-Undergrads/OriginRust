# T146: Gadgets > Cluster

**Status:** done 2026-11-19 (Claude Windows "t30-origin", graphing "Cluster gadget"), **not measured**
(Origin COM unregistered on the test machine). Left: Origin's settings dialog, masking and copying the
points inside, per-cluster output sheets; measure and match.
**Size:** S. **Source:** greyed inventory package B9 (Gadgets > Cluster...).
**Spec:** `docs/spec/graphing.md` ("Cluster gadget").

## Done when

The menu item opens a rectangle on the active plot, the points inside are marked and counted live as it
is dragged or resized, the inventory no longer lists it, and fmt, strict Clippy and the full tests pass.
