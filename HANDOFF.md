# Handoff

Last rewritten: 2026-11-12, about 22:30. This file is short on purpose. **To start work, read this
file, then `docs/tasks/README.md`, then one brief `docs/tasks/Txx-*.md`.** Do not read
`docs/audit/origin-differences-2026-11-12.md` or whole specs up front; the brief names the section to
open. History of what was merged when is in `git log` (not repeated here).

## Standing rules from the owner

- **When unsure, copy Origin 2016:** measure it through `tools/origin-ref/` (or read its docs for
  behaviour) and copy it; record the measurement in the spec. Don't ask the owner what Origin can
  answer. Clean-room: copy behaviour and layout, never code, icons, help text or files.
- **Subagents:** Sonnet for building, research, reviews; Haiku for running tests and checking outputs.
- **No co-author line:** never add `Co-Authored-By: Claude ...` or "Generated with Claude Code" to
  commits or PRs; say so in every subagent prompt. (History was rewritten on 2026-11-12 to remove
  them; branches older than that must be rebased onto `main`, matching commits by tree.)
- **Repository:** https://github.com/Electrochemistry-Undergrads/OriginRust (private). Nothing on the
  owner's personal account (UlanS1).
- **Two or more sessions** often share this repo. Claim work in `re-workspace/scratch/peer-notes.md`
  (local only, read its last 40 lines on start); merge to `main` one branch at a time, after
  `git fetch`. See LESSONS "Two sessions on one repository need a written split".

## State of `main` (`6913bfe` + this rewrite)

- Milestones 1-5 done (M2 waits on the owner's click-through; M5 lacks plot groups/legend frames).
  Owner requests of 2026-11-11 done: parity audit, Origin-style workspace (ADR 0007, most steps),
  `.opj` writer for workbooks and graphs, `.opj` as default save with a `.opj.sheaf` companion for
  what the writer cannot store (ADR 0008). Now in milestone 6 (breadth) plus closing Origin
  differences.
- The owner's daily workflow (CH Instruments import, CSV, overlay plots, integration, fitting,
  figures) works end to end.
- Tests: 1057 passed after the last code merge (`9e7ef2b`, per peer notes; not re-run since,
  only docs changed). Parity table: 41 done, 109 partial, 80 missing of 230 rows; P0 31 / 70 / 0
  (`python tools/parity-count.py`).

## In flight (do not start these; check `git branch -a` and peer-notes for newer state)

| Branch | Owner session | What | State at 22:05 |
|---|---|---|---|
| `axis-range` | "Rust prompt handoff" | Automatic axis range as Origin (8 % margin, step rule {1,2,2.5,5}, log padding), Rescale Margin, `nice_range` in the `.opj` writer | started, nothing committed |
| `graph-defaults` | "Rust prompt handoff" | Origin's default colour list, tick label font 18 pt, axis line 1.5 pt, tick length, symbol size, dash patterns | started, nothing committed |
| `opj-params-welch` | "Rust prompt handoff" | `.opj` writer names parameter rows and column formats (`a2952df`, done); Welch t-test interval as Origin (pooled SE, df n1+n2-2) | half done |
| `nlfit-origin` | "Rust prompt handoff" | NLFit tabs, lower panel, Origin's Fit/Done/Cancel | 3 commits, not merged |
| `sheet-menus` | 05b367c2 | Worksheet scrolls on drag past edge; typing into a block; fit report fixed-parameter SE = 0 | started |
| `opj-groups-frames` | 05b367c2 | `.opj` reader keeps plot groups and legend frames; frames drawn | started |
| `export-margin` | 05b367c2 | Auto Apply in the analysis frame; export margin options | started |
| `text-numeric` | 05b367c2 | Mixed text/number import kept as Origin's Text & Numeric | started |
| (several) | `originrust-3a` | Matrices, contour plots, heatmaps | running |

Stale worktrees of merged branches (`merge-keys-nlfit`, `merge-pub-export`, `nlfit-steps`,
`handoff-rule`, `origin-diff-4`, `worktree-agent-*`) can be removed by whoever merges next
(`git worktree remove`, then `tools/clean-target.sh`).

## Next step

Pick the first open brief in `docs/tasks/README.md` whose "Status" has no unmet dependency. Suggested
order by effect on the owner's work: T01 (greyed items with code), T07 (export/copy), T17 (docs, cheap),
T03, T05, T12, then the rest. All earlier "open questions" are assigned to the in-flight branches
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
