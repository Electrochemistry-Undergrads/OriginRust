# Sheaf MCP server (`sheaf-mcp`)

Sheaf's own interface for AI assistants. Design: ADR 0011. Clean-room: the tool names and shapes are
Sheaf's. Status: specification for T25 phase 1; the tools that exist at any time are the ones
`tools/list` returns.

## 1. Process and framing

- Binary `sheaf-mcp`, stdio transport. Options: `--root DIR` (repeatable), `--version`, `--help`.
- Messages are JSON-RPC 2.0, UTF-8, one per line, no embedded newlines, on stdin and stdout.
- Stdout carries protocol messages only. Logs go to stderr. End of stdin ends the process, exit code 0.
- Requests are handled one at a time, in order. A batch array is answered as an array.
- One `Project` per process (ADR 0011). It starts empty.

## 2. Protocol revisions

Supported: `2025-06-18`, `2025-03-26`, `2024-11-05`. `initialize` answers the client's
`protocolVersion` when it is one of these, else the newest (`2025-06-18`). The text of the newest MCP
revisions could not be fetched when this was written; read
https://modelcontextprotocol.io/specification and add a revision here only after reading its changes.
None of the three changes a message used here, except that `structuredContent` is sent only for
`2025-06-18`; the same data is always also in a text block.

## 3. Methods

| Method | Behaviour |
| --- | --- |
| `initialize` | Result: `protocolVersion`, `capabilities` = `{"tools":{"listChanged":false},"resources":{"subscribe":false,"listChanged":false}}`, `serverInfo` = `{"name":"sheaf-mcp","version":<crate version>}`, `instructions` (short text on names as handles and on the roots). |
| `notifications/initialized` | Accepted, no reply. |
| `ping` | Result `{}`. |
| `tools/list` | `{"tools":[{name, description, inputSchema, annotations}]}`. No paging. `annotations.readOnlyHint` is true for tools that change neither the project nor any file. |
| `tools/call` | Params `{name, arguments}`. Result `{content:[...], structuredContent?, isError}`. |
| `resources/list` | The resources of section 6. |
| `resources/templates/list` | The templates of section 6. |
| `resources/read` | Params `{uri}`. Result `{contents:[{uri, mimeType, text}]}`, or `blob` (base64) for PNG. |
| anything else | Error `-32601`. Unknown notifications are ignored. |

Prompts are not offered in phase 1 (the capability is not advertised).

Protocol errors: `-32700` parse error, `-32600` invalid request, `-32601` unknown method, `-32602`
invalid params (also an unknown tool name), `-32002` resource not found. Errors of the work itself
are never protocol errors; they are tool results with `isError: true` (section 8).

### Tool results

```json
{"content":[{"type":"text","text":"<JSON of the result, 2-space indented>"}],
 "structuredContent":{},
 "isError":false}
```

Image content block (PNG from `export_graph`):
`{"type":"image","data":"<base64>","mimeType":"image/png"}`.

## 4. Naming

Handles are names, never ids. A workbook: short name (`Book1`) or long name. A sheet: name,
case-insensitive; omitted means the active sheet. A column: short name or long name (the lookup of
`Worksheet::column_by_name`). A graph: name (`Graph1`). Row numbers count from 1. A tool that needs a
book defaults to the first workbook. A tool that needs a graph defaults to the only graph and fails
with the list of names when there are several (as the CLI does).

Notation: `string`, `number`, `integer`, `boolean`, `[T]` array, `?` optional.

- `Rows`: `rows` as `"FROM:TO"` or `x_range` as `"FROM:TO"`, at most one; an end may be empty. Same
  meaning as the CLI's `--rows` and `--x-range`.
- `Path` strings are absolute or relative to the first root and must resolve under a root.

Every result also carries `warnings: [string]` (notes from reading and writing), empty when there is
nothing to say.

## 5. Tools

### 5.1 Project

