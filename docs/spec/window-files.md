# Window files: Save Window As, `.sheafbook` and `.sheafgraph` (T54)

The mechanism is in `crates/io/src/window_file.rs` (files), `crates/core/src/extract.rs`
(`Project::extract_window`) and `Project::append_windows` in `crates/core/src/project.rs`; the lists of
Recent Books and Recent Graphs are in `crates/io/src/recent.rs` (`recent-files.md`). The menus and
clicks are built by the app part (`crates/app`).

## 1. Purpose

File > Save Window As... writes the active window to a file of its own: a workbook as
`<name>.sheafbook`, a graph as `<name>.sheafgraph` together with the workbooks its plots read, so the
graph opens with its data. Opening such a file (File > Open, drag and drop, command line, Recent Books,
Recent Graphs) adds its windows to the open project, as File > Append does, and renames them on name
clashes. It never replaces the project.

## 2. Origin facts

**Not measured** (no Origin on the machine that wrote this). From Origin's public documentation,
paraphrased: Save Window As saves the active window as a window file (`.ogw` or `.ogwu` for a workbook,
`.ogg` or `.oggu` for a graph); opening one adds the window to the open project; Recent Books and Recent
Graphs list those files. Origin's window files are Origin's own binary format. Sheaf neither reads nor
writes them (clean-room rule; `.opj` is the only Origin format Sheaf writes). Save Template As (style
without data) is a separate feature (T18) and stays greyed.

To measure later on Windows with `tools/origin-ref` (files and screenshots to `re-workspace/scratch/t54/`):

1. Whether a graph file carries its data or only links to the books by name (save a graph, delete or
   rename the book, open the file in a fresh project).
2. Whether a graph file also brings the books of plots that read other books through formulas.
3. The dialog: default folder, default file name (window short name or long name), file type list, and the
   extension added when none is typed; whether an existing file asks before replacing.
4. How a name clash is resolved on opening (renamed `Book2`, asks, replaces) and whether the renamed window
   keeps its long name.
5. Where the opened window goes: the active folder or the root; whether it is shown at once and in which
   place and size; whether it is a change that asks to save the project on closing.
6. What happens to formulas that read another book, and to analysis results that recalculate, in a saved
   window.
7. Whether Recent Books and Recent Graphs list files that were saved, opened, or both; their length and
   order (T38 measured Recent Books for projects: same form as Recent Projects).
8. Whether a window file opened twice makes two windows.

Until then every choice below is Sheaf's own.

## 3. Sheaf's choices (labelled as such; none measured)

| Choice | Value |
|---|---|
| Format | the `.sheaf` ZIP archive (`project-file-format.md`) of a small project, with the manifest key `"window": "workbook"` or `"graph"` |
| Extensions | `.sheafbook` (workbook), `.sheafgraph` (graph and its source books); matched in any letter case |
| What a workbook file holds | the workbook, all its sheets, columns, labels, cell formats, notes and conditional formats |
| What a graph file holds | the graph and each workbook that holds a column the graph reads (plot X, Y, errors, labels, size and colour columns, offsets, tick-label columns; whatever `Graph::remap_columns` visits), each once, whole (not only the plotted columns) |
| Other workbooks | a workbook the graph does not read is not saved |
| Ids | window and column ids are kept as in the project; opening shifts them past the open project's ids |
| Folders | flattened: the file has no folders and no window places; windows that were in folders are saved without them |
| Project settings | page setup, mask colour, hide masked and masking switch are not saved (the columns keep their own mask state) |
| Formulas that read a book outside the file | `[Book2]Sheet1!col(A)` where `Book2` is not one of the saved workbooks (by short or long name): the formula, its Recalculate mode, row range and state are removed from the column; its values stay; a note is made per column |
| Formulas that read a book inside the file | kept (a graph file holds all the source books, so formulas between them stay) |
| Analysis results that recalculate | no link is kept (as File > Append keeps none): the lock marks on the result columns are cleared, the values stay, one note says how many links touched the saved columns. Links that lie wholly inside the saved window could be kept; they are not, because a link's tool options hold column ids of several kinds |
| Opening | the windows are added with `Project::append_windows`: ids moved past the open project's, names that are taken replaced by free `Book<n>` / `Graph<n>` names (smallest free number), column ids fresh, graphs keep plotting their own columns |
| Where opened windows go | the root folder of the open project, no new folder (File > Append makes a folder named after the file; one window does not need one) |
| Formulas after a rename | a renamed book is followed: `[Book1]` in the formulas of the opened workbooks becomes `[Book3]` when `Book1` was renamed to `Book3` (the same fix applies to File > Append, which dropped these references before) |
| Opening twice | two copies, the second renamed |
| Never a project | `project_file::load` and `open_project` refuse a window file (`IoError::IsAWindowFile`); `window_file::load_window` refuses a project (`IoError::NotAWindowFile`); `is_project_file` is false for a window file, also one renamed to `.sheaf` |
| Writing | to a temporary file next to the target, renamed over it (as a project); a failed save leaves the old file |
| Unknown marker, version 99, damaged archive, missing data member | an error that names the file and the member; nothing is added to the project |
| Undo | opening is one undo step (app part) |

## 4. API (`sheaf-io`, `sheaf-core`)

