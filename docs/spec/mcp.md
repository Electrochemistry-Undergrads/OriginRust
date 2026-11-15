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
| `initialize` | Result: `protocolVersion`, `capabilities` = `{"tools":{"listChanged":false},"resources":{"subscribe":false,"listChanged":false},"prompts":{"listChanged":false}}`, `serverInfo` = `{"name":"sheaf-mcp","version":<crate version>}`, `instructions` (short text on names as handles and on the roots). |
| `notifications/initialized` | Accepted, no reply. |
| `ping` | Result `{}`. |
| `tools/list` | `{"tools":[{name, description, inputSchema, annotations}]}`. No paging. `annotations.readOnlyHint` is true for tools that change neither the project nor any file. |
| `tools/call` | Params `{name, arguments}`. Result `{content:[...], structuredContent?, isError}`. |
| `resources/list` | The resources of section 6. |
| `resources/templates/list` | The templates of section 6. |
| `prompts/list` | `{"prompts":[{name, title, description, arguments:[{name, description, required}]}]}`: the prompts of section 5.5. |
| `prompts/get` | Params `{name, arguments}`. Result `{description, messages:[{role:"user", content:{type:"text", text}}]}`. An unknown prompt, a missing required argument or non-object `arguments` is `-32602`. |
| `resources/read` | Params `{uri}`. Result `{contents:[{uri, mimeType, text}]}`, or `blob` (base64) for PNG. |
| anything else | Error `-32601`. Unknown notifications are ignored. |

Prompts are offered: section 5.5. A message that has no `id` (a notification) is never answered, whatever is wrong with it.

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

A long name that two workbooks, or two columns of a sheet, share is refused with the short names to
use (a short name always wins over a long name). Analysis tool names are the command's names in
snake_case (section 5.3).

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
 "excel": {"sheets": ["Name or 1-based position"], "include_hidden": false},
 "plot": false}
```

`files` takes files, folders and wildcards. `layout` defaults to `books` (one workbook per file);
`sheets` puts each file in a sheet of one workbook. `format` and the option defaults are those of
`sheaf import`. An Excel file (`.xls`, `.xlsx`, `.xlsm`) becomes one workbook with a sheet per Excel
sheet; `excel.sheets` picks sheets, `excel.include_hidden` adds hidden ones, and the header and row
settings of `ascii` apply too (see `docs/spec/import-excel.md`). `plot` adds a default graph per book. Result
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

**`plot`** creates a graph or restyles an existing one.

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
 "size": {"width_in": 5, "height_in": 3.5},
 "options": {}}
```

`template` takes the names of `sheaf plot --template` (`Template::from_name`). `y` defaults to the
sheet's Y columns. `graph` restyles that graph instead of creating one; omit `template`, `y` and `y2` with it. `panels` fields and values are
those of `sheaf plot --panels` and its panel flags. `overlay_files` imports each file and overlays
each file's Y columns. `text_labels` and `draw` positions are fractions of the layer frame, (0,0) the
top left; `size` sets the page size (the same as `style.width` and `style.height`, not both). The
`style` fields are those of `sheaf plot`'s style flags. Result `{graph, created, plots, warnings}`. Rules kept from the CLI: panels do not
combine with double-Y or waterfall templates; a waterfall takes no `y2`; `graph` and `panels` are
exclusive; `y2` and `graph` are exclusive; `overlay_files` excludes `book` and `sheet`. Style values
are checked as the CLI checks them: width and height 0.1 to 100 inches, finite axis limits that
differ, colors parsed by `Color::parse`.