**`new_project`** `{}`. Replaces the project with an empty one. Result `{summary}` (as `list_contents`).

**`open_project`** `{path}` for `.opj`, `.opju`, `.sheaf`. Result
`{path, foreign, companion_used, warnings, summary}`. `foreign` is true for an Origin file; saving over
it later first copies it to `.bak` (ADR 0008).

**`save_project`** `{path?}`. Default: the path the project was opened from or last saved to;
required for a new project. `.opj` (or no extension) writes the Origin project, plus
`<name>.opj.sheaf` when something cannot be stored. `.sheaf` writes a Sheaf project. `.opju` is
refused. Result `{path, companion?, companion_removed, backup?, warnings}`. When the `.opj` writer
refuses the project, only the companion is written: the call is a tool error whose text names the
companion and the reason, and `structuredContent` has `{companion, refusal}`.

**`import_files`** inserts data into the current project.

```json
{"files": ["Path"],
 "layout": "books|sheets",
 "book_name": "string?",
 "format": "auto|chi|ascii",
 "file_name_in_comments": false,
 "ascii": {"delimiter": "auto|comma|tab|semicolon|space|<one character>",
           "header_lines": 0, "long_name_line": 0, "units_line": 0, "comments_line": 0,
           "decimal_comma": false, "thousands": "auto|off|comma|dot|space|apostrophe",
           "skip_rows": 0, "max_rows": 0, "dates": "detect|text", "date_order": "mdy|dmy"},
 "chi": {"split_scans": true, "split_segments": true, "add_milliamp": true,
         "add_charge": true, "invert_current": false, "exact_time": true},
 "plot": false}
```

`files` takes files, folders and wildcards. `layout` defaults to `books` (one workbook per file);
`sheets` puts each file in a sheet of one workbook. `format` and the option defaults are those of
`sheaf import`. `plot` adds a default graph per book. Result
`{imported:[{file, book, format, suggested_template, warnings}], errors:[string], graphs:[string]}`.
A file that fails is listed in `errors` and the others are imported; the call is a tool error only
when no file was imported.

**`list_contents`** `{book?}` (everything when omitted). The same facts as `sheaf info`:

```json
{"folders":[{"name":"","windows":["Book1"],"folders":[]}],
 "workbooks":[{"name":"Book1","long_name":"","sheets":[{"name":"","rows":0,"active":true,
    "columns":[{"short_name":"A","long_name":"","units":"","comments":"",
                "designation":"X|Y|Z|YError|XError|Label|Disregard|Group|Subject",
                "rows":0,"kind":"numeric|text|date|mixed","formula":null}]}]}],
 "graphs":[{"name":"Graph1","long_name":"","layers":1,"plots":[{"y":"Book1:Sheet1:B","x":"Book1:Sheet1:A"}],
            "width_in":0,"height_in":0}]}
```

**`read_columns`** `{book?, sheet?, columns?: [string], from?: integer, to?: integer, limit?: integer}`.
Columns default to all. `from` and `to` are 1-based, inclusive; defaults 1 and the last row. A reply
holds at most 10 000 cells (columns times rows); `limit` can lower the cap, not raise it. Result:

```json
{"book":"","sheet":"","columns":["A","B"],"from":1,"to":500,"total_rows":12000,
 "rows":[[0.1, 2.5]],
 "truncated":true,
 "next":{"from":501,"to":1000}}
```

Numbers are JSON numbers, text and dates are strings, missing cells are `null`, and NaN or infinite
values are the strings `"NaN"`, `"Infinity"`, `"-Infinity"`. `next` is present when `truncated`; pass
it to continue.

**`set_column_values`** `{book?, sheet?, column, formula, rows?, create?: {designation?, long_name?, units?}}` (`rows` is
"FROM:TO", counted from 1, an end may be empty).
Sets a column from a formula of Sheaf's expression language (`docs/spec/expression-language.md`, for
example `col(B)*1000`). A column that does not exist is created when `create` is given. Result
`{column, created, rows, recalculate: "none|auto|manual"}`. A column made by `create` is removed again when
the formula fails. Limit: 10 000 000 rows. Nothing changes on error.

