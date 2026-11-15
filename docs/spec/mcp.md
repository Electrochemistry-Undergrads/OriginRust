# Sheaf MCP server (`sheaf-mcp`)

Sheaf's own interface for AI assistants. Design: ADR 0011 (headless server) and ADR 0013 (attach to the
open desktop app). Clean-room: the tool names and shapes are Sheaf's. Status: sections 1 to 10 are
T25 phase 1 (done); section 11 specifies phase 2, `sheaf-mcp --attach` (built). The tools
that exist at any time are the ones `tools/list` returns.

## 1. Process and framing

- Binary `sheaf-mcp`, stdio transport. Options: `--root DIR` (repeatable), `--attach` (phase 2, section
  11), `--version`, `--help`.
- Messages are JSON-RPC 2.0, UTF-8, one per line, no embedded newlines, on stdin and stdout.
- Stdout carries protocol messages only. Logs go to stderr. End of stdin ends the process, exit code 0.
- Requests are handled one at a time, in order. A batch array is answered as an array. (With `--attach`
  the process also reads stdin while a call waits, so `ping` and `notifications/cancelled` are handled
  at once: section 11.6.)
- Without `--attach` there is one `Project` per process (ADR 0011). It starts empty. With `--attach`
  there is none: the project is the one open in the desktop app, which must be listening.

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
`{book, sheet, column, created, rows, recalculate: "none|auto|manual"}` (`book` and `sheet` name what was used). A column made by `create` is removed again when
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
`style` fields are those of `sheaf plot`'s style flags. Result `{graph, created, plots, book, sheet, warnings}` (`book` and `sheet` name the data plotted, and are
`null` when an existing graph was restyled). Rules kept from the CLI: panels do not
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
`format: "xlsx"` (`all_sheets` writes every sheet of the book). Result `{book, sheet, path, rows, columns}`;
for xlsx also `{sheets, warnings}`. `path` is the file that was written: an xlsx export to a name with no
extension is written as `NAME.xlsx` (the reply says so), headless and attached.

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
`condition` is a column formula. Results: `sort_sheet` `{book, sheet, rows}`; `filter_sheet`
`{book, source_sheet, sheet, rows, before}`, where `source_sheet` is the sheet that was filtered and `sheet`
is where the kept rows are now (the new sheet when `in_place` is false, as in phase 1).

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
| `sheaf://app` | `application/json`: current desktop window, selection, Data Reader pin and dialog/edit state; listed only by an attached desktop host (section 11.18). |
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
  named, as the CLI does. Attached (section 11.11) exports never replace a file.
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
that every tool's schema accepts its documented example. `tests/analyze_bridge.rs` enumerates the subcommands of `sheaf analyze` from clap and checks that each is reachable by name through `analyze` and survives empty options, and compares the numbers of ten bridged tools with the CLI code run in-process. `tests/hardening.rs` covers links inside a root that point outside, shared long names and notifications. The tests of phase 2 are listed in section 11.15.

Phase 2 also changes the headless server (section 11.10): it runs its project in a `Document`, so a
formula column follows `set_column_values`, `fit` is atomic (a failed plot write undoes the fit), and
opening a project does not recalculate it. Five results gain `book` and `sheet`.

## 11. Phase 2: attach to the open desktop app

Design: ADR 0013. Status: built (T25 phase 2). Everything in sections 1 to 10 holds unless this section
says otherwise. The tool names, arguments and result shapes do not change, except the additive `book` and
`sheet` fields of section 11.10.

### 11.1 Overview

The desktop app (`sheaf-app`) can listen on a local endpoint. `sheaf-mcp --attach` does not open a
project of its own: it forwards the tool calls of the assistant to the running app. Each change is one
undo step of the app's project, windows the assistant creates open as usual, and the owner sees every
change and can undo it within the undo limit (100 steps; files written to disk are not undone). Results
equal those of the headless server and the CLI, because the app runs the same `sheaf-mcp` and `sheaf-ops`
code (section 11.10).

Options of `sheaf-mcp` added: `--attach`. With `--attach`, `--root DIR` (repeatable) lists the folders the
assistant may touch. The proxy makes each root absolute and canonical (the code of section 7) and sends
absolute paths only. The process still speaks MCP on stdio.

The app side is off by default. The owner turns it on with Tools > `AI Assistants (MCP)` or with
`sheaf-app --mcp`. The menu tick is the owner's consent to the link; the roots are chosen by the client
(section 11.13).

### 11.2 Run folder and discovery

- **Candidates**, in this order. The first is the variable `SHEAF_MCP_RUN_DIR`, when set (tests and
  unusual systems). On Linux `$XDG_RUNTIME_DIR/sheaf`, when that variable is set and absolute. Then the
  data-directory fallback: `~/Library/Application Support/Sheaf/run` on macOS, `$XDG_DATA_HOME/sheaf/run`
  or `~/.local/share/sheaf/run` on Linux, `%LOCALAPPDATA%\Sheaf\run` on Windows. The list is
  `sheaf_io::recovery::run_dir_candidates()`. There is no `/tmp` fallback.
