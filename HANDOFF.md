# Handoff

Last updated: 2026-11-15. Start here, then read [the prioritized plan](docs/tasks/PLAN.md),
[the task workflow/index](docs/tasks/README.md), and **one** task brief. Open only the spec sections
that brief names. This update finishes the paused cloud session's planning work; it does not
implement T58–T63 or claim a fresh comparison with Origin.

## Current priorities

1. **Basic formatting:** graph text and its Format toolbar (**T58**), then worksheet cells
   (**T59**). Bold, italic, underline, scripts, Greek, font, size and colour must be usable without
   manually writing escape codes.
2. **Stacked figures and layers:** merge existing graphs and arrange their layers (**T60**), add
   layers/insets and three/four Y axes (**T61**), then remaining stack/offset commands (**T62**).
3. **Existing-feature wire-ups:** **T63**, claimed by group, can proceed beside those priorities
   when its files do not overlap. Then follow waves 4–9 of the plan.

Do not rebuild working stacked lines/waterfall, Double-Y, column-based 2/4/9-panel or Stack plots,
or stacked bars/areas. The gap is editing/combining finished graphs and the remaining controls.
T58 owns the shared Format toolbar dispatch; T59 adds the worksheet target after that lands.
T60 owns arranging/merging layers; T61 owns adding/deleting them. Coordinate edits to shared files.

## Where the detail lives

- [PLAN](docs/tasks/PLAN.md): wave order, dependencies, ownership boundaries and agent loop.
- [Task index](docs/tasks/README.md): status and leftovers for every brief, including T58–T63.
- [Greyed inventory](docs/tasks/greyed-items.md): **459 listed item locations** at the planning
  baseline (318 menu bar, 47 toolbar, 94 context menu). Run `python3 tools/greyed-count.py --check`
  after changes. This is a hand-maintained inventory, not a generated audit of the Rust source.
- [Known issues and UI walkthrough](docs/tasks/known-issues.md): preserved findings and acceptance
  cases. Specs remain the source of measured behaviour and numeric fixtures.
- [Roadmap](docs/ROADMAP.md) and [parity](docs/spec/feature-parity.md): broad progress, not task claims.
- [Archived handoff](docs/archive/HANDOFF-2026-11-15.md): the complete previous file at `dec7374`,
  unchanged, including merge history, detailed findings, owner questions and local resource notes.

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

## Latest integration

T02 Column Browser Exclude (`e606eef`) now temporarily hides selected upper-list columns,
selects the first removed list position (or the final remaining row), and preserves original
column ordinals and already-added ranges. Scope changes and reopening restore excluded columns;
gapped original columns cannot form one block. Ten valid isolated Origin cases and seven new UI
regressions cover this follow-up (46 focused Go To tests). Formatting, strict workspace all-target
Clippy, 3,394 workspace tests (eight existing ignored) and inventory validation pass on the combined
T48/T64 tree; incoming Windows MCP test fix `81765e8` is preserved. Saved browser preferences remain.

T48 graph Data menu measured in Origin 2016 (branch `t48-origin`, graphing 20.4/20.5): Mask (a click masks,
a drag masks every plot's Y cells, Tab active plot, Space freehand) and Remove Bad (pick, then Enter;
`@YPR` 1 keeps the row) now follow Origin, with its undo texts and Esc/right-click ending a mode.
Move Data Points stays unmeasured (synthetic input never engaged it). Checks: fmt, strict Clippy, full tests.

T08 static worksheet-cell substitution (`34c1ba3`) adds fixed qualified cell references,
measured 15-digit defaults, column-display `W` and preserved text. Expression/core/I/O tests
cover recalculation, cycles, Undo and native/companion reopen; expression-language section 3
records scalar measurements and the existing no-generated-code boundary. Active-sheet/book-only,
dataset/graph/session and generated-reference forms remain open. Formatting, strict workspace
all-target Clippy and 3,356 tests pass (eight existing ignored); inventory and eight Python
tooling checks also pass on the combined browser/slope/prioritization tree.

T25 proxy disconnect cleanup (`ef98bf7`) and T02 browser row ranges (`d9b4af7`) landed during
planning; both claims are released. T52 slope annotation also landed (`33c4d246`);
its implementation and unmeasured Origin defaults are recorded in graphing 17.6 and T52. Results and recorded checks are preserved in
[known issues/recent results](docs/tasks/known-issues.md).

## In flight (check for recent edits before claiming)

