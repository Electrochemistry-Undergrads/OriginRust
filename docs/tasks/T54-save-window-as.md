# T54: Save Window As (single-window files) and Recent Books / Recent Graphs

**Status:** done, merged 2026-11-15 (Claude macOS "mac-savewin"; spec `docs/spec/window-files.md`): `Project::extract_window`/`append_windows`, `sheaf_io::window_file` (`.sheaf` zip with a window marker, `.sheafbook`/`.sheafgraph`, a window file never opens as a project), Save Window As... in the workbook and graph File menus, open-as-append routing, Recent Books and Recent Graphs. File > Append now also rewrites `[OldName]` references of renamed books. Reviewed by three agents with each finding checked; seven fixed. Left: the 8 Origin measurements in window-files.md; keeping analysis links wholly inside the window; Save Template As.
**Size:** M. **Branch:** `save-window`. **Model:** Sonnet. No Origin needed (Sheaf's own file kinds); Origin's
dialog details are measured once on Windows.
**Spec:** a new section in `docs/spec/workbook-model.md` or the project-file spec (find where `.sheaf` is
specified) and `docs/spec/recent-files.md`.

## Goal

File > Save Window As... writes the active window to a file of its own: a workbook as `<name>.sheafbook`, a
graph as `<name>.sheafgraph` together with the workbooks its plots read (so it opens with its data).
Opening such a file (File > Open, drag and drop, command line, Recent Books / Recent Graphs) adds its windows
to the current project, as File > Append does, renaming on name clashes; it never replaces the project.
File > Recent Books and File > Recent Graphs (grey since T38) list these files.

## Origin facts

- Not measured. From Origin's public documentation (paraphrased): Save Window As saves the active window as
  a window file (`.ogw`/`.ogwu` for a workbook, `.ogg`/`.oggu` for a graph); opening one adds the window to
  the open project; Recent Books and Recent Graphs list those files. Origin's window files are Origin's own
  binary format: Sheaf neither reads nor writes them (clean-room; `.opj` is the only Origin format Sheaf
  writes). Measure on Windows: whether a graph file carries its data or only links to books by name, the
  dialog's default folder and name, and how name clashes are resolved on open. Until then Sheaf's choices
  above, labelled.
- Save Template As (style without data) is a separate feature (T18 "Templates"); keep it grey.

## Where

- `crates/io`: the window files are the `.sheaf` zip format with a marker in the manifest (like the `.opj`
  companion marker) and their own extensions; `save_window(project, window, path)` builds a minimal project
  (the window, plus source books for a graph, folders flattened) and writes it atomically; `is_window_file`;
  loading reuses `project_file::load`.
- `crates/core`: if Append has a project-merging function (find File > Append / `AppendProject`), reuse it
  for opening a window file; renaming rules on clashes as Append does. Formula and analysis links that
  point outside the saved window are dropped with a note (as Append drops recalculation links), never left
  dangling.
- `crates/app`: enable Save Window As... (workbook and graph File menus; keep Save Template As grey); the
  save chooser filtered to the right extension; open routing for the two extensions (Open, drop, command
  line); Recent Books / Recent Graphs submenus via `sheaf_io::recent` (add `books`/`graphs` lists with the
  same rules; keep reading older `recent.json` files).
- CLI (optional, if cheap): `sheaf export --window NAME -o x.sheafgraph`.

## Steps

1. Spec text (Sheaf's choices labelled; Origin measurements listed).
2. `save_window` and loading with tests (book round trip; graph with two source books round trip; links
   outside the window dropped with a note; corrupt file; a window file is never mistaken for a project).
3. App: menu items, open routing as append, Recent Books / Recent Graphs; UI tests: save a graph window,
   start a new project, open the file: the graph and its books appear and plot the same data; a clash
   renames; recent lists fill and reopen; Save Template As stays grey.
4. Parity rows (Save Window As to ◐; Recent row note), T38 Status.

## Done when

Save Window As writes a workbook or graph window file that opens into another project with its data; Recent
Books and Recent Graphs work; fmt, clippy and tests pass.

## Don't

- Don't read or write Origin's window file formats.
- Don't replace the open project when a window file is opened.
- Don't enable Save Template As.
