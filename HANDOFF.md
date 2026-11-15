# Handoff

Last updated: 2026-11-15. This file is short on purpose. **To start work, read this
file, then `docs/tasks/README.md`, then one brief `docs/tasks/Txx-*.md`.** Do not read
`docs/audit/origin-differences-2026-11-12.md` or whole specs up front; the brief names the section to
open. History of what was merged when is in `git log` (not repeated here).

## Standing rules from the owner

- **When unsure, copy Origin:** measure it through `tools/origin-ref/` (or read its docs for
  behaviour) and copy it; record the measurement in the spec. Don't ask the owner what Origin can
  answer. Clean-room: copy behaviour and layout, never code, icons, help text or files.
- **Subagents:** Claude sessions use Sonnet for building, research and reviews, and Haiku for tests
  and output checks. **Codex sessions use Terra instead of Sonnet and Luna instead of Haiku.**
- **Codex GitHub monitor:** keep a dedicated Luna subagent checking GitHub periodically while work
  is in flight. Watch remote `main`, branches and task claims for another session taking or
  overlapping the task; reconcile new work before merging and never overwrite another session's work.
- **Codex continuous work:** the owner requested an ongoing queue: finish, review, verify and
  merge/push one suitable task, then find and claim another. Do not stop merely because one task
  is complete. Refresh GitHub and the handoff before selecting the next task; respect active claims.
- **No co-author line:** never add `Co-Authored-By: Claude ...` or "Generated with Claude Code" to
  commits or PRs; say so in every subagent prompt. (History was rewritten on 2026-11-12 to remove
  them; branches older than that must be rebased onto `main`, matching commits by tree.)
- **Repository:** https://github.com/Electrochemistry-Undergrads/OriginRust (private). Nothing on the
  owner's personal account.
- **Two or more sessions** often share this repo. Pull the latest changes and inspect branches,
  worktrees and claims before choosing work. Publish the task claim in this file on `main` (push it)
  and in `re-workspace/scratch/peer-notes.md` (local only, read its last 40 lines on start), so other
  computers can see the split. Merge to `main` one branch at a time, after `git fetch`.
  See LESSONS "Two sessions on one repository need a written split".
- **Inactive task claims:** If a task has had no edits for at least 48 consecutive hours,
  another agent may take ownership. Check the task brief, its branch and worktree when
  available, and the latest peer notes for evidence of newer work. Record the takeover in
  `re-workspace/scratch/peer-notes.md` with the date, new owner, branch and scope, and preserve
  earlier work. This rule applies to claims listed below and in the task briefs.

## State of `main`

- T27 Standard toolbar Print (2026-11-15): the button uses the existing File > Print command
  and its active-window availability, with Origin's measured tooltip "Print the active window".
  A UI regression checks menu/toolbar parity and dispatch without sending a job. Formatting,
  strict all-target Clippy and 2,438 workspace tests pass (two ignored).

- T27 native macOS graph print panel (2026-11-15): File > Print uses PDFKit with the saved
  paper-sized vector PDF, no added margins/scaling/rotation. Construction checks on the main
  thread cover Letter/A4 portrait/landscape and invalid input; standard tests never print.
  Formatting, strict all-target Clippy and 2,437 workspace tests pass (two ignored). Opening/
  cancelling the panel and physical output remain unverified; workbook printing is separate.

- T28 saved-state checkpoints (2026-11-15, macOS, no Origin): undo/redo back to the last saved
  content clears the modified flag and recovery copy. Saved points survive history branches and
  merged edits; recovered projects stay modified until saved. Pending recovery workers schedule
  cleanup polling after a clean state. Latest masking synchronization is retained. Formatting,
  strict all-target Clippy and 2,436 workspace tests pass (two ignored).

- T02 Go To picker/name lookup (2026-11-15, Windows): the flyout lists short names,
  designations and long names, fills an unambiguous short-name/quoted-long-name reference,
  and Reset clears only the column. Typed names ignore ASCII case; repeated long names
  choose the first column. Escape closes the flyout before the dialog. Origin 2016 UI
  probes, four new UI regressions and a parser fixture cover these rules. Formatting,
  strict all-target Clippy and 2276 workspace tests pass (three existing ignored). The
  separate project-wide Column Browser remains unimplemented (gui-look section 5).
- T13 Draw Line in Front/Behind (2026-11-15, Windows): the Line + Symbol choice controls
  both plotted curves and legend samples, defaulting to Behind. Gap to Symbol disables the
  radios and overrides the retained choice, as Origin 2016 controls/PNG exports confirm.
  Native/companion saves preserve it; `.opj` explicitly warns. Formatting, strict all-target
  workspace Clippy and 2,243 tests pass (three existing ignored), including five new regressions.

- CI build size (2026-11-15): dev/test debug information is disabled and the cache prefix
  is reset to `debug0-v1-rust` after the hosted runner exhausted its disk during test linking.
  The format/lint/test commands remain unchanged. Local formatting, strict all-target workspace
  Clippy and all 2,226 tests pass with debug information disabled (three existing ignored).

- T13 Symbol/Line interface (2026-11-14, Windows): Gap to Symbol and No Line Break for
  Skipped Point(s) follow Origin 2016 horizontal, zigzag and three-point diagonal exports.
  Circular clearance is independent of square/circle marker shape; the current fixed 30%
  preference and unmeasured styles remain explicit in graphing 18. Shared scene/picking and
  native/companion saves retain the behavior; `.opj` warns. Formatting, strict all-target
  workspace Clippy and 2,226 tests pass (three existing ignored), including eight new regressions.

- T34 worksheet frozen panes (2026-11-15, macOS, no Origin): View > Freeze/Unfreeze Panes (Ctrl+Shift+F),
  a toggle from a cell, row or column selection (D4 freezes 3 rows and 3 columns, index up to 10); per sheet
  `Worksheet.frozen`, saved in `.sheaf`, `.opj` warns. grid.rs draws frozen rows as extra head rows of the one
  table and frozen columns as an overlay over empty placeholder columns (exact scroll sync, hit guard
  `visible_part`); darker heads, 2 px divider, resizable frozen columns. Rules unmeasured (gui-look.md 3).
