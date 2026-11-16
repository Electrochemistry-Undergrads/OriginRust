# Handoff

Last updated: 2026-11-15. This file is short on purpose. **To start work, read this file, then
[`docs/tasks/PLAN.md`](docs/tasks/PLAN.md), then one brief `docs/tasks/Txx-*.md`**
([`docs/tasks/README.md`](docs/tasks/README.md) has the working method and the index of briefs). Do
not read `docs/audit/origin-differences-2026-11-12.md` or whole specs up front; the brief names the
section to open.

**Keep it short.** This file holds the rules, the priorities, the claims and the open problems. What
was merged, with its test counts and limits, goes in the brief's Status line and the commit message,
not here. The merge notes that used to be here are in
[`docs/handoff/archive-2026-11.md`](docs/handoff/archive-2026-11.md) (reference only).

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
- **Continuous run is the default (owner, 2026-11-16):** an agent that finishes a package does not
  stop. It closes out, then immediately selects, claims and builds the next open package, and keeps
  going until every package of `docs/tasks/PLAN.md` is ticked or blocked. The loop, the definition of
  "blocked", the list of things that are never a reason to stop and the one place it may end (the
  STOP CHECK) are in PLAN.md "Continuous run". Only an explicit "do this one task only" from the
  owner overrides it. This generalises the Codex rule above to every agent.
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

## Current priorities (owner, 2026-11-15)

The owner listed everything that is greyed "Not available in Sheaf yet" and set the order:

1. **Basic formatting** (bold, italic, underline, superscript, subscript, Greek, font, size, colour):
   [T58](docs/tasks/T58-graph-text-formatting.md) graph text and the Format toolbar, then
   [T59](docs/tasks/T59-worksheet-cell-formatting.md) worksheet cells.
2. **Stacked and multi-layer graphs**:
   [T60](docs/tasks/T60-merge-graphs-layer-management.md) Merge Graph Windows and Layer Management,
   [T61](docs/tasks/T61-layers-and-multi-y.md) adding layers, insets and three or more Y axes,
   [T62](docs/tasks/T62-stack-offset-leftovers.md) the stack and offset items still grey.
3. **Items whose feature already exists**:
   [T63](docs/tasks/T63-greyed-items-with-code-2.md), a parallel lane any session can take in groups.
4. Then waves 4 to 9 of [`docs/tasks/PLAN.md`](docs/tasks/PLAN.md): everyday graph editing, worksheet
   work, analysis breadth, templates and settings, new window types; 3D graphs and scripting last.

Do not start new work from wave 4 or later while a brief of 1 or 2 is unclaimed and you can do it.
To start or resume an agent, paste the start prompt at the end of PLAN.md's "Continuous run": it
makes the agent work through this list without stopping.
Claimed work in the table below is finished by its owner and does not wait for this order.

## Not available in Sheaf yet (greyed menu items)

463 greyed items on 2026-11-15: 319 in the menu bar, 47 on the toolbars, 97 in context menus. The
full list, by menu, with the package that will enable each item, is
[`docs/tasks/greyed-items.md`](docs/tasks/greyed-items.md). When you enable an item, delete it there
in the same commit and run `python tools/greyed-count.py`.

## State of `main`