### 5.2 Graphs and export

**`plot`** creates a graph or adds plots to an existing one.

```json
{"template": "line|scatter|...",
 "book": "string?", "sheet": "string?",
 "y": ["string"],
 "y2": ["string"],
 "graph": "string?",
 "panels": {"shape": "stack|2x2", "per_panel": 2, "y": [["B","C"],["D"]],
            "gap": 0.0, "margins": [0.1], "order": "bottom_up|top_down", "link": "x|y|xy|none"},
 "overlay_files": ["Path"],
 "style": {"x_title": "", "y_title": "", "y2_title": "",
           "x_min": 0, "x_max": 1, "y_min": 0, "y_max": 1, "x_step": 0, "y_step": 0,
           "y2_min": 0, "y2_max": 1, "y2_step": 0, "axis_colors": "plot|fixed",
           "width": 5, "height": 3.5, "line_width": 1.5,
           "colors": ["#1f77b4"], "labels": ["legend text"], "no_legend": false,
           "stack": "auto:10", "offsets": [0], "waterfall": [0],
           "no_y_tick_labels": false, "no_y_ticks": false,
           "x_labels": "auto|numbers|<column>", "x_label_angle": 0,
           "bar_gap": 0, "bar_overlap": 0,
           "text_labels": [{"text": "peak", "x": 0.5, "y": 0.2}],
           "draw": [{"kind": "line|arrow|rectangle|ellipse", "x1": 0, "y1": 0, "x2": 1, "y2": 1}]},
 "size": {"width_in": 5, "height_in": 3.5}}
```

`template` takes the names of `sheaf plot --template` (`Template::from_name`). `y` defaults to the
sheet's Y columns. `graph` adds to that graph instead of creating one. `panels` fields and values are
those of `sheaf plot --panels` and its panel flags. `overlay_files` imports each file and overlays
each file's Y columns. `text_labels` and `draw` positions are fractions of the layer frame, (0,0) the
top left; `size` sets the page size (the same as `style.width` and `style.height`, not both). The
`style` fields are those of `sheaf plot`'s style flags. Result `{graph, created, plots, warnings}`. Rules kept from the CLI: panels do not
combine with double-Y or waterfall templates; a waterfall takes no `y2`; `graph` and `panels` are
exclusive; `y2` and `graph` are exclusive; `overlay_files` excludes `book` and `sheet`. Style values
are checked as the CLI checks them: width and height 0.1 to 100 inches, finite axis limits that
differ, colors parsed by `Color::parse`.

**`export_graph`**
`{graph?, path?, format?: "png|svg|pdf|eps|tiff|emf|jpg|bmp", dpi?, width?, height?, margin?, return_image?}`.
`format` defaults to the extension of `path`, else `png`. `dpi` defaults to 300, range 36 to 1200;
the pixel count is capped at 40 million. `width` and `height` are lengths (`3.5in`, `9cm`, `800px`),
as the CLI's `--image-width`. `return_image` (default true when `path` is absent, false otherwise)
adds the PNG as an image content block, scaled to at most 1600 pixels on the long side; the file
keeps its full size. Result `{graph, path?, format, width_px, height_px, bytes}`.

**`export_data`** `{book?, sheet?, path, format?: "csv|tsv"}` writes the sheet as delimited text
(tabs for `.tsv` and `.txt`). Result `{path, rows, columns}`.

### 5.3 Analysis

**`list_analysis_tools`** `{}`. Result
`{tools:[{name, summary, options:[{name, type, default, description, required}], second_column}]}`.
This is the registry of ADR 0011 item 5; it names exactly what `analyze` accepts.

**`analyze`**