- T35 conditional formatting (2026-11-15, macOS, no Origin): Worksheet > Conditional Formatting (also on
  the cell menu) > Highlight Min Max (row-/column-wise, Origin `wcolorbycr` default colours), Highlight Top
  Bottom (Top/Bottom N items or N %), Remove Conditional Format; rules per sheet (`Worksheet.cond_formats`,
  core `condformat.rs`), evaluated once per document revision, kept through duplicate/append/insert/delete.
  T34 frozen panes (same day): View > Freeze/Unfreeze Panes (Ctrl+Shift+F). Both unmeasured (gui-look.md).
- T33 lollipop plot (2026-11-15, macOS, no Origin): Plot > Symbol > Lollipop Plot and `sheaf plot -t
  lollipop` (two or more Y columns, X/Y exchanged, drop lines from each plot to the next at the same
  row, the last back to the first); Plot Details Drop Lines "Drop To" (Axis / Next Data Plot) and "Y
  Value: Plus-Minus" colour; `.opj` save warns. Origin's defaults are unmeasured (graphing.md 18.8).
- T32 multi-page PDF (2026-11-15, macOS, no Origin): several graphs into one PDF, one page per
  graph (Origin 2018b `expGraph` `multipage`). `sheaf_plot::render_multipage_pdf` (each page = that
  graph's one-page PDF export with the same size/margin; sRGB page groups) and
  `render_print_multipage_pdf` (Page Setup sheets); Export Graphs check box "Export as multi-page PDF
  file"; `sheaf export`/`sheaf print` repeated `--graph` and `--all-graphs` (print to `lp`: one job per
  graph); MCP `export_graph` `graphs`/`all_graphs`. Origin's label, file name and page sizing are
  unmeasured (graphing.md 9.7, T32 brief).
- Milestones 1-5 done (M2 waits on the owner's click-through; M5 lacks plot groups/legend frames).
  Owner requests of 2026-11-11 done: parity audit, Origin-style workspace (ADR 0007, most steps),
  `.opj` writer for workbooks and graphs, `.opj` as default save with a `.opj.sheaf` companion for
  what the writer cannot store (ADR 0008). Now in milestone 6 (breadth) plus closing Origin
  differences.
- The owner's daily workflow (CH Instruments import, CSV, overlay plots, integration, fitting,
  figures) works end to end.
- T13 follow-ups (Windows, 2026-11-14): Data on Top of Axes (`0b9bd4e`), Skip Points
  (`ef9ed77`), data-label White Out, Individual X/Y multipliers and label Frame/Margin are merged.
  Point decorations share the measured Origin 2016 frequency. White Out rotates with text;
  Box/Shadow frames enclose it in page-aligned boxes with configurable border, fill and margin.
  Font metrics outside the measured Arial cases remain approximate. Multipliers transform the
  drawing while errors, labels and calculations retain source units; reader/gadget positions
  follow the curves. Formatting, strict all-target Clippy and 2,142 workspace tests passed
  (three existing ignored), including seven new frame regressions and a separate autosave UI
  source-fixture isolation fix. See the T13 Status.
- Tests: 2588 passed (two ignored) on `mcp-attach` on macOS on 2026-11-15 after merging main (T25 phase 2 with saved-state undo checkpoints, masking, print panel); formatting and strict all-target Clippy passed. Before that: 2219 passed (one ignored) on `excel-io` on macOS on 2026-11-14 night after merging main (T30 Excel with T27 CLI print, T13 label frames); formatting and strict all-target Clippy passed; parity 48 done, 140 partial, 50 missing of 238 rows. Before that: 2119 passed (one ignored) on macOS after T27 CLI graph printing on 2026-11-14; formatting and strict all-target Clippy passed. Before that: 2115 passed (one ignored) on `autosave` after merging main (T08 cross-sheet, T25 plot/fit bridge) on macOS on 2026-11-14 night (T28); formatting and strict all-target Clippy passed; parity 48 done, 138 partial, 52 missing of 238 rows. Before that: 1955 passed (one ignored) on `main` on macOS on 2026-11-14 night after the T25 port (`analyze` over MCP reaches every CLI tool, prompts, `.opj` backup link fix); formatting and strict all-target Clippy passed. Before that: 1857 passed (two ignored) on `main` on Windows on 2026-11-14 evening after Envelope, Coherence, Decimation and Trace Interpolation (Signal Processing / Mathematics, analysis-tools 13.2-13.4 and 16); formatting and strict all-target Clippy (toolchain 1.99, two `needless_range_loop` fixes) passed; parity 48 done, 136 partial, 54 missing of 238 rows, P0 32 / 69 / 0. Before that: 1815 passed (one ignored) on `main` on macOS after the T13 look settings, cross tabulation, Dunnett, distribution fit, probability and Q-Q plots and the app icon (2026-11-14); formatting and strict all-target Clippy passed. Before that: 1718 on `main` with filtered plots, the FFT phase-boundary repair and latest FFT Filters. Previous Windows run: 1706 with FFT/IFFT and FFT Filters. Parity: 46 done, 135 partial, 54 missing of 235 rows; P0 32 / 69 / 0 (`python tools/parity-count.py`).
- FFT/IFFT CI portability: phase unwrapping treats half-turn roundoff consistently across FFT backends; the original Origin fixtures remain unchanged (analysis-tools.md 15.1). The Linux-only failure is covered by an adjacent-representable-value regression.
- CI repair (`4b048d1`): the repeated engineering-format failure (`1.0e-24` instead of `1e-24` with one significant digit) is fixed by parsing the decimal scaling power. Strict formatting expectations remain, with a regression across positive and negative decimal powers. The full macOS workspace at that commit passed 1576 tests (one ignored), formatting, and all-target Clippy. Pushes to `main` cancel older CI runs; use pull-request CI for verification while other sessions are pushing frequently.

