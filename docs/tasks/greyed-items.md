# Greyed items: what Sheaf does not have yet

Every Origin menu item Sheaf lacks is drawn greyed with the tooltip "Not available in Sheaf yet"
(`NOT_YET`; `grey`/`greys`/`grey_sub` in `crates/app/src/menus.rs`, `g(...)` in
`crates/app/src/toolbar.rs`, and `crates/app/src/panels/explorer.rs`). This file lists every one of
them, by menu, and tags each line with the package that will enable it. The packages and their order
are in [PLAN.md](PLAN.md).

Listed at commit `23ac847` (2026-11-15, `main` after T55) in the paused cloud session;
recounted against its package tags during recovery.
Formatting and layer gaps were checked in source during recovery; other labels retain the
original inventory baseline. This is not a new runtime audit. **453 greyed items: 312 in the menu bar, 47 on the toolbars, 94 in context menus.**

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
| `T58` | 15 |
| `T59` | 2 |
| `T60` | 7 |
| `T61` | 20 |
| `T62` | 2 |
| `T63` | 31 |
| `B1` | 27 |
| `B2` | 51 |
| `B3` | 28 |
| `B4` | 30 |
| `B5` | 33 |
| `B6` | 31 |
| `B7` | 8 |
| `B8` | 14 |
| `B9` | 13 |
| `B10` | 6 |
| `B11` | 10 |
| `B12` | 16 |
| `B13` | 7 |
| `B14` | 44 |
| `B15` | 21 |
| `B16` | 15 |
| `N` | 19 |
<!-- greyed-count:packages:end -->

Briefs (`T`) hold 80 items, backlog packages (`B`) 354, not planned (`N`) 19.

Counts are **locations**, not unique capabilities: the same command in a toolbar and a context
menu counts twice; window qualifiers on one line do not multiply it. Retag items when ownership
changes. The baseline labels remain open until implementation or a documented reassignment.

## Menu bar

### File
- `B14` New: Matrix... · Image · Excel · Layout · Notes · Function Plot
- `B12` New: Graph
- `B11` New: From Template
- `T63` Open Excel...
- `B12` Open Sample Projects
- `T56` Save Template As... [Workbook, Graph]
- `B11` Save Workbook As Analysis Template... [Workbook]
- `N` Add Word Bookmarks to Analysis Template... [Workbook]
- `B11` Batch Processing... [Workbook, Graph]
- `B12` Save Project without Data...
- `B12` Duplicate Project without Data
- `B14` Import: Image... · Binary 2D Array... [no window, Workbook]
- `B12` Import: Re-Import Directly · Re-Import... [no window, Workbook]
- `T63` Export: Graph as Image... · Multi-Page PDF... [no window, Workbook]
- `B12` Export: Worksheet as Image... [no window, Workbook]
- `B10` Export: Recent Exports [no window, Workbook]
- `B10` Recent Exports [Graph]

### Edit
- `B5` Copy: Copy formula · Copy label rows · Copy formula and label rows · Copy All [Workbook]
- `B5` Paste Link · Paste Special [Workbook, Graph]
- `B5` Paste Transpose · Paste Link Transpose · Select... [Workbook]
- `B5` Remove Links · Set As Begin · Set As End · Reset to Full Range [Workbook]
- `B7` Find in Project... [Workbook]
- `B16` Button Edit Mode [Workbook, Graph]
- `B2` Cut · Copy · Paste · Clear [Graph]
- `N` Insert New Object · Links · Update Client [Graph]
- `B2` Copy Format: All · All Style Formats · Colors · Fonts · Background · Symbol/Line/Fill · Dimensions · Scales · Text [Graph]
- `B2` Paste Format... [Graph]
- `B7` Find in Project [Graph]