```json
{"tool": "integrate|smooth|differentiate|find_peaks|statistics|normalize|...",
 "options": {},
 "book": "string?", "sheet": "string?",
 "columns": ["string"],
 "rows": "FROM:TO", "x_range": "FROM:TO",
 "output": "next_to_input"}
```

`options` are those `list_analysis_tools` gives for the tool. A column a tool reads besides the Y
columns (a reference, a second signal) is an option holding a name. `output` accepts only `next_to_input`: results go next to the input and
tools that write a table use a new sheet by themselves (ADR 0005); `new_sheet` is refused with a
message. `list_analysis_tools` lists what is registered; analysis tools of the command line that it
does not list (tests, ANOVA, peak chains, FFT filter, coherence, averaging, ...) are not yet
available here. The result is the `Report`:

```json
{"book":"","sheet":"","new_columns":["Book1:Sheet1:D"],"new_sheet":"Integrate1","new_graph":null,
 "table":{"headers":[],"units":[],"rows":[{"label":"","values":[1.0,null]}],"row_units":[]},
 "message":"","notes":[],"warnings":[]}
```

`table` is null when a tool writes columns only; the values are then in the new columns (read them
with `read_columns`). Missing values are `null`. The numbers equal those of `sheaf analyze` for the
same file and options.

**`sort_sheet`** `{book?, sheet?, key, descending?}` and **`filter_sheet`**
`{book?, sheet?, condition, in_place?}` are separate tools because they are not analysis tools.
`condition` is a column formula.

### 5.4 Fitting

**`list_fit_models`** `{category?}`. Result `{models:[{name, category, parameters, priority}],
special:["linear","linearx","poly:N","formula","fdf"]}` (the catalog of
`docs/spec/fit-function-catalog.md`).

**`fit`**

```json
{"model": "linear|linearx|poly:3|<built-in name>",
 "formula": "string?", "params": ["a","b"],
 "fdf": "Path?",
 "book": "string?", "sheet": "string?", "y": ["string"], "rows": "FROM:TO", "x_range": "FROM:TO",
 "init": {"A": 1.0}, "fix": {"B": 0.0}, "bounds": {"A": [0, 10]},
 "weighting": "none|instrumental|statistical|direct", "yerr": ["string"], "xerr": "string?",
 "confidence": 0.95, "bands": "none|confidence|prediction|both", "band_level": 95,
 "max_iterations": 400, "tolerance": 1e-9, "scale_errors": true,
 "compare_with": "model?", "rank": ["model"], "global": {"share": ["param"]}, "regress": ["x columns"],
 "plot": "Path?"}
```

One mode at a time: a plain fit (one per Y column), `compare_with`, `rank`, `global` or `regress`.
Checks as in the CLI: confidence in (0,1), band level in (0,100), iterations at least 1, tolerance
above 0, `yerr` only with instrumental or direct weighting, one `yerr` column for all Y columns or one
each. `plot` renders the first fit as an image at that path. Result for a fit:

```json
{"outcomes":[{"model":"","equation":"","book":"","sheet":"","param_sheet":"","curve_sheet":"",
  "fitted_column":"","residual_column":"",
  "parameters":[{"name":"","value":0,"stderr":0,"t":0,"prob":0,"lcl":0,"ucl":0,"fixed":false}],
  "statistics":{"n":0,"dof":0,"rss":0,"reduced_chi2":0,"r2":0,"adj_r2":0,"pearson_r":0,"iterations":0},
  "anova":{}, "derived":[{"name":"FWHM","value":1.2}], "warnings":[], "message":""}],
 "image":null}
```

Comparison and ranking results carry the tables of `CompareOutcome` and `RankOutcome`. NaN is `null`.

## 6. Resources

| URI | Content |
| --- | --- |
| `sheaf://project` | `application/json`: the `list_contents` result. |
| `sheaf://column/{book}/{sheet}/{column}` | `text/csv` (template): the column with its X column, at most 10 000 rows; a header line carries the long name and unit. |
| `sheaf://graph/{name}.png` | `image/png` blob (template), 150 dpi. |

