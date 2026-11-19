# Greyed items: what Sheaf does not have yet

Every Origin menu item Sheaf lacks is drawn greyed with the tooltip "Not available in Sheaf yet"
(`NOT_YET`; `grey`/`greys`/`grey_sub` in `crates/app/src/menus.rs`, `g(...)` in
`crates/app/src/toolbar.rs`, and `crates/app/src/panels/explorer.rs`). This file lists every one of
them, by menu, and tags each line with the package that will enable it. The packages and their order
are in [PLAN.md](PLAN.md).

Listed at commit `23ac847` (2026-11-15, `main` after T55) in the paused cloud session;
recounted against its package tags during recovery.
Formatting and layer gaps were checked in source during recovery; other labels retain the
original inventory baseline. This is not a new runtime audit. **241 greyed items: 158 in the menu bar, 18 on the toolbars, 65 in context menus.**

## How to read and keep this file

- A line is `` - `PACKAGE` Group: item · item [windows] ``. The tag in backticks is the package: a
  brief (`T58`), a backlog package of PLAN.md (`B4`), or `N` (not planned).
- A tag in square brackets at the end names the windows where the item is greyed when it is not on
  every menu bar: [Workbook], [Graph], [no window].
- Items greyed only because of the current state are left out (the current plot type under Change
  Plot to, Hide Other Layers with one layer, Go to Book with no workbook, "(none)" recent lists).
- **When you enable an item, delete it from its line here in the same commit** (delete the line when
  it is empty), then run `python3 tools/greyed-count.py --write`. It refreshes this table, the wave tables and
  the handoff totals. Use `--check` to detect stale counts before merging.
- When you grey a new item (a new Origin menu entry copied without its feature), add it here with a
  package tag.
- To check the list against the code, grep the helpers named above; the list is not generated.

## Count by package

Generated from the tagged lines below by `python3 tools/greyed-count.py --write`:

<!-- greyed-count:packages:start -->
| Package | Items |
|---|---|
| `T56` | 1 |
| `T57` | 2 |
| `T58` | 0 |
| `T59` | 0 |
| `T60` | 0 |
| `T61` | 5 |
| `T62` | 0 |
| `T63` | 20 |
| `T65` | 0 |
| `T66` | 9 |
| `T67` | 0 |
| `T69` | 0 |
| `T70` | 0 |
| `T71` | 0 |
| `T72` | 20 |
| `T73` | 0 |
| `T75` | 0 |
| `T76` | 0 |
| `T78` | 0 |
| `T79` | 0 |
| `T80` | 0 |
| `T81` | 0 |
| `T82` | 0 |
| `T83` | 0 |
| `T91` | 0 |
| `T96` | 0 |
| `T114` | 0 |
| `T120` | 0 |
| `T121` | 0 |
| `T122` | 0 |
| `T123` | 0 |
| `T124` | 0 |
| `T125` | 2 |
| `T126` | 0 |
| `T127` | 0 |
| `T128` | 0 |
| `B2` | 29 |
| `B3` | 6 |
| `B4` | 1 |
| `B5` | 12 |
| `B6` | 20 |
| `B7` | 0 |
| `B8` | 3 |
| `B9` | 5 |
| `B11` | 3 |
| `B12` | 1 |
| `B13` | 2 |
| `B14` | 44 |
| `B15` | 21 |
| `B16` | 15 |
| `N` | 20 |
<!-- greyed-count:packages:end -->

Briefs (`T`) hold 59 items, backlog packages (`B`) 162, not planned (`N`) 20.

Counts are **locations**, not unique capabilities: the same command in a toolbar and a context
menu counts twice; window qualifiers on one line do not multiply it. Retag items when ownership
changes. The baseline labels remain open until implementation or a documented reassignment.

## Menu bar

### File
- `B14` New: Matrix... · Image · Excel · Layout · Notes · Function Plot
- `T63` Open Excel...
- `T56` Save Template As... [Workbook, Graph]
- `B11` Save Workbook As Analysis Template... [Workbook]
- `N` Add Word Bookmarks to Analysis Template... [Workbook]
- `B11` Batch Processing... [Workbook, Graph]
- `B14` Import: Image... · Binary 2D Array... [no window, Workbook]
- `T63` Export: Graph as Image... · Multi-Page PDF... [no window, Workbook]

### Edit
- `B5` Paste Link · Paste Special [Workbook, Graph]
- `B5` Paste Link Transpose · Select... [Workbook]
- `B5` Remove Links [Workbook]
- `B16` Button Edit Mode [Workbook, Graph]
- `B2` Cut · Copy · Paste · Clear [Graph]
- `N` Insert New Object · Links · Update Client [Graph]
- `T72` Copy Format: Symbol/Line/Fill · Text [Graph]

### View
- `N` Menu
- `B16` Command Window · Code Builder
- `N` Quick Help · Smart Hint Log
- `B6` Actively Update Plots · Page Break Preview Lines [Workbook]
- `T66` Show: Active Layer Indicator · Axis Layer Icons · Object Grid · Layer Grid · Frame · Labels · Data · Master Items on Screen [Graph]
- `T66` Show Data Info [Graph]

