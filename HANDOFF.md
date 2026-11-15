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
- **Claim before you start, and put the claim on `main`:** before touching any code, docs or
  spec for a task, add a claim line (task, branch, worktree, session, date) to this file and to
  `re-workspace/scratch/peer-notes.md`, **commit the claim and push it to `main` first** (a docs-only
  commit; `git fetch` and rebase right before the push), and only then begin. Other agents and other
  computers only see work that is on `main`; a claim that exists only in your worktree or in the
  local peer notes does not count. If someone else's claim already covers the task, pick another.
  When the work is finished and merged to `main`, release or update the claim in the same push.
- **Inactive task claims:** If a task has had no edits for at least 48 consecutive hours,
  another agent may take ownership. Check the task brief, its branch and worktree when
  available, and the latest peer notes for evidence of newer work. Record the takeover in
  `re-workspace/scratch/peer-notes.md` with the date, new owner, branch and scope, and preserve
  earlier work. This rule applies to claims listed below and in the task briefs.

## State of `main`

- T25 proxy disconnect cleanup (2026-11-15, Windows, feature `ef98bf7`): disposing of the
  active link signals its reader and polls cancellation every 250 ms, releasing its socket
  clone even when shutdown does not wake the receive. Live partial frames survive polls;
  cancelled partial replies are discarded. A real-TCP regression omits shutdown and checks
  peer EOF for idle/partial replies. The original ten-second integration assertion is unchanged;
  1,750 stress checks pass. Formatting, strict workspace all-target Clippy and 3,306 tests pass
  (eight existing ignored), with formula formatting, digital filters and ROC integrated.

- T55 IIR and FIR filters (2026-11-15, macOS, no Origin): Analysis > Signal Processing > IIR Filter... (no longer
  greyed; Butterworth, Chebyshev I/II, elliptic; low/high/band pass, band stop; fixed or minimum order; Hz or normalized;
  forward-backward) and FIR Filter... (Window with 17 windows, Least Squares), `sheaf analyze iir-filter|fir-filter`, MCP
  `iir_filter`/`fir_filter`, Recalculate and `.opj` warning. Designs and outputs match scipy.signal (zpk 1e-11, firwin 1e-10,
  firls 1e-8). Left: Origin measurements, coefficient/SOS/ZPK sheets, preview panels (analysis-tools.md 17). Formatting,
  strict all-target Clippy and 3,319 workspace tests pass (five ignored).
- T24 drawn-shape ordering (2026-11-15, Codex): Bring to the front / Push to the back reorder
  shapes within their layer, preserving geometry and style. Selection follows the moved shape;
  changed shape stacks invalidate stale selections, menus and drags across Undo/Redo and Undo AI.
  Properties dialogs close on stack changes; no-op/invalid moves add no undo step. Ordering
  across layers/object kinds remains open; this within-layer scope is Sheaf's choice, unmeasured.
  Formatting, strict all-target Clippy and 3,199 parallel workspace tests pass (five ignored).

- T02 Column Browser selection/block add (2026-11-15, Windows, feature `3208db0`):
  Ctrl/Command and Shift selection, Shift+Up/Down and select-all feed individual Add or
  one contiguous same-sheet block. Noncontiguous/mixed-book selections disable block add;
  Remove deletes the whole lower block. Nine valid isolated Origin cases and five new UI
  regressions cover eligibility, grouping, reference transfer and original-sheet navigation.
  All 29 Go To tests pass; formatting, strict workspace all-target Clippy and 3,032 tests
  pass (eight existing ignored), with Parallel Plot and Windows MCP privacy integrated.
  Exclude, lower row-range editing and saved preferences remain (gui-look section 5).

- T25 Windows endpoint acceptance (2026-11-15): inherited profile grants exposed token-file
  read access to other local accounts. The owned runtime folder/lock now have protected DACLs;
  token files are private from creation, and clients verify/refuse broad or foreign-owned
  descriptors without permission changes. Only the selected runtime objects are changed.
  Native app/proxy checks confirm loopback, token/HTTP refusal, no firewall prompt and clean
  shutdown. Windows Authz denies two non-owner accounts while allowing the owner (effective-access
  checks, without another interactive login). Feature `26e8e43`; five new Windows regressions,
  eleven discovery tests, formatting, strict workspace all-target Clippy and 3,006 tests pass
  (eight existing ignored) with Go To/hexbin integrated. ADR 0013 and mcp.md 11 record the limits.

- T02 Go To Column Browser (2026-11-15, Windows, feature `c941fb0`): Select Columns opens
  a metadata table with project/book/sheet/folder scopes and an added-range list. Add/Remove,
  OK/Cancel and Escape preserve the parent until navigation; typed edits and Reset discard the
  browser choice. Eight isolated Origin probes establish qualified references, original-sheet
  position mapping for other books, and the multiple-range last-column rule. Five UI regressions
  and one folder-scope fixture cover it; all 24 Go To tests pass. Formatting, strict workspace/
  all-target Clippy and 2974 workspace tests pass (eight ignored). Selection/block add is
  covered by the later follow-up; Exclude, row-range editing and saved preferences remain.

- T21 thin-width dashes (2026-11-15, Windows): the shared stroke conversion uses
  verified 0.5/1 pt visible-pattern samples, interpolating into the unchanged normal-width
  ratios at 1.5 pt. Four regressions cover seven styles at two raster resolutions,
  data/legend consistency, normal-width/continuity and SVG/EPS/EMF units. The two
  reviewed goldens, shapes/ternary, change only dash arrays. Origin's rounded caps and
  backend width rounding remain explicit limits (graphing 2.1). Feature `ebcf467`;
  separate one-tag theme TEST isolation repair `3d2da51` fixes a reproduced parallel
  fixture collision without product changes. Formatting, strict workspace all-target
  Clippy and 2,968 tests pass (eight existing ignored); 20 parallel theme runs pass too.

