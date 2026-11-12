# Roadmap

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
- [x] Origin can be driven by COM automation to produce reference numbers (`tools/origin-ref/`)

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
- [ ] A person clicks through the desktop app once (the four desktop-app items above were checked
      with screenshots of every window, not with mouse and keyboard)
- [ ] Check the CH Instruments importer against real export files from the owner

## Milestone 3: analysis, part 1

- [x] Column formulas ("set column values" style expressions): `sheaf-expr`, `Project::set_column_values`,
      dialog in the app
- [x] Sort and filter worksheets (`Project::sort_sheet`, `filter_sheet`; app and `sheaf analyze`)
- [x] Normalize, interpolate, differentiate. Cubic B-spline: with a smoothing factor above 0 it
      matches Origin (about 1e-14); with Origin's default factor 0 it does not, because Origin's
      automatic factor was not found (documented in the spec, the dialog and the help)
- [x] Integrate: area under a curve with a baseline; charge from current vs. time
- [x] Smoothing: Savitzky-Golay, adjacent averaging, percentile, FFT filter (the FFT filter matches
      47 Origin outputs to about 1e-15)
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

## Milestone 5: Origin project import

- [x] Specification of the `.opj` container from public sources and inspection of sample files
- [x] Read workbooks and column data from Origin 2016-era `.opj` (the owner's five projects match
      Origin cell by cell; `tools/origin-ref/opj-compare-cells.py`)
- [x] Read basic graphs: page, layers, axes, titles and text, line, scatter and column plots with
      error bars (checked on graphs built in Origin 2016 and the owner's projects; graphs of files
      older than Origin 9 are read but not verified; the reader finds 7 of the 41 graphs Origin
      lists in one owner project: windows without axis blocks are missed)
- [ ] Graphs the reader misses (34 of 41 in "Integration of Delithiation SF data"), plot groups,
      plot offsets, legend frames
- [ ] (Later, if needed) `.opju`

## Owner's requests of 2026-11-11 (they come before milestone 6)

The owner asked for three things: the desktop app should look and work like Origin so their skills
carry over; Sheaf should have all of Origin's features (checked, not assumed); and `.opj` should be
the default project format.

- [x] Feature parity audit: every feature, menu item, toolbar button, dialog and shortcut of Origin
      2016 compared with Sheaf's code; result in `docs/spec/feature-parity.md` with the order of
      work (231 rows: 34 done, 91 partial, 106 missing; of the 103 P0 rows 28 done, 61 partial,
      14 missing; judged from code, nothing was run)
- [ ] Study of Origin's interface (local screenshots only) written up in `docs/spec/gui-look.md`;
      design record `docs/adr/0007-origin-style-workspace.md`
- [ ] Origin-style workspace: child windows inside the main window instead of tabs, Project
      Explorer, menus that follow the active window type with Origin's names and order, toolbars
      and the Tools bar (our own icons), right-click menus and double-click actions as in Origin
- [ ] Write `.opj`: workbooks (names, long names, units, comments, designations, text and numbers),
      verified by opening the files in Origin 2016 and comparing every cell
- [ ] Write `.opj`: graphs
- [ ] `.opj` is the default format of Save, Save As and `--save`. While the writer cannot store
      everything a project holds (graphs at first), saving also writes a `.sheaf` copy next to the
      `.opj` and says what only the copy holds; the copy is dropped once nothing would be lost
- [ ] Close the gaps from the parity audit in its order of work

## Milestone 6: breadth

- [ ] More 2D graph types: column and bar, area, error bars, double-Y, multi-panel, log and
      reciprocal scales
- [ ] Matrices, contour plots and heatmaps
- [ ] Statistics: t-tests, ANOVA
- [ ] Batch processing and analysis templates
- [ ] Scripting through an embedded language
- [ ] 3D graphs

## Proposals (not scheduled)

- Golden-image comparison against the same figure made in Origin.
- Automatic recalculation of formula columns when their source columns change (Origin's
  "Recalculate: Auto").
- Automated tests of the desktop app that simulate clicks (for example with `egui_kittest`).
- Tick labels from a text column (sample names on the X axis of an ICP-OES plot).
- Open only some windows as tabs when a project with very many windows is opened.
- Choose the integration range by dragging on the graph (like Origin's integration gadget), and
  show the integrated area as a shaded region.
- A baseline through anchor points picked on the graph (the numerics exist in sheaf-calc).
- "Re-import" button that re-reads the source file of a workbook with the stored options.
- Mixed text-and-number columns, as Origin's "Text & Numeric": today a cell such as `<LOD` or
  `n.d.` in a numeric column is read as a missing value (with a warning), so its text is lost.
- Choose whether a newly added plot takes the next palette color or re-spreads a gradient
  (today automatic colors re-spread; colors chosen by hand stay).