### View
- `N` Menu
- `B16` Command Window · Code Builder
- `N` Quick Help · Smart Hint Log
- `B12` View Windows: None · Windows in Active Folder · Active Folder and Subfolders
- `B6` Actively Update Plots · Page Break Preview Lines [Workbook]
- `B1` Print View · Page View · Window View · Draft View · Zoom In · Zoom Out · Whole Page · Zoom All [Graph]
- `B1` Show: Layer Icons · Active Layer Indicator · Axis Layer Icons · Object Grid · Layer Grid · Frame · Labels · Data · Active Layer Only · Master Items on Screen [Graph]
- `B1` Show Data Info [Graph]
- `B1` Full Screen [Graph]

### Plot
- `T63` Line: Horizontal Step · Vertical Step · Spline Connected
- `B4` Symbol: Scatter Central · Column Scatter · Vertical Drop Line
- `B4` Line + Symbol: Line Series · 2 Point Segment · 3 Point Segment
- `B4` Column/Bar/Pie: Column + Label · Grouped Columns (indexed data)
- `T61` Multi-Y: 3Ys Y-YY · 3Ys Y-Y-Y · 4Ys Y-YYY · 4Ys YY-YY · Multiple Y Axes...
- `T62` Y-offset/Waterfall: Waterfall Y:Color Mapping · Waterfall Z:Color Mapping
- `B15` Y-offset/Waterfall: 3D Waterfall
- `B15` 3D XYY: XYY 3D Bars · 3D Ribbons · 3D Walls · 3D Waterfall
- `B15` 3D Surface: Color Fill Surface · X Constant with Base · Y Constant with Base · Color Map Surface · Colormap Surface with Projection · Wire Frame · Wire Surface · 3D Ternary Colormap Surface
- `B15` 3D Symbol/Bar/Vector: 3D Bars · 3D Scatter · 3D Trajectory · 3D Scatter + Error Bar · 3D Vector XYZ XYZ · 3D Vector XYZ dXdYdZ
- `T63` Statistics: Probability Plot · Q-Q Plot
- `B4` Statistics: Grouped Box Charts · Interval Plot · Histogram + Probabilities · Marginal Histograms · Marginal Box Charts · 2D Kernel Density · QC (X bar R) Chart · Pareto · Scatter Matrix
- `B14` Contour/Heat Map: Color Fill · B/W Lines + Labels · Gray Scale Map · Heat Map · Polar Contour theta(X) r(Y) · Polar Contour r(X) theta(Y) · Ternary Contour
- `B14` Profile: Contour Profiles · Image Profiles
- `B4` Specialized: Wind Rose · Piper · Smith Chart · High-Low-Close · Vector XYAM · Vector XYXY · Zoom
- `B4` Stock: High-Low-Close · Japanese Candlestick · OHLC Bar Chart · OHLC-Volume · Line
- `B11` System Templates...
- `B10` Most Recently Used

### Column
- `T63` Fill Column With: Row Numbers · Uniform Random Numbers · Normal Random Numbers
- `B5` Fill Column With: A set of Numbers... · A set of Date/Time values... · Arbitrary set of Text&Numeric values...
- `B5` Set Sampling Interval... · Show X Column...

### Worksheet
- `B7` Conditional Formatting: Conditional Highlight... · Conditional Heatmap... · Conditional Pareto... · Conditional Outliers... · Conditional Format Manager...
- `B7` Pivot Table...
- `B14` Convert to Matrix: Direct · XYZ Gridding · Extract
- `B16` Worksheet Script...
- `B10` Most Recently Used Items

### Analysis
- `B8` Mathematics: XYZ Surface Area...
- `B14` Mathematics: Special Filter... · Inverse...
- `B8` Fitting: Nonlinear Implicit Curve Fit... · Simulate Surface... · Compare Datasets...
- `B14` Signal Processing > FFT: 2D FFT... · 2D IFFT...
- `T57` Signal Processing: Deconvolution... · STFT...
- `B8` Signal Processing > Wavelet: Decompose... · Reconstruction... · Denoise... · Smooth... · Continuous Wavelet...
- `B8` Peaks and Baseline: Multiple Peak Fit · Batch Peak Analysis Using Theme...
- `B10` Most Recently Used Items

