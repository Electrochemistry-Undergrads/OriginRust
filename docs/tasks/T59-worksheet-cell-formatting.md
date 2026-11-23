# T59: Worksheet cell formatting

**Status:** steps 1, 2 and 4 done 2026-11-15 (`cell-format`). Step 3 first toolbar slice adds selected cells/rows/columns/label targets to T58's shared dispatch: installed font, size, Bold/Italic, basic text colours, mixed-state summaries, stale-request protection and one-step Undo. Grid point sizes and derived row geometry are corrected, including print/image geometry. Basic Origin 2016 toolbar/font/row-growth measurements are in GUI 3; `codex/t59-format-toolbar` passes fmt, strict workspace/all-target Clippy, all 3,934 workspace tests (8 ignored), 17 toolbar regressions and the inventory check; the desktop rendering was inspected. Merged to main with the T74/Line Series integration; remaining work is listed below. Slice 2 (`codex/t59-worksheet-controls`) adds worksheet Underline, basic fill/None and contextual Ctrl+B/I/U after Origin measurements; fmt, strict workspace/all-target Clippy, all 3,971 workspace tests (8 ignored), 26 related toolbar regressions, the inventory check and an inspected isolated desktop capture pass. Slice 3 (`codex/t59-worksheet-dialog`; fmt, strict workspace/all-target Clippy, 4,051 workspace tests with 8 ignored, all 40 related UI regressions, the 338-item inventory and inspected desktop pass) measures the separate Worksheet Properties/category defaults and enables the measured Set Data Style > More... path to the existing Format Cells dialog. The fictitious Default placeholder is removed; Rich Text/Wrap Text/Float remain disabled under B5. Slice 4 (`codex/t59-custom-colors`) adds worksheet-only Custom... text/fill RGB choices after native chooser measurements (GUI 3), with explicit OK/Cancel, stale request and effective no-op guards, compact data/label targets and existing project/companion persistence. Fmt, strict workspace/all-target Clippy, all 4,146 workspace tests (8 ignored), all 47 related UI regressions, the 330-item inventory, eight Python tooling tests, unchanged source/goldens and inspected compact desktop choosers pass. Slice 5 (`codex/t59-classic-colors`) adds the other 16 native Classic presets for worksheet text/fill in a compact submenu, completing all 24 RGB choices (both native palettes measured; GUI 3). Five UI regressions cover every RGB pair, Undo/no-ops, menu closure, stale requests, compact columns/labels, persistence and graph/RGB compatibility. Fmt, strict workspace/all-target Clippy, all 4,157 workspace tests (8 ignored), all 52 related UI regressions, inventory 327, eight Python tooling tests, unchanged source/goldens and inspected desktop submenus pass on the combined T97 tree. Remaining: bulk/Standard/header font defaults, category colors/text controls, vertical alignment, persistent/sticky row heights, named colour lists/project palettes and remaining dialog/precedence measurements. Printing still omits empty label rows, including styled ones, under its existing T27 extent policy.
Excel export slice (`codex/t59-xlsx-formats`) combines existing resolved worksheet fonts,
point sizes, style flags, text/fill RGB and horizontal alignment with column number/date/time
formats. Label offsets, overlapping/reset ranges, blank cells and mixed data retain their
semantics; conditional colours export as a reported snapshot. Omitted label/future-cell styles
and partial transparency have explicit limits (workbook-model 13b).
Formatting, strict workspace all-target Clippy and 4,169 combined tests pass
(8 existing ignored), including all six independent output regressions and all 50 existing
Excel tests. Inventory, eight Python tooling tests, unchanged source/golden hashes and an
independent openpyxl file review pass. No UI, renderer or project-model changes. No project-model, import,
GUI, toolbar or print changes. Six independent output regressions and all 50 existing Excel
tests pass. Combined workspace validation follows before publication.

