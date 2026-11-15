# ADR 0011: MCP server (`sheaf-mcp`) on a shared operations crate

Status: accepted (2026-11-14). Task T25, phase 1.

## Context

An AI assistant that speaks the Model Context Protocol (MCP) should be able to open or import data,
plot, analyze, fit, export and save with the same numbers as the command line and the desktop app.
Three facts from the code decide how:

- The work of the CLI sits in the binary crate `sheaf-cli` (`commands.rs`, `analyze.rs`, `fit.rs`),
  mixed with clap structs and `println!`. Nothing there can be imported. Stdout is the protocol
  channel of a stdio MCP server, so any `println!` in reused code would corrupt it.
- The CLI keeps a process-wide list of Origin files it read (`FOREIGN_INPUTS`) so that saving over
  one makes a backup. A long-lived server cannot use a global like that.
- `sheaf_core::analysis::Tool` (about 70 variants) and the fit request types have no serde derives.
  `schemars` and `rmcp` are not in `Cargo.lock` or the offline registry cache. `tokio` in the lock
  is a featureless stub. Only `serde` and `serde_json` are available without network access.

## Decision

1. **Transport.** stdio only in phase 1. The server reads one JSON-RPC 2.0 message per line from
   stdin and writes one per line to stdout. Logs go to stderr. Streamable HTTP is later work.
2. **Hand-written loop, not `rmcp`.** About 300 lines over `serde_json`, synchronous, no `tokio`,
   no macros, no new dependency. The server handles one request at a time, which matches the
   work (CPU-bound, one project).
