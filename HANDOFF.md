# Handoff

Last updated: 2026-11-12, about 16:00, by session 05b367c2 ("Sonnet subagents from
ORIGIN_RUST_PROMPT"). The owner asked this session to finish the open work while the session
`originrust-3a` had made no change since 14:23, so this session now merges to `main` and edits this
file, docs/ROADMAP.md and docs/LESSONS.md. If `originrust-3a` resumes, read
`re-workspace/scratch/peer-notes.md` first; only one session should merge at a time.

**Merged at about 16:00:** branch `opj-graph-read` (`e46863f`): the `.opj` reader keeps the offsets
of stacked curves (the owner's XRD graphs now draw apart, as in Origin), reads the second layer of a
double-Y graph as a linked layer, and the `\(nnn)` text escape works. The "34 of 41 graphs missed"
in "Integration of Delithiation SF data" are sparkline previews, not graph windows: the file holds
7 graphs and all are read (opj-format.md 10.0).

## The owner's requests of 2026-11-11 (they come before the rest of milestone 6)

1. The desktop app should **look and work like Origin** (OriginPro 2016): same window arrangement,
   menu names and order, dialogs, right-click menus, double-click actions and shortcuts. Icons,
   artwork and help text stay our own (clean-room rule). Plan: ADR 0007.
2. Sheaf should have **all of Origin's features**, checked, not assumed (`docs/spec/feature-parity.md`).
3. **`.opj` is the default project format.** While the writer cannot store everything, saving also
   writes a `<name>.opj.sheaf` companion and says what only the copy holds (ADR 0008).

## Where things stand

- **GitHub** (private): https://github.com/UlanS1/OriginRust. CI runs on push.
- Milestones 1 to 5 are done and tagged (milestone 2 still needs the owner for two checks; milestone
  5 misses some graphs of one owner project). Owner requests: parity audit, interface study, `.opj`
  writer for workbooks and graphs, `.opj` as default with companion: done.
- **Merged on 2026-11-12 (afternoon)**:
  - The second session's graph work (branch `graph-p0`, its four commits up to `6c668ed`): double-Y
    as linked layers, stacked lines and waterfall, column charts with sample names, multi-panel
    graphs. An independent review found that saving to `.opj` silently dropped all of it; fixed in
    `11f6fdb` (the writer now names each such feature, so the companion keeps it). Its ADRs are now
    0009 and 0010.
  - ADR 0007 step 8, the graph window (`0e23919`): page on grey ground, layer icons and an active
    layer, click selects, double-click opens the settings of that axis, plot or legend, Origin's
    right-click menus per object, Delete, Ctrl+R, Layer Contents (F12), Plot Setup. Picking uses new
    `sheaf_plot::hit` (the scene records which items belong to which object).
  - The "popup that swallows the next click" bug was not reproduced; a UI test now guards it.
  - Window places saved in the project (ADR 0007 step 10, `fb9471c`), the axis dialog and Plot
    Details in Origin's layout with OK/Cancel/Apply (step 9, by a subagent), and the Integrate
    gadget (Gadgets > Integrate). Then Copy Page (Ctrl+J), the Data Reader tool and the Rescale and
    Column toolbar buttons. All checks pass on `main` (`ba9f527`).
- **Merged later on 2026-11-12**: column formulas and nested folders in written `.opj` files
  (a subagent; checked in Origin 2016, where the formulas recalculate), and area plots. A save to
  `.opj` now needs the companion only for graph features the writer does not know yet. 915 tests
  pass on `main`.
- Also merged: the nonlinear fit dialog (Origin's NLFit layout: Category/Function, Settings pages,
  Code, Parameters, Bounds; Fit tries without writing, Done keeps the result) and the import dialog
  as Origin's tree of option groups. 917 tests pass on `main`. No subagent is running.

## What is left (overview, 2026-11-12)

The owner's daily workflow (CH Instruments import, CSV, overlay plots, integration, fitting,
figures) works end to end. The parity table (`docs/spec/feature-parity.md`, recount with
`python tools/parity-count.py`) has 230 rows: 38 done, 101 partial, 91 missing; of the 101 P0 rows
30 done, 65 partial, 6 missing. These counts predate the work of 2026-11-12 afternoon (graph window,
dialogs, gadget, Data Reader, area plots, formulas and folders in `.opj`); the rows were not
updated, so the real "done" numbers are higher. Updating the rows is a task of its own.

1. **Checks only the owner can do**: a click-through of the app by a person; the i-t text importer
   against a real export; the `.bin` details (chronopotentiometry sample interval, current sign,
   exact comparison for `GCD(0.5C).bin`) need text exports of the same runs.
2. **Origin look and feel (ADR 0007)**: column properties and the analysis-tool dialogs are not
   in Origin's layout; no Import Wizard (Ctrl+3); NLFit lacks "1 Iteration" and "Fit till
   converged" (the fit engine needs single steps); no per-plot left/right Y axis choice in the
   app (the CLI has it); many menu items are greyed because the feature is missing.
3. **`.opj` files**: saving does not yet write double-Y, stack offsets, panels, bar spacing, area
   plots, tick number formats or window places (the companion keeps them); the reader does not
   read plot groups, legend frames, panel links or sparklines (the small previews in a sheet's
   sparkline row, which Origin lists as graphs); `.opju` is not supported.
4. **Breadth (milestone 6)**: reciprocal and other scales, stacked/fill area, pie, box,
   histogram plot type, bubble, polar, ternary; matrices, contour plots and heatmaps (nothing yet);
   statistics beyond t-tests, one-way ANOVA, frequency counts and normality (two-way ANOVA,
   nonparametric tests, correlation); analysis templates (batch import exists); an embedded
   scripting language; 3D graphs (last).
5. **Smaller points**: automatic axis ranges can leave a wide empty band; plot groups and
   log-axis tick labels do not look like Origin's; B-spline with smoothing factor 0 differs from
   Origin.

## Exact next step

1. Write the new graph features into `.opj` (linked layers for double-Y and panels, stack
   offsets, bar spacing, tick label sources, area plots, tick number formats) so the companion is
   rarely needed. The formula work is the model: `tools/origin-ref/opj-verify-features.ps1`, spec
   13.10, checked by opening the written files in Origin 2016.
2. The remaining dialogs in Origin's layout (analysis tools, column properties, Import Wizard;
   `docs/spec/gui-look.md` section 5). Open choices from the dialog work: the graph's long name is
   on Plot Details' Page tab; a plot's legend text is on its Group tab.
