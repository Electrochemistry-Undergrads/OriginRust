# T441: Find in Project Files in the background

**Status:** done 2026-11-25 (Claude Windows "claude:1", gui-look "Find in Project Files"), not measured (Origin 2026b
feature). Find All searches on a background thread: files with hits appear as they are found, a spinner and
"Searching n of N files..." show progress, Stop ends the search after the file in hand; closing the window or Open
stops it too.
**Size:** S. **Source:** HANDOFF C, the rest of T361 and T407.

## Tests

- `crates/app/src/dialogs/find_files.rs`: `a_background_search_finds_what_the_plain_one_finds` (and Stop).
- `crates/app/src/app/ui_tests.rs`: `find_in_project_files_lists_the_matching_projects_and_opens_one` (frames take
  the results in).

## Left

- Jumping to the hit inside the opened project.
