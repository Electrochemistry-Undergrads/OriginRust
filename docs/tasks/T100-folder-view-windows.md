# T100: View Windows, Folders List, folder Properties

**Status:** claimed 2026-11-16 (Claude Windows "t30-origin", branch `view-windows`).
**Size:** M. **Source:** greyed inventory package B12 (Project Explorer > View Windows: None, Windows in
Active Folder, Active Folder and Subfolders; Window > Folders > Folders List; the explorer folder's
Properties...).
**Spec:** `docs/spec/gui-look.md` or `workbook-model.md` (a new section on folders).

## Goal

Which windows the workspace shows follows the Project Explorer's active folder as Origin 2016's View
Windows setting says; Window > Folders lists folders to go to; a folder's Properties... shows (and edits)
what Origin's does.

## Origin facts

Measure in Origin 2016 (screenshots): the default View Windows choice and what each does when the active
folder changes, what Window > Folders lists and does, and the folder Properties window.

## Done when

The items are enabled and behave as measured, the setting is saved where Origin keeps it, fmt/clippy/tests
pass, inventory and spec updated.