3. A per-plot left/right axis choice in the app (offsets are already in Plot Details > Stack).
4. Update the rows of `docs/spec/feature-parity.md` for the work of 2026-11-12, then follow its
   order of work (matrices and heatmaps are the largest missing area).

The small points of the graph review are fixed too: `--offsets` gives its values to stack members
(a fitted curve takes none), and deleting a column drops the tick labels and offsets that name it.

## Open questions noted by the agents (no answer needed to continue)

- NLFit: should Fit write its results and keep the dialog open, as Origin does (today Fit only
  tries and Done writes)? Should Cancel then undo them?

- Fit reports show the standard error of a fixed parameter as blank (Origin's sheet stores 0).
- Origin's bound handling can stop above the true constrained minimum; Sheaf copies it.
- `.opj` graphs: plot groups (Origin's colour increments) are not modelled; legend frames and
  Origin-style log tick labels are not drawn.
- The automatic axis range can leave a wide empty band (data from -0.2 to 1.0 gave -0.5 to 1.0).

## Commands that verify the current state

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p sheaf-app -- data.csv --screenshot window.png
cargo run -p sheaf-cli -- plot a.txt b.txt -t cv --panels stack -o cv.png
```

Build with `CARGO_TARGET_DIR=target/lead` and `CARGO_PROFILE_DEV_DEBUG=0` (the disk filled once; it
is at about 88 %). `re-workspace/scratch/gen_chi.js` (local only) writes synthetic test files.

## Needs from me (the owner)

0. For the `.bin` reader: export `GCD(0.5C).bin`, the CV and one i-t file as text from the CH
   Instruments program and put each `.txt` next to its `.bin` (same name). Or allow me to run a copy
   of the CH Instruments program myself (only when no experiment is running).
1. **Try the app** (`cargo run --release -p sheaf-app`): import a real CH Instruments file, plot,
   right-click and double-click the graph, try Analysis > Integrate and Fit; tell me what is wrong.
2. Example figures you make most often, to match their style.
3. How you choose integration limits in Origin (by rows, by potential, or by dragging on the graph).
4. Do you plot charge/discharge curves against time or capacity, and per gram (needs the mass)?
4a. NLFit: should Fit keep its results and leave the dialog open, as in Origin? (Today Fit only
   tries and Done writes.)
5. May I delete the remote branch `chi-text-real` on GitHub? It holds a few numbers from your
   charge-discharge file in a spec paragraph (the merged copy on `main` does not).

## Local-only resources

- `re-workspace/scratch/` holds scratch scripts, raw Origin dumps, synthetic files and screenshots;
  `re-workspace/scratch/peer-notes.md` is the note file between the two sessions.
- The owner's `.opj` and `.bin` files are in the repo root (git-ignored).
- No Ghidra project and no saved REA evidence bundle exist.
- Origin is driven through COM by `tools/origin-ref/` (system-wide lock: one Origin at a time).
- A hidden Origin instance started by an agent of the second session may still be running.
