# ADR 0008: `.opj` is the default project format, with a companion file for what it cannot hold

Status: accepted (2026-11-12). Extends ADR 0004 (project file) and item 9 of ADR 0007.

## Context

The owner decided that File > Save Project, Save Project As, Ctrl+S and the command line's saves
write Origin `.opj` projects. The writer (`sheaf_io::opj`, spec section 13 of `opj-format.md`)
stores workbooks only. It leaves out graphs, folders, column formulas with their recalculate mode
and characters outside Windows-1252, and it refuses names Origin cannot store. A save must never
lose data.

## Decision

1. **Default.** The save dialog offers "Origin project (*.opj)" first and "Sheaf project
   (*.sheaf)" second. A new project is suggested as `Untitled.opj`. A project opened from a `.sheaf`
   file keeps saving to it (Save As to a `.opj` name switches). In the command line a project path
   without extension becomes `.opj`; an explicit `.sheaf` path still writes a Sheaf project.
2. **Companion.** If the writer reports anything it dropped, Sheaf also writes the whole project as
   `<name>.opj.sheaf` beside the `.opj` (the name cannot collide with a user's own `<name>.sheaf`).
   The status bar and a message say what only the companion holds. The companion is written after
   the `.opj`, so it is never older than it. The manifest of the companion carries
   `"companion": true`.
3. **Cleanup.** When a later save loses nothing, a companion beside the target is deleted only if
   its manifest says Sheaf wrote it. Any other file of that name is left alone and is not trusted
   when opening.
4. **Refusal.** When the writer refuses the project (`IoError::OpjWrite`), no `.opj` is written, an
   existing `.opj` stays as it was, and only the companion is saved. The app explains why and how to
   fix the name; its project counts as saved (the data is in the companion) and Ctrl+S tries the
   `.opj` again. The command line prints the same and exits with an error, because the file asked
   for does not exist.
5. **Opening `x.opj`.** If a Sheaf-written `x.opj.sheaf` exists and is at least as new as the
   `.opj`, it is opened (graphs included) and the user is told. If the `.opj` is newer (changed in
   Origin), the `.opj` is read and the user is told that the older companion was not used. Opening
   the companion by name also saves to the `.opj`.
6. **A `.opj` opened without companion** is not written back by a plain Save: Sheaf reads only part
   of what Origin keeps there, so the user chooses Save As on purpose (and sees the overwrite
   prompt of the file dialog).

The logic lives in `sheaf_io::save` (`save_project`, `SaveOutcome`) and `sheaf_io::open_project`;
the app and the command line only show the result.

## Consequences

Two files can exist per project; they should be kept together. Sheaf trusts file times to decide
which is newer, so copying only the `.opj` into the companion's folder with a later time makes the
companion look old (the user is told in that case).

## Safeguards added after review

- Saving over the Origin file a project was imported from (the app's Save As onto the same name, or a CLI `--save`/`-o` naming an `.opj` input) first copies the original to `<name>.opj.bak`. The first copy is never replaced, so it stays the true original.
- A file named `<name>.opj.sheaf` that Sheaf did not write (no companion mark) is never replaced. The save fails with a message before anything is written, and the user moves or renames that file.
