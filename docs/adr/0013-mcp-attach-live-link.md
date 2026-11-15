# ADR 0013: MCP attach mode, a live link from `sheaf-mcp` to the open desktop app

Status: accepted (2026-11-15, macOS and Windows). Windows live-app checks and effective-access evaluations for two non-owner accounts exposed inherited token-file access; the runtime permissions repair and validation are recorded in the Windows amendment below. Task T25, phase 2.
Builds on ADR 0011.

## Context

Phase 1 gives an assistant its own headless project. The owner then has to save a file and open it in
the app to see the result. Phase 2 lets the assistant act on the project that is open in the desktop
app. The owner sees every change as it happens and can undo it, within the undo limit.

Facts from the code (checked again against `main` on 2026-11-15):

- The tools take `&mut Server` and reach `s.project`, `s.roots`, `s.path`, `s.foreign` and
  `s.suggested` (`crates/mcp/src/tools.rs`, 2051 lines, 16 tools in `schemas::defs()`). `resources::list`
  and `resources::read` take `&Server` too. Nothing separates the session from the project, so the app
  cannot lend its project.
- `Document::edit` (`crates/core/src/history.rs`) clones the project, restores it on `Err`, runs
  `recalculate_since` and `update_analysis_links` on `Ok`, and pushes one undo step only if the project
  changed. It is not panic safe. `Document::replace` swaps the project, clears the history and does no
  recalculation. The history limit is a private field fixed at 100; there is no setter.
- The headless server never calls `recalculate_since` or `update_analysis_links`. Only `Document` does.
  If the app ran the tools inside `Document::edit` and the headless server did not, the same call could
  give different numbers in a recalculating column.
- `fit` commits inside `Server::transact`, then builds the plot window and writes the plot file outside
  it. A failed plot write leaves a half-done fit in the project.
- `export_graph` (also the multi-page PDF), `export_data` and the plot path of `fit` write with
  `std::fs::write` or a writer that takes a path (`write_xlsx`, `write_graph`). They replace an existing
  file. `sheaf_io::save::write_atomically` is `pub(crate)`.
- An omitted sheet means the workbook's `active_sheet`, which is view state inside the project: the
  owner changes it by clicking a sheet tab (`Document::adjust_view`, no undo step). An omitted book means
  the first workbook in project order (`sheaf_ops::pick::pick_book`), not the active window.
- The results of `set_column_values`, `export_data`, `plot`, `sort_sheet` and `filter_sheet` do not name
  the book (and mostly not the sheet) they used.
- `sync_windows` already opens every new window it finds in the project. A window the assistant adds
  needs no extra code if the call runs before `sync_windows`.
- The UI thread owns the `Document`. Dialogs hold ids and indices of windows and columns, and some
  preview live through `apply_merging`. A cell being edited holds the cell's old text
  (`BookUi.editing`, `editing_label`).
- The app (binary `sheaf-app`) depends on `sheaf-core`, `sheaf-io`, `sheaf-plot` and `sheaf-calc`. It does
  not depend on `sheaf-ops` or `sheaf-mcp`. `sheaf-mcp` has no egui dependency and depends only on
  `sheaf-core`, `sheaf-io`, `sheaf-plot` and `sheaf-ops`, so the layering "app depends on the library"
  holds as it stands.
- `UnixStream::peer_cred` is still unstable (rust issue 42839), and `getpeereid` needs `libc` or
  `unsafe`. `File::try_lock` and `try_lock_shared` are stable in `rust-version` 1.90. `getrandom` 0.3.4
  is already in `Cargo.lock`.
- The Windows build and the named-pipe API cannot be run on the macOS development machine.

## Decision

### 1. One executor for both modes

A new module `crates/mcp/src/exec.rs` holds `Session { path, foreign, suggested }`, a `Policy`
(`attached`, the roots of the caller, the protected set of decision 8) and
`run_on_document(doc, session, policy, name, args, structured) -> Outcome`. Tool functions take
`&mut Ctx` instead of `&mut Server` (a mechanical change, its own commit).

- The headless `Server` runs on a `Document` too, with its history limit set to 0 (a new
  `Document::set_limit`), so it keeps no steps. Both modes call the same function, so recalculation and
  analysis-link updates match the app and there is no second copy of the logic.
