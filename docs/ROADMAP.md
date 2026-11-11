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
- [ ] Specs from the study of the installed Origin: UI command map, fit functions and FDF format,
      analysis tools, text import, workbook model, graphing
- [ ] `docs/spec/feature-inventory.md` with P0/P1/P2 tags
- [ ] CI: one light job (format, lint, test) with caching
- [ ] Check that Origin can be driven by COM automation to produce reference numbers

## Milestone 2: first vertical slice (P0 end to end)

- [ ] Import CH Instruments text files; split cyclic voltammetry into scans (`sheaf-io`)
- [ ] Import delimited text / CSV with auto-detection and options (`sheaf-io`)
- [ ] Project file save and load (`sheaf-io`)
- [ ] Figure rendering to SVG, PNG at a chosen DPI, and PDF (`sheaf-plot`)
- [ ] Built-in cyclic voltammetry figure template (smooth lines, no grid, ticks in, closed black
      box, current axis symmetric about zero)
- [ ] Command-line program: `import`, `plot`, `export`, `info` (`sheaf-cli`)
- [ ] Desktop app: workbook grid with column designations, long names, units, comments
- [ ] Desktop app: import dialog with options and preview
- [ ] Desktop app: graph window; edit axes, ticks, fonts, line styles and legend
- [ ] Desktop app: export dialog (PNG with DPI, SVG, PDF); save and open projects; undo and redo
- [ ] Golden-image regression tests for the built-in templates
- [ ] Independent review of the milestone by a fresh-context agent; findings fixed

## Milestone 3: analysis, part 1

- [ ] Column formulas ("set column values" style expressions)
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
- "Re-import" button that re-reads the source file of a workbook with the stored options.
