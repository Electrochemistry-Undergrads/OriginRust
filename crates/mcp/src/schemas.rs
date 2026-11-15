//! The tools' names, descriptions and input schemas, written by hand, each with an example call.

use serde_json::{Value, json};

pub struct Def {
    pub name: &'static str,
    pub description: &'static str,
    /// True when the tool changes neither the project nor any file.
    pub read_only: bool,
    pub schema: Value,
    pub example: Value,
}

fn t(ty: &str, d: &str) -> Value {
    json!({"type": ty, "description": d})
}

fn strs(d: &str) -> Value {
    json!({"type": "array", "items": {"type": "string"}, "description": d})
}

fn en(values: &[&str], d: &str) -> Value {
    json!({"type": "string", "enum": values, "description": d})
}

fn obj(props: Vec<(&str, Value)>, required: &[&str]) -> Value {
    let mut p = serde_json::Map::new();
    for (k, v) in props {
        p.insert(k.to_string(), v);
    }
    json!({"type": "object", "properties": p, "required": required, "additionalProperties": false})
}

const BOOK: &str = "Workbook: short name (Book1) or long name. Default: the first workbook.";
const SHEET: &str = "Sheet name. Default: the active sheet.";
const ROWS: &str = "Worksheet rows \"FROM:TO\", counted from 1; an end may be empty.";
const XRANGE: &str =
    "Interval of X \"FROM:TO\" (the first unbroken run of rows inside it). Use rows or x_range, not both.";

fn style_schema() -> Value {
    let n = |d: &str| t("number", d);
    obj(
        vec![
            ("x_title", t("string", "X axis title.")),
            ("y_title", t("string", "Y axis title.")),
            ("x_min", n("X axis minimum.")),
            ("x_max", n("X axis maximum.")),
            ("y_min", n("Y axis minimum.")),
            ("y_max", n("Y axis maximum.")),
            ("x_step", n("X axis major tick step.")),
            ("y_step", n("Y axis major tick step.")),
            ("y2_title", t("string", "Right Y axis title.")),
            ("y2_min", n("Right Y axis minimum.")),
            ("y2_max", n("Right Y axis maximum.")),
            ("y2_step", n("Right Y axis step.")),
            ("axis_colors", en(&["plot", "fixed"], "Whether axis colors follow the plots.")),
            ("width", n("Page width in inches, 0.1 to 100.")),
            ("height", n("Page height in inches, 0.1 to 100.")),
            ("line_width", n("Line width in points.")),
            ("colors", strs("Plot colors in legend order: names or #RRGGBB.")),
            ("labels", strs("Legend text of the plots, in legend order.")),
            ("no_legend", t("boolean", "Hide the legend.")),
            ("stack", t("string", "none, auto, auto:GAP or step:STEP.")),
            ("offsets", json!({"type": "array", "items": {"type": "number"}, "description": "Y offset of each plot."})),
            ("waterfall", json!({"type": "array", "items": {"type": "number"}, "description": "Waterfall offsets."})),
            ("no_y_tick_labels", t("boolean", "Hide the Y tick labels.")),
            ("no_y_ticks", t("boolean", "Hide the Y ticks.")),
            ("x_labels", t("string", "auto, numbers or a column name for the X tick labels.")),
            ("x_label_angle", n("Angle of the X tick labels in degrees.")),
            ("bar_gap", n("Gap between bars, percent.")),
            ("bar_overlap", n("Overlap of bars, percent.")),
            (
                "text_labels",
                json!({"type": "array", "description": "Text placed in the layer; x and y are fractions of the layer frame, (0,0) the top left.",
                       "items": obj(vec![("text", t("string", "The text.")), ("x", n("X fraction.")), ("y", n("Y fraction."))], &["text", "x", "y"])}),
            ),
            (
                "draw",
                json!({"type": "array", "description": "Shapes; coordinates are fractions of the layer frame.",
                       "items": obj(vec![
                           ("kind", en(&["line", "arrow", "rectangle", "ellipse"], "The shape.")),
                           ("x1", n("First corner X.")), ("y1", n("First corner Y.")),
                           ("x2", n("Second corner X.")), ("y2", n("Second corner Y.")),
                       ], &["kind", "x1", "y1", "x2", "y2"])}),
            ),
        ],
        &[],
    )
}

