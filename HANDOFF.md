# Handoff

Last updated: 2026-11-14. This file is short on purpose. **To start work, read this
file, then `docs/tasks/README.md`, then one brief `docs/tasks/Txx-*.md`.** Do not read
`docs/audit/origin-differences-2026-11-12.md` or whole specs up front; the brief names the section to
open. History of what was merged when is in `git log` (not repeated here).

## Standing rules from the owner

- **When unsure, copy Origin:** measure it through `tools/origin-ref/` (or read its docs for
  behaviour) and copy it; record the measurement in the spec. Don't ask the owner what Origin can
  answer. Clean-room: copy behaviour and layout, never code, icons, help text or files.
- **Subagents:** Sonnet for building, research, reviews; Haiku for running tests and checking outputs.
- **No co-author line:** never add `Co-Authored-By: Claude ...` or "Generated with Claude Code" to
  commits or PRs; say so in every subagent prompt. (History was rewritten on 2026-11-12 to remove
  them; branches older than that must be rebased onto `main`, matching commits by tree.)
- **Repository:** https://github.com/Electrochemistry-Undergrads/OriginRust (private). Nothing on the
  owner's personal account.
- **Two or more sessions** often share this repo. Claim work in `re-workspace/scratch/peer-notes.md`
  (local only, read its last 40 lines on start); merge to `main` one branch at a time, after
  `git fetch`. See LESSONS "Two sessions on one repository need a written split".

## State of `main`

- Milestones 1-5 done (M2 waits on the owner's click-through; M5 lacks plot groups/legend frames).
  Owner requests of 2026-11-11 done: parity audit, Origin-style workspace (ADR 0007, most steps),
  `.opj` writer for workbooks and graphs, `.opj` as default save with a `.opj.sheaf` companion for
  what the writer cannot store (ADR 0008). Now in milestone 6 (breadth) plus closing Origin
  differences.
- The owner's daily workflow (CH Instruments import, CSV, overlay plots, integration, fitting,
  figures) works end to end.
- T13 follow-ups (Windows, 2026-11-14): Data on Top of Axes (`0b9bd4e`), Skip Points
  (`ef9ed77`), data-label White Out and Individual X/Y multipliers are merged. Point decorations
  share the measured Origin 2016 frequency; label backgrounds follow rotation; multipliers
  transform the drawing while errors, labels and calculations retain source units. Reader and
  gadget positions follow the transformed curves. Formatting, strict all-target Clippy and
  1,993 workspace tests passed (two ignored), including integrated Windows clipboard work and
  ten new multiplier regressions. See the T13 Status.
- Tests: 1955 passed (one ignored) on `main` on macOS on 2026-11-14 night after the T25 port (`analyze` over MCP reaches every CLI tool, prompts, `.opj` backup link fix); formatting and strict all-target Clippy passed. Before that: 1857 passed (two ignored) on `main` on Windows on 2026-11-14 evening after Envelope, Coherence, Decimation and Trace Interpolation (Signal Processing / Mathematics, analysis-tools 13.2-13.4 and 16); formatting and strict all-target Clippy (toolchain 1.99, two `needless_range_loop` fixes) passed; parity 48 done, 136 partial, 54 missing of 238 rows, P0 32 / 69 / 0. Before that: 1815 passed (one ignored) on `main` on macOS after the T13 look settings, cross tabulation, Dunnett, distribution fit, probability and Q-Q plots and the app icon (2026-11-14); formatting and strict all-target Clippy passed. Before that: 1718 on `main` with filtered plots, the FFT phase-boundary repair and latest FFT Filters. Previous Windows run: 1706 with FFT/IFFT and FFT Filters. Parity: 46 done, 135 partial, 54 missing of 235 rows; P0 32 / 69 / 0 (`python tools/parity-count.py`).
- FFT/IFFT CI portability: phase unwrapping treats half-turn roundoff consistently across FFT backends; the original Origin fixtures remain unchanged (analysis-tools.md 15.1). The Linux-only failure is covered by an adjacent-representable-value regression.
- CI repair (`4b048d1`): the repeated engineering-format failure (`1.0e-24` instead of `1e-24` with one significant digit) is fixed by parsing the decimal scaling power. Strict formatting expectations remain, with a regression across positive and negative decimal powers. The full macOS workspace at that commit passed 1576 tests (one ignored), formatting, and all-target Clippy. Pushes to `main` cancel older CI runs; use pull-request CI for verification while other sessions are pushing frequently.

