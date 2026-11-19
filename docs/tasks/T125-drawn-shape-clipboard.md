# T125: Drawn-shape Cut, Copy and Paste (B2 slice)

**Status:** done 2026-11-19 (Claude Windows "t30-origin"): right-click Cut and Copy of one drawn object and the page's right-click Paste at the clicked point, with the contract in graphing 17.8 (not measured). Left: Edit menu and keyboard routes, several objects, Ctrl+drag. The documentation audit was completed 2026-11-17.
**Size:** M. **Priority:** wave 4 (B2). **Spec:** graphing 17.8.
**Inventory:** drawn-shape context Cut and Copy (two locations). The shared graph Edit
Cut/Copy/Paste/Clear entries remain B2 until their other object targets are implemented.

## Goal

Copy one existing line, arrow, rectangle or ellipse and paste it at a chosen position
on the active graph layer. Cut also removes the source in one undo step. Preserve all
supported shape styles and endpoint orientation. Publish the precise destination,
size, lifetime and ordering contract before enabling commands.

## Reference and version boundaries

Origin's [drawing-object guide](https://docs.originlab.com/origin-help/create-draw-objects/),
read 2026-11-17, documents selecting an object and using context Copy, Edit Copy or
Ctrl+C. Ordinary Paste uses a location clicked in the destination graph. Its separate
same-location Ctrl+Shift+V operation was introduced in 2025; do not represent a clone
with unchanged frame coordinates as the ordinary 2016 Paste operation. The guide also
dates a stacking change to 2018b: previously pasted objects went behind the last pasted
object, afterwards in front. These are documentation facts, not fresh Native2016
measurements. The existing unavailable COM activation is not retried or repaired here.

The guide supports multiple objects and Ctrl-drag duplication too. Those routes are
outside this single-shape slice. Copy Page, linked graph/OLE, Copy Format, plot copying
and copying a line into worksheet XY data are different operations.

## Where

- `crates/app/src/menus.rs`: `graph_shape_menu`, graph Edit dispatch/availability.
- A new private App clipboard module and minimal `app.rs` dispatch/keyboard hooks.
- `graph_view.rs`: future bounded destination capture/conversion hook, only after the
  active `shape-resize` owner releases or coordinates its geometry/interaction scope.
- Existing Core Shape and graph-edit history APIs; no new model or project schema.
- New focused UI tests/registry and graphing 17.8 documentation.

The audit found no production paste-point API: existing `page_point`, `point_on` and
`page_and_content` helpers are test-only. Shape coordinates are layer-frame fractions;
blindly copying them neither defines a clicked anchor nor preserves physical dimensions
between different frames. This brief does not authorize edits to the resize owner's code.

## Steps

1. Claim this implementation publicly and publish exact files/functions after resolving
   the graph-view ownership boundary. Establish clicked anchor, active-layer selection,
   cross-frame physical size, attachment and pre-2018b insertion behavior with primary
   documentation or Native fixtures; label any remaining Sheaf policy explicitly.
2. Define an owned clipboard snapshot for the four supported kinds. Specify whether
   it survives source edits/deletion, project replacement, restart and other instances;
   do not imply system/foreign-application clipboard interoperability without proof.
3. Add menu/shortcut routing with text-editor and worksheet focus precedence. A stale
   graph/layer/selection/generation must not edit a different object. Copy is read-only;
   Cut and Paste each produce exactly one history step, with correct no-op behavior.
4. Capture the destination independently of resize/drag state and convert the snapshot
   under the published anchor/size policy. Keep rendering and picking order consistent.
5. Test all four kinds, reversed endpoints, root/inset frames, unequal page/layer sizes,
   repeated paste, target/source deletion, project switches, focus precedence and full
   Undo/Redo. Compare complete graph snapshots and native project companions. Inspect
   real owned desktop context/menu/keyboard results and saved/exported geometry.
6. Run fmt, strict workspace/all-target Clippy, full workspace tests and inventory checks;
   update the index, release only this claim, merge and push after all actors terminate.

## Done when

The four shapes copy/cut/paste at the documented destination with stable size, style,
history and stale-target behavior; enabled menu entries and shortcuts agree. Until
then, this brief remains open and the two inventory entries remain disabled.

## Do not

Do not implement multi-selection, grouping, alignment, other object kinds, Ctrl-drag,
resize handles or a unified stacking model. Do not change the Core shape/project schema,
renderer, IO, T72 format clipboard/profile, worksheet clipboard or another active claim.
Do not enable Cut alone when the resulting snapshot has no usable Paste route.
