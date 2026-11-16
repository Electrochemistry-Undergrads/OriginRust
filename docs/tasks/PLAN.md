# Plan: closing the "Not available in Sheaf yet" list

Written 2026-11-15 at the owner's request, against `main` at `dec7374` (documentation recovery baseline). This is the order in which
agents work through the 453 greyed items of [greyed-items.md](greyed-items.md) and the other open
work. Read [HANDOFF.md](../../HANDOFF.md) first, then this file, then **one** brief.

## The owner's priorities

1. **Basic formatting**: bold, italic, underline, superscript, subscript, Greek, font, size and
   colour, from the Format toolbar, on graph text first and on worksheet cells second.
2. **Stacked and multi-layer graphs**: build a stacked figure from graphs that already exist, add and
   arrange layers, more than two Y axes, and the stack and offset items that are still grey.
3. Everything else, in the order below: cheap items first, then what an electrochemistry figure or
   analysis needs most often, then rare features. 3D graphs and scripting come last. All T58–T63 rows below are **open plans**, not completed features.

Waves 1 and 2 are the owner's priorities. **Do not start new work from wave 4 or later while a wave
1 or wave 2 brief is unclaimed and you are able to do it.**

## How an agent goes through the plan

1. **Sync.** `git fetch`, read `HANDOFF.md` (rules, "In flight" table) and the last 40 lines of
   `re-workspace/scratch/peer-notes.md` if the machine has it.
2. **Pick** the first package in the wave tables below that is unclaimed and that you can do (some
   steps need Origin on the Windows machine; a session without Origin builds from the brief and
   marks its choices "unmeasured"). One package per session at a time.
3. **Brief.** A `T` package has a brief: read it. A `B` package has none: write one first, in the
   format of [README.md](README.md) "Brief format", with the next free `T` number; add it to the
   README index and replace the `B` tag in greyed-items.md and in this file with the new number.
   Split a large `B` package into several briefs when one would take more than a few days.
4. **Claim** on `main` as HANDOFF "Standing rules" says (docs-only commit, pushed, before
   implementation docs/specs or code). Preserve other sessions' claims and name any group/slice split.
5. **Check the list.** Grep each item's label in `crates/app/src/menus.rs`, `toolbar.rs` and
   `panels/explorer.rs`. If an item is no longer grey, delete it from greyed-items.md and go on.
6. **Build in this order**, one commit per slice, each with tests:
   model (`crates/core`) → drawing (`crates/plot`) → window and menu (`crates/app`) → CLI and MCP
   (`crates/ops`, `crates/cli`, `crates/mcp`) when the feature makes sense headless → files
   (`.sheaf` saves it; `.opj` writes it or the save warns, ADR 0008) → spec section and parity row.
7. **Enable the items.** Replace `grey(...)`/`greys`/`grey_sub`/`g(...)` with a live item, and
   delete the item from greyed-items.md in the same commit. Enable an item only when it does what
   Origin's item does; a partly built window stays grey or opens with the unbuilt controls disabled.
8. **Verify and merge** as [README.md](README.md) steps 6 and 7 say (fmt, strict Clippy, workspace
   tests; merge `main` in first). Then run `python3 tools/greyed-count.py --write`, update the counts in
   greyed-items.md, tick the package below, update the brief's Status and the README index, and
   release the claim in the same push.
9. **Go to step 1.** Leftovers that you did not build go into the brief's Status line, not into
   HANDOFF.

### Working in parallel

Several sessions share the repository. Packages in the same wave can run at the same time when
their files do not overlap; the "Main files" column shows where each one works.

- **Hot files** that almost every package touches: `crates/app/src/menus.rs`, `toolbar.rs`,
  `actions.rs`, `crates/core/src/graph.rs`, `HANDOFF.md`, `docs/tasks/README.md`,
  `docs/tasks/greyed-items.md`. Keep edits there small and local to your items, put new code in new
  modules, and merge `main` into your branch before every push.
- **T63 (wire-ups) is the parallel lane**: any session may take a group of its items while another
  session works on wave 1 or 2. Claim the group, not the whole brief.
- Two sessions must not change the same dialog or the same model type at once. T58 and T59 share
  the Format toolbar: T58 owns `toolbar.rs`'s Format bar and adds the dispatch point; T59 only adds
  its worksheet target behind it. T60 and T61 share layer code: T61 owns adding and deleting layers,
  T60 owns arranging them.

## Wave 1: basic formatting (owner's priority 1)

