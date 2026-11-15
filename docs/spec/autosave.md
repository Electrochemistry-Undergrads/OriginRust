# Autosave and recovery (T28)

## 1. Purpose

An unsaved or modified project is written every few minutes to a recovery file that is not the
user's own file. After a crash (or a power cut, or a kill) the next start offers to open what was
recovered. A normal save never reads or depends on the recovery file, and a clean exit removes it.
The mechanism is in `crates/io/src/recovery.rs` (files, locks, settings) and, for the timer and the
dialogs, in `crates/app`.

## 2. Origin facts (not measured)

Nothing here was measured: the session that wrote this had no Origin. From Origin's public
documentation, paraphrased:

- Tools > Options > Open/Close has a switch for autosaving the project with an interval in minutes,
  and a separate switch that backs the project up before saving.
- After a crash Origin offers the autosaved project on the next start.

To measure later on Windows with `tools/origin-ref`: whether autosave is on by default, the default
interval, the folder and file name of the autosaved project, and what the start-up prompt offers.
Put a screenshot of the Options tab in `re-workspace/scratch/t28/`. Until then every number below is
Sheaf's own choice.

## 3. Sheaf's choices (labelled as such)

| Choice | Value |
|---|---|
| Autosave | on |
| Interval | 10 minutes, settable from 1 to 120 (a value outside is brought into the range) |
| Age limit | recovery files older than 30 days are dropped at start-up |
| Format | always the native `.sheaf` project, never `.opj`: a recovery must store everything, including what the `.opj` writer cannot (`project-file-format.md`, ADR 0008) |
| Place | a folder of Sheaf's own, never beside the user's project and never the user's file itself |

### 3.1 Folders (from environment variables only)

| System | Recovery folder | Settings file |
|---|---|---|
| Windows | `%LOCALAPPDATA%\Sheaf\Recovery` | `%APPDATA%\Sheaf\autosave.json` |
| macOS | `$HOME/Library/Application Support/Sheaf/Recovery` | `$HOME/Library/Application Support/Sheaf/autosave.json` |
| Other Unix | `$XDG_DATA_HOME/sheaf/recovery`, else `$HOME/.local/share/sheaf/recovery` | `$XDG_CONFIG_HOME/sheaf/autosave.json`, else `$HOME/.config/sheaf/autosave.json` |

An XDG variable counts only when it is set to an absolute path. When the variables are missing there
is no folder, and the app runs without recovery. The command line of the app has `--recovery-dir DIR`
to use another folder and `--no-recovery` for none (tests and scripts).

The settings file is `{"enabled": true, "minutes": 10}`. A missing or damaged file gives the
defaults; a missing key keeps its default.

### 3.2 Files of one running instance

Each running app instance has a session id `<pid>-<unix seconds>-<n>` (`n` counts the sessions of
one process, so two sessions in one process never share an id). It owns, in the recovery folder:

| File | Content |
|---|---|
| `<id>.lock` | An empty file that the instance holds under an exclusive operating system file lock for as long as it runs. |
| `<id>.sheaf` | The project as of the last autosave, written to a temporary file (`.<name>.<pid>.tmp`) and renamed, so a crash never leaves half a project. |
| `<id>.json` | The sidecar, written after the project file. |

