# Handoff

Last updated: 2026-11-15. Start here, then read [the prioritized plan](docs/tasks/PLAN.md),
[the task workflow/index](docs/tasks/README.md), and **one** task brief. Open only the spec sections
that brief names. This update finishes the paused cloud session's planning work; it does not
implement T58–T63 or claim a fresh comparison with Origin.

## Current priorities

1. **Basic formatting:** graph text and its Format toolbar (**T58**), then worksheet cells
   (**T59**). Bold, italic, underline, scripts, Greek, font, size and colour must be usable without
   manually writing escape codes.
2. **Stacked figures and layers:** merge existing graphs and arrange their layers (**T60**), add
   layers/insets and three/four Y axes (**T61**), then remaining stack/offset commands (**T62**).
3. **Existing-feature wire-ups:** **T63**, claimed by group, can proceed beside those priorities
   when its files do not overlap. Then follow waves 4–9 of the plan.

Do not rebuild working stacked lines/waterfall, Double-Y, column-based 2/4/9-panel or Stack plots,
or stacked bars/areas. The gap is editing/combining finished graphs and the remaining controls.
T58 owns the shared Format toolbar dispatch; T59 adds the worksheet target after that lands.
T60 owns arranging/merging layers; T61 owns adding/deleting them. Coordinate edits to shared files.

## Where the detail lives

- [PLAN](docs/tasks/PLAN.md): wave order, dependencies, ownership boundaries and agent loop.
- [Task index](docs/tasks/README.md): status and leftovers for every brief, including T58–T63.
- [Greyed inventory](docs/tasks/greyed-items.md): **307 listed item locations** at the planning
  baseline (211 menu bar, 19 toolbar, 77 context menu). Run `python3 tools/greyed-count.py --check`
  after changes. This is a hand-maintained inventory, not a generated audit of the Rust source.
- [Known issues and UI walkthrough](docs/tasks/known-issues.md): preserved findings and acceptance
  cases. Specs remain the source of measured behaviour and numeric fixtures.
- [Roadmap](docs/ROADMAP.md) and [parity](docs/spec/feature-parity.md): broad progress, not task claims.
- [Archived handoff](docs/archive/HANDOFF-2026-11-15.md): the complete previous file at `dec7374`,
  unchanged, including merge history, detailed findings, owner questions and local resource notes.

## Standing rules from the owner

- **When unsure, copy Origin:** measure it through `tools/origin-ref/` (or read its docs for
  behaviour) and copy it; record the measurement in the spec. Don't ask the owner what Origin can
  answer. Clean-room: copy behaviour and layout, never code, icons, help text or files.
- **Subagents:** Claude sessions use Sonnet for building, research and reviews, and Haiku for tests
  and output checks. **Codex sessions use Terra instead of Sonnet and Luna instead of Haiku.**
- **Codex GitHub monitor:** keep a dedicated Luna subagent checking GitHub periodically while work
  is in flight. Watch remote `main`, branches and task claims for another session taking or
  overlapping the task; reconcile new work before merging and never overwrite another session's work.
- **Codex continuous work:** the owner requested an ongoing queue: finish, review, verify and
  merge/push one suitable task, then find and claim another. Do not stop merely because one task
  is complete. Refresh GitHub and the handoff before selecting the next task; respect active claims.
- **No co-author line:** never add `Co-Authored-By: Claude ...` or "Generated with Claude Code" to
  commits or PRs; say so in every subagent prompt. (History was rewritten on 2026-11-12 to remove
  them; branches older than that must be rebased onto `main`, matching commits by tree.)
- **Repository:** https://github.com/Electrochemistry-Undergrads/OriginRust (private). Nothing on the
  owner's personal account.
- **Two or more sessions** often share this repo. Pull the latest changes and inspect branches,
  worktrees and claims before choosing work. Publish the task claim in this file on `main` (push it)
  and in `re-workspace/scratch/peer-notes.md` (local only, read its last 40 lines on start), so other
  computers can see the split. Merge to `main` one branch at a time, after `git fetch`.
  See LESSONS "Two sessions on one repository need a written split".
- **Claim before you start, and put the claim on `main`:** before touching any code, docs or
  spec for a task, add a claim line (task, branch, worktree, session, date) to this file and to
  `re-workspace/scratch/peer-notes.md`, **commit the claim and push it to `main` first** (a docs-only
  commit; `git fetch` and rebase right before the push), and only then begin. Other agents and other
  computers only see work that is on `main`; a claim that exists only in your worktree or in the
  local peer notes does not count. If someone else's claim already covers the task, pick another.
  When the work is finished and merged to `main`, release or update the claim in the same push.
- **Inactive task claims:** If a task has had no edits for at least 48 consecutive hours,
  another agent may take ownership. Check the task brief, its branch and worktree when
  available, and the latest peer notes for evidence of newer work. Record the takeover in
  `re-workspace/scratch/peer-notes.md` with the date, new owner, branch and scope, and preserve
  earlier work. This rule applies to claims listed below and in the task briefs.

## Latest integration

