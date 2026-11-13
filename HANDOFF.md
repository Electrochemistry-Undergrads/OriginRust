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
- Tests: 1170 passed on `main` at `c32e585` (2026-11-13). Parity table: 42 done, 108 partial, 80 missing of 230 rows; P0 32 / 69 / 0 (`python tools/parity-count.py`).

## In flight (do not start these; check `git branch -a` and peer-notes for newer state)

| Branch | Owner session | What | State |
|---|---|---|---|
| `worktree-agent-a79f85b1afb5f0034` | `originrust-3a` | Matrices, heat maps, contour plots, `sheaf matrix` CLI (`7994c3b`..`8c21175`, 48 files, 10k lines) | 4 commits, local only, based on `148e9b1`, not merged; session offline since 2026-11-12 19:00 |
| `analysis-frame` | "Handoff.md token optimization" | T03 analysis dialogs (worktree `stats-breadth`) | started 2026-11-13 |

`merge-05b` (worktree `.claude/worktrees/merge-05b`) is session 05b367c2's unfinished merge of the same
branches that are now on `main`; it holds nothing `main` lacks and can be removed.

On 2026-11-13 no other OriginRust session was live, so the finished branches of the other sessions were
integrated into `main` by "Handoff.md token optimization": `text-numeric`, `opj-params-welch` (plus the
Welch interval, `f522bb2`), `opj-groups-frames`, `export-margin`, `sheet-menus` (its Multi-Panel and Y
Error duplicates of T01 dropped), `nlfit-origin`. `axis-range` and `graph-defaults` never got a commit:
they are briefs T20 and T21 now.

Merged from briefs: T05 (`c32e585`), T21 (`b015f97`), T20 (`522c625`), T14 items 1-4 (`c44a1a6`), T01 (`a2fb77b`), T08 (`04a8a80`), T07 (`826f390`), T17 part (`e209068`), T02
(`bbdb60a`), T09 (`20c37e1`), T16 part (`b967898`). See each brief's Status for what is left.

Stale worktrees of merged branches (`.claude/worktrees/agent-*`, `wf_*`, `merge-*`, `origin-diff-4`,
`opj-graph-write2`, `export-options`, `graph-analysis`) and their `target/wt-*` build folders can be
removed (`git worktree remove`, then `tools/clean-target.sh`); the disk is at 94 %.

## Next step

Pick the first open brief in `docs/tasks/README.md`. Suggested order by effect on the owner's figures and
analysis: T03 (in flight), T12, T15, T13, T11, then the rest. All earlier "open questions" are assigned to the in-flight branches
above or to briefs; none waits on the owner.

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

0. For the `.bin` reader: export `GCD(0.5C).bin`, the CV and one i-t file as text from the CH
   Instruments program, saved next to each `.bin` with the same name (or allow an agent to run the
   CH Instruments program when no experiment is running).
1. **Try the app** (`cargo run --release -p sheaf-app`): import a real CH Instruments file, plot,
   right-click and double-click the graph, try Analysis > Integrate and Fit; report what is wrong.
   (Also checks the i-t text importer against a real export.)
2. Example figures you make most often, to match their style.
3. How you choose integration limits in Origin (rows, potential, or dragging on the graph).
4. Charge/discharge curves: against time or capacity, and per gram (needs the mass)?
5. May the remote branch `chi-text-real` be deleted? It holds a few numbers from your
   charge-discharge file in a spec paragraph (the copy on `main` does not).
6. If scripting cannot open them: screenshots of Origin's Peak Analyzer pages 2+, Import Wizard
   pages 2-6 and NLFit's Code and Parameters pages (briefs T04, T19; NLFit: branch `nlfit-origin`).

## Local-only resources

- `re-workspace/scratch/`: scratch scripts, raw Origin outputs (`audit-2/`, `audit-3/`, `audit-3b/`
  hold the measurements cited in the audit), synthetic files (`gen_chi.js` writes CHI test files),
  screenshots; `peer-notes.md` is the note file between sessions.
- The owner's `.opj` and `.bin` files are in the repo root (git-ignored, read-only inputs).
- No Ghidra project and no saved REA evidence bundle exist.
- Origin is driven through COM by `tools/origin-ref/` (system-wide lock: one Origin at a time). A
  hidden Origin started by an agent may still be running.
