<p align="center">
  <img src="docs/brand/sheaf.png" width="160" height="160" alt="Sheaf bronze S logo">
</p>

<p align="center">
  <b>Lab data. Clear figures.</b><br>
  A native Rust app for workbooks, scientific plots, and data analysis.
</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/built_with-Rust-b7410e?style=flat-square&amp;logo=rust">
  <a href="#license"><img alt="License: MIT OR Apache-2.0" src="https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-6b5b4b?style=flat-square"></a>
  <img alt="Early development" src="https://img.shields.io/badge/status-early_development-a66b3f?style=flat-square">
</p>

<p align="center">
  <a href="#features">Features</a> ·
  <a href="#get-started">Get started</a> ·
  <a href="#command-line">Command line</a> ·
  <a href="#documentation">Documentation</a> ·
  <a href="#license">License</a>
</p>

## Features

<table>
  <tr>
    <td width="50%" valign="top">
      <h3>Bring your data</h3>
      Import CH Instruments files and CSV into workbooks.
    </td>
    <td width="50%" valign="top">
      <h3>Make your figures</h3>
      Overlay curves, adjust axes and legends, and export PNG, SVG, or PDF.
    </td>
  </tr>
  <tr>
    <td width="50%" valign="top">
      <h3>Find the signal</h3>
      Integrate, smooth, find peaks, run statistics, and fit curves.
    </td>
    <td width="50%" valign="top">
      <h3>Keep your workflow</h3>
      Work with Origin project files and familiar workbook and graph controls.
    </td>
  </tr>
</table>

> [!NOTE]
> Early development. Expect rough edges; Origin `.opj` support is partial.
> See the [roadmap](docs/ROADMAP.md) for current progress.

## Get started

Requires [Rust](https://www.rust-lang.org/tools/install) 1.90 or newer.

```sh
git clone https://github.com/Electrochemistry-Undergrads/OriginRust.git
cd OriginRust
cargo run --release -p sheaf-app
```

Drop a data file onto the window, or open one directly:

```sh
cargo run --release -p sheaf-app -- run1.txt
```

## Command line

```sh
cargo run --release -p sheaf-cli -- plot run1.txt -o figure.png --dpi 300
cargo run --release -p sheaf-cli -- analyze integrate run1.txt
cargo run --release -p sheaf-cli -- --help
```

## Documentation

[Roadmap](docs/ROADMAP.md) ·
[Specifications](docs/spec/) ·
[Design decisions](docs/adr/)

Contributors: start with [HANDOFF.md](HANDOFF.md).
Run checks with `cargo test --workspace`.

## License

Sheaf is dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE),
at your option. Third-party dependencies retain their own licenses.

---

Sheaf is an independent project and is not affiliated with OriginLab.
