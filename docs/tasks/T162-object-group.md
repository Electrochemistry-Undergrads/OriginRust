# T162: Object Edit Group / Ungroup, dragging a selection (B2 slice, T159 follow-up)

**Status:** done 2026-11-19 (Claude Windows "obj-align"). Built from Origin's Object Edit Toolbar
help page ("Group the selected objects", "Ungroup the selected objects"), **not measured**.
**Size:** M. **Source:** greyed inventory package B2 (Object Edit toolbar).
**Spec:** `docs/spec/graphing.md` ("Aligning drawn objects and text").

## Done

- `Shape::group` and `TextLabel::group` are serde-default group numbers, saved only when set.
- The Group and Ungroup buttons send `Cmd::GroupSelected`. A click on a grouped object selects the
  whole group, and Shift+click adds or removes the whole group.
- A drag on one of several selected objects, or on a grouped object, moves them all as one undo step
  (`Action::MoveObjects`).
- A pasted copy of a member is not in the group.
- Tests: one unit test in `obj_align.rs` and two UI tests in `app/ui_tests/obj_align.rs`.

## Left

- Nested groups, which Origin may keep. Sheaf has one flat group number per object.
- Groups across layers.
- Writing groups to `.opj`.
- Measuring Origin.
