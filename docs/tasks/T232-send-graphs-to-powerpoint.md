# T232: Send Graphs to PowerPoint (parity "Send to PowerPoint or Word")

**Status:** done 2026-11-20 (Claude Windows "claude:1"), **not measured** in Origin. The graph page's Send
Graphs to PowerPoint... (active graph or all graphs) and `sheaf export project -o slides.pptx` write a
PowerPoint file with one 16:9 slide per graph, the graph as a PNG fitted and centred
(`sheaf_io::pptx`, `sheaf_ops::render::write_graphs_pptx`). Opened in PowerPoint through COM without repair.
**Size:** S. **Source:** feature-parity "Send to PowerPoint or Word" (sendppt).
**Spec:** `docs/spec/graphing.md` 17.11.

## Tests

- `crates/io/src/pptx.rs`: picture fit, package parts and escaping, errors.
- `crates/cli/tests/send_pptx.rs`: `--all-graphs` (two slides, the tall page's picture), one `--graph`.
- `crates/app/src/app/ui_tests/send_ppt.rs`: the menu item, the window's choice, the written file.

## Left

- Origin's sendppt options (slide template, placement, slide titles, embedding as a graph object), the
  window title-bar menu entry, the Project Explorer folder entry, and Send to Word.
