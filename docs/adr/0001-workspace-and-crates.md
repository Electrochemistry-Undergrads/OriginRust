# ADR 0001: Workspace layout, crate boundaries and working name

Status: accepted (2026-11-11)

## Decision

The application's working name is **Sheaf** (a bundle of sheets). It does not use "Origin" or any
OriginLab trademark. The repository is one Cargo workspace:

| Crate | Folder | Depends on | Purpose |
|---|---|---|---|
| `sheaf-core` | `crates/core` | serde | Document model: project, workbooks, columns, graph model, figure templates, undo history. No file formats, no drawing, no GUI. |
| `sheaf-calc` | `crates/calc` | numeric crates only | Numerics on plain slices of numbers: integration, smoothing, fitting, statistics. Does not know about workbooks. |
| `sheaf-io` | `crates/io` | core | Importers (CHI, delimited text), exporters, the project file, later the Origin project reader. |
| `sheaf-plot` | `crates/plot` | core | Turns a graph plus data into SVG, PNG and PDF. No GUI dependency. |
| `sheaf-cli` | `crates/cli` | core, calc, io, plot | The `sheaf` command-line program: everything without a window. Also the test harness for rendering. |
| `sheaf-app` | `crates/app` | all of the above, egui | The desktop application. |

## Reasons

- The graph model lives in `core` (not in `plot`) because a project contains graphs, and the
  project must be loadable and editable without pulling in the renderer.
- `calc` works on slices so that it stays portable, easy to test against reference numbers, and
  usable from scripts later.
- Only `app` depends on a GUI toolkit, so all other crates build and test on any platform and in
  CI without a display.

## Consequences

Glue that applies a calculation to worksheet columns (pick input columns, write output columns,
record the step for undo) lives in the callers for now. If the CLI and the app start to duplicate
it, it moves into its own crate.
