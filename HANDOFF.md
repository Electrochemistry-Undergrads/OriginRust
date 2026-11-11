# Handoff

Last updated: 2026-11-11, fourth session, start of work.

## Where things stand

- **The repository is on GitHub** (private): https://github.com/UlanS1/OriginRust. `main` is
  pushed; CI ran for the first time on this push.
- **Milestone 2 (first vertical slice)** works. The independent review found 11 real problems
  (listed below); they are being fixed. Then the milestone is tagged `m2-first-slice`.
- **Milestone 3 (analysis)**: everything except the FFT filter (smoothing) and cubic B-spline
  interpolation is built and checked against Origin.
- **Milestone 4 (fitting)**: `crates/fit` has the shared report types (`report.rs`), t and F
  probabilities (`dist.rs`, tested) and empty modules `linear`, `nlsf`, `library`, `fdf`.
- **Milestone 5 (Origin projects)**: workbooks and column data are read. Tools to compare the
  reader with Origin cell by cell exist (`b3f97be`) but have not been run to the end.

All checks pass at `b3f97be`: `cargo fmt --check`, `cargo clippy --workspace --all-targets` (no
warnings) and `cargo test --workspace` (about 270 tests).

## In progress (background agents, each in its own git worktree under `.claude/worktrees/`)

Each agent commits on its own `worktree-...` branch; nothing reaches `main` until it is reviewed
and merged. If a session ends before they finish, list the branches with `git branch` and the
worktrees with `git worktree list`; merge what is complete, delete the rest.

1. Import fixes (`crates/io/src/ascii.rs`): a text cell such as `n.d.` in the first rows turned
   data rows into header lines; `NA`, `--`, `n.d.` are not read as missing; quoted CSV fields
   with line breaks split a record.
2. App and document fixes: Enter in a cell opens the next cell empty and clicking away erases it;
   opening a cell and leaving it rounds the stored number; the import dialog re-reads the whole
   file every frame; an absurd row range in Set Column Values aborts the app; adding a plot
   recolours all plots; a workbook without sheets panics.
3. Figure and command-line fixes: the Y axis title is cut off when tick labels are wide; unclear
   errors for a zero size or reversed axis limits.
4. FFT filter smoothing and cubic B-spline interpolation (`crates/calc`).
5. Linear and polynomial fits (`crates/fit/src/linear.rs`, `docs/spec/fitting-linear.md`).
6. Nonlinear fitting, built-in functions and `.FDF` import (`nlsf.rs`, `library.rs`, `fdf.rs`,
   `docs/spec/fitting.md`).
7. Origin project reader: cell-by-cell comparison with Origin on the owner's five projects, then
   reading basic graphs (`crates/io/src/opj`, `docs/spec/opj-format.md`).

The independent review of milestone 3 (analysis) at `d27d87f` is finished: five reviewers, each
finding re-checked by a second one; 16 of 40 findings confirmed, to be fixed after the merges
above (they touch files the agents are editing):

- Medium: Integrate, Statistics (and Peaks) result tables take every row's unit from the first
  chosen column; choosing the A and mA columns of a CH Instruments import labels the mA row's
  area A·V (factor 1000). Needs a unit per row (`ResultTable`, the result sheet, the CLI table).
- Medium: `sheaf analyze sort|filter` accept `--rows`, `--x-range`, `--y` and ignore them.
- Low: Set Column Values on part of a text column loses the text outside the range; the
  smoothing derivative with an unusable window (points 1, or order above the polynomial order)
  gives an empty column without an error; normalize divide by 0 writes infinity; `stats` sum of an
  empty column is -0; absolute-area sign test multiplies (underflows below 1e-154); per-row tools
  do not name the columns they added and the `--x-range` error shows a row-range example.
- Low, expression language: `^`/`pow` loses missing values (`na()^0` gives 1); `sd` of data with
  infinity gives 0; `round(x, missing)` ignores the missing digits; a chain of about 200 operators
  or 56 nested calls overflows the 1 MB stack in debug builds.
- Low, tests: the comment in `tools/origin-ref/interp-suite1.labtalk.txt` says 35 points (it is
  33); one assertion in `crates/calc/tests/baseline.rs` (lines 176-179) checks nothing.
- Also worth doing (rejected as "intended", but data loss): an in-place filter that keeps no row
  empties the sheet without a warning.

Done in this session so far: the GitHub repository (`dd86f77`), ADR 0006 on fit results
(`d27d87f`), tag `m1-foundations`, first CI run passed.

## Exact next step

1. Merge the agents' branches after their reviews; run the full checks; push.
2. Tag `m2-first-slice` when stream 1 to 3 are merged.
3. Put fitting into the app and the command-line program (`sheaf fit`, a Fit window, the fitted
   curve on the graph, a report sheet) following `docs/adr/0005-analysis-results.md`.
4. Independent review of milestone 3; tag `m3-analysis`.

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

1. **Try the app** (`cargo run --release -p sheaf-app`), import one of your real CH Instruments
   files, try Analysis > Integrate on a scan, and tell me what is wrong or missing. Real CV and
   i-t export files in the repo folder would also let me check the importer against real headers.
2. Example figures you make most often, to match their style.
3. How you choose the limits when you integrate a peak in Origin (by rows, by potential, or by
   dragging on the graph). The app offers rows and an X interval today; dragging on the graph is
   not built.

Received: five `.opj` projects in the repo root. They are ignored by git and are used only on this
computer to develop and check the Origin project reader.

## Local-only resources

- `re-workspace/scratch/` holds scratch scripts, raw dumps, synthetic test files and screenshots.
  `scratch/calc/p1..p4, q1..q2` are Origin outputs of the FFT filter and B-spline interpolation
  on our own inputs (from the previous session); `scratch/opj/ours/*.opj.txt` are our reader's
  listings of the owner's projects.
- The owner's `.opj` files are in the repo root (git-ignored).
- No Ghidra project and no saved REA evidence bundle exist.
- Origin can be driven through COM automation with `tools/origin-ref/run-labtalk.ps1`.
- Git identity is set for this repository only (`UlanS1`, GitHub no-reply address).
