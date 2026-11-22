# Task briefs

Read [HANDOFF](../../HANDOFF.md), then [PLAN](PLAN.md) for the current order: **T58/T59
formatting**, **T60–T62 stacked figures/layers**, then later waves. [Greyed items](greyed-items.md)
assigns every listed disabled entry to a package; `python3 tools/greyed-count.py --check` detects
stale count tables. Writing a brief does not complete its feature.

Each `Txx-*.md` file is one self-contained piece of work: read HANDOFF.md, this file and **one**
brief, then start. Do not read `docs/audit/origin-differences-2026-11-12.md` or whole specs up front;
a brief names the exact spec section and audit letter to open only when you need detail.

## How to work a task (applies to every brief)

1. **Claim it publicly before work.** Fetch remote `main`, inspect its "In flight" table and
   `git branch -a`, and read the last 40 peer-note lines if present. Existing branches are leads:
   check their brief, recent edits and claims rather than assuming every old branch is active.
   Follow HANDOFF's 48-hour inactivity rule for takeovers. Publish task/group, branch, worktree,
   session/date and owned files in HANDOFF on `main` (docs-only commit, push after syncing), and
   in `re-workspace/scratch/peer-notes.md` (local only), before implementation docs/specs or code.
2. **Branch and worktree.** `git worktree add .claude/worktrees/<branch> -b <branch> main`. Build
   with your own folder: `CARGO_TARGET_DIR=target/wt-<branch> CARGO_PROFILE_DEV_DEBUG=0`. Check
   `df -h .` (macOS/Linux) or `df -h /c` (Windows Git Bash) first (LESSONS "Parallel build
   folders fill the disk").
3. **Find code by name, not line number.** Line numbers in the briefs and the audit drift. Use
   Grep for the function, enum or menu label given.
4. **Origin is the reference.** When unsure how something looks or behaves, measure Origin
   with `tools/origin-ref/` (see its README; scripts take the global lock
   `Global\SheafOriginReference`, one Origin at a time) and copy it. Record each measurement in the
   named spec section. Clean-room rule: copy behaviour and layout, never Origin's code, icons, help
   text or files. Origin outputs go in `re-workspace/scratch/<task>/` (local only).
5. **Tests.** Add tests that guard the behaviour (numbers from Origin become fixtures with
   tolerances, as in `crates/calc/tests/origin_*.rs`). App changes: a UI test in
   `crates/app/src/app/ui_tests.rs` and/or `cargo run -p sheaf-app -- FILE --show ITEM --screenshot
   out.png` (LESSONS "Look at the desktop app without a person").
6. **Checks before merge** (all must pass, from the worktree):
   ```
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   python3 tools/greyed-count.py --check
   ```
7. **Merge** one branch at a time: `git fetch`, merge `main` into the branch, re-run the checks, then
   merge into `main` and push. Clean only your task's build folder: Windows Git Bash
   `tools/clean-target.sh wt-<branch>`; macOS/Linux `cargo clean --target-dir target/wt-<branch>`.
   Tick `docs/ROADMAP.md` and the rows of `docs/spec/feature-parity.md` you closed (recount with
   `python3 tools/parity-count.py`). Remove/retag inventory entries, run
   `python3 tools/greyed-count.py --write`, update the brief/index **Status** and plan, and release
   only your own HANDOFF claim. Preserve other sessions' claims and results.
8. **Commits:** Conventional Commits subjects. **Never** add a `Co-Authored-By: Claude` or
   "Generated with Claude Code" line (owner rule); repeat this rule in every subagent prompt.
9. **Subagents:** follow HANDOFF's session-specific rules (Claude: Sonnet/Haiku; Codex:
   Terra/Luna). Give each its own worktree, build folder and named files; repeat the no-co-author
   rule. Coordinate hot files as PLAN describes.

## Brief format

Every brief has: **Status**, **Size** (S < 2 h, M half a day, L several days), **Goal**, **Origin
facts** (what is already measured, with the source), **Where** (files and symbols), **Steps**,
**Done when**, and **Don't** (traps). Keep it that way when you add a brief.

## Current priority briefs

All six are **open** (T72, measured, joins them). The older index below retains implementation history and partial leftovers.