- **The app** listens in the first candidate that passes the checks below. **The client** probes every
  candidate, in order, because an assistant started by another program can have another environment
  than the app.
- **Files.** `mcp.lock`, `mcp.json`, and on Unix `mcp.sock`.
  - `mcp.lock` is held with an exclusive `File::try_lock` for as long as the app listens, as
    `sheaf_io::recovery::Session` does. The system drops the lock when the process ends, even after a crash.
  - `mcp.json` is written atomically (mode 0600) after the endpoint is bound:
    `{"v":1,"pid":N,"app":"<version>","endpoint":{"kind":"unix","path":"/.../mcp.sock"}}` or
    `{"kind":"tcp","addr":"127.0.0.1:49152","token":"<64 hex>"}`.
  - The process that gets the lock removes an old `mcp.sock` and `mcp.json`, then binds.
- **Folder checks** (Unix; the app and the client both run them). Every missing level is created with
  `DirBuilder` at mode 0700 (the umask can only remove bits). An existing folder must be a real directory
  (`symlink_metadata`, not a link) and owned by the current user, which is tested as: the owner of the
  folder equals the owner of a probe file that the process just created in it and removed. If its mode has
  any group or other bit and it is owned by the user, it is set to 0700 and checked again; if it still has
  them, or has another owner, it is refused. The client does not create a missing folder (it skips it).
  On Windows the folder is under the user's profile and keeps the rights it inherits; they are not
  checked. A link is refused there too.
- **Unix socket.** A `UnixListener` on `mcp.sock`, set to mode 0600. A path longer than 100 bytes is
  refused with a message that names `SHEAF_MCP_RUN_DIR`. The peer's user is not asked (`peer_cred` is
  unstable): the 0700 folder is the access control.
- **Windows.** A `TcpListener` on `127.0.0.1`, port 0 (the system picks one), and a 256-bit token from
  `getrandom` (64 hex digits). The same TCP code is built on every platform and tested on all; Unix
  uses it only in tests. No other address is ever bound.
- **One endpoint per user.** A second app instance that cannot get the lock reports "busy". The app shows a
  warning in the status bar and its menu item stays unticked.
- **Client discovery.** For each candidate folder: run the folder checks; open `mcp.lock` read-only and
  call `try_lock_shared`. If it gets the lock, no app listens there: unlock and go on. If the lock would
  block, read `mcp.json` (if it is missing, retry for 2 seconds, since the app writes it after the lock)
  and connect. A connection that fails goes on to the next folder. When nothing answers, the error names
  the folders and the reason for each: `no Sheaf is listening (looked in: /run/user/1000/sheaf: no lock;
  /home/me/.local/share/sheaf/run: does not exist); turn on Tools > AI Assistants (MCP) in Sheaf, or start
  it with --mcp`. The proxy answers `initialize` and `tools/list` without the app, connects when the first
  call needs it, and connects again for a later call.

### 11.3 Framing and handshake

JSON, one message per line, UTF-8, on the socket. A request line is at most 8 MiB. A reply line is at
most 64 MiB (a PNG in base64 plus text). The reader is the one `sheaf-mcp` uses for stdin
(`link/frame.rs`). An oversize request line is answered with an error and the connection is closed.

The first line of a connection must be a hello, within 10 seconds. The clock runs whatever the peer sends:
a peer that sends a byte now and then is cut off at 10 seconds too, and the first line is at most 64 KiB
(the 8 MiB limit applies after the hello). A line over the cap is answered with an error and the
connection is closed.

```json
{"hello":{"v":1,"client":"claude-code 1.2","roots":["/Users/me/Data"],"token":"<64 hex, TCP only>"}}
```

The app answers `{"hello":{"v":1,"app":"<version>"}}`. Anything else as the first line, including HTTP
text, closes the connection without a reply. A TCP hello with a missing or wrong token gets the error
`not allowed` and is closed; the token is compared in constant time (a fixed-length byte comparison with
no early exit). The `client` text is untrusted: it is sanitised (section 11.8) and cut to 40 characters
before it is shown.

Each root must be an absolute path; a hello with a relative root is refused with an error and the
connection is closed. A hello names at most 16 different roots (a repeated root counts once); more is
refused with `too many roots (at most 16)`. The app builds a `Roots` for the connection with `Roots::new`: a root that does not
exist or is not a folder gives an error reply and the connection is closed. An empty list keeps the phase-1
behaviour: no folder is allowed.

Up to 8 connections may be open (a connection that has not finished its hello counts). A ninth gets the
error `Sheaf is busy with other assistants; try again` and is closed.

### 11.4 Requests and replies

Each request has a number `id` chosen by the client. One call may be in flight per connection; a second
`call` before the first is answered is an error.