T58 full named Classic editor palette (`codex/t58-classic-editor-colors`, 2026-11-16):
Text color > More Classic colors adds the remaining 16 names to the shared label/title/legend
editor, completing all 24 measured Classic index/RGB choices (graphing 7.2/7.8). The compact
submenu grouping is Sheaf policy. Existing selection/caret, Unicode/scripts, base fonts,
references, automatic text, draft/Apply/OK/Cancel, Undo and project/companion rules are retained;
named choices seed the existing custom RGB fields. The private palette and own tests are the
only Rust changes. Fmt, strict workspace/all-target Clippy, all 4,221 workspace tests (eight
ignored), all 95 related formatting UI/helper/preview tests, inventory 309 and eight Python
tooling tests pass on combined T61/T103 source. Frozen source/goldens and freshly built,
inspected complete submenu/Royal RGB0/0/160 desktop captures pass; all owned apps exit0.
Own claim/window released by this push. T58 remains partial for other editor insertion
controls, special escape aliases and native palette storage.

T61 stock multi-Y legends (`codex/multi-y-template-legends`, 2026-11-16): all four templates
use Origin-measured top-left anchors above their frames, Arial 22 pt (YY-YY: 20 pt), and no
frame/background. Placement aligns visible legend bounds, keeping long labels growing right
and entries clear of the axes. Empty and populated template paths agree; the Legend editor,
Undo, `.sheaf` and `.opj` companions retain the anchor. Bare `.opj` writes a fixed position with
a loss warning. Graphing 21.12 records 24 native controls/readbacks and remaining spacing and
extra-column membership limits. Fmt, strict workspace/all-target Clippy, all 4,214 tests (eight
ignored), seven focused regressions, four SVG goldens, inventory 310 and eight Python tests pass.
Four fresh CLI PNGs were inspected with frozen source/golden hashes and four test threads.
Only this claim is released; the fixed push window expired at 11:15 CST. T61 remains partial.

