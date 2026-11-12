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

## Companion file of an `.opj` (optional manifest key)

`manifest.json` may carry `"companion": true`. Sheaf sets it only on the file it writes as
`<name>.opj.sheaf` beside an Origin project `<name>.opj` that could not hold the whole project
(see `docs/adr/0008-opj-default-with-companion.md`). A reader that does not know the key ignores it,
so the format version stays 1. The key tells Sheaf that it wrote the file itself: only such a file
is opened in place of the `.opj`, and only such a file is deleted when a later save to the `.opj`
loses nothing. A project the user saved as `<name>.sheaf` never has the key and is never deleted.

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

## Compatibility promise

Version 1 files stay readable by all later versions. A change that old readers cannot ignore raises
`version`.
