# Handoff

Last updated: 2026-11-11, second session, while work is in progress.

## Where things stand

- **Milestone 2 (first vertical slice) is built and works**: import of CH Instruments and delimited
  text files, workbooks, overlay plots, figure export (PNG with DPI, SVG, PDF), project save and
  open, in the command-line program and in the desktop app. What is left of it: an independent
  review, and a person clicking through the app once (see "Not verified").
- **Milestone 3 (analysis) and milestone 5 (Origin project import) have started** through two
  background agents; their work may be half-written (see "In progress").
- The repository exists **only on this computer**. Creating the GitHub repository is refused by
  Claude Code's automatic permission check, so nothing has been pushed. See "Needs from me".

## Finished and verified in this session

Commits `cc88b2b` and `c2d2b05` (plus later ones; see `git log`).

- `sheaf-io`: CH Instruments importer with scan splitting, delimited-text importer, project file,
  text export. 42 tests pass.
- `sheaf-plot`: SVG, PNG and PDF rendering. 59 tests pass, including a stored reference image for
  each of the four built-in templates (`crates/plot/tests/golden/`).
- `sheaf-cli` (`sheaf info | import | plot | export | export-data`): run by hand on synthetic CV,
  current-vs-time and CSV files; the PNG figures were looked at. The cyclic voltammetry figure
  matches the owner's style (smooth lines, no grid, ticks in, closed box, symmetric current axis).
- `sheaf-expr`: expression language for column formulas and fit functions. 21 tests pass. Spec in
  `docs/spec/expression-language.md`.
- `sheaf-core`: 28 tests pass. New: `Project::set_column_values` (column formulas),
  `Document::adjust_view`, and default plot columns that keep one unit (a current-vs-time file
  plots only the current in mA, not current and charge on one axis).
- `sheaf-app`: menus, project explorer, docked workbook and graph tabs, import dialog with options
  and preview, column properties, set column values, graph settings (axes, plots, legend, page),
  export dialog, save and open, undo and redo, drag and drop, unsaved-changes question.
  Each window was checked with a screenshot (`sheaf-app FILE --show ITEM --screenshot OUT.png`).

## Not verified

- **Mouse and keyboard use of the desktop app.** Screenshots show that every window draws
  correctly, but nobody has clicked through it: cell editing, column selection, the context
  menus, live editing in the graph settings, file choosers and keyboard shortcuts are untested.
- The CI workflow (`.github/workflows/ci.yml`) has never run, because nothing is pushed.
- The CH Instruments importer has only seen synthetic files written to the layout in
  `docs/spec/chi-text-format.md`, never a real export.

## In progress (may be half-written if the session was cut off)

- **Analysis numerics agent** (`crates/calc`, `docs/spec/analysis-tools.md`, new LabTalk scripts
  in `tools/origin-ref/`): integrate, differentiate, smooth, normalize, interpolate, baseline
  subtraction, peak finding, descriptive statistics, each checked against numbers from the
  installed Origin. Check which tools are complete before trusting them.
- **Origin project reader agent** (`crates/io/src/opj/`, `crates/io/examples/opj_dump.rs`,
  `docs/spec/opj-format.md`): reads workbooks and column data from `.opj` files. While it is
  unfinished, `crates/io` may not compile (the line `pub mod opj;` in `crates/io/src/lib.rs`).
  If it is broken at resume, either finish the module or remove that line.

## Exact next step

1. Run the verification commands below. Finish or repair `crates/calc` and `crates/io/src/opj`.
2. Put the analysis tools into the app and the command-line program: an Analysis menu
   (integrate with a baseline, differentiate, smooth, normalize, statistics) that writes results
   to new columns, and `sheaf analyze`.
3. Add "Open Origin project" to the app and `sheaf info file.opj` to the command-line program;
   compare a few cells against Origin for the owner's five projects (Origin can be driven by
   `tools/origin-ref/run-labtalk.ps1`).
4. Fresh-context review of milestone 2 against `docs/ROADMAP.md` and the specs; fix findings; tag
   `m2-first-slice`.
5. Milestone 4 (fitting). Note for the `.FDF` import: formulas in those files are written in
   Origin's C dialect, where `log` is the natural logarithm, while in column formulas `log` is
   base 10 (see `docs/LESSONS.md`).

## Commands that verify the current state

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace
cargo run -p sheaf-cli -- plot some_cv_export.txt -o cv.png --dpi 300
cargo run -p sheaf-app -- some_cv_export.txt --screenshot window.png
```

`re-workspace/scratch/gen_chi.js` (local only) writes synthetic CV, current-vs-time and CSV files:
`node re-workspace/scratch/gen_chi.js re-workspace/scratch/cli`.

## Needs from me (the owner)

1. **Create the GitHub repository.** Run this once in the repo folder:
   `gh repo create OriginRust --private --source . --remote origin --push`
   (or add a permission rule that lets the assistant run `gh repo create` and `git push`).
2. **Try the app** (`cargo run --release -p sheaf-app`), import one of your real CH Instruments
   files and tell me what is wrong or missing. Real CV and i-t export files in the repo folder
   would also let me check the importer against real headers.
3. Example figures you make most often, to match their style.

Received: five `.opj` projects in the repo root (thank you). They are ignored by git and are used
only on this computer to develop and check the Origin project reader.

## Local-only resources

- `re-workspace/scratch/` holds scratch scripts, raw dumps, synthetic test files and screenshots.
- The owner's `.opj` files are in the repo root (git-ignored).
- No Ghidra project and no saved REA evidence bundle exist. Only REA's target-free PE tools were
  used so far.
- Origin can be driven through COM automation with `tools/origin-ref/run-labtalk.ps1`; only one
  Origin instance may run at a time.
- Git identity is set for this repository only (`UlanS1`, GitHub no-reply address).