- `sheaf_io::window_file::{BOOK_EXTENSION, GRAPH_EXTENSION}`, `WindowFileKind { Book, Graph }` with
  `extension()`, `marker()`, `from_marker()`, `from_extension(path)`, `of_window(&Window)`.
- `is_window_file(path)`: the extension names a window file, or a `.sheaf` file carries the manifest key
  (reads only the manifest). `window_file_kind(path)`: the kind in the manifest, whatever the name.
  `default_extension(path, kind)`: the name is kept when it already ends in the extension of `kind`; any other
  name gets that extension added after it (`keep.sheaf` becomes `keep.sheaf.sheafgraph`). The app's Save Window As uses it.
- `save_window(&Project, WindowId, &Path) -> Result<SavedWindow { kind, windows, notes }, IoError>`.
- `load_window(&Path) -> Result<WindowFile { kind, project, primary }, IoError>`: `project` is appended with
  `Project::append_windows`; `primary` is the position of the saved window in `project.windows` (the first
  graph of a graph file).
- `Project::extract_window(id) -> Result<WindowExtract { project, primary, notes }, ProjectError>`.
- `Project::append_windows(other) -> (new ids, dropped links)`; `Project::append` is unchanged except that
  appended formulas follow renamed books.
- `sheaf_io::recent`: `RecentList.books` and `.graphs`, `push_window(kind, path)`, `clear_books`,
  `clear_graphs`, `record_window(file, kind, path)`; `remove` and `forget` act on all four lists.

## 5. App part (`crates/app`)

- **Save Window As...** (`Cmd::SaveWindowAs`, `Action::SaveWindowAs(WindowId)`): enabled in the File menu of a
  workbook and of a graph (`Save &Window As...`); acts on the active window. Save Template As and Save
  Workbook As Analysis Template stay greyed. The chooser is filtered to the window's own extension (title
  "Save workbook as" or "Save graph as"); the starting name is the window's short name with the
  extension (Sheaf's choice: the short name is always a valid file name; the long name is not), in the
  project's folder (`SheafApp::folder`). `save_window_to` writes the file: a name whose extension is not the
  kind's (any case) gets the kind's extension appended (`x.sheaf` becomes `x.sheaf.sheafgraph`), so a
  project file or a file of the other kind is never written over by a mistyped name. Afterwards the status
  line says how many windows were written, the file goes to the top of Recent Books or Recent Graphs, and
  the dropped formulas and links are listed in the Messages log and in a window "Notes about the saved
  window". Saving changes neither the project, its name nor its saved state.
- **Opening** (`SheafApp::append_window_file`): File > Open (the chooser's first filter lists projects and
  window files), a drop, the command line, Recent Books, Recent Graphs and File > Append all route a file
  for which `is_window_file` is true there, before the project check. The windows are added with
  `Project::append_windows` in one undo step "Open window file" (undo removes all of them, redo brings them
  back), the saved window comes to the front (the graph of a graph file; its books are open behind it),
  the status line says how many windows were added, and the file goes to Recent Books or Recent Graphs. A
  window file never replaces the project and never asks about unsaved changes. When a drop or the command
  line has a project and window files, the project opens first and the window files are added to it (after
  the question about unsaved changes, if it is asked; when the project cannot be opened the window files
  are not added). File > Open shows the chooser first and asks about unsaved changes only when the chosen
  file is a project, so a window file chosen there asks nothing, as with a drop or Recent Books.
- **Renames** are listed in the window "Notes about the opened window file" and in the Messages log, one
  line per window: `Book1 is renamed Book2, because the project already has a window of that name.` A
  damaged, cut or foreign file shows "Could not open the window file" with the reason and adds nothing.
- **Tests:** `crates/app/src/app/ui_tests/save_window.rs`: the menu (Save Window As enabled in both kinds, Save
  Template As grey); a graph saved, a new project, the file opened: the book comes with it and the plots read
  the same numbers; a workbook round trip; a name clash renames and says so, and each copy plots its own
  columns; one undo step and redo; a window file never replaces the project; a command line and a drop;
  project and window file together; saving lists the file in the right recent list and the file; a click on
  an entry opens it; a missing or wrong-kind entry is reported and removed; Clear; a damaged file; the
  extension rules and an existing project file never written over; a formula outside the window dropped
  with a note. Not tested: the native file choosers (they cannot run in the test harness).

## 6. Tests that guard it

`crates/core/src/extract.rs`: a workbook alone; a graph with the books it reads once and not the others;
a formula outside dropped with a note and its values kept; formulas inside or on the own sheet kept;
analysis marks and links dropped with a note; an unknown window.
`crates/core/src/project.rs`: `append_windows` makes no folder; renamed books followed in formulas; the
one-pass rename.
`crates/io/src/window_file/tests.rs`: workbook round trip (text, NaN, infinity, units); graph with two
source books opened into a project with clashing names, same plotted data; formula outside dropped; garbage,
cut and absent files; unknown marker, no window of the kind, missing project member, newer version,
foreign format, missing column data; window file never taken for a project and the reverse (also renamed
to `.sheaf`); extension case; default extension; atomic replace and failed save; opening twice.
`crates/io/src/recent/tests.rs`: books and graphs lists (order, repeat, cap, round trip, an older file
without them, remove and clear, damaged entries).
