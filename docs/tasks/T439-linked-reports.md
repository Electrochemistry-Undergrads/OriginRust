# T439: Linked reports from a Notes window (Origin 2019b linked HTML/Markdown analysis reports)

**Status:** done 2026-11-25 (Claude Windows "claude:work", notes-window.md "Linked reports"). A Notes window's text
reads as a report with live links into the project. `%([Book1]Sheet1,B,3)` is a cell, `$(formula)` is any formula over
named sheets, and a line `![caption](Graph1)` is a figure. Preview shows the report resolved. Export Report... writes
it as HTML (headings, paragraphs, figures) or Markdown, with each figure's graph as a 150 dpi PNG beside it. A link
that fails shows `[link error: ...]`. Sheaf's own design (Origin 2016's Notes window has no links).
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Linked HTML/Markdown analysis reports".

## Left

- Reusing a report over a batch of files, figures drawn inside the Preview, links to fit report values by name,
  metadata links (long name, units), and rich text (bold, lists, tables) in the HTML.

## Tests

- `crates/core/src/report.rs`: a cell link, a formula link, a figure, a bad link, Markdown and HTML.
- `crates/app/src/app/ui_tests/notes_window.rs`: `a_report_previews_its_links_and_exports_html_with_the_graph`.
