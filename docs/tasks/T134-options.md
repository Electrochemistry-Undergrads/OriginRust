# T134: Tools > Options...

**Status:** done 2026-11-18 (Claude Windows "t30-origin", gui-look "Options"), **not measured** (Origin COM
unregistered on the test machine). First slice: Open/Close (autosave) and File Locations. Left: Origin's
other tabs (Numeric Format, Text Fonts, Graph, Page, Excel, Axis, Miscellaneous, System Path) as Sheaf grows
settings for them; measure the tab layout and match it.
**Size:** S. **Source:** greyed inventory package B13 (Tools > Options...).
**Spec:** `docs/spec/gui-look.md` ("Options").

## Done when

Tools > Options... opens the window, OK applies and keeps the autosave settings, the inventory no longer
lists the item, and fmt, strict Clippy and the full tests pass.