### Statistics
- `B8` Descriptive Statistics: Partial Correlation Coefficient...
- `B8` ANOVA: Three-Way ANOVA... · Repeated Measures ANOVA...
- `B10` Most Recently Used Items

### Image
- `B14` Adjustments: Brightness... · Contrast... · Gamma... · Levels...
- `B14` Arithmetic Transform: Math... · Subtract...
- `B14` Conversion: To Binary... · Color to Gray... · RGB Split...
- `B14` Geometric Transform: Crop... · Resize... · Rotate... · Flip...
- `B14` Spatial Filters: Smooth... · Sharpen... · Edge Detection...

### Graph
- `T63` Add Plot to Layer: Line... · Scatter... · Line + Symbol... · Column... · Area...
- `T60` Layer Management... · Merge Graph Windows... · Fit Layer to Graph · Fit Page to Layers...
- `B3` Add Function Graph... · Add Straight Line... · New Table · New Color Scale · Convert to Standard Font Size
- `T61` New Layer(Axes): Horizontal Axis Layer... · Vertical Axis Layer...
- `B3` Legend: Add Legend Title

### Data
- `B3` Set Display Range · Reset to Full Range · Mark Data Range · Clear Data Markers
- `B3` Analysis Markers: Hide · Show Partial Ranges · Show All · Tiny · Small · Medium · Large
- `B3` Lock Position: Layer Icon · First Point · Last Point

### Gadgets
- `B9` Statistics... · Quick Fit... · Quick Sigmoidal Fit... · Quick Peaks... · Differentiate... · Interpolate... · Intersect... · Rise Time... · Cluster... · FFT... · 2D Integrate... · Vertical Cursor... · Go To...

### Tools
- `B13` Options...
- `B13` System Variables...
- `B15` 3D OpenGL Settings...
- `N` Apps on File Exchange...
- `B13` Protection: Protect Project... · Audit Log
- `B13` Fitting Function Builder... · Fitting Function Organizer... · Import Filters Manager...
- `B11` Theme Organizer...
- `B14` Virtual Matrix Manager...
- `B16` Package Manager... · Custom Menu Organizer... · X-Function Builder... · X-Function Script Samples...
- `B16` R Console... · Rserve Console... · Python Console...

### Format
- `N` Menu
- `B16` Programming Control...
- `B2` Object Properties...
- `T61` Layer Properties... [Graph]
- `B2` Snap Layers to Grid · Snap Objects to Grid [Graph]
- `T60` Fit Page To Layers [Graph]
- `B3` Fit Page To Printer [Graph]
- `B15` Axes: Z Axis... [Graph]
- `T63` Axis Tick Labels: X Axis Tick Labels... · Y Axis Tick Labels... [Graph]
- `T58` Axis Titles: X Axis Title... · Y Axis Title... [Graph]
- `T59` Worksheet... [Workbook]
- `B6` Snap to Grid · Merge cells [Workbook]

### Window
- `B12` Arrange Icons
- `B11` Duplicate (Batch Plotting): With New Sheets/Books · With New Columns [Graph]
- `B6` Split [Workbook]
- `B16` Command Window · Script Window
- `B12` Folders: Folders List

### Help
- `N` Use Online Help · Check for Updates...
- `N` Origin: Help · X-Functions · Programming
- `N` Programming: LabTalk · Origin C · Python · Automation Server
- `B12` Open Sample Projects: Sample Projects
- `N` Online Resources: Web Links

## Toolbars