Vertical-alignment measurement slice (`codex/t59-cell-vertical-align`, 2026-11-16):
selected A1 and Worksheet Properties Data Format/Size controls were inspected in Origin
2016. No vertical text-alignment choice was found in these routes; another supported
route and its mutation semantics remain unestablished (GUI 3). No model/UI field was
added from that negative evidence. Successful captures and identity-guard cleanup are
hashed locally; failed/unselected/early-paint attempts are excluded.
Formatting, strict workspace all-target Clippy and 4,205 combined tests pass
(8 existing ignored); inventory, eight Python tooling tests and frozen source/golden
hashes pass. Six Native actors and their owned instances/guards/helpers are terminal;
accepted screenshots/control lists were inspected and hashed. This is a measurement
slice; supported per-range vertical alignment remains unestablished.

Vertical-alignment documentation audit (codex/t59-cell-vertical-alignment,
2026-11-16): current worksheet Format-tab and wproperties documentation supplies
no supported per-range route. GUI 3 separates the 2023b Data Info/tooltip table
alignment feature from worksheet cells; those controls do not establish worksheet
mutation behavior. The earlier accepted 2016 probes remain bounded negative
evidence. No new Native probe, Rust/model/UI/export change or parity claim is made.
Per-range implementation stays deferred until its worksheet route is established.
The exact latest verified Rust source is retained: fmt, strict all-target Clippy
and 4,369 tests passed (eight ignored), with all 912 source/fixture/asset hashes
unchanged. Inventory283 and documentation whitespace checks pass.

Worksheet Data-default slice (`codex/t59-worksheet-default-fonts`, 2026-11-16): native
family/size application, manual 12-over-20 priority, future columns and the Font apply-mask
are measured (GUI 3). A separate Format > Worksheet/F4 dialog applies sparse Data defaults
beneath manual ranges, with the existing grid/print/image geometry policy, Excel effective
styles and `.opj` companion loss reporting. Remaining bulk/Standard/header propagation, other worksheet tabs,
category colors/text controls and native sticky geometry remain open. Formatting, strict
workspace all-target Clippy and 4,293 combined tests pass (8 existing ignored),
including all11 new model/UI/output regressions. Inventory, eight Python tooling tests,
frozen source/goldens, independent openpyxl review and inspected desktop captures pass.

Label-size slice (`codex/t59-label-default-fonts`, 2026-11-16): five Native2016 cases
establish independent Long Name/Units/Comments sizes for existing/future columns and
specific-over-bulk priority in both operation orders (GUI3). Sparse point-size fields and
the F4 category selector apply those three categories; Data family/size remain independent.
Category drafts survive switching, Apply/OK commits together, and existing manual-label
precedence remains an explicitly unverified Sheaf policy. Label family and all broader
category mutators stay disabled. Existing derived geometry and T27 omission rules remain.
Project/OPJ companions and effective Excel label styles retain sizes; omitted label defaults
warn without styling data blanks. Formatting, strict workspace all-target Clippy and all
4,302 combined tests pass (8 existing ignored), including all17 category-font regressions
and six new label-size regressions. Inventory, eight Python tooling tests and frozen source/
goldens pass. Independent openpyxl output review and fresh desktop captures were inspected;
all owned builders, Native/App instances, helpers and guards are terminal.

Reusable Custom RGB wells (codex/t59-custom-color-wells, 2026-11-17):
worksheet toolbar text/fill Custom... and Format Cells share the graph/editor
profile pool. Add saves an opaque draft colour without applying formatting;
choosing a well seeds the existing numeric or optional-colour draft. Existing
OK/Apply commits, Cancel retains preferences and discards unapplied formats,
and removal changes only the pool. GUI3/workbook13b/graphing7.12 record the
documented chooser contract and explicit Sheaf profile/menu policies, with no
fresh Native2016 measurement. Only the two private dialog UI functions and
shared menu's dialog-versus-submenu boundary change; storage/model/grid/font
defaults/IO remain unchanged. Named lists/project palettes, HSL/eyedropper,
category-default work and other measurements remain open.
Fmt, strict workspace/all-target Clippy, all4,445 tests (eight ignored),
inventory275 and eight Python tests pass with all925 source/fixture/asset hashes
frozen. Five new regressions cover both dialog routes, Add/Cancel/restart,
optional colours/Apply, no-op/Undo, compact columns/labels, stale guards,
removal and full worksheet/native-companion saves. Existing graph/editor wells
also pass after the shared menu gains a dialog boundary. Fresh private-profile
desktop Add in worksheet text/fill choosers, restart and Format Cells saved-well
choices/Apply/Undo match a complete saved worksheet; restored image-export pixels
match exactly. Choose/Apply/Undo do not rewrite preferences. Both owned apps and
guards close0; initial fixture-type pilots are excluded and source stays fixed.

