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

Early development. The first slice works: import CH Instruments and delimited text files, look at
the data in workbooks, make overlay plots, edit the figure, export PNG, SVG and PDF, save and open
projects. See `HANDOFF.md` for the exact current state and `docs/ROADMAP.md` for the plan.

## Using it

Desktop app:

```
cargo run --release -p sheaf-app                 # empty project
cargo run --release -p sheaf-app -- run1.txt     # import a data file and plot it
cargo run --release -p sheaf-app -- lab.sheaf    # open a project
```

Files can also be dropped onto the window. Double-click a column header for its properties,
right-click it to plot or to set values from a formula, double-click a graph to change its axes,
plots and legend.

Command line (no window):

```
sheaf info run1.txt                              # what is in the file
sheaf plot run1.txt -o cv.png --dpi 300          # a cyclic voltammetry file becomes a CV overlay
sheaf plot run1.txt run2.txt -o overlay.pdf      # several files in one figure
sheaf plot data.csv -o fig.svg --template line-symbol --y B,C
sheaf import run1.txt run2.txt -o lab.sheaf --plot
sheaf export lab.sheaf --graph Graph1 -o fig.png
sheaf analyze integrate it.txt                   # charge of a current-vs-time file
sheaf analyze integrate cv.txt --rows 700:900 --baseline   # peak area above a sloping background
sheaf analyze smooth data.csv --y B --points 11 --export smoothed.csv
sheaf analyze stats lab.sheaf --book Book2
```

`sheaf analyze --help` lists the tools: integrate, differentiate, smooth, normalize, baseline,
interpolate, peaks, stats, sort, filter. In the app the same tools are in the Analysis menu; they
work on the selected columns and each run is one undo step.

`sheaf` is `cargo run --release -p sheaf-cli --`, or `target/release/sheaf.exe` after a build.

## Layout

| Path | What it is |
|---|---|
| `crates/core` | Document model: project, workbooks, columns, graphs, figure templates, undo |
| `crates/expr` | Expression language for column formulas and fit functions |
| `crates/calc` | Analysis numerics on plain number slices, checked against Origin |
| `crates/fit` | Curve fitting (in progress) |
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
