# Sheaf

Sheaf is a native Rust desktop application for lab data: import instrument exports, organise them
in workbooks, make publication figures, and run analysis such as integration, smoothing, peak
finding and curve fitting. It is being written to do the jobs its owner uses OriginLab Origin 2016
for, with a familiar workflow, and to open existing Origin project files.

"Sheaf" is a working name. The project is not affiliated with OriginLab. It is an independent,
clean-room implementation: behaviour was learned from documentation, readable configuration files
and black-box observation, written down as specifications in `docs/spec/`, and implemented from
those. No OriginLab code or assets are in this repository.

## Status

Early development. See `HANDOFF.md` for the exact current state and `docs/ROADMAP.md` for the plan.

## Layout

| Path | What it is |
|---|---|
| `crates/core` | Document model: project, workbooks, columns, graphs, figure templates, undo |
| `crates/calc` | Numerics on plain number slices |
| `crates/io` | Importers (CH Instruments text, delimited text), project file, exporters |
| `crates/plot` | Figure rendering to SVG, PNG and PDF, independent of the GUI |
| `crates/cli` | `sheaf` command-line program (no window) |
| `crates/app` | Desktop application |
| `docs/spec` | Specifications the code is written from |
| `docs/adr` | Design decisions and their reasons |
| `docs/ROADMAP.md` | Milestones and tasks |
| `docs/LESSONS.md` | Things learned that later sessions should know |
| `HANDOFF.md` | Where work stopped and what to do next |
| `ORIGIN_RUST_PROMPT.md` | The brief this project is built from |

## Building

```
cargo build --release
cargo test --workspace
```

The command-line program is `target/release/sheaf` and the desktop application is
`target/release/sheaf-app`.
