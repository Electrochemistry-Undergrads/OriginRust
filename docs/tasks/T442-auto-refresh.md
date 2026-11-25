# T442: File > Import > Auto Refresh (Origin 2019b Data Connectors, 2026 periodic refresh)

**Status:** done 2026-11-25 (Claude Windows "claude:work", workbook-model.md 13t). An imported workbook can re-import
its source file on a timer, as Re-Import Directly does (T98). The choices are: every N seconds (5 to 86400, 60 to
start with), only when the file's modification time changed (on to start with), and Paused. Stop Refreshing removes
it. The setting saves with the workbook. A failed read pauses the refresh and says why. Each refresh is a "Re-import"
undo step. Sheaf's own design (Origin 2016 has no periodic refresh).
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Data Connectors and periodic refresh".

## Left

- Data Connectors proper (a connector object per source with a chosen data subset, web and database sources),
  refreshing while the project is closed, an indicator on the book, and keeping the link after a save as.

## Tests

- `crates/core/src/auto_refresh.rs`: only imported books, the interval range, paused books, save round trip.
- `crates/app/src/app/ui_tests/notes_window.rs`: `auto_refresh_re_imports_a_changed_file_and_pauses_when_it_is_gone`.