- A tool with `mutates` runs inside `Document::edit`. The closure catches panics and turns them into an
  error, so a panic or an `Err` rolls the project (and the `Session`) back and leaves no step.
  `mutates` means exactly "goes through `edit`": `import_files`, `set_column_values`, `plot`, `analyze`,
  `sort_sheet`, `filter_sheet`, `fit`.
- `new_project` and `open_project` go through `Document::replace`, never through `edit`, so opening a
  project does not recalculate it. They are headless only.
- Any other tool runs on a clone of the project that is thrown away (the columns are shared, ADR 0004).
- `fit` becomes atomic in both modes: its plot window and plot file are inside the same call. This is a
  phase-1 behaviour change. A phase-1 test that asserted the old non-atomic result changes; the design
  does not bend to it.
- `Server::transact` is removed.

### 2. Two flags on every tool definition

`schemas::Def` gains `mutates` and `headless_only` next to `read_only`. `read_only` keeps its meaning
(the annotation). `export_graph` and `export_data` are `read_only: false` and `mutates: false`: they
write files, not the project. `headless_only` is true for `new_project`, `open_project` and
`save_project`.

### 3. Attach mode refuses `new_project`, `open_project` and `save_project`

`Document::replace` clears the undo history and would drop the owner's unsaved work, and `save_project`
would bypass the app's own path, dirty flag and Origin backup logic. The refusal sits in the executor when
the policy is attached, so the app cannot forget it. The proxy also hides the three tools from
`tools/list`. The owner saves the file. Autosave and recovery already cover assistant edits, because
they go through `Document`.

### 4. Undo step names and one sanitizer

The label is `"AI: " + tool + " " + target`, built from the project before the call (`Document::edit`
takes it up front): the book, the columns as `B(Y)` (designation read from the project, at most 3, then
`+N`), the model or analysis tool, the formula for `set_column_values`, the first file plus `+N` for
`import_files`. Examples: `AI: plot Book1 B(Y)`, `AI: fit Gauss Book1 B(Y)`.

Every string that came over the wire and is shown in the interface goes through one function,
`sheaf_mcp::link::sanitize(text, max_chars)`: it removes control characters, bidi controls and the other
format characters (Unicode Cc and Cf, plus U+2028 and U+2029), turns runs of white space into one space,
and cuts at a character boundary. It is used for the label (60 characters), the client name (40), the
roots in the tooltip, the file names and error text in the status bar and the Messages Log. These are
the strings the link layer shows. Text that a tool stores in the project (a column long name or units from
`set_column_values`, a formula, the names of imported sheets and files) is not cleaned: it is project data
and reaches the grid headers and explorers as it came, like text the owner typed or a file the owner
imported.

### 5. Transport: a local endpoint, built only from the standard library

- **Unix (macOS, Linux): a Unix domain socket** (`std::os::unix::net`) in a per-user run folder. The
  folder is created with `DirBuilder` at mode 0700 for every missing level. If it exists it must be a
  real directory (not a link) owned by the current user (the owner of a probe file the process just
  created in it); if it is too permissive the process sets it to 0700 and checks again; a folder of
  another owner, or a link, is refused. The socket file is set to 0600. The peer is not asked for its user
  id, because `peer_cred` is unstable and `getpeereid` needs `unsafe`; the 0700 folder is the access
  control, and this is stated in the spec.
- **Windows: loopback TCP** on `127.0.0.1`, an ephemeral port, and a 256-bit random token (64 hex digits
  from `getrandom` 0.3, added as a direct dependency of `sheaf-mcp` for all targets so the code is built
  and tested everywhere). The token travels in the hello and is compared in constant time. There is no
  named-pipe code and no `unsafe`.
- **Run folder candidates**, in order: `SHEAF_MCP_RUN_DIR`; on Linux `$XDG_RUNTIME_DIR/sheaf` when that
  is set and absolute; the data-directory fallback (`~/Library/Application Support/Sheaf/run` on macOS,
  `<XDG data>/sheaf/run` on Linux, `%LOCALAPPDATA%\Sheaf\run` on Windows), from a new
  `sheaf_io::recovery::run_dir_candidates()`. There is no `/tmp` fallback. The app uses the first
  candidate that passes the folder checks. The client probes every candidate, because an assistant
  started by another program may have another environment than the app.
