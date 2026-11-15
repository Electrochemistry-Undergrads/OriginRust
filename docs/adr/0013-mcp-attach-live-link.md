# ADR 0013: MCP attach mode, a live link from `sheaf-mcp` to the open desktop app

Status: proposed (2026-11-15). Task T25, phase 2. Builds on ADR 0011.

## Context

Phase 1 gives an assistant its own headless project. The owner then has to save a file and open it in
the app to see the result. Phase 2 lets the assistant act on the project that is open in the desktop
app. The owner sees every change as it happens and can undo it.

Facts from the code that decide the design:

- The tools take `&mut Server` and reach `s.project`, `s.roots`, `s.path`, `s.foreign` and
  `s.suggested`. Nothing separates the session from the project, so the app cannot lend its project.
- `Document::edit` (`crates/core/src/history.rs`) clones the project, restores it on `Err`, runs
  `recalculate_since` and `update_analysis_links` on `Ok`, and pushes one undo step only if the project
  changed. It is not panic safe: a panic leaves the project half changed.
- The headless server never calls `recalculate_since` or `update_analysis_links`. Only `Document`
  does. If the app ran the tools inside `Document::edit` and the headless server did not, the same call
  could give different numbers in a recalculating column.
- `fit` commits inside `Server::transact`, then builds the plot window and writes the plot file outside
  it. A failed plot write leaves a half-done fit in the project.
- `sync_windows` already opens every new window it finds in the project. A window the assistant adds
  needs no extra code if the call runs before `sync_windows`.
- The UI thread owns the `Document`. Dialogs hold ids and indices of windows and columns. A cell
  being edited holds the cell's old text.
- The app depends on `sheaf-core`, `sheaf-io`, `sheaf-plot` and `sheaf-calc`. It does not depend on
  `sheaf-ops` or `sheaf-mcp` today. `sheaf-mcp` has no egui dependency.
- The workspace has no `unsafe_code` ban. `windows-sys` 0.61 is already an app dependency on Windows.

## Decision

1. **One executor for both modes.** A new module `crates/mcp/src/exec.rs` holds
   `Session { path, foreign, suggested }`, `Ctx { project, roots, session, attached }` and
   `run_on_document(doc, roots, session, attached, name, args, structured) -> Outcome`. Tool functions
   take `&mut Ctx` instead of `&mut Server` (a mechanical change, its own commit).
   - A tool with `mutates` runs inside `Document::edit(label, ..)`. The closure catches panics and
     turns them into an error, so a panic rolls back and leaves no step. An `Err` rolls back and leaves
     no step.
   - Any other tool runs on a clone of the project that is thrown away.
   - The headless `Server` now owns a `Document` too (history limit 0, so it keeps no steps) and calls
     the same function. Headless therefore gains recalculation and analysis-link updates, and `fit`
     becomes atomic in both modes. Parity holds by construction: there is no second copy of the logic.
   - `Server::transact` stays but is redundant.
2. **A `mutates` flag on every tool definition** (`schemas::Def`), next to `read_only`. `read_only`
   keeps its meaning (annotations). `mutates` is true for `new_project`, `open_project`,
   `import_files`, `set_column_values`, `plot`, `analyze`, `sort_sheet`, `filter_sheet` and `fit`.
3. **Attach mode refuses `new_project`, `open_project` and `save_project`.** `Document::replace` clears
   the undo history and would drop the owner's unsaved work, so neither is an undo step. The refusal
   sits inside `tools::call` when `ctx.attached`, so the app cannot forget it. The proxy also hides the
   three tools from `tools/list`. The owner saves the file. Autosave and recovery already cover
   assistant edits, because they go through `Document`.
4. **Undo step name.** `"AI: " + tool`, then the book, then the columns as `B(Y)` (designation read
   from the project, at most 3, then `+N`), then the model or tool, the formula for `set_column_values`
   and the first file plus `+N` for `import_files`. Control and bidi characters are stripped,
   whitespace collapsed, the result capped at 60 characters, with `"AI: <tool>"` as the fallback.
   Examples: `AI: plot Book1 B(Y)`, `AI: fit Gauss Book1 B(Y)`. The label is computed from the
   project before the call, because `Document::edit` takes it up front.
