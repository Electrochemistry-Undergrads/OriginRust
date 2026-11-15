# Recent projects and recent imports (T38)

No `workspace.md` spec exists, so this is a spec of its own. The mechanism is in
`crates/io/src/recent.rs` (lists, file, merging); the menus and the clicks are built by the app part
(`crates/app`).

## 1. Purpose

File > Recent Projects lists the projects the owner opened or saved most recently, newest first, and
clicking one opens it through the normal Open path (the question about unsaved changes included).
File > Import > Recent Imports lists the data files imported most recently and imports one again with
the default settings of its type. The lists survive a restart and are shared by every running instance.

## 2. Origin facts (not measured)

Nothing here was measured: the session that wrote this had no Origin. From Origin's public
documentation, paraphrased: the File menu has Recent Books, Recent Graphs and Recent Projects
submenus, and Import and Export each have a Recent submenu; the lists hold the most recently used
files, newest first.

To measure later on Windows with `tools/origin-ref` (screenshots to `re-workspace/scratch/t38/`):

1. How many entries each list keeps (projects, books, graphs, imports, exports), and whether the
   number can be set (Tools > Options).
2. Whether entries are numbered (`&1 path`, with the accelerator), and the text of an entry: file
   name only, full path, or the path shortened with an ellipsis, and the length where it is shortened.
3. Whether the list is shared between two running Origin instances, and when it is written (at once,
   or when the program ends).
4. What happens when an entry's file is missing: removed silently, a message, or kept.
5. Whether opening a project from the list moves it to the top; whether Save As adds the new name,
   and whether a project opened from the command line or by double click is added.
6. Whether an import adds one entry per file for a multi-file import, in which order, and whether the
   entry repeats the import with the settings last used for that type or with the defaults.
7. Whether the same file in two spellings (case, `..`, a link) counts as one entry.
8. The text of an empty list (greyed "(none)" or nothing) and whether a "Clear" item exists.

Until then every number and wording below is Sheaf's own choice.

## 3. Sheaf's choices (labelled as such)

| Choice | Value |
|---|---|
| Length | 10 entries in Recent Projects, 10 in Recent Imports (separate lists) |
| Order | newest first; using an entry again moves it to the top, never repeats it |
| Entry text | numbered `&1 name` to `&9 name`, then `1&0 name` for the tenth (name = the file name), the folder after it in a weaker colour, the full path as the tooltip |
| Empty list | one greyed item "(none)" |
| Clear | an item "Clear Recent Projects" / "Clear Recent Imports" below a separator |
| Missing file | a message in the Messages log and a dialog naming the file; the entry is removed |
| Not a regular file | a folder, or a link to one, is reported and never opened or imported; the entry is removed |
| Books, graphs, exports | Recent Books, Recent Graphs and Recent Exports stay greyed: Sheaf cannot save a single window ("Save Window As", parity row) or repeat an export (no saved export settings) |
| Import again | with the default settings of the file's type, as if the file had been dropped on the window |
| What is listed | a project file opened or saved (Open, Save, Save As, command line, drag and drop, the list itself), and a data file imported (menu, drag and drop, command line); not recovered projects, not the autosave files, not templates |
| Batch import | the files are pushed one after the other in import order, so the last one imported is first |
| Long file name | a name longer than 60 characters loses its middle (`...`) and keeps its end, the extension; the tooltip shows the whole path (unmeasured) |
| Where | not in the project file |

## 4. The file

`recent.json` in the folder of the autosave settings (`recovery::default_settings_path`, section 3.1
of `autosave.md`): `%APPDATA%\Sheaf\recent.json` on Windows,
`$HOME/Library/Application Support/Sheaf/recent.json` on macOS, `$XDG_CONFIG_HOME/sheaf/recent.json`
or `$HOME/.config/sheaf/recent.json` elsewhere. Without those variables there is no file and the app
runs without lists. The app takes the path from `Startup.recent_path`, which `main.rs` fills with
`recent::default_path()`; tests pass `None` or a temporary file and never the real folder.

```json
{
  "projects": ["/Users/a/Data/thesis.sheaf", "/Users/a/Data/old.opj"],
  "imports": ["/Users/a/Data/run1.csv"]
}
```

Each entry is a string (the path as text). A path that is not valid text is written as
`{"bytes": [..]}` on Unix (the raw bytes) or `{"wide": [..]}` on Windows (the UTF-16 units), so every
path a system can name survives a restart; an entry in the other system's form is skipped.

Reading never fails. A missing, unreadable or damaged file gives empty lists; inside a good file a
damaged entry is skipped and the others are kept; entries that are not absolute paths and repeated
entries are dropped; each list is cut to 10; unknown keys are ignored. Reading touches no listed file
(they may be on a slow network share).

## 5. Rules