| Request | Meaning |
| --- | --- |
| `{"id":1,"op":"call","name":"plot","args":{...},"structured":true}` | A `tools/call`. `structured` is true when the negotiated protocol is `2025-06-18`. |
| `{"id":2,"op":"cancel","target":1}` | Cancel the call `1` if it has not started. The reply is `{"id":2,"result":{"cancelled":true}}` or `false`. When it is `true`, call `1` is answered with the error `-32800` `cancelled`. |
| `{"id":3,"op":"resources_list"}` | The `resources/list` result of section 6 for the live project. |
| `{"id":4,"op":"resource_read","uri":"sheaf://project"}` | The `resources/read` result. |

Replies: `{"id":1,"result":{...}}` where `result` is exactly what `tools/call` (or the resource method)
returns in section 3, or `{"id":1,"error":{"code":-32602,"message":"..."}}` for a protocol failure
(`-32602` bad parameters or unknown tool, `-32800` cancelled, `-32000` anything else: busy, the link
turned off, the change limit). A failing tool is still a `result` with `isError: true`. The proxy copies
`result` into its JSON-RPC reply unchanged and only adds its own envelope. The app checks the tool name
again and refuses an unknown one.

The app can also write a line that answers no request, then closes the connection:
`{"notice":{"kind":"replaced","message":"the project in Sheaf was replaced"}}` or
`{"notice":{"kind":"closed","message":"the owner turned the link off"}}`.

### 11.5 What the proxy answers by itself

`initialize`, `ping`, `tools/list`, `prompts/list`, `prompts/get`, `resources/templates/list` and
notifications are answered in `sheaf-mcp` with no app. `tools/list` shows 13 tools: the 16 of section 5
minus the three with `headless_only` (`new_project`, `open_project`, `save_project`). A call to one of
the three is answered by the proxy with the tool error `not available while attached: the person using
Sheaf opens and saves projects` (the app refuses it too, whatever the client). The `instructions` text of
`initialize` is replaced: the assistant is attached to a project that a person has open in Sheaf; every
change appears in the person's undo list as "AI: ..."; the person saves the file; name the sheet in calls,
because an omitted sheet means the sheet the person has selected; a call may wait while the person has a
dialog open or is typing in a cell; files written are never replaced, so pick a new name; and the folders
allowed.

### 11.6 The proxy while a call waits

The proxy has a reader thread on stdin and one on the link, feeding one event queue.

- `ping`, other notifications and `initialize` are answered at once, also while a call waits. A further
  request that needs the app waits its turn (requests are handled in order).
- `notifications/cancelled` for a request that waits behind the call in flight, or that has not arrived
  yet: the proxy remembers the id (the last 1024) and drops that request instead of sending it, with no
  reply (the client has abandoned it). A cancelled call never reaches the app.
- `notifications/cancelled` for the waiting call: the proxy sends the `cancel` operation. If the app
  answers `cancelled: true` the proxy sends nothing for that MCP request (the client has abandoned it).
  If the call has started, the proxy goes on waiting.
- The proxy waits at most **50 seconds** for a reply. Then it sends `cancel` and waits 2 seconds for the
  answer. If the app confirms that the job never started, the tool error is `the owner is busy or has a
  dialog open in Sheaf; the call did not run; try again`. Otherwise it is `still running in Sheaf; check
  list_contents before retrying`. It then closes that connection and opens a new one for the next call,
  and ignores any reply that arrives late for an old id.
- **No automatic retry of a mutating call.** A request is written once. If writing fails before any byte
  was sent, the proxy connects again and writes it once more. If the write succeeded and the reply was
  lost (the app closed), the proxy returns the tool error `connection to Sheaf lost; check the project
  before retrying`, because the change may have been made.
- A `notice` of kind `replaced` is kept by the proxy and returned once, as a tool error to the call in
  flight or to the next call (which is not sent): `the project in Sheaf was replaced; look at it again
  with list_contents before changing anything`. The proxy then connects again for the call after.
- End of stdin: every line read before it is still answered, on every path: the call that waits and the
  calls behind it finish and get their replies, then the proxy closes the link and exits with code 0. So
  `printf '<request>' | sheaf-mcp --attach` is answered. (An assistant that stops without closing a
  call cancels it with `notifications/cancelled`; a call that already went to the app is not undone by
  the end of input.)

### 11.7 Which tools forward, and how

| Tool | In the app |
| --- | --- |
| `list_contents`, `read_columns`, `export_graph`, `export_data` | Run on a copy of the live project. No undo step. `export_*` write only under the roots, never replace a file, and never touch the protected set (section 11.11). |
| `list_analysis_tools`, `list_fit_models` | Run at once, even while a dialog is open. They do not read the project. |
| `import_files`, `set_column_values`, `plot`, `analyze`, `sort_sheet`, `filter_sheet`, `fit` | `mutates`: run inside `Document::edit`. One undo step, named as in section 11.8. |
| `new_project`, `open_project`, `save_project` | `headless_only`: refused (section 11.5). |

Resource operations run on a copy of the live project, like the reads.