## In flight (check for recent edits before claiming)

| Branch | Owner session | What | State |
|---|---|---|---|
| `shape-ordering` | Codex macOS | T24 drawn-shape Bring to front/Push to back within a layer, selection safety and undo/redo; app graph actions/view/shape menu and focused tests | claimed 2026-11-15; no clipboard, resizing, model geometry, plot types, cell notes or other sessions’ scope |
| `codex/t02-find-replace` | Codex Windows, t02-find-replace | T02 existing Find in Sheets/Replace layout, defaults and options measured against Origin; find dialog and focused UI tests | claimed 2026-11-15; no Find in Project, grid/Go To, core/expr/history, plot, statistics, printing or other sessions' scopes |
| `axis-breaks` | Claude Windows, "axis-breaks" | T11 more than one axis break (Origin's 1-7 breaks, per-break position), measured Origin break layout; core Axis break list, plot axis/tick drawing, graph_axis.rs Breaks tab, persistence and focused tests | claimed 2026-11-15; Axis fields only, no PLOT/plots.rs draw_plot, graph_details, navigation, stats or other sessions' scopes |
| `codex/t13-fill-sidelines` | Codex Windows, t13-fill-sidelines | T13 Fill Area Sidelines only / Sidelines and Base; plot fill-mode/group copy, draw_fill_under/draw_line boundaries, draw_legend sample, graph_details selector and focused tests | claimed 2026-11-15; no Axis/layout geometry/PDF, mask policy/color, navigation, labels, stats or other sessions scopes |
| `codex/formula-format-defaults` | Codex Windows, formula-format-defaults | T08 omitted/empty numeric-format defaults for text/format expressions; expr formatter/evaluation and isolated formula fixtures | claimed 2026-11-14; fresh-session Origin defaults only, no global Options UI/settings, core model/history, IO, plot, statistics, MCP or other sessions' scope |
| `stats-origin` | Claude Windows, "t14-origin-checks" | T14 item 6: Origin measurements of cross tabulation and Dunnett (statistics.md 15.3), distribution fit, probability and Q-Q plots (16.4); calc crosstab/dunnett/distfit, core analysis/dist.rs and the crosstab/ANOVA runners, their option dialogs, Origin fixtures | claimed 2026-11-14 night; continues 318653a1's scratch/stats-origin probes; no vartest, stats-multi, print, recovery, MCP attach, Go To, symbol gaps or other sessions' scope |
| `vartest2` | Claude Windows, "continue all tasks" | T26 leftovers: Origin measurements of the two-sample proportion test (unpooled, tails, differences, Fisher, small samples), two-sample variance dialog with intervals, power analysis outputs; calc vartest.rs, core var/prop runners, their dialogs, statistics.md 21 | claimed 2026-11-15 |
| `cell-notes` | Claude macOS, "mac-multipdf" | T36 worksheet cell notes (Origin 2022): core per-sheet notes kept through row/column ops and sort, grid corner tab + hover popup, cell menu Add/Edit/Delete Note, note editor | claimed 2026-11-15; core worksheet + sheetops note upkeep, grid.rs body_cell, menus/app/dialogs; no cond-format/frozen/filter/Go To, plot, stats or other sessions' scopes |
| `violin-plot` | Claude macOS, "violin" | T37 violin plot: core PlotKind/Template, plot data/plots/layout (new violin drawing beside the box chart), calc kernel density, Plot > Statistics menu, CLI/MCP template lists, graphing.md 14.22 | claimed 2026-11-15; no fill-under/sidelines, axis breaks, cell notes, worksheet grid or other sessions' scopes |
| `worktree-agent-a79f85b1afb5f0034` | `originrust-3a` | Matrices, heat maps, contour plots, `sheaf matrix` CLI (`7994c3b`..`8c21175`, 48 files, 10k lines) | 4 commits, local only, based on `148e9b1`, not merged; session offline since 2026-11-12 19:00 |

`merge-05b` (worktree `.claude/worktrees/merge-05b`) is session 05b367c2's unfinished merge of the same
branches that are now on `main`; it holds nothing `main` lacks and can be removed.

On 2026-11-13 no other OriginRust session was live, so the finished branches of the other sessions were
integrated into `main` by "Handoff.md token optimization": `text-numeric`, `opj-params-welch` (plus the
Welch interval, `f522bb2`), `opj-groups-frames`, `export-margin`, `sheet-menus` (its Multi-Panel and Y
Error duplicates of T01 dropped), `nlfit-origin`. `axis-range` and `graph-defaults` never got a commit:
they are briefs T20 and T21 now.

`.opju` (`39178fc`, owner's request): workbooks of Origin Unicode projects are read (CLI, File > Open, drag-and-drop); column designations, labels, graphs and folders are not, and about 200 columns of the 19 gallery samples decode wrongly part-way (cut and warned, opju-format.md 5.4, 9). Sheaf cannot write `.opju`; a save to that name is refused and points to `.opj`. Small items merged on their own (2026-11-13, Windows): Edit > Find in Sheets (Ctrl+F) and Replace (Ctrl+H), File > Append and Close, Window > Duplicate, View > Status Bar, Worksheet > Clear Worksheet, the Linear/Spline/B-Spline Interpolation menu items and Interpolate/Extrapolate Y from X, Analysis > Mathematics > Simple Column Math (Origin's `vmathtool`, analysis-tools.md 9.1), Analysis > Signal Processing > Convolution and Correlation (Origin's `conv` and `corr1`, analysis-tools.md 13), Level Crossing (Origin's `levelcrossing`, analysis-tools.md 14), Hilbert Transform (13.1), FFT, IFFT and FFT Filters (Origin's `fft1`, `ifft1`, `fft_filters`, 15), Analysis > Data Manipulation > Reduce Duplicate X Data (measured against Origin's `reducedup`, analysis-tools.md 12.1), TGA and PCX export, Origin's blue for floating columns. Merged late on 2026-11-13 (session "Handoff.md token optimization", the other sessions idle since 13:27-18:06): `origin-checks-graphs` (`05aa191`: polar, radar, stacked and floating bars measured; floating plots chain their columns), `origin-checks-data` (`b7e6c7d`: impASC window, short names, Day of Week, Month, Engineering, Time columns measured), `stats-more2` (`d724c40`: T14 item 5 Holm comparisons, power and sample size), and on `main`: T13 item 1 (Drop Lines and Label tabs, Fill Area Under Curve, line Transparency, layer Background; Origin's Plot Details measured, graphing 18), the Plot Details tree names, Group tab (18.1), Offset, Display, Size/Speed (units) and page tabs (Print/Dimensions, Display, Legends/Titles), the Workbook button, Reduce to Evenly Spaced X rebuilt as Origin's `reduce_ex` (analysis-tools 12.2), Average Multiple Curves with Origin's defaults and names and Intersect names (analysis-curve-math 4), a drag-selection regression test. Merged from briefs: T22 (`734a1ff`: Average Multiple Curves, Intersect, Polygon Area; `analysis-curve-math.md`), T23 (`f66dbea`: Subtract Straight Line, Reduce to Evenly Spaced X, Curve Translate X/Y, Median Filter; analysis-tools.md 12), T24 (`542fef7`: drawn lines, arrows, rectangles, ellipses; graphing.md 17), each reviewed by independent agents and the confirmed findings fixed before merge; T12 radar, ternary and Origin's two pies (spec 14.19-14.21: radar as a polar layer with spokes, ternary as `Coords::Ternary`, 3D Color Pie and hatched 2D B&W Pie replacing "2D Color Pie"), T12 polar and the horizontal bars (`861166a`, `6033098`: polar theta(X) r(Y) and r(X) theta(Y) through a layer-level `Layer::coords`, spec 14.18; Stacked Bar, 100 % Stacked Bar, Floating Bar), T19 items 1 and 3 (`7ab562a`, `7c8bf12`: multi-character and fixed-width delimiters, Data Import Options, Origin's mode names, Apply; Column Properties Text & Numeric, Month, Day of Week, Engineering, Decimal:1000, 19 date and 14 time patterns), T15 items 4-6 (`e525575`: Simplex, Compare Models checked on 24 Origin pairs, Rank Models, multiple linear regression checked on 6 Origin cases, linear fit with X error by York's method, not compared with Origin), T16 item 2 (`94063b4`: Spline as Origin, natural cubic for increasing X up to 900 points, B-spline and Bezier), T08 remaining (`d8d1416`: `sum` is a running sum and `count` counts missing cells as in Origin, text values and string functions, local `now()`), T15 items 1-3 (`704a1d6`: direct weights, scale-errors switch, multi-dataset and global fits, quick fits), owner-reported `.opj` graph fix (`e908922`: text objects placed from their stored box as Origin turns them, group colours from the records, objects off the page hidden; opj-format.md 10.4, 10.9), T12 items 3-5 and a flat pie (`plot-types-more`: stacked and 100 % stacked area, fill area, histogram, box chart, bubble, colour mapped, pie; Origin's per-template colour lists), T06 (`opj-graph-rest`: X offsets, waterfall as offsets, month/day/year ticks, window places in written `.opj`), T10 (`worksheet-ops`: insert/delete rows, columns and cells, move, hide, width, Origin's sort rules, nested sort, masks, a live filter, transpose/stack/unstack/split/reduce, categorical, Group and Subject), T11 (`axis-scales`: eight more scales, Reverse, First Tick, Prefix/Suffix, an axis break, special ticks, tick label formula, minor tick labels, tick colour, Exchange X-Y, the axis dialog's Apply To and Bottom/Top/Left/Right pages), T12 items 1-2 (`d0dbb37`), T03 (`859c78e`; analysis results that recalculate are named in `.opj` save warnings), T05 (`c32e585`), T21 (`b015f97`), T20 (`522c625`), T14 items 1-4 (`c44a1a6`), T01 (`a2fb77b`), T08 (`04a8a80`), T07 (`826f390`), T17 part (`e209068`), T02
(`bbdb60a`), T09 (`20c37e1`), T16 part (`b967898`). See each brief's Status for what is left.