- T21 stock legend defaults (2026-11-15, Windows): new Line/Scatter/Line+Symbol/Column graphs
  use the measured upper-left anchor (0.8223, -0.1044) of the layer and an opaque white
  background. Twelve verified Origin exports include resize/content cases, reopening the
  generated OPJ and a legend hiding the frame/curve beneath it. Automatic can be restored
  in the existing position chooser and is kept in native/companion files; raw OPJ writes
  the fixed rectangle. Other templates and saved corner/manual placements stay unchanged.
  Feature `0051de5`; separate test-only Windows Recent Files path repair `b6e588c` changes
  two fixture helpers, preserving exact assertions and product behavior. Formatting, strict
  workspace all-target Clippy and 2,964 tests pass (eight existing ignored); five new
  regressions and seven inspected SVG goldens pass. See graphing 2.4 and T21 for limits.

- Connect line across Missing Data (2026-11-15, Windows, branch `missing-lines`): the page Display
  tab's switch (off by default, `PageStyle::connect_across_missing`) joins the points on either side of
  rows with a missing value, as Origin 2016 draws it (measured, graphing 18.7b); values that cannot be
  placed still break the line. Formatting, strict all-target Clippy and 2,741 workspace tests pass (8 ignored) on 9bf6240.
- T25 owner Undo AI Changes (2026-11-15, Windows): Edit > Undo AI Changes, also in the
  no-window File Undo/Redo block, undoes consecutive completed AI calls and stops before manual
  work or the history limit. Editor/dialog guards are checked again on dispatch. It uses ordinary
  Undo cleanup/saved-state tracking; Redo restores calls individually. Exports and assistant
  connections stay in place. Formatting, strict workspace all-target Clippy and 2,740 workspace
  tests pass (eight existing ignored), including four new real-proxy/UI regressions, on the
  combined Find, window-switcher, X:Y ratio and axis-break-lines source. Feature `e490370`;
  ADR 0013 and mcp.md 11.19 describe the contract. Windows endpoint acceptance is covered by the Windows follow-up above.

- Lines across axis breaks (2026-11-15, Windows, branch `break-lines`): as Origin 2016 draws them
  (measured, graphing 3.6), a segment joining points on both sides of an X or Y break is left out over
  the band of the break's gap; the page Display tab's "Connect line across Axis Break" (off by default,
  `PageStyle::connect_across_breaks`) draws it through (that case is unmeasured). Also measured: the
  same tab's "Connect line across Missing Data" is off by default, which Sheaf already matches.
  Formatting, strict all-target Clippy and 2736 workspace tests pass (8 ignored) on 1113ff9.
- T02 Find/Replace selected range (2026-11-15, Windows, feature `a8ea895`): the checked
  range option snapshots opening blocks, columns and rows for Numeric/String searches and
  both replacement paths, including conditional missing values. Restricted hits retain the
  saved scope; a successful unchecked search removes it. Eleven successful Origin GUI cases
  and the final option restore, four UI regressions and one matching fixture cover it
  (gui-look section 5, including the visible-selection difference). Formatting, strict
  workspace/all-target Clippy and 2735 workspace tests pass (8 ignored), with T41,
  X:Y ratio and T42 integrated.

- T13 X:Y ratio (2026-11-15, Windows, branch `xy-ratio`): Size/Speed "Link Axis Length to
  Scale with X:Y Ratio" (`Layer::xy_ratio`). Measured in Origin 2016 (graphing 18.7a): ticking it or
  changing the ratio shrinks the frame's width or height to fit; afterwards the width stays and the
  height follows the axis ranges, even past the page. Sheaf copies both; it compares spans in the
  axes' own scale (log unmeasured) and ignores the link with axis breaks, for linked child layers and
  in polar/ternary layers. Also measured: Speed Mode's worksheet limit is ticked at 3000 by default
  (spec table said off; speed mode itself is still not done). Formatting, strict all-target Clippy and 2714 workspace tests pass (8 ignored) on 9200269.
- T11 several axis breaks (2026-11-15, Windows, branch `axis-breaks`): an axis holds up to
  seven breaks (`Axis::axis_breaks`; the former single `axis_break` still reads as one, keeping
  its place). Measured in Origin 2016's Breaks tab and PNG exports (graphing 3.6): Auto Position
  puts break k of n at k/(n+1); the automatic gap is about 1.75 % of the axis length; the mark is
  two strokes rising to the right, centred on the segment ends, about 8 pt across the line; each
  added row is the middle half of what the last break left. Decided without Origin's rule: Sheaf
  keeps the axis tick step across breaks (Origin changed it on two of four data sets, no rule
  found) and offers a step after each break; the Break Half Length unit is unmeasured (typing in
  the scripted dialog did not take), so Sheaf shows one gap in percent of the axis. Formatting
  and strict all-target Clippy pass and 2,666 workspace tests pass (seven ignored) on the pushed
  main 61d568b, which also holds the MCP link admission repair 50b5ecb.
- T25 busy greeting repair (2026-11-15, Windows): an over-limit TCP connection receives its
  busy reason before closing, using a send-half-close and a bounded receive drain. This
  avoids Winsock reset errors caused by unread hello data, while retaining connection limits
  and bounded refusal time. The original regression failed on baseline timing attempt 4;
  the fixed binary passed all 30 attempts plus a 32-refusal/silent-peer integration fixture.
  Separate repair 50b5ecb; no resource/context/proxy or admission policy changes. Combined
  formatting, strict all-target Clippy and 2662 workspace tests pass (seven existing ignored),
  including the latest app-resource merge.
- T02 Find/Replace (2026-11-15, Windows): one tabbed window, selected-display query seed,
  Numeric initial mode with measured comparisons/relative tolerance/missing/sign flags,
  String wildcards and reverse search. `!=` is strict, independently of tolerant equality.
  Replacement remains one undo action; tabs have distinct accessibility roles from actions.
  Origin 2016 GUI fixtures, four new UI tests and four matching fixtures cover these rules.
  Feature e5e0b5a passes combined formatting, strict all-target Clippy and 2662 workspace
  tests (seven existing ignored), with both native previews inspected.
  Active-sheet data only; wider scopes, label rows, cell links and saved dialog history remain
  (gui-look section 5).