Label-family slice (`codex/t59-label-default-families`, 2026-11-16): the official Format-tab
manual supplies the family-control contract for Long Name, Units and Comments (GUI3).
Native2016 category controls were observed, but ineffective family assignments/posted commits
are excluded; later COM activation refused before instance creation. No new family mutation
parity is claimed. Optional independent family strings, installed chooser/search, existing
manual/future-column policy and Automatic clearing preserve the shipped point-size schema and
Data behavior. Family-only edits keep sizes absent; Undo/drafts, project/OPJ companions and
Excel label styles share existing paths. Native family/Auto/manual parity and broader
categories remain open. Formatting, strict workspace Clippy, Nextest (4,356 passed, 8 skipped),
workspace doc tests, inventory and eight Python tooling tests passed. All 23 category-font tests
passed, including the six new family tests. The desktop review showed the category fonts on
existing and new columns. An independent XLSX reader confirmed the effective font styles and
preserved numeric values, and the SVG retained the label text.

Column Labels follow-up (2026-11-21). F4 sets a common family and size for Long
Name, Units, Comments, formula and parameter label cells. Individual label and
manual properties take precedence. Origin 2016 size coverage and the documented
family control are recorded in GUI 3.

Column Header and Row Header follow-up. Worksheet Properties sets the font
family and size of column names, row numbers and label-row captions. Origin
2016 size measurements and Sheaf's grid and print sizing rules are in GUI 3.

Standard font follow-up. Worksheet Properties sets the sheet's base family
and point size beneath category and manual values. Grid, print and image
geometry scale with the Standard size. The Origin 2016 size and family probes
used the worksheet's `wks.fsize` and `wks.font` properties (GUI 3).

Formula label follow-up. F4 > F(x)= sets family and size for formula label
cells below manual formatting and above Column Labels. Origin 2016 size,
precedence and new-column measurements are in GUI 3.

**Size:** L. **Branch:** `cell-format` (model/dialog), `codex/t59-format-toolbar` (toolbar/font geometry), `codex/t59-worksheet-controls` (underline/fill/keys), `codex/t59-worksheet-dialog` (dialog measurements/More entry), `codex/t59-custom-colors` (one-off RGB chooser), `codex/t59-classic-colors` (24 Classic worksheet presets), `codex/t59-xlsx-formats` (Excel cell styles). **Priority:** P0, wave 1.
**Spec:** [workbook model](../spec/workbook-model.md), new Cell formats section;
[GUI](../spec/gui-look.md) 3. **Inventory:** the worksheet-wide `T59` menu location now opens the bounded Data-font dialog; the three unsupported text-control choices are B5.
**Depends on:** T58's toolbar dispatch (slice 3). Model/dialog work can start independently.
**Ownership:** worksheet format model, range maintenance, grid and print drawing, Format Cells
window; worksheet adapter behind T58's dispatch. Coordinate worksheet/sheetops/grid edits with
existing cell-note, conditional-format and frozen-pane work.

## Goal

Format selected cells, rows, columns and label rows: bold/italic/underline, font/size, text/fill
colours and alignment. Formats follow the data through sheet operations, survive saves, and
appear in printed sheets and worksheet images. Numeric display format remains Column Properties.

