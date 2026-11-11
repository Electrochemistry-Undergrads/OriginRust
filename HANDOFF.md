# Handoff

Last updated: 2026-11-11, first session, while work is in progress.

## Where things stand

- **Milestone 1 (inventory and foundations)** is mostly done; **milestone 2 (first vertical slice)**
  has started.
- The repository exists **only on this computer**. Creating the GitHub repository was refused by
  Claude Code's automatic permission check, so nothing has been pushed. See "Needs from me".

## Finished and verified in this session

- Cargo workspace with six crates; `sheaf-core` (document model, figure templates, undo history)
  has 21 passing tests and is lint-clean.
- Design records `docs/adr/0001` to `0004`.
- Hand-written specs: `docs/spec/chi-text-format.md`, `project-file-format.md`,
  `figure-rendering.md`.

## In progress (may be half-written if the session was cut off)

- Sonnet research agents are writing specs from the installed Origin into `docs/spec/`:
  `ui-command-map.md`, `fdf-format.md`, `fit-function-catalog.md`, `data/fit-functions.json`,
  `analysis-features.md`, `xfunction-index.md`, `import-ascii.md`, `workbook-model.md`,
  `graphing.md`, then `feature-inventory.md`. Check each file is complete before trusting it.
- Sonnet build agents are implementing `crates/io` (CHI and delimited-text importers, project
  file, text export) and `crates/plot` (SVG, PNG, PDF rendering). They may not compile yet.
- `crates/calc`, `crates/cli` and `crates/app` are placeholders.

## Exact next step

1. Run the verification commands below. Fix or finish `crates/io` and `crates/plot` against their
   specs (`docs/spec/chi-text-format.md`, `project-file-format.md`, `figure-rendering.md`).
2. Write `crates/cli` (`sheaf import | plot | export | info`), render a cyclic voltammetry figure
   and look at the PNG.
3. Write the desktop app (`crates/app`).

## Commands that verify the current state

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

## Needs from me (the owner)

1. **Create the GitHub repository.** Run this once in the repo folder, then tell the assistant:
   `gh repo create OriginRust --private --source . --remote origin --push`
   (or allow the assistant to run `gh repo create` and `git push`).
2. Real CH Instruments export files (one cyclic voltammetry file with several cycles, one i-t
   file), so the importer can be checked against real headers. Until then it is tested with
   synthetic files written to the layout in `docs/spec/chi-text-format.md`.
3. One or two of your typical `.opj` projects and example figures to match (needed from
   milestone 5 on; not urgent).

## Local-only resources

- `re-workspace/scratch/` holds the research agents' scratch scripts and raw dumps (git-ignored).
- No Ghidra project and no saved REA evidence bundle exist yet. Only REA's target-free PE tools
  were used so far.
- Git identity is set for this repository only (`UlanS1`, GitHub no-reply address).