| ✓ | Package | What | Size | Origin needed | Greyed items | Main files |
|---|---|---|---|---|---|---|
| [ ] | [T58](T58-graph-text-formatting.md) | Format toolbar on graph text; underline, strikethrough and colour and font escapes; formatting buttons and preview in the text window; Axis Titles menu items | L | for the toolbar's exact behaviour; build first without | 15 | `plot/markup.rs`, `plot/text.rs`, backends, `app/toolbar.rs`, `dialogs/text_object.rs` |
| [ ] | [T59](T59-worksheet-cell-formatting.md) | Format Cells: font style, colours and alignment of worksheet cells; Format toolbar on a cell selection | L | for the Format Cells window | 2 | `core/worksheet.rs` (new `cellformat.rs`), `app/grid.rs`, new dialog |

T58 goes first (it builds the toolbar's dispatch); T59 can start on its model and dialog at once.

## Wave 2: stacked and multi-layer graphs (owner's priority 2)

| ✓ | Package | What | Size | Origin needed | Greyed items | Main files |
|---|---|---|---|---|---|---|
| [x] | [T60](T60-merge-graphs-layer-management.md) | Merge Graph Windows, Layer Management (arrange, size, link), Fit Layer to Graph, Fit Page to Layers | L | for both windows | 0 | `core/panels.rs`, new `core/merge.rs`, new dialogs |
| [ ] | [T61](T61-layers-and-multi-y.md) | Add Layer buttons, inset, layer icon menu, Layer Properties, 3Ys/4Ys templates, Multiple Y Axes | L | for templates' placement | 20 | `core/graph.rs` (layers), `core/template.rs`, `app/graph_view.rs` |
| [ ] | [T62](T62-stack-offset-leftovers.md) | Offset Grouped Data in Layer menu (with Cumulative), Stacked Histograms, waterfall colour mapping | M | for Stacked Histograms and waterfall colours | 8 | `core/stack.rs`, `plot/stack.rs`, `core/template.rs` |

What already works and must not be rebuilt: Stacked Lines by Y Offsets and Waterfall (graphing 13,
ADR 0010), Plot > Multi-Panel > 2/4/9 Panel, Stack... and Multiple Panels by Label (graphing 15),
Double-Y on linked layers (ADR 0009), stacked columns, bars and areas (graphing 14.12, 14.14).

## Wave 3: items whose feature already exists (parallel lane)

| ✓ | Package | What | Size | Origin needed | Greyed items | Main files |
|---|---|---|---|---|---|---|
| [ ] | [T63](T63-greyed-items-with-code-2.md) | Second round of T01: toolbar plot buttons, step lines, Probability and Q-Q plots in the Plot menu, Open Excel, export from any window, Duplicate, Fill Column With, axis tick label items, Add Plot to Layer | M | no | 31 | `app/menus.rs`, `app/toolbar.rs` |

## Wave 4: everyday graph editing

Backlog packages: write the brief first (step 3 above).

| ✓ | Package | What | Size | Greyed items |
|---|---|---|---|---|
| [ ] | B1 | Graph view: zoom in/out, whole page, pan, view modes, show/hide toggles, full screen | M | 27 |
| [ ] | B2 | Graph objects: cut/copy/paste, Copy/Paste Format, align, same size, group, order across object kinds, snap to grid | L | 51 |
| [ ] | B3 | Graph additions: function graph, straight line, table, colour scale, legend title, Change X/Y, Data menu display range and markers | L | 28 |
| [ ] | B4 | More 2D plot types (continues T12): symbol and segment variants, statistics plots, stock, vector, wind rose, Piper, Smith | L | 30 |

- **B1.** The graph view draws the page to fit the window. Add a view transform (zoom factor and
  scroll offset) in `crates/app/src/graph_view.rs` so hit-testing (`sheaf_plot::hit`) still works,
  then the Zoom In/Out tools, Whole Page, mouse-wheel zoom and Pan. Origin's four view modes (Print,
  Page, Window, Draft) and the View > Show toggles need a measurement of what each changes. Do the
  zoom and pan first; they are what a person misses.
- **B2.** Continues T24 (drawn shapes; ordering inside one layer and resize handles are done or in
  flight). Needs one ordering for all objects of a page (labels, legend, shapes, axis titles), an
  object clipboard, the nine Copy Format choices with Paste Format, and the Object Edit toolbar on
  a multi-selection (the view has single selection today: `selected: Option<(usize, ObjectKind)>`).
  Split it: (a) multi-selection and align/same size, (b) cut/copy/paste, (c) copy/paste format,
  (d) grouping and ordering across kinds.
