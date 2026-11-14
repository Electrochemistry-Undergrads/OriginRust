# T25: MCP server so Claude and other AI tools can use Sheaf

**Status:** open (owner's request, 2026-11-14). **Size:** L. **Model:** Sonnet.

## Goal

An AI assistant (Claude Code, Claude Desktop, or any client that speaks the Model Context Protocol)
can drive Sheaf: open or import data, read columns, plot, run analyses and fits, export figures and
save projects, with the same results as the CLI and the desktop app. Phase 1 works headless on a
project held by the server; phase 2 lets the assistant act on the project open in the desktop app, so
the owner sees each change on screen and can undo it.

## Origin facts

None needed: this is Sheaf's own interface. (Origin's counterparts are its COM automation server,
LabTalk and the `originpro` Python package; do not copy their names or command set. Clean-room rule
as usual.)

## Where

- New crate `crates/mcp`, binary `sheaf-mcp` (workspace member; depends on `sheaf-core`, `sheaf-io`,
  `sheaf-plot`, `sheaf-fit`, `sheaf-calc`, never on `sheaf-app`/egui).
- What it reuses: `crates/cli/src/commands.rs`, `analyze.rs`, `fit.rs` (the CLI's import, plot,
  export, analyze and fit paths); `sheaf_core::analysis` (`Tool`, `Request`, `Report`, already
  `Serialize`/`Deserialize`); `sheaf_core::project::Project`; `sheaf-io` readers and the `.opj`/`.sheaf`
  writers; `sheaf-plot` renderers (PNG/SVG/PDF).
- Phase 2: `crates/app` (a local listener), `crates/core` undo stack.
- Docs: new ADR `docs/adr/0011-mcp-server.md`, new spec `docs/spec/mcp.md`, README section, ROADMAP line.

## Steps

### Phase 1: headless server (stdio)

1. **ADR 0011.** Record the choices: stdio transport first; the official Rust SDK `rmcp`
   (modelcontextprotocol/rust-sdk; pin an exact version at least two weeks old) or, if its tokio
   dependency is unwanted, a small hand-written JSON-RPC 2.0 loop over stdin/stdout with `serde_json`
   (MCP's stdio framing is one JSON message per line). One `Project` lives in the server process per
   session.
2. **One shared code path.** Move the bodies of the CLI's commands into a library module (for
   example `sheaf_core::ops` or a new `sheaf-api` crate) that both `sheaf-cli` and `sheaf-mcp` call,
   so a result can never differ between the two. The CLI keeps its flags; only the work moves.
3. **Tools** (names `snake_case`, inputs described by JSON Schema; `schemars` can derive the schemas
   from the request types):
   - `new_project`, `open_project` (`.opj`, `.opju`, `.sheaf`), `save_project` (`.opj` with the
     `.opj.sheaf` companion, as File > Save)
   - `import_files` (CH Instruments text and `.bin`, CSV, ASCII with the import options)
   - `list_contents` (folders, workbooks, sheets, columns with designations and labels, graphs)
   - `read_columns` (values for a row range; capped, see step 5), `set_column_values` (a formula,
     through `sheaf-expr`)
   - `plot` (template, columns, into a new or existing graph), `export_graph` (PNG/SVG/PDF/EPS to a
     path; PNG also returned as MCP image content so the assistant can look at the figure)
   - `list_analysis_tools`, `analyze` (a `Tool` and its options as JSON, returning the `Report` and
     where the results were written)
   - `list_fit_models`, `fit` (model, ranges, weights, returning parameters, errors and statistics)
4. **Resources:** the project tree, a column as CSV, a graph rendered as PNG. **Prompts** (optional):
   "plot these CV files", "integrate a peak", "fit this curve".
5. **Safety and size:** file access only under roots given on the command line
   (`sheaf-mcp --root ~/Data`), refused elsewhere; no shell or script execution; returned data capped
   (for example 10 000 cells per call, with the total row count and a hint to page); errors returned
   as tool errors with the same wording as the CLI. Nothing but protocol messages on stdout (logs go
   to stderr).
6. **Tests:** an integration test that starts `sheaf-mcp`, sends `initialize`, `tools/list` and a
   `tools/call` for import → plot → analyze → fit → export → save, and checks the replies against
   the CLI's output for the same files (synthetic data only, `gen_chi.js` or the fixtures).
7. **Hook-up docs:** README and `docs/spec/mcp.md` with the setup for Claude Code
   (`claude mcp add sheaf -- <path>/sheaf-mcp --root <data folder>`), Claude Desktop
   (`claude_desktop_config.json`) and any other stdio MCP client. `tools/make-macos-app.sh` can copy
   `sheaf-mcp` into `Sheaf.app/Contents/MacOS/` so one install gives both.

### Phase 2: drive the open desktop app

8. **Live link (ADR first).** The desktop app listens on a local socket (Unix socket on macOS and
   Linux, named pipe on Windows), off by default and turned on from a menu or a `--mcp` flag. When it
   is listening, `sheaf-mcp --attach` forwards tool calls to the app instead of its own project.
9. Each forwarded change goes through the app's undo stack as one step named after the tool
   ("AI: plot Book1 B(Y)"), and windows the assistant creates open as usual, so the owner sees and
   can undo everything. A status-bar note shows when an assistant is connected.
10. **Later:** HTTP (streamable) transport for remote clients; tools for graph styling
    (Plot Details settings, axes, legend) and worksheet operations once phase 1 is in use.

## Done when

- `sheaf-mcp` builds with the workspace; `cargo fmt`, strict Clippy and `cargo test --workspace` pass.
- From Claude Code with the server added, an assistant can import a CHI file, plot it, integrate a
  peak, fit it, export a PNG it can see and save an `.opj` that opens in Sheaf, with numbers equal to
  the CLI's.
- The tool list, inputs and limits are documented in `docs/spec/mcp.md`; ADR 0011 is written;
  ROADMAP and this Status are updated. Phase 2 is a separate merge.

## Don't

- Don't duplicate analysis, import or plotting logic in the MCP crate: call the shared path (step 2).
- Don't let `sheaf-mcp` depend on `sheaf-app`, egui or a window; phase 1 must run on a machine with
  no display.
- Don't expose arbitrary script or shell execution, or file access outside the given roots.
- Don't print anything but protocol messages to stdout; it corrupts the stdio stream.
- Don't return whole large columns in one reply; page them.
- Don't use the owner's real data files in committed tests (they are git-ignored inputs).