`resources/list` lists the project, each graph, and each column of the active sheet. Resource reads obey
the limits of section 7. No resource reads a file by path.

## 7. Limits and safety

- **Roots.** Only `--root` folders. A path is made absolute and canonical (links and `..` resolved; for
  a file not yet written, its nearest existing parent plus the file name) and must lie under a root.
  Wildcard matches, the `.opj.sheaf` companion and the `.bak` copy are checked the same way. Refused
  with `path is outside the allowed folders: <path>`. Without `--root` nothing is allowed: a tool that
  names a file fails with `no folder is allowed: start sheaf-mcp with --root DIR`. A link that points
  nowhere is refused (a write through it could land anywhere). Network shares (`\\host\share`,
  `//host/share`, `\\?\UNC\...`), device paths (`\\.\...`, `\\?\GLOBALROOT\...`) and reserved Windows
  device names in any part (`CON`, `PRN`, `AUX`, `NUL`, `COM1`-`COM9`, `LPT1`-`LPT9`, with or without an
  extension) are refused before the file system is asked, so no connection or device open happens. A
  project named in `import_files` is checked with its companion, as in `open_project`.
- **No shell, no script.** No tool runs a command or a script. Formulas use the expression language,
  which has no file or process access.
- **Size.** 10 000 cells per `read_columns` or resource reply; the caps of `export_graph` above; an
  imported file is at most 256 MB; a request line at most 8 MB.
- **Overwriting.** `save_project` over an Origin file the server opened first copies it to
  `<name>.opj.bak` (an older `.bak` is never replaced). Other saves and exports replace the file
  named, as the CLI does.
- **Stdout** carries protocol only.

## 8. Errors

A failing tool returns `{"content":[{"type":"text","text":"<message>"}],"isError":true}`. The message
is the CLI's, produced by `sheaf-ops`, with the cause chain joined by `: ` and without the `error: `
prefix. Where a shared message names a CLI flag, the request from JSON names the parameter instead
(`x_min and x_max must differ`, not `--x-min ...`); the rest of the text is identical. Argument
errors name the field (`missing field "formula"`). Partial success (some files not imported) is not
an error; see `import_files`.

## 9. Setup

Build: `cargo build --release -p sheaf-mcp`. The binary is `target/release/sheaf-mcp` (`.exe` on
Windows). On macOS `tools/make-macos-app.sh` copies it into `Sheaf.app/Contents/MacOS/`.

**Claude Code**

```
claude mcp add sheaf -- /path/to/sheaf-mcp --root ~/Data
```

Add `--scope project` to share the entry through `.mcp.json`. Check with `claude mcp list`.

**Claude Desktop** (`claude_desktop_config.json`, Settings > Developer > Edit Config)

```json
{"mcpServers":{"sheaf":{"command":"C:\\Program Files\\Sheaf\\sheaf-mcp.exe",
                        "args":["--root","C:\\Users\\me\\Data"]}}}
```

Restart Claude Desktop after editing.

**Other stdio clients.** Start `sheaf-mcp --root <folder>` as a child process with pipes, send
`initialize`, then `notifications/initialized`, then use the methods of section 3. A quick test:

```
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}' '{"jsonrpc":"2.0","method":"notifications/initialized"}' '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' | sheaf-mcp --root .
```

## 10. Tests

`crates/mcp/tests/` starts the binary with pipes and runs `initialize`, `tools/list` and a
`tools/call` chain import, plot, analyze, fit, export, save on synthetic files (`gen_chi.js` or the
fixtures). It compares the numbers with `sheaf-ops` called directly, which the CLI also calls. Unit
tests cover the root check (`..`, links, companion and backup paths), paging of `read_columns`, and
that every tool's schema accepts its documented example.

## 11. Phase 2 (not specified here)

`sheaf-mcp --attach` forwards tool calls to the open desktop app, each as one undo step. It needs its
own ADR and a section here.
