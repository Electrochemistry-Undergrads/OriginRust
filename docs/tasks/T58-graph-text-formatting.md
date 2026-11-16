# T58: Graph text formatting and the Format toolbar

**Status:** open; recovered and refined 2026-11-15 from the paused cloud planning session.
First renderer slice (`codex/text-decorations`): inline underline/strikethrough now use nested
run styles and shared paths, with resolved-face OpenType metrics, script/rotation/anchor and
export regressions, plus SVG and font-independent stroke PNG goldens. Six Origin 2016 Arial
controls are recorded in graphing 7.1. This leaves the rest of step 1 and steps 2-4 open;
all 15 UI inventory locations still awaited wiring at that slice. This rendering work is separate from the multiple-comparisons plot
(`T64-significance-plot.md`). Formatting, strict workspace all-target Clippy and the combined
3,624-test suite pass (eight existing ignored); the 421-item inventory and eight Python tooling
tests also pass.
Second renderer slice (`codex/text-run-colors`): indexed `\cN(text)` colours compose with
nested font/script styles and underline/strikeout strokes. The 24-colour Classic list and
numeric boundaries are measured from Origin 2016 EMF records (graphing 7.2); independent
glyph/stroke/export fixtures and new colour SVG/PNG goldens cover them. Font switching,
combined scripts, other special colour syntax and the separately claimed editor/toolbar
work remain open. Formatting, strict workspace all-target Clippy and 3,654 combined tests
pass (eight existing ignored), as do the 414-item inventory and eight Python tooling tests.
Third renderer slice (`codex/text-run-rgb`): RGB-tagged numeric colour escapes use the
measured one-based text-code convention, including channel borrows, black/white boundaries
and nested restoration. Graphing 7.3 records ten native RGB and twelve boundary controls;
seven regressions and new SVG/stroke PNG goldens cover glyphs, strokes and vector/raster
exports. Font-family switches, combined scripts and other special colour tags remain open;
editor/toolbar work stays with its owners.
Formatting, strict workspace all-target Clippy and 3,677 combined tests pass (eight existing
ignored); inventory 407 and all eight Python tooling tests pass.
Editor slice (`codex/t58-text-editor`): free-label Text window selection/caret buttons and
live export-renderer preview are implemented; Unicode selections, nested/toggled formats,
Normal, empty drafts, Cancel/Apply/history and saved-project preservation are covered.
Origin's documented Text-tab workflow and explicitly unmeasured choices are recorded in
graphing 7.5. Title/legend editor reuse, the two title menu entries, inline font/colour
selection and combined-script insertion remain open; the separate Format/renderer claims
retain their scopes. This does not complete all of step 2 or the whole brief.
Formatting, strict workspace all-target Clippy and 3,725 combined tests pass (eight
existing ignored); inventory validation, eight Python tooling tests and the inspected native
app capture also pass. All twelve owned UI/helper/preview tests pass in the complete suite.
Whole-object toolbar slice (`codex/t58-format-toolbar`): shared target/snapshot/dispatch;
installed family, size, Bold/Italic, measured 1.2 size scaling and eight basic font colours for
selected labels/legends/titles. Seven toolbar locations enabled; the six other toolbar controls
and two title-menu entries remain. Graphing 7.4 records measured controls and the native legend
Bold/Italic difference; Sheaf's whole-font flags are explicit choices. No renderer/editor,
worksheet adapter or new model/file-format fields in this slice. UI regressions cover selection,
Undo/Redo, validation, stale actions/palettes, SVG and native/companion saves. Remaining steps are
still open; the renderer and text-editor results above are preserved.
Formatting, strict workspace all-target Clippy and the combined 3,713-test suite pass
(8 existing ignored), with all seven toolbar UI regressions, source-hash checks,
the 397-item inventory and eight Python tooling tests; a desktop capture was inspected.
Title/legend editor reuse (`codex/t58-title-editors`): the existing X/Y axis title entries and
title/legend Edit Text routes use the shared selection editor and actual-object preview.
Automatic text, raw metadata/legend notation, base fonts, visibility, draft cancellation and
history are preserved; read-only title/region facades reuse existing rendering rules (7.7).
The two title-menu entries are enabled. Inline family/colour pickers, combined-script insertion
and remaining renderer gaps still need their separate slices. Final combined validation passed formatting, strict workspace all-target Clippy and
3,957 tests (eight existing ignored), including all 21 related editor tests. Inventory
and eight Python tooling tests pass; X-title, Y-title and legend desktop captures were
inspected, and their four editor/test source files remain unchanged on the combined tree.
Selected-text colour slice (`codex/t58-inline-colors`): the shared text editor offers eight
named Classic colours through Text color, with selection/caret insertion, idempotent recolouring
and style/literal/reference preservation (graphing 7.8). The renderer and base font are unchanged.
Automatic text stays automatic when the menu is dismissed; changed colour remains a draft until
Apply/OK. Existing history and native/Origin-companion persistence are used. Broader/custom
colour controls, inline family selection and combined-script insertion remain separate work.
Formatting, strict workspace all-target Clippy and 3,981 combined tests pass
(eight existing ignored), including all 29 related editor UI/helper/preview tests.
Inventory validation, eight Python tooling tests and three inspected desktop captures pass;
the editor/test source hashes remain unchanged through validation and capture.