- **Files in the run folder**: `mcp.lock` (an exclusive `File::try_lock` held while the app listens, as
  `sheaf_io::recovery::Session` does), `mcp.json` (atomically written after the endpoint is bound:
  `{v, pid, app, endpoint}`, with the token on Windows; mode 0600) and `mcp.sock` on Unix.
- **One endpoint per user.** A second app instance cannot take the lock, shows a warning and its menu
  item stays unticked. The process that takes the lock removes an old `mcp.sock` and `mcp.json` first.
- **Client discovery.** For each candidate folder: apply the same checks as the app, open `mcp.lock`
  read-only and `try_lock_shared`. If the lock can be taken, no app listens there. If it would block,
  read `mcp.json` and connect. When nothing listens, the error names every folder that was looked in and
  why each one was skipped.

### 6. Protocol: JSON lines, hello first

The first line must be `{"hello":{"v":1,"client":"..","roots":[..],"token":".."}}` within 10 seconds (the
clock runs whatever the peer sends, so a peer that sends a byte now and then is cut off too; the first line
is at most 64 KiB, and a hello names at most 16 different roots). Anything else, including HTTP text, closes the
connection. Then four operations: `call`, `cancel`, `resources_list`, `resource_read`. A reply is
`{id, result}`, where `result` is exactly what `ToolOut::into_result` gives, or `{id, error:{code,
message}}`. One call may be in flight per connection. The app may also send a `notice` line, used for
"the project in Sheaf was replaced" and "the owner turned the link off". `initialize`, `ping`,
`tools/list`, `prompts/*`, `resources/templates/list` and notifications are answered by the proxy.

### 7. Roots are client-chosen and sent as absolute paths

The `sheaf-mcp` proxy canonicalises its `--root` values with the same code as phase 1 (`Roots::new`) and
sends only absolute paths. The app rejects a hello that holds a non-absolute root, then builds a
per-connection `Roots` with `Roots::new` (so a root that does not exist is an error). No roots means the
phase-1 refusal "no folder is allowed". The owner's consent is the menu tick, not the roots: the client
chooses them. The client's name and its granted roots are shown in the tooltip of the status-bar chip and
written to the Messages Log when it connects. Confinement is voluntary against any process of the same
user that can reach the endpoint (see "What this protects against").

### 8. Writes in attach mode

- `export_data`, `export_graph` (also the multi-page PDF) and the plot path of `fit` never replace an
  existing file: they refuse with a clear message. A destination is checked with `symlink_metadata`,
  so a link (even a dangling one) and any non-regular file count as existing.
- A write goes to a new temporary file in the target folder (`create_new`, so no link is followed; the
  name keeps the extension because the writers choose the format from it), then is published without
  replacing: `hard_link` to the final name (it fails if the name exists) and the temporary file is
  removed. Where the file system has no hard links the write falls back to a second check and `rename`,
  which leaves a small race. The write lives in `sheaf-mcp`'s `files` module and follows the pattern of
  `sheaf_io::save::write_atomically`, which stays private to `sheaf-io`.
- The **protected set** is computed per call from the live document and refuses a new file with any of
  these names: the open project's path, its `.sheaf` companion and `.bak`, the Origin file it was read
  from (with its companion and `.bak`), the original path of a recovered project, the temporary names of
  those files, any name with a project extension (`.opj`, `.opju`, `.sheaf`, `.bak`, `.lock`), and
  anything inside the recovery folder or the run folder. Names are compared by canonical parent plus
  file name, case-insensitively on macOS and Windows. This matters even though nothing is replaced: a
  planted `.bak` would stop the app's Origin backup, and a planted file in the recovery folder would be
  offered back as a recovery.
- Headless writes are unchanged (they replace, as the CLI does).

### 9. The proxy

`sheaf-mcp --attach` runs a reader thread on stdin and a reader thread on the link, feeding one event
queue, so `ping` and `notifications/cancelled` are handled while a call waits. A `notifications/cancelled` for a request that waits behind the call in flight, or that has not arrived
yet, is remembered and the request is dropped instead of sent. At the end of stdin every line already read
is still answered, the waiting call included. On `notifications/
cancelled` for the waiting call the proxy sends a `cancel` operation, and the app moves the job QUEUED to CANCELLED with a
compare-and-swap (if the job has started, nothing is cancelled). The proxy waits at most 50 seconds for
a reply. Then it sends `cancel`, waits two seconds, and answers with `still running in Sheaf; check
list_contents before retrying` (or, when the app confirms the job never started, `the owner is busy or
has a dialog open in Sheaf; the call did not run; try again`). It closes that connection and uses a new
one for the next call, and it ignores any reply that comes late. It never retries a mutating call by
itself.