### Plot
- `T63` Line: Horizontal Step · Vertical Step · Spline Connected
- `B15` Y-offset/Waterfall: 3D Waterfall
- `B15` 3D XYY: XYY 3D Bars · 3D Ribbons · 3D Walls · 3D Waterfall
- `B15` 3D Surface: Color Fill Surface · X Constant with Base · Y Constant with Base · Color Map Surface · Colormap Surface with Projection · Wire Frame · Wire Surface · 3D Ternary Colormap Surface
- `B15` 3D Symbol/Bar/Vector: 3D Bars · 3D Scatter · 3D Trajectory · 3D Scatter + Error Bar · 3D Vector XYZ XYZ · 3D Vector XYZ dXdYdZ
- `B8` Statistics: Probability Plot · Q-Q Plot
- `B4` Statistics: Histogram + Probabilities
- `B14` Contour/Heat Map: Color Fill · B/W Lines + Labels · Gray Scale Map · Heat Map · Polar Contour theta(X) r(Y) · Polar Contour r(X) theta(Y) · Ternary Contour
- `B14` Profile: Contour Profiles · Image Profiles

### Column
- `T63` Fill Column With: Row Numbers · Uniform Random Numbers · Normal Random Numbers

### Worksheet
- `B14` Convert to Matrix: Direct · XYZ Gridding · Extract
- `B16` Worksheet Script...

### Analysis
- `B14` Mathematics: Special Filter... · Inverse...
- `B14` Signal Processing > FFT: 2D FFT... · 2D IFFT...
- `T57` Signal Processing: Deconvolution... · STFT...
- `B8` Peaks and Baseline: Batch Peak Analysis Using Theme...

### Image
- `B14` Adjustments: Brightness... · Contrast... · Gamma... · Levels...
- `B14` Arithmetic Transform: Math... · Subtract...
- `B14` Conversion: To Binary... · Color to Gray... · RGB Split...
- `B14` Geometric Transform: Crop... · Resize... · Rotate... · Flip...
- `B14` Spatial Filters: Smooth... · Sharpen... · Edge Detection...

### Graph
- `N` Fit Layer to Graph
- `B3` Convert to Standard Font Size
- `T61` New Layer(Axes): Open Dialog...
- `B3` Legend: Add Legend Title

### Data
- `B3` Lock Position: Layer Icon · First Point · Last Point

### Gadgets
- `B9` Differentiate... · Interpolate... · Cluster... · 2D Integrate... · Go To...

### Tools
- `B13` System Variables...
- `B15` 3D OpenGL Settings...
- `N` Apps on File Exchange...
- `B13` Protection: Protect Project...
- `B14` Virtual Matrix Manager...
- `B16` Package Manager... · Custom Menu Organizer... · X-Function Builder... · X-Function Script Samples...
- `B16` R Console... · Rserve Console... · Python Console...

### Format
- `N` Menu
- `B16` Programming Control...
- `B2` Object Properties...
- `B2` Snap Layers to Grid · Snap Objects to Grid [Graph]
- `B15` Axes: Z Axis... [Graph]
- `T63` Axis Tick Labels: X Axis Tick Labels... · Y Axis Tick Labels... [Graph]
- `B6` Snap to Grid · Merge cells [Workbook]

### Window
- `B12` Arrange Icons
- `B6` Split [Workbook]
- `B16` Command Window · Script Window

### Help
- `N` Use Online Help · Check for Updates...
- `N` Origin: Help · X-Functions · Programming
- `N` Programming: LabTalk · Origin C · Python · Automation Server
- `N` Online Resources: Web Links

## Toolbars

- `B14` Standard: New Matrix · New Layout · New Notes
- `T63` 2D Graphs: Bar · Stacked Area · Box Chart · Histogram · Bubble
- `B2` Object Edit: Align Left · Align Right · Align Top · Align Bottom · Align Horizontal Center · Align Vertical Center · Make Same Width · Make Same Height · Group · Ungroup

## Context menus

- `B5` Worksheet column header: Remove Links
- `T63` Worksheet column header: Frequency Count...
- `B6` Worksheet column header: Add or Update Sparklines...
- `T63` Worksheet column header > Fill Column with: Row Numbers · Uniform Random Numbers · Normal Random Numbers
- `B5` Worksheet data cell: Paste Link · Paste Link Transpose · Remove Links
- `B6` Worksheet data cell: Insert Graph... · Insert Images from Files... · Insert Sparklines... · Insert Variables...
- `B6` Worksheet data cell > Insert Arrow: Arrow · Double Arrow
- `B5` Worksheet data cell > Set Data Style: Rich Text · Wrap Text · Float
- `B6` Worksheet row number: Insert Graph...
- `B6` Worksheet sheet tab: Add Graph as Sheet... · Add Notes as Sheet
- `B6` Worksheet corner: Parameters · Sparklines · Filter · Categories · Page Break Preview Lines
- `T72` Graph axis > Copy Format: Dimensions
- `T72` Graph axis > Paste Format To: Tick Direction · Ticks · Line · Tick Labels · Title Styles · Colors · Thickness · Title Dimensions · Title Text
- `B2` Graph axis title: Cut · Copy · Bring to the front · Push to the back
- `B2` Graph text label: Cut · Copy
- `T125` Graph drawn shape: Cut · Copy
- `B2` Graph legend: Cut · Copy · Bring to the front · Push to the back · Bring to front of data · Push to back of data
- `B16` Graph legend: Programming Control...
- `B11` Graph legend: Save Format as Theme...
- `B3` Graph legend > Legend: Add Legend Title
- `T72` Graph legend > Copy Format: Dimensions · All
- `T61` Graph page > New Layer(Axes): Open Dialog...
- `T72` Graph page > Copy Format: Page Background · Legend Translation Mode
- `T72` Graph page > Paste Format To: Bottom Axes · All X Axes · All Axes · All
- `T61` Graph layer icon: Show Active Layer Only · Set Active Layer By Layer Icon Only · Double click to open Plot Setup

No greyed item was found in the label-cell menu or the Integrate gadget menu.
