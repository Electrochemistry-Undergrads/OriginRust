# T102: Save Format as Theme and Theme Organizer

**Status:** claimed 2026-11-16 (Claude Windows "t30-origin", branch `graph-themes`).
**Size:** M. **Source:** greyed inventory package B11 (graph axis and legend menus: Save Format as
Theme...; Tools > Theme Organizer... (F7)).
**Spec:** `docs/spec/graphing.md` (with section 26, Copy Format and Paste Format).

## Goal

Save a copied format (T72's format clip of a graph, an axis or a legend) as a named graph theme file in the
settings folder, and list, apply, rename and delete themes in a Theme Organizer, as Origin 2016's.

## Origin facts

Measure in Origin 2016: the Save Format as Theme window (name, default, where the file goes), what an axis
or legend theme holds, the Theme Organizer window (tabs, columns, buttons) and what Apply does to the
active graph.

## Done when

The items are enabled and behave as measured, themes survive a restart, one undo step per apply,
fmt/clippy/tests pass, inventory and spec updated.
