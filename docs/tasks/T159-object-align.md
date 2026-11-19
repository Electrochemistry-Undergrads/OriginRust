# T159: Shift+click multi-selection, Object Edit Align and Make Same Width/Height (B2 slice)

**Status:** done 2026-11-19 (Claude Windows "obj-align"). Built from Origin's Object Edit Toolbar
help page, **not measured**.
**Size:** M. **Source:** greyed inventory package B2 (Object Edit toolbar).
**Spec:** `docs/spec/graphing.md` ("Aligning drawn objects and text").

## Goal

The eight Object Edit toolbar buttons Align Left, Align Right, Align Top, Align Bottom, Align
Horizontal Center, Align Vertical Center, Make Same Width and Make Same Height work on several selected
drawn objects and text labels. As Origin documents, the last-selected object is the reference.

## Done

- Shift+click adds a drawn object or text label to the graph selection, or takes it out
  (`GraphUi::also`, `GraphUi::multi_selection`). Every selected object gets handles.
- `crates/app/src/obj_align.rs` plans the moves and resizes, and the eight buttons send
  `Cmd::ArrangeSelected`. Each press is one undo step.
- Index safety: when a layer's drawn-object list changes, its shapes leave the selection.
- Tests: five unit tests in `obj_align.rs` and three UI tests in `app/ui_tests/obj_align.rs`.

## Left

- Group, Ungroup and dragging the whole selection: done in T162.
- Origin's Distribute and Swap buttons. Sheaf's Object Edit bar does not show them.
- Measuring Origin.