T96 wavelets (B8 slice, branch `wavelets`, analysis-tools.md 19, Origin 2016 measured): Analysis > Signal Processing >
Wavelet > Decompose, Reconstruction, Denoise, Smooth (Origin's 21 wavelets and two boundary modes; dwt/idwt exact) and
Continuous Wavelet (Morlet, DGauss, MexHat; CWTResults sheet). Open rules: the sqtwolog threshold is a fit to measured
brackets, Smooth is exact only for periodic power-of-two lengths, denoise boundary samples can differ (analysis-tools 19.5).

T59 worksheet Classic presets (`codex/t59-classic-colors`, 2026-11-16): More Classic colors
adds 16 named choices to the worksheet Font Color and Fill Color menus, completing all 24
Classic RGB colours. Both native palettes and selected-cell Dark Yellow/Royal/Dark Cyan/None
effects were measured in Origin 2016 (GUI 3); the compact submenu grouping is Sheaf policy.
Existing captured requests preserve one-step Undo, effective no-ops, stale-target protection,
compact whole columns, independent labels and project/companion persistence. Both menu levels
close after choosing a colour; Custom RGB remains available and graph lists retain their scope.
Fmt, strict workspace/all-target Clippy, all 4,157 workspace tests (eight ignored), all 52 related
UI regressions, inventory 327 and eight Python tests pass on the combined T97 source, with frozen
source/goldens and inspected desktop submenus from the tested executable. Own claim/window
released by this push; T59 remains partial for worksheet category defaults, vertical alignment,
sticky heights, Excel formats, other colour lists and palette storage.

T59 worksheet custom RGB colours (`codex/t59-custom-colors`, 2026-11-16): worksheet
Font Color and Fill Color > Custom... accept opaque RGB bytes, preview a swatch and commit
on OK/Enter; Cancel/Escape discard the draft. Native Colors/Custom and selected-cell effects
were measured in Origin 2016 (GUI 3). Starting from the current manual colour is Sheaf policy.
Existing captured range/revision/generation guards, no-op checks and one-step Undo preserve
independent data/label styles and compact whole columns; `.sheaf` and `.opj` companion reopen
retain both colours. Fmt, strict workspace/all-target Clippy, all 4,146 workspace tests (eight
ignored), all 47 related UI regressions, inventory 330 and eight Python tooling tests pass,
with unchanged source/goldens and inspected compact choosers from the tested executable.
Own claim/window released by this push. T59 remains partial: worksheet category defaults,
vertical alignment, sticky heights, Excel formats and full Classic/palette storage remain open.

T91 (B8 slice, branch `fit-surface-more`, fitting.md 16-18, analysis-tools.md 18, Origin 2016 measured): Analysis >
Fitting > Simulate Surface (22 surface functions, Origin's defaults and noise rule), Nonlinear Implicit Curve Fit
(circle/ellipse by orthogonal distance, matches Origin's geometric fit), Compare Datasets (report unmeasured: pooled-fit
F-test is Sheaf's own), Mathematics > XYZ Surface Area (Delaunay area as Origin's xyzarea to 1e-12).

T59 worksheet dialog/menu audit (`codex/t59-worksheet-dialog`, 2026-11-16): measured Origin
2016 Set Data Style > More... now opens the existing selected-range Format Cells window with
Apply/Cancel/Undo. The fictitious Default placeholder is replaced by the actual disabled Rich
Text, Wrap Text and Float choices (future B5 text controls). Worksheet Properties/F4 has separate
category defaults; its nine Apply To targets and visible dialog defaults are recorded in GUI 3,
and F4 remains grey pending the proper model. Fmt, strict workspace/all-target Clippy, all 4,051
workspace tests (8 ignored), all 40 related UI regressions and the 338-item inventory pass on
current fonts/paired-script/T90 source, with unchanged source/goldens and freshly rebuilt,
inspected submenu/More desktop captures. Only own claim/window released; T59 remains partial.

T58 paired sub/superscript columns: `\=(sub,sup)` now uses Origin-measured shared left
columns and maximum branch advance, with nested style/font/colour/Greek restoration,
60% script size and 49% up / 35% down baselines (graphing 7.10). Twenty native controls
and four size checks, ten regressions, SVG/stroke PNG goldens and actual legend/tick/
rotated-title layout cover the shared export path. Editor insertion remains separate.
Formatting, strict workspace/all-target Clippy and 4,047 tests pass (eight existing ignored),
with four test threads and frozen source/golden hashes. Inventory 340 and eight Python
tooling tests pass. No editor, model, IO or general backend changes in this slice.

T83 Origin measurements (2026-11-16, Claude Windows, docs only, graphing 14.29): Origin 2016's Interval Plot
uses the same Student-t 95 % interval as Sheaf but a two-entry legend and the Y title "Range"; its Pareto
(raw data) puts the cumulative points at the bar centres, labels them, and scales the count axis to 1.1 x
the total so it lines up with a 0..110 % axis titled "Cumulative Frequency". Differences listed in the T83
Status for a code slice. No build again this run: every build folder had been used within four hours and
25 GB were free, too little for another full one.

T58 inline font families: full-name font escapes now reach per-run measurement, glyphs
and decoration metrics, with nested restoration, scripts, Greek and rotation retained.
Graphing 7.6 records the native selected-face controls and name/fallback limits; eight
regressions and SVG/stroke PNG goldens cover standard-family and shared export behaviour.
Combined scripts and special aliases remain open; editor and Format-toolbar claims remain
with their owners.
Formatting, strict workspace all-target Clippy and 4,037 combined tests pass (eight existing ignored).
The 340-item inventory and eight Python tooling tests pass; all workspace tests ran
with four test threads, retaining every test and assertion.

T81 (B8 slice, branch `stats-anova3`, statistics.md 24-26, Origin 2016 measured): Statistics > Descriptive
Statistics > Partial Correlation Coefficient (Origin's pcorrcoef), ANOVA > Three-Way ANOVA (type III, means comparisons,
Levene, power) and One-Way / Two-Way Repeated Measures ANOVA (Mauchly, Greenhouse-Geisser, Huynh-Feldt), with
`sheaf analyze partial-corr|anova3|rmanova`. Three-way refuses more than 30 levels per factor. fmt, strict Clippy, full tests pass.

T59 worksheet controls (`codex/t59-worksheet-controls`, 2026-11-16): Underline and the basic eight
fill colours plus None now format selected worksheet cells, label cells and compact whole-column
ranges. Contextual Ctrl+B/I/U keeps text editors, dialogs and graph shortcuts independent; each
press or effective palette change is one Undo step, and stale requests/no-ops are rejected.
Origin 2016 U/shortcut/fill and label-versus-column behaviour was measured on synthetic data.
Fmt, strict workspace/all-target Clippy, all 3,971 workspace tests (8 ignored), all 26 related
toolbar regressions and the 348-item inventory pass, with unchanged source/goldens and an
inspected isolated desktop capture. Only newer claim metadata was merged after validation;
the exact tested program is unchanged. Own claim/integration slot released; T59 remains partial.

T59 worksheet Format-toolbar adapter (`codex/t59-format-toolbar`, 2026-11-16): Font, Size,
Bold/Italic and the basic eight text colours now target selected cells, rows, columns and label
cells. Installed faces and point-to-pixel conversion drive derived grid/print row geometry,
including filtered views and frozen panes. Enter commits sizes before the editor loses focus
for worksheet and graph targets. Mixed summaries, stale-selection/revision guards and no-op
checks preserve data and one-step Undo; whole-column styles remain compact and survive native
and companion saves. Origin 2016 measurements are in GUI 3: Arial 9 pt, per-cell styles/colour,
mixed-size blank field, first-cell mixed Bold state and row growth. Sheaf can shrink derived
heights; Origin retains enlarged heights. Worksheet-wide scope, Set Data Style, toolbar
underline/fill/shortcuts, persistent heights and remaining measurements stay open. Empty label
rows retain the existing print omission policy (T27). Validation: fmt, strict workspace/all-target
Clippy, full 3,934 passed / 0 failed / 8 ignored, all 17 toolbar regressions, inventory 368, frozen
source/goldens and an inspected isolated desktop capture. Source scope and integration window
released by this push; the task remains partial.

T82 (`line-series`, B4 slice): Plot > Line + Symbol > Line Series as Origin 2016's (one plot, k-point
segment connection, rows never joined, column-name ticks; data in a LineSeries sheet; graphing 18.15).

T79 (`column-scatter`, B4 slice): Plot > Symbol > Column Scatter as Origin 2016's (columns at 1, 2, ...,
diamonds size 8, jitter within +-0.15, column-name ticks, Y title Range; graphing 18.14).

T62 Origin measurements (2026-11-16, Claude Windows, docs only, graphing 13.16-13.17): Origin 2016's
Stacked Histograms put the first column at the bottom, give each panel its own bins and X scale and draw
every panel in the same red hatch on a portrait page with 5 % gaps; its Waterfall Y/Z Color Mapping colour
from red (low) to blue (high), the reverse of Sheaf, on Origin's oblique waterfall. The differences are listed
in the spec and the T62 Status for a code slice. No build this run: the disk had 836 MB free, so the idle
build folder `target/wf-autosave` was deleted (23 GB free after) and no new one was made.

