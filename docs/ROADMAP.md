# Roadmap

Last reconciled: 2026-11-13, against `main` at `d0c5dc4` and the merged task briefs.
This update carries forward the verification recorded in those briefs and specs; it does not
claim a new app run or a new comparison with Origin. Work that exists only on an unmerged branch
is still open here. See [task status and remaining work](tasks/README.md) for the detailed checklist.

A box is ticked only after the task was verified (tests pass, or the output was looked at).
Priorities: P0 is the owner's daily workflow (CH Instruments files, CSV, overlay plots, integration,
publication figures), P1 is common Origin use, P2 is later.

## Milestone 1: inventory and foundations

- [x] Repository with ignore rules; local git history
- [x] Private GitHub repository `OriginRust` created and pushed
- [x] Cargo workspace with the six crates (ADR 0001)
- [x] Document model in `sheaf-core`: project, workbooks, columns, graph model, templates, undo
- [x] Design records: rendering pipeline, GUI stack, project file and undo (ADR 0002-0004)
- [x] Specs written by hand: CHI text format, project file format, figure rendering
- [x] Specs from the study of the installed Origin: UI command map, fit functions and FDF format,
      X-Function index, text import, workbook model, graphing
- [x] `docs/spec/feature-inventory.md` with P0/P1/P2 tags
- [x] CI: one light job (format, lint, test) with caching; first run passed on 2026-11-11
      (Linux runner, about 7 minutes; the desktop app crate is left out)
- [x] Reference fixtures and validation utilities (`tools/origin-ref/`)

## Milestone 2: first vertical slice (P0 end to end)

- [x] Import CH Instruments text files; split cyclic voltammetry into scans (`sheaf-io`)
- [x] Import delimited text / CSV with auto-detection and options (`sheaf-io`)
- [x] Project file save and load (`sheaf-io`)
- [x] Figure rendering to SVG, PNG at a chosen DPI, and PDF (`sheaf-plot`)
- [x] Built-in cyclic voltammetry figure template (smooth lines, no grid, ticks in, closed black
      box, current axis symmetric about zero)
- [x] Command-line program: `import`, `plot`, `export`, `info` (`sheaf-cli`)
- [x] Desktop app: workbook grid with column designations, long names, units, comments
      (measured 2026-11-11, release build: a 1,000,000-row, 27 MB CSV imports in 1.5 s; the app
      opens it, draws 20 frames and saves a screenshot in 2.0 s in all; `sheaf plot` of it at
      300 dpi takes 1.2 s)
- [x] Desktop app: import dialog with options and preview
- [x] Desktop app: graph window; edit axes, ticks, fonts, line styles and legend
- [x] Desktop app: export dialog (PNG with DPI, SVG, PDF); save and open projects; undo and redo
- [x] Golden-image regression tests for the built-in templates
- [x] Independent review of the milestone by a fresh-context agent; findings fixed (11 confirmed
      findings fixed on 2026-11-11, each fix reviewed again; merges `058b2a6`, `915ddab`,
      `912ba75`, follow-up `b336226`)
- [x] Automated desktop-app interaction tests (`crates/app/src/app/ui_tests.rs`): menus, worksheet
      editing, dialogs, plotting, analysis, fitting, save/open, drawing and window commands