A mutating call that fails or panics is rolled back and leaves no step, in the project and in the
connection's `Session`. A call that changes nothing leaves no step. `fit` is atomic: its plot window and
plot file are part of the same call, and a failure of the plot write undoes the fit. After a mutating
call the formula columns are recalculated and analysis links updated (it is a `Document::edit`).

Windows the call creates (graphs, workbooks) are found by `sync_windows` in the same frame and open as
usual. The app does not add a second way to open windows.

### 11.8 Undo step names and the sanitizer

The name is built before the call, from the tool name, the arguments and the live project:
`AI: ` + tool, then the book name, then the columns as `B(Y)` (short name and designation as the project
says; at most 3, then `+N`), then a model or analysis tool name, the formula for `set_column_values`, and
the first file name with `+N` for `import_files`. If nothing useful is found the name is `AI: <tool>`.
Examples: `AI: plot Book1 B(Y)`, `AI: fit Gauss Book1 B(Y)`, `AI: set_column_values Book1 C = A*2`,
`AI: import_files run1.csv +2`.

**One sanitizer** (`sheaf_mcp::link::sanitize(text, max_chars)`) cleans every string that came over the
wire before the interface shows it: the undo name (60 characters), the client name (40), the roots in the
tooltip, file names, and the error text of a failed call in the status bar and the Messages Log. It
removes control characters (Unicode Cc), bidi controls and the other format characters (Cf, for example
U+202A to U+202E, U+2066 to U+2069, U+200B to U+200F, U+FEFF, tag characters), and U+2028 and U+2029;
turns runs of white space into one space; trims; and cuts at a character boundary with `...` added when
it cut. These are the strings the link layer shows. Text that a tool stores in the project is not cleaned:
a column long name or units from `set_column_values` `create`, a formula, and the names of imported
sheets and files reach the grid headers and the explorers as they came, like text the owner typed or
a file the owner imported. They are data of the project, not messages of the link.

The Edit menu shows `Undo AI: plot Book1 B(Y)` through the usual undo label. Undo restores the whole call.
Redo works as for any step. A new call clears the redo list, as any edit does. The history limit of the
app (100 steps) applies, so an old AI step can leave the list; project changes are undoable within that
limit, and files an assistant wrote on disk are not undone. Each connection may make at most 1000
mutating calls (a call that starts counts, whatever its result); after that the call is refused with
`this connection has made 1000 changes; ask the owner to turn the link off and on, or restart the
assistant`.

### 11.9 The app's queue and the owner's work

The app runs the endpoint on its own threads: an accept thread, and a reader thread and a writer thread
per connection. They do socket work and queueing only; they never touch the project.

1. A reader thread reads a line and makes a job: the client, the roots, the operation, an atomic state
   (QUEUED, RUNNING, CANCELLED) and a reply sender for the writer thread. It puts the job on a queue (at
   most one per connection, so at most 8 jobs) and asks the window for a repaint. A `cancel` operation is
   handled by the reader thread at once: it tries to change the state from QUEUED to CANCELLED.
2. Each frame the UI thread, in `SheafApp::live_step` (`crates/app/src/live.rs`, `Live::step`), pops the
   whole queue into a local pending list. It runs after the actions of the frame and before `sync_windows`. While a connection is open or a job is
   pending, the app asks for a repaint every 250 ms, so a lost wake-up cannot stall.
   `list_analysis_tools` and `list_fit_models` never reach the UI thread: the connection's reader thread
   answers them itself.
3. **Non-mutating jobs run at once**, in arrival order, whatever else is deferred. A job that is not
   mutating sees the project as drawn, including the live preview of an open dialog.
4. **A mutating job is deferred** while any dialog is open (`dialog_open()`) or a cell is being edited in
   any workbook (`editing` or `editing_label`). **At most one mutating job runs per frame.** The pure
   tools of section 11.7 are never deferred.
5. To run, the UI thread calls `Job::execute(doc, protected)`: it changes QUEUED to RUNNING (if that fails
   the job was cancelled and is skipped), runs the call through `run_on_document` and sends the reply. A
   panic becomes a tool error. `protected` is computed for that job from the live document (the open
   path, the Origin source, the recovered path, the recovery folder and the run folder). Each frame
   `host::sweep` drops jobs that are no longer queued and cancels the old ones (item 6).
6. A job that is still QUEUED 55 seconds after it was made is cancelled by the app with the reply `the
   owner is busy or has a dialog open in Sheaf; the call did not run; try again`. This is the backstop for
   clients other than the proxy, whose 50 seconds come first.
7. A closed connection cancels its queued job; a running job finishes and its reply is dropped.
8. After a mutating call the status bar and the Messages Log show the undo name. A failed call shows a
   status-bar warning `AI: <tool> failed: <message>` and a Messages Log line (the message is
   sanitised). No message box opens.

A heavy tool (fit, a large import, a large render) blocks the UI for its run. This is accepted. A
snapshot worker for reads is later work.

### 11.10 One implementation, and what differs

