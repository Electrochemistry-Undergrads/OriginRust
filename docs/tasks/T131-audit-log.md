# T131: Tools > Protection > Audit Log

**Status:** done 2026-11-18 (Claude Windows "t30-origin", gui-look "Audit Log"), **not measured** (Origin COM
unregistered on the test machine). Left: Protect Project...; measure Origin's log columns, user names and
switch, and match them.
**Size:** S. **Source:** greyed inventory package B13 (Tools > Protection > Audit Log).
**Spec:** `docs/spec/gui-look.md` ("Audit Log").

## Goal

A per-project log of the changes made to the project, turned on in the Audit Log window, saved with the
project and not taken back by Undo.

## Done when

The menu item opens the window, the log records edits, undo and redo, the inventory no longer lists the
item, and fmt, strict Clippy and the full tests pass.
