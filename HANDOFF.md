# Handoff

Last updated: 2026-11-12, about 15:00, by the lead Claude Code session. Only one session should
merge to `main` and edit this file, docs/ROADMAP.md and docs/LESSONS.md (the lead).

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
- **Running when this was written**: a subagent on branch `worktree-agent-a175d68f1d5696c0d` (worktree
  `.claude/worktrees/agent-a175d68f1d5696c0d`) writes column formulas and nested folders into `.opj` and reads them back,
  checked in Origin through COM. If this session ended before merging it, look for that branch
  (`git log main..worktree-agent-a175d68f1d5696c0d`), read its commit messages, run the checks and merge.

Also running: a subagent on branch `worktree-agent-a23ee953326faea53` reworks the nonlinear fit
dialog and the import dialog into Origin's layout (only `crates/app`). If this session ended, check
that branch the same way.

## Exact next step

1. Merge the `.opj` formulas-and-folders branch and the fit/import dialogs branch when they are done
   (full checks after each merge).
2. The other dialogs in Origin's layout (import wizard, NLFit, analysis tools, column
   properties; `docs/spec/gui-look.md` section 5). Open choices from the dialog work: the graph's
   long name is on Plot Details' Page tab; a plot's legend text is on its Group tab.
3. Write the new graph features into `.opj` (linked layers, offsets, bar spacing, tick label
   sources) so the companion is rarely needed; tick number formats in written graphs.
4. A per-plot left/right axis choice in the app (offsets are now in Plot Details > Stack).
5. Then the parity-audit gaps in their order.

The small points of the graph review are fixed too: `--offsets` gives its values to stack members
(a fitted curve takes none), and deleting a column drops the tick labels and offsets that name it.

## Open questions noted by the agents (no answer needed to continue)

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
5. May I delete the remote branch `chi-text-real` on GitHub? It holds a few numbers from your
   charge-discharge file in a spec paragraph (the merged copy on `main` does not).

## Local-only resources

- `re-workspace/scratch/` holds scratch scripts, raw Origin dumps, synthetic files and screenshots;
  `re-workspace/scratch/peer-notes.md` is the note file between the two sessions.
- The owner's `.opj` and `.bin` files are in the repo root (git-ignored).
- No Ghidra project and no saved REA evidence bundle exist.
- Origin is driven through COM by `tools/origin-ref/` (system-wide lock: one Origin at a time).
- A hidden Origin instance started by an agent of the second session may still be running.