## Origin facts

GUI section 3 records Format Cells on cell/column/row menus, Set Data Style > More... opening
the selected-range Format Cells dialog, and Ctrl+U underlining a selected column. Origin 2016
has no Default data-style entry. F4 opens separate Worksheet Properties with nine Apply To
categories and Data > Standard priority; visible cell/dialog defaults are recorded in GUI 3.
The X-Function index names `wcellformat`.
**Measured 2026-11-16:** custom Font Color/Fill Color slot opens Colors/Custom; RGB 17/34/51 text and 32/128/224 fill apply only to A1. Palette storage/HSL/eyedropper remain open (GUI 3).
**Measured 2026-11-16:** worksheet U button, Ctrl+B/I/U, Fill Color red/None, independent label
underline and data-only whole-column underline; GUI 3 records the synthetic UI runs.
**Measured 2026-11-15:** selected-cell Arial 9 pt default, Bold/Italic, Times New Roman, red text,
mixed-size blank field, first-cell mixed Bold state and row growth. Size-step buttons were disabled.
Reducing font size leaves Origin's enlarged height; Sheaf's current derived height can shrink.
**Not measured:** remaining Format Cells controls and mutation rules, remaining worksheet category-default
family/manual-priority and unsupported-category rules, rich text inside a cell, precedence of manual/conditional colours, sort behaviour and
`.opj` storage. Measure on Windows; label interim choices. The cloud draft proposed conditional
colours overriding manual colours and styles following rows during sort; these are proposals.

## Where

- New `crates/core/src/cellformat.rs`, compatible default/empty `Worksheet.cell_formats`.
  Store column IDs plus row spans and label-row addresses, with optional style properties.
  Whole-column styling must not allocate one record per row.
- `crates/core/src/notes.rs`, `sheetops.rs`: follow the existing maintenance pattern for row and
  column insert/delete/sort/move/duplicate/append, including column-ID remapping.
- `crates/core/src/condformat.rs`: existing `CellStyle`/`CellStyles` supply conditional colours;
  define explicit resolution with manual styles and cache lookups by document revision.
- `crates/app/src/grid.rs` (`body_cell`, frozen overlays), new `dialogs/cell_format.rs`,
  `toolbar.rs` worksheet target and contextual Ctrl+B/I/U shortcuts.
- `crates/plot/src/sheet.rs`, `crates/core/src/sheet_print.rs`: identical resolved styles.
- `crates/io`: `.sheaf` round trip; unsupported `.opj` fields warn/persist in companion per
  ADR 0008. Excel font/colour export if small, otherwise record as a leftover.

## Steps

1. Specify range/overlap/clear semantics and label-row addressing; implement model tests for
   all mutations above, undo/redo and append remapping. Record unmeasured decisions in the spec.
2. Draw styles and build Format Cells. One commit on OK/Apply; Cancel is inert. Test imported
   and new sheets, filtered selections, frozen panes and a whole-column million-row selection.
3. Add T58's worksheet adapter once its dispatch lands. Enable Format > Worksheet/Cell, the
   column/cell/row Format Cells items and Set Data Style > More... only when each is implemented.
   Rich Text/Wrap Text/Float are separate text controls (B5), not Clear Formats.
   The worksheet entry must implement its measured worksheet-wide scope, not merely alias a
   cell dialog with misleading semantics.
4. Verify print/image rendering, save/reopen, `.opj` warnings/companion and any Excel export.
5. Measure Origin's remaining dialog and precedence rules; reconcile and record differences.

## Done when

Styles work from toolbar and dialog, survive selection and structural operations, print as shown,
round-trip without loss, and all six inventory locations are enabled or reassigned with reasons.
Task-workflow checks pass. Conditional and manual formats interact as documented.

## Don't

Do not add merged cells, borders or rich text inside a cell here. Do not alter numeric display
formats, allocate per-cell styles for a whole column, or allocate while drawing every cell.