Worktrees left on 2026-11-13: `axis-range2` and `stats-breadth` (session "Handoff.md token
optimization", free for reuse with their build folders `target/wt-axis` and `target/wt-stats`),
`axis-scales` and `stats-more2` (session "Rust prompt handoff"), the matrix worktree (keep), and
`merge-05b`, an unfinished merge of branches that are all on `main` already (abort it, do not push it).
The disk is about 96 % full; reuse a worktree and its build folder instead of making new ones.

Merged on 2026-11-14 (session "mac-integrate", macOS) after the new `main` was merged into each, main's
Origin-measured versions winning wherever both had a feature: `plot-details` (T13 look settings ported onto main's
Plot Details as Sheaf's own, unmeasured, graphing 18.6: fill hatch/gradient/transparency, bar Width (in %), label
column/rows/decimals/offsets, subgroups by size, background gradient and border, X/Y Axes switches, page border,
icons, comments, title, Miscellaneous tab; the branch's second Layer Contents/Plot Setup/Group tab dropped),
`stats-compare` (cross tabulation and Dunnett in the one-way ANOVA, statistics.md 15; its Holm and 2D binning
dropped for main's) and `stats-dist` (distribution fit, probability and Q-Q plots, statistics.md 16; its power
module dropped for main's). All three are scipy-checked or unmeasured, not Origin. Each had adversarial review
and fixes before merge. Known leftovers: Dunnett runs on the UI thread (about 8 s at 30 groups); hatch cost is
O(lines x edges) on a very long fill-under run; Dunnett at s = 1 above 5000 df has no reference fixture.

App icon (owner's request, 2026-11-14, branch `app-icon`): `tools/make-icon.py` builds the icon from the
owner's bronze S (`docs/brand/sheaf.png`) into `crates/app/assets/icon/` (PNG, `.ico`, `.icns`); the app embeds
`sheaf-256.png` as its window, Dock and taskbar icon. `tools/make-macos-app.sh --install` builds `Sheaf.app` into
`~/Applications` (Spotlight, Launchpad, Dock). T31 (Windows, 2026-11-14) embeds the existing `.ico`
in `sheaf-app.exe` resources through a target-gated `build.rs`; all seven built-PE frames match the
source and Windows extracts the large/small icons. `tools/check-windows-icon.py` verifies the
actual executable (see `docs/spec/windows-packaging.md`). Formatting, strict all-target Clippy
and 1,973 workspace tests pass (two ignored). Left: a Start Menu shortcut and opening files
double-clicked in Finder (Apple open-document events; the bundle declares no document types until then).

**T29 leftovers merged (2026-11-15, session "continue all tasks", branch `pca-plots`):** PCA draws Origin's four graphs (Scree, Loading, Scores, Biplot; biplot loading axes by Origin's scaling rule, 13 of 14 measured axes equal), the PCA, k-means and Kaplan-Meier result sheets follow Origin's report tables and labels, and Kaplan-Meier curve limits are Origin's linear S -/+ z se, not clipped (statistics.md 17, 18, 20). Known flaky: the `sheaf-calc` test binary `origin_stats2_tests` crashed twice (exit 0xffffffff, no failed assertion) under full parallel `cargo test --workspace` load; reruns and 240 stress runs passed; cause unknown.

**T27 workbook printing merged (2026-11-15, session "continue all tasks"):** with a workbook active, File > Print and Print Preview print the active sheet as a table (head, label rows, visible rows and columns as displayed, grid, "Page n of m"), rows down then columns across, multi-page EMF on Windows with a page range, multi-page PDF elsewhere (graphing 19.8; Origin's workbook Print dialog measured, its printed layout not). On macOS a graph uses the PDFKit panel and a sheet goes to `lp`; **the macOS build of this merge was not compiled (Windows session): a macOS session should run `cargo clippy --workspace --all-targets` once.** Note: Windows' "manage my default printer" made Microsoft Print to PDF the default after the headless checks; printing once to the HP printer switches it back.

**T26 merged (2026-11-15, session "continue all tasks", continuing fe4b37eb's 7ec9e48/5a08edf):** Statistics > Hypothesis Testing > One-/Two-Sample Test for Variance and One-/Two-Sample Proportion Test, in Origin's menu order, with Origin's dialog controls and report layout, `sheaf analyze vartest|proptest [--pooled]` (statistics.md 21). Measured: `vartest1`/`vartest2` over tails and alphas, the variance dialog reports, 13 one-sample proportion runs (normal z clipped at 0, exact p, Wald and Clopper-Pearson); the two-sample proportion test in one run only. Exact tests refuse more than 2e6 trials. Left (T26 Status): the rest of the two-sample proportion cases and Fisher, the two-sample variance dialog with intervals, power analysis, raw/indexed input.

**T29 merged (2026-11-15, session "continue all tasks"):** Statistics > Multivariate Analysis > Principal Component Analysis and K-Means Cluster Analysis, Statistics > ROC Curve and Survival Analysis > Kaplan-Meier Estimator, with dialogs, report sheets, graphs (scree, ROC, survival with censor marks, cluster scatter) and `sheaf analyze pca|roc|kaplan-meier|kmeans` (statistics.md 17-20). PCA, Kaplan-Meier (product limit, Greenwood, quartiles, log-rank family) and k-means (Origin's initial centres, Hartigan-Wong moves; 9 Origin runs to 1e-12) are measured against Origin; ROC is scipy-checked (Origin's ROC is menu-only). Left (T29 Status): Origin's report and graph layouts, PCA loading plot and biplot, KM curve limits and Cox regression, k-means missing values and options.

**T27 Print, Windows half merged (2026-11-14, session "continue all tasks"):** File > Print... (Ctrl+P) prints the active graph page through the system dialog as vector EMF placed by `PageSetup::place` (Letter, Automatic orientation, 1 inch margins by default, as Origin's Page Setup showed); Print Preview draws the same placement; Page Setup is saved in `.sheaf` and the `.opj.sheaf` companion (graphing.md 19). A headless print to Microsoft Print to PDF matched the placement exactly. Left (T27 Status): workbook printing, the macOS print panel (the `lp` fallback is untested), `sheaf print`, Origin's own Print and Print Preview windows (unmeasured).

**T38 recent files merged (2026-11-15, macOS session "mac-recent"):** File > Recent Projects and File > Import > Recent Imports (10 entries each, newest first, `recent.json` beside `autosave.json`, re-read and merged under a lock so several instances share it; Clear items; a missing or non-regular file is reported and dropped). Recent Books/Graphs/Exports stay grey. Origin's list details unmeasured (`docs/spec/recent-files.md`).

**T25 phase 2 merged (2026-11-15, macOS session "mac-attach"):** Tools > AI Assistants (MCP) or `sheaf-app --mcp` lets `sheaf-mcp --attach --root <folder>` drive the open app: each change is one undo step named "AI: <tool> <target>", windows open as usual, a status-bar chip "AI: connected (n)" names the client and its roots. Unix socket in a 0700 per-user folder; loopback TCP plus a token on Windows (named pipes not chosen: unverifiable from macOS). Headless `sheaf-mcp` now runs on a `Document` too (formula columns recalculate after edits; `fit` is atomic). Attached exports never overwrite files and refuse the project's own files. ADR 0013, `docs/spec/mcp.md` section 11. **Windows check needed:** a second user account must not read `mcp.json`; no firewall prompt for the loopback listener.

**T30 Excel merged (2026-11-14 night, macOS session "mac-excel"):** File > Import > Excel (XLS, XLSX, XLSM)... (sheets, hidden sheets, header rows as the ASCII import, import modes, partial import, preview), Export > Worksheet as Excel..., drag-and-drop, `sheaf import/info/export/export-data` with `--sheets`/`--hidden-sheets`, MCP `import_files`; `calamine` and `rust_xlsxwriter` (ADR 0012). **Measured on Windows (2026-11-15, session "t30-origin")** against Origin 2016's `impExcel` (probes in `re-workspace/scratch/t30/`, import-excel.md 2) and copied where Sheaf differed: hidden sheets are read by default (`--skip-hidden-sheets`, untick in the window), the book long name keeps the extension, leading empty columns are kept, dates among text stay dates in a Text & Numeric column; Sheaf still detects header rows (Origin reads none) and keeps Excel's dates before 1900-03-01 (Origin's are a day early). Brief step 6 done: three workbooks on the Windows machine agree cell by cell with `openpyxl`. Left: Origin's import window and export (import-excel.md 10).


**T27 CLI graph printing (macOS, no Origin, 2026-11-14):** `sheaf print PROJECT
[--graph NAME] --output FILE.pdf` writes a paper-sized vector PDF using the project's saved Page
Setup and `PageSetup::place`. Without `--output`, macOS/Linux submit it through `lp` to the default
or `--printer NAME` destination; Windows currently supports PDF output only. No print job is sent
by automated tests (a fake `lp` verifies arguments and cleanup). The PDF's transforms and paper
size, invalid settings and source-preserving atomic output are covered. Native macOS panel,
workbook printing, Windows CLI submission and hardware printer checks remain open. See T27 and
graphing.md 19.6. Formatting, strict all-target Clippy and 2,119 workspace tests passed (one ignored).

**T28 autosave merged (2026-11-14 night, macOS session "mac-autosave"):** Tools > Autosave... (on, 10 minutes, Sheaf's choice; Origin's defaults unmeasured) writes the modified project as `.sheaf` into the platform recovery folder on a worker thread; a crashed run's copy is offered at the next start (Recover / Discard / Later); a clean exit or normal save removes it, 30-day-old leftovers are pruned. Liveness by a held lock file per running instance (`sheaf_io::recovery::Session`), not process ids. `--recovery-dir DIR`/`--no-recovery` for checks; UI tests never touch the real folders. Spec `docs/spec/autosave.md`; leftovers in the T28 Status.

**T25 phase 1 merged (2026-11-14, Windows session 318653a1):** `sheaf-mcp` (crate `crates/mcp`) runs over stdio on the shared `sheaf-ops` crate that now holds the CLI's import, plot, export, analyze and fit work; setup in README and `docs/spec/mcp.md`, leftovers in the T25 Status. Extended the same night (macOS session 'mac-mcp', branch `mcp-port`): the CLI's clap options (`sheaf_ops::args`) and `job_from` (`sheaf_ops::cli_analyze`) moved into `sheaf-ops`, so `analyze` reaches all 55 `sheaf analyze` tools (21 hand-mapped, 34 through `crates/mcp/src/bridge.rs`, schemas generated from the CLI's options); MCP prompts; `backup_original` never writes through a link; shared long names refused instead of taking the first. The CLI was checked byte for byte against the previous `main` on 325 invocations (identical). That session had also built a second, separate MCP server (local branch `mcp-server`, never pushed, deleted); the owner chose to port its useful parts into this one. Lesson: claims in the local peer notes are invisible to the Windows machine; claim a brief in HANDOFF on `main` (pushed) before starting. Plan as written:

**Owner's request (2026-11-14): an MCP server so Claude or any AI tool can use Sheaf.** Brief
[T25](docs/tasks/T25-mcp-server.md). Plan in short:
1. ADR 0011: stdio transport, the Rust SDK `rmcp` (or a small JSON-RPC loop with `serde_json`), one
   `Project` per server session.
2. Move the CLI's command bodies into a shared library path so `sheaf-cli` and the new `sheaf-mcp` give
   identical results (no second copy of import, plot, analysis or fit logic).
3. New crate `crates/mcp` (binary `sheaf-mcp`, no egui): tools `new_project`/`open_project`/`save_project`,
   `import_files`, `list_contents`, `read_columns`/`set_column_values`, `plot`, `export_graph` (PNG returned
   as an image the assistant can see), `list_analysis_tools`/`analyze` (the serializable `Tool`/`Request`),
   `list_fit_models`/`fit`; resources for the project tree, columns as CSV and graph images.
4. Safety: file access only under `--root` folders, no script or shell execution, paged data, stdout kept
   for protocol messages only.
5. Tests: start the server, run import -> plot -> analyze -> fit -> export -> save over the protocol and
   compare with the CLI on synthetic data. Docs: `docs/spec/mcp.md`, README setup for Claude Code
   (`claude mcp add sheaf -- <path>/sheaf-mcp --root <folder>`), Claude Desktop and other clients;
   bundle `sheaf-mcp` inside Sheaf.app.
6. Phase 2 (separate merge, ADR first): the desktop app listens on a local socket (off by default);
   `sheaf-mcp --attach` drives the open project, every change one undo step, so the owner sees and can undo it.

**T25 plot/fit bridge (macOS, no Origin, 2026-11-14):** MCP `plot` and `fit` accept a
clap-derived `options` object as well as their existing structured arguments. Both frontends use
shared CLI-to-library conversions in `sheaf-ops`; mixed selections are validated together,
duplicate settings are refused, FDF paths stay under the allowed roots, and numeric strings obey
integer limits. CLI-parity, schema and protocol regressions cover the bridge. Formatting, strict
all-target Clippy and 2,057 workspace tests passed (one ignored) after integrating the latest
cross-sheet formula work. Phase 2 attach remains open; new-sheet output is covered below.
No Origin measurements are needed for this interface.

**T25 new-sheet output (macOS, no Origin, 2026-11-14):** MCP `analyze` accepts
`output: new_sheet` for hand-mapped and CLI-bridged tools, through `sheaf_ops::analyze::run_job_to`
and the existing core `analyze_to` path. Source worksheets, including masks and X data, stay
unchanged; result reports name the new sheet and qualified columns. Intrinsic tables and power
keep their own layout. Sort/filter refuse the setting before mutation; the default remains
`next_to_input`. Formatting, strict all-target Clippy and 2,143 workspace tests passed (one ignored).
Phase 2 attach is still separate.

## Known issues and table UI verification

- T01 sorting: Sort Range and range Custom respect selected cell/row bounds. Sort Columns
  Ascending/Descending move selected rows together; Each Column modes sort independently.
  Sort Columns and column-header Custom now use the existing nested-sort dialog. Masks,
  outside rows/columns and Undo/Redo are covered; see workbook-model.md 13.1.

- Filtered table navigation: keyboard movement, edit commits and Enter/Tab within blocks skip
  hidden rows/columns. New/imported-table regressions ensure hidden values are not edited;
  filter changes repair the active cell (workbook-model.md 13.3).

- T10 filtered plots: graphs, exports, automatic axes/offsets and histogram/box/pie summaries
  skip worksheet-filtered rows. Masks, labels and error bars retain source-row references;
  the Data Reader skips hidden points. Filter Disable/Remove and Undo/Redo restore the view.
  Category/spoke compression, filter overrides and graph gadgets remain open (workbook-model 13.2a).

- T07 clipboard: quoted table fields keep commas within one cell and preserve tabs, line breaks
  and quotes through Copy/Cut/Paste. Both new and imported-table UI paths cover Undo/Redo.
  External CSV/TSV paste detects separators outside quotes; Origin Copy defaults and pasting
  into other applications remain unmeasured (workbook-model.md 13.4).

- T05 PSD graph export: Export Graphs > Photoshop Document and CLI/API `.psd` output support
  RGB, grayscale and 256 colors with DPI metadata. The image is flattened onto white, without
  editable graph layers. PSD dimensions are limited before pixel allocation; graphing.md 9.4
  records the published references and limits. TIFF now also offers 256 Colors, with the
  selected None/PackBits/LZW compression and DPI retained; Monochrome, 16 Colors and 16-bit Color (BMP, TGA) added 2026-11-14 with Origin's per-format lists, ordered dither and 5-5-5 layout (graphing 9.4).

- T19 import separators: **Other** reads a set of delimiter characters, with optional
  **Treat Consecutive Delimiters as One**; **Multiple Characters** reads a whole sequence.
  CLI equivalents: `--delimiter-chars` / `--combine-delimiters` and `--delimiter` respectively.
  Explicit modes handle custom-delimiter quoted multiline fields; quoting, empty-field choices
  and the remaining Auto-detection bootstrap limitation are in import-ascii.md section 2.
  Partial Import now selects first/last rows and columns plus repeating read/skip patterns, with
  source labels and parameter values preserved (shared wizard controls and CLI; section 9).
  Batch scope **All files except first** (CLI `--partial-except-first`) keeps the first data file
  full while selecting later files; preview and final import share the same options. Custom
  range lists remain open.

- T08 text substitution: `substitute(s,new,find[,n])$` follows Origin's documented argument order;
  omitted/zero occurrence replaces all, a positive integer chooses one. Function > Text exposes it.
  Text-returning functions accept `$`; worksheet regressions cover Auto recalculation and undo.
  Built-in column headers (`G/L/U/C/O`, e.g. `col(A)[U]$`) now support text/numeric reads,
  Auto/Manual recalculation after header edits, and self-label references without false cycles.
  `char(n)$` and `code(s)` create characters and read code points (Function > Text); the
  published references, Unicode bounds and unmeasured edge choices are in expression-language.md section 3.
  Typed `format(x,fmt)$` shares Text's explicit formatter; formatting columns work with both names.
  `text(col(A))$` and indexed-cell Text calls use the numeric source column format and update after
  format edits. Origin global/`@SD` defaults remain open; explicit empty formats stay general.

- [x] **Single yellow-cell selection** (fixed 2026-11-13): labels have their own `Sel::Label`
      selection, with a border around just that cell; column letters still select columns.
      Typing/F2, navigation, clipboard/clear, Escape, and undo work without targeting data beneath
      the label. Formula cells keep Set Column Values. See workbook-model 13.3 and `owner_regression_*`.

- [x] **Cell shortcuts and interaction logic across all tables** (fixed 2026-11-13):
      shared label/data selection and focus handling is covered through both File > New and CSV
      import. Regressions exercise empty-cell mouse selection, typing, Enter/Tab/Shift+Tab,
      navigation, clipboard/paste growth, clear, and undo on both paths. Specific newly reported
      interactions can be added to this coverage; the unconfirmed drag observation stays below.

- [x] **Reversed typing (`a` then `b` became `ba`)** (reported/reproduced/fixed 2026-11-13):
      a replacement edit reused an old caret at zero. Each new data/label edit now has a fresh,
      cell-specific editor state with the caret after the initial text. Tests cover both table
      creation paths, stale carets after Home/Escape, Unicode input, and cursor movement during an edit.

### Table UI walkthrough (2026-11-13)

Scope: table interactions only, using synthetic data in a separate Sheaf instance. Checked the
running release build; its exact commit was not verified and it was not rebuilt for this pass.

- [x] **Fixed: Tab after editing loses worksheet keyboard focus.** Reproduction: click A1
      in a new table, type `8`, press Tab, then type `9`. A1 commits as 8 and the selection
      outline moves to B1, but the next digit is ignored. Shift+Tab and arrow navigation also
      stop controlling the table until a cell is clicked again. Reproduced twice in the new
      table and also in an imported CSV table (click A1, type `6`, Tab, then `9`). Expected:
      committing with Tab leaves keyboard focus in the next cell so entry and navigation continue.
      Reproduced in the UI harness and fixed: both editors retain Tab instead of letting egui's
      widget traversal steal focus. Regression checks successive typing after Tab and Shift+Tab
      on new and imported tables.
- [ ] **Needs confirmation: mouse drag selects only the final cell.** In an imported table,
      dragging from A1 to B3, then from A1 to B5, left only B3 or B5 selected rather than the
      rectangular range. Observed in two automated drag attempts; confirm with a manual drag
      before treating it as an application defect, since input timing could be involved.
      2026-11-13: the UI harness selects the block with every event timing tried (one jump, jump and
      release in one frame, press and jump in one frame; `a_drag_over_cells_selects_the_block_whatever_the_event_timing`),
      so the automated drags were most likely the cause; still worth one manual drag by the owner.
      Fast press-and-move frames select the correct block in new/imported-table UI regressions;
      the original release-build observation was not reproduced as an application defect.
- [x] With grid focus, Shift+arrow keys select a rectangular block in the imported table.
      Ctrl+C/Ctrl+V copied and pasted a 2-by-2 block; Ctrl+Z removed the paste in one step
      and Ctrl+Shift+Z restored it. Typing in a new-table data cell and committing with Tab
      worked up to the subsequent focus failure described above.
- [ ] Continue table UI coverage: row/column operations, sheet controls and switching, and
      the remaining new-table/imported-table shortcut cases. These were not completed.

## Next step

Pick the first open brief in `docs/tasks/README.md`. Suggested order by effect on the owner's figures and
analysis: T25 (MCP server, owner's request; phase 1 needs no Origin), T13 leftovers (other fill modes, Offset multiplier, Data on Top of Axes, several groups per layer, Binned increments, X:Y ratio and speed mode, `.opj` writing of the 18.6 settings), T14 item 6 (2D binning, discrete frequency, cross tabulation, Dunnett, distribution fit, probability and Q-Q plot are done; left: the Windows Origin measurements of cross tabulation and Dunnett in statistics.md 15.3 and of the distribution fit and plots in 16.4, since all are scipy-checked, not Origin), T16 item 6 (done 2026-11-14: Origin's automatic B-spline factor, analysis-tools 5), T18 backlog items that need no Origin
(briefs first), merging the matrix branch (asks the owner first: it is another session's unpushed work), T19 item 2 and T04 (wizards, need Origin pages), then
the new briefs written from the T18 backlog on 2026-11-14: T30 (Excel import and xlsx export, P1; the owner's data is often in Excel), T27 (Print, P1), T28 (autosave), T29 (PCA, k-means, ROC, Kaplan-Meier); T26 (variance and proportion tests) is being built by another session, then
the leftovers named in each brief's Status (T22-T24 leftovers: level crossing done 2026-11-14, analysis-tools 14, and cumulative sum dropped, Origin has none; drawn
objects read from `.opj`, page/scale attachment, copy/paste, ordering, resize handles). All earlier "open
questions" are assigned to briefs; none waits on the owner.

## Commands that verify the state

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p sheaf-app -- data.csv --screenshot window.png
cargo run -p sheaf-cli -- plot a.txt b.txt -t cv --panels stack -o cv.png
```

Build with `CARGO_TARGET_DIR=target/<your-folder>` and `CARGO_PROFILE_DEV_DEBUG=0`; the disk was
about 88 % full, check `df -h /c` before parallel builds.

## Needs from the owner

0. Done 2026-11-13: text exports of the owner's six runs were made with the CHI627E program (opened only to
   convert, instrument off) and compared with the `.bin` reader; three reader bugs were fixed (alternating
   channels, CV axis from Init E, time from 0; `docs/spec/chi-binary-format.md`). The owner confirmed the
   i-t runs were made on the CHI1040C. The local test `local_binary_files_with_a_text_export_give_the_same_workbook`
   compares every `.bin` with a `.txt` beside it.
1. **Try the app** (`cargo run --release -p sheaf-app`): import a real CH Instruments file, plot,
   right-click and double-click the graph, try Analysis > Integrate and Fit; report what is wrong.
   (An agent ran the CLI and the app's screenshots on the six real files on 2026-11-13: imports, plots,
   integrate, fit and .opj/.sheaf round trips agreed with independent checks; a person's click-through is still open.)
2. Example figures you make most often, to match their style.
3. How you choose integration limits in Origin (rows, potential, or dragging on the graph).
4. Charge/discharge curves: against time or capacity, and per gram (needs the mass)?
5. May the remote branch `chi-text-real` be deleted? It holds a few numbers from your
   charge-discharge file in a spec paragraph (the copy on `main` does not).
6. Done for the Peak Analyzer (2026-11-13: opened through its menu with `origin-dialog.ps1 -MenuClicks`, T04); the
   Import Wizard pages were measured the same way (Ctrl+3, T19 item 2).
7. **Origin checks for the macOS session's work**: done on 2026-11-13/14 on the Windows machine for polar, bars,
   radar, ASCII import, Column Properties (merged branches `origin-checks-*`), ternary (graphing 14.20), pies (14.21),
   curve mathematics (analysis-curve-math 4), Reduce to Evenly Spaced X (analysis-tools 12.2) and the drawn-object
   tools (graphing 17), Peak Analyzer and Import Wizard pages, Layer Contents, Plot Setup, partial import and
   quoted fields (import-ascii 16). Left: the scipy-checked statistics (cross tabulation and Dunnett, statistics.md 15.3; distribution fit, probability and Q-Q plots, 16.4) and the T13 look settings of graphing 18.6 (hatch list, gradient, bar width, label options); avecurves spline ends and a turning X; the interactive Subtract Straight Line and Curve
   Translate on the graph; Shift snapping and the ellipse tool; ternary title placement.

## Local-only resources

- `re-workspace/scratch/`: scratch scripts, raw Origin outputs (`audit-2/`, `audit-3/`, `audit-3b/`
  hold the measurements cited in the audit), synthetic files (`gen_chi.js` writes CHI test files),
  screenshots; `peer-notes.md` is the note file between sessions.
- The owner's `.opj` and `.bin` files are in the repo root (git-ignored, read-only inputs).
- No Ghidra project and no saved REA evidence bundle exist.
- Origin is driven through COM by `tools/origin-ref/` (system-wide lock: one Origin at a time). A
  hidden Origin started by an agent may still be running.