### 10. Lifecycle

Unticking Tools > AI Assistants (MCP), or exit, sets a closing flag, wakes the accept loop with a
connection to itself, replies to every queued job with "the owner turned the link off" (QUEUED to
CANCELLED), shuts down every connection, joins the threads with a bounded wait (the UI thread never
hangs; a thread that does not end is left to end by itself), removes `mcp.sock` and `mcp.json` and drops
the lock. The toggle runs on the UI thread, so no job is running at that moment. Off then on works. When the
owner replaces the project (New, Open, Recover; counted by a new `Document::generation`, which
`replace` bumps) the app sends the notice "the project in Sheaf was replaced" to every connection and
closes it, so the assistant is told once to look again before it acts on names from the old project.

### 11. The UI thread

Connection threads do socket work and queueing only. They never touch the project. Each frame the UI
thread pops the whole job queue into a local pending list. Non-mutating jobs run at once (they are
never stuck behind deferred ones). A mutating job waits while a modal dialog is open or a cell is being
edited. At most one mutating job runs per frame, in `live_step`, after the actions loop and before
`sync_windows`. A failure is a status-bar warning and a Messages Log line, not a message box. The
status bar shows a boxed `AI: connected (n)` with a tooltip that lists the clients and their roots.

### 12. Undo

The undo limit stays at 100 steps. Project changes the assistant makes are undoable within that limit.
Files an assistant writes on disk are not undone. The spec does not claim that everything can be undone.
Each connection may make at most 1000 mutating calls; after that it is refused with a message, which
bounds how fast an assistant can push the owner's own steps out of the history.

### 13. Results name the book and sheet

The results of `set_column_values`, `export_data`, `plot`, `sort_sheet` and `filter_sheet` gain `book`
and `sheet` fields naming what was used (additive, in both modes). `filter_sheet` keeps `sheet` as the
sheet that holds the rows now (as in phase 1) and adds `source_sheet`, the sheet that was filtered. An omitted book or sheet resolves
exactly as headless does. The dependence on view state (the active sheet follows the owner's tab) is a
documented difference, not hidden: the assistant is told to name the sheet, and the result says which
one was used.

### 14. Dependency direction

The app depends on `sheaf-mcp` as a library for the executor and the link. `sheaf-mcp` never depends on
the app or egui. The shared pieces live in modules of the `sheaf-mcp` library: `exec`, `link` (frame
reader, hello and operations, sanitizer, discovery, the transports, the host that accepts connections and
queues jobs, the client) and `proxy`. The binary `main.rs` stays thin. The host hands the app plain
data: a job (client, roots, name, arguments, a reply sender); the app code is `crates/app/src/live.rs`.
A `sheaf-mcp` entry is added to the root `[workspace.dependencies]`.

## What this protects against

- **Other users on the computer.** On Unix the 0700 folder and the 0600 socket keep them out. On Windows
  the endpoint is a loopback port that any local user can reach; the 256-bit token, compared in constant
  time, is the connection barrier. The Windows amendment below restricts its runtime folder/file
  permissions to the process user, SYSTEM and administrators before publishing it.
- **A web page.** A browser can send HTTP to a loopback port. The first line of a connection must be the
  hello with the token, so an HTTP request line closes the connection, and a page cannot know the token.
- **A stale or planted socket.** The lock decides who listens; the folder checks refuse a folder that is
  a link or belongs to someone else.
- **An assistant that misbehaves** (or is led astray by text in a data file): it can only call the tools
  of the spec, which run no command or script, stay under the roots, never replace a file, cannot touch
  the project's own files, and make undo steps the owner can see.

Not protected: **any process of the same user that can read the run folder** can connect, say what roots it
likes and drive the app. Confinement by roots is voluntary on the client's side. This is the
confused-deputy case: the app lends the owner's authority (the open project, the owner's files under the
roots the caller names) to whoever speaks the protocol, and only a well-behaved proxy sends the roots
the owner configured. The owner's consent is the menu tick. A process that already runs as the user can
read the user's files anyway; what it gains here is the open project and its undo list. The spec says so
plainly.

## Alternatives considered

