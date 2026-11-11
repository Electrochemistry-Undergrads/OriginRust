# Handoff

Last updated: 2026-11-11, fifth session (resumed after a usage limit cut the fourth one off).

## Where things stand

- **GitHub** (private): https://github.com/UlanS1/OriginRust. `main` is pushed.
- **Milestone 2 (first vertical slice)** is done and tagged `m2-first-slice`. Two items still
  need the owner: a hands-on try of the app, and real CH Instruments files.
- **Milestone 3 (analysis)**: all tools except the FFT filter (smoothing) and cubic B-spline
  interpolation are built, checked against Origin, and in the app and `sheaf analyze`. All 16
  confirmed findings of the independent review are fixed (`adb1414`, `2cefe20`, `2aa5b37`).
- **Milestone 4 (fitting)**: on branches, not yet on `main` (see below).
- **Milestone 5 (Origin projects)**: workbooks and column data are read; the cell-by-cell check
  against Origin and graph reading are on a branch.

All checks pass on `main` at `2aa5b37`: `cargo fmt --check`, `cargo clippy --workspace
--all-targets` (no warnings), `cargo test --workspace` (about 286 tests).

## In progress: four agent streams, each in its own git worktree

The fourth session's agents were cut off. Their unfinished work was saved as `wip:` commits on
their branches, `main` was merged into each branch, and the streams were restarted. Nothing
reaches `main` until the lead has reviewed and merged it.

| Stream | Branch | Worktree | State when restarted |
|---|---|---|---|
| FFT filter smoothing, cubic B-spline interpolation | `stream-calc-fft-bspline` | `.claude/worktrees/calc-fft-bspline` | Started fresh from `main`. Earlier Origin runs on our inputs are in `re-workspace/scratch/calc/` (`p4`, `q2`, `r3`, `r4`). |
| Linear and polynomial fits | `worktree-wf_7107af90-7f2-5` | `.claude/worktrees/wf_7107af90-7f2-5` | Draft `crates/fit/src/linear.rs` compiles; no tests, no Origin comparison. |
| Nonlinear fitting, function library, `.FDF` import | `worktree-wf_7107af90-7f2-6` | `.claude/worktrees/wf_7107af90-7f2-6` | Engine, 206 built-in functions and `.FDF` import committed with 20 passing tests; not yet compared with Origin; spec draft. |
| Origin project reader: cell check, then graphs | `worktree-wf_7107af90-7f2-7` | `.claude/worktrees/wf_7107af90-7f2-7` | Reader keeps the raw graph blocks; nothing interprets them; the cell check has not run. |

If a session ends before they finish: `git worktree list` and `git log main..<branch>` show
what each did. Merge what is complete and reviewed; otherwise restart the stream from its branch.

## Exact next step

1. When the streams finish: review each branch, merge one at a time into `main`, run the full
   checks after each merge, push.
2. Put fitting into the app and the command-line program (`sheaf fit`, a Fit window, the fitted
   curve on the graph, report sheets) following `docs/adr/0006-fit-results.md`.
3. Tag `m3-analysis` once the FFT filter and B-spline are merged.

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
- The owner's `.opj` files are in the repo root (git-ignored).
- No Ghidra project and no saved REA evidence bundle exist.
- Origin can be driven through COM automation with `tools/origin-ref/run-labtalk.ps1` (it takes a
  system-wide lock so only one Origin runs at a time).
- Git identity is set for this repository only (`UlanS1`, GitHub no-reply address).