| Brief | Wave | Size | Scope and dependency |
|---|---|---|---|
| [T58](T58-graph-text-formatting.md) | 1, P0 | L | Renderer and graph Format toolbar/shared dispatch published in slices; selected-object font toolbar has all 24 Classic and custom RGB colours, and shared label/title/legend selection editor has installed-family/paired-script insertion plus all 24 Classic and custom RGB colours; documented prime escapes render through the shared text path (7.11); graph font/fill/line and shared text-editor Custom RGB wells share a private profile pool with worksheet controls (7.12); remaining native measurements, special escape aliases and named/project palettes stay open |
| [T59](T59-worksheet-cell-formatting.md) | 1, P0 | L | Cell fonts, styles and colours, shared RGB wells, and Worksheet Properties fonts for Data, Column Labels, Long Name, Units and Comments (GUI 3). Left: Standard/header defaults, category colours and other controls, and vertical alignment pending a supported worksheet route. Worksheet text controls are B5 |
| [T60](T60-merge-graphs-layer-management.md) | 2, P0 | L | Merge Graph Windows (dialog, CLI, manual/Auto preview, source filters and grouping, panel labels and letter/number enumeration, extra grid layers, Common Scale), Layer Management (Arrange with Common Scale and extra grid layers, Size/Position, Link, Display Background Color), Fit Page to Layers. Left: source-name and Roman labels, Layer Management Axes and remaining Display controls, MCP. Border fill/outline need independent geometry and storage |
| [T61](T61-layers-and-multi-y.md) | 2, P0 | L | Nine Add Layer kinds, insets, Delete Layer, Layer Properties, displaced Y axes and Move to Layer, 3Ys/4Ys templates and Multiple Y Axes (graphing 21). Hide Layer and Hide Other Layers (21.13–14), New Layer (Axes) preset dialog (21.15), layer-icon switches (21.16) and Layer Management Add with scale selection and linked parent (21.17). Left: User Defined and Auto Offset in the preset dialog, Custom Formula and Discrete scales, and template legend spacing/membership |
| [T62](T62-stack-offset-leftovers.md) | 2, P0 | M | Done 2026-11-15 (unmeasured Sheaf rules, graphing 13.15-13.17): Offset Grouped Data menu with Cumulative, Stacked Histograms, waterfall Y/Z colour mapping |
| [T63](T63-greyed-items-with-code-2.md) | 3, parallel | M | Existing-feature wiring; F/G/J wired, H, A, B, I and K wired 2026-11-19 (workbook-model 13c, graphing "Step and spline line plots"), C deferred to B8/T14; D and E open (D belongs to B14; E needs Origin measured) |
| [T65](T65-graph-view-zoom-pan.md) | 4 (B1), P1 | M | Zoom In mode, Zoom Out, Whole Page and Zoom-Panning tool landed 2026-11-15; Standard toolbar zoom combo added 2026-11-18 (Scale In/Out and Zoom All: T69) |
| [T66](T66-graph-view-modes.md) | 4 (B1), P1 | M | Partly done: four view modes, Layer Icons, Active Layer Only, Labels, Data, Active Layer Indicator, Frame and Full Screen (graphing 22). Four Show items and Show Data Info remain |
| [T67](T67-add-straight-line.md) | 4 (B3), P1 | M | Done 2026-11-15: Add Straight Line (Origin's addline), dialog, CLI --vline/--hline; left: dragging (Moveable) |
| [T70](T70-axis-center-fit-page-to-printer.md) | 4 (B3), P1 | S | Done 2026-11-15: axis Center, Fit Page To Printer, Format > Fit Page To Layers... opens its window |
| [T71](T71-point-segments.md) | 4 (B4), P1 | S | Done 2026-11-15: Plot > Line + Symbol > 2 and 3 Point Segment (line connections 2 and 3) |
| [T73](T73-change-xy-ungroup.md) | 4 (B3), P1 | S | Done 2026-11-15: plot right-click Change X, Change Y, Ungroup as Origin 2016 |
| [T75](T75-scatter-central.md) | 4 (B4), P1 | S | Done 2026-11-16: Scatter Central (axes crossing in the middle, X axis offset) |
| [T79](T79-column-scatter.md) | 4 (B4), P1 | S | Done 2026-11-16: Column Scatter |
| [T82](T82-line-series.md) | 4 (B4), P1 | S | Done 2026-11-16: Line Series (one plot, k-point segments, helper sheet) |
| [T85](T85-histogram-probabilities.md) | 4 (B4), P1 | S | Histogram + Probabilities; done 2026-11-19 (graphing "Histogram + Probabilities"); left: Results Log summary |
| [T76](T76-grouped-indexed-plots.md) | Plot > Column/Bar/Pie > Grouped Columns (indexed data), Plot > Statistics > Grouped Box Charts (B4 slice) | M | Done 2026-11-16 (`grouped-indexed`, graphing 14.28, Origin docs only, unmeasured); left: dialog, raw-data statistics, stacking, table tick labels, more than two groups |
| [T83](T83-interval-and-pareto.md) | Plot > Statistics > Interval Plot and Pareto (B4 slice) | M | Done 2026-11-16 (`interval-pareto`, graphing 14.29, measured, matched by T316); left: dialogs, binned Pareto, groups, drop lines, count table |
| [T80](T80-color-and-bubble-scale.md) | 4 (B3), P2 | M | Done 2026-11-16: Graph > New Color Scale, legend Add Color Scale / Add Bubble Scale (layer scale objects, graphing 27); left: dragging, properties dialog, Map link |
| [T78](T78-recent-lists.md) | 4 (B10), P1 | S | Done 2026-11-16: Most Recently Used lists in Plot, Worksheet, Analysis, Statistics menus and Recent Exports (File > Export and graph window) |
| [T74](T74-fill-column-patterns.md) | Fill Column With: A set of Numbers, Date/Time values, Text & Numeric values (Origin patternN/D/T) | M | done 2026-11-16 (workbook-model 13c); left: Origin's window layout, text quoting |
| [T84](T84-sampling-interval.md) | Set Sampling Interval and Show X Column (Origin colint/colshowx) | M | done 2026-11-16 (workbook-model 13d); label row added by T94; left: reading the interval from `.opj`, analysis tools |
| [T86](T86-swap-columns-paste-transpose.md) | Swap Columns and Paste Transpose (Origin colswap, Paste Transpose) | S | done 2026-11-16 (workbook-model 13e); left: Paste Transpose measured in Origin |
| [T87](T87-set-as-begin-end.md) | Set As Begin, Set As End, Reset to Full Range (column data range) | M | done 2026-11-16 (workbook-model 13f); descriptive Statistics on Columns/Rows readers added as T126/T127; other analysis readers remain |
| [T126](T126-descriptive-statistics-data-range.md) | Descriptive Statistics on Columns respects saved Begin/End ranges (T87 follow-up) | S | done 2026-11-17 (workbook-model13f.1); five Core/two menu-history UI regressions, full4,458 tests and fresh saved desktop result/Undo pass; other analysis readers remain open |
| [T127](T127-row-statistics-data-range.md) | Statistics on Rows respects each selected column's saved Begin/End range (T87 follow-up) | S | done 2026-11-17 (workbook-model13f.2); four Core/two menu-history UI regressions, full4,469 tests and fresh saved desktop/Undo pass; other analysis readers remain open |
| [T128](T128-multiple-peak-fit.md) | Analysis > Peaks and Baseline > Multiple Peak Fit (B8 slice) | M | done 2026-11-17 (analysis-tools 11.3), not measured; left: Open NLFit, Use Peak Marker, table on graph |
| [T88](T88-copy-variants.md) | Edit > Copy: Copy formula, Copy label rows, Copy formula and label rows, Copy All | S | done 2026-11-16 (workbook-model 13.4c); left: user parameter rows in Origin's text |
| [T89](T89-copy-columns-to.md) | Copy Columns to... (Origin colcopy) | S | copy dialog landed 2026-11-16; Ignore Hidden Rows added 2026-11-18 (workbook-model 13g); system parameter rows remain open |
| [T90](T90-sheet-tab-names.md) | Sheet tab: Duplicate Without Data, Sheet Name to Show, Name and Comments | M | names and duplication landed 2026-11-16; sheet comments added 2026-11-18 (workbook-model 13h) |
| [T91](T91-simulate-surface-compare-implicit-area.md) | Simulate Surface, Compare Datasets, Nonlinear Implicit Curve Fit, XYZ Surface Area (B8 slice) | L | done 2026-11-16 (`fit-surface-more`, Claude Windows); Origin measured (fitting.md 16-18, analysis-tools.md 18); all four items built; Compare Datasets' test and report layout are Sheaf's own (Origin's not measured), the implicit fit does Circle and Ellipse (PlaneMod not) |
| [T92](T92-worksheet-corner-menu.md) | Worksheet corner menu: label rows, column/row headers, grid lines | M | base done 2026-11-16; Column/Row Header switches added 2026-11-18 (workbook-model 13i); other label rows stay open |
| [T93](T93-sheet-navigate.md) | Sheet tab Navigate... | S | sheet activation, Comments column and Apply Reorder (workbook-model 13h); Actions menu with multi-select (T180), to-new-book moves and Exclude from Plotting (T184) landed 2026-11-19, Move to another workbook (T191) 2026-11-20 |
| [T94](T94-sampling-interval-row.md) | Sampling Interval label row (corner menu switch) | S | done 2026-11-18 (workbook-model 13i), read-only row with automatic visibility and saved per-sheet switch |
| [T95](T95-find-in-project.md) | Find in Project (F3) | M | done 2026-11-16; worksheet Comments, Within, folder metadata and Folder result column added 2026-11-18 (workbook-model 13j); left: Notes, folder info variables and double-click parity |
| [T96](T96-wavelets.md) | Wavelets: Decompose, Reconstruction, Denoise, Smooth, Continuous Wavelet (B8 slice) | L | done 2026-11-16 (`wavelets`, Claude Windows); Origin measured (analysis-tools.md 19, fixtures `tools/origin-ref/t96/t96-origin.json`); all five items built; Continuous Wavelet reproduces Origin's `cwt` to 1e-14 (Origin's pseudo-frequency output, coefficient matrix and image plot are not built, no script route); open corners: `sqtwolog` threshold, denoise boundary samples, Smooth for non-power-of-two lengths |
| [T97](T97-worksheet-as-image.md) | Export > Worksheet as Image... (Origin expWks) | S | active sheet export in PNG, PDF, SVG, JPEG, TIFF, BMP, GIF, TGA, PCX, PSD, EPS and EMF (workbook-model 13k); left: other vector types, multiple sheets, export controls and range-display measurement |
| [T98](T98-reimport.md) | File > Import > Re-Import Directly, Re-Import... (Origin reimport) | S | done 2026-11-16 (workbook-model 13l); left: stored import options |
| [T99](T99-batch-plotting.md) | Duplicate (Batch Plotting): With New Sheets/Books, With New Columns | M | done 2026-11-16 (graphing 28); left: Graph Short/Long Name fields |
| [T100](T100-folder-view-windows.md) | Project Explorer View Windows, Window > Folders, folder Properties | M | done 2026-11-16 (workbook-model 13m); left: title path, folder dates |
| [T102](T102-graph-themes.md) | Save Format as Theme, Theme Organizer (graph themes) | M | done 2026-11-16 (graphing 29); left: legend themes, system themes |
| [T103](T103-new-from-template.md) | File > New > From Template | S | done 2026-11-16 (graphing 24); left: workbook templates |
| [T104](T104-system-templates.md) | Plot > System Templates... | S | done 2026-11-16 (graphing 24); left: Plot Setup... |
| [T105](T105-fitting-function-organizer.md) | Tools > Fitting Function Organizer (browse the built-in fit functions) | S | done 2026-11-16 (fitting.md); left: user functions, sample curve |
| [T106](T106-import-filters-manager.md) | Tools > Import Filters Manager (saved import themes) | S | done 2026-11-16 (import-ascii.md); left: filter switches, Customize Menu |
| [T107](T107-vector-plots.md) | Plot > Specialized > Vector XYAM, Vector XYXY | M | plots, legend arrows, Vector tab styling and data selectors, and XYAM angle conventions and real-world magnitude (graphing 30) |
| [T108](T108-stock-plots.md) | Plot > Stock: High-Low-Close, Japanese Candlestick, OHLC Bar Chart | M | done 2026-11-16 (graphing 31); left: Plot Details price tabs, date X axis |
| [T109](T109-stock-volume-line.md) | Plot > Stock: OHLC-Volume, Line | M | done 2026-11-16 (graphing 31); left: exact X range, volume label format |
| [T110](T110-marginal-plots.md) | Plot > Statistics: Marginal Histograms, Marginal Box Charts | M | done 2026-11-16 (graphing 32); left: Min-Max whiskers, count-axis rounding |
| [T111](T111-qc-chart.md) | Plot > Statistics: QC (X bar R) Chart | M | done 2026-11-16 (graphing 33); left: hatched band, statistics table, Num Sigma |
| [T112](T112-zoom-plot.md) | Plot > Specialized > Zoom | M | done 2026-11-16 (graphing 34); left: curve over the region |
| [T113](T113-wind-rose.md) | Plot > Specialized > Wind Rose (Binned Data, Raw Data) | M | done 2026-11-16 (graphing 36), not measured (Origin COM unregistered); left: measure and match |
| [T114](T114-scatter-matrix.md) | Plot > Statistics > Scatter Matrix (B4 slice) | M | done 2026-11-16 (`scatter-matrix`, graphing 35, measured by T313); left: Mixed display, box/distribution diagonals, tick modes, ellipse, linear fit, grouping, listwise missing |
| [T115](T115-smith-chart.md) | Plot > Specialized > Smith Chart | M | done 2026-11-16 (graphing 37), not measured (Origin COM unregistered); left: measure and match |
| [T116](T116-kernel-density-2d.md) | Plot > Statistics > 2D Kernel Density | M | done 2026-11-16 (graphing 38); grid and bandwidths measured and matched by T289; left: contour drawing, points shown, method choice |
| [T117](T117-piper.md) | Plot > Specialized > Piper | M | done 2026-11-16 (graphing 39), not measured (Origin COM unregistered); left: measure and match |
| [T118](T118-data-markers.md) | Graph Data menu: Set Display Range, Reset to Full Range, Mark Data Range, Clear Data Markers (B3 slice) | M | done 2026-11-16 (graphing 40), not measured (Origin COM unregistered); left: measure and match |
| [T119](T119-add-function-graph.md) | Graph > Add Function Graph... (B3 slice) | S | done 2026-11-16 (graphing 41), not measured (Origin COM unregistered); left: measure and match |
| [T120](T120-conditional-heatmap.md) | Worksheet > Conditional Formatting > Conditional Heatmap... (B7 slice) | M | done 2026-11-17 (gui-look "Conditional Heatmap"), not measured; left: Other Colors mixing, Load Palette |
| [T121](T121-conditional-outliers-pareto.md) | Conditional Formatting > Conditional Outliers and Conditional Pareto (B7 slice) | S | done 2026-11-17 (gui-look "Conditional Pareto and Conditional Outliers"), not measured |
| [T122](T122-conditional-format-manager.md) | Conditional Formatting > Conditional Format Manager (B7 slice) | M | done 2026-11-17 (gui-look "Conditional Format Manager"), not measured; left: range editing, drag reorder |
| [T123](T123-conditional-highlight.md) | Conditional Formatting > Conditional Highlight (B7 slice) | M | done 2026-11-17 (gui-look "Conditional Highlight"), not measured; left: dates, statistics values, whole-row extend, text custom |
| [T124](T124-pivot-table.md) | Worksheet > Pivot Table (B7 slice) | M | done 2026-11-17 (workbook-model 13n), not measured; left: Others, extra values, user-parameter position |
| [T129](T129-slide-show-dependent-graphs.md) | Column header > Slide Show of Dependent Graphs (B5 slice) | S | done 2026-11-18 (gui-look "Slide Show of Dependent Graphs"), not measured; left: measure and match |
| [T130](T130-favorites.md) | Graph page > Add Shortcut to Favorites, Project Explorer Favorites folder (B12 slice) | S | done 2026-11-18 (gui-look "Favorites"), not measured; left: measure and match |
| [T131](T131-audit-log.md) | Tools > Protection > Audit Log (B13 slice) | S | done 2026-11-18 (gui-look "Audit Log"), not measured; left: Protect Project, measure and match |
| [T132](T132-fitting-function-builder.md) | Tools > Fitting Function Builder (B13 slice) | S | done 2026-11-18 (fitting "Fitting Function Builder"), not measured; left: Origin's wizard pages, measure and match |
| [T133](T133-sample-projects.md) | File / Help > Open Sample Projects (B12 slice) | S | done 2026-11-18 (gui-look "Sample Projects"); Sheaf's own synthetic samples |
| [T134](T134-options.md) | Tools > Options... (B13 slice: Open/Close autosave, File Locations) | S | done 2026-11-18 (gui-look "Options"), not measured; left: Origin's other tabs |
| [T135](T135-analysis-markers.md) | Graph Data > Analysis Markers (B3 slice) | S | done 2026-11-18 (graphing "Analysis markers"), not measured; left: Lock Position, measure and match |
| [T136](T136-graph-table.md) | Graph > New Table (B3 slice) | S | done 2026-11-18 (graphing "Tables"), not measured; left: table styles, cell formats, links |
| [T137](T137-label-order.md) | Text label Bring to the front / Push to the back (B2 slice) | S | done 2026-11-18 (graphing "Text label order"), not measured; left: order across object kinds |
| [T138](T138-rise-time-gadget.md) | Gadgets > Rise Time (B9 slice) | S | done 2026-11-18 (graphing "Rise Time gadget"), not measured; left: settings dialog, output, measure |
| [T139](T139-intersect-gadget.md) | Gadgets > Intersect (B9 slice) | S | done 2026-11-18 (graphing "Intersect gadget"), not measured; left: settings, output, measure |
| [T140](T140-quick-peaks-gadget.md) | Gadgets > Quick Peaks (B9 slice) | S | done 2026-11-18 (graphing "Quick Peaks gadget"), not measured; left: settings, output, measure |
| [T141](T141-sigmoid-gadget.md) | Gadgets > Quick Sigmoidal Fit (B9 slice) | S | done 2026-11-18 (graphing "Quick Sigmoidal Fit gadget"), not measured; left: other sigmoids, output, measure |
| [T142](T142-fft-gadget.md) | Gadgets > FFT (B9 slice) | S | done 2026-11-18 (graphing "FFT gadget"), not measured; left: preview window, output, measure |
| [T143](T143-statistics-quick-fit-gadgets.md) | Gadgets > Statistics, Quick Fit (B9 slice) | S | done 2026-11-18 (graphing "Statistics and Quick Fit gadgets"), not measured; left: other fit functions, output |
| [T144](T144-vertical-cursor.md) | Gadgets > Vertical Cursor (B9 slice) | S | done 2026-11-18 (graphing "Vertical Cursor gadget"), not measured; left: settings, output |
| [T145](T145-differentiate-interpolate-gadgets.md) | Gadgets > Differentiate, Interpolate (B9 slice) | S | done 2026-11-18 (graphing "Differentiate and Interpolate gadgets"), not measured; left: settings, previews, output |
| [T146](T146-cluster-gadget.md) | Gadgets > Cluster (B9 slice) | S | done 2026-11-19 (graphing "Cluster gadget"), not measured; left: settings, masking, output |
| [T147](T147-label-clipboard.md) | Graph text label Cut/Copy (B2 slice) | S | done 2026-11-19 (graphing 17.8), not measured; left: Edit menu and keyboard routes |
| [T148](T148-title-clipboard.md) | Axis title Cut/Copy (B2 slice) | S | done 2026-11-19 (graphing 17.8), not measured; left: title order |
| [T149](T149-graph-edit-clipboard.md) | Graph Edit menu Cut/Copy/Paste/Clear (B2 slice) | S | done 2026-11-19 (graphing 17.8), not measured |
| [T150](T150-object-properties.md) | Format > Object Properties... on a graph (B2 slice) | S | done 2026-11-19 (graphing), not measured; left: workbook objects |
| [T151](T151-prob-plot-options.md) | Plot > Statistics > Probability Plot, Q-Q Plot with plot_prob options (B8 slice) | M | done 2026-11-19 (statistics 16.2a), measured by T308 |
| [T152](T152-legend-order.md) | Legend Bring to front/back, front/back of data (B2 slice) | S | done 2026-11-19 (graphing "Legend order"), not measured; left: legend Cut/Copy |
| [T153](T153-title-order.md) | Axis title Bring to the front / Push to the back (B2 slice) | S | done 2026-11-19 (graphing "Legend order"), not measured |
| [T154](T154-notes-window.md) | Notes windows: File > New > Notes, New Notes (B14 slice) | M | done 2026-11-19 (notes-window spec), not measured; left: Add Notes as Sheet, .opj |
| [T237](T237-layout-page.md) | Layout pages: File > New > Layout, New Layout, Add Graph (B14 slice 1) | L | slices 1-12 done 2026-11-20 (layout-page spec), not measured; left: dashes, text toolbar, .opj |
| [T234](T234-matrix-profiles.md) | Plot > Profile: Contour and Image Profiles as row/column cuts (B14 slice) | S | done 2026-11-20 (matrices 7l), not measured |
| [T226](T226-ternary-contour.md) | Plot > Contour/Heat Map > Ternary Contour (B14 slice) | S | done 2026-11-20 (matrices 7k), not measured |
| [T225](T225-polar-contour.md) | Plot > Contour/Heat Map > Polar Contour theta(X) r(Y), r(X) theta(Y) (B14 slice) | S | done 2026-11-20 (matrices 7k), not measured |
| [T221](T221-matrix-surface-fit.md) | Nonlinear Surface Fit on a matrix through XYZ columns | S | done 2026-11-20 (matrices 7j), not measured |
| [T218](T218-image-rgb.md) | Colour pictures as RGB sheets; Image > Color to Gray, RGB Split (B14 slice) | S | done 2026-11-20 (matrices 7h/7i), not measured |
| [T213](T213-new-function-plot.md) | File > New > Function Plot (B14 slice) | S | done 2026-11-20 (graphing 41), not measured |
| [T210](T210-image-menu-2.md) | Image menu Levels, Subtract, Crop, Resize (B14 slice) | S | done 2026-11-20 (matrices 7i), not measured |
| [T207](T207-image-menu.md) | Image menu on a matrix: adjustments, math, binary, rotate/flip, spatial filters (B14 slice) | M | done 2026-11-20 (matrices 7i), not measured |
| [T204](T204-import-image.md) | File > Import > Image into a matrix book of gray levels (B14 slice) | S | done 2026-11-20 (matrices 7h), measured by T301 |
| [T201](T201-gray-scale-map.md) | Plot > Contour/Heat Map > Gray Scale Map (B14 slice) | S | done 2026-11-20 (matrices 7g), measured by T297 |
| [T199](T199-import-binary-2d.md) | File > Import > Binary 2D Array into a matrix (B14 slice) | S | done 2026-11-20 (matrices 7f), measured by T294 |
| [T196](T196-matrix-fft2.md) | 2D FFT and 2D IFFT of a matrix (B14 slice) | S | done 2026-11-20 (matrices 7e), not measured |
| [T193](T193-matrix-inverse.md) | Analysis > Mathematics > Inverse of a matrix (B14 slice) | S | done 2026-11-20 (matrices 7d), measured by T290 |
| [T192](T192-matrices.md) | Matrix books, heat maps, contour plots, `sheaf matrix` (B14 slice 1, port of the offline originrust-3a branch) | L | slices 1-4 done 2026-11-19 (matrices spec, Origin 2016 measured; app window, New Matrix, Matrix menu, Contour/Heat Map plots, Convert to Matrix); left: options windows, statistics, .opj |
| [T178](T178-legend-clipboard.md) | Legend right-click Cut/Copy; `\l(n)` and `%(n)` in free text labels (B2 slice) | S | done 2026-11-19 (graphing "Legend order"), not measured |
| [T162](T162-object-group.md) | Object Edit Group/Ungroup, dragging a selection or a group (B2 slice) | M | done 2026-11-19 (graphing "Aligning drawn objects and text"), not measured |
| [T159](T159-object-align.md) | Shift+click multi-selection, Object Edit Align and Make Same Width/Height (B2 slice) | M | done 2026-11-19 (graphing "Aligning drawn objects and text"), not measured; left: Group/Ungroup, distribute |
| [T155](T155-3d-scatter.md) | 3D Scatter, 3D Trajectory (B15 slice 1) | L | done 2026-11-19 (graphing "3D Scatter and 3D Trajectory"), not measured; left: rotation, Z axis dialog, other 3D plots |
| [T156](T156-3d-bars-errors.md) | 3D Bars, 3D Scatter + Error Bar (B15 slice 2) | M | done 2026-11-19 (graphing "3D Scatter and 3D Trajectory"), not measured |
| [T157](T157-3d-vectors.md) | 3D Vector XYZ XYZ, XYZ dXdYdZ (B15 slice 3) | M | done 2026-11-19 (graphing "3D Scatter and 3D Trajectory"), not measured |
| [T158](T158-3d-xyy.md) | 3D XYY: XYY 3D Bars, 3D Walls, 3D Waterfall (B15 slice 4) | M | done 2026-11-19 (graphing "3D Scatter and 3D Trajectory"), not measured |
| [T161](T161-3d-ribbons.md) | 3D XYY: 3D Ribbons (B15 slice 5) | S | done 2026-11-19 (graphing "3D Scatter and 3D Trajectory"), not measured |
| [T164](T164-3d-axes.md) | 3D Axis window: Format > Axes > Z Axis..., X/Y Axis... on a 3D layer (B15 slice 6) | S | done 2026-11-19 (graphing "3D Scatter and 3D Trajectory"), not measured; left: 3D axis titles, tick labels, scale types |
| [T166](T166-3d-surface.md) | 3D Surface from XYZ columns: Color Map, Color Fill, Wire Frame, Wire Surface (B15 slice 7) | M | done 2026-11-19 (graphing "3D Scatter and 3D Trajectory"), not measured; left: matrices, matrices |
| [T168](T168-3d-projection.md) | 3D Surface: Colormap Surface with Projection (B15 slice 8) | S | done 2026-11-19 (graphing "3D Scatter and 3D Trajectory"), not measured |
| [T170](T170-3d-constant-base.md) | 3D Surface: X Constant with Base, Y Constant with Base (B15 slice 9) | S | done 2026-11-19 (graphing "3D Scatter and 3D Trajectory"), not measured |
| [T172](T172-3d-view.md) | 3D view Turn and Tilt in the 3D Axis window (B15 follow-up) | S | done 2026-11-19 (graphing "3D Scatter and 3D Trajectory"), not measured; left: mouse rotation |
| [T173](T173-3d-ternary-surface.md) | 3D Surface: 3D Ternary Colormap Surface (B15 slice 10) | M | done 2026-11-19 (graphing "3D Scatter and 3D Trajectory"), not measured; 3D Surface submenu fully live |
| [T175](T175-sparklines.md) | Worksheet Sparklines: column header Add or Update Sparklines..., corner Sparklines switch (B6 slice) | S | done 2026-11-19 (workbook-model 13i), not measured; left: Insert Sparklines in cells, the options dialog |
| [T186](T135-analysis-markers.md) | Graph analysis locks + Data > Lock Position: Layer Icon, First Point, Last Point (B3) | S | done 2026-11-20 (graphing "Analysis markers"), not measured; lock click menu (Recalculate, Go to Results) added 2026-11-20 as T195; left: Change Parameters |
| T189 | Gadgets > 2D Integrate... on heat map / contour layers (B9) | S | done 2026-11-20 (graphing "2D Integrate gadget"), not measured; New Output to a results workbook added as T198, Region > Circle as T201, FWHM as T203, base plane as T206; left: free-hand regions |
| T209 | Graph gadgets skip worksheet-filtered and masked rows (T10 remainder) | S | done 2026-11-20 (workbook-model 13.2a) |
| T212 | Table UI walkthrough coverage: row/column operations, moving columns, sheet switching on new and imported tables | S | done 2026-11-20 (known-issues; added sheets now get A(X), B(Y)) |
| T215 | Sheet control coverage: Insert/Duplicate/Delete with undo, paste across sheets, new and imported tables | S | done 2026-11-20 (known-issues) |
| T217 | Graph UI coverage from a brand-new table: plot, change plot type, add layer, rescale, undo | S | done 2026-11-20 (known-issues) |
| [T66](T66-graph-view-modes.md) (T177) | Graph View > Show > Labels and Data (T66 slice) | S | done 2026-11-19 (graphing 22.8), documentation contract, not measured; seven Show items left |
| [T160](T160-workbook-templates.md) | User workbook templates: Save Template As on a workbook, New > From Template (T56/T103 follow-up) | M | done 2026-11-19 (workbook-model 13o), not measured; left: analysis templates (B11), CLI/MCP |
| [T163](T163-analysis-templates.md) | Save Workbook As Analysis Template (B11 slice, on T160) | M | done 2026-11-19 (workbook-model 13o), not measured; left: Origin options, Batch Processing |
| [T165](T165-batch-processing.md) | File > Batch Processing... with a workbook or analysis template (B11 slice) | M | done 2026-11-19 (workbook-model 13p), not measured; left: summary sheet, Origin options |
| [T167](T167-legend-theme.md) | Graph legend Save Format as Theme... (last B11 item) | S | done 2026-11-19 (graphing 29), not measured |
| [T169](T169-batch-peaks.md) | Peak Analyzer Save as Theme, Batch Peak Analysis Using Theme... (last B8 item) | M | done 2026-11-19 (analysis-tools 11.4), not measured |
| [T171](T171-axis-paste-groups.md) | Axis Paste Format To: Tick Direction, Ticks, Line, Tick Labels, Title Styles, Colors, Thickness, Title Text (T72 slice) | S | done 2026-11-19 (graphing 26.13), not measured; left: Title Dimensions |
| [T174](T174-page-axis-paste.md) | Page Paste Format To > Bottom Axes / All X Axes / All Axes with an axis format (T72 slice) | S | done 2026-11-19 (graphing 26.14), not measured; left: first-level All |
| [T176](T176-copy-format-groups.md) | Copy Format: Symbol/Line/Fill, Text (Edit menu), Page Background (page menu) (T72 slice) | S | done 2026-11-19 (graphing 26.15), not measured; left: Legend Translation Mode |
| [T179](T179-legend-format-all.md) | Legend Copy Format / Paste Format To: Dimensions and All (T72 slice) | S | Shipped 2026-11-19. Origin 2016 format API measured 2026-11-21 (graphing 26.16). Live menu fields remain open. |
| [T182](T182-export-no-graph.md) | File > Export > Graphs / Graph as Image / Multi-Page PDF from a workbook or no window (T63 group E) | S | done 2026-11-19 (graphing 9.7a), not measured |
| [T183](T183-layer-icon-switches.md) | Layer icon menu: Show Active Layer Only, Set Active Layer By Layer Icon Only, Double click to open Plot Setup (T61 leftovers) | S | done 2026-11-19 (graphing 21.16), not measured |
| [T185](T185-page-break-lines.md) | Worksheet Page Break Preview Lines (View menu, corner menu) (B6 slice) | S | done 2026-11-19 (workbook-model 13i), not measured |
| [T187](T187-categories-row.md) | Worksheet corner Categories row (B6 slice) | S | done 2026-11-20 (workbook-model 13i), not measured |
| [T188](T188-notes-sheet.md) | Sheet tab Add Notes as Sheet (B6 slice) | S | done 2026-11-20 (notes-window spec), not measured |
| [T190](T190-graph-sheet.md) | Sheet tab Add Graph as Sheet... (B6 slice) | S | done 2026-11-20 (workbook-model 13i), not measured |
| [T194](T194-cell-graph.md) | Data cell Insert Graph... (B6 slice) | S | done 2026-11-20 (workbook-model 13i), not measured; left: row-number item, print/export |
| [T200](T200-cell-sparkline.md) | Data cell Insert Sparklines... (B6 slice) | S | done 2026-11-20 (workbook-model 13i), not measured |
| [T202](T202-cell-image.md) | Data cell Insert Images from Files... (PNG) (B6 slice) | S | done 2026-11-20 (workbook-model 13i), not measured; left: other formats |
| [T205](T205-cell-arrow.md) | Data cell Insert Arrow, row number Insert Graph... (B6 slice) | S | done 2026-11-20 (workbook-model 13i), not measured |
| [T208](T208-cell-rich-text.md) | Set Data Style > Rich Text (B5 slice) | S | done 2026-11-20 (gui-look Set Data Style), not measured; left: print/export, Wrap Text, Float |
| [T211](T211-cell-links.md) | Cell links: Paste Link, Paste Link Transpose, Remove Links (B5 slice, ADR 0014) | M | done 2026-11-20 (workbook-model 13j0), not measured; left: graph Paste Link, Paste Special |
| [T214](T214-cell-float.md) | Set Data Style > Float (B5 slice) | S | done 2026-11-20 (gui-look Set Data Style), not measured; left: print/export, Wrap Text |
| [T216](T216-graph-picture.md) | Graph page Insert Images From Files... (PNG picture objects) | M | done 2026-11-20 (graphing 17.9), not measured; left: other formats, EMF/EPS |
| [T220](T220-window-dependents.md) | Window Properties lists Dependents / plotted workbooks | S | done 2026-11-20 (ui-command-map Window Properties), not measured |
| [T224](T224-plot-by-group.md) | sheaf plot --group with XY templates: one curve per group value (plotgroup) | S | done 2026-11-20 (graphing 17.10), not measured; left: graph/layer grouping |
| [T228](T228-sampling-interval-check.md) | Set Sampling Interval: From X Column checks even spacing (vinc) | S | done 2026-11-20 (workbook-model 13d), not measured |
| [T231](T231-import-sparklines.md) | Import Options > Add Sparklines (No / Yes / if less than 50 columns), themes, sheaf import --sparklines | S | done 2026-11-20 (import-ascii 10), default No; left: auto width, write protection |
| [T232](T232-send-graphs-to-powerpoint.md) | Send Graphs to PowerPoint... and sheaf export -o .pptx: one slide per graph | S | done 2026-11-20 (graphing 17.11), not measured; left: slide templates, Send to Word |
| [T236](T236-format-clipboard-race.md) | Format clipboard file: retry an open or rename Windows refuses during a replace (flaky concurrency test) | S | done 2026-11-20 |
| [T239](T239-hour-minute-second.md) | Formula functions hour, minute, second of a date-time number | S | done 2026-11-20 (expression-language 7), not measured (Origin not installed) |
| [T240](T240-mcp-link-test-flakes.md) | MCP link tests wait for a queued call or a logged job instead of sleeping (load flakes) | S | done 2026-11-20, tests only |
| [T242](T242-send-graphs-to-word.md) | Send Graphs window's Word choice and sheaf export -o .docx: a graph per page | S | done 2026-11-20 (graphing 17.11), not measured |
| [T244](T244-statistics-percentiles.md) | Statistics on Columns: Percentiles (%) list, one column per p | S | done 2026-11-22 (analysis-tools), not measured |
| [T367](T367-date-time-parts-measured.md) | hour/minute/second measured in Origin; T262's date bounds re-checked | S | done 2026-11-22, test + docs |
| [T233](T233-batch-summary.md) | File > Batch Processing: Summary Sheet, Dataset Identifier, Delete Intermediate Books (a Batch Summary book stacking one sheet of every book) | S | done 2026-11-20 (workbook-model 13p), not measured; left: Origin's layout and identifier choices |
| [T235](T235-batch-cli.md) | sheaf batch: Batch Processing without a window (template, summary, --summary-csv), core Project::batch_process shared with the window | S | done 2026-11-20 (workbook-model 13p); MCP: T238 |
| [T238](T238-batch-mcp.md) | MCP batch_process: the shared batch run (books, summary, delete) on the session project, one undo step attached | S | done 2026-11-20 (mcp.md 5.1); left: read options |
| [T241](T241-error-bar-generation.md) | Graph > Add Error Bars: Percent of Data, Standard Deviation of Data x Scaling Factor, Square Root of Data into a new Y Error column | S | done 2026-11-20 (graphing 14.13a), not measured |
| [T243](T243-axis-at-position.md) | Axis Position "At Position =": an axis line at a value of the other axis (an axis at zero) | S | done 2026-11-20 (graphing 21.3); left: .opj posType, end arrows |
| [T245](T245-axis-arrows.md) | Axis Line and Ticks: Arrow at Begin / Arrow at End (filled arrowheads past the axis line ends) | S | done 2026-11-20 (graphing 21.3), sizes not measured |
| [T246](T246-axis-title-tokens.md) | %(?X) and %(?Y) inside axis titles are replaced when drawn; the .opj reader keeps text around them | S | done 2026-11-20 (graphing 3.4); left: token options, title rotation/offset |
| [T247](T247-export-selection.md) | Export Worksheet as ASCII: Selected cells only (expASC select) | S | done 2026-11-20 (import-ascii 13.1), select not measured |
| [T248](T248-parity-stale-rows.md) | Parity rows corrected: Project Explorer, Window properties and rename, Fill Column With (checked in the code) | S | done 2026-11-20, docs only |
| [T249](T249-window-comments.md) | Window Properties: Comments for every window (Explorer tooltip; .opj companion) | S | done 2026-11-20 (ui-command-map), not measured |
| [T250](T250-export-folder.md) | Export Graphs: All in Active Folder and All in Active Folder (Recursive) | S | done 2026-11-20 (graphing 9.7b) |
| [T251](T251-export-preview-tabs.md) | Export Graphs preview: Image and Graph tabs (the page with the exported area outlined) | S | done 2026-11-20 (graphing 9.7c) |
| [T252](T252-step-center.md) | Step Horizontal Center (code 13) and Step Vertical Center line connections; two stale parity rows corrected | S | done 2026-11-20, not measured |
| [T253](T253-anova-masks.md) | Bug: two-way, three-way and repeated-measures ANOVA counted masked cells and filtered rows | S | done 2026-11-20 |
| [T254](T254-anova-level-masks.md) | Bug: masked factor/subject cells and filtered rows in the ANOVAs' level columns | S | done 2026-11-20 |
| [T255](T255-group-masks.md) | Bug: masked group/censor/state cells in Kaplan-Meier, ROC and grouped summary statistics | S | done 2026-11-20 |
| [T256](T256-fit-masks.md) | Bug: Compare Datasets/Models, Rank Models, global fit, Multiple Linear Regression and the implicit fit counted masked cells | S | done 2026-11-20 |
| [T257](T257-arrange-icons.md) | Window > Arrange Icons (minimised windows lined up in window order) | S | done 2026-11-20 |
| [T258](T258-operand-masks.md) | Bug: masked reference/operand cells in Simple Curve Math and Simple Column Math | S | done 2026-11-20 |
| [T259](T259-mask-docs.md) | Parity 'Masking' row corrected (.opj masks are read and written since T10) | S | done 2026-11-20, docs only |
| [T260](T260-normalize-ref.md) | Normalize: match a statistic of a reference column, divide by a reference cell (Origin's ref, cell), window and CLI | M | done 2026-11-20, not measured |
| [T261](T261-tukey-tail.md) | Tukey p-values computed directly in the far tail (~1e-9 relative); Origin 2016 found less accurate (HANDOFF A1) | S | done 2026-11-20, measured |
| [T262](T262-date-parts.md) | year/month/day/weekday/hour/minute/second as Origin: -1 without a date or for an empty cell, Origin's date range and time rules (measured) | S | done 2026-11-20 |
| [T263](T263-probit-inc.md) | Probit axis: increment 3 as Origin stores it, ticks at every whole probit whatever it is (HANDOFF A4) | S | done 2026-11-20, measured |
| [T264](T264-formula-masks.md) | Formulas read masked cells as missing (Origin measured); fixtures for masked and empty rows in statistics | S | done 2026-11-20 |
| [T265](T265-numextract.md) | NumExtract() and NumBreak() string functions (Origin 2026 SR1) | S | done 2026-11-20, not measured |
| [T266](T266-axis-measure.md) | Axis At Position = measured in Origin (posType 2): outside the range the axis is drawn outside the frame (fixed); arrows not scriptable | S | done 2026-11-20 |
| [T267](T267-poisson-rate.md) | One- and two-sample Poisson rate tests (Origin 2026b): calc, analysis, CLI, dialogs | M | done 2026-11-20, not measured; left: Raw/Indexed input, plots, power |
| [T268](T268-step-measure.md) | Centred steps measured in Origin: codes 13 and 14 match T252; .opj reads/writes 14 | S | done 2026-11-20 |
| [T269](T269-normalize-measure.md) | Normalize ref/cell measured in Origin: scaling as built, type default min (now the default) | S | done 2026-11-20 |
| [T270](T270-run-chart.md) | Plot > Statistics > Run Chart (Origin 2025b/2026) | S | done 2026-11-21, not measured |
| [T271](T271-grouped-histogram.md) | Plot > Statistics > Grouped Histogram (Origin 2026) | S | done 2026-11-21, not measured |
| [T272](T272-custom-colors-race.md) | Custom colours file: retry while Windows says access denied (flaky test) | S | done 2026-11-21 |
| [T273](T273-export-select-measure.md) | Export of the selected cells measured in Origin (expASC select): matches T247 byte for byte | S | done 2026-11-21 |
| [T274](T274-layout-measure.md) | Layout page default size measured in Origin 2016 (768 x 576 pt); Add Graph placement not scriptable | S | done 2026-11-21, partly measured |
| [T275](T275-stem-and-leaf.md) | Stem-and-leaf plot (Origin 2026 app): Statistics > Descriptive Statistics, Count/Stem/Leaf sheet, leaf unit, split, cap, outliers | M | done 2026-11-21 |
| [T276](T276-pivot-measure.md) | Pivot Table measured against Origin 2016 wpivot: names, totals, sorting, normalize fixed | S | done 2026-11-21, measured |
| [T277](T277-same-sheet-in-folder.md) | Sheet tab: Activate This Sheet in All Books in Folder (Origin 2027), by sheet name | S | done 2026-11-21 |
| [T278](T278-image-measure.md) | Image menu measured against Origin 2016 (8-bit img* tools): Brightness as % of range, To Binary band | S | done 2026-11-21, partly measured |
| [T279](T279-stemleaf-cli.md) | Stem-and-leaf display from the command line (`sheaf analyze stemleaf`) and MCP | S | done 2026-11-21 |
| [T280](T280-gray-measure.md) | Color to Gray with Origin 2016's imgC2gray weights (2R+5G+B)/8 | S | done 2026-11-21, measured |
| [T281](T281-axis-token-options.md) | Axis-title token options (`%(?Y,@LL)` and the others) measured in Origin 2016 and copied | S | done 2026-11-21 |
| [T282](T282-resize-measure.md) | Image Resize: Nearest (Origin 2016 default) or Bilinear; Crop checked | S | done 2026-11-21, measured |
| [T283](T283-opj-window-comments.md) | Window comments read from and written to .opj (label after the long name), measured in Origin 2016 | S | done 2026-11-21 |
| [T284](T284-image-median.md) | Image > Spatial Filters > Median as Origin 2016 imgMedian | S | done 2026-11-21, measured |
| [T285](T285-image-edge.md) | Image Edge Detection as Origin 2016 imgEdge: binary map, Sobel/Prewitt, threshold | S | done 2026-11-21, measured |
| [T286](T286-explorer-multiselect.md) | Project Explorer: Ctrl+click, Shift+click and Ctrl+A highlight several windows; Delete, Show, Hide and drag act on them | S | done 2026-11-21 |
| [T287](T287-fft2-measure.md) | Matrix 2D FFT measured against Origin 2016 fft2 (numpy conventions match); Power sheet added | S | done 2026-11-21, measured |
| [T292](T292-deconv-stft-measure.md) | Deconvolution (Linear 2N-1 padding, Normalize default) and STFT defaults as Origin 2016 | S | done 2026-11-21, measured |
| [T296](T296-cv-charge.md) | CV Charge: anodic/cathodic sweeps, charge = integral I dE / scan rate (parity P0 gaps) | S | done 2026-11-21 |
| [T288](T288-explorer-search.md) | Project Explorer search box (name, long name, comments; Subfolders) | S | done 2026-11-21 |
| [T289](T289-kde2d-measure.md) | Measure 2D Kernel Density against Origin (32 x 32 grid, Rule of Thumb, Bivariate) | S | done 2026-11-21 (graphing 38) |
| [T290](T290-minverse-measure.md) | Measure Matrix Inverse against Origin (pseudoinverse, full-rank test, output book) | S | done 2026-11-21 (matrices 7d) |
| [T291](T291-axis-title-rotation.md) | Axis title Rotate (deg.) and Offset (pt), turned about its middle as measured in Origin 2016 | S | done 2026-11-21 |
| [T293](T293-font-underline.md) | Whole-font Underline beside Bold and Italic (drawn, font boxes, .opj as \u(..)) | S | done 2026-11-21 |
| [T294](T294-bin2d-measure.md) | Measure Binary 2D Array import against Origin (impBin2D options and defaults, rows from size, partial row, target) | S | done 2026-11-21 (matrices 7f) |
| [T295](T295-newbook-measure.md) | New workbook measured in Origin 2016: 32 rows shown, default column width 7 | S | done 2026-11-21 |
| [T298](T298-parity-formula-refs.md) | Stale parity row corrected: formula reference notation (cross-sheet, cell(), label reads exist) | S | done 2026-11-21 (docs only) |
| [T299](T299-import-comment-lines.md) | Text import: comment lines from .. to joined, 15-line label detection, as Origin 2016 impasc | S | done 2026-11-21, measured |
| [T297](T297-grayscale-measure.md) | Measure Gray Scale Map against Origin (8 x 6 bands, 32 grays, out-of-range boxes) | S | done 2026-11-21 (matrices 6, 7g) |
| [T300](T300-rnormalize-measure.md) | Normalize Columns writes what Origin's rnormalize writes (columns at the end, Normalized1.., method comments) | S | done 2026-11-21 |
| [T302](T302-interp-output.md) | Interpolate writes a Y column at the end of the sheet with the grid as sampling interval, as Origin's interp1xy | S | done 2026-11-21 |
| [T303](T303-opj-sampling.md) | Sampling intervals read from and written to .opj (column record 0x1D2/0x1DA), measured in Origin 2016 | S | done 2026-11-21 |
| [T304](T304-sampling-x.md) | A column's sampling interval is its X in analysis and every plot (T302 follow-up) | S | done 2026-11-21 |
| [T305](T305-add-columns-count.md) | Add New Columns... / Ctrl+D ask how many columns; the toolbar button adds one | S | done 2026-11-21 |
| [T306](T306-sheet-tab-drag.md) | Drag a sheet tab onto another to move the sheet (one undo step) | S | done 2026-11-21 |
| [T301](T301-image-import-measure.md) | Measure File > Import > Image against Origin (book names, ranges 0..n-1, gray values) | S | done 2026-11-21 (matrices 7h) |
| [T307](T307-before-formula-scripts.md) | Set Column Values: Before Formula Scripts with name = expression statements | M | done 2026-11-21 |
| [T308](T308-probplot-measure.md) | Measure Probability Plot against Origin (scores, estimates, reference line, band for every family) | S | done 2026-11-21 (statistics 16.2a) |
| [T309](T309-add-param-row.md) | Sheet corner menu: Add User Parameter Row... | S | done 2026-11-21 |
| [T310](T310-param-row-edit.md) | Rename and delete a user parameter row from its title | S | done 2026-11-21 |
| [T314](T314-sig-digits.md) | Significant and Scientific displays measured in Origin 2016 and copied (326 of 328 cells) | S | done 2026-11-21 |
| [T317](T317-scientific-auto.md) | Scientific with default digits as Origin shows it (NumberFormat::ScientificAuto) | S | done 2026-11-21 |
| [T321](T321-reducedup-sheet.md) | Reduce Duplicate X Data writes its columns at the end of the input sheet, as Origin's reducedup | S | done 2026-11-21 |
| [T322](T322-curvemath-measure.md) | Intersect writes into the input sheet as Origin; numbered labels keep their number on recalculation | S | done 2026-11-21 |
| [T325](T325-avecurves-turning.md) | Average Multiple Curves sorts a turning X and averages equal X, as Origin's avecurves | S | done 2026-11-21 |
| [T326](T326-trace-output.md) | Trace Interpolation writes InterpolatedX1/Y1 at the end of the input sheet, as Origin's interp1trace | S | done 2026-11-21 |
| [T329](T329-adr5-origin-placement.md) | ADR 0005 addendum: tools that write at the end of the sheet as Origin; owner decision on Smooth/Differentiate | S | done 2026-11-21 (docs only) |
| [T330](T330-opj-group-subject.md) | Group and Subject columns read from and written to .opj (roles 7 and 8), measured in Origin 2016 | S | done 2026-11-21 |
| [T334](T334-opj-width.md) | Column widths read from and written to .opj (0x4A in tenths), measured in Origin 2016 | S | done 2026-11-21 |
| [T337](T337-opj-numfmt.md) | Numeric display formats read from and written to .opj (0x1E/0x1F, Numeric cells), measured in Origin 2016 | S | done 2026-11-21 |
| [T343](T343-opj-sheet-longname.md) | Sheet long names read from and written to .opj (__LONGNAME annotation), measured in Origin 2016 | S | done 2026-11-21 |
| [T345](T345-parity-recalc-links.md) | Stale parity row corrected: Recalculate links 15 tools, not 3 | S | done 2026-11-21 (docs only) |
| [T311](T311-qqplot-measure.md) | Measure Q-Q Plot against Origin (expected values, y = x line ends, band) | S | done 2026-11-21 (statistics 16.2a) |
| [T312](T312-import-nonnum.md) | Text import: text in a numeric column read as text, stop, new sheet or new columns (NonNum), as Origin 2016 | S | done 2026-11-21, measured |
| [T313](T313-scatter-matrix-measure.md) | Measure Scatter Matrix against Origin (page, grid, tick labels, titles, symbols, tick step) | S | done 2026-11-21 (graphing 35) |
| [T315](T315-import-two-digit-years.md) | Text import: two-digit years (M/d/yy, dd.MM.yy, dd-MMM-yy) with Origin 2016's 00-44 window | S | done 2026-11-21, measured |
| [T316](T316-interval-pareto-measure.md) | Copy Origin 2016's measured Pareto chart and interval plot title | S | done 2026-11-21 (graphing 14.29) |
| [T318](T318-baseline-als-dataset.md) | Peak Analyzer baselines Use Existing Dataset and Asymmetric Least Squares Smoothing (window, CLI) | M | done 2026-11-21; ALS not measured (Pro) |
| [T319](T319-pareto-binned.md) | Pareto Chart - Binned Data as Origin 2016 (ParetoBin sheet, sort, same chart) | S | done 2026-11-21 (graphing 14.29) |
| [T366](T366-distribution-overview.md) | Distribution Overview plot for right-censored life data (Origin 2026b) | S | done 2026-11-22 (statistics 37) |
| [T365](T365-process-capability.md) | Process capability, normal and nonnormal (Origin 2027 Quality app) | S | done 2026-11-22 (statistics 36) |
| [T364](T364-rare-event-charts.md) | G and T control charts for rare events (Origin 2027 Quality app) | S | done 2026-11-22 (statistics 35) |
| [T363](T363-autocorrelation.md) | Autocorrelation (Origin 2024b Time Series app): ACF, PACF and Ljung-Box per lag with graphs | S | done 2026-11-22 (statistics 34) |
| [T362](T362-manhattan-plot.md) | Manhattan plot (Origin 2017 app): -log10 p along grouped positions with a significance line | S | done 2026-11-22 (statistics 33) |
| [T361](T361-find-project-files.md) | File > Find in Project Files: search the saved projects of a folder, open a match (Origin 2026) | S | done 2026-11-22 (gui-look) |
| [T360](T360-repeat-analysis.md) | Analysis > Repeat Last Analysis: the last tool and options on the new selection (Origin 2026) | S | done 2026-11-22 (gui-look) |
| [T359](T359-column-across-sheets.md) | Column > Add Column to All Sheets: a matching column and formula in every sheet (Origin 2026b) | S | done 2026-11-22 (workbook-model 13q) |
| [T358](T358-time-series-pivot.md) | Time Series Pivot (Origin 2025): values per calendar period of their date-time X | S | done 2026-11-22 (statistics 32) |
| [T357](T357-integrate-gadget-prefs.md) | Integrate gadget Preferences: typed range, baseline, Mathematical/Absolute area | S | done 2026-11-22 (graphing gadgets) |
| [T356](T356-binned-scatter.md) | Binned scatter (Origin 2027): mean Y per bin of X, equal counts or equal width, sheet and graph | S | done 2026-11-22 (statistics 31) |
| [T355](T355-region-extract.md) | Extract Data in Region: the data inside a drawn rectangle or ellipse to a new workbook (Origin 2024b) | S | done 2026-11-22 (graphing 17.12) |
| [T353](T353-integrate-range-pick.md) | Integrate Ranges on a graph: each peak's limits picked with the Data Reader | S | done 2026-11-22 (analysis-tools 10) |
| [T351](T351-pa-anchor-pick.md) | Peak Analyzer: User Defined anchors picked on the graph with the Data Reader | S | done 2026-11-21 (analysis-tools 11.6a) |
| [T349](T349-cv-feet.md) | CV Charge on a graph: pick the peak's feet with the Data Reader, sloped baseline through them | S | done 2026-11-21 (analysis-tools 21) |
| [T348](T348-subtract-line-pick.md) | Subtract Straight Line on a graph: pick the two points with the Data Reader, the result drawn | S | done 2026-11-21 (analysis-tools 12) |
| [T346](T346-change-point.md) | Change point analysis (Origin 2019b app): mean, variance or both by PELT, Statistics menu window, `analyze changepoint` | M | done 2026-11-21 (statistics 30) |
| [T342](T342-quantile-regression.md) | Linear quantile regression (Origin 2019b app): exact fits, Statistics menu window, `analyze quantreg` | M | done 2026-11-21 (statistics 29) |
| [T339](T339-legend-options.md) | Legend `%(n, @option)` substitutions as Origin 2016 makes them (21 options measured) | S | done 2026-11-21 (graphing 6.1) |
| [T335](T335-title-drag.md) | Drag an axis title on the graph to move it (adds to its offset, one undo step) | S | done 2026-11-21 (graphing T291 paragraph) |
| [T332](T332-color-lists.md) | Origin 2016's 18 named colour lists in the Group tab and `plot --color-list` | S | done 2026-11-21 (graphing 1.5) |
| [T323](T323-symbol-shapes.md) | Symbols: Origin 2016's shapes 9-20, twelve interiors and a fill colour, measured and drawn; .opj bytes 0x17/0x18 | M | done 2026-11-21 (graphing 5.4) |
| [T320](T320-fit-covariance.md) | Fit report: Covariance Matrix and Correlation Matrix options (off as in Origin 2016), checked against Origin's errors and dependencies | S | done 2026-11-21 (fitting 4) |
| [T322](T322-funcgraph-measure.md) | Measure Graph > Add Function Graph against Origin (Function tab, Auto range, Plot Type) | S | done 2026-11-21 (graphing 41) |
| [T324](T324-windrose-measure.md) | Measure Wind Rose - Raw Data against Origin (speed bands, counts, colours) | S | done 2026-11-21 (graphing 36) |
| [T327](T327-windrose-binned.md) | Measure Wind Rose - Binned Data against Origin (22.5 degree wedges, colours, no window) | S | done 2026-11-21 (graphing 36) |
| [T328](T328-epw-measure.md) | Measure Peak Analyzer End Points Weighted (stretch rows on smooth data; anchor rule on noisy data not found) | S | measured 2026-11-21 (docs only) |
| [T331](T331-fit-auto-init.md) | Nonlinear fit: Auto Parameter Initialization on and off (window tick, `fit --no-auto-init`) | S | done 2026-11-21 |
| [T333](T333-peak-results-measure.md) | Measure Peak Analyzer Integrate Peaks output sheets and limits (local minima; left-edge rule open) | S | measured 2026-11-21 (docs only) |
| [T338](T338-formula-shorthand.md) | Bare column names, A1 and x/y in Set Column Values measured (none in Origin 2016); the error points to col(A) | S | done 2026-11-21, measured |
| [T341](T341-fourier-self-deconvolution.md) | Fourier Self-Deconvolution (Kauppinen): narrower bands to separate overlapping peaks (window, CLI) | M | done 2026-11-21 (Sheaf's own) |
| [T344](T344-fsd-peak-search.md) | Peak Analyzer Find Peaks on the Fourier self-deconvolved curve (hidden peaks), window and CLI | S | done 2026-11-21 (Sheaf's own) |
| [T347](T347-best-subset-selection.md) | Best Subset Selection: every predictor subset by R^2, adjusted R^2, Cp, AIC, BIC (window, CLI) | M | done 2026-11-21 (Sheaf's own) |
| [T352](T352-piecewise-peak-fitting.md) | Piecewise peak fitting: the Peak Analyzer per X region (own baseline, peaks, fit), window and CLI | S | done 2026-11-21 (Sheaf's own) |
| [T68](T68-drop-line-and-column-label.md) | 4 (B4 slice 1), P1 | S | Done 2026-11-15 (graphing 18.12): Vertical Drop Line and Column + Label as Origin's DROPLINE and ColumnLabel; left: the rest of B4 |
| [T69](T69-zoom-all-scale-tools.md) | 4 (B1), P1 | S | Done 2026-11-15: View > Zoom All (zoom to all objects, not a presentation), Tools-bar Scale In / Scale Out (measured in graphing 23); left: Ctrl+drag "Enlarged" graph, double-click restore, combo percentage, drawing objects outside the page |
| [T72](T72-copy-paste-format.md) | 4 (B2 slice 1), P1 | M | Partly done: graph and axis formats, legend Styles/Dimensions/All (T179, graphing 26.16), Apply Formats and private clipboard restart persistence. The spec records measured behavior, Sheaf field mappings and the remaining choices. |

## Index

[Origin 2016 to 2026–2027 functionality gaps](origin-2026-2027-gaps.md) lists newer capabilities
that are missing from Sheaf and have no existing brief or handoff entry.

| Brief | Area | Size | Status |
|---|---|---|---|
| [T01](T01-greyed-menus-with-code.md) | Enable greyed menu items whose feature already exists | M | done (`a2fb77b`); custom sort wired with selected-row bounds; Stack and label-group panel dialogs left |
| [T02](T02-menu-shortcut-differences.md) | Menu labels, shortcuts and context menus that differ from Origin | M | done (`bbdb60a`); unmeasured details listed in brief |
| [T03](T03-analysis-dialog-frame.md) | Analysis dialogs: Recalculate, Output, smoothing methods, pkFind options | M | done (`859c78e`); left: pkFind methods win/first/second/residue, linking more tools |
| [T04](T04-peak-analyzer-wizard.md) | Peak Analyzer as Origin's multi-page wizard | L | pages measured and the wizard built (analysis-tools 11.1); Voigt fit added and checked against Origin (11.2), a function per peak; left: Auto integration window rule, baseline goals and modes |
| [T05](T05-export-graphs-dialog.md) | Export Graphs dialog, raster formats, units, Ctrl+G, Copy Page | M | done (`c32e585`); TGA, PCX, flattened PSD and 256-color TIFF done; mono, 16-colour and 16-bit modes done (Origin's lists, dither, 5-5-5 layout); named export Dialog Themes/defaults done (graphing 9.8); Graph Theme left |
| [T06](T06-opj-remaining-graph-features.md) | `.opj` writer: X offsets, bar overlap, waterfall, date ticks, window places | L | X offsets, waterfall as offsets, month/day/year ticks, window places written (13.14); bar overlap, group colours, grouped error bars, hidden end label not found (need the GUI) |
| [T07](T07-worksheet-export-and-copy.md) | Worksheet export and clipboard as Origin (missing `--`, formats, label rows) | S | done (`826f390`); clipboard text round-trip fixed; Origin Copy defaults unmeasured |
| [T08](T08-formula-functions.md) | Missing Set Column Values functions in `sheaf-expr` | M | groups 1-3, 5 done (`04a8a80`); `sum`/`count` as Origin, local `now()`, strings and `cell()` dependencies done (`d8d1416`); `substitute`, text-function `$` suffix and built-in column labels done; `char`/`code` done; cross-sheet refs, parameter rows and Origin global/`@SD` defaults left; explicit `format` and direct-column Text defaults done |
| [T09](T09-set-values-dialog.md) | Set Column Values dialog in Origin's layout | M | done (`20c37e1`); scripts box, Set Multiple Columns left |
| [T10](T10-worksheet-operations.md) | Insert/delete rows, move/hide columns, mask, sort column, transpose | L | done; left: mask colour/condition/from graph, per-column and date/text filters, measured layouts of stack/unstack/split/reduce, Group/Subject file codes |
| [T11](T11-axis-scales-and-features.md) | More axis scales and axis-dialog features | L | done; several breaks with Origin's measured layout done (2026-11-15); left: Origin's tick-step change with breaks, reference lines |
| [T12](T12-plot-types.md) | Stacked/horizontal columns, histogram, box, pie, bubble, polar | L | items 1-6 done (polar, radar, ternary, 3D and B&W pie merged 2026-11-13); left: listed in the brief Status |
| [T13](T13-plot-details-and-axis-dialog-tabs.md) | Missing Plot Details tabs and axis-dialog controls | M | done; measured (graphing 18 to 18.5): Drop Lines, Label, Fill Area, Transparency, Background colour, Group tab, Offset, Display, Size/Speed, Link, Stack, page tabs, tree names, Layer Contents and Plot Setup windows, Workbook and `>>`; Sheaf's own and unmeasured (18.6): fill hatch/gradient/transparency, bar width, label column/rows/decimals/offsets, subgroups by size, background gradient and border, X/Y axes switches, page border, icons, comments and title choice; Offset multipliers, Data on Top of Axes, Skip Points, label White Out and Frame/Margin, symbol gaps, No Line Break, Front/Behind line order and Inclusive / Sidelines only / Sidelines and Base also done; left: other fill modes, several groups, Binned and speed mode (X:Y ratio done 2026-11-15), `.opj` writing of the new settings |
| [T14](T14-statistics-breadth.md) | Descriptive-statistics columns, outlier tests, more tests | M | items 1-4 done (`c44a1a6`); 5 (Holm; Dunnett not measurable) and power analysis merged (`d724c40`); 2D binning and discrete frequency done (statistics.md 13, 14); cross tabulation and Dunnett (one-way ANOVA) done, scipy-checked not Origin (statistics.md 15; merged from `stats-compare`); distribution fit, probability plot and Q-Q plot done (calc `distfit`, statistics.md 16; scipy-checked, not Origin); left: Origin measurements of cross tabulation, Dunnett, distribution fit and the probability plots (what Origin fits, plots and reports is unmeasured), PCA, ROC, survival (backlog T18) |
| [T15](T15-fitting-breadth.md) | Multi-dataset global fit, Simplex, surface fits, compare models | L | items 1-6 done (merged 2026-11-13); item 7 surface fits on XYZ columns done 2026-11-15 (fitting.md 14-15); Rank Models and linear fit with X error not compared with Origin; left: surface fits on matrices and their 3D plot |
| [T16](T16-numeric-differences.md) | Small numeric/behaviour differences (Mann-Whitney print, spline, error caps) | S | 1, 5 done, 4 investigated (`b967898`); 3 done (`d0dbb37`) (cap width 8 pt); 2 done (`94063b4`) (natural spline, 900 limit, B-spline and Bezier added); 6 done (Origin's automatic B-spline factor, analysis-tools 5) |
| [T17](T17-docs-cleanup.md) | Stale spec paragraphs and parity counts | S | done (item 3 with T20, `522c625`) |
| [T18](T18-backlog.md) | Large items not yet briefed (scripting, 3D, Options, importers, templates) | - | backlog |
| [T19](T19-import-and-column-dialogs.md) | ASCII import options, Import Wizard pages, Column Properties formats | M | items 1 and 3 done; delimiter sets, consecutive-run combining and row/column range-pattern selection with except-first batch scope added; seven Import Wizard pages measured and added (remaining controls in brief) |
| [T20](T20-automatic-axis-range.md) | Automatic axis range as Origin (8 % margin, step rule, log padding), Rescale Margin | M | done (`522c625`) |
| [T21](T21-graph-defaults.md) | Graph defaults as Origin: colour order, fonts, line widths, ticks, symbols, dashes | M | done; four stock legend anchors, major tick lengths in .opj and measured thin-width dash interpolation copied; round dash caps/backend rounding, other template legend placements and chosen minor tick lengths remain |
| [T22](T22-curve-mathematics.md) | Mathematics: Average Multiple Curves, Intersect, Polygon Area | M | done, merged (`734a1ff`); Origin details unmeasured (spec `analysis-curve-math.md`) |
| [T23](T23-data-manipulation-tools.md) | Subtract Straight Line, Reduce to Evenly Spaced X, Curve Translate X/Y, Median Filter | M | done, merged (`f66dbea`); Origin details unmeasured (spec section 12) |
| [T24](T24-drawn-graph-objects.md) | Drawn objects on graphs: line, arrow, rectangle, ellipse | M | done, merged (`542fef7`); within-layer shape front/back ordering and endpoint/corner resizing added (resizing merged 2026-11-19 from the idle `origin/shape-resize`); other object kinds/layers remain |
| [T25](T25-mcp-server.md) | MCP server (`sheaf-mcp`) so Claude and other AI tools can drive Sheaf | L | phase 1 done, merged 2026-11-14 (stdio server, shared `sheaf-ops`, ADR 0011, `docs/spec/mcp.md`); phase 2 (`--attach` to the open app) done 2026-11-15 (ADR 0013, mcp.md 11); `sheaf://app` current-window/selection context and owner Undo AI Changes also done; Windows endpoint acceptance and runtime DACL repair also done; `output: new_sheet` done; CHI protocol tests are documented in T25-protocol-validation; done since: CLI-option bridges for `plot` and `fit`, `analyze` reaches all 55 CLI analysis tools (CLI options shared through `sheaf_ops::args`) and prompts |
| [T26](T26-variance-proportion-tests.md) | Tests for variance and proportions (Hypothesis Testing) | M | done, merged 2026-11-15 (statistics.md 21): four tests, Fisher, power analysis, two-sample proportion measured; left: raw and indexed input forms, variance-dialog plots |
| [T27](T27-print.md) | Print, Print Preview, Page Setup (vector EMF/PDF path) | M | Windows graph, macOS graph panel and workbook printing done (graphing 19, 19.7, 19.8); left: Origin's printed worksheet layout and Print Preview window, range/selection printing, CLI workbook printing |
| [T28](T28-autosave-recovery.md) | Autosave to a recovery folder and crash recovery at start-up | M | done, merged 2026-11-14 (`docs/spec/autosave.md`); left: Origin's defaults unmeasured, backup-before-save switch, Tools > Options home |
| [T29](T29-multivariate-roc-survival.md) | PCA, k-means, ROC curve, Kaplan-Meier | L | done, merged 2026-11-15 (statistics.md 17-20): PCA (with Origin's four graphs), Kaplan-Meier (curve limits), k-means Origin-measured with Origin's report layouts; ROC measured against Origin 2016 (standard error, interval, p-value, cut-offs, statistics.md 19.5); left: ROC against Origin, PCA observation labels, Notes/Input Data report tables, Cox |
| [T30](T30-excel-import-export.md) | Excel import (xls, xlsx, xlsm) and xlsx export | L | done, merged 2026-11-14 (ADR 0012, `docs/spec/import-excel.md`); Origin's import measured and copied 2026-11-15 (import-excel.md 2); owner's workbooks checked; left: Origin's import window and export, `.xlsb`/`.ods` tests |
| [T31](T31-windows-executable-icon.md) | Embed the existing app icon in Windows executable resources | S | done; all seven built-PE icon frames match the source and Windows extracts both sizes |
| [T32](T32-multipage-pdf-export.md) | Several graphs in one multi-page PDF (Export Graphs, CLI, MCP) | M | done 2026-11-15 (graphing 9.7); left: Origin's label, file name and page sizing unmeasured |
| [T33](T33-lollipop-plot.md) | Lollipop plot: drop lines to the next data plot, Plus-Minus colour | M | done 2026-11-15 (graphing 18.8); left: Origin's defaults unmeasured; chord and rug plots |
| [T34](T34-frozen-panes.md) | Worksheet frozen panes (View: Freeze/Unfreeze Panes) | L | done 2026-11-15 (gui-look.md 3 "Frozen panes"); left: Origin's counting, limit, divider look and unfreeze rules unmeasured |
| [T35](T35-conditional-highlight.md) | Conditional formatting: highlight min/max, top/bottom | M | done 2026-11-15; rest of the submenu done as T120-T123 |
| [T36](T36-cell-notes.md) | Worksheet cell notes (add, hover, edit, delete) | M | done 2026-11-15 (`cell-notes`, macOS) |
| [T37](T37-violin-plot.md) | Violin plot (kernel density silhouettes per Y column, box/quartile/point variants) | M | done 2026-11-15 (graphing 14.22); left: Split Violin, grouped violins, violin settings (bandwidth, extend, scale), Origin's defaults unmeasured |
| [T38](T38-recent-files.md) | Recent Projects and Recent Imports (File menu lists that survive a restart) | M | done, merged 2026-11-15 (`docs/spec/recent-files.md`); left: Origin measurements, Recent Books/Graphs/Exports |
| [T39](T39-plot-sort-x.md) | Line plots connected in X order without sorting the sheet | M | done 2026-11-15 (graphing 18.9, `plot-sort-x`, macOS); left: Origin's effect on symbols, fills, missing values, ties and splines unmeasured |
| [T40](T40-graphical-summary.md) | Graphical Summary: statistics, CIs, Anderson-Darling and charts per variable/group | L | done 2026-11-15 (statistics.md 22); left: three other arrangements, Origin layout unmeasured |
| [T41](T41-rug-plot.md) | Rug plots: Axis dialog Rug tab and Scatter + Rug template | M | done 2026-11-15 (graphing 18.10); left: Origin defaults unmeasured, rug hit-testing |
| [T42](T42-window-switcher.md) | Window Switcher: Ctrl+Tab through child windows with previews | M | done 2026-11-15 (`window-switcher`, macOS) |
| [T43](T43-trendlines.md) | On-graph trendlines (linear, polynomial, power, log, exp, moving average) | M | done 2026-11-15 (graphing 18.11); left: Mini Toolbar, dragged label, Origin fit method/R-squared unmeasured |
| [T44](T44-ridgeline-plot.md) | Ridgeline chart: offset density curves per Y column | M | done 2026-11-15 (graphing 14.23); left: grouped data, jitter, Origin defaults unmeasured |
| [T45](T45-hexbin-plot.md) | Hexbin plot: hexagonal bins coloured by count | M | done 2026-11-15 (`hexbin`, macOS) |
| [T46](T46-import-themes.md) | Saved import settings: Dialog Theme row (`<last used>`, named themes) in the ASCII, CHI and Excel import windows, `sheaf import --theme` | M | done, merged 2026-11-15 (import-ascii.md 17); left: Origin measurements of the theme list and defaults |
| [T47](T47-simulate-curve.md) | Simulate Curve: a curve from any fit-library function with typed parameters, window, CLI and MCP | M | done, merged 2026-11-15 (fitting.md 13); Origin's window measured and copied 2026-11-15 (13.1); left: Noise Level, user-defined functions, Simulate Surface |
| [T48](T48-graph-data-tools.md) | Graph Data menu: Mask Data Points (rectangle), Remove Bad Data Points, Move Data Points | M | done, merged 2026-11-15 (graphing.md 20); Mask and Remove Bad measured in Origin 2016 and followed; left: Move Data Points unmeasured (synthetic input fails), other plot kinds, display range |
| [T49](T49-parallel-plot.md) | Parallel Plot and Parallel Index (parallel coordinates) | M | done 2026-11-15 (graphing 14.25, `parallel-plot`, macOS); left: Parallel Sets, curvature, colour map, axis reordering, Origin defaults unmeasured |
| [T50](T50-treemap.md) | Treemap: nested rectangles sized by value (six Origin layouts) | M | done 2026-11-15 (graphing 14.26, `treemap`, macOS); left: Origin defaults/colours/labels unmeasured, hover and drill-down, colour by value |
| [T51](T51-sunburst.md) | Sunburst plot: hierarchy as concentric rings coloured by value | M | done 2026-11-15 (graphing 14.27, `sunburst`, macOS); left: Origin defaults unmeasured, hover and Collapse/Expand drilling |
| [T52](T52-slope-annotation.md) | Slope annotation between two data points (per decade on log axes) | M | done 2026-11-15 (graphing 17.6, Tools > Slope, `--slope`, macOS); left: Origin's look and label unmeasured, label dragging, decimals |
| [T53](T53-set-multiple-columns.md) | Set Multiple Columns Values (Ctrl+Shift+Q): one formula over the selected columns with `j` | M | done, merged 2026-11-15 (expression-language 7); Origin's window and `j` measured 2026-11-15 (7.1); left: non-adjacent, locked and text columns in Origin |
| [T54](T54-save-window-as.md) | Save Window As (`.sheafbook`/`.sheafgraph` window files opened by appending) and Recent Books / Recent Graphs | M | done, merged 2026-11-15 (window-files.md); left: Origin measurements, Save Template As |
| [T55](T55-iir-fir-filters.md) | IIR Filter (Butterworth, Chebyshev I/II, elliptic) and FIR Filter (window, least squares) design and application | L | done, merged 2026-11-15 (analysis-tools 17, `iir-filter`/`fir-filter`, macOS); left: Origin measurements, SOS/ZPK/state-space/coefficient output sheets, preview panels, custom FIR window, `dfilterapply`; Bessel is not an Origin method |
| [T57](T57-deconvolution-stft.md) | Deconvolution and STFT (Signal Processing) | M | done 2026-11-19 (analysis-tools 20), not measured; STFT writes a worksheet, not a matrix |
| [T56](T56-user-graph-templates.md) | Save Template As and Plot > User Templates (a graph's look without data, applied to new columns; CLI `--template`) | M | done 2026-11-15 (graphing.md 24): Save Template As, Plot > User Templates, Manage, `sheaf plot -t NAME`; left: Origin measurements |
| [T64](T64-significance-plot.md) | Multiple comparisons plot with significance brackets | M | done 2026-11-15 (statistics 23, `significance-plot`, macOS); left: two-way ANOVA, paired and one-sample, Origin measurements, editable brackets |
| [T77](T77-new-graph-and-project-without-data.md) | File > New > Graph and Standard New Graph (empty graph from the default template), Save Project without Data... and Duplicate Project without Data (B12 slice) | M | done 2026-11-16 (`new-graph-nodata`, graphing.md 25, project-file-format.md); Origin behaviour of the two project items unmeasured |
| [T125](T125-drawn-shape-clipboard.md) | Drawn-shape Cut/Copy/Paste (B2 slice) | M | done 2026-11-19 (graphing 17.8): right-click Cut/Copy and page Paste at the clicked point, not measured; left: Edit menu/keyboard routes, several objects |
| [T72](T72-copy-paste-format.md) | Graph, axis and legend Copy Format and Paste Format To (B2 slice 1) | M | Partly done. Legend Styles, Dimensions and All are available. See graphing 26 for measurements and field mappings. |
| [T81](T81-partial-correlation-anova3-rmanova.md) | Partial Correlation Coefficient, Three-Way ANOVA, One-Way and Two-Way Repeated Measures ANOVA (B8 slice) | L | done 2026-11-16 (`partial-corr`, `anova3`, `rmanova`, statistics.md 24-26, Origin 2016 runs reproduced); left: Raw layout, missing values, two-way means comparisons, mean/plot graphs |

For every active claim (including matrices/contour/heatmaps), read the current
[HANDOFF table](../../HANDOFF.md). This index is not a second claims ledger. Older branches and
completion rows do not override that table or HANDOFF's inactivity/takeover policy.
