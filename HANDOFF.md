# Handoff

Last updated: 2026-11-11, third session, while work is in progress.

## Where things stand

- **Milestone 2 (first vertical slice) is built and works.** Open: findings of the independent
  review (running, see "In progress"), and a person trying the app once.
- **Milestone 3 (analysis) is nearly done.** The numerics are checked against Origin, and the tools
  are in the app (Analysis menu) and in the command-line program (`sheaf analyze`). Sorting and
  filtering of sheets work. Open: FFT filter smoothing (an agent is on it).
- **Milestone 4 (fitting)** has started in the new crate `crates/fit` (an agent is on it; the crate
  may be half-written).
- **Milestone 5 (Origin project import)**: workbooks and column data are read and can be opened in
  the app and the command-line program. An agent is checking the reader cell by cell against
  Origin and then adding graphs.
- The repository exists **only on this computer**: there is no GitHub remote yet. See "Needs from
  me".

## Finished and verified in this session

Commits `a16d708` and `0a006c3`.

- `sheaf-calc`: integrate, differentiate, smooth (adjacent averaging, Savitzky-Golay, percentile),
  normalize, interpolate (linear, cubic spline, Akima), baseline subtraction, peak finding,
  statistics. 32 tests compare with numbers produced by the installed Origin 2016.
- `sheaf-core`, module `analysis`: `Project::analyze`, `sort_sheet`, `filter_sheet` (10 tests).
  Where results go is decided in `docs/adr/0005-analysis-results.md`.
- `sheaf-cli`: `sheaf analyze integrate|differentiate|smooth|normalize|baseline|interpolate|peaks|
  stats|sort|filter`. Run by hand on synthetic current-vs-time, cyclic voltammetry and CSV files;
  the tables and exported sheets were read.
- `sheaf-app`: Analysis menu, Worksheet > Sort Sheet by Column and Filter Rows. The Integrate,
  Find Peaks and Filter windows were looked at in screenshots; two tests press OK in the Integrate
  and Filter windows without a real window and check the result and the undo step.
- The Origin driver scripts take a system-wide lock, so several agents can ask Origin for
  reference numbers without starting two instances.

## Not verified

- **Mouse and keyboard use of the desktop app by a person.** Tests that click through the app
  without a window exist (`crates/app/src/app/ui_tests.rs`): 3 pass, 8 older drafts are marked
  ignored until an agent has matched them to the real widgets (in progress).
- The CI workflow has never run, because nothing is pushed.
- The CH Instruments importer has only seen synthetic files, never a real export.
- The Analysis windows other than Integrate and Filter were only drawn, not operated.

## In progress (may be half-written if the session was cut off)

Background agents share the working tree. Each owns the paths listed; nothing of theirs is
committed until it is checked.

- **FFT filter and B-spline interpolation**: `crates/calc`, `docs/spec/analysis-tools.md`.
- **Fitting**: `crates/fit` (linear, polynomial, Levenberg-Marquardt, function library, `.FDF`
  import), small additions to `crates/expr`, `docs/spec/fitting.md`,
  `tools/origin-ref/fit-*.labtalk.txt`.
- **Origin project reader**: `crates/io/src/opj`, `crates/io/examples`, `docs/spec/opj-format.md`,
  `tools/origin-ref/opj-*`. First a cell-by-cell comparison with Origin for the owner's five
  projects, then reading of basic graphs.
- **Click tests of the app**: `crates/app` (the eight ignored tests).
- **Review of milestone 2** by three reviewers in private git worktrees under `.claude/`
  (read-only; they change nothing).

If a session starts and these are unfinished: run the verification commands. A crate that does not
compile is one of the above; finish it or set its uncommitted changes aside with
`git stash push <path>`.

## Exact next step

1. Collect the agents' results, check each (tests, lint, a spot check against Origin), commit per
   stream. Fix the confirmed findings of the milestone 2 review; tag `m2-first-slice`.
2. Put fitting into the app and the command-line program (`sheaf fit`, a Fit window, the fitted
   curve on the graph, a report sheet), following `docs/adr/0005-analysis-results.md`.
3. Independent review of milestone 3; tag `m3-analysis`.
4. Milestone 6 (more graph types) by the owner's priorities.

## Commands that verify the current state

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace
cargo run -p sheaf-cli -- analyze integrate some_it_export.txt
cargo run -p sheaf-cli -- plot some_cv_export.txt -o cv.png --dpi 300
cargo run -p sheaf-app -- some_cv_export.txt --show analysis:integrate --screenshot window.png
```

`re-workspace/scratch/gen_chi.js` (local only) writes synthetic CV, current-vs-time and CSV files:
`node re-workspace/scratch/gen_chi.js re-workspace/scratch/cli`.

## Needs from me (the owner)

1. **Create the GitHub repository.** Run this once in the repo folder:
   `gh repo create OriginRust --private --source . --remote origin --push`
   (or add a permission rule that lets the assistant run `gh repo create` and `git push`). Until
   then every checkpoint is a local commit only.
2. **Try the app** (`cargo run --release -p sheaf-app`), import one of your real CH Instruments
   files, try Analysis > Integrate on a scan, and tell me what is wrong or missing. Real CV and
   i-t export files in the repo folder would also let me check the importer against real headers.
3. Example figures you make most often, to match their style.
4. How you choose the limits when you integrate a peak in Origin (by rows, by potential, or by
   dragging on the graph). The app offers rows and an X interval today; dragging on the graph is
   not built.

Received: five `.opj` projects in the repo root. They are ignored by git and are used only on this
computer to develop and check the Origin project reader.

## Local-only resources

- `re-workspace/scratch/` holds scratch scripts, raw dumps, synthetic test files and screenshots.
- The owner's `.opj` files are in the repo root (git-ignored).
- No Ghidra project and no saved REA evidence bundle exist.
- Origin can be driven through COM automation with `tools/origin-ref/run-labtalk.ps1`.
- Git identity is set for this repository only (`UlanS1`, GitHub no-reply address).
