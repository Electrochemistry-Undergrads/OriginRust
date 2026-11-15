# Task briefs

Each `Txx-*.md` file is one self-contained piece of work: read HANDOFF.md, this file and **one**
brief, then start. Do not read `docs/audit/origin-differences-2026-11-12.md` or whole specs up front;
a brief names the exact spec section and audit letter to open only when you need detail.

## How to work a task (applies to every brief)

1. **Claim it.** Check the "In flight" table in HANDOFF.md and `git branch -a`; if the task's branch
   exists, someone has it. Otherwise add a line to `re-workspace/scratch/peer-notes.md` (local
   only): session name, branch, files you will touch.
2. **Branch and worktree.** `git worktree add .claude/worktrees/<branch> -b <branch> main`. Build
   with your own folder: `CARGO_TARGET_DIR=target/wt-<branch> CARGO_PROFILE_DEV_DEBUG=0`. Check
   `df -h /c` first (the disk has filled twice; LESSONS "Parallel build folders fill the disk").
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
   ```
7. **Merge** one branch at a time: `git fetch`, merge `main` into the branch, re-run the checks, then
   merge into `main` and push. Delete the build folder with `tools/clean-target.sh wt-<branch>`.
   Tick `docs/ROADMAP.md` and the rows of `docs/spec/feature-parity.md` you closed (recount with
   `python tools/parity-count.py`). Update the brief's **Status** line and the HANDOFF tables.
8. **Commits:** Conventional Commits subjects. **Never** add a `Co-Authored-By: Claude` or
   "Generated with Claude Code" line (owner rule); repeat this rule in every subagent prompt.
9. **Subagents:** Sonnet for building, research and review; Haiku for running tests and checking
   outputs. Give each its own worktree, build folder and named files.

## Brief format

Every brief has: **Status**, **Size** (S < 2 h, M half a day, L several days), **Goal**, **Origin
facts** (what is already measured, with the source), **Where** (files and symbols), **Steps**,
**Done when**, and **Don't** (traps). Keep it that way when you add a brief.

## Index

[Origin 2016 to 2026–2027 functionality gaps](origin-2026-2027-gaps.md) lists newer capabilities
that are missing from Sheaf and have no existing brief or handoff entry.

| Brief | Area | Size | Status |
|---|---|---|---|
| [T01](T01-greyed-menus-with-code.md) | Enable greyed menu items whose feature already exists | M | done (`a2fb77b`); custom sort wired with selected-row bounds; Stack and label-group panel dialogs left |
| [T02](T02-menu-shortcut-differences.md) | Menu labels, shortcuts and context menus that differ from Origin | M | done (`bbdb60a`); unmeasured details listed in brief |
| [T03](T03-analysis-dialog-frame.md) | Analysis dialogs: Recalculate, Output, smoothing methods, pkFind options | M | done (`859c78e`); left: pkFind methods win/first/second/residue, linking more tools |
| [T04](T04-peak-analyzer-wizard.md) | Peak Analyzer as Origin's multi-page wizard | L | pages measured and the wizard built (analysis-tools 11.1); Voigt fit added and checked against Origin (11.2), a function per peak; left: Auto integration window rule, baseline goals and modes |
| [T05](T05-export-graphs-dialog.md) | Export Graphs dialog, raster formats, units, Ctrl+G, Copy Page | M | done (`c32e585`); TGA, PCX, flattened PSD and 256-color TIFF done; mono, 16-colour and 16-bit modes done (Origin's lists, dither, 5-5-5 layout); Graph Theme, Dialog Theme row left |
| [T06](T06-opj-remaining-graph-features.md) | `.opj` writer: X offsets, bar overlap, waterfall, date ticks, window places | L | X offsets, waterfall as offsets, month/day/year ticks, window places written (13.14); bar overlap, group colours, grouped error bars, hidden end label not found (need the GUI) |
| [T07](T07-worksheet-export-and-copy.md) | Worksheet export and clipboard as Origin (missing `--`, formats, label rows) | S | done (`826f390`); clipboard text round-trip fixed; Origin Copy defaults unmeasured |
| [T08](T08-formula-functions.md) | Missing Set Column Values functions in `sheaf-expr` | M | groups 1-3, 5 done (`04a8a80`); `sum`/`count` as Origin, local `now()`, strings and `cell()` dependencies done (`d8d1416`); `substitute`, text-function `$` suffix and built-in column labels done; `char`/`code` done; cross-sheet refs, parameter rows and Origin global/`@SD` defaults left; explicit `format` and direct-column Text defaults done |
| [T09](T09-set-values-dialog.md) | Set Column Values dialog in Origin's layout | M | done (`20c37e1`); scripts box, Set Multiple Columns left |
| [T10](T10-worksheet-operations.md) | Insert/delete rows, move/hide columns, mask, sort column, transpose | L | done; left: mask colour/condition/from graph, per-column and date/text filters, measured layouts of stack/unstack/split/reduce, Group/Subject file codes |
| [T11](T11-axis-scales-and-features.md) | More axis scales and axis-dialog features | L | done; several breaks with Origin's measured layout done (2026-11-15); left: Origin's tick-step change with breaks, reference lines |
| [T12](T12-plot-types.md) | Stacked/horizontal columns, histogram, box, pie, bubble, polar | L | items 1-6 done (polar, radar, ternary, 3D and B&W pie merged 2026-11-13); left: listed in the brief Status |
| [T13](T13-plot-details-and-axis-dialog-tabs.md) | Missing Plot Details tabs and axis-dialog controls | M | done; measured (graphing 18 to 18.5): Drop Lines, Label, Fill Area, Transparency, Background colour, Group tab, Offset, Display, Size/Speed, Link, Stack, page tabs, tree names, Layer Contents and Plot Setup windows, Workbook and `>>`; Sheaf's own and unmeasured (18.6): fill hatch/gradient/transparency, bar width, label column/rows/decimals/offsets, subgroups by size, background gradient and border, X/Y axes switches, page border, icons, comments and title choice; Offset multipliers, Data on Top of Axes, Skip Points, label White Out and Frame/Margin, symbol gaps, No Line Break, Front/Behind line order and Inclusive / Sidelines only / Sidelines and Base also done; left: other fill modes, several groups, Binned and speed mode (X:Y ratio done 2026-11-15), `.opj` writing of the new settings |
| [T14](T14-statistics-breadth.md) | Descriptive-statistics columns, outlier tests, more tests | M | items 1-4 done (`c44a1a6`); 5 (Holm; Dunnett not measurable) and power analysis merged (`d724c40`); 2D binning and discrete frequency done (statistics.md 13, 14); cross tabulation and Dunnett (one-way ANOVA) done, scipy-checked not Origin (statistics.md 15; merged from `stats-compare`); distribution fit, probability plot and Q-Q plot done (calc `distfit`, statistics.md 16; scipy-checked, not Origin); left: Origin measurements of cross tabulation, Dunnett, distribution fit and the probability plots (what Origin fits, plots and reports is unmeasured), PCA, ROC, survival (backlog T18) |
| [T15](T15-fitting-breadth.md) | Multi-dataset global fit, Simplex, surface fits, compare models | L | items 1-6 done (merged 2026-11-13); Rank Models and linear fit with X error not compared with Origin; left: surface fits |
| [T16](T16-numeric-differences.md) | Small numeric/behaviour differences (Mann-Whitney print, spline, error caps) | S | 1, 5 done, 4 investigated (`b967898`); 3 done (`d0dbb37`) (cap width 8 pt); 2 done (`94063b4`) (natural spline, 900 limit, B-spline and Bezier added); 6 done (Origin's automatic B-spline factor, analysis-tools 5) |
| [T17](T17-docs-cleanup.md) | Stale spec paragraphs and parity counts | S | done (item 3 with T20, `522c625`) |
| [T18](T18-backlog.md) | Large items not yet briefed (scripting, 3D, Options, importers, templates) | - | backlog |
| [T19](T19-import-and-column-dialogs.md) | ASCII import options, Import Wizard pages, Column Properties formats | M | items 1 and 3 done; delimiter sets, consecutive-run combining and row/column range-pattern selection with except-first batch scope added; seven Import Wizard pages measured and added (remaining controls in brief) |
| [T20](T20-automatic-axis-range.md) | Automatic axis range as Origin (8 % margin, step rule, log padding), Rescale Margin | M | done (`522c625`) |
| [T21](T21-graph-defaults.md) | Graph defaults as Origin: colour order, fonts, line widths, ticks, symbols, dashes | M | done; four stock legend anchors, major tick lengths in .opj and measured thin-width dash interpolation copied; round dash caps/backend rounding, other template legend placements and chosen minor tick lengths remain |
| [T22](T22-curve-mathematics.md) | Mathematics: Average Multiple Curves, Intersect, Polygon Area | M | done, merged (`734a1ff`); Origin details unmeasured (spec `analysis-curve-math.md`) |
| [T23](T23-data-manipulation-tools.md) | Subtract Straight Line, Reduce to Evenly Spaced X, Curve Translate X/Y, Median Filter | M | done, merged (`f66dbea`); Origin details unmeasured (spec section 12) |
| [T24](T24-drawn-graph-objects.md) | Drawn objects on graphs: line, arrow, rectangle, ellipse | M | done, merged (`542fef7`) |
| [T25](T25-mcp-server.md) | MCP server (`sheaf-mcp`) so Claude and other AI tools can drive Sheaf | L | phase 1 done, merged 2026-11-14 (stdio server, shared `sheaf-ops`, ADR 0011, `docs/spec/mcp.md`); phase 2 (`--attach` to the open app) done 2026-11-15 (ADR 0013, mcp.md 11); `sheaf://app` current-window/selection context and owner Undo AI Changes also done; Windows endpoint acceptance and runtime DACL repair also done; `output: new_sheet` done; CHI protocol tests are documented in T25-protocol-validation; done since: CLI-option bridges for `plot` and `fit`, `analyze` reaches all 55 CLI analysis tools (CLI options shared through `sheaf_ops::args`) and prompts |
| [T26](T26-variance-proportion-tests.md) | Tests for variance and proportions (Hypothesis Testing) | M | done, merged 2026-11-15 (statistics.md 21): four tests, Fisher, power analysis, two-sample proportion measured; left: raw and indexed input forms, variance-dialog plots |
| [T27](T27-print.md) | Print, Print Preview, Page Setup (vector EMF/PDF path) | M | Windows graph, macOS graph panel and workbook printing done (graphing 19, 19.7, 19.8); left: Origin's printed worksheet layout and Print Preview window, range/selection printing, CLI workbook printing |
| [T28](T28-autosave-recovery.md) | Autosave to a recovery folder and crash recovery at start-up | M | done, merged 2026-11-14 (`docs/spec/autosave.md`); left: Origin's defaults unmeasured, backup-before-save switch, Tools > Options home |
| [T29](T29-multivariate-roc-survival.md) | PCA, k-means, ROC curve, Kaplan-Meier | L | done, merged 2026-11-15 (statistics.md 17-20): PCA (with Origin's four graphs), Kaplan-Meier (curve limits), k-means Origin-measured with Origin's report layouts; ROC scipy-checked; left: ROC against Origin, PCA observation labels, Notes/Input Data report tables, Cox |
| [T30](T30-excel-import-export.md) | Excel import (xls, xlsx, xlsm) and xlsx export | L | done, merged 2026-11-14 (ADR 0012, `docs/spec/import-excel.md`); Origin's import measured and copied 2026-11-15 (import-excel.md 2); owner's workbooks checked; left: Origin's import window and export, `.xlsb`/`.ods` tests |
| [T31](T31-windows-executable-icon.md) | Embed the existing app icon in Windows executable resources | S | done; all seven built-PE icon frames match the source and Windows extracts both sizes |
| [T32](T32-multipage-pdf-export.md) | Several graphs in one multi-page PDF (Export Graphs, CLI, MCP) | M | done 2026-11-15 (graphing 9.7); left: Origin's label, file name and page sizing unmeasured |
| [T33](T33-lollipop-plot.md) | Lollipop plot: drop lines to the next data plot, Plus-Minus colour | M | done 2026-11-15 (graphing 18.8); left: Origin's defaults unmeasured; chord and rug plots |
| [T34](T34-frozen-panes.md) | Worksheet frozen panes (View: Freeze/Unfreeze Panes) | L | done 2026-11-15 (gui-look.md 3 "Frozen panes"); left: Origin's counting, limit, divider look and unfreeze rules unmeasured |
| [T35](T35-conditional-highlight.md) | Conditional formatting: highlight min/max, top/bottom | M | done 2026-11-15; left: Heatmap, Pareto, Outliers, custom highlight, Manager |
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
| [T48](T48-graph-data-tools.md) | Graph Data menu: Mask Data Points (rectangle), Remove Bad Data Points, Move Data Points | M | done, merged 2026-11-15 (graphing.md 20); left: Origin measurements, display range and markers |
| [T49](T49-parallel-plot.md) | Parallel Plot and Parallel Index (parallel coordinates) | M | done 2026-11-15 (graphing 14.25, `parallel-plot`, macOS); left: Parallel Sets, curvature, colour map, axis reordering, Origin defaults unmeasured |
| [T50](T50-treemap.md) | Treemap: nested rectangles sized by value (six Origin layouts) | M | done 2026-11-15 (graphing 14.26, `treemap`, macOS); left: Origin defaults/colours/labels unmeasured, hover and drill-down, colour by value |
| [T51](T51-sunburst.md) | Sunburst plot: hierarchy as concentric rings coloured by value | M | done 2026-11-15 (graphing 14.27, `sunburst`, macOS); left: Origin defaults unmeasured, hover and Collapse/Expand drilling |
| [T52](T52-slope-annotation.md) | Slope annotation between two data points (per decade on log axes) | M | claimed 2026-11-15 (`slope-annotation`, macOS) |
| [T53](T53-set-multiple-columns.md) | Set Multiple Columns Values (Ctrl+Shift+Q): one formula over the selected columns with `j` | M | done, merged 2026-11-15 (expression-language 7); left: Origin measurements of `j` |
| [T54](T54-save-window-as.md) | Save Window As (`.sheafbook`/`.sheafgraph` window files opened by appending) and Recent Books / Recent Graphs | M | claimed 2026-11-15 (`save-window`, Claude macOS "mac-savewin") |
| [T55](T55-iir-fir-filters.md) | IIR Filter (Butterworth, Chebyshev, elliptic, Bessel) and FIR Filter design and application | L | claimed 2026-11-15 (`digital-filters`, Claude macOS "violin") |

In flight (do not start these): matrices/contour/heatmaps (`originrust-3a`). All other branches of
2026-11-12 are merged or turned into briefs (T20, T21).
