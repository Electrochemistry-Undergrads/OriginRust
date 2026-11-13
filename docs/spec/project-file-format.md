# Project file format (`.sheaf`), version 1

**Summary.** A project file is a ZIP archive. It holds one JSON file that describes every window
(workbooks with their column labels, graphs with their styles), and one small data file per column.
Anyone can unzip a project and read it with a text editor or a few lines of Python. The format has a
version number so later versions of the application can still read old files.

## Archive members

| Path | Content |
|---|---|
| `manifest.json` | `{"format": "sheaf-project", "version": 1, "app": "<app version that wrote the file>"}` |
| `project.json` | The project description: the JSON form of the `Project` type of the `sheaf-core` crate (windows in creation order, folder tree, next free identifier). Column *data* is not in this file. |
| `data/<column id>.f64` | Numeric column: the values as consecutive 8-byte IEEE-754 doubles, little-endian, no header. A missing value is NaN. The number of rows is the file size divided by 8. |
| `data/<column id>.txt.json` | Text column: a JSON array of strings, one per row. A missing value is the empty string. |

`<column id>` is the decimal column identifier used in `project.json` (field `id` of a column).
Every column has exactly one data member; an empty numeric column has a zero-length `.f64` member.

Members are stored with DEFLATE compression. `manifest.json` is the first member so that a reader
can check the format without reading the rest.

## Matrix windows (optional)

`project.json` windows may also be `{"type": "matrix", "id", "name", "long_name", "sheets": [...], "active_sheet"}`
(a matrix book; see `matrices.md`). Each sheet is `{"name", "data_id", "rows", "cols", "x_from", "x_to",
"y_from", "y_to"}` plus the optional label objects `x`, `y` and `z` (`long_name`, `units`, `comments`, each
omitted when empty). The sheet's numbers are the archive member `data/m<data_id>.f64`: `rows * cols`
IEEE-754 doubles, little-endian, row by row, a missing value is NaN. `data_id` shares the identifier counter with
windows and columns. A reader checks that the member has exactly `rows * cols` values and names the sheet when
it does not; a matrix without a data member is an error. A file with no matrix window has no `data/m*.f64`
member, so files written before matrix windows existed read as before and the format version stays 1 (a reader
that does not know matrices would fail on the unknown window type, so a file with matrices should be opened
with a version of Sheaf that has them).

## Heat maps and contour plots (optional key of a graph layer)

A layer of a graph in `project.json` may carry `field`: `{"source": {"kind": "matrix", "window", "sheet"} or
{"kind": "xyz", "x", "y", "z"} (column ids), "kind": "contour" or "cells", "fill", "lines", "labels",
"palette" (`rainbow`, `blue_white_red`, `gray`), "levels": {"min", "max" (optional), "major", "minor"}, "line",
"label_font", "scale": {"visible", "title" (optional), "font", "title_font"}}`. A layer without `field` is an
ordinary layer, so older files read as before.

## Companion file of an `.opj` (optional manifest key)

`manifest.json` may carry `"companion": true`. Sheaf sets it only on the file it writes as
`<name>.opj.sheaf` beside an Origin project `<name>.opj` that could not hold the whole project
(see `docs/adr/0008-opj-default-with-companion.md`). A reader that does not know the key ignores it,
so the format version stays 1. The key tells Sheaf that it wrote the file itself: only such a file
is opened in place of the `.opj`, and only such a file is deleted when a later save to the `.opj`
loses nothing. A project the user saved as `<name>.sheaf` never has the key and is never deleted.

## Window files (optional manifest key)

`manifest.json` may carry `"window": "workbook"` or `"graph"`. Sheaf sets it on the file that File > Save
Window As writes (`.sheafbook`, `.sheafgraph`): the archive of a small project that holds one window (and,
for a graph, the workbooks it reads). Such a file is never opened as a project (it is appended; see
`window-files.md`). A reader that does not know the key ignores it, so the format version stays 1.

## Window places (optional key of `project.json`)

`project.json` may carry `window_places`: a list, bottom of the stacking order first, of
`{"window": <window id>, "x", "y", "width", "height", "state"}`. `x` and `y` are the window's
top-left corner relative to the top-left corner of the desktop app's workspace, and `width` and
`height` its size in its normal (not maximised) state, all in screen points. `state` is `normal`,
`minimised`, `maximised` or `hidden` (closed, listed in the project explorer only); a missing
`state` means `normal`. The topmost shown window is the active one. The key is absent when no window
was ever shown; windows that are not listed are placed by the cascade. It is view state: the app
writes it when saving, never as an undo step, and an older reader ignores it, so the format version
stays 1. The `.opj` writer does not store it yet, so a project reopened from an `.opj` without a
companion gets the cascade.

## Reading rules

1. Read `manifest.json`. If `format` is not `sheaf-project`, the file is not a project. If `version`
   is greater than the newest version the reader knows, refuse with a message that names both
   versions. Unknown JSON fields anywhere are ignored, so a newer minor revision can add fields.
2. Read `project.json`, then attach each column's data member. A column with no data member is an
   error that names the column.
3. After loading, make sure the "next free identifier" is larger than every identifier in use.

## Writing rules

Write to a temporary file in the same folder, then rename it over the target, so a crash while
saving never leaves a half-written project. JSON is pretty-printed (two-space indent) so that it is
readable and diffs are small.

## Projects without data (T77)

File > Save Project without Data... and File > Duplicate Project without Data use `Project::without_data()`.
Origin 2016 does not document the two items; the nearest documented feature is Clone Project
(`clrAllData`, mode 0: clear all data, keep windows and operations). [W]
(Origin help, X-Function `clrAllData`: https://www.originlab.com/doc/X-Function/ref/clrAllData) The rest is an UNMEASURED
Sheaf choice. Kept: windows and places, names, column labels, units, designations, number formats,
formulas, graphs. Removed: every value (a column keeps its storage kind), masks, cell notes, cell and
conditional formats, row filters, analysis marks and links, report comparison sources.
Save writes this copy to a file of the person's choice (`.opj` or `.sheaf`) and leaves the open
project, its file name and its unsaved state alone. Duplicate asks about unsaved changes, then the
copy replaces the open project as an unsaved UNTITLED project; the old file is not touched.

## Compatibility promise

Version 1 files stay readable by all later versions. A change that old readers cannot ignore raises
`version`.
