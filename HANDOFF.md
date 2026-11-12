# Handoff

Last updated: 2026-11-11, about 23:30, by the Claude Code session `originrust-3a` (the lead). Only
one session should work on the repository at a time.

## The owner's new requests (2026-11-11, evening)

1. The desktop app should **look and work like Origin** (OriginPro 2016), so the owner's skills
   carry over: same window arrangement, menu names and order, dialogs, right-click menus,
   double-click actions and shortcuts. Icons, artwork and help text stay our own (clean-room rule).
2. Sheaf should have **all of Origin's features**, and this must be checked, not assumed.
3. **`.opj` is to be the default project format** (Save, Save As, `--save`, export). Agreed
   safeguard: while the writer cannot store everything (formulas, folders, some graph details), saving also writes a
   `.sheaf` copy next to the `.opj` and says what only the copy holds.

These come before milestone 6 (see docs/ROADMAP.md, "Owner's requests of 2026-11-11").

## Two sessions at work: who owns what (read this first)

The owner runs two Claude sessions on this repository. Cross-session messages are not getting
through (they are held for approval and expire), so the split is written here:

- **Lead: session `originrust-3a`** (Claude Code, cf09e3ad). Only the lead merges to `main`,
  pushes, and edits HANDOFF.md, docs/ROADMAP.md and docs/LESSONS.md. The lead owns `crates/app`
  (being rebuilt in Origin's style, ADR 0007), the `.opj` writer (`crates/io/src/opj/`, new
  files), the CH Instruments binary reader, curve math, date and time columns and recalculating
  formulas (workflows `chi-binary-reader`, `parity-gui-opj`, `p0-gaps-core`).
- **Second session: "Sonnet subagents from ORIGIN_RUST_PROMPT"** (05b367c2). It owns branch
  `chi-text-real` (CH Instruments text importer checked against the owner's real text files) and
  `graph-p0`: milestone 6 graph work in `crates/core/src/graph.rs`, `crates/plot` and `crates/cli`
  (column and bar plots with text tick labels, offset and stacked plots, multi-panel, double Y,
  log and reciprocal scales, error bars) plus reading plot offsets from `.opj` in
  `crates/io/src/opj/graph.rs`. Please stay out of `crates/app` (expose options in the model,
  renderer and CLI; the lead wires them into the app), never merge to `main`, and leave notes for
  the lead in `re-workspace/scratch/peer-notes.md` (local file) naming each branch that is ready.
- Each session builds in its own folders under `target/` with `CARGO_PROFILE_DEV_DEBUG=0` (the
  disk filled up once; see LESSONS.md).

## Where things stand

- **GitHub** (private): https://github.com/UlanS1/OriginRust. `main` is pushed; CI runs on push.
- **Milestones 1 to 3 are done** and tagged. Milestone 2 still needs the owner for two checks.
- **Milestone 4 (fitting) is done** and tagged `m4-fitting`: its 11 review findings were fixed,
  reviewed again and merged (`fe407b1`, `0ac9546`, `83a09de`).
- **CH Instruments binary files** are read (`2ca7d26`): CV, i-t and chronopotentiometry `.bin`
  files give the same workbooks as text exports. The owner's six `.bin` files (git-ignored) import
  and plot correctly by eye. Open points are in `docs/spec/chi-binary-format.md` section 5 (CP
  sample interval, which CP current is cathodic). CH Instruments programs on the Desktop must not
  be started without the owner's go-ahead (a potentiostat may be connected; their `.ini`/`.cfg`
  files are the owner's settings).
- **Milestone 5**: workbooks and basic graphs are read from `.opj` and merged (`d3c3a3f`). Origin's
  text escapes `\f:`, `\p`, `\x`, `\v` are now understood in labels. Known gaps: 34 of 41 graphs of
  the owner's "Integration of Delithiation SF data" project are missed; XRD curves that Origin
  draws offset are drawn on top of each other (plot offsets are not read).
- **Done on 2026-11-12**: the parity audit (`docs/spec/feature-parity.md`), the interface study
  (`docs/spec/gui-look.md`, ADR 0007 accepted), the `.opj` writer for workbooks (verified in
  Origin 2016), the CH Instruments `.bin` reader, curve math, multi-range integration, date and
  time columns, recalculating formulas, fit bands and controls, CSV/ICP-OES import extras with
  batch import, the peak analysis chain.
- **Merged on 2026-11-12 (afternoon)**: `.opj` is the default save format with a `.opj.sheaf`
  companion (ADR 0008); the app shell has Origin's theme, child windows, Window menu and menu bars
  per window type (ADR 0007 steps 1 to 4); `docs/spec/gui-look.md` completed from a second set of
  pictures (`re-workspace/scratch/gui-study2/`, local only).
- **Merged on 2026-11-12 (midday)**: graphs written into `.opj` (Origin opened 18 written graphs;
  the companion is now needed only for formulas, folders and graph details the writer lacks),
  basic statistics (Statistics menu: t-tests, one-way ANOVA, frequency counts, normality), and the
  second session's CH Instruments text work (`89e617a`, squashed so no numbers from the owner's
  files enter the history). Note: the second session pushed its branch `chi-text-real` to GitHub
  with a few numbers from the owner's charge-discharge file in a spec paragraph; delete that
  remote branch when the second session no longer needs it (ask the owner).
- **Running**: workflow `origin-look-2` (run `wf_4cb25432-db5`): ADR 0007 steps 5 to 7 (workbook
  look with selection and copy/paste, toolbars, Project Explorer and panels). The second session
  works on `graph-p0` (double Y, stacked and waterfall plots committed there).
- **Known app bug** (seen by an agent, not traced): after a click into the graph body a popup layer
  of about 450 x 470 px can stay on screen and swallow the next click on a window title below it.

All checks pass on `main` at `4b02ea3` (687 tests). Parity counts are recomputed with `python tools/parity-count.py`.

## Exact next step

1. Merge the `origin-look-2` branch after its reviews; full checks after each merge. Merge `chi-text-real` and
   `graph-p0` (second session) when its note says they are ready.
2. Continue ADR 0007: steps 8 to 10 (graph window look and mouse actions, dialogs in Origin's
   layout, window geometry in the project file), and fix the popup bug above.
3. Write column formulas (and folders) into `.opj`, so the `.opj.sheaf` companion is rarely
   needed; tick number formats in written graphs.
4. Then the gaps from the parity audit in its order (column and bar charts with text labels,
   offset and stacked plots, multi-panel, double Y, log scales are P0 for the owner).

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

0. For the `.bin` reader: export the charge-discharge file `GCD(0.5C).bin` (most useful), the CV
   and one i-t file as text from the CH Instruments program and put each `.txt` next to its `.bin`
   (same name). A test then compares them automatically. Or allow me to run a copy of the CH
   Instruments program myself (only when no experiment is running).

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
