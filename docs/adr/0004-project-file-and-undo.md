# ADR 0004: Zipped JSON project file, and undo by shared snapshots

Status: accepted (2026-11-11)

## Decision 1: project file

(Since ADR 0008 the default file written by Save is an Origin `.opj`, with a `.opj.sheaf` companion
when the `.opj` cannot hold everything. The `.sheaf` format described here is the lossless one.)

A project is saved as a ZIP archive (`.sheaf`) with a JSON description of all windows and one raw
little-endian `f64` file per numeric column. The full layout is in
`docs/spec/project-file-format.md`.

Reasons: it can be inspected and repaired with standard tools; column data stays compact and loads
fast (no number parsing); a version number in `manifest.json` allows the format to grow.

## Decision 2: undo and redo

The undo history stores whole copies of the project. Column data sits behind `Arc`, so a copy
shares all numbers and duplicates only names and styles. Editing a column copies just that
column's data (copy-on-write).

Reasons: every edit is undoable with no per-command undo code, which removes a whole class of
"undo forgot to restore X" bugs. Memory cost per step is the size of the changed columns only.

Consequence: code that changes the project goes through `Document::edit` (or `apply`) with a label
for the Edit menu. The history is capped at 100 steps.

Addendum (2026-11-12, ADR 0007 step 10): the project also holds view state that is saved but is
not an edit: which sheet is shown, and the places of the windows in the workspace
(`Project::window_places`, `docs/spec/project-file-format.md`). It changes through
`Document::adjust_view`, which neither adds an undo step nor marks the project as changed.


Addendum (2026-11-14, T28): undo snapshots also retain a content-state identity. The document's
saved checkpoint uses that identity, separately from the redraw revision, so undo/redo back to a
saved state clears the modified flag and its recovery copy. Identities are fresh across branches
and history trimming. Save is a boundary for merged edits; view-only changes retain the current
content identity. Recovered projects have no saved checkpoint until a successful normal save.
