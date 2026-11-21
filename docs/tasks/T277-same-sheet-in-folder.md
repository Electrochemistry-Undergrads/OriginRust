# T277: Activate the same sheet in every workbook of a folder

**Status:** done 2026-11-21 (Claude Windows "claude:3"). HANDOFF next candidates C (Origin 2027). The
sheet tab's menu has Activate This Sheet in All Books in Folder: every workbook of the book's Explorer
folder (not its subfolders) with a sheet of the same name (any case) shows that sheet. Not measured
(Origin 2027 is not installed; the release note names only the feature): the matching rule and the
menu wording are Sheaf's.
**Size:** S. **Spec:** `docs/spec/workbook-model.md` 13h.

## Tests

- `crates/core/src/project.rs`: `the_sheet_of_one_name_becomes_active_in_every_book_that_has_it`.
- `crates/app/src/app/ui_tests/sheet_tab.rs`: `a_sheet_becomes_active_in_the_other_books_of_the_folder`.

## Left

- The 2027 system variables for rich text and dynamic merge in the long name and comments label rows.
