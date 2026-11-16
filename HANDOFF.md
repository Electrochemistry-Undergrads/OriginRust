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
- [Greyed inventory](docs/tasks/greyed-items.md): **400 listed item locations** at the planning
  baseline (285 menu bar, 28 toolbar, 87 context menu). Run `python3 tools/greyed-count.py --check`
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

T66 graph view modes (B1 slice 2, branch `graph-view-modes`, graphing 22, Origin 2016 measured): View > Print View
(Ctrl+2), Page View (Ctrl+1, default), Window View, Draft View (Ctrl+0), saved per graph in `.sheaf`; View > Show >
Layer Icons; Full Screen (Ctrl+Shift+J, reminder, Esc ends, arrows/Home/End switch graphs). Nine Show items and Show Data
Info stay grey (unmeasured: Origin's collapsing menu). fmt, strict Clippy, full tests pass.

T58 indexed text colours: `\cN(text)` now uses the original 24-colour list, with measured
positive-index byte selection and black fallback. Nested spans restore their surrounding
colour; glyphs and underline/strikeout strokes share the run colour. Exact Origin 2016 EMF
fixtures, new SVG/stroke PNG goldens and text/export regressions are documented in graphing
7.2. Font switching, combined scripts and tagged RGB/special colour codes remain open; editor
and Format-toolbar work stay with their owners. Formatting, strict workspace all-target Clippy
and 3,654 tests pass (eight existing ignored); inventory 414 and eight Python tooling tests
also pass on the combined zoom integration tree.

T65 graph view zoom and pan (`graph-zoom`, B1 slice 1): Origin 2016's View Zoom In mode (x2,
re-centred on the click), Zoom Out by halves to 25 %, Whole Page and the Tools-bar Zoom-Panning tool,
measured and copied (gui-look 4.x); the Tools-bar magnifiers are Origin's axis Scale In/Out (still B1).

T58 graph text, first renderer slice: inline underline and strikethrough use shared scene
paths and resolved-font metrics, including nested scripts and rotation. Nine focused tests,
SVG and font-independent stroke PNG goldens cover glyph anchors, title/legend/label/tick
placement and EPS/EMF/PDF/raster output. Origin scalar/PNG observations and layout limits are
recorded in graphing 7.1. Per-run colour/font switches, combined scripts and editor/toolbar
formatting remain open; the 15 UI inventory locations remain disabled. The separately claimed
T64 significance plot is unaffected. Formatting, strict workspace all-target Clippy and
3,624 tests pass (eight existing ignored), with the 421-item inventory and eight Python
tooling tests passing on the current T59/T60/T61/T62/T63 integration tree.

T63 J (`t63-add-plot`): Graph > Add Plot to Layer > Line/Scatter/Line + Symbol/Column/Area open
Layer Contents with that plot type chosen, as measured in Origin 2016; Area joined the type list.

