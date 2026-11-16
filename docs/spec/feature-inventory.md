# Feature inventory (Origin / 9.3)

This is the single list of Origin features that the Rust application tracks. Each row has a stable ID, a name, a one-line description, a priority, the spec file and section that describe it, and an empty Status column for the engineers (planned or done). Priorities: P0 is the owner's daily workflow (CH Instruments cyclic voltammetry and chronoamperometry text files, CSV, overlay line plots, integration of peaks and charge, publication figures). P1 is common Origin use. P2 is later or probably never. Details are not repeated here; follow the spec reference. The list ends with the spec gaps.

Spec file short names used in the "Spec" column:

| Short name | File (all in docs/spec) |
|---|---|
| UI | ui-command-map.md (sections A to G) |
| FDF | fdf-format.md |
| FIT | fit-function-catalog.md |
| XF | xfunction-index.md (one row per X-Function, with category folder) |
| IMP | import-ascii.md |
| WB | workbook-model.md |
| GR | graphing.md |
| CHI | chi-text-format.md |
| FIG | figure-rendering.md |
| PRJ | project-file-format.md (our own project format, not Origin's) |
| DATA | data/fit-functions.json |

Note: the analysis agent's own feature-level spec (analysis-features.md) does not exist. Where a row says "XF only", the only written source is the one-line entry in xfunction-index.md; the option tables and algorithm notes from that research exist only in the scratch dump (re-workspace/scratch/analysis/xf.json) and in the lead's context. See the gap list.

## 1. Project and windows

| ID | Feature | Description | Pri | Spec | Status |
|---|---|---|---|---|---|
| PRJ-01 | Project save and open | Save and load the whole project (windows, folders, column data) in our own open format | P0 | PRJ | |
| PRJ-02 | Origin project import (.opj/.opju) | Read the owner's existing Origin project files | P2 | none (binary format not studied) | |
| PRJ-03 | Project Explorer | Folder tree plus window list, with right-click menus for folder, window, multiple selection | P1 | UI F | |
| PRJ-04 | Window types | Workbook, graph, matrix, layout page, notes, image window; each has its own menu bar | P1 | UI A0, A4, F | |
| PRJ-05 | Menu structure per window type | Menu bars for workbook and graph windows, shared menus | P1 | UI A1 to A3 | |
| PRJ-06 | Dynamic Analysis/Statistics/Gadgets menus | Menus built from the function library at run time | P1 | UI A5, XF | |
| PRJ-07 | Toolbars | About 30 toolbars and their buttons | P2 | UI C | |
| PRJ-08 | Keyboard shortcuts | Accelerator tables (import, set column values, rescale, and others) | P1 | UI E | |
| PRJ-09 | Window properties and naming | Short name, long name, comments of windows; rename | P1 | UI D3, D5; WB 2 | |
| PRJ-10 | Folders in the project | Nested folders that group windows | P1 | UI F | |
| PRJ-11 | Undo and redo | Undo of edits | P1 | none | |
| PRJ-12 | Preferences (Tools > Options) | Application options tabs | P2 | UI D10 | |
| PRJ-13 | Notes window | Free-text note window | P2 | UI A4 | |
| PRJ-14 | Layout page window | Page that combines graphs and sheets | P2 | UI A4 | |
| PRJ-15 | Project autosave and backup | Recovery files | P2 | none | |

## 2. Import

| ID | Feature | Description | Pri | Spec | Status |
|---|---|---|---|---|---|
| IMP-ASCII-01 | Delimited text import | One engine for text files: delimiter, structure, number separators | P0 | IMP 1, 2, 5 | |
| IMP-ASCII-02 | Header and sub-header handling | Main header lines and column-aligned label lines mapped to long name, units, comments | P0 | IMP 3 | |
| IMP-ASCII-03 | Data-start auto-detection | Decide where data begins from the first consistent lines | P0 | IMP 4 | |
| IMP-ASCII-04 | Non-numeric, missing, leading zero rules | Text in numeric columns, empty cells, missing markers | P0 | IMP 7 | |
| IMP-ASCII-05 | Date and time detection | Recognise time stamps in columns | P0 | IMP 8 | |
| IMP-ASCII-06 | Import mode and naming | Replace, new book, new sheet, append; book and sheet naming | P0 | IMP 10 | |
| IMP-ASCII-07 | Partial import | Row and column ranges, skip lines | P1 | IMP 9 | |
| IMP-ASCII-08 | Encoding handling | Character set of the input file | P1 | IMP 11 (unknown) | |
| IMP-ASCII-09 | Import filter files (.oif) and saved settings | Named import presets | P1 | IMP 1 (partly); UI B2 | |
| IMP-CSV-01 | CSV import | Comma-separated entry point with quote handling | P0 | IMP 6, 12 | |
| IMP-CHI-01 | CH Instruments file recognition | Detect CHI header and column header, keep run parameters | P0 | CHI 1 | |
| IMP-CHI-02 | CHI techniques other than multi-cycle CV | Chronoamperometry, LSV, and other single-block runs | P0 | CHI 2.1 | |
| IMP-CHI-03 | CV scan splitting | Split a multi-cycle CV into separate scans | P0 | CHI 2.2 | |
| IMP-CHI-04 | Current-range clipping warning | Flag saturated current data | P0 | CHI 2.3 | |
| IMP-CHI-05 | Batch import of many files | Import a list of files into books or sheets in one step | P0 | IMP 10; UI B2 | |
| IMP-WIZ-01 | Import wizard and re-import | Wizard UI, re-import with remembered settings | P1 | UI B2 | |
| IMP-XLS-01 | Excel import (.xls/.xlsx) | Read spreadsheets | P1 | XF (Import and Export folder only) | |
| IMP-IMG-01 | Image file import | Read raster images into an image window or matrix | P2 | XF only | |
| IMP-OTH-01 | Other importers | Binary, instrument, database, MATLAB, HDF and similar importers | P2 | UI B3, XF | |
| IMP-DRAG-01 | Drag and drop of files | Open files dropped on the window | P1 | none | |
| IMP-CLIP-01 | Paste from clipboard | Paste text or cells into a sheet | P1 | none | |

## 3. Workbook and columns

| ID | Feature | Description | Pri | Spec | Status |
|---|---|---|---|---|---|
| WB-MOD-01 | Book / sheet / column containment | Workbook holds sheets, sheets hold columns | P0 | WB 1, 14 | |
| WB-MOD-02 | Names and limits | Short and long names, size limits | P0 | WB 2 | |
| WB-MOD-03 | New sheet default shape | Initial columns and rows | P0 | WB 3 | |
| WB-LAB-01 | Column label rows | Long name, units, comments, user parameters, formula rows | P0 | WB 4 | |
| WB-DES-01 | Plot designations | X, Y, Z, yEr, xEr, Label, Disregard, Group, Subject and how they pair | P0 | WB 5 | |
| WB-FMT-01 | Column data formats | Numeric, Text, Time, Date, Month, Day of Week, Text and Numeric | P0 | WB 6 | |
| WB-FMT-02 | Numeric display format | Decimal places, significant digits, scientific notation | P0 | UI D5 (captions only) | |
| WB-TYP-01 | Internal numeric types | double, real, short, long and others | P1 | WB 7 | |
| WB-MIS-01 | Missing values | Representation, display, effect on plots and statistics | P0 | WB 8 | |
| WB-REF-01 | Range and reference notation | col(), wcol(), [Book]Sheet!Col, row ranges | P0 | WB 9 | |
| WB-MSK-01 | Data masking | Exclude cells from analysis and plots | P1 | WB 12; UI A2 | |
| WB-SRT-01 | Sorting | Sort range, columns, worksheet, nested keys | P0 | WB 13; XF wsort | |
| WB-FLT-01 | Row filter | Hide rows that do not match a condition | P1 | WB 13; UI A2 | |
| WB-COL-01 | Add, delete, move, insert columns | Structural edits | P0 | UI A2 (Ctrl+D) | |
| WB-COL-02 | Set as X/Y/... and set sampling interval | Change designation, implicit X | P0 | UI A2 | |
| WB-CELL-01 | Cell editing, copy, paste, fill | Basic grid editing | P0 | none | |
| WB-FIND-01 | Find and replace in sheet | Search values | P2 | XF findreplace | |
| WB-SHEET-01 | Multiple sheets per book | Add, rename, reorder, delete sheets | P0 | WB 1; XF addsheet | |
| WB-CAT-01 | Categorical columns | Text categories and ordering | P1 | UI A2 (name only) | |
| WB-LNK-01 | External data link | Column data linked to another file | P2 | none | |
| WB-FRM-01 | Worksheet cell formatting | Colours and fonts of the grid | P2 | none | |
| WB-VIEW-01 | Show/hide columns and label rows, freeze | View options | P1 | none | |

## 4. Column math

| ID | Feature | Description | Pri | Spec | Status |
|---|---|---|---|---|---|
| MATH-CV-01 | Set Column Values | Fill a column from a formula over a row range | P0 | WB 10; UI D4 | |
| MATH-CV-02 | Recalculation modes | None, auto, manual; self-reference rule | P0 | WB 10 | |
| MATH-CV-03 | Before-formula script | Local variables for use in the formula | P1 | WB 10 | |
| MATH-FN-01 | Built-in function library | Arithmetic, trig, log, string, statistical functions in formulas | P0 | WB 11 | |
| MATH-FN-02 | Row index and system variables | i, x, wcol, col references with offsets | P0 | WB 9, 11 | |
| MATH-FILL-01 | Fill column with row numbers, random numbers, sets | Fill commands | P1 | UI A2 | |
| MATH-MULTI-01 | Set multiple columns values | Same formula over many columns | P1 | UI A2 | |
| MATH-DATE-01 | Date and time arithmetic | Time columns as numbers, conversions | P1 | WB 6 (partly) | |
| MATH-COMP-01 | Complex numbers | Complex column type and functions | P2 | WB 7 (type only) | |
| MATH-STR-01 | String functions | Text operations in formulas | P1 | WB 11 | |

## 5. Data manipulation

| ID | Feature | Description | Pri | Spec | Status |
|---|---|---|---|---|---|
| DM-SUBL-01 | Subtract straight line | Subtract a line from a curve (baseline for CV) | P0 | XF subtract_line | |
| DM-SUBR-01 | Subtract reference curve | Subtract one curve from another on common X | P0 | XF subtract_ref | |
| DM-EXTR-01 | Extract or mask rows by condition | Worksheet filter to new sheet | P0 | XF wextract | |
| DM-SORT-01 | Sort worksheet | Same as WB-SRT-01 | P0 | XF wsort | |
| DM-NORM-01 | Normalize | Scale to range or by statistic | P0 | XF normalize | |
| DM-INT-01 | Interpolate and resample XY | Evenly spaced X by interpolation | P0 | XF interp1xy | |
| DM-INT-02 | Interpolate onto given X | Evaluate curve at other X values | P1 | XF interp1 | |
| DM-SHIFT-01 | Translate curve in X or Y | Shift by amount or interactively | P1 | XF cxt, curve translate | |
| DM-LVL-01 | Level crossing | X where a curve crosses a level | P1 | XF levelcrossing | |
| DM-RED-01 | Data reduction | Reduce by groups, bins, or fixed factor | P1 | XF reduce* | |
| DM-RES-01 | Sampling interval and evenness check | Compute average step, check even spacing | P1 | XF vinc, vinc_check | |
| DM-COPY-01 | Copy data and extract range | Copy numeric ranges | P1 | XF copydata | |
| DM-STACK-01 | Stack and unstack columns | Reshape between wide and long | P1 | XF only | |
| DM-TRANS-01 | Transpose worksheet | Swap rows and columns | P1 | XF only | |
| DM-CONC-01 | Concatenate and merge worksheets | Join sheets, append | P1 | XF only | |
| DM-CONVOL-01 | Convolution and deconvolution | Convolve two curves | P2 | XF only | |
| DM-GRID-01 | XYZ gridding | Make a matrix from scattered points | P2 | XF Gridding folder | |
| DM-DB-01 | Database access | Query external databases | P2 | XF Database Access | |
| DM-RANK-01 | Rank, remove duplicates, bin | Data cleaning | P2 | XF only | |

## 6. Graphing: plot types

| ID | Feature | Description | Pri | Spec | Status |
|---|---|---|---|---|---|
| GP-LINE-01 | Line plot | Straight-segment connection | P0 | GR 5, 8; UI A2 | |
| GP-SCAT-01 | Scatter plot | Symbols only | P0 | GR 5; UI A2 | |
| GP-LSYM-01 | Line plus symbol | Combined | P0 | GR 5 | |
| GP-OVER-01 | Overlay of many curves on shared axes | Several Y columns or several books in one layer, with plot grouping | P0 | GR 1.5; XF plotxy | |
| GP-ERR-01 | Error bars (Y, X, XY) | Error bars from error columns | P0 | GR 5.5 | |
| GP-DY-01 | Double-Y and multiple Y axes | Two or more Y axes over one X axis (Origin: further layers linked to the first) | P0 | GR 8, GR 12; UI A2 | double Y and right Y axis done (model, renderer, `sheaf plot --y2`, `-t double-y`); 3Ys/4Ys templates, Multiple Y Axes and axis offsets done (T61); the .opj file does not hold the axis offset (companion) |
| GP-OFF-01 | Stacked lines by Y offsets and waterfall | Shifted curves | P0 | GR 8, GR 13; UI A2 | stacked lines done (model, renderer, Auto as in Origin, Constant, Individual, `-t stacked`, `--stack`, `--offsets`); a simple 2D waterfall (`-t waterfall`, `--waterfall`); app controls, Cumulative mode and reading offsets from .opj not yet |
| GP-COL-01 | Column and bar charts | Grouped, stacked, floating, 100 percent | P1 | GR 8, GR 14; UI A2 | column charts done (bars side by side per layer, gap and overlap, bars from zero, `-t column`, `--bar-gap`, `--bar-overlap`, the gap read from .opj); horizontal bars, stacked, floating and 100 percent, plot groups and app controls not yet |
| GP-SPL-01 | Spline and step connection | Smooth and stair-step lines | P1 | GR 5.2 | |
| GP-PAN-01 | Multi-panel and stack graphs | Several layers per page | P1 | GR 8, GR 15; XF plotstack, layarrange | done in the model, renderer and command line (`sheaf plot --panels stack` with a shared X axis and the inner X labels hidden, `--panels RxC` grids, `--panel-y`, `--per-panel`, `--panel-gap`, `--panel-margins`, `--panel-order`, `--panel-link`; Arrange Layers as `Graph::arrange`); app menus, panel labels (a), (b) and reading layer links from .opj not yet |
| GP-AREA-01 | Area and fill-area plots | Filled regions | P1 | UI A2 | |
| GP-DROP-01 | Drop lines, bubble, colour mapped | Third-variable plots | P1 | UI A2 | |
| GP-HIST-01 | Histogram | Binned counts | P1 | UI A2 | |
| GP-BOX-01 | Box chart and interval plot | Distribution summaries | P1 | UI A2 | |
| GP-PIE-01 | Pie chart | 2D pie | P2 | UI A2 | |
| GP-POLAR-01 | Polar, ternary, radar, Smith, wind rose | Specialised coordinate systems | P2 | UI A2 | |
| GP-STOCK-01 | Stock charts | High-low-close, candlestick, OHLC | P2 | UI A2 | |
| GP-VEC-01 | Vector plots | XYAM and XYXY | P2 | UI A2 | |
| GP-PLOTXY-01 | Plot from selection | Create plot from selected columns, designation rules | P0 | XF plotxy; WB 5 | |
| GP-TPL-01 | Plot from template | Create plot using a saved template | P1 | GR 10; UI A2 | |
| GP-ADD-01 | Add and remove data plots on an existing graph | Layer content dialog | P0 | UI D1 (name only) | |

## 7. Graphing: styling

| ID | Feature | Description | Pri | Spec | Status |
|---|---|---|---|---|---|
| GS-OBJ-01 | Graph object model | Page, layers, axes, plots, text objects | P0 | GR 1 | |
| GS-UNIT-01 | Units and sizing | Page and layer size units, scale elements | P0 | GR 2 | |
| GS-AXIS-01 | Axis scale and range | Linear/log scales, from/to, rescale modes | P0 | GR 3.1, 4 | |
| GS-AXIS-02 | Axis line and ticks | Major and minor ticks, direction, length, width | P0 | GR 3.2 | |
| GS-AXIS-03 | Tick labels | Number format, font, rotation, display | P0 | GR 3.3, GR 14 | text from a column (sample names under the X axis, automatic for a text X column, `--x-labels`) and rotated X labels (`--x-label-angle`) done; automatic rotation, tick-indexed labels and app controls not yet |
| GS-AXIS-04 | Axis titles | Default text from long name and units | P0 | GR 3.4 | |
| GS-AXIS-05 | Grids | Major and minor gridlines | P1 | GR 3.5 | |
| GS-AXIS-06 | Axis breaks | Break the axis range | P1 | GR 3.6 | |
| GS-AXIS-07 | Reference lines and special ticks | Lines at given values, custom ticks | P1 | GR 3.7 | |
| GS-AXIS-08 | Opposite axes and central axes | Top and right axes, axes at zero | P0 | GR 11 (unknown); UI D2 | |
| GS-AXIS-09 | Symmetric y-axis around zero | Centre zero current (owner's CV style) | P0 | GR 3.1 (partly); FIG 2 | |
| GS-AUTO-01 | Auto-scale and tick algorithm | Nice-number tick choice | P0 | GR 4; FIG 2 | |
| GS-LINE-01 | Line style, width, colour, connect | Data line formatting | P0 | GR 5.2, 5.3 | |
| GS-SYM-01 | Symbol shape, size, fill, edge | Symbol formatting | P0 | GR 5.4 | |
| GS-GRP-01 | Plot groups and increments | Automatic colour/style cycling across overlaid curves | P0 | GR 1.5 | |
| GS-COL-01 | Colour palettes and colour lists | Default lists, custom colours | P0 | GR 1.5 | |
| GS-LEG-01 | Legend | Entries, substitution syntax, update modes, frame, position | P0 | GR 6 | |
| GS-TXT-01 | Text objects and escape sequences | Annotations with super/subscript, Greek, fonts | P0 | GR 7; FIG 5 | |
| GS-LINEOBJ-01 | Drawn lines, arrows, rectangles | Annotation graphics | P1 | GR 1.6 | |
| GS-THEME-01 | Themes | Saved style sets applied to graphs (system theme on new graphs) | P1 | GR 10 | |
| GS-TPL-01 | Graph templates | Save and reuse a graph's style | P1 | GR 10 | |
| GS-LAY-01 | Layer arrangement and margins | Position and size of layers; merge graphs | P1 | GR 2; XF layarrange, merge_graph | |
| GS-FONT-01 | Fonts and default font | Font family and size defaults | P0 | GR 2.2 (unknown); FIG 7 | |
| GS-BG-01 | Page and layer background, frame | Colours and borders | P1 | GR 1.2, 1.3 | |
| GS-RESC-01 | Rescale after data change | Ctrl+R behaviour | P0 | GR 4 | |
| GS-CURS-01 | Data reader and screen reader | Read coordinates from the graph | P1 | UI (gadgets) | |
| GS-INSET-01 | Zoomed inset | Magnified region | P2 | none | |
| GS-IMG-01 | Pictures in graphs | Embedded images | P2 | none | |

## 8. Graph export

| ID | Feature | Description | Pri | Spec | Status |
|---|---|---|---|---|---|
| GE-PNG-01 | Export PNG | Raster output with dpi and size | P0 | GR 9; FIG 6 | |
| GE-SVG-01 | Export SVG | Vector output (our scene format) | P0 | FIG 6 (SVG is not in Origin's export list seen) | |
| GE-PDF-01 | Export PDF | Vector PDF | P0 | GR 9.5; FIG 6 | |
| GE-EMF-01 | Export EMF and WMF | Windows metafile for Office | P1 | GR 9 | |
| GE-TIF-01 | Export TIFF and JPEG | Print-ready raster | P1 | GR 9.4 | |
| GE-EPS-01 | Export EPS | Postscript | P1 | GR 9 | |
| GE-SIZE-01 | Export size, margin, DPI | Width, height, units, tight margins | P0 | GR 9.3 | |
| GE-NAME-01 | Overwrite and naming rules, batch export | Auto rename, replace, skip, ask; export all graphs | P1 | GR 9.1; UI B4 | |
| GE-CLIP-01 | Copy graph to clipboard | Bitmap or metafile copy | P1 | GR 9.6 | |
| GE-OTH-01 | Other export types | AI, BMP, CGM, DXF, GIF, PCX, PSD, TGA | P2 | GR 9 (names only) | |
| GE-PPT-01 | Send graphs to PowerPoint | Office automation | P2 | UI F | |
| GE-REND-01 | Scene renderer | One scene drawn for screen and all exports | P0 | FIG 1 to 7 | |

## 9. Analysis: mathematics

| ID | Feature | Description | Pri | Spec | Status |
|---|---|---|---|---|---|
| AM-INT-01 | Integrate (trapezoid) | Area, peak height, peak X, width; optional cumulative curve | P0 | XF integ1 | |
| AM-INT-02 | Integrate over a range and with baseline | Limit region, baseline options, charge from current over time | P0 | XF integ1 | |
| AM-INT-03 | Integrate on graph with gadget | Interactive range integration | P1 | XF addtool_integ2, addtool_curve_integ | |
| AM-DIF-01 | Differentiate | Numeric derivative, order 1 to 9, optional Savitzky-Golay | P0 | XF differentiate | |
| AM-MTH-01 | Simple math on curves | Add, subtract, multiply, divide, power with constant or curve | P0 | XF mathtool | |
| AM-NRM-01 | Normalize | Same as DM-NORM-01 | P0 | XF normalize | |
| AM-ITP-01 | Interpolate and extrapolate | Linear, spline, B-spline, Akima | P0 | XF interp1, interp1xy | |
| AM-AVG-01 | Average multiple curves | Mean of curves on common X | P1 | XF avecurves | |
| AM-INTX-01 | Curve intersection | Where two curves cross | P1 | XF addtool_curve_intersect | |
| AM-RISE-01 | Rise time | Rise time of a transition | P2 | XF addtool_rise_time | |
| AM-POLY-01 | Polynomial roots, 2D integration | Less common math | P2 | XF only | |
| AM-COV-01 | Correlation, covariance, cumulative sum | Series operations | P1 | XF only | |

## 10. Analysis: signal processing

| ID | Feature | Description | Pri | Spec | Status |
|---|---|---|---|---|---|
| SP-SM-01 | Smooth: Savitzky-Golay | Polynomial smoothing | P0 | XF smooth | |
| SP-SM-02 | Smooth: adjacent average and percentile | Moving mean and percentile filters | P0 | XF smooth | |
| SP-SM-03 | Smooth: FFT filter, Lowess, Loess, binomial | Other smoothing methods | P1 | XF smooth | |
| SP-FFT-01 | FFT and inverse FFT | Spectrum and window functions | P1 | XF fft1, ifft1 | |
| SP-FLT-01 | FFT filters | Low pass, high pass, band pass, band block | P1 | XF fft_filters | |
| SP-FLT-02 | IIR and FIR filters | Butterworth, Chebyshev, elliptic IIR; window and least squares FIR | P2 | XF dfilter, FIR Filter app | |
| SP-CORR-01 | Correlation and convolution | Signal operations | P2 | XF only | |
| SP-WAV-01 | Wavelet analysis | Transform and denoise | P2 | XF Signal Processing/Wavelet | |
| SP-DEC-01 | Decimation, envelope, coherence | Other signal tools | P2 | XF only | |

## 11. Analysis: peaks and baseline

| ID | Feature | Description | Pri | Spec | Status |
|---|---|---|---|---|---|
| PK-FIND-01 | Peak finding | Methods: maximum, window, first or second derivative, residue; height and width filters | P0 | XF pkFind | |
| PK-BASE-01 | Baseline modes | Constant, user points, dataset, endpoints, line, minimum-maximum, ALS | P0 | XF pa_basemode, pa_basecreate | |
| PK-BASE-02 | Baseline subtraction | Subtract computed baseline | P0 | XF pa_basesubtr | |
| PK-INT-01 | Peak integration | Area, centre, height, FWHM per peak | P0 | XF pa_int | |
| PK-INT-02 | Charge from a CV peak with sloped baseline | Integrate between feet over a baseline line | P0 | XF integ1, pa_int; CHI | |
| PK-PA-01 | Peak Analyzer workflow | Goal, baseline, peaks, results pages | P0 | UI D9; XF pa, pa_goal | |
| PK-PA-02 | Peak analyzer fit step | Fit found peaks with peak functions | P1 | XF pa_fit, fitpeaks | |
| PK-MULTI-01 | Multiple-curve peak analysis | Apply to many Y columns | P1 | XF paMultiY | |
| PK-QUICK-01 | Quick peaks gadget | Interactive peak finding on a graph | P1 | XF addtool_quickpeaks | |
| PK-RESULT-01 | Results sheet layout | Output tables of peak analysis | P0 | none | |

## 12. Analysis: fitting

| ID | Feature | Description | Pri | Spec | Status |
|---|---|---|---|---|---|
| FT-LIN-01 | Linear regression | Slope, intercept, errors, r | P0 | XF fitLR, FitLinear | |
| FT-POLY-01 | Polynomial fit | Polynomial regression and report | P0 | XF fitpoly, FitPolynomial | |
| FT-NL-01 | Nonlinear curve fit engine | Levenberg-Marquardt, simplex, weights, bounds | P0 | XF nlbegin, nlfit, nlend; UI D7 | |
| FT-NL-02 | Fit report | Parameters, errors, chi-square, R-squared, covariance | P0 | XF (partial) | |
| FT-NL-03 | Confidence and prediction bands | Fit curve with bands | P1 | none | |
| FT-FDF-01 | Fit function definition format | Parse built-in function files | P1 | FDF | |
| FT-LIB-01 | Core function set (top 30) | Line, Gauss, Lorentz, Voigt, ExpDec, Boltzmann, and others | P0 | FIT top-30; DATA | |
| FT-LIB-02 | Full library (237 functions) | All built-in functions | P1 | FIT categories; DATA | |
| FT-LIB-03 | User-defined fit function | Write own function and parameters | P1 | FDF | |
| FT-INIT-01 | Initial parameter heuristics | Starting values for each function | P0 | FIT (outline); FDF 6 | |
| FT-MULTI-01 | Multi-peak fit | Several peaks with shared baseline | P1 | FDF 7; FIT | |
| FT-GLOB-01 | Global fit and shared parameters | Fit several datasets together | P2 | FIT (note only) | |
| FT-SURF-01 | Surface fit | Fit two-variable functions | P2 | FIT surface | |
| FT-ODR-01 | Orthogonal distance regression | Errors in X and Y | P2 | none | |
| FT-COMP-01 | Compare models | F-test, AIC, BIC | P2 | none | |
| FT-QUICK-01 | Quick fit gadget | Interactive fit on graph | P1 | XF addtool_quickfit | |
| FT-SIG-01 | Sigmoidal fit gadget | Interactive sigmoidal fit | P2 | XF addtool_sigmoidal_fit | |

## 13. Statistics

| ID | Feature | Description | Pri | Spec | Status |
|---|---|---|---|---|---|
| ST-DESC-01 | Descriptive statistics on columns | Mean, SD, SEM, N, min, max, quartiles, and others | P0 | XF colstats, DescStats | |
| ST-DESC-02 | Statistics of one vector | Quick stats | P0 | XF stats | |
| ST-DESC-03 | Row statistics and moments | Per-row stats, moments | P1 | XF rowstats, moments | |
| ST-DESC-04 | Quantile definition | Which quantile formula | P1 | none (open) | |
| ST-HYP-01 | t-tests | One sample, two sample, paired | P1 | XF ttest1, ttest2, ttestpair | |
| ST-ANOVA-01 | ANOVA, post hoc, nonparametric tests | Group comparisons | P2 | XF only | |
| ST-NORM-01 | Normality tests and probability plot | Shapiro-Wilk and others | P2 | XF only | |
| ST-FREQ-01 | Frequency counts and binning | Bin data | P1 | UI A2 (plot only) | |
| ST-CORR-01 | Correlation coefficients | Pearson and others | P1 | XF only | |
| ST-MULT-01 | Multivariate: PCA, cluster, discriminant | Advanced statistics | P2 | XF only | |
| ST-SURV-01 | Survival, power analysis, DOE | Specialised | P2 | XF only | |
| ST-STAT-01 | Statistics gadget on graph | Stats in a selected region | P1 | XF addtool_curve_stats | |

## 14. Matrices, contour and 3D

| ID | Feature | Description | Pri | Spec | Status |
|---|---|---|---|---|---|
| MX-MOD-01 | Matrix window and matrix sheets | Z values on a grid with X and Y mapping | P2 | UI A4 | |
| MX-CONV-01 | Worksheet to matrix conversion | Direct, XYZ gridding | P2 | XF Matrix, Gridding | |
| MX-OPS-01 | Matrix operations | Transpose, rotate, expand, shrink, interpolate | P2 | XF only | |
| MX-CNT-01 | Contour and heat map | Colour fill, lines, labels | P2 | UI A2 | |
| MX-3D-01 | 3D surface, scatter, bars | 3D graphs | P2 | UI A2 | |
| MX-IMG-01 | Image processing | Filters, thresholds, morphology | P2 | XF Image folders | |
| MX-PROF-01 | Profiles | Along a line in a contour or image | P2 | XF imageprofile | |

## 15. Batch processing and templates

| ID | Feature | Description | Pri | Spec | Status |
|---|---|---|---|---|---|
| BT-REPLAY-01 | Repeat analysis on new data | Re-run a saved analysis when data changes | P1 | none | |
| BT-BATCH-01 | Batch plotting | Make a graph per book or sheet from a template | P1 | UI F (menu name only) | |
| BT-BATCH-02 | Batch import, analysis and export in one run | Process a folder of CHI files to figures and a result table | P0 | none (workflow only) | |
| BT-WBT-01 | Workbook templates | Save a workbook layout and formulas | P1 | none | |
| BT-GT-01 | Graph templates | Reuse graph styles | P1 | GR 10 | |
| BT-THEME-01 | Themes | Same as GS-THEME-01 | P1 | GR 10 | |
| BT-SUM-01 | Summary table collection | Gather results from many sheets | P1 | none | |
| BT-DEP-01 | Dependents tracking | Output updates when source changes | P1 | UI F (name only) | |

## 16. Scripting and automation

| ID | Feature | Description | Pri | Spec | Status |
|---|---|---|---|---|---|
| SC-LT-01 | LabTalk script | Command language | P2 | none | |
| SC-OC-01 | Origin C | Compiled extension language | P2 | none | |
| SC-XF-01 | X-Function interface | Name, inputs, outputs, defaults, dialogs generated from a variable list | P1 | XF (index only) | |
| SC-CLI-01 | Command line and scripting of the Rust app | Replacement for batch workflows | P1 | none | |
| SC-PY-01 | Python or embedded scripting | External packages only in Origin | P2 | none | |
| SC-COM-01 | COM automation | Office and COM control | P2 | none | |

## 17. Interoperability file formats

| ID | Feature | Description | Pri | Spec | Status |
|---|---|---|---|---|---|
| IO-TXT-01 | Read text, CSV, CHI text | See Import | P0 | IMP; CHI | |
| IO-TXT-02 | Write text and CSV | Export worksheet as text | P1 | IMP 13 | |
| IO-XLSX-01 | Write xlsx | Export sheets and charts for the lab's Excel workflow | P1 | none | |
| IO-XLSX-02 | Read xlsx | Same as IMP-XLS-01 | P1 | XF only | |
| IO-FDF-01 | Read FDF fit definitions | Import the function library | P1 | FDF | |
| IO-OTH-01 | Read Origin themes (.oth) | XML theme files | P2 | GR 10 | |
| IO-OTP-01 | Read Origin templates (.otp) | Binary templates | P2 | none (not read) | |
| IO-OPJ-01 | Read Origin project (.opj, .opju) | Same as PRJ-02 | P2 | none | |
| IO-PRJ-01 | Our project format (.sheaf) | ZIP with JSON and column data | P0 | PRJ | |
| IO-IMG-01 | Image files | PNG, JPEG, TIFF, BMP | P1 | GR 9 | |
| IO-JSON-01 | Fit-function JSON | Machine-readable library | P1 | DATA | |

## Spec gaps

These features are in the inventory but no spec covers them well enough to implement yet.

1. Analysis features (AM-*, SP-*, PK-*, FT-NL-*, ST-*): the planned analysis-features.md was never written. Only one-line entries exist in xfunction-index.md. Needed: the option tables, defaults and algorithms for integ1, differentiate, smooth, interp1 and interp1xy, normalize, pkFind, the Peak Analyzer baseline modes, nlfit settings and statistic formulas. The research exists in the scratch dump and the research agent's notes, and must be re-emitted as a spec.
2. Nonlinear fit engine (FT-NL-01 to 03): stopping rule, bounds handling, weights, report fields and the confidence band formulas are not in a spec. FDF and FIT cover the functions only.
3. Initial parameter heuristics (FT-INIT-01): only an outline in the catalog; numeric rules are not defined.
4. Peak Analyzer (PK-PA-01, PK-RESULT-01): page flow and results layout are inferred in UI D9 and not specified.
5. Charge integration of CV peaks (PK-INT-02, AM-INT-02): the sign convention for the reverse sweep, the baseline choice and the units of charge are not written down together with the CHI import. Needs one worked example.
6. Batch workflow (BT-BATCH-02, BT-REPLAY-01, BT-BATCH-01): no spec at all.
7. Concrete numeric graph defaults (GS-FONT-01, GS-AXIS-02, GS-SYM-01, GS-LINE-01, GS-AXIS-08): GR section 11 lists them as unknown; they must be measured in running Origin or chosen by us.
8. Export details (GE-EMF-01, GE-TIF-01, GE-EPS-01, GE-SVG-01): per-format settings and the default type are not fixed; SVG is not a listed Origin type.
9. Excel read and write (IMP-XLS-01, IO-XLSX-01, IO-XLSX-02): no spec, though the lab workflow uses xlsx.
10. CSV details (IMP-CSV-01): quote handling, which strings become missing, and ISO dates are unknown (IMP unknowns).
11. Encoding (IMP-ASCII-08): no option in Origin; our behavior must be decided.
12. CHI format (IMP-CHI-*): chi-text-format.md states that no real CHI file has been checked yet.
13. Column display formats (WB-FMT-02), view options (WB-VIEW-01), cell editing and clipboard (WB-CELL-01, IMP-CLIP-01): not specified.
14. Undo and redo (PRJ-11), drag and drop (IMP-DRAG-01), preferences (PRJ-12): not specified.
15. Statistics beyond descriptive statistics and t-tests (ST-ANOVA-01 onward): only names in the index; quantile and kurtosis conventions are open.
16. Matrix, image and 3D (MX-*): only menu names; intentionally deferred.
17. Scripting (SC-*): no spec for LabTalk, Origin C or our own automation layer.
18. Origin binary formats (PRJ-02, IO-OPJ-01, IO-OTP-01): not studied.
19. Fit bound flags (X/I, ON/OFF) and hidden fit categories (FDF 14) need a check in running Origin.
