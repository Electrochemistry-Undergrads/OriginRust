# T95: Find in Project

**Status:** done 2026-11-16 (Claude Windows "t30-origin", workbook-model.md 13j): Origin's Find window, defaults and hits measured; Sheaf searches graph, book, sheet and column names and labels, data cells and graph text with the same switches, and goes to a hit. Left: folders and notes, Within, Origin's double-click.
**Size:** M. **Source:** greyed inventory package B7 (Edit > Find in Project... on a workbook, Find in
Project on a graph; F3).
**Spec:** `docs/spec/gui-look.md` section 5 (Find) or `workbook-model.md`.

## Goal

Find in Project (F3) searches the whole project for a text, as Origin 2016's: the window, what it searches
(window names and long names, column names and labels, cells, graph text) and how a hit is shown and
opened.

## Origin facts

Measure in Origin 2016: the window's fields and defaults, what kinds of objects it searches, the result
list's columns, and what activating a result does.

## Done when

Both items and F3 are enabled and behave as measured (fixtures), fmt/clippy/tests pass, inventory and
spec updated.
