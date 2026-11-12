# Handoff

Last updated: 2026-11-11, about 21:10, by the Claude Code session `originrust-3a` (the fourth
session, resumed after its usage limit). A fifth session in Claude Desktop ("Sonnet subagents from
ORIGIN_RUST_PROMPT") did the work from 17:30 to 18:40 and then stopped; the owner asked this
session to continue. Only one session should work on the repository at a time.

## Where things stand

- **GitHub** (private): https://github.com/UlanS1/OriginRust. `main` is pushed; CI runs on push.
- **Milestones 1 to 3 are done** and tagged `m1-foundations`, `m2-first-slice`, `m3-analysis`.
  Milestone 2 still needs the owner for two checks (a hands-on try of the app, real CH Instruments
  files).
- **Milestone 4 (fitting) is on `main`**: linear and polynomial fits, the nonlinear engine, 206
  built-in functions, user formulas, `.FDF` import, `Project::fit`, `sheaf fit` and the app's Fit
  window. Each part was reviewed on its branch. Still owed: the milestone review of the merged
  whole, then the tag `m4-fitting`.
- **Milestone 5 (Origin projects)**: the reader for workbooks is on `main`. On the branch
  `worktree-wf_7107af90-7f2-7` (worktree `.claude/worktrees/wf_7107af90-7f2-7`): the cell-by-cell
  comparison with Origin (the owner's five projects match exactly) and reading of basic graphs. Its
  review found that workbooks of pre-9 projects are also read as bogus graphs; an agent is fixing
  that now (workflow "opj-review-fixes"), then the branch is merged.

All checks pass on `main` at `cf81d6b`: `cargo fmt --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo test --workspace`.

Checked by hand in this session: `sheaf fit peak.csv -m Gauss --plot` on a synthetic peak gives the
built-in parameters back and a correct figure; screenshots of the Fit, Smooth and Interpolate
windows look right; a new click test picks the FFT filter in the Smooth window and checks its result.

## Exact next step

1. When the OPJ fix and its review finish: merge `worktree-wf_7107af90-7f2-7` into `main`, run the
   full checks, push, tick milestone 5's first two items.
2. Independent review of milestone 4 (fitting) on `main`; fix the confirmed findings; tag
   `m4-fitting`.
3. Then milestone 6 by the owner's priorities: column and bar charts with text labels (ICP-OES),
   stacked and offset plots and multi-panel layouts (XRD), error bars, double Y, log scales.
4. Clean up: remove merged worktrees and branches (`git worktree remove`, `git branch -d`); the
   three `wf_7107af90-7f2-*` worktrees are marked locked by a stopped workflow, so
   `git worktree unlock` them first.

## Open questions noted by the agents (no answer needed to continue)

- Fit reports show the standard error of a fixed parameter as blank (Origin's sheet stores 0).
- Origin's bound handling can stop above the true constrained minimum; Sheaf copies it. An option
  for the true constrained minimum could be added.
- Should the B-spline label say more clearly that factor 0 is not Origin's result?
- `.opj` graphs: plot groups (Origin's colour increments) are not modelled; legend frames and
  Origin-style log tick labels are not drawn.
- The automatic axis range can leave a wide empty band (data from -0.2 to 1.0 gave an axis from
  -0.5 to 1.0); compare with Origin's rescale rule.

## Commands that verify the current state

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p sheaf-cli -- fit data.csv -m Gauss --plot fit.png
cargo run -p sheaf-cli -- analyze smooth cv.txt --method fft --points 10
cargo run -p sheaf-app -- data.csv --show fit:nonlinear --screenshot window.png
```

`re-workspace/scratch/gen_chi.js` (local only) writes synthetic CV, current-vs-time and CSV files:
`node re-workspace/scratch/gen_chi.js re-workspace/scratch/cli`.

## Needs from me (the owner)

1. **Try the app** (`cargo run --release -p sheaf-app`), import one of your real CH Instruments
   files, try Analysis > Integrate and Analysis > Fit, and tell me what is wrong or missing. Real
   CV and i-t export files in the repo folder would also let me check the importer.
2. Example figures you make most often, to match their style.
3. How you choose the limits when you integrate a peak in Origin (by rows, by potential, or by
   dragging on the graph).

Received: five `.opj` projects in the repo root (git-ignored; used only on this computer).

## Local-only resources

- `re-workspace/scratch/` holds scratch scripts, raw Origin dumps, synthetic files and screenshots.
- The owner's `.opj` files are in the repo root (git-ignored).
- No Ghidra project and no saved REA evidence bundle exist.
- Origin is driven through COM by the scripts in `tools/origin-ref/` (they take a system-wide lock
  so only one Origin runs at a time).
- Git identity is set for this repository only (`UlanS1`, GitHub no-reply address).