- [ ] A person clicks through the desktop app once (screenshots and automated interaction tests
      exist; the owner's end-to-end mouse-and-keyboard check is still open)
- [x] Check CH Instruments CV, i-t and chronopotentiometry against the owner's real files and
      paired text exports (2026-11-13): binary channel order, CV potential starting at Init E and
      time starting at zero corrected; Header and Note metadata kept by text and binary importers
- [x] Read CH Instruments binary `.bin` files (CV, i-t, chronopotentiometry), detected by content;
      the owner's six runs compared with CHI627E text exports, including the i-t runs made on a
      CHI1040C (`7968f30`, `docs/spec/chi-binary-format.md`). The local paired-file test is optional;
      instrument files remain outside the repository

## Milestone 3: analysis, part 1

- [x] Column formulas ("set column values" style expressions): `sheaf-expr`, `Project::set_column_values`,
      dialog in the app; text substitution of all or a chosen occurrence, text-function `$` suffixes,
      character-code conversion, and built-in column-header reads that recalculate after header edits (T08)
- [x] Sort and filter worksheets (`Project::sort_sheet`, `filter_sheet`; app and `sheaf analyze`)
- [x] Normalize, interpolate, differentiate. Cubic B-spline: with a smoothing factor above 0 it
      matches Origin (about 1e-14); with Origin's default factor 0 it does not, because Origin's
      automatic factor was not found (documented in the spec, the dialog and the help)
- [x] Integrate: area under a curve with a baseline; charge from current vs. time
- [x] Smoothing: Savitzky-Golay, adjacent averaging, percentile, FFT filter (the FFT filter matches
      47 Origin outputs to about 1e-15); Lowess, Loess and binomial smoothing added with Origin
      fixtures (`859c78e`, T03)
- [x] Baseline subtraction (constant, minimum, line through end points or two given points)
- [x] Peak finding (local maximum method)
- [x] Descriptive statistics
- [x] Reference results from Origin for each tool, committed as fixtures with tolerances
      (`crates/calc/tests/origin_*.rs`, scripts in `tools/origin-ref/`)
- [x] The tools in the desktop app (Analysis menu) and the command-line program (sheaf analyze),
      results as new columns or result sheets (ADR 0005)
- [x] Independent review of the milestone by a fresh-context agent; findings fixed (16 confirmed
      findings, fixed in `adb1414`, `2cefe20`, `2aa5b37`; the FFT and B-spline work had its own
      review)

## Milestone 4: fitting

- [x] Linear and polynomial fits with full statistics (coefficients and errors match Origin's fitlr
      and fitpoly to 1e-9 and the nlfit report values to 1e-6; badly scaled x to 1e-7; confidence
      limits, unscaled errors, direct weights and Prob>F were not compared with Origin; Origin's
      Linear Fit dialog report could not be scripted; see `docs/spec/fitting-linear.md`)
- [x] Nonlinear least squares (Levenberg-Marquardt) with bounds and fixed parameters (21 fits match
      Origin's NLFit to 5e-10 with the same iteration counts)
- [x] Built-in function library covering Origin's common categories (206 functions; only Gauss,
      Lorentz, ExpDec1, ExpDec2, Boltzmann and Line were compared with Origin)
- [x] User-defined functions; import of `.FDF` fit definitions
- [x] Fit report (parameters, standard errors, R², reduced chi-square, residuals); fitted curve on the
      graph; `sheaf fit` and the app's Fit window (ADR 0006)
- [x] Independent review of the milestone by a fresh-context agent; findings fixed (11 confirmed
      findings, among them wrong starting values of `Gaussian` and base-e EC20/EC80; fixed and
      reviewed again in three branches, merged `fe407b1`, `0ac9546`, `83a09de`)

- [x] Fitting breadth (T15, merged 2026-11-13): Direct weighting and error-scaling controls,
      independent multi-dataset fits and global fits with shared parameters, quick exponential,
      sigmoidal and single-peak fits; Simplex alongside Levenberg-Marquardt
- [x] Compare Models (24 Origin pairs), multiple linear regression (6 Origin cases), Rank Models
      and Linear Fit with X Error (`e525575`). Rank Models uses Sheaf's AICc ranking; it and the
      York-method X-error fit have not been compared with Origin
- [ ] Surface fits and functions of several variables (T15 item 7; depends on matrices)

## Milestone 5: Origin project import

- [x] Specification of the `.opj` container from public sources and inspection of sample files
- [x] Read workbooks and column data from Origin `.opj` (the owner's five projects match
      Origin cell by cell; `tools/origin-ref/opj-compare-cells.py`)
- [x] Read basic graphs: page, layers, axes, titles and text, line, scatter and column plots with
      error bars (checked on graphs built in Origin and the owner's projects; graphs of files
      older than Origin 9 are read but not verified)
- [x] The graphs Origin lists that the reader does not read, explained: in the owner's projects
      only sparkline previews, which are not graph windows (opj-format.md 10.0, checked with
      Origin's own window list on 14 projects); stacked-curve offsets and double-Y layers are read
      (`8a3de72`)
- [x] Read plot groups (including member list colours) and legend frames (`opj-groups-frames`,
      merged 2026-11-13); imported text placement and group colours corrected (`e908922`)
- [ ] Read panel links
- [ ] (Later, if needed) `.opju`

## Owner's requests of 2026-11-11 (they come before milestone 6)

The owner asked for three things: the desktop app should look and work like Origin so their skills
carry over; Sheaf should have all of Origin's features (checked, not assumed); and `.opj` should be
the default project format.

- [x] Feature parity audit: every feature, menu item, toolbar button, dialog and shortcut of Origin compared with Sheaf's code; result in `docs/spec/feature-parity.md` with the order of
      work (231 rows: 34 done, 91 partial, 106 missing; of the 103 P0 rows 28 done, 61 partial,
      14 missing; judged from code, nothing was run). Current table at `d0c5dc4`: 43 done,
      124 partial, 64 missing of 231 rows; P0: 32 done, 69 partial, none missing. These are audit
      rows, not a count of individual features; partial rows still have work left
- [x] Study of Origin's interface (local screenshots only) written up in `docs/spec/gui-look.md`;
      design record `docs/adr/0007-origin-style-workspace.md` (accepted; 13-step plan)
- [ ] Origin-style workspace (ADR 0007): theme, child windows, window arrangement, per-window
      menus, shortcuts, workbook grid, toolbars, Project Explorer, Object Manager, logs, status bar
      and direct graph editing are present; window places are saved. Axis and Plot Details dialogs
      already follow Origin's layout. Added since the previous roadmap update:
  - [x] Shared analysis dialog frame: input ranges (also from a graph), Preview/Auto Preview,
        Apply/Auto Apply, output destinations and Recalculate controls (T03); linked Manual/Auto
        results for Smooth, Integrate, Find Peaks and the newer curve/data-manipulation tools
  - [x] NLFit tabs and lower panel, iteration controls and Fit/Done/Cancel (`nlfit-origin`);
        Set Column Values layout and controls (T09); Column Properties formats and ASCII import
        options in Origin's layout (T19 items 1 and 3)
  - [x] Export Graphs layout, preview, filename substitution, Apply and image settings (T05);
        worksheet export dialog (T07)
  - [x] Window/worksheet commands and shortcuts (T01/T02): multi-panel and Y-error plotting,
        Add Error Bars, legend update, Refresh/recalculation, Go to and Explorer rename/delete
  - [x] Edit > Find in Sheets (Ctrl+F), Replace (Ctrl+H), Worksheet > Clear Worksheet,
        View > Status Bar, File > Close (keeps data), Window > Duplicate (independent workbook
        columns or a graph sharing its data); interpolation menu presets (2026-11-13)
  - [ ] Peak Analyzer wizard (T04), Import Wizard pages 2-6 measured against Origin (T19 item 2),
        remaining Plot Details/axis controls and separate Layer Contents/Plot Setup dialogs (T13)
  - [ ] Owner's click-through; remaining dialog options listed in each brief
- [x] Write `.opj`: workbooks (names, long names, units, comments, designations, text and numbers),
      verified by opening the files in Origin and comparing every cell (`sheaf export ... -o
      x.opj`; seven test projects up to 100000 rows opened in Origin with no difference)
- [x] Write `.opj`: graphs (page, layers, axes, line, scatter, line+symbol and column plots, error
      bars, text, legend; Origin opened 18 written graphs and their exported pictures match
      Sheaf's by eye). Later additions: double-Y, linked panels, stack Y offsets, bar gaps, area
      fill, decimal/scientific tick formats and column groups; X offsets, waterfall represented
      as offsets, month/day/year ticks and window places (T06, opj-format.md 13.12-13.14).
      Bar overlap, custom group colours, grouped-column error bars, hidden end tick labels and
      other date formats remain; newer plot types and drawn objects need the companion
- [x] Write `.opj`: column formulas with None/Auto/Manual recalculation and nested folders,
      read back by Sheaf and checked in Origin (formulas recalculate there; `6bf5148`)
- [x] `.opj` is the default format of Save, Save As and `--save`. While the writer cannot store
      everything a project holds (some graph details), saving also writes a `<name>.opj.sheaf`
      companion and says what only it holds; opening the `.opj` uses the companion when it is at
      least as new; saving over an Origin-made `.opj` keeps a `.opj.bak` first (ADR 0008)
- [ ] Close the gaps from the parity audit in its order of work. Done so far (merged on
      2026-11-12 and 2026-11-13; review and verification details are in the briefs):
  - [x] Subtract Reference Data and Simple Math (+ - * / power with a constant or a reference
        curve), matching Origin's subtract_ref and mathtool to 1e-10
  - [x] Integrating several ranges at once into one table (area, peak, width per range)
  - [x] Date and time columns: Origin's date numbers, detection in CSV import, display formats,
        date axis labels
  - [x] Column formulas that recalculate (Set Column Values None, Auto, Manual), inside one undo
        step, cycles refused
  - [x] Fit confidence and prediction bands (closed form of Origin's straight-line bands checked
        with numbers from Origin), maximum iterations and tolerance, 27 more starting values
  - [x] Import: thousands separators, parameter rows, data blocks as sheets, import modes (new
        book, sheet, columns, rows), batch import of folders and patterns (`sheaf import`)
  - [x] Basic statistics: one-, two-sample (pooled, Welch) and paired t-tests, one-way ANOVA with
        Tukey, Bonferroni, Sidak, Fisher LSD and Scheffe, frequency counts with a histogram graph,
        five normality tests, checked against Origin. Automatic bin sizes now match 593 of
        600 measured cases (T16; `crates/calc/tests/fixtures/bins_origin.rs`)
  - [x] Area plots (Plot > Area, Change Plot to > Area, `sheaf plot -t area`)
  - [x] Copy Page (Ctrl+J, a 300 dpi picture on the clipboard) and the Data Reader tool (snaps to
        data points, pins one, arrow keys step through rows)
  - [x] The Integrate gadget on graphs (Gadgets > Integrate): drag a range, see the shaded area
        and its value live, write it to a sheet; same rows and rule as Analysis > Integrate
  - [x] Peak analysis chain: baseline, find peaks (four methods, filters), integrate each peak,
        multi-peak Gauss or Lorentz fit (Analysis > Analyze Peaks, `sheaf analyze peaks`); the
        limit rules and the multi-peak fit are not yet compared with Origin's Peak Analyzer
  - [x] Curve mathematics (T22, `734a1ff`): Average Multiple Curves (X-grid/range and interpolation
        choices, optional SD/count), Intersect and Polygon Area; app dialogs, CLI and Manual/Auto
        recalculation links. Tested with hand-worked values; Origin's exact behaviour remains
        unmeasured (`docs/spec/analysis-curve-math.md`)
  - [x] Data manipulation (T23, `f66dbea`): Subtract Straight Line, Reduce to Evenly Spaced X,
        Curve Translate X/Y and Median Filter; app, CLI, previews and Manual/Auto links. Origin
        details remain unmeasured (`docs/spec/analysis-tools.md` section 12)
  - [x] Mixed Text & Numeric columns preserve text cells on import and project round trips;
        Month/Day of Week and Engineering/SI/Decimal:1000 displays, 19 date and 14 time patterns;
        ASCII import supports delimiter sets with consecutive-run combining, multi-character and fixed-width delimiters, row/column range-pattern selection, and Data Import Options
        (T19; some format defaults still need measurement)
  - [x] Worksheet operations (T10): insert/delete rows, columns and cells; move/hide/resize columns;
        column/range/nested sort; masks excluded from analysis; non-destructive row filter;
        transpose, stack/unstack, split/reduce; categorical columns and Group/Subject designations.
        Remaining mask/filter options, plot filtering and file codes are listed in the brief
  - [x] Formula functions (T08): running/statistical, date/time, random and string functions,
        text results and `col(A)$`, `cell(r,c)` dependency tracking; `sum`/missing-cell `count`
        aligned with measured Origin behaviour. Cross-sheet/book references, label-row access
        and some string functions remain
  - [x] Worksheet export (T07): displayed numeric formats, missing markers and label-row options;
        clipboard full-precision copying covered by UI tests (Origin clipboard parity remains open)

## Milestone 6: breadth

- [ ] More 2D graph types (T12). Present in the app and CLI, with regression fixtures:
  - [x] Double-Y linked layers, stacked lines, waterfall, multi-panel/shared-X graphs, sample-name
        tick labels, area plots and per-plot left/right Y-axis selection (ADR 0009/0010)
  - [x] Stacked and 100 % stacked columns; horizontal, stacked, 100 % stacked and floating bars;
        floating columns; X/Y error bars with direction and cap-width controls (`d0dbb37`)
  - [x] Stacked and 100 % stacked areas, fill between curves, histogram, box chart, bubble and
        colour-mapped symbols (`179a21d`)
  - [x] Polar (both theta/radius column orders), radar, ternary, flat pie (CLI), 3D Color Pie and
        2D B&W Pie (merged 2026-11-13). Their Origin defaults have not been measured
  - [ ] Polar/ternary axis dialogs, grouped boxes, histogram distribution curve/hatches, bubble
        size legend, full colour maps, exploded pies, drop lines and other families (T12 Status).
        New plot types are not yet written to `.opj`; save warnings and the companion keep them
- [x] Axis breadth (T11): ln, log2, reciprocal/offset reciprocal, probability, probit, logit and
      Weibull; reverse, first tick, prefix/suffix, one break per axis, special ticks, label formula,
      minor labels, tick colour/thickness and Exchange X-Y; axis-dialog edge pages and Apply To
- [ ] Further axis work: multiple breaks, reference lines, measured reciprocal step/break layout
      and `.opj` storage of the newer fields (T11); remaining dialog controls (T13)
- [x] Automatic linear/log range rules and editable Rescale Margin matched to measured Origin
      cases (T20, `522c625`); graph colour lists, fonts, widths, symbols and dash defaults updated
      (T21, `b015f97`). Remaining range cases, legend placement and tick lengths are in the briefs
- [x] Line connections (T16, `94063b4`): natural cubic spline for increasing X up to 900 points,
      plus B-spline and Bezier; distinct from the unresolved analysis smoothing-factor rule
- [x] Drawn graph objects (T24, `542fef7`): line, arrow, rectangle and ellipse tools; Shift
      constraints, selection/move/delete, properties and undo; SVG/PNG/PDF rendering and CLI.
      Geometry and UI tests exist; Origin behaviour has not been measured
- [ ] Drawn-object `.opj` read/write, page/scale attachment, copy/paste, ordering and resize handles;
      pictures and inset graphs (T24/T18). The `.sheaf` companion preserves current objects
- [x] Figure export breadth: SVG, PDF, EPS, EMF and PNG/TIFF/JPEG/BMP/GIF/TGA/PCX/PSD; size units,
      margins, colour depth and compression/quality controls; Copy Page bitmap and Windows EMF
      (T05). PSD is a flattened RGB, grayscale or indexed image with resolution metadata.
      Graph/dialog themes and some palette modes remain
- [ ] Matrices, contour plots and heatmaps (matrix branch is not merged)
- [x] Statistics: t-tests, one- and two-way ANOVA, Mann-Whitney, Wilcoxon, Kruskal-Wallis,
      correlation and normality tests (`148e9b1`); expanded descriptive statistics and statistics
      on rows, Grubbs/Dixon outlier tests, Friedman, sign, median and two-sample KS tests measured
      against Origin (T14 items 1-4, `c44a1a6`)
- [ ] Statistics breadth left: Holm/Dunnett comparisons, cross tabulation, 2D binning, distribution
      fitting, probability plots and power analysis (T14 items 5-6); documented Kendall/normality
      p-value differences remain (T16)
- [ ] Batch processing and analysis templates (batch file import already works)
- [ ] Scripting through an embedded language
- [ ] General 3D graphs (the 3D-styled pie above is available)

## Proposals (not scheduled)

- Golden-image comparison against the same figure made in Origin.
- Open only some windows as tabs when a project with very many windows is opened.
- A baseline through anchor points picked on the graph (the numerics exist in sheaf-calc).
- "Re-import" button that re-reads the source file of a workbook with the stored options.
- Choose whether a newly added plot takes the next palette color or re-spreads a gradient
  (today automatic colors re-spread; colors chosen by hand stay).