T72 Copy Format / Paste Format (B2 slice 1, branch `copy-format`, graphing 26, Origin 2016 measured): Edit,
page and axis menus copy a graph or axis format (All, All Style Formats, Colors, Fonts, Background, Dimensions,
Scales; axis All) and paste it as one "Apply Theme" undo step; Edit > Paste Format... opens Apply Formats (its
graph-clipboard grid is Sheaf's reading, unmeasured). Unmeasured choices stay grey; the clipboard is in-app only.

T78 recent lists (`recent-lists`, macOS, no Origin): Most Recently Used submenus at the end of the Plot,
Worksheet, Analysis and Statistics menus (up to 10 menu-bar commands each, newest first, re-resolved against the
current window when chosen) and Recent Exports (files written by graph, ASCII and Excel exports; choosing one reopens
that export dialog on its window and folder). Both persist in `recent.json` (T38). New `app/mru.rs`, spec
`recent-lists.md`; Origin's theme entries and re-run semantics are unmeasured (B10 done).

T77 New Graph and projects without data (`new-graph-nodata`, macOS, no Origin): File > New > Graph and the
Standard toolbar's New Graph add an empty graph from the default template (one undo step); File > Save Project
without Data... writes a copy with every value, mask, note, format, filter and fit mark removed (structure,
formulas and graphs kept), never over the open project's own file; Duplicate Project without Data offers to save,
then opens that copy as an untitled project (as Origin's later Clone Project). New `core/without_data.rs`.
Origin 2016's behaviour of the two items is undocumented; unmeasured choices in graphing 25 and project-file-format.

T83 interval plot and Pareto (`interval-pareto`, macOS, no Origin; renumbered from T82, which `line-series` also took): Plot > Statistics > Interval Plot (mean of each Y
column with a t-CI/SD/SE error bar; `sheaf plot -t interval-plot [--interval-error ci|sd|se]`) and Pareto (counts of one
column, descending, with a cumulative-percent line on a 0-100 right axis; `-t pareto -y COL`). New `Plot.stat`
(`StatPlot`), `Project::plot_interval`/`plot_pareto`. Origin's dialogs, binned Pareto and measured defaults are
leftovers (graphing 14.29, unmeasured).

T76 grouped indexed plots (`grouped-indexed`, macOS, no Origin; renumbered from T75): Plot > Column/Bar/Pie >
Grouped Columns (indexed data) and Plot > Statistics > Grouped Box Charts split one data column by one or two
group columns (selection: group columns + one Y column; `sheaf plot -t grouped-columns|grouped-box --group G1[,G2]`).
Sorted categories on X, one plot per subgroup level (legend), mean bars with optional SD/SE (Plot Details Error Bar
tab, `--group-error`), dodged boxes with the existing box statistics. New `core/indexed.rs`, `Plot.indexed`.
Origin's dialog, statistic choices, table tick labels and >2 groups are leftovers (graphing 14.28, unmeasured).

T75 (`scatter-central`, B4 slice 3): Plot > Symbol > Scatter Central as Origin 2016's (both axes at
-50 %, boxed frame, titles at the axis ends); `Axis::offset_pct` now works for X (graphing 18.13).

T73 (`change-xy`, B3 slice 3): plot right-click Change X, Change Y and Ungroup as measured in Origin
2016 (column lists, greying, no rescale, colours kept on Ungroup; graphing 15.7d).

T58 Format-bar remainder (`format-bar-rest`, macOS, no Origin): Underline, Superscript, Subscript and Greek
toggle one whole-text markup group on a selected free label or explicit X/Y title (unmatched parentheses
escaped); Fill Color sets a selected rectangle/ellipse fill (or None) or the legend background; Line Color
sets a drawn shape's or straight line's colour. One undo step each, stale requests rejected, selection kept.
New `app/object_format.rs` beside `text_format.rs`. Auto titles, legend markup, label frames and plots stay
grey; whole-object semantics are unmeasured Sheaf choices (graphing 7.4). Three rustc 1.100 Clippy lints fixed.

T71 (`b4-segments`, B4 slice 2): Plot > Line + Symbol > 2 and 3 Point Segment as Origin 2016's line
connections 2 and 3 (measured; graphing 5.2), in the plot settings' connect lists and `.opj`. Scatter
Central needs an X axis inside the frame and stays B4.

T69 rest of B1 (branch `b1-rest`, graphing 23, Origin 2016 measured): View > Zoom All zooms the active graph so all
objects fit (it is not a presentation of all graphs; T65's note corrected); Tools-bar Scale In sets the active
layer's axes to a dragged rectangle (linked axes follow, log axes in log10, one undo step "Zoom In/Out"), Scale
Out steps back through earlier scales. Left: Ctrl+drag "Enlarged" graph, double-click restore. B1 is complete.

T70 (`b3-small`, B3 slice 2): axis right-click Center and Format > Fit Page To Printer as measured in
Origin 2016; Format > Fit Page To Layers... now opens its window as Origin's does (graphing 15.7d).
Add Legend Title is not in Origin 2016 and stays B3.

T58 label text editor: Normal, bold, italic, underline, strikethrough, scripts and Greek
buttons now format the selection or caret group, with a cached preview through the public
export renderer. Twelve focused regressions cover Unicode selections, nesting/toggling,
empty drafts, Cancel/Apply/Undo and native/Origin-project preservation; an inspected desktop
capture shows the scientific label preview and unobscured controls. Graphing 7.5 records
Origin's published Text-tab workflow and the unmeasured Sheaf choices. Title/legend editor
reuse, title menu entries and inline family/colour pickers remain open; other T58 owners keep
their scopes. Formatting, strict workspace all-target Clippy, 3,725 tests (eight existing
ignored), inventory and eight Python tooling tests pass on the latest integration tree.

T58 whole-object Format toolbar (`178e28ca`) now applies installed font family, size,
Bold/Italic, measured size scaling and eight basic font colours to selected labels, legends
and generated/explicit axis titles. Seven UI regressions cover selection, stale palettes/actions,
validation, Undo/Redo, SVG and native/companion saves. Graphing 7.4 records the Origin controls
and the unresolved 2016 whole-object Bold/Italic difference. Formatting, strict workspace
all-target Clippy, 3,713 tests (8 ignored), inventory 397 and eight Python tooling
tests pass; source hashes match. An isolated desktop capture was inspected. The shared dispatch
is ready for T59's worksheet adapter; remaining T58 controls/editor work retain their scopes.

T80 colour and bubble scales (`color-scale`, B3 slice 4, macOS, no Origin): `Layer::scales` (colour scale = hexbin's bar over a
colour mapped plot's Z, bubble scale = 14.16's four-circle size legend), Graph > New Color Scale and the legend's Add Color
Scale / Add Bubble Scale (enabled when the layer has a matching plot), select/Delete, `.opj` warning (graphing 27).

T67 Add Straight Line (`straight-line`, B3 slice 1): Origin 2016's addline measured (names, ends,
label text and its 5 % placement) and copied: `Layer::straight_lines`, drawing, the Graph menu
window, select/Delete/edit, `sheaf plot --vline/--hline`, `.opj` warning (graphing 17.7).

T68 (B4 slice 1, `plot-types-b4`, Claude Windows): Plot > Symbol > Vertical Drop Line and Plot >
Column/Bar/Pie > Column + Label as Origin 2016's DROPLINE and ColumnLabel templates (measured,
graphing 18.12); data labels of grouped columns are now centred on their own bar, as Origin draws
them. Formatting, strict all-target Clippy and 3679 workspace tests pass (8 ignored) on 91c8d3b2.
T58 RGB text colours: numeric `\cN(text)` escapes now follow the measured one-based
RGB encoding, including channel borrows and the white boundary. Nested palette/RGB spans
restore their surroundings; glyphs and decoration paths use the same colour. Graphing 7.3
records native scalar/vector controls and new SVG/stroke PNG goldens. Font-family switching,
combined scripts and special colour tags remain open; editor and Format-bar work retain
their separate claims.
Formatting, strict workspace all-target Clippy and 3,677 combined tests pass (eight existing
ignored); the 407-item inventory and eight Python tooling tests also pass.