Three full designs were drawn up and compared. This one took the structure of the smallest and added
parts of the other two. The design critique then changed the transport.

- **Minimal design: swap the app's project into a persistent `Server` inside `Document::edit`.** Least
  code, but the headless server and the app would run different code (`edit` recalculates, the server
  did not), `fit` would stay non-atomic, and a panic inside the swap would leave the project in the
  wrong place. Kept: forwarding at the tool level and one `Document::edit` per call. Dropped: the swap.
- **Security-first design: TCP on 127.0.0.1 with a per-run token.** One code path, no `unsafe`, testable on
  any machine. Rejected on Unix, where a socket in a 0700 folder gives the OS-enforced user check that TCP
  cannot. Adopted on Windows, with the token in the hello (not in a header, so there is no HTTP
  parsing) and the hello-first rule that closes HTTP.
- **A named pipe on Windows with a protected DACL.** Not chosen. Its security rests on details that
  cannot be verified from the macOS development machine: `FILE_FLAG_FIRST_PIPE_INSTANCE`,
  `PIPE_REJECT_REMOTE_CLIENTS`, the DACL string and the security-QoS flags on the client. A wrong DACL is
  a silent hole, the code needs `unsafe` and a new `windows-sys` feature set, and its failure modes (pipe
  busy, instance limits) show only on a real Windows machine. Loopback TCP with a token is plain
  standard-library code that runs, and is tested, on every platform; its weakness (any local user can
  reach the port, the token is the barrier) is known and stated.
- **UX-first design: label and deferral rules.** Kept: the label built from the project (designations,
  not raw argument text), deferral while a cell is being edited, and deferral of mutating jobs while a
  dialog is open. Not kept: extra owner prompts per connection (the menu toggle is the consent), and an
  "Undo AI changes" command (follow-up).
- **AF_UNIX sockets on Windows.** `std::os::unix::net` is `cfg(unix)` only, so it needs hand-written
  Winsock code. Rejected.
- **A pipe or socket name per app instance, with `--attach=ID`.** One endpoint per user is simpler and
  a wrong-project write is worse than a refusal. Follow-up if two instances are really needed.
- **Allowing `new_project`, `open_project` and `save_project` in attach mode.** The first two drop the
  undo history; `save_project` would bypass the app's own path, dirty flag and Origin backup logic.
  Rejected. The owner owns the file.
- **Letting attach writes replace files, as headless does.** Rejected: the assistant is not the one who
  owns the folder, and a replace cannot be undone.
- **Reads on a snapshot worker thread.** Keeps the UI smooth during a long export. More threading for
  a rare case. Follow-up.
- **A new crate for the executor and the link.** `sheaf-mcp` already holds the tool code, and a new crate
  would split `tools.rs` from the schemas it shares with it.

## Consequences

- The tool functions change signature (`&mut Ctx`), and `resources` take a project instead of a server.
  The first commit is a no-behaviour refactor, and every phase-1 test must stay green.
- Headless results change in three places: columns with formulas follow `set_column_values` and other
  edits (the headless server now recalculates), `fit` is atomic, and five tools add `book` and `sheet`
  to their results. A headless test pins the formula column, and another pins that opening a project
  with a formula column leaves its data unchanged.
- `Document` gains `set_limit` and `generation`, and `Document::edit` restores the project when the change
  or the recalculation after it panics. `sheaf-io` gains `run_dir_candidates()`; the no-replace write is in
  `sheaf-mcp`. `sheaf-mcp` gains `getrandom` as a direct dependency (an edge in
  `Cargo.lock`, no new package). No `unsafe` code is added.
- The app gains a dependency on `sheaf-mcp` (and through it `sheaf-ops`), so it compiles more code.
- A read sees the project as drawn, including the live preview of an open dialog; a mutating call waits
  for the dialog to close. A heavy tool (fit, a big import, a large render) blocks the UI for its run.
- An omitted sheet follows the owner's sheet tab. The result names the sheet used.
- The app's undo limit is 100 steps and files written to disk are not undone.
- Only one app instance per user can listen.
- Windows acceptance found that profile inheritance can grant another local group token-file read
  access. The Windows amendment below replaces that assumption with owned runtime objects and
  protected DACLs, verified by native app/proxy and effective-access checks.
- Follow-ups, not in phase 2: a snapshot worker for reads, `--attach=ID` for several instances, and
  `peer_cred` when it is stable. App context and Undo AI Changes are described in amendments below.


