# T486: Folders and matrices in the Project Browser

**Status:** done 2026-11-26 (Claude Windows "claude:3", workbook-model.md 13s). Left item of T433 (claude:work's, unclaimed).
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Project Browser": folders remain.

## What it does

View > Project Browser gets a Folder column (the project folder of each item's window, searched and sortable) and lists matrix books
and their sheets (Show: Matrix, Matrix Sheet). A matrix sheet's Long Name, Units and Comments are its Z labels, set on many at once or
edited in place like a column's.

## Tests

- `crates/core/src/project_browser.rs`: `the_browser_shows_folders_and_matrices`.
- `crates/app/src/dialogs/project_browser.rs`: the header sort (Long Name is now the fifth column).
