# T147: Graph text label Cut and Copy (B2 slice)

**Status:** done 2026-11-19 (Claude Windows "t30-origin"), **not measured** (Origin's COM class is
unregistered). It follows T125: a text label's right-click Cut and Copy use T125's clipboard, and the
page's right-click Paste puts the label's anchor at the clicked point.
**Size:** S. **Source:** greyed inventory package B2 (graph text label: Cut, Copy).
**Spec:** `docs/spec/graphing.md` 17.8.

## Done when

Cut and Copy are live on a label's menu, Paste places the copy, Cut is one undo step, the inventory no
longer lists them, and fmt, strict Clippy and the full tests pass.