`crates/mcp/src/exec.rs` holds `Session { path, foreign, suggested }`, the `Policy` (attached or not, the
roots of the caller, the protected set) and `run_on_document`. The headless `Server` and the app both
call it. The `Server` owns a `Document` with its history limit set to 0 (`Document::set_limit`), so it keeps no
steps.

- A `mutates` tool runs in `Document::edit`, with panics caught inside the closure. `Document::edit` also
  puts the project back when the recalculation of formula columns and analysis links after the tool
  panics, and the executor catches that panic too: no half-made change, no undo step, no dirty flag, and
  the session as it was. A tool that is not
  `mutates` runs on a copy that is discarded: no step, no recalculation, no dirty flag. `new_project` and
  `open_project` (headless only) go through `Document::replace`, never `edit`: opening a project with a
  formula column leaves its data as the file has it.
- The same code gives the same `result` in both modes. This changes phase 1 in three places: a formula
  column follows `set_column_values` and the other edits (headless recalculates now), `fit` is atomic, and
  five results gain `book` and `sheet`.
- **`book` and `sheet` in results.** `set_column_values`, `export_data`, `plot`, `sort_sheet` and
  `filter_sheet` add `book` (the short name) and `sheet` (the sheet used) to their result. `filter_sheet`
  keeps `sheet` as it was in phase 1, the sheet that holds the rows now (the new sheet when `in_place` is
  false), and adds `source_sheet`, the sheet that was filtered. `plot` gives `null` for both when it
  restyled an existing graph. The other tools already name them.
- **Known differences between headless and attached**, all documented and not hidden:
  - An omitted `sheet` means the workbook's active sheet. Attached, that is the sheet tab the person has
    selected (view state in the project), and it can change between two calls. Headless, it is the
    sheet the last import or call left active. The assistant is told to name the sheet; the result says
    which one was used.
  - An omitted `book` means the first workbook in project order, not the active window, in both modes.
  - `resources/list` lists the columns of the active sheet of the first workbook that has one; the same
    view-state dependency.
  - `Session.suggested` (the template an imported book suggests) is kept per connection and starts empty
    for a new connection.
  - Reads see a dialog's live preview (item 3 of section 11.9).
- `Session.path` and `Session.foreign` are not used while attached: nothing there saves or opens.
- `Document` gains `set_limit(n)` and `generation()` (bumped by `replace`; the app drops the connections
  when it changes, section 11.12). `sheaf-mcp` gains `getrandom` 0.3 as a direct dependency (already in
  `Cargo.lock`) and no `unsafe` code. The app depends on the `sheaf-mcp` library; `sheaf-mcp` never
  depends on the app.

Modules of the library: `exec` (executor, labels, policy), `link` (`frame`, hello and operations,
`sanitize`, `discovery`, `transport`, `host` that accepts connections and queues jobs, `client`), `proxy`
(the stdio loop of section 11.6). `main.rs` only parses options. The app side is
`crates/app/src/live.rs`: `Live` owns the host and runs the frame's work (`Live::step`), `SheafApp::live_step`
calls it, and the status bar draws the chip.

### 11.11 Files the assistant writes

Attached, `export_data`, `export_graph` (also the multi-page PDF) and the `plot` path of `fit`:

- **Never replace.** If the destination exists the call fails with `will not overwrite: '<path>' exists;
  choose a new file name`. The check is `symlink_metadata`, so a link (even one that points nowhere) and any
  file that is not a regular file count as existing, and a destination that is a link is never followed.
- **Atomic.** The file is written to a new temporary file in the target folder (`create_new`, so no link
  is followed; the name keeps the extension, because the writers choose the format from it), then
  published without replacing: `hard_link` to the final name (it fails when the name exists) and the
  temporary file is removed. Where the file system has no hard links the write checks again and uses
  `rename`, which leaves a small race. A failed write leaves no temporary file. Headless writes are
  unchanged (section 7).
- **The protected set** is computed for each call from the live document and refuses a new file with any of
  these names: the open project's path, its `.sheaf` companion and its `.bak`; the Origin file it was read
  from with its companion and `.bak`; the original path of a recovered project; the temporary names
  (`.<name>.<pid>.tmp`) of those files; any name that ends in `.opj`, `.opju`, `.sheaf`, `.bak` or `.lock`;
  and anything inside the recovery folder or the run folder. Names are compared by canonical parent plus
  file name, case-insensitively on macOS and Windows. Refused with `that name belongs to the project open
  in Sheaf: <path>`. The rule matters although nothing is replaced: a planted `.bak` would stop the backup
  of an Origin project (an older `.bak` is never replaced), and a file planted in the recovery folder
  would be offered back as a recovery.
- **Names Windows would read as another name** are refused on every platform: a final component that ends
  in a dot or a space, or that contains `:` (Windows drops the trailing dot or space, and reads `name::$DATA`
  as a stream of `name`, so `X.opj.bak.` would otherwise create the protected `X.opj.bak`). The message is
  `a file name may not end in a dot or a space or contain ':'`.
