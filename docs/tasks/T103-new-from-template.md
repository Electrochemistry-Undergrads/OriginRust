# T103: File > New > From Template

**Status:** done 2026-11-16 (Claude Windows "t30-origin", graphing.md 24): Origin's submenu (templates by name, More...) measured; Sheaf lists the user graph templates and makes an empty graph from one. Left: workbook templates (Sheaf has none).
**Size:** S. **Source:** greyed inventory package B11 (File > New > From Template).
**Spec:** `docs/spec/graphing.md` section 24 (user templates).

## Goal

Make a new window from a saved template without data, as Origin 2016's File > New > From Template:
Sheaf's user graph templates (T56) give an empty graph in the template's look.

## Origin facts

Measure in Origin 2016: the window From Template opens (file dialog, folder, file types) and what the new
window is (name, content).

## Done when

The item is enabled and behaves as measured, one undo step, fmt/clippy/tests pass, inventory and spec
updated.
