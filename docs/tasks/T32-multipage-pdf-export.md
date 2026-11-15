# T32: Multi-page PDF export

**Status:** claimed 2026-11-15 (session "mac-multipdf", macOS, no Origin), branch `multipage-pdf`.
**Size:** M. **Source:** `origin-2026-2027-gaps.md` (Import, reporting and export: Multi-page PDF
figure export, Origin 2018b).

## Goal

Several graphs go into one PDF file, one graph per page, from the Export Graphs dialog, the CLI
and the MCP server. Pages stay vector (text kept as text), as the one-graph PDF export does.

## Origin facts

- Origin 2018b release notes: "Option to export multiple graphs as a single multi-page PDF file"
  (Import and Export). [W:originlab.com/2018b]
- It is the `multipage` variable of `expGraph` (File > Export Graphs): int, default `0`; set, it
  exports "the multiple graph pages to one PDF file with multi-pages". [W:x-function/ref/expgraph]
- Which graphs: `export` (Active Page default; All in Active Folder, recursive, open, embedded,
  project, Specified). Indexing of file names follows the Graph Page list order.
- Not measured (no Origin on this machine): the dialog label and where the check box sits, the
  file name used for the combined file, whether each page keeps its graph's own size or a common
  paper size, page order. Sheaf's choices are labelled in graphing.md 9.6.

## Where

- `crates/plot/src/lib.rs` (`render_pdf`, `render_print_pdf`, `export_bytes_with`),
  `crates/plot/src/raster.rs` (`to_pdf`, `svg2pdf`): new multi-page writer.
- `crates/app/src/dialogs/export.rs` (`ExportDialog`, `Which`, `plan`, `run`): the check box.
- `crates/ops/src/args.rs` (`ExportArgs`, `PrintArgs`), `crates/cli`: several `--graph` and
  `--all-graphs` for `.pdf` output.
- `crates/mcp`: `export_graph` with several graphs.
- `docs/spec/graphing.md` 9.6 (new), `docs/spec/mcp.md`, README CLI section.

## Steps

1. Multi-page writer in `sheaf-plot`: each graph converted with svg2pdf's chunk API and placed on
   its own page sized to that graph's exported area (export size/margin settings apply to each).
   Also a paper variant through Page Setup (`print_svg`) for `sheaf print`.
2. Export Graphs dialog: with PDF and more than one graph, an "Export as multi-page PDF" check box;
   one file named from the first graph's substituted name; overwrite rule applies to that file.
3. CLI `sheaf export`/`sheaf print`: repeated `--graph` and `--all-graphs`; more than one graph
   with a non-PDF output is refused with a clear message.
4. MCP `export_graph`: a `graphs` list (or all) for `.pdf`.
5. Tests: page count and page sizes of the written PDF, text kept as text, order, refusal cases,
   dialog plan, CLI and MCP round trips.

## Done when

One PDF with one page per selected graph is written from the dialog, the CLI and MCP; the checks
pass; graphing.md 9.6 records what is Origin's and what is Sheaf's choice.

## Don't

- Don't rasterise pages, or rebuild the per-graph SVG/PDF rendering; reuse `render_svg` and the
  print placement.
- Don't change single-graph PDF bytes or the other formats.