pub fn defs() -> Vec<Def> {
    let n = |d: &str| t("number", d);
    let s = |d: &str| t("string", d);
    let b = |d: &str| t("boolean", d);
    let ascii = obj(
        vec![
            ("delimiter", s("auto, comma, tab, semicolon, space or one character.")),
            ("header_lines", t("integer", "Lines before the first data row (default: detect).")),
            ("long_name_line", t("integer", "Header line with the long names.")),
            ("units_line", t("integer", "Header line with the units.")),
            ("comments_line", t("integer", "Header line with the comments.")),
            ("decimal_comma", b("Numbers use a decimal comma.")),
            ("thousands", en(&["auto", "off", "comma", "dot", "space", "apostrophe"], "Thousands separator.")),
            ("skip_rows", t("integer", "Data rows to skip at the start.")),
            ("max_rows", t("integer", "Read at most this many data rows (0: all).")),
            ("dates", en(&["detect", "text"], "Read dates as dates or keep them as text.")),
            ("date_order", en(&["mdy", "dmy"], "Order of day and month in dates.")),
        ],
        &[],
    );
    let chi = obj(
        vec![
            ("split_scans", b("One sheet per scan of a multi-scan file (default true).")),
            ("split_segments", b("Split segments (default true).")),
            ("add_milliamp", b("Add a current column in mA (default true).")),
            ("add_charge", b("Add a charge column (default true).")),
            ("invert_current", b("Invert the sign of the current.")),
            ("exact_time", b("Use the exact time stamps of the file (default true).")),
        ],
        &[],
    );
    let panels = obj(
        vec![
            ("shape", s("stack, or ROWSxCOLUMNS such as 2x2.")),
            (
                "per_panel",
                json!({"description": "Y columns per panel: one number or a list.", "oneOf": [{"type": "integer"}, {"type": "array", "items": {"type": "integer"}}]}),
            ),
            (
                "y",
                json!({"type": "array", "items": strs("Column names of one panel."), "description": "Y columns of each panel, as a list of lists (not with per_panel or the plot's y)."}),
            ),
            (
                "gap",
                json!({"description": "Gap between panels.", "oneOf": [{"type": "number"}, {"type": "array", "items": {"type": "number"}}]}),
            ),
            ("margins", json!({"type": "array", "items": {"type": "number"}, "description": "Panel margins."})),
            ("order", en(&["bottom_up", "top_down"], "Order of the panels.")),
            ("link", en(&["x", "y", "xy", "none"], "Which axes are linked.")),
        ],
        &["shape"],
    );
    let fit_bounds = json!({"type": "object", "description": "Bounds per parameter as [low, high]; null for no bound on a side.",
        "additionalProperties": {"type": "array", "items": {"type": ["number", "null"]}}});
    let numbers = json!({"type": "object", "additionalProperties": {"type": "number"}});
    vec![
        Def {
            name: "new_project",
            description: "Replace the project with an empty one. Unsaved work is lost.",
            read_only: false,
            schema: obj(vec![], &[]),
            example: json!({}),
        },
        Def {
            name: "open_project",
            description: "Open a .opj, .opju or .sheaf project, replacing the current project. Origin files are read in part: save_project over one keeps a .bak copy.",
            read_only: false,
            schema: obj(vec![("path", s("Project file under an allowed folder."))], &["path"]),
            example: json!({"path": "data/run1.opj"}),
        },
        Def {
            name: "save_project",
            description: "Save the project as .opj (default; with a .opj.sheaf companion when the .opj cannot hold everything) or .sheaf.",
            read_only: false,
            schema: obj(
                vec![("path", s("File to write. Default: the file the project was opened from or last saved to."))],
                &[],
            ),
            example: json!({"path": "out/analysis.sheaf"}),
        },
        Def {
            name: "import_files",
            description: "Import data files, folders or wildcards (CH Instruments text and .bin, CSV, other text) into the project. A file that fails is listed in errors; the others are imported.",
            read_only: false,
            schema: obj(
                vec![
                    ("files", strs("Files, folders, wildcards (in the file name) or project files.")),
                    (
                        "layout",
                        en(
                            &["books", "sheets"],
                            "One workbook per file (default) or one sheet per file in one workbook.",
                        ),
                    ),
                    ("book_name", s("Long name of the shared workbook of layout sheets.")),
                    ("format", en(&["auto", "chi", "ascii"], "Which reader.")),
                    ("file_name_in_comments", b("Put the file name in the column comments.")),
                    ("ascii", ascii),
                    ("chi", chi),
                    ("plot", b("Also make a default graph of each book.")),
                ],
                &["files"],
            ),
            example: json!({"files": ["cv/*.txt"], "layout": "books", "plot": true}),
        },
        Def {
            name: "list_contents",
            description: "The folders, workbooks, sheets, columns (designation, units, formula) and graphs of the project.",
            read_only: true,
            schema: obj(vec![("book", s(BOOK))], &[]),
            example: json!({}),
        },
        Def {
            name: "read_columns",
            description: "Values of columns for a row range. At most 10000 cells per reply; when truncated, pass next to continue.",
            read_only: true,
            schema: obj(
                vec![
                    ("book", s(BOOK)),
                    ("sheet", s(SHEET)),
                    ("columns", strs("Column names (short or long). Default: all.")),
                    ("from", t("integer", "First row, from 1.")),
                    ("to", t("integer", "Last row, inclusive. Default: the last row.")),
                    ("limit", t("integer", "Cells per reply, lower than 10000.")),
                ],
                &[],
            ),
            example: json!({"columns": ["A", "B"], "from": 1, "to": 100}),
        },
        Def {
            name: "set_column_values",
            description: "Fill a column from a formula of Sheaf's expression language, for example col(B)*1000. Nothing changes on error.",
            read_only: false,
            schema: obj(
                vec![
                    ("book", s(BOOK)),
                    ("sheet", s(SHEET)),
                    ("column", s("Column name (short or long).")),
                    ("formula", s("The formula.")),
                    ("rows", s(ROWS)),
                    (
                        "create",
                        obj(
                            vec![
                                (
                                    "designation",
                                    en(
                                        &["X", "Y", "Z", "YError", "XError", "Label", "Disregard", "Group", "Subject"],
                                        "Role of the new column.",
                                    ),
                                ),
                                ("long_name", s("Long name.")),
                                ("units", s("Units.")),
                            ],
                            &[],
                        ),
                    ),
                ],
                &["column", "formula"],
            ),
            example: json!({"column": "D", "formula": "col(B)*1000", "create": {"designation": "Y", "units": "uA"}}),
        },
        Def {
            name: "plot",
            description: "Make a graph from columns, or, with graph, restyle an existing graph (template, y and y2 are then refused). Returns the graph name; use export_graph to see it.",
            read_only: false,
            schema: obj(
                vec![
                    (
                        "template",
                        s(
                            "line, scatter, line+symbol, column, ... (the names of sheaf plot --template). Default: what the data suggests.",
                        ),
                    ),
                    ("book", s(BOOK)),
                    ("sheet", s(SHEET)),
                    ("y", strs("Y columns. Default: the sheet's Y columns.")),
                    ("y2", strs("Columns on the right Y axis (double-Y).")),
                    ("graph", s("Style this existing graph instead of making one.")),
                    ("panels", panels),
                    (
                        "overlay_files",
                        strs(
                            "Import each file and overlay its Y columns in one graph (not with book, sheet or graph).",
                        ),
                    ),
                    ("style", style_schema()),
                    (
                        "options",
                        crate::bridge::schema(
                            &crate::bridge::subcommand(&["plot"]).expect("plot command"),
                            &crate::bridge::session_filter("plot"),
                        ),
                    ),
                    (
                        "size",
                        obj(
                            vec![("width_in", n("Page width in inches.")), ("height_in", n("Page height in inches."))],
                            &[],
                        ),
                    ),
                ],
                &[],
            ),
            example: json!({"template": "line", "y": ["B"], "style": {"x_title": "E / V", "y_title": "I / A"}}),
        },
        Def {
            name: "export_graph",
            description: "Render a graph to a file (png, svg, pdf, eps, tiff, emf, jpg, bmp) and/or return it as a PNG image to look at.",
            read_only: false,
            schema: obj(
                vec![
                    ("graph", s("Graph name. Default: the only graph.")),
                    ("path", s("File to write; the extension picks the format.")),
                    (
                        "format",
                        en(
                            &["png", "svg", "pdf", "eps", "tiff", "emf", "jpg", "bmp"],
                            "Format; default: the extension of path, else png.",
                        ),
                    ),
                    ("dpi", n("Resolution, 36 to 1200 (default 300).")),
                    ("width", s("Image width with unit: 3.5in, 9cm, 800px or 50%.")),
                    ("height", s("Image height with unit.")),
                    ("margin", en(&["auto", "border", "tight", "page", "tight-in-page"], "Margin mode.")),
                    ("return_image", b("Also return the PNG as an image (default: true when path is absent).")),
                ],
                &[],
            ),
            example: json!({"graph": "Graph1", "path": "out/cv.png", "dpi": 300, "return_image": true}),
        },
        Def {
            name: "export_data",
            description: "Write a sheet as delimited text (comma for csv, tab for tsv).",
            read_only: false,
            schema: obj(
                vec![
                    ("book", s(BOOK)),
                    ("sheet", s(SHEET)),
                    ("path", s("File to write.")),
                    ("format", en(&["csv", "tsv"], "Default: tabs for .tsv and .txt, commas otherwise.")),
                ],
                &["path"],
            ),
            example: json!({"path": "out/book1.csv"}),
        },
        Def {
            name: "list_analysis_tools",
            description: "Every analysis tool analyze accepts (all the tools of `sheaf analyze`), with their options, types and defaults. Tools marked bridged also carry a JSON Schema.",
            read_only: true,
            schema: obj(vec![], &[]),
            example: json!({}),
        },
        Def {
            name: "analyze",
            description: "Run an analysis tool on Y columns: any tool of `sheaf analyze` (integrate, smooth, fft_filter, ttest, anova, mannwhitney, histogram, normality, corr, ...). Results go to new columns next to the input or to a new sheet; the reply says where and carries any table. Names are snake_case; options are keys of list_analysis_tools (snake_case or kebab-case).",
            read_only: false,
            schema: obj(
                vec![
                    ("tool", s("Tool name from list_analysis_tools.")),
                    (
                        "options",
                        json!({"type": "object", "description": "The tool's options (see list_analysis_tools)."}),
                    ),
                    ("book", s(BOOK)),
                    ("sheet", s(SHEET)),
                    ("columns", strs("Y columns to work on. Default: the sheet's Y columns.")),
                    ("rows", s(ROWS)),
                    ("x_range", s(XRANGE)),
                    (
                        "output",
                        en(&["next_to_input"], "Where results go. Tools that write a table always use a new sheet."),
                    ),
                ],
                &["tool"],
            ),
            example: json!({"tool": "integrate", "options": {"cumulative": true}, "columns": ["B"], "x_range": "0.2:0.6"}),
        },
        Def {
            name: "sort_sheet",
            description: "Sort the rows of a sheet by a column.",
            read_only: false,
            schema: obj(
                vec![
                    ("book", s(BOOK)),
                    ("sheet", s(SHEET)),
                    ("key", s("Column to sort by.")),
                    ("descending", b("Sort from large to small.")),
                ],
                &["key"],
            ),
            example: json!({"key": "A", "descending": false}),
        },
        Def {
            name: "filter_sheet",
            description: "Keep the rows of a sheet for which a formula is true, in a new sheet unless in_place.",
            read_only: false,
            schema: obj(
                vec![
                    ("book", s(BOOK)),
                    ("sheet", s(SHEET)),
                    ("condition", s("Formula, for example col(B)>0.5.")),
                    ("in_place", b("Remove the other rows from the sheet itself.")),
                ],
                &["condition"],
            ),
            example: json!({"condition": "col(B)>0.5"}),
        },
        Def {
            name: "list_fit_models",
            description: "The built-in fit functions with their parameters, and the special model names.",
            read_only: true,
            schema: obj(vec![("category", s("Only this category."))], &[]),
            example: json!({"category": "Peak"}),
        },
        Def {
            name: "fit",
            description: "Fit a model to Y columns (one fit per column), compare two models, rank models, fit globally or regress on several columns. Results go to new sheets; the reply has parameters, errors and statistics.",
            read_only: false,
            schema: obj(
                vec![
                    (
                        "model",
                        s("linear, linearx, poly:N or a built-in function name (list_fit_models). Default linear."),
                    ),
                    ("formula", s("Fit this formula instead of model; its parameters are in params.")),
                    ("params", strs("Parameter names of formula.")),
                    ("fdf", s("An Origin .FDF file that defines the function (under an allowed folder).")),
                    ("book", s(BOOK)),
                    ("sheet", s(SHEET)),
                    ("y", strs("Y columns to fit. Default: the sheet's Y columns.")),
                    ("rows", s(ROWS)),
                    ("x_range", s(XRANGE)),
                    ("init", numbers.clone()),
                    ("fix", numbers),
                    ("bounds", fit_bounds),
                    ("weighting", en(&["none", "instrumental", "statistical", "direct"], "How points are weighted.")),
                    ("yerr", strs("Error or weight columns for instrumental or direct weighting.")),
                    ("xerr", s("X error column (linearx).")),
                    ("confidence", n("Confidence level of parameter limits, between 0 and 1 (default 0.95).")),
                    ("bands", en(&["none", "confidence", "prediction", "both"], "Bands around the curve.")),
                    ("band_level", n("Band level in percent (default 95).")),
                    ("max_iterations", t("integer", "Iteration limit (default 400).")),
                    ("tolerance", n("Convergence tolerance (default 1e-9).")),
                    ("scale_errors", b("Scale parameter errors with the reduced chi-square (default true).")),
                    ("compare_with", s("Compare model with this model (F-test, AIC, BIC).")),
                    ("rank", strs("Rank these built-in functions on the data.")),
                    ("global", obj(vec![("share", strs("Parameters shared by all columns."))], &[])),
                    ("regress", strs("Multiple linear regression of the first y on these X columns.")),
                    ("plot", s("Also render the first fit as an image at this path.")),
                    (
                        "options",
                        crate::bridge::schema(
                            &crate::bridge::subcommand(&["fit"]).expect("fit command"),
                            &crate::bridge::session_filter("fit"),
                        ),
                    ),
                ],
                &[],
            ),
            example: json!({"model": "Gauss", "y": ["B"], "x_range": "0.2:0.6", "init": {"xc": 0.4}, "bounds": {"w": [0, null]}}),
        },
    ]
}

