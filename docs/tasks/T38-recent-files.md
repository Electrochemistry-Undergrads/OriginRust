# T38: Recent projects and recent imports

**Status:** done, merged 2026-11-15 (Claude macOS "mac-recent"; spec `docs/spec/recent-files.md`): `sheaf_io::recent` (locked re-read-and-merge writes, canonical case-insensitive de-duplication, non-UTF-8 paths kept), File > Recent Projects and Import > Recent Imports. Reviewed by three agents with each finding checked; six fixed. Origin 2016's list length (15), numbering (1-9, A-F) and path display (80 characters, extension left out) measured and copied 2026-11-15 (recent-files.md 2). Left: missing-file behaviour and the other unmeasured points; Recent Books/Graphs/Exports wait for single-window files (T54 has since made Recent Books and Recent Graphs work; Recent Exports works since T78). Brief written from parity
row "Recent projects, books, graphs, imports" (P1, ✗) and the greyed File menu items.
**Size:** M. **Branch:** `recent-files`. **Model:** Sonnet. No Origin needed for the mechanism; the list
length and menu wording are measured once on Windows.
**Detail:** audit A (File menu greyed items); `docs/spec/ui-command-map.md` lines on Recent Imports,
Recent Exports, Recent Books, Recent Graphs, Recent Projects.
**Spec:** new section in `docs/spec/workspace.md` if it exists, else a new `docs/spec/recent-files.md`.

## Goal

File > Recent Projects lists the projects the owner opened or saved most recently, newest first, and
clicking one opens it through the normal Open path (unsaved-changes question included). File > Import >
Recent Imports lists the data files imported most recently and imports one again with the default
settings of its type. The lists survive a restart and are shared by every running instance.

## Origin facts

- Not measured. From Origin's public documentation (paraphrased): the File menu has Recent Books, Recent
  Graphs and Recent Projects submenus, and Import and Export each have a Recent submenu; the lists hold
  the most recently used files, newest first. Measure on Windows with `tools/origin-ref`: how many entries
  each list keeps, whether entries are numbered (`&1 path`), how long paths are shortened, what happens on
  a missing file (removed? message?), whether opening a project from the list moves it to the top.
  Screenshots to `re-workspace/scratch/t38/`.
- Recent Books and Recent Graphs list window files (`.ogw`, `.ogg`, `.otp`...) saved on their own. Sheaf
  cannot save a single window (parity row "Save Window As"), so those two stay grey; so does Recent
  Exports (repeating an export needs saved export settings).

## Where

- `crates/io`: a small `recent` module next to `recovery` (reuse `recovery::default_settings_path`'s folder
  logic): `RecentList { projects, imports }` stored as `recent.json` in the settings folder, written
  atomically; `push_project(path)`, `push_import(paths)`, `remove(path)`, load that tolerates a missing or
  corrupt file; entries de-duplicated by canonical path (case-insensitive on macOS and Windows), newest
  first, capped (Sheaf's choice until measured: 10 projects, 10 imports). Two instances: re-read the file
  before each change and merge, so one instance does not wipe the other's entries.
- `crates/app/src/menus.rs`: the greyed "Recent Projects" and "Recent Imports" become submenus filled
  from the list (numbered `&1 name` with the folder shown, the full path as a tooltip; empty list shows a
  greyed "(none)"); "Clear Recent Projects"/"Clear Recent Imports" at the bottom. Recent Books, Recent
  Graphs and Recent Exports stay grey.
- `crates/app/src/app.rs`: push on a successful open/save/save-as of a project and on a successful import
  (menu, drag-and-drop, command line); open through `open_paths`/`load_project` so the unsaved-changes
  question and `.opj` companion rules apply; a missing file shows a message and is dropped from the list.
- `Startup`: a `recent_path: Option<PathBuf>` like `settings_path` (T28): `main.rs` fills it, UI tests pass
  `None` (no list) or a temp file, never the real folder.

## Steps

1. Spec section (Sheaf's choices labelled; Origin measurements listed for Windows).
2. `sheaf_io::recent` with tests in a temp folder (order, de-duplication, cap, corrupt file, two writers
   merging, canonical and case-insensitive paths, non-UTF-8 path round trip on Unix).
3. Menus and app wiring; UI tests: an opened project appears first in Recent Projects; clicking it reopens
   it (asking about unsaved changes first); a missing file is reported and removed; an imported CSV appears
   in Recent Imports and clicking it imports again; Clear empties the list; tests never touch the real
   settings folder.
4. Parity row to ◐ (Books/Graphs/Exports left), ui-command-map note, HANDOFF.

## Done when

Opening or saving a project and importing a file fill the lists, the lists survive a restart, clicking an
entry opens or imports it, fmt, clippy and tests pass.

## Don't

- Don't store the lists inside the project file.
- Don't follow an entry that is not a regular file (a folder or a link to one); report it instead.
- Don't block the UI on a slow network path when building the menu: show the stored text, check
  existence only when an entry is clicked.