## Amendment: read-only app context (2026-11-15)

The desktop host advertises and serves `sheaf://app`, describing its active window and worksheet,
cell/row/column/label or graph-object selection, Data Reader pin, and dialog/edit state (mcp.md 11.18).
The existing `resources_list` / `resource_read` operations are sufficient; link version 1 stays intact.
Headless hosts omit this resource and retain resource-not-found behavior for its URI.

The UI supplies a borrowed context reader to the queued job executor. It runs only for this URI,
on the UI thread after the job begins, with read-only access to the current `Document`. The app
reads its existing view state without adding selection changes or history operations. The result
contains bounded metadata and source-row indices; editor text and cell/label values are excluded.
A mid-frame worksheet change suppresses the old grid selection until the view redraws. No context
is persisted in the project, and healthy reads leave no undo step or document revision.

The proxy's initialization instructions point clients to the resource when listed. Clients still
name the workbook, sheet and columns in tool arguments. Context requests use the existing queue,
cancellation, connection lifecycle and error handling. The Windows endpoint acceptance checks
above and the separate snapshot-worker follow-up remain open.


## Amendment: owner Undo AI Changes (2026-11-15)

Provide an owner-only Edit > Undo AI Changes command beside ordinary Undo/Redo. The no-window bar
keeps it in the existing File Undo/Redo fallback. It uses the existing Document history to undo
consecutive completed entries whose labels start with the executor's `AI: ` prefix, starting at
the top. It stops at the first manual entry or the undo limit. This
preserves manual work and requires no history-schema or selective-undo mechanism. The command
is disabled while an editor or dialog is active and when the top entry is not an AI step; the
handler checks the same conditions and reuses the existing Undo action, preserving its cleanup
hooks. Ordinary Redo restores one completed call at a time.

The command changes only the in-memory document through ordinary undo. Files written by exports
stay on disk. It does not cancel queued calls, disconnect assistants or expose a new MCP tool;
the existing AI Assistants toggle is how the owner stops further calls. Real-proxy/UI tests cover
a whole AI batch, a manual-history boundary, editor/dialog guards, saved-state restoration and redo.
Windows endpoint acceptance is recorded in the Windows amendment below; the snapshot-worker follow-up remains separate.


## Amendment: Windows runtime permissions and acceptance (2026-11-15)

The baseline native app/proxy worked on Windows without a firewall prompt, but its discovery
file inherited a read grant for another local group. Windows Authz evaluations for two actual
non-owner local accounts, with their resolved groups, both granted file-data read access.
Relying on the profile's inherited ACL therefore did not satisfy the acceptance condition.

The app now verifies the runtime object's owner against its process token user and refuses
reparse points. It protects only the selected runtime folder, lock and discovery file with
allow entries for that user, SYSTEM and administrators. Missing parents may be created, but
existing parent/profile/account permissions are not rewritten. The temporary token file is
created with its protected descriptor atomically using CREATE_NEW, then rechecked before writing. A client verifies private
folder/file permissions and refuses a broad or foreign-owned descriptor without repairing it.
A filesystem that cannot establish these permissions cannot host this endpoint. The protocol,
random token, proxy, queues and UI/history behavior are unchanged.

The small Windows-only adapter uses the already-locked windows-sys 0.61 APIs. Native allocations
and handles have RAII ownership; unsafe calls document buffer/pointer lifetimes. The permission
operations follow [SetNamedSecurityInfoW](https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-setnamedsecurityinfow),
[CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)
and [SDDL descriptor conversion](https://learn.microsoft.com/en-us/windows/win32/api/sddl/nf-sddl-convertstringsecuritydescriptortosecuritydescriptorw).
This supersedes the earlier no-unsafe/inherited-rights implementation decision for Windows.

On the fixed native run, the same two accounts were denied FILE_READ_DATA and the owner was
allowed. This used Windows Authz effective-access evaluation, without another interactive login
or any account changes. The proxy listed tools and read app context, wrong-token/HTTP clients
were refused, the listener was loopback-only, and no firewall prompt appeared. Both native
processes exited cleanly and removed discovery; the captured app window was inspected.
Five Windows regressions plus the existing discovery/transport/UI tests cover the contract.
Administrators retain their normal machine-wide authority, as root does on Unix.
