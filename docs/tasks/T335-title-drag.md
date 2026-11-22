# T335: Drag an axis title

**Status:** done 2026-11-21 (Claude Windows "claude:b284"). The parity row "Axis titles" said dragging a title with
the mouse was not built. With the pointer tool, pressing on an X or Y axis title and dragging it moves it: the
selection handles follow during the drag and, when the button is released, the move (page points) is added to
`Axis::title_offset_pt` (T291) in one undo step "Move axis title". Origin keeps a moved title where it was put
(graphing, T291 paragraph); the Title tab shows the new offset, and the .opj writer already writes it.
**Size:** S. **Spec:** graphing (T291 paragraph "Rotation and position").

## Left

- Titles of the top and right axes (their own title objects in Origin) and snapping.

## Tests

- `crates/app/src/app/ui_tests.rs`: `an_axis_title_is_dragged_to_a_new_place_in_one_undo_step` (X and Y title, the
  title can be picked where it was dropped, Undo).
