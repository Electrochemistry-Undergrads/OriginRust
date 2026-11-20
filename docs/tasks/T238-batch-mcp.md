# T238: MCP `batch_process` (Batch Processing for assistants)

**Status:** done 2026-11-20 (Claude Windows "claude:3"). The MCP tool `batch_process {files, template,
summary?, identifier?, delete_books?}` runs the batch of File > Batch Processing... (T165, T233) and
`sheaf batch` (T235) on the session project through the same `Project::batch_process`; attached to
the app it is one undo step.
**Size:** S. **Source:** T235 "Left" (MCP access).
**Spec:** `docs/spec/mcp.md` 5.1 (`batch_process`), `docs/spec/workbook-model.md` 13p.

## Tests

- `crates/mcp/tests/batch.rs`: books made and readable, the summary with the books deleted, refusals
  (unknown sheet, delete_books without summary, a template outside the roots, no readable file) that
  change nothing.
- `crates/mcp/tests/stdio.rs`: the tool is listed.

## Left

- Read options (format, ASCII settings) as `import_files` has them; a template by name from the
  template folder.