T66 graph view modes (B1 slice 2, branch `graph-view-modes`, graphing 22, Origin 2016 measured): View > Print View
(Ctrl+2), Page View (Ctrl+1, default), Window View, Draft View (Ctrl+0), saved per graph in `.sheaf`; View > Show >
Layer Icons; Full Screen (Ctrl+Shift+J, reminder, Esc ends, arrows/Home/End switch graphs). Nine Show items and Show Data
Info stay grey (unmeasured: Origin's collapsing menu). fmt, strict Clippy, full tests pass.

T58 indexed text colours: `\cN(text)` now uses the original 24-colour list, with measured
positive-index byte selection and black fallback. Nested spans restore their surrounding
colour; glyphs and underline/strikeout strokes share the run colour. Exact Origin 2016 EMF
fixtures, new SVG/stroke PNG goldens and text/export regressions are documented in graphing
7.2. Font switching, combined scripts and tagged RGB/special colour codes remain open; editor
and Format-toolbar work stay with their owners. Formatting, strict workspace all-target Clippy
and 3,654 tests pass (eight existing ignored); inventory 414 and eight Python tooling tests
also pass on the combined zoom integration tree.

T65 graph view zoom and pan (`graph-zoom`, B1 slice 1): Origin 2016's View Zoom In mode (x2,
re-centred on the click), Zoom Out by halves to 25 %, Whole Page and the Tools-bar Zoom-Panning tool,
measured and copied (gui-look 4.x); the Tools-bar magnifiers are Origin's axis Scale In/Out (still B1).

T58 graph text, first renderer slice: inline underline and strikethrough use shared scene
paths and resolved-font metrics, including nested scripts and rotation. Nine focused tests,
SVG and font-independent stroke PNG goldens cover glyph anchors, title/legend/label/tick
placement and EPS/EMF/PDF/raster output. Origin scalar/PNG observations and layout limits are
recorded in graphing 7.1. Per-run colour/font switches, combined scripts and editor/toolbar
formatting remain open; the 15 UI inventory locations remain disabled. The separately claimed
T64 significance plot is unaffected. Formatting, strict workspace all-target Clippy and
3,624 tests pass (eight existing ignored), with the 421-item inventory and eight Python
tooling tests passing on the current T59/T60/T61/T62/T63 integration tree.