- **The xlsx name.** `export_data` to xlsx with a name that has no extension is written as `NAME.xlsx`;
  that name is checked against the roots and these rules, not the one the caller gave.

### 11.12 Owner interface and lifecycle

- **Menu.** Tools > `AI Assistants (MCP)`, a tick item. Ticked means the endpoint is listening. The state
  is never saved: the app starts with it off.
- **Flag.** `sheaf-app --mcp` starts the app with the endpoint on. If it cannot start (another instance
  holds it, the folder fails the checks), the app starts anyway and shows the reason in the status bar and
  the Messages Log.
- **Status bar.** A boxed `AI: connected (n)` while at least one assistant is connected. The tooltip lists
  each client's name and its roots (sanitised), and says that its changes are undo steps named `AI: ...`
  and that files it writes are not undone. A connect and a disconnect each write a line to the status bar
  and the Messages Log with the client name and roots.
- **Turning it off** (untick, or exit). The app sets a closing flag; wakes the accept thread by connecting
  to itself; answers every queued job with `the owner turned the link off` (QUEUED to CANCELLED) and sends
  the notice `closed`; shuts down every connection; waits for the threads for at most 2 seconds (the UI
  thread never hangs; a thread that does not end is left to end when its socket fails); removes `mcp.sock`
  and `mcp.json`; drops the lock. The toggle runs on the UI thread, so no job is running at that moment.
  Turning it on again works and makes a new endpoint (and token).
- **The owner replaces the project** (New, Open, Recover, a dropped file: whatever calls
  `Document::replace`, seen as a change of `Document::generation()`). Every connection gets the notice
  `replaced` and is closed, and queued jobs are answered with the same text. The proxy turns this into one
  tool error (section 11.6), so the assistant looks at the project again before it acts on names from the
  old one.

**As built** (`crates/app/src/live.rs`, `app.rs`, `menus.rs`):

- The tick item sits under Tools, after Autosave, and is ticked while the endpoint listens. `SheafApp::
  set_link` starts it with the folders of `Startup::mcp_dirs` (`run_dir_candidates()` in the app, none in
  tests) and turns it off with `Live::stop`, which `on_exit` calls too. A start that fails (another Sheaf
  holds the lock, no usable folder) is a status-bar warning and a Messages Log line; the tick stays off.
- A connect and a disconnect write `AI assistant connected: <name> (may use: <roots>)` and
  `AI assistant disconnected: <name>` to the status bar and the Messages Log. The chip is `AI: connected (n)`;
  its tooltip lists each client and its folders and says that the changes are undo steps named `AI: ...`
  and that files it writes are not undone.
- After a mutating call that changed the project the status bar and the log show the undo name; a failed
  call shows `AI: <tool> failed: <message>` (sanitised, at most 200 characters) in both.
- The generation check runs in `Live::step`, after the owner's actions: New, Open and Recover all end in
  `Document::replace`, so all three drop the connections (the log says `The project was replaced: n AI
  connection(s) closed`). Undo and Redo do not replace the project and do not drop them.
- Untick and exit call `LinkServer::stop` (bounded by 2 seconds). Calls the UI thread had already taken
  into its pending list are answered by the same stop, because they share their state with the queue.

### 11.13 Safety rules

- Local and user-only: Unix 0700 folder and 0600 socket; Windows loopback with a 256-bit token. No other
  address is bound.
- A first line that is not a hello closes the connection (a web page cannot drive the endpoint, and cannot
  know the token).
- No tool runs a command or a script, and none is added. The app executes only the tools of section 5.
- **File access is confined to the roots of the connection** (section 7), in the process that touches the
  files. A client that sends no roots can touch no files. **The roots are chosen by the client, and the
  app does not ask the owner about them**; it shows them (tooltip, Messages Log). The confinement is
  therefore voluntary against any process of the same user that can reach the endpoint: such a process
  can connect, name any roots and drive the app. It is not defence against an attacker already running as
  the user, who can read the user's files anyway; what it gains is the open project. The same is true of
  the Windows token, which any process of the user that can read `mcp.json` has. This is the
  confused-deputy limit of ADR 0013.
- Writes never replace and never touch the protected set (section 11.11).
- A request over 8 MiB, a ninth connection, and a second call in flight are refused with an error.
- Wire text is sanitised before it is shown (section 11.8).
- The owner sees every change (status bar, Messages Log, undo list) and can undo each one within the undo
  limit. The tick in the menu is the owner's consent; there is no per-call prompt.

### 11.14 Failure modes