- **Canonical paths.** A path is stored absolute, with links and `..` resolved
  (`std::fs::canonicalize`); on Windows the `\\?\` prefix that this gives is removed (`\\?\C:\x`
  becomes `C:\x`, `\\?\UNC\s\x` becomes `\\s\x`). A file that does not exist keeps its name and gets
  the canonical form of its folder; with no folder either, only the absolute form.
- **One entry per file.** Two entries are the same file when their stored paths are equal, ignoring
  letter case on Windows and macOS (default file systems there are case-insensitive) and exactly on
  other systems. On a repeat the new spelling replaces the old and goes to the top.
- **Both lists** may hold the same file.
- **Merging two instances.** Every change (`recent::update` and the helpers over it) takes an
  exclusive lock on `recent.json.lock` next to the file, re-reads the file, applies the change, writes
  the file and releases the lock. So an entry another instance added since this one last read is
  kept. The wait for the lock is 2 seconds; after that the change goes on without it (a hung
  instance must not block the others). The file is written to a temporary file and renamed, so a
  crash never leaves half a file. The lock file stays in the folder and is empty.
- **Removing.** `remove` drops a file from both lists, whatever its spelling, also after the file has
  been deleted. The app calls it when a click finds the file missing or not a regular file.
- **Clearing** empties one list and leaves the other.
- **No existence check on build.** The menu shows the stored text. The file is checked only when its
  entry is clicked (a slow or gone network path must not freeze the menu).

## 6. App part (built in `crates/app`)

- **Where.** `crates/app/src/recent.rs` holds the lists in memory (`Recent`), `Startup.recent_path` names
  the file (`main.rs` fills it with `recent::default_path()`; the tests pass a temporary file or
  `None`, which keeps no lists and writes nothing), and `menus.rs` draws the entries.
- **Menus.** "Recent Projects" (File) and "Recent Imports" (File > Import) are submenus: the numbered
  entries (section 3), then a separator and "Clear Recent Projects" / "Clear Recent Imports". An empty
  list shows a greyed "(none)" and a greyed Clear. The folder after the name is cut from the left with
  `...` when longer than 48 characters and a name longer than 60 characters loses its middle (Sheaf's
  choices, not measured); the tooltip and the hint line
  show the whole path. A `&` in a file name is doubled so it is not an accelerator. Recent Books,
  Recent Graphs and Recent Exports stay greyed.
- **Reading.** The menus are drawn every frame, so they read the lists from memory and the file is
  not touched while drawing. The file is read at start and once each time the File menu opens (so a
  change by another instance shows), not again while it stays open.
- **Writing.** Every change goes through `recent::update` (lock, re-read, change, atomic write) and the
  result replaces the lists in memory, so another instance's entries are kept. A failed write is
  logged once in the Messages log (not the status line) until a write works again; the change still
  shows in this run.
- **What pushes.** `load_project` after a successful open (menu, drop, command line, the list),
  `save_to` after a successful Save or Save As (the final name, with `.opj` added when it was), and
  `finish_import` for every file that was imported, in import order (menu, import window, wizard,
  Excel window, drop, command line: all end there). A failed open, save or import pushes nothing. A
  recovered project is not pushed. A save the `.opj` writer refused (an Origin-unsafe name) writes only
  the companion `x.opj.sheaf`; that file is what is pushed, because `x.opj` was never written (when
  `x.opj` is written, it is the one pushed). An import is pushed under the path the file was read from
  (`Imported.path`, the raw `PathBuf`), not the display text in `workbook.source.path`, so a name that is
  not valid text on Unix still names the real file.
- **Clicking an entry.** The file is checked now: it must exist, be a regular file (a folder, or a
  link to one, is refused; a link to a file is followed), and be a project file (projects list) or not
  one (imports list). If not, the message window says why ("Could not open the project" / "Could not
  import the file", the path, the reason, "It was removed from the recent files."), the Messages log
  gets a line, and the entry is removed from both lists and the file. Otherwise a project goes
  through `open_paths`, as a dropped file does: the question about unsaved changes comes first, then
  `load_project` (`.opj` companion rules included), which moves it to the top. An import goes
  through `open_paths` without asking: default settings of its type, plotted as a dropped file is (an
  Excel workbook with its import defaults, not graphed).
- **Pictures.** `--show menu:File>Recent Projects` and `--show menu:File>Import>Recent Imports` hold
  the submenus open for a screenshot. To keep the real folder out of it, run with `HOME` set to a
  temporary folder that has `Library/Application Support/Sheaf/recent.json` (macOS).

## 7. Tests that guard it

`crates/io/src/recent/tests.rs`:

- order (newest first), repeat moves to top, cap of 10 for both lists, batch import order, a file in
  both lists;
- canonical form (`..`, `.`, a link, a relative path, a missing file in an existing folder), case
  folding of the key, the case behaviour of this system, the Windows verbatim prefix text;
- file round trip, missing file, damaged files (several shapes), a damaged entry among good ones,
  relative and repeated entries, a hand-edited long list, unknown keys, folder creation;
- two writers: a stale reader's change keeps the other's entries; eight threads writing at once lose
  none;
- remove (both lists, any spelling, after deletion, unknown file), clear one list;
- text with spaces and accents; paths that are not text round trip on Unix (and do not collide).

`crates/app/src/recent.rs` (unit tests): numbering and the accelerator on the tenth, `&` in names, folder
cutting, a long name losing its middle, the click checks (missing, folder, link to a folder, link to a file, wrong kind of file),
merging between two `Recent` values, a failing write reported once and still shown.

`crates/app/src/app/ui_tests/recent.rs` (the menus and the clicks, each test with its own recent file):
an opened project first in the list and in the file; Save and Save As; failed open or save not listed;
a click reopens a project, and asks about unsaved changes first (Cancel keeps everything, Don't save
opens); a missing file and a folder are reported and removed; an imported CSV listed and imported again
by a click; batch order; a failed import not listed; a missing import reported and removed; Clear on
each list; "(none)" and greyed Clear; Recent Books, Graphs and Exports grey; the file read when File
opens and not each frame; another instance's entries kept; lists survive a restart; no file means no
lists; ten entries at most and the tenth numbered 10; an `.opj` save the writer refused lists the
companion, not the missing `.opj`; on Unix an import with a name that is not valid text is listed
under its real name (skipped where the file system refuses such a name, as macOS does).