- T25 current app context (2026-11-15, Windows): attached `sheaf://app` reports the active
  window/sheet, worksheet or graph selection, Data Reader pin and dialog/edit state, with one-based
  indices and bounded metadata. It reads current UI state without values/editor text, document or
  history changes; headless mode omits it. The existing resource-read wire operation is retained.
  Formatting, strict workspace all-target Clippy and 2,653 tests pass (7 existing ignored),
  including six new context/proxy/UI regressions. ADR 0013 amendment and mcp.md 11.18 document it.

- T13 Inclusive fill mode (2026-11-15, Windows): finite runs fill and outline to their own
  endpoint chords; missing rows separate polygons, and isolated points keep round stroke caps.
  Three corrected Origin 2016 captures verify the sloping/missing/closed-polygon cases. Filled
  styling/legends and native/companion saves retain the mode. Formatting, strict workspace
  all-target Clippy and 2,616 tests pass (seven existing ignored), with four new regressions.

- T13 sidelines fill modes (2026-11-15, Windows): Sidelines only and Sidelines and Base
  use the connecting stroke to the frame bottom, with no fill or internal missing-run sidelines.
  Origin 2016 controls/exports confirm geometry and line legends; group/history and native/companion
  saves retain the mode. Formatting, strict workspace all-target Clippy and 2,609 tests pass
  (five existing ignored), including eight new regressions. Windows MCP portability fixes also
  keep strict Clippy and three displayed-path fixtures compatible with canonical drive paths.

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
  project-wide Column Browser was added in `c941fb0`; Exclude, row-range editing and saved
  preferences remain (gui-look section 5).
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
- T42 Window Switcher (2026-11-15, macOS, no Origin): Ctrl+Tab / Ctrl+Shift+Tab (physical Control on macOS)
  over the windows of the Project Explorer folder, hidden included, most recent first; graph thumbnails rendered
  on open, workbook miniatures; release Ctrl activates, Esc cancels; Window > Window Switcher (gui-look.md).
