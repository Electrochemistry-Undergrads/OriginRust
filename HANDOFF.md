# Handoff

## Standing rules from the owner (2026-11-12)

- **Subagents:** use Sonnet subagents when work needs delegating (building, research, reviews),
  and Haiku subagents for testing (running the test suites, click tests, checking outputs).
- **No co-author line:** never add a `Co-Authored-By: Claude ...` line, or a "Generated with
  Claude Code" line, to commits or pull requests; say so in every subagent prompt too. The whole
  history on GitHub was rewritten on 2026-11-12 to remove these lines, so every commit hash older
  than that changed; branches made before it must be rebased onto the new `main` (match commits by
  tree) before merging.
- **Repository:** https://github.com/Electrochemistry-Undergrads/OriginRust (private). Nothing of
  it may live on the owner's personal account (UlanS1).

**Added later on 2026-11-12:** the section "Differences from Origin" below (a five-part audit, A to
E) lists where the current app differs from Origin 2016; nothing else in this file was changed.

Last updated: 2026-11-12, about 16:00, by session 05b367c2 ("Sonnet subagents from
ORIGIN_RUST_PROMPT"). The owner asked this session to finish the open work while the session
`originrust-3a` had made no change since 14:23, so this session now merges to `main` and edits this
file, docs/ROADMAP.md and docs/LESSONS.md. If `originrust-3a` resumes, read
`re-workspace/scratch/peer-notes.md` first; only one session should merge at a time.

**Merged at about 16:00:** branch `opj-graph-read` (`e46863f`): the `.opj` reader keeps the offsets
of stacked curves (the owner's XRD graphs now draw apart, as in Origin), reads the second layer of a
double-Y graph as a linked layer, and the `\(nnn)` text escape works. The "34 of 41 graphs missed"
in "Integration of Delithiation SF data" are sparkline previews, not graph windows: the file holds
7 graphs and all are read (opj-format.md 10.0).

**Merged later on 2026-11-12 (this session):** per-plot left/right Y axis in the app (`e2443c8`), the
parity table brought up to date (`90de8dd`), Origin-style log tick labels (`c467f42`), TIFF export
and export sizes that scale the whole figure (`dc334f6`), free text labels (`eb211ee`: Graph > Add
Text, the T tool, the Text window, dragging, `sheaf plot --label`), the Analysis menu on a graph's
data plot (`94f48b2`), and .opj writing of stack offsets, double-Y layers, bar gaps, axis sides and
automatic axis colours (opj-format.md 13.12). Round trip checked on the owner's XRD project: Origin
file -> Sheaf -> `sheaf export -o x.opj` -> Origin 2016 draws the stacked six-pattern figure as the
original. 944 tests pass on `main`.

**Merged late on 2026-11-12 (session "Rust prompt handoff", at the owner's request, while
`originrust-3a` works on matrices, contour plots and heatmaps):**
- `.opj` graph features (`bc01323`, opj-format.md 13.13): area plots, decimal and scientific tick
  formats, column plots as plot groups (bars side by side), panels as linked layers.
- The worksheet works like Origin's from the keyboard and mouse (`5639a99`): arrow keys,
  Shift+arrow blocks, Ctrl+arrow, Home/End, PageUp/PageDown, Tab and Enter move; typing starts
  editing, F2 edits, Delete clears; dragging over column heads or row numbers selects them.
- NLFit: 1 Iteration, Fit to convergence and Revert (`5639a99`; the fit engine has a stepping
  `Fitter`, results unchanged).
- EPS and EMF export (vector, from the scene) in the app and CLI; legend text with Origin's
  `\l(n) %(n)` notation, legend columns, Graph > Legend > Reconstruct Legend (Ctrl+L), dragging
  the legend. Not checked with ghostscript (not installed); an EMF was drawn by GDI+ by hand.

## The owner's requests of 2026-11-11 (they come before the rest of milestone 6)

1. The desktop app should **look and work like Origin** (OriginPro 2016): same window arrangement,
   menu names and order, dialogs, right-click menus, double-click actions and shortcuts. Icons,
   artwork and help text stay our own (clean-room rule). Plan: ADR 0007.
2. Sheaf should have **all of Origin's features**, checked, not assumed (`docs/spec/feature-parity.md`).
3. **`.opj` is the default project format.** While the writer cannot store everything, saving also
   writes a `<name>.opj.sheaf` companion and says what only the copy holds (ADR 0008).

## Where things stand

- **GitHub** (private): https://github.com/Electrochemistry-Undergrads/OriginRust. CI runs on push.
- Milestones 1 to 5 are done and tagged (milestone 2 still needs the owner for two checks; milestone
  5 misses some graphs of one owner project). Owner requests: parity audit, interface study, `.opj`
  writer for workbooks and graphs, `.opj` as default with companion: done.
- **Merged on 2026-11-12 (afternoon)**:
  - The second session's graph work (branch `graph-p0`, its four commits up to `6c668ed`): double-Y
    as linked layers, stacked lines and waterfall, column charts with sample names, multi-panel
    graphs. An independent review found that saving to `.opj` silently dropped all of it; fixed in
    `11f6fdb` (the writer now names each such feature, so the companion keeps it). Its ADRs are now
    0009 and 0010.
  - ADR 0007 step 8, the graph window (`0e23919`): page on grey ground, layer icons and an active
    layer, click selects, double-click opens the settings of that axis, plot or legend, Origin's
    right-click menus per object, Delete, Ctrl+R, Layer Contents (F12), Plot Setup. Picking uses new
    `sheaf_plot::hit` (the scene records which items belong to which object).
  - The "popup that swallows the next click" bug was not reproduced; a UI test now guards it.
  - Window places saved in the project (ADR 0007 step 10, `fb9471c`), the axis dialog and Plot
    Details in Origin's layout with OK/Cancel/Apply (step 9, by a subagent), and the Integrate
    gadget (Gadgets > Integrate). Then Copy Page (Ctrl+J), the Data Reader tool and the Rescale and
    Column toolbar buttons. All checks pass on `main` (`ba9f527`).
- **Merged later on 2026-11-12**: column formulas and nested folders in written `.opj` files
  (a subagent; checked in Origin 2016, where the formulas recalculate), and area plots. A save to
  `.opj` now needs the companion only for graph features the writer does not know yet. 915 tests
  pass on `main`.
- Also merged: the nonlinear fit dialog (Origin's NLFit layout: Category/Function, Settings pages,
  Code, Parameters, Bounds; Fit tries without writing, Done keeps the result) and the import dialog
  as Origin's tree of option groups. 917 tests pass on `main`. No subagent is running.

## What is left (overview, 2026-11-12)

The owner's daily workflow (CH Instruments import, CSV, overlay plots, integration, fitting,
figures) works end to end. The parity table (`docs/spec/feature-parity.md`, recount with
`python tools/parity-count.py`) has 230 rows: 38 done, 101 partial, 91 missing; of the 101 P0 rows
30 done, 65 partial, 6 missing. These counts predate the work of 2026-11-12 afternoon (graph window,
dialogs, gadget, Data Reader, area plots, formulas and folders in `.opj`); the rows were not
updated, so the real "done" numbers are higher. Updating the rows is a task of its own.

1. **Checks only the owner can do**: a click-through of the app by a person; the i-t text importer
   against a real export; the `.bin` details (chronopotentiometry sample interval, current sign,
   exact comparison for `GCD(0.5C).bin`) need text exports of the same runs.
2. **Origin look and feel (ADR 0007)**: column properties and the analysis-tool dialogs are not
   in Origin's layout; no Import Wizard (Ctrl+3); NLFit lacks "1 Iteration" and "Fit till
   converged" (the fit engine needs single steps); many menu items are greyed because the
   feature is missing. (A plot can now be put on the right Y axis from Plot Details > Group and
   from its right-click menu, `0d74eee`.)
3. **`.opj` files**: saving now writes double-Y (right Y layer and its legend), stack offsets (Y
   only), bar gaps, axis sides and automatic axis colours (13.12), and area plots, decimal and
   scientific tick formats, column plots as a plot group (bars side by side) and panels as linked
   layers with scale links (13.13). It still does not write X offsets, the overlap of bars,
   waterfall, date/time tick formats, group colours that differ from Origin's list, grouped
   columns with error bars, the hidden end tick label of a stack, or window places (the companion
   keeps them; the way Origin stores bar overlap, X offsets and waterfall was not found). The reader
   reads panel links, but not plot groups, legend frames or sparklines (the small previews in a
   sheet's sparkline row, which Origin lists as graphs); `.opju` is not supported.
4. **Breadth (milestone 6)**: reciprocal and other scales, stacked/fill area, pie, box,
   histogram plot type, bubble, polar, ternary; matrices, contour plots and heatmaps (nothing yet);
   statistics beyond t-tests, one-way ANOVA, frequency counts and normality (two-way ANOVA,
   nonparametric tests, correlation); analysis templates (batch import exists); an embedded
   scripting language; 3D graphs (last).
5. **Smaller points**: Origin pads automatic axis ranges more than Sheaf (measured, an open
   choice below); plot groups do not look like Origin's; B-spline with smoothing factor 0 differs
   from Origin. Log-axis tick labels now follow Origin (plain numbers from 0.001 to 1000000).

## Differences from Origin (audit of 2026-11-12, code against the specs)

Five Haiku subagents read the code and compared it with `docs/spec/` (Origin 2016 as measured or
read there). Nothing was built, and Origin was not run, so Origin-side facts are only as good as the
specs; items the specs mark unverified are marked so here. File references are the subagents'.
Sections: A menus and toolbars, B dialogs, C graphing, D analysis and statistics, E data and files.

### A. Menus, toolbars, shortcuts, panels

Matches Origin: top-level menu names and order for the empty, workbook and graph menu bars
(`menus.rs:229-268`), the Window menu, the Analysis and Statistics folder order, the Plot menu's
group order, Cascade/Tile.

- **Missing:** window types other than Workbook and Graph (Matrix, Layout, Notes, Excel, Image,
  Function; `core/src/project.rs:15-18`). Panels Command Window (Alt+3), Code Builder (Alt+4), Quick
  Help (Alt+5), Smart Hint Log (Alt+7), Script Window (Alt+Shift+3), and the Results Log View Mode
  submenu. Toolbars: Column/Worksheet Data, Layout, Mask, Markers and Locks, Style, 3D Rotation,
  Arrow, Database Access, Border, AutoUpdate; the Standard bar lacks Import Wizard, Import Excel,
  Open/Save Template, Refresh, zoom box, Template Organizer, Batch Processing; the Tools bar lacks
  Screen Reader, Data Selector, Draw Data and the drawing/region tools. File > Import has no Database
  Import and only ASCII, CSV, Excel, Image; Tools > Digitizer, Video Builder, Transfer User Files;
  Gadgets > Image/Contour Profile; the graph title-bar right-click menu; Project Explorer keys F2,
  Del, Ctrl+A; Edit > Merge (Embedded) Graphs.
- **Greyed or not wired:** File: Open Excel (Ctrl+E), Open Sample, Append, Close, Save Window As, Save
  Template As, Batch Processing, Print, Print Preview, Page Setup, Re-Import (Ctrl+4). Workbook Edit:
  Find (F3, Ctrl+F), Replace (Ctrl+H), Go to (Ctrl+G), Paste Link, Select, Insert, Set As Begin/End.
  View: Status Bar, zoom, Print/Page/Draft view, Full Screen. Plot: Y Error, XY Error, Multiple Y
  Axes, 3Ys/4Ys, stacked/fill area, Multi-Panel, System Templates. Column: Set as Categorical, Set
  Multiple Columns Values, Fill Column With, Sampling Interval, Filter, Mask, Hide/Unhide, header-menu
  Sort Column. Worksheet: sorts, Clear, Worksheet Script, Split, Pivot, Reduce Rows. Analysis
  (P0): Simple Column Math, Interpolate Y from X, Multiple Linear Regression, Exponential, Sigmoidal,
  Single Peak Fit, Compare/Rank Models, Multiple Peak Fit, Batch Peak Analysis, FFT Filters,
  Convolution. Statistics: Statistics on Rows, Cross Tabulation, Grubbs, Dixon, Distribution Fit,
  three-way/repeated ANOVA, survival, multivariate, power, ROC. Graph menu: Add Plot to Layer, Layer
  Management, Add Error Bars, Add Function Graph, Add Straight Line, New Layer, Merge Graph Windows,
  Reconstruct Legend (Ctrl+L), Fit Layer/Page, Exchange X-Y. **The whole Data menu** (Mark Data Range,
  Lock Position, Mask, Analysis Markers). Format: Layer Properties, Snap, Axis Tick Labels, Axis
  Titles, Worksheet (F4), Cell. **Every Tools item** (no Options dialog, F7, F8, F9). Window: Arrange
  Icons, Refresh (F5), Duplicate, Split. Corner menu (Row Labels, Column/Row Grid). In the 2D Graphs
  toolbar: Bar, Pie, Stacked Area, HLC, Box, Histogram, Bubble, Polar, Ternary, Radar; the whole
  Format bar, graph zoom buttons and Object Edit bar.
- **Different:** Single ASCII, Multiple ASCII and Comma Delimited all open the same multi-file picker
  (`menus.rs:377-379`). Ctrl+G (Export Graphs) is unbound (Export is File > Export > Graphs). Ctrl+Shift+C
  (copy with label rows) missing. Save Project As is Ctrl+Shift+S. Alt+Enter and Explorer
  "Properties..." open a rename dialog (long name only), not Origin's Properties. Explorer window menu
  lacks Save As, Activate, Minimize, Maximize, Dependents, Auto Short Name, Copy Page, Print; Folder
  menu lacks New Window, Find, Save As Project; Delete asks no confirmation. A maximised child still
  draws its own title bar. Object Manager is docked right (Origin: left strip). Plot Details lacks the
  Drop Lines, Label and Offset tabs; layer tabs lack Display. The column header menu adds Recalculate
  and Add New Columns and reads "Statistics on Columns...". Find Peaks and Subtract Baseline are menu
  items, in Origin steps of the Peak Analyzer. Tools bar has Rescale/Zoom where Origin has Scale
  In/Out; Edit bar has Redo, Origin's does not. Status bar lacks Origin's column-range and theme
  boxes in graphs.

### B. Dialogs

- **Plot Details:** no Workbook button or ">>" side panel; no Line transparency or Fill Area Under
  Curve (`dialogs/graph_details.rs:203-225`); no Drop Lines, Label, Offset tabs; Group tab lacks Edit
  Mode, increment rows, Group Members, Subgrouping (`:367-395`); no column Pattern transparency,
  gradient, Pattern Color, Spacing "Width (in %)"; no Layer Size/Speed, Display, Link Axes Scales;
  Page tab lacks Print/Dimensions, Legends/Titles, Miscellaneous, Display (`:671-725`); Stack lacks
  "normalize to percent"; the Background tab is greyed; the window is 900x560 (Origin about 1005x650).
- **Axis dialog:** no "Apply To..." and no per-side list (Bottom/Top/Left/Right; the app has
  Horizontal/Vertical, `graph_axis.rs:50-54`); Tick Labels sub-tabs Format, Table, Minor Tick Labels
  missing; no "Use Same Options for Left and Right"; no `%(?Y)` title text; no extra "X =" grid line;
  no colour and thickness for major/minor ticks (`:374-390`); Special Ticks and Breaks are placeholders
  (`:34, :69-70`); Prefix/Suffix and the Reverse checkbox are greyed; rescale options differ (Round
  out / Exact / Symmetric against Origin's Normal).
- **Layer Contents (F12) and Plot Setup** both open the Plot Details layer tab (`menus.rs:934-935`);
  no Select Columns for Plotting or Error Bars dialog; Page Properties lacks speed mode, skip points,
  max points per curve, units.
- **Column Properties** (`column_props.rs`): no text/numeric, month, day-of-week display types; no
  Engineering or decimal-with-commas format (`:252-328`); no internal data type, label rows, Window
  Properties; Column Width is greyed (`:160-162`); X error and Y error are separate designations
  (Origin: one Err).
- **Set Column Values** (`set_values.rs`): no function list with Add Function and description pane;
  no Undo, "Before formula scripts", Set Multiple Columns Values; AutoUpdate is a None/Manual/Auto
  radio row; title is "Set column values: {target}"; button order OK, Apply, Cancel (Origin OK,
  Cancel, Apply). Text object dialog has the same button order.
- **Import Wizard** (`import_wizard.rs`): first page matches. The filter list adds CH Instruments and
  Delimited Text (Origin lists only <None>); Target Window Worksheet is always enabled; file-name
  naming options reduced to two ticks. The five later pages of Origin's wizard were never seen, so
  they are unchecked.
- **ASCII import** (`import.rs`): delimiters lack multi-character and Fixed width; no "Data Import
  Options" (rename columns from header, rename sheet, apply new range to plots); mode labels differ
  ("New book" against "New books", "Replace the data" against "Replace existing data",
  `batch.rs:42-50`); no Apply.
- **NLFit** (`fit.rs`): flat list of 7 pages rather than tabs; no Find X/Y and Output pages
  (`:828-834`); no Multi-Data Fit mode or Input Data range text; no Fitted Curves options (preview on
  source graph, plot in report table, stack with residual, update legend, plot type); no Advanced >
  Recalculate, Replica, Residual Analysis; Bounds lacks Auto Parameter Initialization and the "< or
  <=" columns; no lower preview with Fit Curve/Residual/Formula/Sample Curve/Messages/Function
  File/Hints tabs; no One Iteration, 100 Iterations, Revert; no Simplex (`:484-485`); default weights
  None/Error bars/1/|y| (Origin: Use Each Range's Setting). Menu labels differ: "Fit Linear..." in
  `fit.rs:31-32` against "Linear Fit..." in `menus.rs:800-801`.
- **Export Graphs** (`export.rs`): only PNG, TIFF, SVG, PDF (no EPS, EMF, JPEG); no Select Graph(s),
  File Name(s) substitution, Path, Overwrite, Graph Theme, Dialog Theme; no Export Settings/Image
  Size/Image Settings groups; no preview pane, Auto Preview, Apply; only Save as and Close
  (`:146-177`); title "Export {name}".
- **Analysis X-Function dialogs** (`analysis.rs`): Recalculate shows only None (Manual/Auto
  disabled, choice not stored, `:1489-1494`); Output node is greyed text, no output range; only Auto
  Preview, no Auto Apply; no Integrate "plot integral curve" or rescale option; no Differentiate
  plot-derivative option; Smoothing lacks Lowess, Loess, Binomial, span, baseline subtraction, pad to
  power of 2 (`:768-786`); Find Peaks lacks "residual after 1st derivative", smoothing points, width
  threshold, foot height, max half width; **Peak Analyzer is one page**, Origin has a wizard (Goal,
  Baseline Mode, Baseline Treatment, Find Peaks, Integrate Peaks, Finish); fit shapes Gauss and
  Lorentz only (no Voigt).
- **No counterpart at all:** Tools > Options (ten tabs), Fitting Function Builder/Organizer (F8, F9),
  Theme Organizer (F7), Import Filters Manager, X-Function Builder, Nested Sort, Color Palette, Set
  Matrix Values, Plot Range, Programming Control; the Dialog Theme row is greyed.

### C. Graphing

- **Missing plot types:** only Line, Scatter, LineSymbol, Column, Area exist (`core/src/graph.rs`
  PlotKind). No pie, box, histogram plot, bubble or colour-mapped symbols, polar, ternary, radar,
  Smith, wind rose, contour, heat map, 3D, stock, vector, Pareto, QC, marginal plots. No stacked,
  100 % stacked, floating or horizontal columns; no cumulative stack offsets (ADR 0010 defers it).
- **Missing scales:** only Linear and Log10 (`graph.rs ~409`); no ln, log2, reciprocal, offset
  reciprocal, probability, probit, logit, Weibull, custom formula, discrete, symmetric log.
- **Missing axis features:** breaks, special ticks, tick count mode, anchor tick, axis position/offset,
  arrows, prefix/suffix, minor tick labels, tick label formula, opposite-axis grid, grid line at a
  value, `%(?X)` title substitution, title rotation/offset, auto-hide or auto-rotate of overlapping tick
  labels, custom top-X scale (links are None or Straight only, `graph.rs ~307`).
- **Missing objects and styling:** drawn lines, arrows, rectangles, pictures (only TextLabel; inset
  greyed, `toolbar.rs:331`); error bars only symmetric Y (no X error, direction, cap width or style,
  `plots.rs ~284`; cap is 0.6 of symbol size); drop lines; fill to a base or between plots; line
  transparency; symbols 8 of Origin's 20, filled-or-not only (Origin has 12 interiors), no size from
  column; connect styles lack 2/3-point segment, B-spline, Bezier, centred steps; Spline is
  Catmull-Rom capped at 2000 points (spec says 900).
- **Legend:** no `%(...)`/`\L(...)` substitution, multi-column, entry order, drop of fit curves, update
  modes (spec graphing.md section 6).
- **Layers and panels:** 3Y/4Y greyed; Multi-Panel menu greyed although the core and CLI support stack
  and grid panels (`core/src/panels.rs`); 2D waterfall only for an unlinked layer.
- **Templates, themes, export:** no save/load of templates (9 built in), no themes (.OTH) or organiser;
  no EMF, EPS, JPEG, GIF export; export sizes in inch and cm only (Origin also mm, pt, px); PNG and
  TIFF are rasterised from the SVG; Copy Page is a fixed 300 dpi bitmap (`app.rs:445-446`), no vector or
  metafile clipboard; graph Cut/Copy/Paste and the Copy Format menu are greyed; palette is a 9-colour
  cycle, not Origin's colour lists.
- **Different, measured in the spec:** automatic axis range. Sheaf rounds the span/5 step outward with
  no margin (`plot/src/ticks.rs resolve_linear ~220-250`); Origin pads each end by about 8 % of the span and
  rounds to half-step multiples (spec graphing.md section 4, 56 cases): data 0..10 gives -2.5..12.5 in
  Origin, 0..10 in Sheaf; -0.2..1.0 gives -0.5..1.25 against -0.5..1.0. Step choice differs too
  (0.91..1.2 gives step 0.5 in Origin). This is the open choice noted under "Open questions".
- **Different, not verified in Origin (do not change without measuring):** log axes always expand to
  whole decades (`ticks.rs ~317-365`; Origin uses linear ticks under one decade, spec 3.1); default tick
  direction Out and no top/right axis (spec U15); default fonts and sizes (Arial 14/12, line 1.5 pt,
  symbol 5 pt) against the spec's [REC] values (22 pt text, 9 pt symbol); legend default text; dash
  patterns (spec U6).

### D. Analysis, fitting, statistics

- **Missing:** global or multi-dataset fit with shared parameters (FDF files with several dependent
  variables are rejected, `fdf.rs:233`); surface fits and 2-3 independent-variable functions (library
  holds 206 one-variable functions, Origin 237 incl. 22 surface); iteration algorithms other than
  Levenberg-Marquardt (no Simplex, One Iteration, Revert); Compare Datasets/Models, Rank Models,
  Multiple Linear Regression, Linear Fit with X Error, Simulate Curve; signal processing beyond
  smoothing (LOWESS, LOESS, binomial smoothing, FFT/IFFT tool, FFT filters other than low-pass, Hilbert,
  convolution, deconvolution, STFT, wavelets, decimation, IIR); Mathematics items (cumulative sum, level
  crossing, curve intersect, translate, reduce to even X, average multiple curves, median/special
  filter, polygon area); statistics Friedman, sign, median and two-sample KS tests, Holm and Dunnett
  comparisons, three-way and repeated-measures ANOVA, power analysis, partial correlation, Grubbs and
  Dixon, distribution fit, probability plot, row statistics, cross tabulation, 2D binning,
  Kaplan-Meier, PCA, k-means, ROC; **recalculation of analysis and fit results from their source**
  (only column formulas recalculate; Analysis Template save is greyed); **any scripting or batch
  runtime** (LabTalk, Python, Origin C, X-Functions; the CLI is the only automation).
- **Partial:** the descriptive-statistics table shows only N, Missing, Mean, SD, SEM, Sum, Min, Q1,
  Median, Q3, Max (`core/analysis.rs:1297-1300`) although `calc/stats.rs` also computes variance,
  skewness, kurtosis, CV, IQR; no confidence limits. Direct weighting exists in the engine
  (`fit/report.rs:97`) but not in `FitWeighting` (`core/fitting.rs:74`); `scale_errors`
  (`fit/nlsf.rs:234`) has no switch. Baseline: constant/line/anchor in `calc`, but the Peak Analyzer
  offers None/EndPoints/Minima only (no ALS, dataset or auto baseline). Multi-peak fit is one shape for
  all peaks, only as a Peak Analyzer checkbox. Interpolation fixes extrapolation to Extrapolate.
  Integrate baseline is zero or end-point only. Derivative smoothing of higher order is unchecked.
  Correlation has Pearson, Spearman, Kendall but no scatter matrix.
- **Numerically or behaviourally different:** Welch confidence interval uses the Welch SE and df
  (`calc/hypothesis.rs:130`; spec says Origin's interval is pooled); default B-spline output differs
  (Origin's automatic smoothing factor not reproduced, `calc/interp.rs:8-20`); histogram automatic bin
  size agrees with Origin in about 57 % of cases (`calc/freq.rs:90-99`); Kendall p-values with missing
  data differ by up to 17 %, Lilliefors p above 0.1 by about 6 %, the KS p-value does not match Origin
  for 5 or 40 numbers; Mann-Whitney asymptotic p with "exact" selected prints the corrected value
  where Origin prints the uncorrected one (`nonparam.rs:128, 217`); fit curve has 1000 points
  (`core/fitting.rs:150`) against FITNPTS 60 in `fdf-format.md:172` (unresolved which the dialog uses);
  iteration limit 400 (read from a fit tree) against ITERATION=100 in the FDF; finite-difference step
  6e-6 against derivStepSize 0.005 (results agree to 5e-10 on 21 cases); fixed-parameter errors blank
  against Origin's 0 (documented choice); peak limits and FWHM are Sheaf's own rules, only the
  local-maximum search is checked against Origin.
- **Strong parts (match documented Origin):** normalize, quartiles, skewness, kurtosis, KS D,
  Shapiro-Wilk, Anderson-Darling, the one-tailed t-test interval, bounded-fit iterates.
- **Stale documents found:** `docs/spec/feature-parity.md` section 7 says peak results lack area percent
  and FWHM, but both exist (`core/analysis.rs:667`).

### E. Data model, import/export, project files

- **Missing:** matrix books and sheets (`Window` is only `Workbook | Graph`, `core/src/project.rs:15-18`;
  Convert to Matrix, Binary 2D Array, XYZ gridding, contour, heat map all greyed); Notes, Layout, Excel
  windows; column mask (no field in `core/src/column.rs:155-193`, the `.opj` reader skips masks);
  transpose, stack/unstack, split columns, pivot, reduce rows, join; hide/unhide, move columns, column
  width, frozen header; insert columns/rows at a position, delete rows; sampling interval, Show X
  Column; categorical columns and the Group and Subject designations (`Designation` is X, Y, Z,
  YError, XError, Label, Disregard, `column.rs:14-27`); column formats Month, Day of Week, Engineering,
  Text & Numeric (Origin's default), Color, Binary, Category (`core/src/format.rs:9-26`; 7 date
  patterns against about 20 in Origin, 3 time against 14); empty cell as distinct from missing (only
  NaN); Set Column Values extras (before-formula script, multi-column set, v1-v4, formula-text
  override, Keep Text); many formula functions (date/time/now/today, year/month/day/weekday, string
  functions, `col(A)$`, `cell(r,c)`, rnd/grnd, cumsum, diff, shift, rank, lookup, stddevp, sem,
  distance, fact, `%()` substitution, cross-sheet and cross-book references; table at
  `crates/expr/src/funcs.rs:91`); Sort Column, range sort, nested sort, sort by label (the one sort is
  single-key, whole sheet, `core/src/analysis.rs:1439`); non-destructive filter and presets (filter
  deletes rows, `analysis.rs:1496`); autosave and recovery; Append Project and Merge in the app (CLI
  `import --append` covers workbooks only); Excel import/export and other importers (HDF5, NetCDF,
  JCAMP, ODBC); Command and Script windows (`PanelId` is Explorer, Objects, Messages, Results only);
  `.opju` reading (refused, `io/src/opj/mod.rs:173`); Paste Transpose/Special/Link, Copy Columns to,
  Fill Column With (clipboard is text only).
- **Partial:** no 13-character book or 18-character column name limit (the column dialog checks
  characters and per-sheet uniqueness only, `column_props.rs:203-212`; the `.opj` writer refuses names
  over 24 bytes); label rows always show and cannot be hidden, no sheet comment (`grid.rs:231-234`);
  "Y finds X" is nearest X on the left, else the first X (`worksheet.rs:56-59`), Y error is any YError
  before the next X or Y (Origin: adjacent column rule); recalculation covers None/Auto/Manual/
  Pending/Broken and cycle refusal but only same-sheet inputs, Pending is a header word not a lock
  icon; undo is full-snapshot, capped at 100 steps (`history.rs:41`); ASCII import has no
  multi-character delimiter, fixed width, column ranges or patterns (`io/src/ascii.rs:26-38`);
  "Multiple ASCII" and "CSV" run the same command, CSV presets no comma; instrument files are CH
  Instruments only; worksheet export is CSV/TSV of the active sheet without options
  (`app.rs:1155-1190`); graph export PNG/TIFF/SVG/PDF only (multi-page PDF, worksheet image, EMF, EPS
  greyed); `.opj` read skips matrices, notes, layouts, fits, masks, plot groups, user parameter rows,
  number formats, hidden columns, widths; `.opj` write refuses the whole file when a name is over 24
  bytes, a text cell over 252 bytes, or a name has non-cp1252 characters (`write/mod.rs:82, 356,
  537-557`), so saving over an older `.opj` then leaves it stale; window places are saved in
  `.sheaf` only; clipboard copy writes displayed text (5 decimals for General), text pasted into a
  number column becomes missing, Copy Page is bitmap only; folders have no duplicate, search or View
  Windows; Results/Messages logs keep 1000 entries.
- **Different:** an imported column is numeric if at least 80 % of cells parse and the rest become
  missing with a warning (`ascii.rs:20, 896-905`); Origin keeps the column as text by default. Typing
  text into a number cell stores missing (`column.rs:322-357`); Origin's Text & Numeric keeps it.
  Worksheet export writes missing values as an empty field (Origin default `--`, wm s8) and numbers as
  shortest round-trip text ignoring the column format (`io/src/export.rs:64-79`), so export and copy
  disagree. The General format doc says 9 significant digits (`format.rs:10-11`) but grid and copy show
  5 decimals (`column.rs:296-300`). The comments row shows only the first line (`grid.rs:252`). **The
  `.opj` writer drops user parameter rows (`Column.params`, filled by ASCII parameter lines and
  batch "SourceFile") and column display formats without any warning** (formula loss is warned;
  `write/mod.rs:596-650`). Ordered comparison with a missing value gives 0 (Origin unchecked).
- **Matches Origin:** new workbook with A(X), B(Y); date day numbers (Origin epoch 2440587);
  `1/0` missing, `%` sign of the dividend, `pi` and `e` constants; Manual recalculation default.
- **Stale spec:** `docs/spec/gui-look.md` section 7 predates the grid work: the code now has the Long
  Name, Units, Comments and F(x)= rows, group numbers in headers ("A(X1)") and 5-decimal display.
  Sparklines and Parameters label rows are still missing.

### How to use this audit

- It is a reading audit (Haiku subagents, code against specs). Before acting on an item, check it in
  the code, and for Origin-side numbers measure in Origin 2016 (`tools/origin-ref/`).
- Suggested order by effect on the owner's workflow: (1) automatic axis padding (C), (2) the silent
  `.opj` writer losses of parameter rows and column formats (E), (3) greyed P0 menu items that have
  working code underneath (Multi-Panel, Y Error, sorts, Find peaks as a wizard) (A), (4) Peak Analyzer
  wizard and the analysis dialogs' Recalculate/Output (B, D), (5) Export Graphs dialog with preview and
  EMF/EPS (B, C).

## Exact next step

1. Done on 2026-11-12 (branch `opj-graph-write2`, spec opj-format.md 13.13, each checked in
   Origin 2016): area plots, tick number formats, plot groups for bars side by side, panels as
   linked layers. Left for the companion: see "What is left" item 3. Finding how Origin stores bar
   overlap, X offsets and waterfall would need new probes (`tools/origin-ref/opj-make-from-lines.ps1`
   and `opj-diff-curves.py` make and compare such files).
2. Done on 2026-11-12 by `originrust-3a` (`12d4f72`): Column Properties, the shared analysis-tool
   frame with preview, the Import Wizard in Origin's layout. Open choices from the dialog work: the
   graph's long name is on Plot Details' Page tab; a plot's legend text is on its Group tab.
3. Done on 2026-11-12: the per-plot left/right axis choice in the app (`0d74eee`), and the rows of
   `docs/spec/feature-parity.md` brought up to date by reading the code (`00a84db`: 40 done, 106
   partial, 84 missing of 230; P0 31 / 68 / 2).
4. Follow the parity table's order of work. Of its stage 1 step 6 (publication figures) free
   text labels, legend text and columns, TIFF, EMF and EPS are done; Origin's colour list (an open
   question below) and size and margin options are left. Stage 2 step 3 lacks Auto Apply. Matrices,
   contour plots and heatmaps are with `originrust-3a`.

The small points of the graph review are fixed too: `--offsets` gives its values to stack members
(a fitted curve takes none), and deleting a column drops the tick labels and offsets that name it.

## Open questions noted by the agents (no answer needed to continue)

- NLFit: should Fit write its results and keep the dialog open, as Origin does (today Fit only
  tries and Done writes)? Should Cancel then undo them? The NLFit window has no preview curve yet;
  each 1 Iteration press restarts the damping (lambda), not compared with Origin.
- Default plot colours: Sheaf uses its own 9-colour list (template.rs); graphing.md 1.5 has a
  23-entry list from the owner's theme, while opj-format.md says a group colours "black, red,
  green, ...". Which order Origin 2016 uses for a fresh overlay is unconfirmed (U1).
- Worksheet: dragging past the edge of the view does not scroll; typing into a selected block (not
  a single cell) does nothing.

- Fit reports show the standard error of a fixed parameter as blank (Origin's sheet stores 0).
- Origin's bound handling can stop above the true constrained minimum; Sheaf copies it.
- `.opj` graphs: plot groups (Origin's colour increments) are not modelled; legend frames are not
  drawn.
- Automatic axis ranges: measured in Origin 2016 (graphing.md section 4): Origin pads by 8 % of the span and rounds to half a major step, so it leaves a wider band than Sheaf (-0.2..1.0 gives -0.5..1.25 in Origin, -0.5..1.0 in Sheaf). Should Sheaf copy Origin's padding? Its step rule was not found.

## Commands that verify the current state

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p sheaf-app -- data.csv --screenshot window.png
cargo run -p sheaf-cli -- plot a.txt b.txt -t cv --panels stack -o cv.png
```

Build with `CARGO_TARGET_DIR=target/lead` and `CARGO_PROFILE_DEV_DEBUG=0` (the disk filled once; it
is at about 88 %). `re-workspace/scratch/gen_chi.js` (local only) writes synthetic test files.

## Needs from me (the owner)

0. For the `.bin` reader: export `GCD(0.5C).bin`, the CV and one i-t file as text from the CH
   Instruments program and put each `.txt` next to its `.bin` (same name). Or allow me to run a copy
   of the CH Instruments program myself (only when no experiment is running).
1. **Try the app** (`cargo run --release -p sheaf-app`): import a real CH Instruments file, plot,
   right-click and double-click the graph, try Analysis > Integrate and Fit; tell me what is wrong.
2. Example figures you make most often, to match their style.
3. How you choose integration limits in Origin (by rows, by potential, or by dragging on the graph).
4. Do you plot charge/discharge curves against time or capacity, and per gram (needs the mass)?
4a. NLFit: should Fit keep its results and leave the dialog open, as in Origin? (Today Fit only
   tries and Done writes.)
5. May I delete the remote branch `chi-text-real` on GitHub? It holds a few numbers from your
   charge-discharge file in a spec paragraph (the merged copy on `main` does not).

## Local-only resources

- `re-workspace/scratch/` holds scratch scripts, raw Origin dumps, synthetic files and screenshots;
  `re-workspace/scratch/peer-notes.md` is the note file between the two sessions.
- The owner's `.opj` and `.bin` files are in the repo root (git-ignored).
- No Ghidra project and no saved REA evidence bundle exist.
- Origin is driven through COM by `tools/origin-ref/` (system-wide lock: one Origin at a time).
- A hidden Origin instance started by an agent of the second session may still be running.