3. **One project per server process.** The server owns one `Project` (and the path it was opened
   from, whether that was an Origin file, and the file's backup state). `open_project` and
   `new_project` replace it. Handles are names (`Book1`, sheet name, column short name, graph
   name), never ids, because ids are not stable between opens.
4. **Shared code path: a new library crate `crates/ops` (package `sheaf-ops`).** It holds the work
   the CLI does today and returns data instead of printing. `sheaf-cli` and `sheaf-mcp` both call
   it. `sheaf-ops` depends on `sheaf-core`, `sheaf-io`, `sheaf-plot`, `sheaf-fit`, `sheaf-calc`,
   `serde`, `serde_json`, `thiserror`/`anyhow`, and `clap` (only for the option definitions the CLI
   and the server share, see the amendment below); never on egui or `sheaf-app`.
   - Reports are structs (`serde::Serialize`) with `warnings: Vec<String>` and notes in them.
     The CLI prints them with its present wording, so its output does not change.
   - The global `FOREIGN_INPUTS` becomes a parameter: `load` returns `foreign`, and `save` takes
     `backup_if_foreign` from the caller.
   - Option strings (`--panels 2x2`, `--stack auto:10`, `--image-width 3.5in`) are parsed in
     `sheaf-ops`; the MCP tools take typed JSON fields and call the same typed functions below
     the parser, so error text names parameters, not flags (see the spec, section 8).
   - Clap rules (`conflicts_with`, `requires`) are re-checked in `sheaf-ops` request validation.
5. **Analysis input as JSON without serde on `Tool`.** `sheaf-ops::analysis_spec` holds a registry:
   for each tool a name, a summary, and typed option descriptors (name, kind, default, help). One
   builder turns `{tool, options}` into a `sheaf_core::analysis::Tool`, resolving column names to
   ids. The CLI's `job_from` is rewritten to collect its flags into the same option map and call
   the builder, so flags and JSON cannot drift. The registry also produces the JSON Schema text
   (`list_analysis_tools`, `tools/list`). Tools are moved into the registry in steps; the tool
   `list_analysis_tools` reports exactly the ones present. Fits use the same pattern for
   `FitRequest` (`fit_spec`).
6. **Hand-written schemas.** Input schemas are `serde_json::json!` values in `crates/mcp/src/schemas.rs`,
   each with an example call that a unit test checks against the schema and the argument parser.
7. **Roots.** `--root DIR` (repeatable) lists the folders the server may touch. Every path is made
   absolute and canonical (the nearest existing parent for files not yet written) and must lie
   under a root. This includes wildcard matches, the `.opj.sheaf` companion and the `.bak` copy.
   Without `--root` no folder is allowed: tools that read or write files refuse and say to start the
   server with `--root`. Tools that touch no file (analysis, plots, fits on data in the project) work.
8. **Images.** `sheaf_plot::export_bytes` renders in memory, so `export_graph` returns PNG as an
   MCP image content block and also writes the file when a path is given. No base64 crate is
   needed; a 40-line encoder lives in `sheaf-mcp`.

## Alternatives considered

- **`rmcp` (official Rust SDK) with `schemars`.** Gives schemas by derive and handles protocol
  details. Rejected for phase 1: it needs `tokio` with runtime features, `schemars` and proc
  macros, none available offline; and `schemars` would require serde derives on the whole `Tool`
  tree anyway. Revisit if HTTP transport is added; the tool bodies are plain functions in
  `sheaf-ops`, so only the thin protocol layer would change.
- **Serde derives on every analysis and fit type.** Hundreds of types, `NaN` values that JSON
  cannot carry, and column ids inside requests. A name-based option registry is smaller and gives
  the CLI the same single path.
- **Put the shared code in `sheaf-core` (`sheaf_core::ops`).** Core is the document model and
  should not know file paths, image export or reports for people. A separate crate keeps the
  dependency direction clean (`ops` above `io` and `plot`).
- **Split `sheaf-cli` into lib and bin.** Would make the library carry clap types. The MCP crate
  would then depend on clap.
- **Call the `sheaf` binary as a subprocess.** Reloads the project per call, parses text output,
  and loses state. Rejected.
- **Several projects per server, addressed by id.** More state for the assistant to track, and
  nothing in the task needs it. A second project is opened by `open_project` after a `save_project`.

## Consequences

- The CLI is changed (a refactor with no visible change); its tests and sample outputs must stay
  identical and are the check that the move was faithful.
- `analyze` over MCP reaches every tool of `sheaf analyze`: 21 are mapped by hand in the registry,
  the rest come from the CLI's own clap definitions (see the amendment). `fit` covers what its
  tool lists; plot and fit also accept clap-derived options (see the second amendment).
- Phase 2 (driving the open desktop app) forwards the same `sheaf-ops` requests to the app, which
  runs them inside `Document::edit`. That needs its own ADR.

## Amendment (2026-11-14): the CLI's option definitions are shared

The hand-written registry reached 21 of the 55 analysis tools of `sheaf analyze`. Instead of
moving the other 34 into it one by one, the clap definitions (`args.rs`) and the translation of
parsed options into a job (`job_from` and its helpers) moved from `sheaf-cli` into `sheaf-ops`
(`sheaf_ops::args`, `sheaf_ops::cli_analyze`); `sheaf-ops` now depends on `clap`. Nothing that prints
moved. `sheaf-mcp` (`crates/mcp/src/bridge.rs`) turns a JSON object into an argv, parses it with the
CLI's clap types and calls `job_from`, and builds the JSON Schema of a tool's options from the same
definitions, so the CLI and the server cannot drift for those tools. The hand-mapped tools keep
their behaviour. Alternative rejected: a second registry for the 34 tools (the drift this ADR set
out to avoid).


## Amendment (2026-11-14): plot and fit options share the CLI bridge

`plot` and `fit` accept an `options` object whose schema and parsing come from their CLI commands.
The existing structured MCP arguments remain supported; overlapping settings are refused rather
than silently overridden. CLI-to-library specification conversions live in `sheaf-ops` and are
called by both the CLI and MCP. File loading, project saving and independent exports stay in their
existing tools. Path options still pass through the server's allowed-root checks. This work needs
no Origin reference: it extends Sheaf's own automation interface.