- T49 Parallel Plot and Parallel Index (2026-11-15, macOS, no Origin): Plot > Categorical > Parallel Plot / Parallel Index
  (`-t parallel`, `-t parallel-index`, `--group`, MCP), one vertical axis per column with its own ticks and a
  long-name-and-unit title, per-axis or shared range, one line per row breaking at missing cells, coloured by a numeric
  column (hexbin's colour scale, now shared) or by group (legend), Plot Details Parallel tab (graphing.md 14.25). Parallel
  Sets, curvature and axis reordering not done.
- T51 Sunburst plot (2026-11-15, macOS, no Origin): Plot > Categorical > Sunburst Plot (`-t sunburst`, `--group`, `--sunburst-start/-hole/-gap/-labels/-order`, MCP),
  the treemap hierarchy as concentric rings (`sheaf_calc::sunburst`), leaves coloured by value and categories by their leaves' mean on the shared
  ramp with a colour scale, unfilled missing categories, radial labels hidden when they do not fit, Plot Details Pie Geometry tab (graphing.md 14.27).
  Defaults are Sheaf's choices, unmeasured; no hover or Collapse/Expand drilling.
- T50 Treemap (2026-11-15, macOS, no Origin): Plot > Categorical > Treemap (`-t treemap`, `--group`, `--treemap-layout/-order/-depth/-gap`, MCP),
  nested rectangles by category columns sized by a value column; six layouts as pure functions in `sheaf_calc::treemap` (squarified,
  squarified+, split, three ordered pivot treemaps) with the hierarchy and gap nesting in `treemap_tree`, Plot Details Layout tab
  (order, depth, gap, per-level layout) (graphing.md 14.26). Defaults, colours and labels are Sheaf's choices, unmeasured; no hover or drill-down.
- T45 hexbin plot (2026-11-15, macOS, no Origin): Plot > Contour/Heat Map > Hexbin (`-t hexbin`, `--hex-bins`, MCP), XY data
  binned into pointy-top hexagons (`sheaf_calc::hexbin`, checked against matplotlib hexbin), coloured by count with the
  colour mapped scatter ramp and a "Count" colour scale, Plot Details Bins tab (graphing.md 14.24). XYZ/matrix input and
  Binned Scatter not done.
- T44 ridgeline chart (2026-11-15, macOS, no Origin): Plot > Statistics > Ridgeline Chart (`-t ridgeline`, MCP), one
  KDE ridge per Y column on baselines labelled with the column names, four display types, Origin's bandwidth rules
  through the shared `sheaf_calc::kde` (grid refined to h/4 near data), quantile lines and rug (graphing.md 14.23).
- T43 trendlines (2026-11-15, macOS, no Origin): per-plot `PlotDecor::trendlines` (Linear, Polynomial 2-10, Power, Log,
  Exp, Moving Average 2-10), drawn with an equation/R2 label; plot menu Add/Edit/Remove Trendline, `plot --trendline`,
  MCP `style.trendline`; `.opj` warns. Power/log/exp are least squares on transformed data, R2 in the fitted space,
  trailing moving average: all unmeasured (graphing.md 18.11).
- T41 rug plots (2026-11-15, macOS, no Origin): Axis dialog Rug tab with Origin's documented controls, strip
  outside the frame with tick labels pushed out, Plot > Symbol > Scatter + Rug (`-t scatter-rug`, MCP); rug
  Data choice is the plot's Y column; histograms mark raw observations; `.opj` warns (graphing.md 18.10).
- T40 Graphical Summary (2026-11-15, macOS, no Origin): Statistics > Descriptive Statistics > Graphical Summary
  (Origin 2026, Separate Graphs arrangement only): per variable/group descriptive statistics, mean (t), median
  (exact binomial) and SD (chi-square) intervals, Anderson-Darling from the existing normality code; one graph per
  variable-group (histogram + normal curve, interval plot, probability plot); `sheaf analyze graphsummary`, MCP.
  Checked against scipy, not Origin (statistics.md 22).
- T39 plot-only X sorting (2026-11-15, macOS, no Origin): Origin 2023 `layer.plotN.line.sortx` as a per-plot
  `LineStyle::sort_x`; line, fill, gaps and No Line Break follow X order, the sheet is not sorted; Plot Details
  Line tab check box, `sheaf plot --sort-x`, MCP `style.sort_x`; `.opj` warns (graphing.md 18.9, unmeasured).
- T36 cell notes (2026-11-15, macOS, no Origin): cell menu Add/Edit/Delete Note (plain-text editor window), a
  dark-red corner tab and a hover popup; per sheet `Worksheet.notes` (core `notes.rs`), kept through sort, row/column
  edits, duplicate and append; `.sheaf` saves them, `.opj` warns. Unmeasured (gui-look.md "Cell notes (T36)").
- T35 conditional formatting (2026-11-15, macOS, no Origin): Worksheet > Conditional Formatting (also on
  the cell menu) > Highlight Min Max (row-/column-wise, Origin `wcolorbycr` default colours), Highlight Top
  Bottom (Top/Bottom N items or N %), Remove Conditional Format; rules per sheet (`Worksheet.cond_formats`,
  core `condformat.rs`), evaluated once per document revision, kept through duplicate/append/insert/delete.
  T34 frozen panes (same day): View > Freeze/Unfreeze Panes (Ctrl+Shift+F). Both unmeasured (gui-look.md).
- T37 violin plot (2026-11-15, macOS, no Origin): Plot > Statistics > Violin Plot / with Box / with Point /
  with Quartile / with Stick / Half Violin and `sheaf plot -t violin` (also `violin-box`, `violin-point`,
  `violin-quartile`, `violin-stick`, `half-violin`; MCP the same): one mirrored Gaussian kernel density
  silhouette per Y column (Scott bandwidth from Origin's documented formulas, `sheaf_calc::kde`, checked
  against scipy), marks inside per variant, masked rows left out, `.opj` save warns. Origin's defaults
  are unmeasured; Split Violin, violin settings and the "Violin with Box" toolbar button are left (graphing.md 14.22).
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
- Tests: 3265 passed (five ignored) on `save-window` on macOS on 2026-11-15 after merging main (T54 with shape ordering, T47/T55 work); formatting and strict all-target Clippy passed. Before that: 3155 passed (five ignored) on `graph-data-tools` on macOS on 2026-11-15 after merging main (T48 with T45, T49-T51); formatting and strict all-target Clippy passed. Before that: 2858 passed (five ignored) on `recent-files` on macOS on 2026-11-15 after merging main (T38 with T39-T44); formatting and strict all-target Clippy passed. Before that: 2588 passed (two ignored) on `mcp-attach` on macOS on 2026-11-15 after merging main (T25 phase 2 with saved-state undo checkpoints, masking, print panel); formatting and strict all-target Clippy passed. Before that: 2219 passed (one ignored) on `excel-io` on macOS on 2026-11-14 night after merging main (T30 Excel with T27 CLI print, T13 label frames); formatting and strict all-target Clippy passed; parity 48 done, 140 partial, 50 missing of 238 rows. Before that: 2119 passed (one ignored) on macOS after T27 CLI graph printing on 2026-11-14; formatting and strict all-target Clippy passed. Before that: 2115 passed (one ignored) on `autosave` after merging main (T08 cross-sheet, T25 plot/fit bridge) on macOS on 2026-11-14 night (T28); formatting and strict all-target Clippy passed; parity 48 done, 138 partial, 52 missing of 238 rows. Before that: 1955 passed (one ignored) on `main` on macOS on 2026-11-14 night after the T25 port (`analyze` over MCP reaches every CLI tool, prompts, `.opj` backup link fix); formatting and strict all-target Clippy passed. Before that: 1857 passed (two ignored) on `main` on Windows on 2026-11-14 evening after Envelope, Coherence, Decimation and Trace Interpolation (Signal Processing / Mathematics, analysis-tools 13.2-13.4 and 16); formatting and strict all-target Clippy (toolchain 1.99, two `needless_range_loop` fixes) passed; parity 48 done, 136 partial, 54 missing of 238 rows, P0 32 / 69 / 0. Before that: 1815 passed (one ignored) on `main` on macOS after the T13 look settings, cross tabulation, Dunnett, distribution fit, probability and Q-Q plots and the app icon (2026-11-14); formatting and strict all-target Clippy passed. Before that: 1718 on `main` with filtered plots, the FFT phase-boundary repair and latest FFT Filters. Previous Windows run: 1706 with FFT/IFFT and FFT Filters. Parity: 46 done, 135 partial, 54 missing of 235 rows; P0 32 / 69 / 0 (`python tools/parity-count.py`).
- FFT/IFFT CI portability: phase unwrapping treats half-turn roundoff consistently across FFT backends; the original Origin fixtures remain unchanged (analysis-tools.md 15.1). The Linux-only failure is covered by an adjacent-representable-value regression.
- CI repair (`4b048d1`): the repeated engineering-format failure (`1.0e-24` instead of `1e-24` with one significant digit) is fixed by parsing the decimal scaling power. Strict formatting expectations remain, with a regression across positive and negative decimal powers. The full macOS workspace at that commit passed 1576 tests (one ignored), formatting, and all-target Clippy. Pushes to `main` cancel older CI runs; use pull-request CI for verification while other sessions are pushing frequently.

## In flight (check for recent edits before claiming)

| Branch | Owner session | What | State |
|---|---|---|---|
| `codex/t05-export-dialog-themes` | Codex Windows | T05 Dialog Theme row: Origin theme-list/save/reuse behavior, named export-settings themes and isolated persistence/UI regressions | claimed 2026-11-15; own `.claude/worktrees/codex-t13-skip-points`, `target/wt-t13-skip-points`; no Graph Theme, preview, renderers/formats, import behavior, recovery, priority docs or other scopes |
| `codex/sheaf-priority-plan` | Codex macOS, cloud-session recovery | Finish paused Claude prioritization docs: archive handoff, PLAN, greyed inventory/counts, T58–T63 briefs, task index, roadmap/parity priorities; no feature implementation | claimed 2026-11-15; worktree `work/OriginRust` in Codex chat; preserves all other claims |
| `codex/formula-worksheet-substitution` | Codex Windows, formula-worksheet-substitution | T08 static worksheet-cell %([Book]Sheet,column,row[,format]) substitution; expr lowering/evaluation, isolated core/IO formula regressions | claimed 2026-11-15; no graph/legend/session/generated-expression substitution, global Options/model fields, UI, MCP, imports, statistics or other owner scopes |
| `t19-fntrim` | Claude Windows, session fe4b37eb | T19 file-name trimming on ASCII import (Origin's FNameToSht/Bk From/To rules) and the book long name keeping the file extension (owner's decision 2026-11-15); crates/io ascii naming, app import window, cli flags, import-ascii.md, title tests | claimed 2026-11-15 |
| `shape-resize` | Codex macOS, continuous-mac | T24 selected drawn-shape endpoint/corner resize handles; app graph_view/actions/shape edits, interaction safety and focused tests | claimed 2026-11-15; worktree `.claude/worktrees/shape-resize`; no clipboard, slope-tool behavior, file-format/model changes or other sessions’ scope |
| `codex/t02-browser-rows` | Codex Windows, t02-browser-rows | T02 Column Browser lower row-range editor: native defaults/validation/Cancel/reference transfer and focused UI tests/docs | claimed 2026-11-15; only go_to/browser and own go_to_browser tests/docs; no Exclude/preferences, original-sheet navigation changes without published refinement, app/MRU/grid/core/history/MCP/plots/stats/fit/formula/labels or other owner work |
| `stats-origin` | Claude Windows, "t14-origin-checks" | T14 item 6: Origin measurements of cross tabulation and Dunnett (statistics.md 15.3), distribution fit, probability and Q-Q plots (16.4); calc crosstab/dunnett/distfit, core analysis/dist.rs and the crosstab/ANOVA runners, their option dialogs, Origin fixtures | claimed 2026-11-14 night; continues 318653a1's scratch/stats-origin probes; no vartest, stats-multi, print, recovery, MCP attach, Go To, symbol gaps or other sessions' scope |
| `cell-notes` | Claude macOS, "mac-multipdf" | T36 worksheet cell notes (Origin 2022): core per-sheet notes kept through row/column ops and sort, grid corner tab + hover popup, cell menu Add/Edit/Delete Note, note editor | claimed 2026-11-15; core worksheet + sheetops note upkeep, grid.rs body_cell, menus/app/dialogs; no cond-format/frozen/filter/Go To, plot, stats or other sessions' scopes |
| `slope-annotation` | Claude macOS, "mac-multipdf" | T52 slope annotation (Origin 2026b): layer SlopeAnnotation linked to two data rows, segment + rise/run legs + slope label (per decade on log axes), toolbar Slope tool with point picking, delete/undo, CLI/MCP | claimed 2026-11-15; core graph.rs layer field, new plot module, app toolbar/graph_view tool mode, small dialog; no drawn-shape ordering, Data Reader changes or other sessions' scopes |
| `t48-origin` | Claude Windows, "continue all tasks" | T48 leftover: Origin 2016 measurements of the graph Data menu (Mask/Remove Bad/Move Data Points, graphing.md 20.4) and Sheaf following them; core graph_data.rs, app data_tools/reader/graph_view Data modes only | claimed 2026-11-15; no shape-resize, slope annotation, browser, stats, MCP or other sessions' scope |
| `deconv-stft` | Claude macOS, "violin" | T57 Deconvolution and STFT: calc signal/new stft module, core analysis tools, Signal Processing menu and dialogs, CLI/MCP analyze, analysis-tools.md 18 | claimed 2026-11-15; no Convolution/FFT/IIR/FIR result changes, no matrices, plot or other sessions' scopes |
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

**T15 surface fits on XYZ columns merged (2026-11-15, session "continue all tasks", branch `surface-fit`):** Analysis > Fitting > Nonlinear Surface Fit... and the NLFit window fit z = f(x, y) from X, Y, Z columns with Origin's 22 Surface Fitting functions; 11 Origin `nlbeginz` runs (Plane, Parabola2D, Poly2D, Gauss2D, Lorentz2D, Voigt2D, Gaussian2D, Exponential2D, Rational2D, missing values) match to 1e-7 from Origin's starting values (Rational2D looser); `FitSurface1`/`FitSurface1Stats` and Origin's 50 x 50 `FitSurfaceCurve1` grid; `sheaf fit --x --y --z` (fitting.md 14-15). Sheaf also adds fitted Z and residual columns, which Origin does not. Left: matrix input, 3D surface plot of the result, Origin's peak-surface starting values, 13 surface functions not run in Origin.

**T26 leftovers merged (2026-11-15, session "continue all tasks", branch `vartest2`):** the two-sample proportion test is measured in Origin over pooled and unpooled z, tails, test differences, small samples and all-success cases; Fisher's exact section, the power analysis outputs (actual and hypothetical power with sample sizes) and Origin's conclusion wording for the proportion and variance tests are in (statistics.md 21). The normal-approximation warning is measured only for fewer than 10 failures; the fewer-than-10-successes half is Sheaf's guess (marked unmeasured).

**T29 leftovers merged (2026-11-15, session "continue all tasks", branch `pca-plots`):** PCA draws Origin's four graphs (Scree, Loading, Scores, Biplot; biplot loading axes by Origin's scaling rule, 13 of 14 measured axes equal), the PCA, k-means and Kaplan-Meier result sheets follow Origin's report tables and labels, and Kaplan-Meier curve limits are Origin's linear S -/+ z se, not clipped (statistics.md 17, 18, 20). Known flaky: the `sheaf-calc` test binary `origin_stats2_tests` crashed twice (exit 0xffffffff, no failed assertion) under full parallel `cargo test --workspace` load; reruns and 240 stress runs passed; cause unknown.