- Milestones 1 to 5 are done (milestone 2 still waits for the owner's click-through of the app).
  Work is in milestone 6 (breadth) and in closing differences from Origin (`docs/ROADMAP.md`).
- The owner's daily workflow works end to end: CH Instruments import (text and binary), CSV and
  Excel import, overlay plots, integration, fitting, figures, `.opj` save.
- Last recorded full check: T55 merge, 2026-11-15, macOS: formatting, strict all-target Clippy and
  3,319 workspace tests pass (five ignored). Parity table (`python tools/parity-count.py`):
  49 rows done, 161 partial, 40 missing of 250; P0 32 done, 70 partial, 1 missing (cell formats, T59).
- What exists, in one line per area (limits are in each brief's Status and the README index):
  - **Files:** `.sheaf`; `.opj` read and default save with a `.opj.sheaf` companion (ADR 0008);
    `.opju` workbooks read only; ASCII/CSV with the Import Wizard and import themes; Excel import and
    export; window files; autosave and recovery; recent files.
  - **Worksheets:** formulas (Set Column Values, Set Multiple Columns), sort, filter, masks, reshape
    tools, Find/Replace, Go To, frozen panes, conditional highlight, cell notes.
  - **Graphs:** line, scatter, column, bar and area families with stacked and floating variants,
    histogram, box, violin, ridgeline, pie, polar, radar, ternary, bubble, hexbin, parallel, treemap,
    sunburst, lollipop; double-Y, stacked lines, waterfall, multi-panel and Stack; axis scales and
    breaks; Plot Details; drawn shapes; text labels with Origin's escapes; trendlines; export to
    raster, SVG, PDF, EPS, EMF and multi-page PDF; printing.
  - **Analysis, fitting, statistics:** the tools of briefs T03, T04, T14, T15, T22, T23, T26, T29,
    T40, T47 and T55 (integration, peaks, smoothing, signal processing, linear and nonlinear fits,
    surface fits on XYZ columns, tests, ANOVA, PCA, k-means, ROC, Kaplan-Meier).
  - **Automation:** the `sheaf` CLI and the MCP server `sheaf-mcp`, headless or attached to the
    open app (ADR 0011, 0013).
- Much of what was built on macOS on 2026-11-15 is not yet measured against Origin; each brief's
  Status says "unmeasured" where that holds. Measuring is the Windows session's work between
  packages.

## In flight (check for recent edits before claiming)

| Branch | Owner session | What | State |
|---|---|---|---|
| `codex/formula-worksheet-substitution` | Codex Windows, formula-worksheet-substitution | T08 static worksheet-cell %([Book]Sheet,column,row[,format]) substitution; expr lowering/evaluation, isolated core/IO formula regressions | claimed 2026-11-15; no graph/legend/session/generated-expression substitution, global Options/model fields, UI, MCP, imports, statistics or other owner scopes |
| `t19-fntrim` | Claude Windows, session fe4b37eb | T19 file-name trimming on ASCII import (Origin's FNameToSht/Bk From/To rules) and the book long name keeping the file extension (owner's decision 2026-11-15); crates/io ascii naming, app import window, cli flags, import-ascii.md, title tests | claimed 2026-11-15 |
| `codex/mcp-disconnect-flake` | Codex Windows | T25 reproduced Windows proxy-exit disconnect race: narrow proxy/transport/host teardown repair and focused lifecycle regressions | claimed 2026-11-15; proxy returned0 yet both TCP endpoints remained ESTABLISHED >10s; no discovery/ACL/auth algorithm/schema/history/UI/model or other owner scopes |
| `shape-resize` | Codex macOS, continuous-mac | T24 selected drawn-shape endpoint/corner resize handles; app graph_view/actions/shape edits, interaction safety and focused tests | claimed 2026-11-15; worktree `.claude/worktrees/shape-resize`; no clipboard, slope-tool behavior, file-format/model changes or other sessions’ scope |
| `codex/t02-browser-rows` | Codex Windows, t02-browser-rows | T02 Column Browser lower row-range editor: native defaults/validation/Cancel/reference transfer and focused UI tests/docs | claimed 2026-11-15; only go_to/browser and own go_to_browser tests/docs; no Exclude/preferences, original-sheet navigation changes without published refinement, app/MRU/grid/core/history/MCP/plots/stats/fit/formula/labels or other owner work |
| `stats-origin` | Claude Windows, "t14-origin-checks" | T14 item 6: Origin measurements of cross tabulation and Dunnett (statistics.md 15.3), distribution fit, probability and Q-Q plots (16.4); calc crosstab/dunnett/distfit, core analysis/dist.rs and the crosstab/ANOVA runners, their option dialogs, Origin fixtures | claimed 2026-11-14 night; continues 318653a1's scratch/stats-origin probes; no vartest, stats-multi, print, recovery, MCP attach, Go To, symbol gaps or other sessions' scope |
| `slope-annotation` | Claude macOS, "mac-multipdf" | T52 slope annotation (Origin 2026b): layer SlopeAnnotation linked to two data rows, segment + rise/run legs + slope label (per decade on log axes), toolbar Slope tool with point picking, delete/undo, CLI/MCP | claimed 2026-11-15; core graph.rs layer field, new plot module, app toolbar/graph_view tool mode, small dialog; no drawn-shape ordering, Data Reader changes or other sessions' scopes |
| `t48-origin` | Claude Windows, "continue all tasks" | T48 leftover: Origin 2016 measurements of the graph Data menu (Mask/Remove Bad/Move Data Points, graphing.md 20.4) and Sheaf following them; core graph_data.rs, app data_tools/reader/graph_view Data modes only | claimed 2026-11-15; no shape-resize, slope annotation, browser, stats, MCP or other sessions' scope |
| `deconv-stft` | Claude macOS, "violin" | T57 Deconvolution and STFT: calc signal/new stft module, core analysis tools, Signal Processing menu and dialogs, CLI/MCP analyze, analysis-tools.md 18 | claimed 2026-11-15; no Convolution/FFT/IIR/FIR result changes, no matrices, plot or other sessions' scopes |
| `worktree-agent-a79f85b1afb5f0034` | `originrust-3a` | Matrices, heat maps, contour plots, `sheaf matrix` CLI (`7994c3b`..`8c21175`, 48 files, 10k lines) | 4 commits, local only, based on `148e9b1`, not merged; session offline since 2026-11-12 19:00 |

`merge-05b` (worktree `.claude/worktrees/merge-05b`) is session 05b367c2's unfinished merge of
branches that are all on `main`; it holds nothing `main` lacks and can be removed (do not push it).
Free worktrees on the Windows machine, with their build folders: `axis-range2`, `stats-breadth`,
`axis-scales`, `stats-more2`. Keep the matrix worktree.

## Known issues and open checks

- **Table UI:** dragging the mouse over cells selected only the last cell in two automated tries on
  a release build; the UI harness selects the block with every timing tried, so it is probably not
  an app defect, but the owner should drag once by hand. Table UI coverage is not finished: row and
  column operations, sheet controls and switching, remaining shortcut cases.
- **Flaky test:** the `sheaf-calc` test binary `origin_stats2_tests` crashed twice (exit
  0xffffffff, no failed assertion) under a full parallel `cargo test --workspace`; reruns and 240
  stress runs passed; cause unknown.
- **Slow paths:** Dunnett's comparison runs on the UI thread (about 8 s at 30 groups); hatch fill
  costs lines times edges on a very long fill-under-curve run.
- **`.opju`:** about 200 columns of the 19 gallery samples decode wrongly part-way (cut, with a
  warning; opju-format.md 5.4, 9); designations, labels, graphs and folders are not read; no writer.
- **macOS:** opening and cancelling the native print panel and paper output are unverified; files
  double-clicked in Finder do not open in Sheaf (the bundle declares no document types).
  **Windows:** no Start Menu shortcut.
- **CI:** pushes to `main` cancel older CI runs; use pull-request CI for verification while other
  sessions push often.
- **Disk:** the Windows machine's disk has filled twice (about 96 % on 2026-11-13). Reuse a worktree
  and its build folder; check `df -h /c` before parallel builds.

## Origin measurements still open (Windows session)

Statistics checked against scipy only: cross tabulation and Dunnett (statistics.md 15.3),
distribution fit, probability and Q-Q plots (16.4), claimed as `stats-origin` below; the T13 look
settings of graphing 18.6 (hatch list, gradient, bar width, label options); Average Multiple Curves
spline ends and a turning X; the interactive Subtract Straight Line and Curve Translate on the
graph; Shift snapping and the ellipse tool; ternary title placement; and every feature whose brief
Status says "unmeasured".

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

1. **Try the app** (`cargo run --release -p sheaf-app`): import a real CH Instruments file, plot,
   right-click and double-click the graph, try Analysis > Integrate and Fit; report what is wrong.
   (An agent ran the CLI and the app's screenshots on the six real files on 2026-11-13; a person's
   click-through is still open.)
2. Example figures you make most often, to match their style. For the new priorities: one figure
   with the text formatting you want (T58) and one stacked figure (T60) would settle the defaults.
3. How you choose integration limits in Origin (rows, potential, or dragging on the graph).
4. Charge/discharge curves: against time or capacity, and per gram (needs the mass)?
5. May the remote branch `chi-text-real` be deleted? It holds a few numbers from your
   charge-discharge file in a spec paragraph (the copy on `main` does not).
6. The matrix, heat-map and contour work of session `originrust-3a` is unpushed and that session has
   been offline since 2026-11-12. May another session take it over, or start again (PLAN wave 8)?

## Local-only resources

- `re-workspace/scratch/`: scratch scripts, raw Origin outputs (`audit-2/`, `audit-3/`, `audit-3b/`
  hold the measurements cited in the audit), synthetic files (`gen_chi.js` writes CHI test files),
  screenshots; `peer-notes.md` is the note file between sessions.
- The owner's `.opj` and `.bin` files are in the repo root (git-ignored, read-only inputs).
- No Ghidra project and no saved REA evidence bundle exist.
- Origin is driven through COM by `tools/origin-ref/` (system-wide lock: one Origin at a time). A
  hidden Origin started by an agent may still be running.
