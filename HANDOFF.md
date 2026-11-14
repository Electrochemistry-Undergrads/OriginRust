# Handoff

Last rewritten: 2026-11-13. This file is short on purpose. **To start work, read this
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
- Tests: 1517 passed on `main` at `244cac5` (2026-11-13, Windows). Parity table: 43 done, 124 partial, 64 missing of 231 rows; P0 32 / 69 / 0 (`python tools/parity-count.py`).

## In flight (do not start these; check `git branch -a` and peer-notes for newer state)

| Branch | Owner session | What | State |
|---|---|---|---|
| `worktree-agent-a79f85b1afb5f0034` | `originrust-3a` | Matrices, heat maps, contour plots, `sheaf matrix` CLI (`7994c3b`..`8c21175`, 48 files, 10k lines) | 4 commits, local only, based on `148e9b1`, not merged; session offline since 2026-11-12 19:00 |

`merge-05b` (worktree `.claude/worktrees/merge-05b`) is session 05b367c2's unfinished merge of the same
branches that are now on `main`; it holds nothing `main` lacks and can be removed.

On 2026-11-13 no other OriginRust session was live, so the finished branches of the other sessions were
integrated into `main` by "Handoff.md token optimization": `text-numeric`, `opj-params-welch` (plus the
Welch interval, `f522bb2`), `opj-groups-frames`, `export-margin`, `sheet-menus` (its Multi-Panel and Y
Error duplicates of T01 dropped), `nlfit-origin`. `axis-range` and `graph-defaults` never got a commit:
they are briefs T20 and T21 now.

