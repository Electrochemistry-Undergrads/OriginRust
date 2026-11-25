# T361: Find in Project Files

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", gui-look "Find in Project Files"). HANDOFF C (Origin 2026
"Find in Project Files"): File > Find in Project Files... searches every `.sheaf`, `.opj` and `.opju` project of a
folder (and its subfolders) with Find in Project's rules, lists the files with hits, and opens the chosen one. Not in
Origin 2016, so the window is Sheaf's own.
**Size:** S. **Spec:** gui-look, Find in Project Files.

## Left

- Jumping to the hit inside the opened project (notes windows: done in T407; searching in the background: T441).

## Tests

- `crates/app/src/dialogs/find_files.rs`: files with hits, subfolders on and off, an unreadable file listed.
- `crates/app/src/app/ui_tests.rs`: `find_in_project_files_lists_the_matching_projects_and_opens_one`.