**`export_graph`**
`{graph?, graphs?: [string], all_graphs?, path?, format?: "png|svg|pdf|eps|tiff|emf|jpg|bmp", dpi?, width?, height?, margin?, return_image?}`.
`format` defaults to the extension of `path`, else `png`. `dpi` defaults to 300, range 36 to 1200;
the pixel count is capped at 40 million. `width` and `height` are lengths (`3.5in`, `9cm`, `800px`),
as the CLI's `--image-width`. `return_image` (default true when `path` is absent, false otherwise)
adds the PNG as an image content block, scaled to at most 1600 pixels on the long side; the file
keeps its full size. Result `{graph, path?, format, width_px, height_px, bytes}`.
`graphs` (names, in page order) or `all_graphs` (every graph, in project order) put several graphs
into one PDF, one page per graph, as Origin 2018b's multi-page PDF export (expGraph `multipage`).
Each page is the one-graph PDF export of that graph with the same size and margin settings, and
each passes the pixel cap. Neither goes with `graph`; names resolve as `graph` does. Sheaf's own
rules, not measured in Origin: with more than one graph a `path` is required, the format must be
pdf (by `format` or the extension), `return_image: true` is refused (no image is returned), and
more than 500 pages (a repeated name counts each time) is refused, writing no file.
Result `{graph, graphs, pages, path, format, width_px: null, height_px: null, bytes}`, `graph`
being the first page's graph. A list that resolves to one graph is the one-graph export.

**`export_data`** `{book?, sheet?, path, format?: "csv|tsv|xlsx", all_sheets?}` writes the sheet as
delimited text (tabs for `.tsv` and `.txt`), or as an Excel workbook for a path ending in `.xlsx` or
`format: "xlsx"` (`all_sheets` writes every sheet of the book). Result `{path, rows, columns}`; for
xlsx also `{sheets, warnings}`.

### 5.3 Analysis

**`list_analysis_tools`** `{}`. Result
`{tools:[{name, summary, options:[{name, type, default, description, required, choices?}], second_column, bridged?, input_schema?}]}`.
It names exactly what `analyze` accepts: the 21 hand-mapped tools first, then every other tool of
`sheaf analyze` with `bridged: true` and the JSON Schema of its options in `input_schema`.

**`analyze`**

```json
{"tool": "integrate|smooth|fft_filter|ttest|anova|mannwhitney|histogram|normality|corr|...",
 "options": {},
 "book": "string?", "sheet": "string?",
 "columns": ["string"],
 "rows": "FROM:TO", "x_range": "FROM:TO",
 "output": "next_to_input"}
```

`analyze` reaches every tool of `sheaf analyze`: the CLI command has 55 subcommands and all of them
are tools here (`peaks` and `stats` are also accepted under their command-line names). Names are the
command name with `_` for `-` (`fft_filter`, `reduce_dup`, `column_math`, `mannwhitney`, `anova2`,
`signal_correlation`); a name with `-` is accepted too, and so is any case. Two kinds:

- **Hand-mapped** (21: integrate, math, column_math, subtract_ref, smooth, differentiate, normalize,
  baseline, interpolate, find_peaks, statistics, fft, hilbert, envelope, level_crossing,
  median_filter, decimate, translate, subtract_line, polygon_area, intersect). Their options are
  those `list_analysis_tools` gives; a column a tool reads besides the Y columns (a reference, a
  second signal) is an option holding a name.
- **Bridged** (the other 34: convolution, signal_correlation, ifft, fft_filter, coherence,
  trace_interp, average, reduce_dup, reduce_x, ttest, anova, power, anova2, mannwhitney, signrank,
  rowstats, grubbs, dixon, sign, ks2, mediantest, friedman, kruskal, corr, histogram, crosstab,
  discfreq, bin2d, normality, distfit, probplot, qqplot, sort, filter). Their options are generated
  from the command line's own option definitions (`sheaf_ops::args`, clap), so a new option or
  a changed default of the CLI appears here without new code and the values are checked by the same
  rules. Each option key is the long option name with `_` for `-` (`--no-graph` is `no_graph`,
  `--row-ranges` is `row_ranges`); the kebab form is accepted. A flag is a boolean (`false` is the same as
  leaving it out), an option that repeats or takes a comma list is an array (each element is one
  value, so a column named `Current, mA` stays whole; a plain text is split on the comma as on the
  command line), a choice is a string among `choices`, numbers are numbers (a whole float such as
  `7.0` is accepted for an integer; an integer option is at most 10 000 000 in size). An unknown key
  is an error that lists the valid ones. The input file, `export`, `save` and the read options
  (delimiter, header lines, ...) are not options here: the data is the project, and
  `export_data` and `save_project` write files. `book`, `sheet`, `columns`, `rows` and
  `x_range` of the call are the CLI's `--book`, `--sheet`, `--y`, `--rows` and `--x-range` (with the
  same rule that `rows` and `x_range` exclude each other, and that `sort`, `filter`, the factor tests
  and `power` refuse the selections that make no sense for them). A path option (`ref_file`) must lie
  under a root. `power` needs no data; its table goes into a new workbook of the project, named in
  the reply. `sort` and `filter` are also available as `sort_sheet` and `filter_sheet` (below).

