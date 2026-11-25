# T459: Figures drawn in the report Preview (rest of T439)

**Status:** done 2026-11-25 (Claude Windows "claude:work", notes-window.md "Linked reports"). A Notes window's Preview
draws each figure's graph as a picture, as wide as the window allows up to 480 points. The picture is kept until the
project changes.
**Size:** S. **Source:** T439 "Left".

## Tests

- `crates/app/src/app/ui_tests/notes_window.rs`: `a_report_previews_its_links_and_exports_html_with_the_graph` (the
  figure's picture).
