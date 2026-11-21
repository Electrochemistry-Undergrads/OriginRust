# T299: Header comment lines (Comments From .. To) as Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:2"), measured.
Feature-parity "Header, long name, units, comments lines" (P0) gaps. Origin 2016's `impasc` joins the
comment lines CommsFrom .. CommsTo with line breaks, and its auto mode reads long name, units and
comments from sub-header lines 1, 2 and 3 on. Its `ByBottom`, `OneLine` and `NameUnitLine` options do
not exist in 2016 (the command fails), so those gaps belong to later Origin versions. Sheaf now
detects up to 15 label lines, joins the comment lines and has `comments_to` (window, CLI
`--comments-to`).
**Size:** S. **Spec:** `docs/spec/import-ascii.md` 3. Probe: `re-workspace/scratch/import-header/`.

## Tests

- `ascii::tests::comment_lines_are_joined_as_origin_does` (io),
  `import_text_options::comment_lines_from_to_are_joined` (cli).

## Left

- The leading-character rule for main header lines (Origin 2018b), counting from the bottom and the
  one-line name+unit splitter (later versions); Comments To in the Excel window, the wizard and MCP.