Fourth renderer slice (`codex/text-run-fonts`): full-name inline font-family switches now
reach run measurement, glyphs and decoration metrics, including nested restoration,
styles/scripts/Greek and rotation. Graphing 7.6 records nineteen native family/name controls
and their limits; eight regressions plus SVG/stroke PNG goldens cover the shared export
path. Combined scripts and special colour/name aliases remain open. Editor and shared
Format toolbar work stay with their owners.
Formatting, strict workspace all-target Clippy and 4,037 combined tests pass (eight existing ignored).
The 340-item inventory and eight Python tooling tests pass; all workspace tests ran
with four test threads, retaining every test and assertion.

Fifth renderer slice (`codex/text-script-pairs`): `\=(sub,sup)` uses measured shared
columns, 60% script size and 49% up / 35% down baselines, with maximum branch advance,
literal commas, empty branches, nesting and inherited style restoration. Graphing 7.10
records twenty native controls and four size checks. Independent placement/parser,
font/decoration/anchor/rotation/export regressions and SVG/stroke PNG goldens cover it.
Combined-script **editor insertion** remains separate; other unsupported escape codes,
font aliases and the separately owned custom-colour/editor work remain open.
Formatting, strict workspace all-target Clippy and 4,047 combined tests pass (eight existing
ignored), including all ten paired-script regressions and the real legend/tick/rotated-title
case. The 340-item inventory and eight Python tooling tests pass. All workspace tests ran
with four test threads, retaining every test and assertion; source/golden hashes stayed fixed.

Custom RGB editor slice (`codex/t58-custom-text-colors`): Text color > Custom RGB... adds
validated Red/Green/Blue fields and an explicit draft choice using the published RGB renderer
encoding (graphing 7.9). Menus stay open while editing; invalid values cannot be applied.
The existing selection/caret, style/literal/reference preservation, history and persistence
paths are reused, including the newly published font-family and paired-script renderer rules.
No renderer, base-font, project-model or worksheet formatting behaviour changes. Full named
Classic palette expansion and other editor insertion controls remain separate work.
Formatting, strict workspace all-target Clippy and 4,062 combined tests pass
(eight existing ignored), including all 37 related editor UI/helper/preview tests.
Inventory, eight Python tooling tests and four inspected desktop captures pass, including
the real RGB popup with verified channel values; editor/test source hashes are unchanged.

Full named Classic editor palette (`codex/t58-classic-editor-colors`): the shared
label/title/legend Text color menu keeps its eight basic choices and adds the other 16 in
More Classic colors, using all 24 measured index/RGB values from graphing 7.2/7.8. The compact
menu grouping is Sheaf policy. Existing draft, typing focus, automatic text, Apply/OK/Cancel,
history, style/reference/base-font and project/companion rules are reused; a named choice
also seeds the custom RGB picker. Six UI regressions cover every index and rendered RGB,
Unicode/script selection, idempotence, automatic-text dismissal, titles/legend persistence
and RGB compatibility. Fmt, strict workspace/all-target Clippy, all 4,221 workspace tests
(eight existing ignored), all 95 related formatting UI/helper/preview tests, inventory 309,
eight Python tooling tests, unchanged source/goldens and inspected desktop controls pass on
the combined T61/T103 source. Other colour lists/palette storage and remaining editor
insertion controls stay open.