- `B12` Standard: New Graph
- `B14` Standard: New Matrix · New Layout · New Notes
- `T63` Standard: Duplicate
- `T58` Format: Font name box · Font size box · Bold · Italic · Underline · Superscript · Subscript · Greek · Increase Font Size · Decrease Font Size · Font Color · Fill Color · Line Color
- `B1` Graph: Zoom In · Zoom Out · Whole Page
- `T61` Graph: Add Layer (Bottom-X Left-Y) · Add Layer (Top-X Left-Y) · Add Layer (Bottom-X Right-Y) · Add Layer (Top-X Right-Y) · Add Inset Graph
- `T63` 2D Graphs: Bar · Stacked Area · Box Chart · Histogram · Bubble
- `B4` 2D Graphs: High-Low-Close
- `B1` Tools: Zoom In · Zoom Out · Pan
- `B2` Object Edit: Align Left · Align Right · Align Top · Align Bottom · Align Horizontal Center · Align Vertical Center · Make Same Width · Make Same Height · Group · Ungroup
- `T63` Object Edit: Bring to Front · Send to Back

## Context menus

- `B5` Worksheet column header: Copy Columns to... · Remove Links · Swap Columns... · Slide Show of Dependent Graphs · Set Sampling Interval... · Show X Column...
- `T63` Worksheet column header: Frequency Count...
- `B6` Worksheet column header: Add or Update Sparklines...
- `T63` Worksheet column header > Fill Column with: Row Numbers · Uniform Random Numbers · Normal Random Numbers
- `B5` Worksheet column header > Fill Column with: A set of Numbers... · A set of Date/Time values... · Arbitrary set of Text&Numeric values...
- `B5` Worksheet data cell: Paste Transpose · Paste Link · Paste Link Transpose · Remove Links · Set as Begin · Set as End
- `B6` Worksheet data cell: Insert Graph... · Insert Images from Files... · Insert Sparklines... · Insert Variables...
- `B6` Worksheet data cell > Insert Arrow: Arrow · Double Arrow
- `T59` Worksheet data cell > Set Data Style: Default
- `B6` Worksheet row number: Insert Graph...
- `B6` Worksheet sheet tab: Add Graph as Sheet... · Add Notes as Sheet · Duplicate Without Data · Navigate...
- `B6` Worksheet sheet tab > Sheet Name to Show: Short Name · Long Name · Both
- `B6` Worksheet corner: Column Header · Row Header · Column Grid · Row Grid · Page Break Preview Lines
- `B6` Worksheet corner > Row Labels for Column Label Rows: Long Name · Units · Comments · F(x)= · Sparklines · Parameters
- `B3` Graph plot: Change X · Change Y · Ungroup
- `B3` Graph axis: Center
- `B11` Graph axis: Save Format as Theme...
- `B2` Graph axis > Copy Format: All · All Style Formats · Axis
- `B2` Graph axis title: Cut · Copy · Bring to the front · Push to the back
- `B2` Graph text label: Cut · Copy · Bring to the front · Push to the back
- `B2` Graph drawn shape: Cut · Copy
- `B3` Graph legend: Add Color Scale · Add Bubble Scale
- `B2` Graph legend: Cut · Copy · Bring to the front · Push to the back · Bring to front of data · Push to back of data
- `B16` Graph legend: Programming Control...
- `B11` Graph legend: Save Format as Theme...
- `B3` Graph legend > Legend: Add Legend Title
- `B2` Graph legend > Copy Format: All · All Style Formats · Text
- `T60` Graph page: Layer Management...
- `B12` Graph page: Add Shortcut to Favorites
- `B1` Graph page: View Full Screen
- `T61` Graph page > New Layer(Axes): Horizontal Axis Layer... · Vertical Axis Layer...
- `B2` Graph page > Copy Format: All · All Style Formats
- `B11` Graph page > Duplicate (Batch Plotting): Duplicate
- `T61` Graph layer icon: Show Active Layer Only · Set Active Layer By Layer Icon Only · Delete Layer · Double click to open Plot Setup · Layer Properties...
- `T60` Graph layer icon: Layer Management...
- `B12` Explorer folder: Properties...
- `T63` Explorer window: Duplicate

No greyed item was found in the label-cell menu or the Integrate gadget menu.
