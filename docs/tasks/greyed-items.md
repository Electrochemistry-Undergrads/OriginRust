# Greyed items: what Sheaf does not have yet

Every Origin menu item Sheaf lacks is drawn greyed with the tooltip "Not available in Sheaf yet"
(`NOT_YET`; `grey`/`greys`/`grey_sub` in `crates/app/src/menus.rs`, `g(...)` in
`crates/app/src/toolbar.rs`, and `crates/app/src/panels/explorer.rs`). This file lists every one of
them, by menu, and tags each line with the package that will enable it. The packages and their order
are in [PLAN.md](PLAN.md).

Listed at commit `23ac847` (2026-11-15, `main` after T55) in the paused cloud session;
recounted against its package tags during recovery.
Formatting and layer gaps were checked in source during recovery; other labels retain the
original inventory baseline. This is not a new runtime audit. **125 greyed items: 100 in the menu bar, 1 on the toolbars, 24 in context menus.**

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
| `T56` | 0 |
| `T57` | 0 |
| `T58` | 0 |
| `T59` | 0 |
| `T60` | 0 |
| `T61` | 0 |
| `T62` | 0 |
| `T63` | 1 |
| `T65` | 0 |
| `T66` | 7 |
| `T67` | 0 |
| `T69` | 0 |
| `T70` | 0 |
| `T71` | 0 |
| `T72` | 4 |
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
| `T125` | 0 |
| `T126` | 0 |
| `T127` | 0 |
| `T128` | 0 |
| `B2` | 3 |
| `B3` | 3 |
| `B4` | 0 |
| `B5` | 12 |
| `B6` | 15 |
| `B7` | 0 |
| `B8` | 0 |
| `B9` | 1 |
| `B11` | 0 |
| `B12` | 1 |
| `B13` | 2 |
| `B14` | 40 |
| `B15` | 1 |
| `B16` | 15 |
| `N` | 20 |
<!-- greyed-count:packages:end -->

Briefs (`T`) hold 12 items, backlog packages (`B`) 93, not planned (`N`) 20.

Counts are **locations**, not unique capabilities: the same command in a toolbar and a context
menu counts twice; window qualifiers on one line do not multiply it. Retag items when ownership
changes. The baseline labels remain open until implementation or a documented reassignment.

## Menu bar

### File
- `B14` New: Image · Excel · Layout · Function Plot
- `T63` Open Excel...
- `N` Add Word Bookmarks to Analysis Template... [Workbook]
- `B14` Import: Image... · Binary 2D Array... [no window, Workbook]

### Edit
- `B5` Paste Link · Paste Special [Workbook, Graph]
- `B5` Paste Link Transpose · Select... [Workbook]
- `B5` Remove Links [Workbook]
- `B16` Button Edit Mode [Workbook, Graph]
- `N` Insert New Object · Links · Update Client [Graph]

### View
- `N` Menu
- `B16` Command Window · Code Builder
- `N` Quick Help · Smart Hint Log
- `B6` Actively Update Plots [Workbook]
- `T66` Show: Active Layer Indicator · Axis Layer Icons · Object Grid · Layer Grid · Frame · Master Items on Screen [Graph]
- `T66` Show Data Info [Graph]

### Plot
- `B14` Contour/Heat Map: Color Fill · B/W Lines + Labels · Gray Scale Map · Heat Map · Polar Contour theta(X) r(Y) · Polar Contour r(X) theta(Y) · Ternary Contour
- `B14` Profile: Contour Profiles · Image Profiles

### Column

### Worksheet
- `B14` Convert to Matrix: Direct · XYZ Gridding · Extract
- `B16` Worksheet Script...

### Analysis
- `B14` Mathematics: Special Filter... · Inverse...
- `B14` Signal Processing > FFT: 2D FFT... · 2D IFFT...

### Image
- `B14` Adjustments: Brightness... · Contrast... · Gamma... · Levels...
- `B14` Arithmetic Transform: Math... · Subtract...
- `B14` Conversion: To Binary... · Color to Gray... · RGB Split...
- `B14` Geometric Transform: Crop... · Resize... · Rotate... · Flip...
- `B14` Spatial Filters: Smooth... · Sharpen... · Edge Detection...

### Graph
- `N` Fit Layer to Graph
- `B3` Convert to Standard Font Size
- `B3` Legend: Add Legend Title

### Data

### Gadgets
- `B9` Go To...

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
- `B2` Object Properties... [Workbook]
- `B2` Snap Layers to Grid · Snap Objects to Grid [Graph]
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

- `B14` Standard: New Layout

## Context menus

- `B5` Worksheet column header: Remove Links
- `B5` Worksheet data cell: Paste Link · Paste Link Transpose · Remove Links
- `B6` Worksheet data cell: Insert Graph... · Insert Images from Files... · Insert Sparklines... · Insert Variables...
- `B6` Worksheet data cell > Insert Arrow: Arrow · Double Arrow
- `B5` Worksheet data cell > Set Data Style: Rich Text · Wrap Text · Float
- `B6` Worksheet row number: Insert Graph...
- `B6` Worksheet sheet tab: Add Graph as Sheet... · Add Notes as Sheet
- `B6` Worksheet corner: Parameters · Filter
- `T72` Graph axis > Copy Format: Dimensions
- `T72` Graph axis > Paste Format To: Title Dimensions
- `B16` Graph legend: Programming Control...
- `B3` Graph legend > Legend: Add Legend Title
- `T72` Graph page > Copy Format: Legend Translation Mode
- `T72` Graph page > Paste Format To: All

No greyed item was found in the label-cell menu or the Integrate gadget menu.