| Branch | Owner session | What | State |
|---|---|---|---|
| `codex/t63-duplicate-buttons` | Codex Windows, t63-duplicate-buttons | T63 groups C/F: record C as deferred to T14 (published plot_prob score/confidence/orientation needs exceed wiring); wire F Standard/Project Explorer Duplicate buttons to existing duplicate commands with focused UI target/state tests | claimed 2026-11-15; own `.claude/worktrees/codex-t01-error-bars-dialog` + `target/wt-t01-error-bars-dialog` reused sequentially; toolbar Standard/Explorer buttons ONLY and local command-target hunks, own tests, T63/HANDOFF and own inventory/parity rows; stats-origin retains probability/Q-Q engines/dialogs/measurements/fixtures; no T58 Format bar, T61 Graph bar, T59 cells, T60/T61 layers, T62 stack, step lines, MCP/import/export, pending root files or index/roadmap |
| `codex/text-decorations` | Codex Windows, text-decorations | T58 GRAPH TEXT FORMATTING (`T58-graph-text-formatting.md`), first rendering slice: inline underline/strikethrough, markup/layout/rendering and isolated export tests | claimed 2026-11-15; own `.claude/worktrees/codex-formula-parameter-labels` + `target/wt-formula-parameter-labels`; no toolbar/editor/model fields, significance-plot/brackets (separate T58 brief), shapes/data-tools/browser/export-dialog/import/stats/MCP/printing or other owners |
| `codex/t05-export-dialog-themes` | Codex Windows | T05 Dialog Theme row: Origin theme-list/save/reuse behavior, named export-settings themes and isolated persistence/UI regressions | claimed 2026-11-15; own `.claude/worktrees/codex-t13-skip-points`, `target/wt-t13-skip-points`; no Graph Theme, preview, renderers/formats, import behavior, recovery, priority docs or other scopes |
| `t19-fntrim` | Claude Windows, session fe4b37eb | T19 file-name trimming on ASCII import (Origin's FNameToSht/Bk From/To rules) and the book long name keeping the file extension (owner's decision 2026-11-15); crates/io ascii naming, app import window, cli flags, import-ascii.md, title tests | claimed 2026-11-15 |
| `shape-resize` | Codex macOS, continuous-mac | T24 selected drawn-shape endpoint/corner resize handles; app graph_view/actions/shape edits, interaction safety and focused tests | claimed 2026-11-15; worktree `.claude/worktrees/shape-resize`; no clipboard, slope-tool behavior, file-format/model changes or other sessions’ scope |
| `stats-origin` | Claude Windows, "t14-origin-checks" | T14 item 6: Origin measurements of cross tabulation and Dunnett (statistics.md 15.3), distribution fit, probability and Q-Q plots (16.4); calc crosstab/dunnett/distfit, core analysis/dist.rs and the crosstab/ANOVA runners, their option dialogs, Origin fixtures | claimed 2026-11-14 night; continues 318653a1's scratch/stats-origin probes; no vartest, stats-multi, print, recovery, MCP attach, Go To, symbol gaps or other sessions' scope |
| `deconv-stft` | Claude macOS, "violin" | T57 Deconvolution and STFT: calc signal/new stft module, core analysis tools, Signal Processing menu and dialogs, CLI/MCP analyze, analysis-tools.md 18 | claimed 2026-11-15; no Convolution/FFT/IIR/FIR result changes, no matrices, plot or other sessions' scopes |
| `merge-graphs` | Claude Windows, session fe4b37eb | T60 Merge Graph Windows and Layer Management (all four steps: core merge.rs + panels.rs arrange helpers, merge/layer-management dialogs, Fit Layer to Graph / Fit Page to Layers, CLI `merge-graphs`, Origin merge_graph/layarrange measurements, graphing 15) | claimed 2026-11-15; own `.claude/worktrees/merge-graphs` + `target/wt-merge-graphs`; no adding/deleting layers or insets (T61), no Offset/stack items (T62), no text formatting (T58) or other sessions' scopes |
| `layers-multi-y` | Claude Windows, "continue all tasks" | T61 layers, insets and 3/4 Y axes (all slices): core graph.rs layer add/delete + axis offset (local edits), template.rs 3Ys/4Ys, plot offset axes, app Add Layer buttons/layer icon/Layer Properties/Multiple Y Axes, ops template names, opj/companion | claimed 2026-11-15; no T60 merge/arrange (panels.rs, merge.rs), T58/T59 formatting, T62 stack/offset |
| `stack-leftovers` | Claude macOS, "mac-multipdf" | T62 all three slices: Offset Grouped Data in Layer menu (None/Cumulative/Constant/Auto/Individual, new Cumulative mode), Stacked Histograms, waterfall Y/Z colour mapping with colour scale | claimed 2026-11-15; core graph.rs Stack + core/stack.rs + autorange, plot/stack.rs, menus offset submenu, template stacked histograms, waterfall colour mapping; no merge/layer management (T60/T61), text formatting (T58) or other sessions' scopes |
| `worktree-agent-a79f85b1afb5f0034` | `originrust-3a` | Matrices, heat maps, contour plots, `sheaf matrix` CLI (`7994c3b`..`8c21175`, 48 files, 10k lines) | 4 commits, local only, based on `148e9b1`, not merged; session offline since 2026-11-12 19:00 |

## Verification and completion

For implementation work, run from the task worktree:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 tools/greyed-count.py --check
```

Use a distinct `CARGO_TARGET_DIR=target/<task>` and `CARGO_PROFILE_DEV_DEBUG=0`. Check free space
first (`df -h .` on macOS/Linux; `df -h /c` on Windows Git Bash). App changes also need a focused UI
regression and a screenshot/export check; numeric changes need reference fixtures. The required
Rust checks also apply before merging this planning update under the current task workflow.

Before merging, fetch again and reconcile new `main` commits and claims. Delete enabled items
from the inventory, run `python3 tools/greyed-count.py --write`, update brief/index status and
roadmap/parity rows, and release only your own claim in the same merge. Never mark a feature done
just because its brief was written.

## Local resources and unresolved owner input

`re-workspace/scratch/peer-notes.md` and raw Origin outputs are local and ignored. Read its last
40 lines if present. The owner's ignored `.opj`/`.bin` files are read-only inputs. Use
`tools/origin-ref/README.md` and the global Origin lock for reference measurements. No Ghidra
project or saved REA evidence bundle is recorded.

The owner still has a real-data click-through and example-figure preferences to provide; these
are optional inputs, not blockers for T58–T63. Do not delete `chi-text-real` without the owner's
answer to the existing question (it contains a few real charge/discharge numbers). Full wording
and the list of pending Origin measurements are in the archived handoff's last two sections.