| Situation | What the assistant sees |
| --- | --- |
| No app is listening | Tool error `no Sheaf is listening (looked in: ...)`. `tools/list` still works. |
| Dialog open or cell edit for more than 50 s | Tool error `the owner is busy or has a dialog open in Sheaf; the call did not run; try again`. |
| A call still running after 50 s | Tool error `still running in Sheaf; check list_contents before retrying`. Not retried. |
| 8 connections open | Tool error `Sheaf is busy with other assistants; try again`. |
| App quits during a call | Tool error `connection to Sheaf lost; check the project before retrying`. Not retried. |
| The owner turned the link off | Tool error `the owner turned the link off`. |
| The owner replaced the project | One tool error `the project in Sheaf was replaced; ...`, then calls work on the new project. |
| 1000 changes made on this connection | Tool error with the limit text of section 11.8. |
| First line is not a hello, wrong token, or a bad or relative root | The connection is closed; for a bad root or token an error reply comes first. |
| Second app instance | The app reports busy in the status bar; the first keeps the endpoint. |
| Run folder fails the checks, or the socket path is too long | The app shows why and does not listen. |
| `new_project`, `open_project`, `save_project` | Refused (section 11.5). |
| Export to a name that exists, or a protected name | Tool error (section 11.11); nothing is written. |
| A tool fails or panics | `isError: true`; the project is as before; no undo step. |

### 11.15 Tests that guard it

In `crates/mcp` (no GUI):

- The undo-name table (section 11.8), and unit tests of the sanitizer: control, bidi, format and
  line-separator characters, collapsed spaces, the cap on a character boundary.
- A parity script run twice, headless and through `run_on_document` on a document with history, with
  identical results: `import_files`, `plot`, `analyze`, `fit` with a plot, `set_column_values`,
  `sort_sheet`, `filter_sheet`, export, `read_columns` paging.
- Headless on a `Document` (limit 0): a formula column follows `set_column_values`; **opening a project
  with a formula column leaves its data unchanged**; `fit` with a failing plot write leaves no fit; a panic
  injected in a tool leaves the project and the `Session` as before; the five `book` and `sheet` fields.
- With history: one step per mutating call; a failing call, a panic and a no-op leave no step; undo
  restores the project; the 1000-call cap.
- The `headless_only` refusals (also when called by hand with `attached`).
- Writes (attached): never replace an existing file, a dangling link, a link to a file, a folder, a FIFO
  (Unix); no temporary file left after a failure; the hard-link publish and its `rename` fallback; every
  name of the protected set, in the case spelled differently on macOS and Windows, through a link to the
  folder (canonical parent), and the planted-`.bak` case; headless still replaces.
- Roots from the hello: a relative root refused by the app; the proxy turns a relative `--root` into an
  absolute one; confinement of plain paths, of paths inside bridged `options`, and of the `fit` `fdf` file;
  empty roots give the phase-1 message.
- Folder checks with injected owner ids: a 0755 folder is set to 0700, a folder of another owner and a
  linked folder are refused, missing levels are made 0700, a long socket path is refused.
- Transport tests with a fake host, on Unix socket and on TCP (the TCP tests run on every platform): stale
  socket and stale `mcp.json`, lock busy, the client names the folders when nothing listens and probes the
  second candidate, an HTTP first line closed, a wrong and a missing token closed, the constant-time
  comparison, a ninth connection refused, two clients, an oversize line, a second call in flight.
- The cancel race: a job answered `busy` or `cancelled` never runs afterwards; the 55 s backstop (clock
  injected).
- Lifecycle: turning off answers queued jobs and removes the files; off then on works; a client that
  never reads does not hold the join past its bound; the `replaced` notice closes the connection.
- The proxy: `ping` answered while a call waits; `notifications/cancelled` sends `cancel`; the 50 s
  timeout (clock injected) gives each of the two messages; a late reply is ignored; a lost reply is not
  retried; the `replaced` notice becomes one error; the hidden tools; end of stdin while waiting.
- A process test of `sheaf-mcp --attach --root` against the fake host, and the no-app error.

