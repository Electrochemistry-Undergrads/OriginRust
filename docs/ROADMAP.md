# Roadmap

A box is ticked only after the task was verified (tests pass, or the output was looked at).
Priorities: P0 is the owner's daily workflow (CH Instruments files, CSV, overlay plots, integration,
publication figures), P1 is common Origin use, P2 is later.

## Milestone 1: inventory and foundations

- [x] Repository with ignore rules; local git history
- [ ] Private GitHub repository `OriginRust` created and pushed (blocked: see HANDOFF "Needs from me")
- [x] Cargo workspace with the six crates (ADR 0001)
- [x] Document model in `sheaf-core`: project, workbooks, columns, graph model, templates, undo
- [x] Design records: rendering pipeline, GUI stack, project file and undo (ADR 0002-0004)
- [x] Specs written by hand: CHI text format, project file format, figure rendering
- [x] Specs from the study of the installed Origin: UI command map, fit functions and FDF format,
      X-Function index, text import, workbook model, graphing
- [x] `docs/spec/feature-inventory.md` with P0/P1/P2 tags
- [ ] CI: one light job (format, lint, test) with caching (written; has never run because nothing
      is pushed yet)
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
- [x] Desktop app: import dialog with options and preview
- [x] Desktop app: graph window; edit axes, ticks, fonts, line styles and legend
- [x] Desktop app: export dialog (PNG with DPI, SVG, PDF); save and open projects; undo and redo
- [x] Golden-image regression tests for the built-in templates
- [ ] Independent review of the milestone by a fresh-context agent; findings fixed
- [ ] A person clicks through the desktop app once (the four desktop-app items above were checked
      with screenshots of every window, not with mouse and keyboard)
- [ ] Check the CH Instruments importer against real export files from the owner

## Milestone 3: analysis, part 1

- [x] Column formulas ("set column values" style expressions): `sheaf-expr`, `Project::set_column_values`,
      dialog in the app
- [ ] Sort and filter worksheets
- [ ] Normalize, interpolate, differentiate
- [ ] Integrate: area under a curve with a baseline; charge from current vs. time
- [ ] Smoothing: Savitzky-Golay, adjacent averaging, FFT filter
- [ ] Baseline subtraction
- [ ] Peak finding
- [ ] Descriptive statistics
- [ ] Reference results from Origin for each tool, committed as fixtures with tolerances

## Milestone 4: fitting

- [ ] Linear and polynomial fits with full statistics
- [ ] Nonlinear least squares (Levenberg-Marquardt) with bounds and fixed parameters
- [ ] Built-in function library covering Origin's common categories
- [ ] User-defined functions; import of `.FDF` fit definitions
- [ ] Fit report (parameters, standard errors, R², reduced chi-square, residuals); fitted curve on the graph

## Milestone 5: Origin project import

- [ ] Specification of the `.opj` container from public sources and inspection of sample files
- [ ] Read workbooks and column data from Origin 2016-era `.opj`
- [ ] Read basic graphs
- [ ] (Later, if needed) `.opju`

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
- "Re-import" button that re-reads the source file of a workbook with the stored options.
