# T248: Parity rows corrected (Project Explorer, Window properties, Fill Column With)

**Status:** done 2026-11-20 (Claude Windows "claude:3"). Docs only. Three rows of
`docs/spec/feature-parity.md` named gaps that have since been built; each was checked in the code:

- Project Explorer: F2 renames, Del deletes, Ctrl+A highlights every window of the folder, folders can
  be renamed and deleted (`crates/app/src/panels/explorer.rs` `handle_keys`, Rename and Delete Folder
  buttons). Left: search and filters, free multi-select.
- Window properties and rename: the window edits the long and the short name and lists the
  Dependents (T220) (`RenameDialog` in `crates/app/src/app.rs`). Left: comments field.
- Fill Column With: Row Numbers and the random fills (T63) and the number, date/time and text
  patterns (T74) are in the Column menu. Left: Origin's pattern window layout and text quoting.

**Size:** S. **Source:** stale rows seen while working on T233 to T247.