In `crates/app` (`app/ui_tests/live.rs`: the real proxy code in the test process, a real socket and a
temporary run folder; `Startup::default()` has no folder, so no test can touch the user's real ones):

- The undo name, and Edit > Undo reverts the whole call.
- A window created by the assistant is shown.
- The status-bar chip appears on connect and goes away when the assistant ends; the tooltip text and the
  Messages Log show a client name with bidi and control characters cleaned, and the tooltip names the
  folders.
- A mutating call waits under every dialog, the Fit more dialog included.
- A mutating call waits while a dialog is open or a cell is being edited, a read does not, and one
  mutating call runs per frame.
- A failed call is a status-bar warning and a Messages Log line, not a message box.
- Replacing the project (New, Open) drops the connections.
- The menu toggle creates and removes the files, and works twice.

On Windows in CI: the TCP and token tests above, the lock behaviour, and the case-insensitive protected
names. Still to be checked by hand on Windows before the ADR is accepted: a second user account cannot read
`mcp.json`, and a loopback bind raises no firewall prompt.

### 11.16 Setup

Start the app with the link on (`sheaf-app --mcp`) or tick Tools > AI Assistants (MCP). Then add the proxy
to the assistant with the folders it may use:

```
claude mcp add sheaf -- /path/to/sheaf-mcp --attach --root ~/Data
```

In `claude_desktop_config.json` use `"args":["--attach","--root","C:\\Users\\me\\Data"]`. When the app
is not listening, `tools/list` still works and a call says where it looked.

### 11.17 Implementation plan

Four commits, each with the workspace green (`cargo fmt --all`, strict Clippy, `cargo test --workspace`).

1. **Headless executor on `Document`.** `Ctx`, `Session` and `Policy` replace `&mut Server` in the tool
   functions; `resources` take a project; `mutates` and `headless_only` on the tool definitions;
   `Document::set_limit` and `generation` in core; `exec::run_on_document`; the headless `Server` runs on a
   `Document` (limit 0), `new_project` and `open_project` through `replace`; atomic `fit`; the undo
   labels and `link::sanitize`; the attached write rules and the protected set (the no-replace write is in
   `sheaf-mcp`'s `files`, modelled on `sheaf_io::save::write_atomically`, which stays private); roots only absolute when attached; `book` and `sheet` in five results; the
   per-connection change cap; the parity, rollback, write and headless tests. Any phase-1 test that relied
   on the non-atomic `fit` changes here.
2. **Link protocol, discovery, transport and proxy (library).** `sheaf_io::recovery::run_dir_candidates()`;
   `link` (`frame`, hello and operations, `discovery` with the folder checks, `transport` with the Unix
   socket and the TCP and token code, `host`, `client`); `getrandom`; `proxy`; the `--attach` option and
   thin `main.rs`; transport, lifecycle, proxy and process tests with a fake host.
3. **App integration.** The `sheaf-mcp` dependency and workspace entry; `crates/app/src/live.rs`; the
   menu tick; `--mcp` in `Startup`; the status-bar chip; `live_step` after the actions and before
   `sync_windows`; the generation check; the UI tests.
4. **Docs.** README, ROADMAP, the T25 status and its step 8 (which still says named pipe), `ui-command-map.md`,
   the `main.rs` comment of the app, and the status of ADR 0013 and of this section.

### 11.18 Current desktop context (`sheaf://app`, 2026-11-15)

The attached desktop host lists `sheaf://app` as `application/json`. Read it with the existing
`resources/read` request, for example `{"uri":"sheaf://app"}`. The MCP reply uses the usual
`contents` envelope; its `text` is a JSON object. The headless server and hosts without a UI-context
provider omit it from `resources/list` and return resource-not-found (`-32002`) when it is read.
The existing version-1 link `resource_read` operation carries the request; the wire format is unchanged.

```json
{
  "document_revision": 4,
  "project_generation": 0,
  "modified": true,
  "active_window": {"id": 1, "name": "Book1", "kind": "workbook"},
  "worksheet": {"name": "Sheet1", "index": 1, "column_count": 3, "row_count": 10},
  "active_layer": null,
  "selection": {
    "kind": "cells", "columns": ["B", "C"], "column_count": 2,
    "columns_truncated": false, "rows": {"from": 2, "to": 5},
    "active_cell": {"column": "B", "row": 2}
  },
  "data_reader": null,
  "editing": false,
  "dialog_open": false
}
```

`active_window` gives the short name usable in tool arguments and an informational model ID. It
is `null` when no project window is active. `worksheet` is present for a workbook, and `active_layer`
is present for a graph. Indices and rows count from **1**, with inclusive range endpoints. Continue
to address tools by workbook, sheet and column **names**; this resource does not change tool defaults
or make the selection an implicit input. Re-read it before acting when the owner may have changed focus.

Worksheet selections use `kind` = `cells`, `columns`, `rows`, `all` or `label`; no selection is `null`.
`columns` lists short names in worksheet order, including nonadjacent selected columns. At most
1,024 names are returned, with the full `column_count` and `columns_truncated` flag. A truncated
list is incomplete. `rows: null` for a column or whole-sheet selection denotes its full extent.
Cell blocks and row selections carry `rows: {from,to}`; a block also gives its current `active_cell`.
A `label` selection has no data rows: `label_row.kind` is `long_name`, `units`, `comments`, `formula`
or `parameter`; a parameter adds its `name`. Editor text and cell/label values are absent.

Graph selections give `kind` = `plot`, `label`, `shape`, `x_axis`, `y_axis`, `x_title`, `y_title` or
`legend`, plus `layer`. Plots, labels and shapes add a one-based `index`. A stored object index that
no longer exists yields `null`. `data_reader` gives `enabled` and the stored pin as `{layer,plot,row}`
or `null`; the row is the source worksheet row. The point is omitted when its plot/column/row no
longer exists. This reports UI state without changing graph selection or the reader.

The resource is read on the UI thread when its queued request runs. Its reader is created for that
frame but serializes only when this URI is requested, using the document after earlier queued jobs.
If a preceding job switched sheets before the grid redrew, that snapshot has `selection: null` rather
than applying the previous sheet's grid selection to the new sheet. `editing` and `dialog_open`
explain why mutating calls may be waiting; reads remain available under either condition. A read
leaves project data, revision, modified state, selection and undo history untouched. UI-only focus
changes can change this resource without changing `document_revision`. There is no subscription
or cached context; clients request it again for a fresh view.
