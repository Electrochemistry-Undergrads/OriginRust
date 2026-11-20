# T242: Send Graphs to Word (parity "Send to PowerPoint or Word")

**Status:** done 2026-11-20 (Claude Windows "claude:1"), **not measured** in Origin. The Send Graphs to
PowerPoint window has a Word (.docx) choice, and `sheaf export project -o graphs.docx` writes the same: a
Word document with one graph per Letter page, the graph as a 300 dpi PNG fitted to the text width and
centred, its name under it (`sheaf_io::docx`, `sheaf_ops::render::write_graphs_docx`). Opened in Word
through COM: 2 pages, 2 inline pictures, the captions.
**Size:** S. **Source:** feature-parity "Send to PowerPoint or Word"; follows T232.
**Spec:** `docs/spec/graphing.md` 17.11.

## Tests

- `crates/io/src/docx.rs`: picture fit, one picture per page with its caption and escaping, errors.
- `crates/cli/tests/send_pptx.rs`: `--all-graphs -o graphs.docx`.
- `crates/app/src/app/ui_tests/send_ppt.rs`: the Word choice and the written file.

## Left

- Page size from the project's Page Setup, A4, Origin's Send Graphs to Word options.