- **B3.** Small independent tools; split freely. Add Straight Line and Add Function Graph reuse
  drawn shapes and the fit library's functions (`sheaf simulate`, T47). Display range and markers
  are T48's leftovers (graphing 20.4). New Color Scale and Add Bubble Scale reuse the colour scale
  that hexbin and parallel plots draw.
- **B4.** One commit per type as in T12. Order by use in the owner's field: Vertical Drop Line,
  Column + Label, Grouped Columns (indexed data), Grouped Box Charts, Interval Plot, Histogram +
  Probabilities, Marginal Histograms and Box Charts (two extra layers: do after T61), Pareto, QC
  chart, Scatter Matrix, 2D Kernel Density, vectors, stock charts, wind rose, Piper, Smith, Zoom.
  Line Series and 2/3 Point Segment are line-connection variants: check `Connect` first.

## Wave 5: everyday worksheet work

| ✓ | Package | What | Size | Greyed items |
|---|---|---|---|---|
| [ ] | B5 | Worksheet editing: copy variants, Paste Transpose/Link/Special, cell links, Set As Begin/End, fill with a set of numbers/dates/text, sampling interval, Copy Columns to, Swap Columns | L | 33 |
| [ ] | B6 | Worksheet view and sheets: header and grid toggles, row label rows, sheet-tab items, Split, objects in cells (graphs, images, sparklines, variables), Merge cells | L | 31 |
| [ ] | B7 | Worksheet tools: rest of Conditional Formatting (T35 leftovers), Pivot Table, Find in Project | M | 8 |

- **B5.** Paste Transpose, the Copy variants, Swap Columns, the fill windows and Copy Columns to
  need no new model. Paste Link, Remove Links and cell links need a link model (a cell that shows
  another cell): write an ADR before building. Set As Begin/End is the worksheet's display range;
  it decides which rows plots and analysis use, so every reader of a column must honour it.
- **B6.** The toggles and sheet-tab items are small. Sparklines and graphs in cells need a cell
  that holds an object; do them together and last. Merge cells changes the grid's layout
  (`crates/app/src/grid.rs` draws one table with frozen overlays): measure Origin's rules first.
- **B7.** Conditional Heatmap and Highlight reuse `crates/core/src/condformat.rs`.

## Wave 6: analysis and statistics breadth

| ✓ | Package | What | Size | Greyed items |
|---|---|---|---|---|
| [ ] | [T57](T57-deconvolution-stft.md) | Deconvolution and STFT (claimed 2026-11-15) | M | 2 |
| [ ] | B8 | Analysis and statistics: wavelets, Multiple Peak Fit, batch peak analysis, implicit fit, Simulate Surface, Compare Datasets, XYZ Surface Area, partial correlation, three-way and repeated-measures ANOVA | L | 14 |
| [ ] | B9 | Gadgets on the graph (Statistics, Quick Fit, Quick Peaks, Differentiate, Interpolate, Intersect, FFT, Rise Time, Cluster, Vertical Cursor, ...) | L | 13 |
| [ ] | B10 | Most Recently Used lists in the Plot, Worksheet, Analysis and Statistics menus; Recent Exports | S | 6 |

