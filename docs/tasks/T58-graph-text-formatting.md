# T58: Graph text formatting and the Format toolbar

**Status:** open; recovered and refined 2026-11-15 from the paused cloud planning session.
First renderer slice (`codex/text-decorations`): inline underline/strikethrough now use nested
run styles and shared paths, with resolved-face OpenType metrics, script/rotation/anchor and
export regressions, plus SVG and font-independent stroke PNG goldens. Six Origin 2016 Arial
controls are recorded in graphing 7.1. This leaves the rest of step 1 and steps 2-4 open;
all 15 UI inventory locations still await wiring. This rendering work is separate from the multiple-comparisons plot
(`T64-significance-plot.md`). Formatting, strict workspace all-target Clippy and the combined
3,624-test suite pass (eight existing ignored); the 421-item inventory and eight Python tooling
tests also pass.
Second renderer slice (`codex/text-run-colors`): indexed `\cN(text)` colours compose with
nested font/script styles and underline/strikeout strokes. The 24-colour Classic list and
numeric boundaries are measured from Origin 2016 EMF records (graphing 7.2); independent
glyph/stroke/export fixtures and new colour SVG/PNG goldens cover them. Font switching,
combined scripts, tagged RGB/special colour syntax and the separately claimed editor/toolbar
work remain open. Formatting, strict workspace all-target Clippy and 3,654 combined tests
pass (eight existing ignored), as do the 414-item inventory and eight Python tooling tests.
**Size:** L, four independently mergeable slices. **Branch:** `text-format`.
**Priority:** P0, wave 1; the owner's first priority. Follow HANDOFF's agent/model rules.
**Spec:** [graphing](../spec/graphing.md) 7 and 16; [GUI](../spec/gui-look.md) 1 and 4;
[command map](../spec/ui-command-map.md) toolbar and shortcut tables.
**Inventory:** 15 locations tagged `T58` in [greyed-items](greyed-items.md).
**Ownership:** graph text model/rendering/editor; Format toolbar dispatch. T59 owns its worksheet
adapter. Coordinate shared `graph.rs`, `toolbar.rs`, actions and menus with other claims.

## Goal

Select a title, text label or legend and apply bold, italic, underline, font, size and colour in
one undo step. In its text editor, select characters and apply scripts, Greek or other formatting
with buttons and preview. Screen and every export must agree. Examples: `H\-(2)O`, `\g(m)A`,
`j / mA cm\+(-2)` and a bold panel letter.

## Origin facts

Recorded in gui-look 1/4 and the command map: the font box says "Default: Arial", the selected
legend supplies its size (22 in the measured case), and no selection shows 0. In-place text
editing has Ctrl+B/I/U/G and script/size shortcuts. These observations do **not** establish what
whole-object toolbar clicks do. Measure on Windows: target kinds, style flags versus escape
wrapping, size increments, font/colour palettes, Fill/Line Color targets and Object Properties
layout. Record unmeasured choices explicitly; do not describe them as measured Origin behaviour.

## Where

- `crates/plot/src/markup.rs`: `Run` carries bold/italic/scale/rise. `\u`, `\s` and `\cN` retain
  text without the requested style. `\f:Name` currently changes Symbol/Greek mapping, **not the
  font family**. Keep nesting limits and `plain()` while extending runs.
- `crates/plot/src/text.rs`, `scene.rs` and backends (`svg.rs`, `eps.rs`, `emf.rs`, raster/PDF):
  carry run font/colour and decoration geometry, including rotation.
- `crates/core/src/graph.rs`: `Font` has family, size, bold, italic and colour; add underline
  compatibly with serde defaults. Avoid changing existing defaults or serialized meaning.
- `crates/app/src/toolbar.rs`: `Bar::Format` is inert (`FontBox`, `SizeBox`, `g` buttons). Add a
  shared target/dispatch contract for selected graph text, editor selection and T59 cells.
- `crates/app/src/dialogs/text_object.rs`, `graph_props.rs`: reusable selection-format editor,
  buttons and `sheaf_plot` preview. `graph_view.rs` supplies selected `ObjectKind` and layer.
- `crates/app/src/menus.rs`: X/Y Axis Title entries open that title's editor.
- `crates/io/src/opj`: preserve escapes; write supported font fields or warn and preserve them
  in the companion (ADR 0008). `.sheaf` must preserve every new field.

## Steps

1. Render underline, strikethrough, colour and combined scripts (`\=`); resolve per-run font
   switches. Add parser/nesting regressions and SVG/PNG goldens plus PDF/EPS/EMF checks, including
   rotated text, a legend and tick labels. Update graphing 7.
2. Add editor buttons, matching shortcuts, selection wrapping/toggling and preview; reuse for
   titles and legend. Example regression: select **`-2`** in `cm-2`, apply superscript, obtain
   `cm\+(-2)`. OK applies one history step; Cancel changes nothing. Enable the two title entries.
3. Implement toolbar target dispatch, font/size controls and style/colour actions. Disable by
   selection state when there is no applicable target. Coordinate the dispatch interface with
   T59 before merging. Fill and line colour may apply to selected plots/shapes only after their
   semantics are verified. Remove each of the 13 toolbar locations only when it works.
4. Measure the unresolved Origin behaviour, record it and fix differences. Until then clearly
   label Sheaf choices (for example whole-object font flags or a provisional size increment).

## Done when

Titles/labels/legend can be formatted without typing escapes; nested scripts/Greek preview and
exports agree; undo/redo and `.sheaf` plus `.opj`/companion round trips preserve the result. All
15 inventory locations are enabled or reassigned with reasons, and task-workflow checks pass.

## Don't

Do not rebuild in-place graph-page editing, worksheet styling (T59), or Origin's icons. Keep
existing template goldens stable except where previously ignored formatting now appears. Do not
make toolbar actions change raw worksheet values or silently drop formatting on save.
