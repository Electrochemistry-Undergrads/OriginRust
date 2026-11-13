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

| Brief | Area | Size | Status |
|---|---|---|---|
| [T01](T01-greyed-menus-with-code.md) | Enable greyed menu items whose feature already exists | M | done (`a2fb77b`); Stack dialog, custom sort left |
| [T02](T02-menu-shortcut-differences.md) | Menu labels, shortcuts and context menus that differ from Origin | M | done (`bbdb60a`); unmeasured details listed in brief |
| [T03](T03-analysis-dialog-frame.md) | Analysis dialogs: Recalculate, Output, smoothing methods, pkFind options | M | open; includes Auto Apply |
| [T04](T04-peak-analyzer-wizard.md) | Peak Analyzer as Origin's multi-page wizard | L | open, needs Origin pages measured |
| [T05](T05-export-graphs-dialog.md) | Export Graphs dialog, raster formats, units, Ctrl+G, Copy Page | M | open; margins, px/%, Copy Page done |
| [T06](T06-opj-remaining-graph-features.md) | `.opj` writer: X offsets, bar overlap, waterfall, date ticks, window places | L | open, needs Origin probes |
| [T07](T07-worksheet-export-and-copy.md) | Worksheet export and clipboard as Origin (missing `--`, formats, label rows) | S | done (`826f390`); clipboard copy left |
| [T08](T08-formula-functions.md) | Missing Set Column Values functions in `sheaf-expr` | M | groups 1-3, 5 done (`04a8a80`); strings, cross-sheet, `sum`/`count` as Origin left |
| [T09](T09-set-values-dialog.md) | Set Column Values dialog in Origin's layout | M | done (`20c37e1`); scripts box, Set Multiple Columns left |
| [T10](T10-worksheet-operations.md) | Insert/delete rows, move/hide columns, mask, sort column, transpose | L | open, split into commits |
| [T11](T11-axis-scales-and-features.md) | More axis scales and axis-dialog features | L | open, after T20 |
| [T12](T12-plot-types.md) | Stacked/horizontal columns, histogram, box, pie, bubble, polar | L | open |
| [T13](T13-plot-details-and-axis-dialog-tabs.md) | Missing Plot Details tabs and axis-dialog controls | M | open, after T11/T12 pieces |
| [T14](T14-statistics-breadth.md) | Descriptive-statistics columns, outlier tests, more tests | M | open |
| [T15](T15-fitting-breadth.md) | Multi-dataset global fit, Simplex, surface fits, compare models | L | open |
| [T16](T16-numeric-differences.md) | Small numeric/behaviour differences (Mann-Whitney print, spline, error caps) | S | 1, 5 done, 4 investigated (`b967898`); 2, 3, 6 open |
| [T17](T17-docs-cleanup.md) | Stale spec paragraphs and parity counts | S | done except item 3 (with T20) |
| [T18](T18-backlog.md) | Large items not yet briefed (scripting, 3D, Options, importers, templates) | - | backlog |
| [T19](T19-import-and-column-dialogs.md) | ASCII import options, Import Wizard pages, Column Properties formats | M | open, wizard needs Origin pages |
| [T20](T20-automatic-axis-range.md) | Automatic axis range as Origin (8 % margin, step rule, log padding), Rescale Margin | M | open |
| [T21](T21-graph-defaults.md) | Graph defaults as Origin: colour order, fonts, line widths, ticks, symbols, dashes | M | open |

In flight (do not start these): matrices/contour/heatmaps (`originrust-3a`). All other branches of
2026-11-12 are merged or turned into briefs (T20, T21).