- **B8.** One brief per tool family, each checked against Origin where it can be scripted and
  against scipy/statsmodels otherwise. Multiple Peak Fit first (peak fitting is daily work; reuse
  the Peak Analyzer's fit, T04).
- **B9.** The Integrate gadget exists (`crates/app/src/gadget.rs`): the region of interest, its
  menu and its live result. Turn it into a shared gadget frame, then add one gadget per commit;
  most reuse an existing analysis tool.
- **B10.** Reuse `crates/app/src/recent.rs` (T38).

## Wave 7: project, templates and settings

| ✓ | Package | What | Size | Greyed items |
|---|---|---|---|---|
| [ ] | [T56](T56-user-graph-templates.md) | Save Template As and user graph templates (open; unverified start on branch `user-templates`) | M | 1 |
| [ ] | B11 | Templates and themes: template library, From Template, analysis templates, Batch Processing, batch-plotting Duplicate, Theme Organizer, Save Format as Theme | L | 10 |
| [ ] | B12 | Project and window management: New Graph, save/duplicate project without data, Re-Import, worksheet as image, View Windows, Folders List, folder Properties, sample projects | M | 16 |
| [ ] | B13 | Tools: Options window, System Variables, Fitting Function Builder and Organizer, Import Filters Manager, project protection, audit log | L | 7 |

- **B11** depends on T56. Analysis templates depend on recalculating analysis results (T03).
- **B13.** Tools > Options is the home for settings that are scattered today (autosave T28, speed
  mode, defaults). The Fitting Function Builder writes user fit functions (`crates/fit/src/fdf.rs`).

## Wave 8: new window types

| ✓ | Package | What | Size | Greyed items |
|---|---|---|---|---|
| [ ] | B14 | Matrix windows, contour and heat-map plots, profiles, Convert to Matrix, 2D FFT; Notes, Layout, Image, Function Plot and Excel windows; the Image menu | L | 44 |

- A `Window` is only a workbook or a graph today (`crates/core/src/project.rs`). Each new window
  kind is its own brief, with an ADR when it changes the project file.
- **Matrices first, and preserve earlier work:** `originrust-3a` has an unpushed, unmerged
  matrix branch (HANDOFF "In flight"). Check recent peer notes, branch/worktree and remote edits;
  apply HANDOFF's 48-hour inactivity rule before taking over. Recover/reuse its work when available.
- Notes windows are the smallest and useful at once (also "Add Notes as Sheet" in B6).
- Analysis > Mathematics > Special Filter and Inverse are tagged here because Origin offers them
  for matrices as far as the documentation shows; check Origin's menu before starting.

## Wave 9: last

| ✓ | Package | What | Size | Greyed items |
|---|---|---|---|---|
| [ ] | B15 | 3D graphs: XYY, surfaces, symbol/bar/vector, 3D waterfall, Z axis, OpenGL settings | L | 21 |
| [ ] | B16 | Scripting: Command Window, Script Window, Code Builder, Worksheet Script, Programming Control, consoles, X-Function Builder, Button Edit Mode | L | 15 |

- **B15** is deferred behind the current formatting and stacked-figure priorities (T18 backlog).
- **B16** needs an ADR first (which language, how it reaches the document; T18 "embedded language").
  Sheaf already exposes its commands through the CLI and the MCP server (ADR 0011, 0013).

## Not planned (`N`, 20 items)

These stay grey until the owner asks for them: OLE and Office links (Insert New Object, Links,
Update Client, Add Word Bookmarks to Analysis Template), OriginLab's online services and help
(Apps on File Exchange, Use Online Help, Check for Updates, the Origin and Programming help items,
Web Links), Quick Help, Smart Hint Log, and the View > Menu and Format > Menu choices of menu sets.
Sheaf has no help files of its own yet; when it gets them, Help > Origin > Help can open them.
Graph > Fit Layer to Graph is here too (T60, 2026-11-15): in Origin 2016 it changed nothing measurable
on one- and two-layer graphs (graphing 15.7d); it waits for a case where Origin's item does something.

## Other open work and where it ranks

- **Claimed work** in the HANDOFF "In flight" table is finished by its owner; it does not wait for
  this plan.
- **Leftovers of finished briefs** (each brief's Status line, summarised in the README index) rank
  with wave 4 to 7 by area, except: leftovers that change how the owner's figures look (T13, T21)
  rank with wave 3.
- **Origin measurements of unmeasured features** (most features built on macOS on 2026-11-15) are
  for the Windows session with Origin, between packages; they never block a wave.
- **[Origin 2016 to 2026-2027 gaps](origin-2026-2027-gaps.md)** (features newer than Origin 2016
  with no greyed item) rank after wave 7 unless the owner asks for one.
- **T18 backlog** rows are now covered by B11 to B16; T18 keeps the importers (HDF5, NetCDF,
  JCAMP, Matlab, Thermo, ODBC), `.opju` and `.opj` reading gaps, which have no greyed item.

## Keeping the plan true

- Tick a package only when all its greyed items are enabled or moved, with reasons, to another
  package.
- When the owner changes the order, change the wave tables here and the "Current priorities"
  section of HANDOFF in the same commit.
- Counts come from `python3 tools/greyed-count.py --write`; use `--check` before merging. This
  validates documentation counts, not whether every disabled Rust command is inventoried.
- Historical status is preserved in the archived handoff; current claims and task briefs win over
  old completion notes. T63 candidates must match their entry point's semantics (especially Open
  Excel versus importing a worksheet); otherwise retag them instead of claiming parity.