The sidecar has `original` (the path the project was opened from or last saved to, or `null` for a
project never saved), `saved_at` (seconds since 1970), `sheaf_version` (the version that wrote it)
and `pid` (for the user's information only). A missing or damaged sidecar does not hide a recovery:
it is listed without those facts (the dialog then says "Untitled" and uses the file's time).

### 3.3 Why a lock, not a process id

Process ids are reused, so "is process 4711 running" can be answered yes for a program that is not
Sheaf, and a recovery would then never be offered. The operating system releases a file lock when
its process ends, however it ends, and never confuses two processes. A recovery belongs to a gone
instance when its lock file is missing or can be locked. A lock that cannot be opened or tested
counts as held: files of a possibly live instance are never listed, discarded or pruned. A live
instance's own files are never offered to itself.

### 3.4 What touches which file

- `Session::save` writes the project and sidecar. `Session::clear` removes them (the project has
  become clean) and the session goes on. `Session::finish` (clean exit) removes the project, the
  sidecar and the lock.
- Dropping a `Session` without `finish` only releases the lock and keeps the files, so a panic, which
  unwinds, leaves the recovery behind as a crash does.
- Only regular files in the recovery folder are listed, deleted or opened. A link is never followed
  and never written through; a folder or a name that is not exactly `<id>.sheaf` with an id of three runs of digits
  (`<pid>-<unix seconds>-<n>`) is ignored, so a user's own `thesis.sheaf` or `config.json` in a
  mis-set recovery folder is never listed, offered or deleted; so are temporary files that do not
  have the shape `.<id>.sheaf.<pid>.tmp` or `.<id>.json.<pid>.tmp`.
- `prune(dir, max_age, now)` discards the recoveries of gone instances whose project file is older
  than `max_age`, and removes leftovers older than that (a lock or sidecar without a project file,
  a temporary file). The age is the project file's modification time. Files of live instances are
  never pruned.

## 4. Start-up dialog (built by the app part)

At start-up, when the app has a recovery folder: prune (30 days), list the orphans, start the
instance's own session (if that fails, say so in the Messages log and run without autosave). If
orphans exist, a modal dialog names the newest one (the original's file name, or "Untitled", and the
time it was saved) and says how many there are when there are several. Buttons:

- **Recover** opens the recovered project as a new, modified document that has no file name, so
  Save asks for one (the original's name and folder are suggested). If the current document is
  modified, the usual question about unsaved changes comes first. The recovered project is written
  to this instance's own recovery first, and only then is the orphan discarded, so the work is never
  held by nothing.
- **Discard** removes the newest recovery; if there are more, the next one is offered. After
  **Recover** the others are kept and offered at the next start (the Messages log says how many).
- **Later** keeps all of them; they are offered again at the next start.

With several orphans the buttons act on the newest. The dialog waits while the question about
unsaved changes is open. A recovery that cannot be read says so in the
Messages log and stays in the folder.

## 5. Timer (built by the app part)

While a session exists, autosave is on, the document is modified and its revision differs from the
last autosaved revision, the project is written when the interval has passed since the later of the
last autosave and the moment the project first became modified. The write works on a snapshot (a
clone with the current window places), takes no undo step and does not change the modified flag, and
runs on a worker thread, one write at a time, so a large project does not freeze the window. An
error is written once to the Messages log, not on every tick. The app asks for a repaint when the
interval will have passed, so the timer fires while the app is idle.

When the document becomes clean (a normal save, New, Open; undoing back to the saved state does not count, because the project
stays marked modified after an undo) the
recovery files are cleared once any write in flight has finished (the window does not wait for it:
the clear is retried each frame, and after a failed removal too). A clean exit joins the worker and
calls `finish`; quitting after choosing to discard the changes is a clean exit too. If the program
is ended while changes are unsaved and nobody was asked (for example by the system), the changes are
written one last time and the session is dropped without `finish`, so the next start offers them.

The interval and the switch are in Tools > Autosave... (OK applies at once and writes the settings
file). The command line has `--show autosave` to open that window for a screenshot. A recovered
project that came from an Origin `.opj` file keeps its `.bak` copy when it is saved over that file.

## 6. Tests that guard it

`crates/io/src/recovery/tests.rs`:

- the folders for each system and the XDG rules; settings defaults, round trip, missing, damaged and
  partial files, and the clamp to 1 to 120 minutes;
- project and sidecar round trip (the loaded project equals the saved one); never `.opj`; a later
  save replaces the earlier one; saving from a worker thread;
- two live sessions in one folder do not see each other, and cannot be discarded or pruned; an
  abandoned session is listed; a dropped session and a panic keep their files;
- a project file with no lock file is listed; a damaged project file is listed, fails to load
  cleanly and can be discarded; a damaged or missing sidecar still lists the recovery; newest first;
  temporary files, unrelated files (also ones named like a project or a sidecar, which prune keeps),
  folders and links are ignored (links are never followed);
- discard, clear and finish remove exactly the right files; prune drops old orphans and keeps young
  and live ones, and removes old leftovers.

`crates/app/src/app/ui_tests/autosave.rs` (app part): a modified project produces a recovery file
after the interval with an injected clock and not before; a clean exit removes it; a normal save
clears it; a planted orphan shows the dialog at start-up; Recover loads its data, leaves the
document modified and discards the orphan only after the instance's own copy exists; Discard removes
it; Later keeps it; autosave off writes nothing; tests never use the user's real folders.