T63 J (`t63-add-plot`): Graph > Add Plot to Layer > Line/Scatter/Line + Symbol/Column/Area open
Layer Contents with that plot type chosen, as measured in Origin 2016; Area joined the type list.

T61 layers, insets and three/four Y axes (branch `layers-multi-y`, graphing 21, measured in Origin 2016): Graph >
New Layer(Axes) with nine kinds and six Graph-toolbar buttons, Add Inset (with data), Delete Layer (children
unlinked, keep their place and ranges), Layer Properties, displaced Y axes in margins and hit-testing, Move to
Layer (Sheaf's own), 3Ys Y-YY / Y-Y-Y and 4Ys Y-YYY / YY-YY templates and Multiple Y Axes (`plot -t 3ys-y-yy` etc.).
Left: Open Dialog..., three layer-icon items, Origin's legends in the templates. fmt, strict Clippy, full tests pass.

T60 Merge Graph Windows and Layer Management (`merge-graphs`): Origin 2016's merge_graph measured in
full (page, cells, automatic grid, text factor, links, Show Axes Frame, aspect, keep/rearrange) and
copied in `sheaf_core::merge`, the Graph menu window and `sheaf merge-graphs`; Layer Management
(Arrange, Size/Position, Link; `layer_manage.rs`) from the Graph menu, page and layer-icon menus; Fit
Page to Layers as `pfit2l` (frames and objects plus a border). Fit Layer to Graph did nothing
measurable in Origin and is tagged `N`. Graphing 15.7c and 15.7d; leftovers in the T60 brief.

T63 F Duplicate entry points (`a7fd735`) now reuse the existing window-copy operation:
Standard follows the active workbook/graph; Explorer's context item follows the clicked window,
including a hidden one. Four new UI regressions cover target identity, empty books, cancellation
and Undo/Redo; the existing duplicate tests and desktop captures also pass. Formatting, strict
workspace all-target Clippy, 3,450 tests (eight existing ignored), frozen source/tree checks and
the 457-item inventory validation pass on the combined T59/T19/T05 tree. T63 C's probability/Q-Q
entries remain disabled and are deferred to B8/T14 with published score/confidence/axis prerequisites
in the T63 brief; the existing statistics tools are unchanged.

T05 export Dialog Themes (`21480e7`) now save/reload named settings, custom defaults/reset and
last successful exports, with modified-name marking and confirmed deletion. Seven regressions,
five isolated Origin sessions and an inspected desktop screenshot cover this scope. Formatting,
strict workspace all-target Clippy and 3,443 tests pass (eight ignored); inventory 459 passes.
Graph Theme, graph-specific settings and comparison/script tools remain (graphing 9.8).

T02 Column Browser Exclude (`e606eef`) now temporarily hides selected upper-list columns,
selects the first removed list position (or the final remaining row), and preserves original
column ordinals and already-added ranges. Scope changes and reopening restore excluded columns;
gapped original columns cannot form one block. Ten valid isolated Origin cases and seven new UI
regressions cover this follow-up (46 focused Go To tests). Formatting, strict workspace all-target
Clippy, 3,394 workspace tests (eight existing ignored) and inventory validation pass on the combined
T48/T64 tree; incoming Windows MCP test fix `81765e8` is preserved. Saved browser preferences remain.

T48 graph Data menu measured in Origin 2016 (branch `t48-origin`, graphing 20.4/20.5): Mask (a click masks,
a drag masks every plot's Y cells, Tab active plot, Space freehand) and Remove Bad (pick, then Enter;
`@YPR` 1 keeps the row) now follow Origin, with its undo texts and Esc/right-click ending a mode.
Move Data Points stays unmeasured (synthetic input never engaged it). Checks: fmt, strict Clippy, full tests.

T08 static worksheet-cell substitution (`34c1ba3`) adds fixed qualified cell references,
measured 15-digit defaults, column-display `W` and preserved text. Expression/core/I/O tests
cover recalculation, cycles, Undo and native/companion reopen; expression-language section 3
records scalar measurements and the existing no-generated-code boundary. Active-sheet/book-only,
dataset/graph/session and generated-reference forms remain open. Formatting, strict workspace
all-target Clippy and 3,356 tests pass (eight existing ignored); inventory and eight Python
tooling checks also pass on the combined browser/slope/prioritization tree.

T25 proxy disconnect cleanup (`ef98bf7`) and T02 browser row ranges (`d9b4af7`) landed during
planning; both claims are released. T52 slope annotation also landed (`33c4d246`);
its implementation and unmeasured Origin defaults are recorded in graphing 17.6 and T52. Results and recorded checks are preserved in
[known issues/recent results](docs/tasks/known-issues.md).

**T58 axis-title and legend editors (2026-11-16, Codex):** Format > Axis Titles and existing
title/legend Edit Text routes use the shared selection editor and actual-object preview
(graphing 7.7). Automatic metadata, legend symbols/name references, fonts, visibility,
Cancel/Apply/Undo and native/Origin-companion saves are covered. Read-only plot facades
reuse existing title and raster rules. Remaining T58 inline family/colour pickers and
combined-script controls remain in the brief. Formatting, strict all-target Clippy and
3,957 workspace tests pass (eight existing ignored), including all 21 related tests;
inventory validation, eight Python tooling tests and three inspected desktop captures pass.

**T58 selected-text colours (2026-11-16, Codex):** Text color in the shared label/title/legend
editor offers eight existing Classic colours, preserving scripts, literal text, name references
and base fonts (graphing 7.8). Choices are idempotent drafts with existing history and project
persistence. Renderer/model/worksheet controls are unchanged. Broader/custom colour controls,
inline families and combined-script insertion remain separate. Formatting, strict all-target
Clippy and 3,981 workspace tests pass (eight existing ignored), including all 29 related
editor tests; inventory validation, eight Python tooling tests and three inspected desktop
captures pass with source hashes unchanged.

**T58 custom RGB text colours (2026-11-16, Codex):** Text color > Custom RGB in the shared
label/title/legend editor validates opaque 0-255 channels and previews a swatch before an
explicit colour choice (graphing 7.9). Field edits stay separate from the text draft; Apply
color restores selection/typing focus, then existing Apply/OK/history and project saves
apply. Existing RGB rendering, font groups, script columns and name references are retained;
no renderer/model/worksheet changes. Full named Classic expansion and other editor insertion
controls remain separate. Formatting, strict all-target Clippy and 4,062 workspace tests
pass (eight existing ignored), including all 37 related editor tests. Inventory, eight
Python tooling tests and four inspected desktop captures pass with source hashes unchanged.

**T59 worksheet Excel cell styles (`codex/t59-xlsx-formats`):** XLSX export now combines resolved
worksheet fonts, point sizes, bold/italic/underline, text/fill RGB and horizontal alignment with
column numeric/date/time codes. Labels, mixed/text values, overlapping resets and bounded blank
cells retain their semantics. Conditional colours are reported snapshots; omitted label/future
formats and partial transparency have explicit limits (workbook-model 13b). Formatting, strict
workspace all-target Clippy and 4,169 tests pass (8 existing ignored), including all six new
output regressions and all 50 existing Excel tests. Inventory and eight Python tooling tests
pass; independent openpyxl review confirms the generated file's values, Unicode, literal text,
styles and labels. All source/golden hashes stayed fixed. No model, UI, renderer or OPJ changes.
T59 category defaults, vertical alignment and sticky row heights remain separate work.

**T59 alignment measurement (`codex/t59-cell-vertical-align`):** inspected Origin 2016
numeric A1 Format Cells and Worksheet Properties Data Format/Size routes supplied no vertical
text-alignment control. GUI 3 and workbook-model 13b record this bounded negative evidence and
excluded probes; other modes/routes remain unestablished. This slice adds measurement notes,
with existing placement retained pending a witnessed native route. All six Native actors and
their owned instances/helpers/identity guards are terminal; accepted captures/control lists
were inspected and hashed. Formatting, strict workspace all-target Clippy, 4,205 tests
(8 existing ignored), inventory, eight Python tooling tests and frozen source/goldens pass.
T59 remains partial; category defaults and sticky heights are separate work.

## In flight (check for recent edits before claiming)

| Branch | Owner session | What | State |
|---|---|---|---|
| `codex/t58-classic-toolbar-colors` | Codex Windows | T58 remaining24 Classic Font Color presets for selected graph labels/titles/legends in Format toolbar | claimed 2026-11-16; own `.claude/worktrees/codex-t13-skip-points` + `target/wt-t13-skip-points` reused afterfbfcc19c/FULL4221/95 related/all actors terminal. Only toolbar/format.rs font-colour adapter/private Classic helper/constants, new own graph UI tests/registry and bounded existing own worksheet-classic graph capability guard (retain worksheet-only RGB guard), GUI1/graphing7.4/T58/index/roadmap/parity metadata. Existing captured requests/fields only; worksheet palettes remain compatible, graph fill/line/shared object_format palette stay separate. No private text editor/palette, Core/model/IO/renderer/fonts/geometry, graph_view/shapes/gadgets/T60/T61, worksheet defaults/T59, themes/templatesT104, stats/fitting/MCP/import/clipboard, palette storage/HSL/eyedropper, reserved tasks/formula-parameter labels or ROOT pending files. Respect T104 current11:58 and T60 future12:15-12:40 windows; no own product window claimed. |
| `codex/t60-layer-display` | Codex Windows | T60 Layer Management Display > Color > Background Color; Native Apply/Cancel/OK/selection measurements completed | claimed 2026-11-16; own `.claude/worktrees/codex-formula-parameter-labels` + `target/wt-formula-parameter-labels`. Only dialogs/layer_management.rs +private background helper, own UI/render/save regressions and graphing15.7d/T60 metadata; existing model/renderer/IO production paths reused. No graph_view/shapes/gadgets, layer lifecycle/T61, merge/Arrange/Add/Axes, other Display controls, T58 editor/palette, T59 defaults, T102/T103/T104 templates/themes, stats/fitting/MCP/import/clipboard/reserved tasks or ROOT pending files. Requested FIXED product integration 2026-11-16 12:15-12:40 CST, releases on publication/failure/deadline. Prior T58 11:40-12:05 respected; ready T104 goes next and its earlier active integration takes precedence. Docs-only claims remain fine. |
| `codex/t59-worksheet-default-fonts` | Codex Windows | T59 worksheet category font defaults for Data and Column Labels/specific label rows; Native inheritance/manual override/new-column measurement first | claimed 2026-11-16; own `.claude/worktrees/codex-t01-error-bars-dialog` + `target/wt-t01-error-bars-dialog` reused after23e89f2c/FULL4205/all actors terminal. Bounded compatible worksheet font-default model/new module +cellformat-index fallback, existing worksheet font-derived metrics/text rendering, new private category-font dialog +existing Worksheet Format/F4 dispatch/state/menu branch/registrations, XLSX effective styles and existing worksheet-format save warning as required, own regressions and GUI3/workbook13b/T59 metadata. Font family/size only; unsupported Standard/header mutators remain disabled. No colour defaults/text controls/vertical alignment/sticky heights, print dialog/layout/extents/T27, graph editor/palette/renderer/fonts/T61, T102 theme storage/T101 gadgets, stats/fitting/MCP/import/clipboard, reserved tasks/formula-parameter labels or ROOT pending files. Measure before implementation. No product window; respect T61 FIXED10:45-11:15 and all other claims. |
| `shape-resize` | Codex macOS, continuous-mac | T24 selected drawn-shape endpoint/corner resize handles; app graph_view/actions/shape edits, interaction safety and focused tests | claimed 2026-11-15; worktree `.claude/worktrees/shape-resize`; no clipboard, slope-tool behavior, file-format/model changes or other sessions’ scope |
| `stats-origin` | Claude Windows, "t14-origin-checks" | T14 item 6: Origin measurements of cross tabulation and Dunnett (statistics.md 15.3), distribution fit, probability and Q-Q plots (16.4); calc crosstab/dunnett/distfit, core analysis/dist.rs and the crosstab/ANOVA runners, their option dialogs, Origin fixtures | claimed 2026-11-14 night; continues 318653a1's scratch/stats-origin probes; no vartest, stats-multi, print, recovery, MCP attach, Go To, symbol gaps or other sessions' scope |
| `deconv-stft` | Claude macOS, "violin" | T57 Deconvolution and STFT: calc signal/new stft module, core analysis tools, Signal Processing menu and dialogs, CLI/MCP analyze, analysis-tools.md 18 | claimed 2026-11-15; no Convolution/FFT/IIR/FIR result changes, no matrices, plot or other sessions' scopes |
| `greyed-2-abdehik` | Claude macOS, "mac-multipdf" | T63 groups A (toolbar plot buttons), B (step/spline templates), D (Open Excel: reassign to B14), E (Graph as Image / Multi-Page PDF with no graph active), H (Row Numbers/Uniform/Normal fills), I (X/Y Axis Tick Labels), K (column-header Frequency Count) | claimed 2026-11-15; only these groups' dispatch/state/tests in menus.rs and toolbar.rs (not the Format bar, T58), greyed-items.md; no G (shape ordering), J (layer contents, T60/T61) or other sessions' scopes |
| `hist-prob` | Claude Windows, session fe4b37eb | B4 slice as new brief T85: Plot > Statistics > Histogram + Probabilities (histogram with the cumulative probability curve on a right axis); Origin 2016 measurements, core template/plot, menu, CLI, tests | claimed 2026-11-16; reuses own `.claude/worktrees/merge-graphs` + `target/wt-merge-graphs`; no other B4 plots (T83 interval/Pareto is done) or other sessions' scopes |
| `gadgets-1` | Claude Windows, "continue all tasks" | B9 slice 1 as new brief T101: Gadgets > Statistics, Quick Fit, Differentiate, Interpolate, Vertical Cursor on the Integrate gadget's ROI framework; new app gadget modules, small graph_view.rs hooks, Gadgets menu | claimed 2026-11-16; no other gadgets, shape-resize, T58/T59, multi-Y legends |
| `worktree-agent-a79f85b1afb5f0034` | `originrust-3a` | Matrices, heat maps, contour plots, `sheaf matrix` CLI (`7994c3b`..`8c21175`, 48 files, 10k lines) | 4 commits, local only, based on `148e9b1`, not merged; session offline since 2026-11-12 19:00 |

## Verification and completion

For implementation work, run from the task worktree:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 tools/greyed-count.py --check
```

Use a distinct `CARGO_TARGET_DIR=target/<task>` and `CARGO_PROFILE_DEV_DEBUG=0`. Check free space
first (`df -h .` on macOS/Linux; `df -h /c` on Windows Git Bash). App changes also need a focused UI
regression and a screenshot/export check; numeric changes need reference fixtures. The required
Rust checks also apply before merging this planning update under the current task workflow.

Before merging, fetch again and reconcile new `main` commits and claims. Delete enabled items
from the inventory, run `python3 tools/greyed-count.py --write`, update brief/index status and
roadmap/parity rows, and release only your own claim in the same merge. Never mark a feature done
just because its brief was written.

## Local resources and unresolved owner input

`re-workspace/scratch/peer-notes.md` and raw Origin outputs are local and ignored. Read its last
40 lines if present. The owner's ignored `.opj`/`.bin` files are read-only inputs. Use
`tools/origin-ref/README.md` and the global Origin lock for reference measurements. No Ghidra
project or saved REA evidence bundle is recorded.

The owner still has a real-data click-through and example-figure preferences to provide; these
are optional inputs, not blockers for T58–T63. Do not delete `chi-text-real` without the owner's
answer to the existing question (it contains a few real charge/discharge numbers). Full wording
and the list of pending Origin measurements are in the archived handoff's last two sections.