5. **Transport: an endpoint the OS restricts to the current user. No TCP, no token.**
   - Unix (macOS, Linux): `std::os::unix::net::UnixListener` in a private run folder (mode 0700), the
     socket set to 0600. The folder must be a real directory (not a link), must have no group or other
     bits, and must be owned by the same user as a probe file we just created.
   - Windows: a byte-mode named pipe `\\.\pipe\sheaf-mcp-<32 hex>`, created with
     `FILE_FLAG_FIRST_PIPE_INSTANCE` and `PIPE_REJECT_REMOTE_CLIENTS`, with a protected DACL that
     grants access to the user's SID only. At most 8 instances, and one pending instance is always
     kept. The client opens it with `security_qos_flags(SECURITY_IDENTIFICATION)`.
   - The run folder is `sheaf_io::recovery::run_dir()`: `$XDG_RUNTIME_DIR/sheaf` on Linux when it is
     set and absolute, else `<data>/sheaf/run`; `~/Library/Application Support/Sheaf/run` on macOS;
     `%LOCALAPPDATA%\Sheaf\run` on Windows. There is no `/tmp` fallback. A socket path over 100 bytes
     is refused with a message that names `SHEAF_MCP_RUN_DIR`.
   - Three files: `mcp.lock` (an exclusive `File::try_lock` held while listening), `mcp.json` (written
     atomically after bind: `{v, pid, app, endpoint}`) and `mcp.sock` on Unix.
   - One endpoint per user. A second app instance gets "busy", shows a warning in the status bar, and
     its toggle stays off. Whoever gets the lock removes old `mcp.sock` and `mcp.json` and binds.
   - Discovery by the client: read `mcp.json`, open `mcp.lock` read-only and `try_lock_shared`. If the
     lock can be taken, no app is listening: fail with "no Sheaf is listening". If it would block,
     connect.
   - The Windows API calls go in `crates/mcp/src/attach/windows.rs`, on a `cfg(windows)` dependency of
     `sheaf-mcp` on `windows-sys` 0.61 with the features Pipes, Storage_FileSystem, Security,
     Security_Authorization, System_Threading and Foundation. This adds one lock-file edge and no new
     package. Every `unsafe` block has a `SAFETY` comment.
6. **Protocol: JSON lines, hello first.** The first line must be
   `{"hello":{"v":1,"client":"..","roots":[..]}}` within 10 seconds. Anything else, including HTTP
   text, closes the connection. Then three ops: `call`, `resources_list`, `resource_read`. The reply is
   `{id, result}` where `result` is exactly what `ToolOut::into_result` gives, or
   `{id, error:{code,message}}`. The proxy passes `result` through unchanged. `initialize`, `ping`,
   `tools/list`, `prompts/*`, `resources/templates/list` and notifications are answered by the proxy
   without the app.
7. **Roots are sent by the client in the hello and enforced in the app.** The app builds a per-connection
   `Roots` with `Roots::new` (so a bad root is an error). No roots means the phase-1 refusal "no folder
   is allowed". Paths are checked in the process that touches the file. A hostile process of the same
   user can already open the user's files, so roots confine the assistant's tools, not an attacker.
8. **Protected writes.** The app passes `Roots::protect(paths)`: the current project path, its
   `.sheaf` companion, its `.bak` and the Origin file it was read from. `export_graph`, `export_data`
   and the `plot` path of `fit` refuse them with "that is the project open in Sheaf". Headless passes an
   empty list. Phase 1 is unchanged.
9. **Threading.** A connection thread parses a line and sends a `Job` to the UI thread over a bounded
   channel (16, full means "busy"), then calls `request_repaint`. The UI thread runs one job per frame
   in `remote_step`, after the actions loop and before `sync_windows`.
   - A job has an atomic state: QUEUED, RUNNING, CANCELLED. The connection thread waits 30 seconds
     for the UI to pick the job up. If it then wins the compare-and-swap QUEUED to CANCELLED, it
     replies "busy" and the job never runs. If the UI won, the job is RUNNING and the thread waits up to
     10 minutes more. A reply that arrives after "busy" can never be followed by a late change.
   - While a dialog is open (`dialog_open()`), every op except the pure tools `list_analysis_tools` and
     `list_fit_models` waits. While a cell is being edited, mutating tools also wait. Waiting jobs
     time out as "busy".
   - Heavy tools (fit, big import, large render) block the UI for a frame. This is accepted for
     phase 2.
