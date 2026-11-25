# T449: Find in Project Files goes to the hit

**Status:** done 2026-11-25 (Claude Windows "claude:1", gui-look "Find in Project Files"), not measured (Origin 2026b
feature). After Open in Find in Project Files, once the project has opened (also after the unsaved-changes question),
the same search runs on it and Sheaf goes to its first hit as Find in Project's hit list does; the status line gives
the number of hits and the first one.
**Size:** S. **Source:** HANDOFF C, the rest of T361 and T441.

## Tests

- `crates/app/src/app/ui_tests.rs`: `find_in_project_files_lists_the_matching_projects_and_opens_one` (the found book
  is the active window, the status line counts the hits).

## Left

- Nothing.