Small items merged on their own (2026-11-13, Windows): Edit > Find in Sheets (Ctrl+F) and Replace (Ctrl+H), View > Status Bar, Worksheet > Clear Worksheet, the Linear/Spline/B-Spline Interpolation menu items, TGA and PCX export, Origin's blue for floating columns. Merged from briefs: T22 (`734a1ff`: Average Multiple Curves, Intersect, Polygon Area; `analysis-curve-math.md`), T23 (`f66dbea`: Subtract Straight Line, Reduce to Evenly Spaced X, Curve Translate X/Y, Median Filter; analysis-tools.md 12), T24 (`542fef7`: drawn lines, arrows, rectangles, ellipses; graphing.md 17), each reviewed by independent agents and the confirmed findings fixed before merge; T12 radar, ternary and Origin's two pies (spec 14.19-14.21: radar as a polar layer with spokes, ternary as `Coords::Ternary`, 3D Color Pie and hatched 2D B&W Pie replacing "2D Color Pie"), T12 polar and the horizontal bars (`861166a`, `6033098`: polar theta(X) r(Y) and r(X) theta(Y) through a layer-level `Layer::coords`, spec 14.18; Stacked Bar, 100 % Stacked Bar, Floating Bar), T19 items 1 and 3 (`7ab562a`, `7c8bf12`: multi-character and fixed-width delimiters, Data Import Options, Origin's mode names, Apply; Column Properties Text & Numeric, Month, Day of Week, Engineering, Decimal:1000, 19 date and 14 time patterns), T15 items 4-6 (`e525575`: Simplex, Compare Models checked on 24 Origin pairs, Rank Models, multiple linear regression checked on 6 Origin cases, linear fit with X error by York's method, not compared with Origin), T16 item 2 (`94063b4`: Spline as Origin, natural cubic for increasing X up to 900 points, B-spline and Bezier), T08 remaining (`d8d1416`: `sum` is a running sum and `count` counts missing cells as in Origin, text values and string functions, local `now()`), T15 items 1-3 (`704a1d6`: direct weights, scale-errors switch, multi-dataset and global fits, quick fits), owner-reported `.opj` graph fix (`e908922`: text objects placed from their stored box as Origin turns them, group colours from the records, objects off the page hidden; opj-format.md 10.4, 10.9), T12 items 3-5 and a flat pie (`plot-types-more`: stacked and 100 % stacked area, fill area, histogram, box chart, bubble, colour mapped, pie; Origin's per-template colour lists), T06 (`opj-graph-rest`: X offsets, waterfall as offsets, month/day/year ticks, window places in written `.opj`), T10 (`worksheet-ops`: insert/delete rows, columns and cells, move, hide, width, Origin's sort rules, nested sort, masks, a live filter, transpose/stack/unstack/split/reduce, categorical, Group and Subject), T11 (`axis-scales`: eight more scales, Reverse, First Tick, Prefix/Suffix, an axis break, special ticks, tick label formula, minor tick labels, tick colour, Exchange X-Y, the axis dialog's Apply To and Bottom/Top/Left/Right pages), T12 items 1-2 (`d0dbb37`), T03 (`859c78e`; analysis results that recalculate are named in `.opj` save warnings), T05 (`c32e585`), T21 (`b015f97`), T20 (`522c625`), T14 items 1-4 (`c44a1a6`), T01 (`a2fb77b`), T08 (`04a8a80`), T07 (`826f390`), T17 part (`e209068`), T02
(`bbdb60a`), T09 (`20c37e1`), T16 part (`b967898`). See each brief's Status for what is left.

Worktrees left on 2026-11-13: `axis-range2` and `stats-breadth` (session "Handoff.md token
optimization", free for reuse with their build folders `target/wt-axis` and `target/wt-stats`),
`axis-scales` and `stats-more2` (session "Rust prompt handoff"), the matrix worktree (keep), and
`merge-05b`, an unfinished merge of branches that are all on `main` already (abort it, do not push it).
The disk is about 96 % full; reuse a worktree and its build folder instead of making new ones.

## Next step

Pick the first open brief in `docs/tasks/README.md`. Suggested order by effect on the owner's figures and
analysis: T13 and T14 items 5-6, T16 item 6, T18 backlog items that need no Origin
(briefs first), merging the matrix branch (asks the owner first: it is another session's unpushed work), T19 item 2 and T04 (wizards, need Origin pages), then
the leftovers named in each brief's Status (T22-T24 leftovers: level crossing and cumulative sum; drawn
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
6. If scripting cannot open them: screenshots of Origin's Peak Analyzer pages 2+, Import Wizard
   pages 2-6 (briefs T04, T19). These two wizards are the open briefs that wait on them.
7. **Origin checks for the macOS session's work** (no Origin there; each item is marked "not measured" in its
   spec). A session on the Windows machine can answer them with `tools/origin-ref/`: polar (graphing 14.18:
   default legend place, whether the radius limits round to a tick, angle units and zero direction); horizontal
   stacked and floating bars (14.12: colour lists, legend order, the 100 % axis); ASCII import (import-ascii.md
   2, 3, 10: whether impASC's dialog has Apply, Data Import Options names and defaults, the 18-character
   short-name limit); Column Properties (workbook-model.md: day-of-week numbers from 1 = Sunday, Engineering
   digits and SI letters, name-style wording, elapsed-time patterns for negative values); radar (14.19: first
   spoke up and clockwise, scale from 0, spoke names, grid polygons, page and legend place); ternary (14.20:
   corner and axis directions, 0.2 step, when Origin uses 0-100, grid and legend defaults, tick direction,
   rows with a negative part); pies (14.21: the 3D pie's band thickness and shade, Origin's hatch list); curve mathematics
   (`analysis-curve-math.md`: avecurves default X set, interpolation, range rule, SD divisor and output
   names; intersect curve count, order, touching points; polyarea sign, closing rule, centroid and
   perimeter output); data manipulation (analysis-tools.md 12: X-Function names, how Subtract Straight
   Line picks its points and where it writes, Reduce to Evenly Spaced X choices, bins, default step and
   ends, Curve Translate in place or copy, Median Filter odd window, edge rule and default size);
   drawn objects (graphing.md 17: one object per drag, Shift snapping, attachment default, properties
   tabs, default width/colour/head size).

## Local-only resources

- `re-workspace/scratch/`: scratch scripts, raw Origin outputs (`audit-2/`, `audit-3/`, `audit-3b/`
  hold the measurements cited in the audit), synthetic files (`gen_chi.js` writes CHI test files),
  screenshots; `peer-notes.md` is the note file between sessions.
- The owner's `.opj` and `.bin` files are in the repo root (git-ignored, read-only inputs).
- No Ghidra project and no saved REA evidence bundle exist.
- Origin is driven through COM by `tools/origin-ref/` (system-wide lock: one Origin at a time). A
  hidden Origin started by an agent may still be running.
