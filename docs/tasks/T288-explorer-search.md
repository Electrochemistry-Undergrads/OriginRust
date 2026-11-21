# T288: Search box in the Project Explorer

**Status:** done 2026-11-21 (Claude Windows "claude:3"). feature-parity "Project Explorer" (P0) gap
"no search or filters". A "Search windows" box over the window list keeps the rows whose name, long
name or comments hold its text (any case, outer spaces ignored); Escape in the box clears it. With
Subfolders ticked the search takes the windows of the folder and every folder below it, and the row's
tooltip names the folder a window is in. Ctrl+A then highlights only the listed windows. Favorites
are searched the same way. Sheaf's layout; Origin's later search box was not measured (Origin 2016
has none).
**Size:** S. **Spec:** feature-parity "Project Explorer".

## Tests

- `crates/app/src/panels/explorer.rs`: `the_search_lists_the_windows_whose_names_or_comments_hold_the_text`.
- `crates/app/src/app/ui_tests.rs`: `the_explorer_search_box_narrows_the_window_list`.
