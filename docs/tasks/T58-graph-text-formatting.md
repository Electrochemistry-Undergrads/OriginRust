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

Paired-script editor (`codex/t58-paired-script-editor`): Paired scripts in the shared label/
title/legend editor provides lower/upper text fields and inserts the existing measured paired
column syntax. Raw selection seeds the lower branch; either branch can be empty. Literal
commas, Unicode, styles/fonts and outside text survive, while unsafe fragments and legend
sample/row-spanning selections remain unchanged. The action stays a draft, restores typing
focus after the column, and reuses preview/Apply/OK/Cancel/history/persistence. Six UI and four
helper regressions cover actual shared-column glyphs, scope, syntax, stale projects and saves.
Graphing 7.10 separates the existing native renderer measurements from this Sheaf UI policy.
Native whole-object Bold/Italic differences, special escape aliases, other palette lists/storage
and further properties-dialog measurements remain open.
Fmt, strict workspace/all-target Clippy, all 4,262 workspace tests (eight ignored), all 120
related formatting UI/helper/preview tests, inventory 304 and eight Python tooling tests pass
with unchanged source/goldens on integrated T107 source. Fresh desktop script fields
and shared-column draft preview from the tested executable were inspected.

Custom RGB graph-font toolbar (`codex/t58-graph-font-rgb`): Font Color > Custom RGB... offers
0 to 255 channel fields, a live swatch and Apply/Cancel; valid Enter also accepts. It changes
only the captured selected label/title/legend base-font colour through existing requests.
Invalid/dismissed/stale/no-op choices retain the existing safety/history rules. Worksheet
custom controls, graph Fill/Line and inline editor colours are separate. Seven UI regressions
cover all four targets/rendered colours, boundaries, invalid text, reseeding/no-op, Enter/Cancel,
stale requests, inline-colour precedence, styles and native/companion persistence. Fresh owned
Origin RGB/black/white property and pixel controls are recorded in graphing 7.4; the malformed
first pilot is excluded and the numeric submenu is explicit Sheaf layout. Other colour lists,
Auto, palette storage, special escape aliases and native dialog measurements remain open.
Fmt, strict workspace/all-target Clippy, all 4,274 workspace tests (eight ignored), all 127
related formatting UI/helper/preview tests, inventory 299 and eight Python tooling tests pass
with unchanged source/goldens on integrated T108 source. Fresh desktop seeded RGB,
invalid-input and closed-menu captures from the tested executable were inspected.

Empty font-name follow-up (`codex/t58-font-aliases`, 2026-11-16): an empty `\f:(...)`
remains literal, matching the accepted Origin 2016 N6 record and 21 fresh controls with a verified Courier parent.
Three regressions
cover exact text, inherited family/style/width and shared vector/raster output. Valid names,
whitespace-name behaviour and current unknown-family fallback are preserved. Modern special
font defaults are documented as a separate 2022b feature; no unmeasured alias mapping is added.
Graphing 7.6 records the native evidence and limits. Fmt, strict workspace/all-target Clippy
and all 4,296 workspace tests pass (eight ignored), including all 11 font regressions.
Inventory 293 and eight Python tooling tests pass with frozen source/goldens and four test threads.

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

Classic object Fill/Line palette follow-up (`codex/t58-object-classic-colors`): the existing
rectangle/circle/legend-fill and drawn-object/straight-line targets offer all 24 Classic colours,
retaining the basic eight and adding More Classic colors. Fresh Origin 2016 rectangle/line
getters and 24 EMF pen/brush controls establish the RGB values (graphing 7.4). Menu grouping
and legend mapping remain explicit Sheaf choices. Existing captured requests, selection,
history and saves are reused; no model, renderer, IO or public basic-palette change.
Custom object RGB, Auto, other lists and palette storage remain open.
Fmt, strict workspace/all-target Clippy and all 4,307 tests pass on the integrated worksheet
label-size source (eight ignored), including
five new object regressions and all 23 related palette/object tests; inventory293 and eight
Python tests pass. Source/goldens stayed frozen; current submenu and colour-effect desktop
captures were inspected and all own app/reference/helper actors closed.

Custom object RGB follow-up (`codex/t58-object-rgb`): the existing supported Fill/Line targets
offer Custom RGB... with validated channels, preview, Apply/Cancel and Enter. Fresh Origin 2016
rectangle fill/outline and straight-line controls establish five RGB values including black,
white and mixed channel boundaries (graphing 7.4). Numeric menu layout, white draft seed for None
and the existing legend mapping remain Sheaf choices. Captured request, selection, history,
stale/no-op and native/companion rules are reused; all 24 Classic presets and None remain.
No model/backend/font/worksheet picker changes. Auto, other lists and palette storage remain open.
Fmt, strict workspace/all-target Clippy and all 4,360 workspace tests pass (eight ignored),
including all seven new and 37 related regressions. Inventory283 and eight Python tests pass;
source/goldens stayed frozen. Current seeded/invalid/applied desktop captures and rendered RGB
pixels were inspected, with every own app/reference/helper actor closed. Earlier setter-only
automation pilots are excluded from appearance evidence.