10. **Owner interface.** A Tools menu item "AI Assistants (MCP)" with a tick, and the flag `--mcp`.
    Off by default and never remembered. The status bar shows a boxed "AI: connected" (with the count
    when more than one). Each change is logged in the status bar and the Messages Log. The Edit menu
    shows "Undo AI: ...". A failure is a status-bar warning, not a message box.
11. **Parity scope.** `pick_book(None)` still means "the first workbook", also in the app, not the
    active window. `Session.suggested` (the template an imported book suggests) is per connection and
    starts empty. Both are documented.
12. **Dependency direction.** The app may depend on the `sheaf-mcp` library. `sheaf-mcp` never depends
    on the app. A `sheaf-mcp` entry is added to the root `[workspace.dependencies]`.

## Alternatives considered

Three full designs were drawn up and compared. This one took the structure of the smallest and added
parts of the other two.

- **Minimal design: swap the app's project into a persistent `Server` inside `Document::edit`.** Least
  code, but the headless server and the app would run different code (`edit` recalculates, the server
  does not), `fit` would stay non-atomic, and a panic inside the swap would leave the project in the
  wrong place. Kept: forwarding at the tool level and one `Document::edit` per call. Dropped: the
  swap trick.
- **Security-first design: TCP on 127.0.0.1 with a per-run token in a 0600 file.** One code path and no
  `unsafe`, testable on any machine. Rejected for the transport because every local user can reach the
  port, so the token is the only barrier; a web page can POST to loopback and a line-based server would
  run the body line, so a strict hello rule is needed; and it needs a CSPRNG on every platform.
  Kept from it: the shared `Document` executor, a compare-and-swap cancel state, protected paths,
  lock-checked discovery, and a queue timeout that is separate from the run timeout. TCP stays the
  fallback if the Windows pipe code proves too costly to maintain.
- **UX-first design: label and deferral rules.** Kept: the label built from the project (designations,
  not raw argument text), deferral while a cell is being edited, and deferral of reads while a dialog
  is open. Not kept: extra owner prompts per connection (the menu toggle is the consent), and an
  "Undo AI changes" command (follow-up).
- **AF_UNIX sockets on Windows.** `std::os::unix::net` is `cfg(unix)` only, so it needs hand-written
  Winsock code. Rejected.
- **A named pipe with the default DACL.** Everyone gets read access, which allows a denial of service
  and an odd state. An explicit protected DACL is used.
- **A pipe or socket name per app instance, with `--attach=ID`.** One endpoint per user is simpler and
  a wrong-project write is worse than a refusal. Follow-up if two instances are really needed.
- **Allowing `new_project`, `open_project` and `save_project` in attach mode.** The first two drop the
  undo history; `save_project` would bypass the app's own path, dirty flag and Origin backup logic.
  Rejected. The owner owns the file.
- **Reads on a snapshot worker thread.** Keeps the UI smooth during a long export. More threading for
  a rare case. Follow-up.
- **A new crate for the executor.** `sheaf-mcp` already holds the tool code, and a new crate would
  split `tools.rs` from the schemas it shares with it.

## Consequences

- The tool functions change signature (`&mut Ctx`). The first commit is a no-behaviour refactor, and
  every phase-1 test must stay green. If a test relied on the non-atomic `fit`, the test changes, not
  the design.
- Headless results change in one place: columns with formulas now follow `set_column_values` and
  other edits, because the headless server also recalculates. This is a fix. A headless test pins it.
- The app gains a dependency on `sheaf-mcp` (and through it `sheaf-ops`), so it compiles more code.
  No new package enters `Cargo.lock`.
- The app's undo limit is 100 steps. A long assistant session can push the owner's own steps out. The
  limit is not raised while assistants are connected, for now.
- A heavy tool blocks one frame. A dialog or a cell edit makes the assistant wait, then fail as
  "busy" after 30 seconds. The assistant is told to try again.
- Only one app instance per user can listen.
- The Windows pipe code (DACL, `FIRST_PIPE_INSTANCE`, pipe-busy retry) cannot be run on the macOS
  development machine. It must be verified on Windows, including with a second user account, before
  this ADR is accepted.
- Follow-ups, not in phase 2: "Undo AI Changes", a `sheaf://app` resource (active window, selection),
  a snapshot worker for reads, and `--attach=ID` for several instances.