T61 layers, insets and three/four Y axes (branch `layers-multi-y`, graphing 21, measured in Origin 2016): Graph >
New Layer(Axes) with nine kinds and six Graph-toolbar buttons, Add Inset (with data), Delete Layer (children
unlinked, keep their place and ranges), Layer Properties, displaced Y axes in margins and hit-testing, Move to
Layer (Sheaf's own), 3Ys Y-YY / Y-Y-Y and 4Ys Y-YYY / YY-YY templates and Multiple Y Axes (`plot -t 3ys-y-yy` etc.).
Left: Open Dialog..., three layer-icon items, Origin's legends in the templates. fmt, strict Clippy, full tests pass.

T60 Merge Graph Windows and Layer Management (`merge-graphs`): Origin 2016's merge_graph measured in
full (page, cells, automatic grid, text factor, links, Show Axes Frame, aspect, keep/rearrange) and
copied in `sheaf_core::merge`, the Graph menu window and `sheaf merge-graphs`; Layer Management
(Arrange, Size/Position, Link; `layer_manage.rs`) from the Graph menu, page and layer-icon menus; Fit
Page to Layers as `pfit2l` (frames and objects plus a border). Fit Layer to Graph did nothing
measurable in Origin and is tagged `N`. Graphing 15.7c and 15.7d; leftovers in the T60 brief.

T63 F Duplicate entry points (`a7fd735`) now reuse the existing window-copy operation:
Standard follows the active workbook/graph; Explorer's context item follows the clicked window,
including a hidden one. Four new UI regressions cover target identity, empty books, cancellation
and Undo/Redo; the existing duplicate tests and desktop captures also pass. Formatting, strict
workspace all-target Clippy, 3,450 tests (eight existing ignored), frozen source/tree checks and
the 457-item inventory validation pass on the combined T59/T19/T05 tree. T63 C's probability/Q-Q
entries remain disabled and are deferred to B8/T14 with published score/confidence/axis prerequisites
in the T63 brief; the existing statistics tools are unchanged.

T05 export Dialog Themes (`21480e7`) now save/reload named settings, custom defaults/reset and
last successful exports, with modified-name marking and confirmed deletion. Seven regressions,
five isolated Origin sessions and an inspected desktop screenshot cover this scope. Formatting,
strict workspace all-target Clippy and 3,443 tests pass (eight ignored); inventory 459 passes.
Graph Theme, graph-specific settings and comparison/script tools remain (graphing 9.8).

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
| `codex/text-run-rgb` | Codex Windows, text-run-rgb | T58 step 1 renderer colour remainder: numeric RGB-tagged `\cN(text)` values returned by Origin Color(r,g,b), measured controls and nested glyph/stroke/export fixtures | claimed 2026-11-15; own `.claude/worktrees/codex-formula-parameter-labels` + `target/wt-formula-parameter-labels`; markup/text + own tests only; no signed Auto/None or other tags, font-family/combined-script changes, editor/Format toolbar/dispatch, model/global palettes/settings, worksheet/core/IO/backends or other scopes |
| `plot-types-b4` | Claude Windows, "axis-breaks" | B4 slice 1 as new brief T68: Plot > Symbol > Vertical Drop Line and Plot > Column/Bar/Pie > Column + Label, measured in Origin 2016; core templates, plot drawing, Plot menu items, CLI template names, tests | claimed 2026-11-15; worktree .claude/worktrees/axis-range2, build target/wf-autosave; no step/spline/line-series items (T63 B), no other B4 types, text formatting or other sessions' scopes |
| `codex/t58-text-editor` | Codex Windows, t58-text-editor | T58 slice 2 ONLY: graph Text window formatting/selection controls and live preview using published renderer capabilities | claimed 2026-11-15; own `.claude/worktrees/codex-t01-error-bars-dialog` + `target/wt-t01-error-bars-dialog` reused sequentially after completed T63 F; dialogs/text_object.rs + private helper, isolated UI tests/registration, existing text startup screenshot hook if needed, T58/graphing7; no markup/fonts/layout/renderers/backends (text-decorations), shared Format bar/dispatch/actions (t58-format-toolbar), core model fields, formula/parameter labels, graph_view/shapes/layers/axis menus, other dialogs, pending root/index/roadmap; unsupported formatting stays disabled until published renderer support |
| `codex/t58-format-toolbar` | Codex Windows | T58 slice 3: shared Format toolbar dispatch and existing whole-object graph font controls; selected-target/undo/persistence tests and Origin measurement | claimed 2026-11-15; own `.claude/worktrees/codex-t13-skip-points` + `target/wt-t13-skip-points`; Format bar only, local actions/app dispatch + new helper/tests; no renderer/fonts/markup (text-decorations), editor slice 2, worksheet adapter/model, Graph/Standard/Explorer bars, layers/stack, graph_view or new model fields without refinement |
| `shape-resize` | Codex macOS, continuous-mac | T24 selected drawn-shape endpoint/corner resize handles; app graph_view/actions/shape edits, interaction safety and focused tests | claimed 2026-11-15; worktree `.claude/worktrees/shape-resize`; no clipboard, slope-tool behavior, file-format/model changes or other sessions’ scope |
| `stats-origin` | Claude Windows, "t14-origin-checks" | T14 item 6: Origin measurements of cross tabulation and Dunnett (statistics.md 15.3), distribution fit, probability and Q-Q plots (16.4); calc crosstab/dunnett/distfit, core analysis/dist.rs and the crosstab/ANOVA runners, their option dialogs, Origin fixtures | claimed 2026-11-14 night; continues 318653a1's scratch/stats-origin probes; no vartest, stats-multi, print, recovery, MCP attach, Go To, symbol gaps or other sessions' scope |
| `deconv-stft` | Claude macOS, "violin" | T57 Deconvolution and STFT: calc signal/new stft module, core analysis tools, Signal Processing menu and dialogs, CLI/MCP analyze, analysis-tools.md 18 | claimed 2026-11-15; no Convolution/FFT/IIR/FIR result changes, no matrices, plot or other sessions' scopes |
| `greyed-2-abdehik` | Claude macOS, "mac-multipdf" | T63 groups A (toolbar plot buttons), B (step/spline templates), D (Open Excel: reassign to B14), E (Graph as Image / Multi-Page PDF with no graph active), H (Row Numbers/Uniform/Normal fills), I (X/Y Axis Tick Labels), K (column-header Frequency Count) | claimed 2026-11-15; only these groups' dispatch/state/tests in menus.rs and toolbar.rs (not the Format bar, T58), greyed-items.md; no G (shape ordering), J (layer contents, T60/T61) or other sessions' scopes |
| `straight-line` | Claude Windows, session fe4b37eb | B3 slice 1 as new brief T67: Graph > Add Straight Line (Origin's addline: vertical/horizontal line at an axis value with colour, style and value label): core layer model (new module), plot drawing, the dialog, .sheaf/.opj (warn), CLI flag, Origin 2016 measurements | claimed 2026-11-15; reuses own `.claude/worktrees/merge-graphs` + `target/wt-merge-graphs`; not the other B3 items (function graph, table, colour scale, legend title, display range/markers, Change X/Y), no shape-resize/graph_view drag code beyond selecting/deleting the new object, no T58/T66 scopes |
| `b1-rest` | Claude Windows, "continue all tasks" | rest of B1 as new brief T69: View > Zoom All presentation and Tools-bar Scale In / Scale Out axis tools; app view_modes.rs, new scale tool module, toolbar Tools items | claimed 2026-11-15; no T65 transform changes, shape-resize, T58, T67, T68 |
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