Installed font abbreviations (`codex/t58-inline-font-aliases`, 2026-11-18): graph text
expands a case-insensitive family prefix when it identifies one installed family. Exact
family names take priority and ambiguous prefixes use the existing fallback. Layout and
SVG use the same resolved family, including for nested Symbol and paired-script runs.
All 666 plot-crate tests passed. Graphing 7.6 records Sheaf's matching policy and the
remaining Native 2016 and configurable-default gaps. Formatting, strict workspace Clippy, Nextest (4,474 passed and
8 skipped), doc tests, inventory and eight Python tooling tests passed.

Overbar (`codex/t58-text-overbar`, 2026-11-18): graph labels, titles and legends can use
`\ab(text)` to draw a bar over the enclosed text. The renderer keeps one continuous stroke
through nested styles and paired scripts, with the same glyph placement and advance. SVG
and other backends receive that stroke from the shared scene. Graphing 7.11 records the
documented contract and Sheaf's placement choices. Native geometry, dot and tilde remain
open. Formatting, strict workspace Clippy, Nextest (4,485 passed and
8 skipped), doc tests, inventory and eight Python tooling tests passed.

Prime notation follow-up (codex/t58-escape-alias-audit, 2026-11-16): exact closed
prime/double-prime escapes now use Unicode U+2032/U+2033 with existing run styles,
script columns and rendering. Graphing 7.11 records the official documented
contract (FAQ159 minimum Origin2015SR0), strict syntax and explicit unmeasured
Native glyph geometry/combining offsets. The alias audit retains 7.6's deferral
of post-2016 special defaults; no lookup or font configuration is invented.
Six regressions compare parser, style restoration, literal/malformed/depth rules,
placement/rotation, exports and real graph contexts with Unicode controls.
Fmt, strict workspace/all-target Clippy and all 4,369 tests pass (eight ignored), including all six new regressions. Inventory283 and eight Python tests pass with all 912 source/fixture/asset hashes frozen. Fresh actual/Unicode/verbatim desktop captures were inspected; actual and Unicode PNG exports match all pixels, while verbatim differs. Raw notation and graphs survive .sheaf and OPJ companion saves. All own build/app/helper actors are terminal; remaining T58 controls and other claims stay open.

Reusable Custom RGB wells (codex/t58-color-list-storage, 2026-11-17): graph font,
fill and line pickers share a profile pool. Add to Custom saves a valid draft
without editing the graph; Cancel leaves the preference available. Choosing a
well reuses captured requests and existing history; context removal changes only
the pool. Graphing7.12 separates FAQ130's documented 2016+ custom wells from later
Color Manager/system-list features and records all storage/UI policies as Sheaf
choices. Applied colours keep existing saves/exports; unused wells are not a
project palette. Worksheet/editor reuse, named lists, OTH/PAL, Auto and broader
native measurements remain open. No fresh Native measurement is claimed.
Fmt, strict workspace/all-target Clippy, all 4,391 tests (eight ignored),
inventory281 and eight Python tests pass with all917 source/fixture/asset files
frozen. Eleven new regressions cover pool/storage and actual picker wiring.
Fresh private-profile desktop controls exercise Add without Apply, Cancel,
restart, shared font/fill/line reuse and Undo. Saved complete graphs match
independent expectations, restored export pixels match exactly and choosing/Undo
do not rewrite preferences. All three owned apps and guards close successfully;
initial removal-menu pilots are excluded and the passed source remains fixed.

Shared text-editor Custom wells (codex/t58-editor-custom-wells, 2026-11-17):
the existing private inline-colour picker reuses the published profile pool.
Add to Custom leaves the text draft/document unchanged; a saved choice uses the
existing selection/caret RGB command and numeric-field seed. Existing Apply/OK,
Cancel, automatic-text, legend-reference, base-font and generation rules remain.
Only Picker::show and new UI regressions change; pool storage, rewrite/apply
engines, toolbar and worksheet controls are unchanged. Graphing7.12 identifies
the compact inline integration as Sheaf policy under the documentation contract,
without a new Native2016 measurement. Worksheet reuse, named/project palettes,
OTH/PAL, Auto and other remaining measurements stay open.
Fmt, strict workspace/all-target Clippy, all4,440 tests (eight ignored),
inventory275 and eight Python tests pass on the integratedT120-T124 tree, with
all924 source/fixture/asset hashes frozen. Five new UI regressions exercise
preferences/drafts/history, selection/caret/focus, titles/legend rendering,
automatic-text removal, generation rejection and native/companion round trips.
Fresh owned accessibility controls Add in the editor, restart and select actual
text, choose the saved well, Apply and Undo. Saved whole graphs match independent
expectations, restored export pixels match exactly, and preferences are not
rewritten by choose/Apply/Undo. Both apps and guards close0. Initial test setup
and menu-name pilots are excluded; six production lines stay fixed through validation.

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
