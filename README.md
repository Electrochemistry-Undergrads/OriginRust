<p align="center">
  <img src="docs/brand/sheaf.png" width="128" height="128" alt="Sheaf bronze S logo">
</p>

<h1 align="center">Sheaf</h1>

<p align="center">
  <b>Scientific data analysis and graphing, Sheaf: an independent, open-source implementation of Origin and OriginPro workflows, built in Rust.</b><br>
  Workbooks, scientific plots, curve fitting, and data analysis,<br>
  in a native app written in Rust. Open source and offline.
</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/built_with-Rust-b7410e?style=flat-square&amp;logo=rust">
  <img alt="macOS · Windows" src="https://img.shields.io/badge/macOS%20%C2%B7%20Windows-native-2f7bf5?style=flat-square">
  <a href="#license-and-credits"><img alt="License: MIT OR Apache-2.0" src="https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-3a3a3a?style=flat-square"></a>
  <img alt="Early development" src="https://img.shields.io/badge/status-early_development-d69e2e?style=flat-square">
</p>

<p align="center">
  <a href="#features">Features</a> ·
  <a href="#command-line">Command line</a> ·
  <a href="#under-the-hood">Under the hood</a> ·
  <a href="#get-started">Get started</a> ·
  <a href="#documentation">Documentation</a> ·
  <a href="#license-and-credits">License and credits</a>
</p>

<br>

## Features

<table>
  <tr>
    <td width="50%" valign="top">
      <h3>Bring your data</h3>
      Import CH Instruments text and binary files, CSV, other delimited text and Excel workbooks (.xls, .xlsx, .xlsm) into workbooks.
    </td>
    <td width="50%" valign="top">
      <h3>Make your figures</h3>
      Overlay curves, adjust axes and legends, and export PNG, SVG, PDF, EPS, EMF, or TIFF.
    </td>
  </tr>
  <tr>
    <td width="50%" valign="top">
      <h3>Find the signal</h3>
      Integrate, smooth, find peaks, run statistics, and fit linear, polynomial, or nonlinear models.
    </td>
    <td width="50%" valign="top">
      <h3>Keep your workflow</h3>
      Work with Origin project files and familiar workbook and graph controls.
    </td>
  </tr>
</table>

> [!NOTE]
> Early development. Origin/OriginPro feature coverage and Origin `.opj` support are partial.
> Expect rough edges; Sheaf is not yet a complete replacement for Origin or OriginPro.
> See the [roadmap](docs/ROADMAP.md) for current progress.

## Command line

```sh
# Import data into a project
cargo run --release -p sheaf-cli -- import run1.txt -o experiment.sheaf --plot

# Export a figure
cargo run --release -p sheaf-cli -- plot run1.txt -o figure.png --dpi 300

# Analyze and fit
cargo run --release -p sheaf-cli -- analyze integrate run1.txt
cargo run --release -p sheaf-cli -- fit run1.txt --model linear

# Explore the available commands
cargo run --release -p sheaf-cli -- --help
```

Projects save as `.sheaf` or Origin `.opj`; see the
[project format documentation](docs/spec/project-file-format.md).

## Use Sheaf from Claude or other AI tools

`sheaf-mcp` is a Model Context Protocol server. An assistant can import data, read columns, plot,
analyze (every tool of `sheaf analyze`), fit, export figures and save projects, and offers three
prompts (plot CV files, integrate a peak, fit a curve). It works on a project held by the server and sees
only the folders you give with `--root`; with no `--root` every file path is refused.

```sh
cargo build --release -p sheaf-mcp

# Claude Code
claude mcp add sheaf -- /path/to/target/release/sheaf-mcp --root /path/to/my/data
```

For Claude Desktop, add this to `claude_desktop_config.json` and restart the app:

```json
{
  "mcpServers": {
    "sheaf": {
      "command": "/path/to/target/release/sheaf-mcp",
      "args": ["--root", "/path/to/my/data"]
    }
  }
}
```

The tools and their limits are listed in [docs/spec/mcp.md](docs/spec/mcp.md).

## Under the hood

- Eight Rust crates separate the workbook model, expression language, analysis, fitting,
  file formats, figure rendering, CLI, and desktop UI.
- The desktop interface uses egui/eframe with an OpenGL renderer.
- The desktop app and CLI share the same data, analysis, and rendering crates.
- Origin compatibility is implemented from observed behavior and documented formats;
  current gaps are tracked in the [feature parity table](docs/spec/feature-parity.md).

## Get started

Requires Git and [Rust](https://www.rust-lang.org/tools/install) 1.90 or newer.

### Windows executable icon

Windows builds embed Sheaf's existing multi-size icon in `sheaf-app.exe`, so Explorer and
shortcuts can display it. Install the Windows C++ build tools and Windows SDK (including `rc.exe`).
Build with `cargo build --release -p sheaf-app`; verify the actual executable with
`python tools/check-windows-icon.py target/release/sheaf-app.exe`. Details are in
[Windows packaging](docs/spec/windows-packaging.md).

### First time: download, build, and run

Run these commands in the folder where you want to keep Sheaf. The first launch downloads
dependencies and builds the app, so it can take a few minutes.

```sh
git clone https://github.com/Electrochemistry-Undergrads/OriginRust.git
cd OriginRust
cargo run --release -p sheaf-app
```

### Run again: use your existing copy

Open a terminal in the `OriginRust` folder you downloaded earlier, then run:

```sh
cargo run --release -p sheaf-app
```

Cargo reuses the existing build when the code and dependencies have not changed.

### Add Sheaf to Applications (macOS)

To start Sheaf like any other app, without a terminal, build its app bundle once from the
`OriginRust` folder:

```sh
tools/make-macos-app.sh --install
```

This puts **Sheaf.app**, with its icon, in your `Applications` folder (the one in your home
folder). Find it with Spotlight (Cmd+Space, type "Sheaf"), in Launchpad, or in Finder, and drag it
to the Dock to keep it there. Run the command again after `git pull` to update the app. Open files
from inside Sheaf (File > Open, or drag a file onto the window); double-clicking a data file in
Finder does not open it in Sheaf yet.

### Open a data file

Drop a data file onto the window, or run this from your existing `OriginRust` folder:

```sh
cargo run --release -p sheaf-app -- run1.txt
```

### Optional: run the tests

From the `OriginRust` folder:

```sh
cargo test --workspace --locked
```

New contributors: start with [`HANDOFF.md`](HANDOFF.md), then [`docs/`](docs/).

## Documentation

[Roadmap](docs/ROADMAP.md) ·
[Specifications](docs/spec/) ·
[Design decisions](docs/adr/)

## License and credits

Sheaf is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
Copyright (c) 2026 Sheaf contributors. Project notices are in [NOTICE](NOTICE).

Third-party dependencies and their bundled fonts retain their own licenses and notices; see
[ATTRIBUTION.md](ATTRIBUTION.md). The Sheaf logo was created by the project owner.

<sub>Origin and OriginPro are products of OriginLab; these names describe compatible workflows and file formats.
Sheaf is an independent project and is not affiliated with, sponsored by, or endorsed by OriginLab.</sub>