`output` defaults to `next_to_input` (results next to their source columns, ADR 0005).
`new_sheet` uses the core analysis Output setting: results go into a new worksheet in the same
workbook, with the X columns needed to plot them. The original worksheet is unchanged. Tools
that intrinsically create tables or result worksheets keep their own output layout; power analysis
keeps creating its result workbook. Both hand-mapped and CLI-bridged analysis tools use the same
core implementation. `new_sheet` is refused for sort and filter jobs before they change anything;
use `filter_sheet` with `in_place: false` to create a filtered copy. A failed call is rolled back.
The result names the new sheet and fully qualified result columns. The result is the
`Report`, for every tool:

```json
{"book":"","sheet":"","new_columns":["Book1:Sheet1:D"],"new_sheet":"Integrate1","new_graph":null,
 "table":{"headers":[],"units":[],"rows":[{"label":"","values":[1.0,null]}],"row_units":[]},
 "message":"","notes":[],"warnings":[]}
```

`table` is null when a tool writes columns only; the values are then in the new columns (read them
with `read_columns`). Missing values are `null`. The numbers equal those of `sheaf analyze` for the
same file and options. Bridged `sort` answers `{sheet, sorted_by, rows}` and `filter` answers
`{sheet, rows, before}`. A long name that two workbooks or two columns share is refused with the
short names to use instead of being resolved to the first.

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
 "plot": "Path?", "options": {}}
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

### CLI options for plotting and fitting

Both `plot` and `fit` accept an optional `options` object. Its schema in `tools/list` is generated
from the CLI's clap definitions, and values are parsed with the same types and constraints as the
CLI. Keys use underscores in place of hyphens; the hyphenated spelling is also accepted. Flags
are booleans; repeated or delimited options accept arrays. Array elements keep embedded commas.
Unknown options and duplicate spellings are errors. Do not give the same setting in both the
structured arguments and `options`.

For example, a plot can use `{"y":["B"],"options":{"x_title":"Potential (V)","line_width":2}}`.
A fit can use `{"y":["B"],"options":{"model":"poly:2","confidence":0.99}}`.
The shared CLI-to-library conversions are used by both frontends; no separate plotting or fitting
implementation runs in the MCP server. Existing structured calls keep their arguments.

The data remains the server's project. CLI input files, read options, save/export switches and
listing switches and export resolution/sizing controls are excluded; use `import_files`,
`save_project`, `export_graph`, `export_data`
and `list_fit_models` for those operations. The existing top-level `fit.plot` path remains
supported; it is not an option flag here. Allowed path options (including `fit.options.fdf`) are
checked against `--root`. CLI enum values use their clap spellings, for example
`options.panel_order: "bottom-up"`; the structured `panels.order` keeps `"bottom_up"`.

### 5.5 Prompts

Three prompts, each a single user message that tells the assistant which tools to call:

| Prompt | Arguments | What it asks for |
| --- | --- | --- |
| `plot_cv_files` | `files` (required) | `plot` with `overlay_files`, `list_contents`, a look at the figure, `export_graph`. |
| `integrate_peak` | `column`, `from`, `to` (required) | `analyze` `integrate` with `x_range` `from:to` and `end_point_baseline`, then the area, peak position, height and width from the table. |
| `fit_curve` | `column`, `model` (required) | `fit` with `model` and `y`, then parameters with errors, R-square and a verdict. |

The text names only tools and options of this server.

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
that every tool's schema accepts its documented example. `tests/analyze_bridge.rs` enumerates the subcommands of `sheaf analyze` from clap and checks that each is reachable by name through `analyze` and survives empty options, and compares the numbers of ten bridged tools with the CLI code run in-process. `tests/hardening.rs` covers links inside a root that point outside, shared long names and notifications.

## 11. Phase 2 (not specified here)

`sheaf-mcp --attach` forwards tool calls to the open desktop app, each as one undo step. It needs its
own ADR and a section here.