## In flight (do not start these; check `git branch -a` and peer-notes for newer state)

| Branch | Owner session | What | State |
|---|---|---|---|
| `codex/formula-cross-sheet` | Codex Windows, formula-cross-sheet | T08 explicit cross-sheet/book formula references, formula dependencies and persistence tests; expr + core formula paths only | claimed 2026-11-14; no percent substitution, app dialogs, graph/plot, statistics, MCP or other sessions' scope |
| `codex/t13-label-frame-margin` | Codex Windows, t13-label-frame-margin | T13 Label Frame (None/Box/Shadow, border/fill colors) and Margin only; core DataLabels, plot label backgrounds, graph_details and focused tests | claimed 2026-11-14; no T01 stack panels, app state/menu, leader lines or other sessions' work |
| `vartest` | Claude Windows, "continue all tasks" (owner's decision) | T26 variance and proportion tests: calc vartest.rs, core Tool::VarTest/PropTest, Hypothesis Testing dialogs, ops analyze vartest/proptest | claimed 2026-11-14, continues 7ec9e48/5a08edf |
| `stats-multi` | Claude Windows, "continue all tasks" | T29 PCA, ROC, Kaplan-Meier, k-means: calc pca/roc/survival/kmeans.rs, core statistics tools, Statistics menu items, ops analyze | claimed 2026-11-14 |
| `autosave` | Claude macOS, "mac-autosave" | T28 autosave and crash recovery: sheaf-io recovery save/list/discard, app timer on a worker thread, start-up Recover/Discard/Later dialog, Tools autosave settings, `docs/spec/autosave.md` | claimed 2026-11-14 night; no `.opj` writer, MCP, statistics or other sessions' scope |
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

**T27 Print, Windows half merged (2026-11-14, session "continue all tasks"):** File > Print... (Ctrl+P) prints the active graph page through the system dialog as vector EMF placed by `PageSetup::place` (Letter, Automatic orientation, 1 inch margins by default, as Origin's Page Setup showed); Print Preview draws the same placement; Page Setup is saved in `.sheaf` and the `.opj.sheaf` companion (graphing.md 19). A headless print to Microsoft Print to PDF matched the placement exactly. Left (T27 Status): workbook printing, the macOS print panel (the `lp` fallback is untested), `sheaf print`, Origin's own Print and Print Preview windows (unmeasured).

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
integer limits. CLI-parity, schema and protocol regressions cover the bridge. Phase 2 attach and
`output: new_sheet` remain open; no Origin measurements are needed for this interface.

## Known issues and table UI verification

- T01 sorting: Sort Range and range Custom respect selected cell/row bounds. Sort Columns
  Ascending/Descending move selected rows together; Each Column modes sort independently.
  Sort Columns and column-header Custom now use the existing nested-sort dialog. Masks,
  outside rows/columns and Undo/Redo are covered; see workbook-model.md 13.1.

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
  selected None/PackBits/LZW compression and DPI retained; mono and 16-color modes remain open.

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
analysis: T25 (MCP server, owner's request; phase 1 needs no Origin), T13 leftovers (other fill modes, Offset multiplier, Data on Top of Axes, several groups per layer, Binned increments, X:Y ratio and speed mode, `.opj` writing of the 18.6 settings), T14 item 6 (2D binning, discrete frequency, cross tabulation, Dunnett, distribution fit, probability and Q-Q plot are done; left: the Windows Origin measurements of cross tabulation and Dunnett in statistics.md 15.3 and of the distribution fit and plots in 16.4, since all are scipy-checked, not Origin), T16 item 6, T18 backlog items that need no Origin
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
