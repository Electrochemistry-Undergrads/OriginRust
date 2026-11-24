# T407: Find notes windows

**Status:** done 2026-11-23 (Claude Windows "claude:1", gui-look "Find in Project Files"), not measured. Find in Project
and File > Find in Project Files search notes windows: their short and long names and their text, one hit each,
element "Notes"; Find in Project's Notes Windows option (on by default) switches it off. `FindSpec::notes`.
**Size:** S. **Source:** HANDOFF C, the rest of T361 (Origin 2026 searches notes in project files).

## Tests

- `crates/app/src/project_find.rs`: `notes_windows_are_searched_by_name_and_text_unless_switched_off`.
- `crates/app/src/dialogs/find_files.rs`: `the_text_of_a_notes_window_is_found_in_a_saved_project`.

## Left

- Searching in the background with progress; jumping to the hit inside the opened project.