/// Checks an example against a schema: required fields, known fields and basic types. The
/// schemas use only these features.
#[cfg(test)]
pub fn check(schema: &Value, value: &Value, at: &str) -> Result<(), String> {
    let ty_ok = |ty: &str, v: &Value| match ty {
        "string" => v.is_string(),
        "number" => v.is_number(),
        "integer" => v.is_i64() || v.is_u64(),
        "boolean" => v.is_boolean(),
        "array" => v.is_array(),
        "object" => v.is_object(),
        "null" => v.is_null(),
        _ => false,
    };
    if let Some(ty) = schema.get("type") {
        let ok = match ty {
            Value::String(t) => ty_ok(t, value),
            Value::Array(ts) => ts.iter().filter_map(Value::as_str).any(|t| ty_ok(t, value)),
            _ => true,
        };
        if !ok {
            return Err(format!("{at}: {value} is not of type {ty}"));
        }
    }
    if let Some(choices) = schema.get("enum").and_then(Value::as_array)
        && !choices.contains(value)
    {
        return Err(format!("{at}: {value} is not one of {choices:?}"));
    }
    if let Some(alts) = schema.get("oneOf").and_then(Value::as_array)
        && !alts.iter().any(|a| check(a, value, at).is_ok())
    {
        return Err(format!("{at}: {value} matches none of the alternatives"));
    }
    if let (Some(props), Some(map)) = (schema.get("properties").and_then(Value::as_object), value.as_object()) {
        for req in schema.get("required").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str) {
            if !map.contains_key(req) {
                return Err(format!("{at}: missing required field {req}"));
            }
        }
        for (k, v) in map {
            match props.get(k) {
                Some(s) => check(s, v, &format!("{at}.{k}"))?,
                None => return Err(format!("{at}: unknown field {k}")),
            }
        }
    }
    if let (Some(items), Some(list)) = (schema.get("items"), value.as_array()) {
        for (i, v) in list.iter().enumerate() {
            check(items, v, &format!("{at}[{i}]"))?;
        }
    }
    if let (Some(extra), Some(map)) = (schema.get("additionalProperties").filter(|a| a.is_object()), value.as_object())
    {
        for (k, v) in map {
            check(extra, v, &format!("{at}.{k}"))?;
        }
    }
    Ok(())
}
