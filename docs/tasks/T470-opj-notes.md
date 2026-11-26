# T470: Notes windows in .opj files

**Status:** done 2026-11-26 (Claude Windows "claude:3", opj-format.md 13.15, measured in Origin 2016).
**Size:** S–M. **Source:** feature-parity "Save project as .opj" and "Open .opj" rows ("notes ... are not written",
"skips ... notes").

## What it does

- Opening an `.opj` gives its Notes windows as Sheaf Notes windows (name and text, CR LF read as a line break) in
  the folders that list them. Origin's results log is left out.
- Saving an `.opj` writes Sheaf's Notes windows as Origin 2016 stores them: an 80-byte header (window rectangle
  cascaded by 26, state, creation and change times), the name and the text, in front of the results log. The folders
  list them with the kind word `0x00100000`.
- A long name or comments of a Notes window are not written (their place in the file was not found); a warning says so.
- Fixed on the way: the reader took a folder's Notes entry for the window with the same index (a Notes window's index
  counts in the notes list).

## Measured

- `win -n n MyNotes`, `win -n n Second` in Origin 2016 (`re-workspace/scratch/t470/n2.opj`): three blocks per window,
  header fields as above, folder entries `00 00 10 00` + index.
- `sheaf-notes.opj` written by `write_notes_project_for_origin` (ignored test), loaded in Origin and saved again
  (`rt.opj`): both texts byte for byte, `Diary` still in folder `Sub`, `exist()` 9 for both windows.

## Tests

- `crates/io/src/opj/write/feature_tests.rs`: `notes_windows_are_written_and_read_back_with_their_folders`,
  `a_notes_window_is_three_blocks_in_front_of_the_results_log`, `a_notes_window_with_a_long_name_warns`.
- `crates/io/src/opj/tests.rs`: `local_origin_file_gives_its_notes_windows` (Origin's own file, local only).