Full Classic graph-toolbar palette (`codex/t58-classic-toolbar-colors`): selected labels,
legends and X/Y titles now offer all 24 measured Classic font colours through the existing
captured font requests. More Classic colors reuses the worksheet presets; worksheet custom RGB
and graph object Fill/Line retain their own controls. Five graph regressions exercise every
colour on all four text targets and rendered output, entire-project scope and one-step Undo,
no-op/stale requests, styles/SVG and native/companion roundtrips. Graphing 7.4 records the accepted
Origin palette pixels and the explicitly excluded fresh pilot. The shared text editor is unchanged.
Remaining editor family/script insertion, other native colour lists and palette storage stay open.
Fmt, strict workspace/all-target Clippy, all 4,238 workspace tests (eight ignored), all 100
related formatting UI/helper/preview tests, inventory 306 and eight Python tooling tests pass
on combined T104/T105/T106/T60 source, with unchanged source/goldens. Fresh desktop
submenu and Royal captures from the tested executable were inspected.

Inline font-family editor (`codex/t58-inline-font-family`): Text font searches installed
families and applies the measured full-name escape to a selection or empty caret group in the
shared label/title/legend editor. Family replacement is idempotent and preserves other styles,
base fonts, literal blocks and legend references. Search/dismissal keeps automatic text;
changes stay drafts through the existing Apply/OK/Cancel/history paths. Six UI and four helper
regressions cover Unicode/scripts/focus, rendered families, styles, Symbol mapping, unsafe
fragments, project replacement and native/companion persistence. Graphing 7.5/7.6 records the
existing accepted Origin controls and the explicitly separate Sheaf picker policy. Remaining
combined-script insertion, special escape aliases and other palette lists/storage stay open.
Fmt, strict workspace/all-target Clippy, all 4,248 workspace tests (eight ignored), all 110
related formatting UI/helper/preview tests, inventory 306 and eight Python tooling tests pass
with unchanged source/goldens on combined T104/T105/T106/T60 source. Fresh desktop font-search
and Times draft-preview captures from the tested executable were inspected.

**Size:** L, four independently mergeable slices. **Branch:** `text-format`.
**Priority:** P0, wave 1; the owner's first priority. Follow HANDOFF's agent/model rules.
**Spec:** [graphing](../spec/graphing.md) 7 and 16; [GUI](../spec/gui-look.md) 1 and 4;
[command map](../spec/ui-command-map.md) toolbar and shortcut tables.
**Inventory:** 0 locations tagged `T58` in [greyed-items](greyed-items.md).
**Ownership:** graph text model/rendering/editor; Format toolbar dispatch. T59 owns its worksheet
adapter. Coordinate shared `graph.rs`, `toolbar.rs`, actions and menus with other claims.

Format-bar remainder slice (format-bar-rest): Underline, Superscript, Subscript, Greek, Fill Color and Line Color
are enabled for their graph targets (new `object_format` module, graphing 7.4): whole-text markup on labels and
explicit axis titles, Fill on rectangles/circles and the legend background, Line on drawn objects and straight lines.
Whole-text wrapping, target limits and palettes are unmeasured Sheaf choices; Origin 2016 measurement, the title
menu items and the editor remain open.

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

- `crates/plot/src/markup.rs`: `Run` carries bold/italic/underline/strikeout/colour/family/scale/rise.
  `\u`, `\s`, indexed/RGB `\cN` and full-name `\f:Name` render through shared text placement.
  Keep nesting limits, Greek mapping and `plain()` while extending runs.
- `crates/plot/src/text.rs`, `scene.rs` and backends (`svg.rs`, `eps.rs`, `emf.rs`, raster/PDF):
  carry run font/colour and decoration geometry, including rotation. The private paired-script layout tree shares one column per `\=(sub,sup)`;
  `text_script_pair_tests.rs` covers actual legend/tick/title layout as well as run geometry.
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