**T27 workbook printing merged (2026-11-15, session "continue all tasks"):** with a workbook active, File > Print and Print Preview print the active sheet as a table (head, label rows, visible rows and columns as displayed, grid, "Page n of m"), rows down then columns across, multi-page EMF on Windows with a page range, multi-page PDF elsewhere (graphing 19.8; Origin's workbook Print dialog measured, its printed layout not). On macOS a graph uses the PDFKit panel and a sheet goes to `lp`; **the macOS build of this merge was not compiled (Windows session): a macOS session should run `cargo clippy --workspace --all-targets` once.** Note: Windows' "manage my default printer" made Microsoft Print to PDF the default after the headless checks; printing once to the HP printer switches it back.

**T26 merged (2026-11-15, session "continue all tasks", continuing fe4b37eb's 7ec9e48/5a08edf):** Statistics > Hypothesis Testing > One-/Two-Sample Test for Variance and One-/Two-Sample Proportion Test, in Origin's menu order, with Origin's dialog controls and report layout, `sheaf analyze vartest|proptest [--pooled]` (statistics.md 21). Measured: `vartest1`/`vartest2` over tails and alphas, the variance dialog reports, 13 one-sample proportion runs (normal z clipped at 0, exact p, Wald and Clopper-Pearson); the two-sample proportion test in one run only. Exact tests refuse more than 2e6 trials. Left (T26 Status): the rest of the two-sample proportion cases and Fisher, the two-sample variance dialog with intervals, power analysis, raw/indexed input.

**T29 merged (2026-11-15, session "continue all tasks"):** Statistics > Multivariate Analysis > Principal Component Analysis and K-Means Cluster Analysis, Statistics > ROC Curve and Survival Analysis > Kaplan-Meier Estimator, with dialogs, report sheets, graphs (scree, ROC, survival with censor marks, cluster scatter) and `sheaf analyze pca|roc|kaplan-meier|kmeans` (statistics.md 17-20). PCA, Kaplan-Meier (product limit, Greenwood, quartiles, log-rank family) and k-means (Origin's initial centres, Hartigan-Wong moves; 9 Origin runs to 1e-12) are measured against Origin; ROC is scipy-checked (Origin's ROC is menu-only). Left (T29 Status): Origin's report and graph layouts, PCA loading plot and biplot, KM curve limits and Cox regression, k-means missing values and options.

**T27 Print, Windows half merged (2026-11-14, session "continue all tasks"):** File > Print... (Ctrl+P) prints the active graph page through the system dialog as vector EMF placed by `PageSetup::place` (Letter, Automatic orientation, 1 inch margins by default, as Origin's Page Setup showed); Print Preview draws the same placement; Page Setup is saved in `.sheaf` and the `.opj.sheaf` companion (graphing.md 19). A headless print to Microsoft Print to PDF matched the placement exactly. Left (T27 Status): workbook printing, the macOS print panel (the `lp` fallback is untested), `sheaf print`, Origin's own Print and Print Preview windows (unmeasured).

**T54 Save Window As merged (2026-11-15, macOS session "mac-savewin"):** File > Save Window As... writes the active workbook as `.sheafbook` or a graph with its source books as `.sheafgraph` (the `.sheaf` zip with a window marker); opening one (Open, drop, command line, Recent Books / Recent Graphs) adds its windows to the current project, never replacing it; links leaving the window are dropped with a note. File > Append now also follows renamed books in formulas. Origin's window files are not read or written; Origin's behaviour unmeasured (window-files.md).

**T53 Set Multiple Columns Values merged (2026-11-15, macOS session "mac-multicol"):** Ctrl+Shift+Q with two or more columns selected sets one formula on all of them, `j` being each column's sheet position (`col(A)*j`, `wcol(j-1)*2`); the formula is stored on each column, recalculates per column (also after columns move), one undo step. A bare `col(j)` still means a column named J when there is one. Origin's `j` rules unmeasured (expression-language 7).

**T48 graph Data menu tools merged (2026-11-15, macOS session "mac-datatools"):** Data > Mask Data Points (rectangle; click unmasks), Remove Bad Data Points (Y cell missing, rows kept), Move Data Points (X and Y follow the drag) on graphs, each one undo step on the source sheet; Auto analysis links now follow mask changes. Display range and markers stay grey. Origin's tool behaviour unmeasured (graphing.md 20.4).

**T47 Simulate Curve merged (2026-11-15, macOS session "mac-simcurve"):** Analysis > Fitting > Simulate Curve... makes X/Y columns (and optionally a graph) from any fit-library function with typed parameters over a linear or log X range or the X of a column, one undo step; `sheaf simulate` and MCP `simulate_curve` give the same numbers. Origin's `simcurve` window and values unmeasured (fitting.md 13).

**T46 import themes merged (2026-11-15, macOS session "mac-themes"):** Origin's Dialog Theme row in the import windows (ASCII/CHI, Import Wizard, Excel): `<default>`, `<last used>` (recorded on OK only) and named themes with Save, Save As, Delete, kept in `import-themes/<kind>.json` beside the other settings; `sheaf import --theme NAME`/`--last-used` (typed flags win). Origin's theme list and defaults unmeasured (import-ascii.md 17.3).

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

## Not available in Sheaf yet (greyed menu items)

Every Origin menu item Sheaf lacks is drawn greyed with the tooltip "Not available in Sheaf yet" (`NOT_YET`,
`grey`/`greys`/`grey_sub` in `crates/app/src/menus.rs`, `g(...)` in `toolbar.rs`). Listed 2026-11-15 by a Haiku agent and
cross-checked by a grep tally. Remove a line when its item is enabled; regenerate by grepping those helpers.

463 greyed items: 319 in the menu bar, 47 on the toolbars, 97 in context menus.

Source: crates/app/src/menus.rs, toolbar.rs and panels/explorer.rs at commit 23ac847 (main after T55; same menus as c0aa34c). Every item here is drawn greyed with the tooltip "Not available in Sheaf yet". Tags in brackets name the windows where the item is greyed when it is not on every bar: [Workbook], [Graph], [no window]. Items greyed only because of the current state (the current plot type under Change Plot to, Hide Other Layers with one layer, Go to Book with no workbook, "(none)" recent lists) are left out.

#### File
- New: Matrix... · Graph · Image · From Template · Excel · Layout · Notes · Function Plot
- Open Excel...
- Open Sample Projects
- Save Template As... [Workbook, Graph]
- Save Workbook As Analysis Template... [Workbook]
- Add Word Bookmarks to Analysis Template... [Workbook]
- Batch Processing... [Workbook, Graph]
- Save Project without Data...
- Duplicate Project without Data
- Import: Image... · Binary 2D Array... · Re-Import Directly · Re-Import... [no window, Workbook]
- Export: Graph as Image... · Worksheet as Image... · Multi-Page PDF... · Recent Exports [no window, Workbook]
- Recent Exports [Graph]

#### Edit
- Copy: Copy formula · Copy label rows · Copy formula and label rows · Copy All [Workbook]
- Paste Link · Paste Special [Workbook, Graph]
- Paste Transpose · Paste Link Transpose · Select... [Workbook]
- Remove Links · Set As Begin · Set As End · Reset to Full Range [Workbook]
- Find in Project... [Workbook]
- Button Edit Mode [Workbook, Graph]
- Cut · Copy · Paste · Clear · Insert New Object · Links · Update Client [Graph]
- Copy Format: All · All Style Formats · Colors · Fonts · Background · Symbol/Line/Fill · Dimensions · Scales · Text [Graph]
- Paste Format... [Graph]
- Find in Project [Graph]

#### View
- Menu
- Command Window · Code Builder · Quick Help · Smart Hint Log
- View Windows: None · Windows in Active Folder · Active Folder and Subfolders
- Actively Update Plots · Page Break Preview Lines [Workbook]
- Print View · Page View · Window View · Draft View · Zoom In · Zoom Out · Whole Page · Zoom All [Graph]
- Show: Layer Icons · Active Layer Indicator · Axis Layer Icons · Object Grid · Layer Grid · Frame · Labels · Data · Active Layer Only · Master Items on Screen [Graph]
- Show Data Info [Graph]
- Full Screen [Graph]

#### Plot
- Line: Horizontal Step · Vertical Step · Spline Connected
- Symbol: Scatter Central · Column Scatter · Vertical Drop Line
- Line + Symbol: Line Series · 2 Point Segment · 3 Point Segment
- Column/Bar/Pie: Column + Label · Grouped Columns (indexed data)
- Multi-Y: 3Ys Y-YY · 3Ys Y-Y-Y · 4Ys Y-YYY · 4Ys YY-YY · Multiple Y Axes...
- Y-offset/Waterfall: Waterfall Y:Color Mapping · Waterfall Z:Color Mapping · 3D Waterfall
- 3D XYY: XYY 3D Bars · 3D Ribbons · 3D Walls · 3D Waterfall
- 3D Surface: Color Fill Surface · X Constant with Base · Y Constant with Base · Color Map Surface · Colormap Surface with Projection · Wire Frame · Wire Surface · 3D Ternary Colormap Surface
- 3D Symbol/Bar/Vector: 3D Bars · 3D Scatter · 3D Trajectory · 3D Scatter + Error Bar · 3D Vector XYZ XYZ · 3D Vector XYZ dXdYdZ
- Statistics: Grouped Box Charts · Interval Plot · Histogram + Probabilities · Stacked Histograms · Marginal Histograms · Marginal Box Charts · 2D Kernel Density · QC (X bar R) Chart · Pareto · Scatter Matrix · Probability Plot · Q-Q Plot
- Contour/Heat Map: Color Fill · B/W Lines + Labels · Gray Scale Map · Heat Map · Polar Contour theta(X) r(Y) · Polar Contour r(X) theta(Y) · Ternary Contour
- Profile: Contour Profiles · Image Profiles
- Specialized: Wind Rose · Piper · Smith Chart · High-Low-Close · Vector XYAM · Vector XYXY · Zoom
- Stock: High-Low-Close · Japanese Candlestick · OHLC Bar Chart · OHLC-Volume · Line
- System Templates...
- Most Recently Used

#### Column
- Fill Column With: Row Numbers · Uniform Random Numbers · Normal Random Numbers · A set of Numbers... · A set of Date/Time values... · Arbitrary set of Text&Numeric values...
- Set Sampling Interval... · Show X Column...

#### Worksheet
- Conditional Formatting: Conditional Highlight... · Conditional Heatmap... · Conditional Pareto... · Conditional Outliers... · Conditional Format Manager...
- Pivot Table...
- Convert to Matrix: Direct · XYZ Gridding · Extract
- Worksheet Script...
- Most Recently Used Items

#### Analysis
- Mathematics: XYZ Surface Area... · Special Filter... · Inverse...
- Fitting: Nonlinear Implicit Curve Fit... · Simulate Surface... · Compare Datasets...
- Signal Processing > FFT: 2D FFT... · 2D IFFT...
- Signal Processing: Deconvolution... · STFT...
- Signal Processing > Wavelet: Decompose... · Reconstruction... · Denoise... · Smooth... · Continuous Wavelet...
- Peaks and Baseline: Multiple Peak Fit · Batch Peak Analysis Using Theme...
- Most Recently Used Items

#### Statistics
- Descriptive Statistics: Partial Correlation Coefficient...
- ANOVA: Three-Way ANOVA... · Repeated Measures ANOVA...
- Most Recently Used Items

#### Image
- Adjustments: Brightness... · Contrast... · Gamma... · Levels...
- Arithmetic Transform: Math... · Subtract...
- Conversion: To Binary... · Color to Gray... · RGB Split...
- Geometric Transform: Crop... · Resize... · Rotate... · Flip...
- Spatial Filters: Smooth... · Sharpen... · Edge Detection...

#### Graph
- Add Plot to Layer: Line... · Scatter... · Line + Symbol... · Column... · Area...
- Layer Management... · Add Function Graph... · Add Straight Line... · Merge Graph Windows... · New Table · New Color Scale · Fit Layer to Graph · Fit Page to Layers... · Convert to Standard Font Size
- New Layer(Axes): Horizontal Axis Layer... · Vertical Axis Layer...
- Legend: Add Legend Title
- Offset Grouped Data in Layer: None · Cumulative · Constant · Auto · Individual

#### Data
- Set Display Range · Reset to Full Range · Mark Data Range · Clear Data Markers
- Analysis Markers: Hide · Show Partial Ranges · Show All · Tiny · Small · Medium · Large
- Lock Position: Layer Icon · First Point · Last Point

#### Gadgets
- Statistics... · Quick Fit... · Quick Sigmoidal Fit... · Quick Peaks... · Differentiate... · Interpolate... · Intersect... · Rise Time... · Cluster... · FFT... · 2D Integrate... · Vertical Cursor... · Go To...

#### Tools
- Options...
- System Variables... · 3D OpenGL Settings... · Apps on File Exchange...
- Protection: Protect Project... · Audit Log
- Fitting Function Builder... · Fitting Function Organizer... · Theme Organizer... · Import Filters Manager... · Package Manager... · Custom Menu Organizer... · Virtual Matrix Manager... · X-Function Builder... · X-Function Script Samples...
- R Console... · Rserve Console... · Python Console...

#### Format
- Menu · Programming Control... · Object Properties...
- Layer Properties... · Snap Layers to Grid · Snap Objects to Grid · Fit Page To Layers · Fit Page To Printer [Graph]
- Axes: Z Axis... [Graph]
- Axis Tick Labels: X Axis Tick Labels... · Y Axis Tick Labels... [Graph]
- Axis Titles: X Axis Title... · Y Axis Title... [Graph]
- Worksheet... · Cell... · Snap to Grid · Merge cells [Workbook]

#### Window
- Arrange Icons
- Duplicate (Batch Plotting): With New Sheets/Books · With New Columns [Graph]
- Split [Workbook]
- Command Window · Script Window
- Folders: Folders List

#### Help
- Use Online Help · Check for Updates...
- Origin: Help · X-Functions · Programming
- Programming: LabTalk · Origin C · Python · Automation Server
- Open Sample Projects: Sample Projects
- Online Resources: Web Links

#### Toolbars
- Standard: New Graph · New Matrix · New Layout · New Notes · Duplicate
- Format: Font name box · Font size box · Bold · Italic · Underline · Superscript · Subscript · Greek · Increase Font Size · Decrease Font Size · Font Color · Fill Color · Line Color
- Graph: Zoom In · Zoom Out · Whole Page · Add Layer (Bottom-X Left-Y) · Add Layer (Top-X Left-Y) · Add Layer (Bottom-X Right-Y) · Add Layer (Top-X Right-Y) · Add Inset Graph
- 2D Graphs: Bar · Stacked Area · High-Low-Close · Box Chart · Histogram · Bubble
- Tools: Zoom In · Zoom Out · Pan
- Object Edit: Align Left · Align Right · Align Top · Align Bottom · Align Horizontal Center · Align Vertical Center · Make Same Width · Make Same Height · Bring to Front · Send to Back · Group · Ungroup

#### Context menus
- Worksheet column header: Copy Columns to... · Remove Links · Frequency Count... · Swap Columns... · Slide Show of Dependent Graphs · Add or Update Sparklines... · Format Cells... · Set Sampling Interval... · Show X Column...
- Worksheet column header > Fill Column with: Row Numbers · Uniform Random Numbers · Normal Random Numbers · A set of Numbers... · A set of Date/Time values... · Arbitrary set of Text&Numeric values...
- Worksheet data cell: Paste Transpose · Paste Link · Paste Link Transpose · Remove Links · Set as Begin · Set as End · Insert Graph... · Insert Images from Files... · Insert Sparklines... · Insert Variables... · Format Cells...
- Worksheet data cell > Insert Arrow: Arrow · Double Arrow
- Worksheet data cell > Set Data Style: Default
- Worksheet row number: Insert Graph... · Format Cells...
- Worksheet sheet tab: Add Graph as Sheet... · Add Notes as Sheet · Duplicate Without Data · Navigate...
- Worksheet sheet tab > Sheet Name to Show: Short Name · Long Name · Both
- Worksheet corner: Column Header · Row Header · Column Grid · Row Grid · Page Break Preview Lines
- Worksheet corner > Row Labels for Column Label Rows: Long Name · Units · Comments · F(x)= · Sparklines · Parameters
- Graph plot: Change X · Change Y · Ungroup
- Graph axis: Center · Save Format as Theme...
- Graph axis > Copy Format: All · All Style Formats · Axis
- Graph axis title: Cut · Copy · Bring to the front · Push to the back
- Graph text label: Cut · Copy · Bring to the front · Push to the back
- Graph drawn shape: Cut · Copy
- Graph legend: Add Color Scale · Add Bubble Scale · Cut · Copy · Bring to the front · Push to the back · Bring to front of data · Push to back of data · Programming Control... · Save Format as Theme...
- Graph legend > Legend: Add Legend Title
- Graph legend > Copy Format: All · All Style Formats · Text
- Graph page: Layer Management... · Add Shortcut to Favorites · View Full Screen
- Graph page > New Layer(Axes): Horizontal Axis Layer... · Vertical Axis Layer...
- Graph page > Copy Format: All · All Style Formats
- Graph page > Duplicate (Batch Plotting): Duplicate
- Graph layer icon: Show Active Layer Only · Set Active Layer By Layer Icon Only · Delete Layer · Double click to open Plot Setup · Layer Properties... · Layer Management...
- Explorer folder: Properties...
- Explorer window: Duplicate
- Label cell: none found
- Integrate gadget: none found

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
