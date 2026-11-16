//! The analysis tools `analyze` accepts: their names, options and how the options become a
//! `sheaf_ops::analyze::Job`. The numbers come from the same code as `sheaf analyze`.

use crate::bridge::{self, Filter};
use anyhow::{Result, anyhow, bail};
use serde_json::{Map, Value, json};
use sheaf_core::analysis::{self, BaselineSpec, InterpSpec, Tool};
use sheaf_ops::analyze::{Job, OperandSpec, parse_pair};

/// One option of a tool, as `list_analysis_tools` shows it.
pub struct OptionDesc {
    pub name: &'static str,
    /// `number`, `integer`, `boolean`, `string`, `[number]` or `[string]`.
    pub ty: &'static str,
    pub default: Value,
    pub description: &'static str,
    pub required: bool,
    pub choices: &'static [&'static str],
}

pub struct ToolDesc {
    pub name: &'static str,
    pub summary: &'static str,
    pub options: Vec<OptionDesc>,
    /// The option that names a second column, when the tool takes one.
    pub second_column: Option<&'static str>,
}

fn num(name: &'static str, default: f64, description: &'static str) -> OptionDesc {
    OptionDesc { name, ty: "number", default: json!(default), description, required: false, choices: &[] }
}

fn int(name: &'static str, default: u64, description: &'static str) -> OptionDesc {
    OptionDesc { name, ty: "integer", default: json!(default), description, required: false, choices: &[] }
}

fn flag(name: &'static str, default: bool, description: &'static str) -> OptionDesc {
    OptionDesc { name, ty: "boolean", default: json!(default), description, required: false, choices: &[] }
}

fn choice(
    name: &'static str,
    default: &'static str,
    choices: &'static [&'static str],
    description: &'static str,
) -> OptionDesc {
    OptionDesc { name, ty: "string", default: json!(default), description, required: false, choices }
}

fn maybe(name: &'static str, ty: &'static str, description: &'static str) -> OptionDesc {
    OptionDesc { name, ty, default: Value::Null, description, required: false, choices: &[] }
}

fn needed(name: &'static str, ty: &'static str, description: &'static str) -> OptionDesc {
    OptionDesc { name, ty, default: Value::Null, description, required: true, choices: &[] }
}

/// The largest value of an integer option such as a window, an order or a polynomial degree.
const OPTION_CAP: usize = 10_000;
/// The largest number of rows an option may ask for: points of a result, a decimation factor.
const ROW_CAP: usize = 10_000_000;

const MATH_OPS: &[&str] = &["add", "subtract", "multiply", "divide", "power"];

fn operand_options(column: bool) -> Vec<OptionDesc> {
    let mut v = vec![
        choice("op", "add", MATH_OPS, "The operation: y op operand."),
        maybe("value", "number", "The operand as a number."),
        maybe("reference", "string", "The operand as a column (short or long name): a reference curve."),
        maybe("ref_book", "string", "Workbook of the reference column (default: the same book)."),
        maybe("ref_sheet", "string", "Sheet of the reference column (default: the same sheet)."),
    ];
    if !column {
        v.push(flag("common", false, "Only the X range that the data and the reference both cover (reference only)."));
    }
    v
}

pub fn registry() -> Vec<ToolDesc> {
    let mut v = Vec::new();
    let mut add = |name, summary, options, second_column| v.push(ToolDesc { name, summary, options, second_column });
    add(
        "integrate",
        "Area under each curve: the area, and by default the running integral in a new column. With ranges, a table of areas.",
        vec![
            choice("area", "mathematical", &["mathematical", "absolute"], "Signed or absolute area."),
            flag("end_point_baseline", false, "Subtract the straight line through the first and last point first."),
            flag("cumulative", true, "Also write the running integral (not with ranges)."),
            maybe("ranges", "[string]", "X intervals \"FROM:TO\"; gives a table of areas."),
            maybe("row_ranges", "[string]", "Row intervals \"FROM:TO\"; gives a table of areas."),
        ],
        None,
    );
    add(
        "math",
        "Curve math: y op value, or y op a reference curve (interpolated onto the X of y).",
        operand_options(false),
        Some("reference"),
    );
    add(
        "column_math",
        "Column math: row by row, y op value or y op a column; no X.",
        operand_options(true),
        Some("reference"),
    );
    add(
        "subtract_ref",
        "Subtract a reference curve from each curve.",
        vec![
            needed("reference", "string", "The reference column (short or long name)."),
            maybe("ref_book", "string", "Workbook of the reference column."),
            maybe("ref_sheet", "string", "Sheet of the reference column."),
            flag("common", false, "Only the X range that the data and the reference both cover."),
        ],
        Some("reference"),
    );
    add(
        "smooth",
        "Smooth each curve into a new column.",
        vec![
            choice(
                "method",
                "savitzky_golay",
                &["savitzky_golay", "average", "weighted_average", "percentile", "fft", "lowess", "loess", "binomial"],
                "The smoothing method.",
            ),
            int("points", 5, "Points of the window."),
            int("poly", 2, "Savitzky-Golay: polynomial order."),
            num("percent", 50.0, "Percentile filter: the percentile, 0 to 100."),
            num("span", 0.1, "Lowess and Loess: share of the points in each local fit, 0 to 1; 0 uses points."),
            int("order", 1, "Binomial: how often the filter is applied."),
            choice(
                "boundary",
                "none",
                &["none", "reflect", "repeat", "periodic", "extrapolate"],
                "Fill where the window sticks out.",
            ),
        ],
        None,
    );
    add(
        "differentiate",
        "Derivative of each curve into a new column.",
        vec![
            int("order", 1, "Derivative order, 1 to 9."),
            flag("smooth", false, "Use a local polynomial fit instead of differences."),
            int("poly", 2, "Polynomial order of the local fit."),
            int("points", 20, "Points of the local fit window."),
        ],
        None,
    );
    add(
        "normalize",
        "Normalize each curve into a new column.",
        vec![
            choice(
                "method",
                "max",
                &[
                    "max", "range", "range100", "z_score", "min", "mean", "median", "sd", "norm", "mode", "sum",
                    "divide",
                ],
                "How to normalize.",
            ),
            maybe("value", "number", "The divisor for method divide."),
        ],
        None,
    );
    add(
        "baseline",
        "Subtract a baseline from each curve: its smallest value unless one of the options is given.",
        vec![
            maybe("constant", "number", "Subtract this constant."),
            flag("end_points", false, "Subtract the line through the first and the last point."),
            maybe("line", "[number]", "Subtract the line through [x1, y1, x2, y2]."),
        ],
        None,
    );
    add(
        "interpolate",
        "Interpolate each curve onto evenly spaced X.",
        vec![
            choice("method", "linear", &["linear", "spline", "akima", "bspline"], "The interpolation method."),
            int("points", 100, "Points of the result."),
            maybe("x_min", "number", "First X of the result (with x_max)."),
            maybe("x_max", "number", "Last X of the result (with x_min)."),
            num("smoothing", 0.0, "B-spline smoothing factor."),
        ],
        None,
    );
    add(
        "find_peaks",
        "Find the peaks of each curve; a table with position, height and more.",
        vec![
            choice("direction", "both", &["both", "positive", "negative"], "Which peaks."),
            int("points", 2, "Neighbours compared on each side."),
            maybe("min_height", "number", "Keep peaks at least this high (default 0)."),
            maybe("count", "integer", "Keep this many of the highest peaks."),
            flag("all", false, "Keep every candidate (no height filter)."),
        ],
        None,
    );
    add(
        "statistics",
        "Descriptive statistics of each column; a table.",
        vec![
            flag("all", false, "Every quantity."),
            maybe("add", "[string]", "Further quantities: iqr, variance, skewness, kurtosis, cv, limits."),
            num("level", 0.95, "Confidence level of the limits of the mean."),
        ],
        None,
    );
    add(
        "fft",
        "Fast Fourier transform of each curve into a new sheet.",
        vec![
            maybe("interval", "number", "Sampling interval (default: from X)."),
            choice(
                "window",
                "rectangle",
                &["rectangle", "welch", "triangular", "bartlett", "hanning", "hamming", "blackman"],
                "The window.",
            ),
            choice("correction", "amplitude", &["none", "amplitude", "power"], "Window correction."),
            flag("normalize", false, "Divide by N."),
            flag("two_sided", false, "All N frequencies."),
            flag("shift", true, "Two-sided: negative frequencies first."),
            flag("unwrap", true, "Unwrap the phase."),
            flag("science", false, "Positive exponent sign."),
            choice("power", "msa", &["msa", "ssa", "tisa"], "Power spectrum normalization."),
        ],
        None,
    );
    add("hilbert", "Hilbert transform of each curve.", Vec::new(), None);
    add(
        "envelope",
        "Upper and lower envelope of each curve.",
        vec![
            choice("kind", "upper", &["upper", "lower", "both"], "Which envelope."),
            num("npts", 2.0, "Points on each side of an extreme."),
        ],
        None,
    );
    add(
        "level_crossing",
        "X values where each curve crosses a level.",
        vec![
            needed("level", "number", "The Y level."),
            choice("row", "any", &["any", "left", "right"], "The row given for a crossing between two rows."),
            int("min_gap", 1, "Drop a pair of crossings closer than this many rows (1 keeps all)."),
            flag("details", false, "Also write direction, row and Y of each crossing."),
        ],
        None,
    );
    add(
        "median_filter",
        "Running median of each curve into a new column.",
        vec![
            int("points", 5, "Points of the window (an even number acts like the next odd one)."),
            choice("edge", "shrink", &["shrink", "repeat", "reflect"], "The window at the ends."),
        ],
        None,
    );
    add(
        "decimate",
        "Keep every n-th point of each curve.",
        vec![
            int("factor", 2, "Keep every factor-th point."),
            choice("filter", "none", &["none", "ave", "fir"], "Filter applied first."),
            int("order", 30, "FIR filter order (8 to 60)."),
        ],
        None,
    );
    add(
        "translate",
        "Shift each curve in X or Y.",
        vec![
            choice("axis", "x", &["x", "y"], "Which axis."),
            maybe("by", "number", "Shift by this amount."),
            maybe("at", "number", "Move the point at this X (with to)..."),
            maybe("to", "number", "...to this value."),
        ],
        None,
    );
    add(
        "subtract_line",
        "Subtract the straight line through two points.",
        vec![
            needed("x1", "number", "X of the first point."),
            maybe("y1", "number", "Y of the first point (default: the curve's value at x1)."),
            needed("x2", "number", "X of the second point."),
            maybe("y2", "number", "Y of the second point (default: the curve's value at x2)."),
        ],
        None,
    );
    add("polygon_area", "Area of the polygon each curve encloses.", Vec::new(), None);
    add(
        "xyz_area",
        "Area of the surface through scattered X, Y, Z points: the Delaunay triangulation of X and Y lifted to Z (rows with the same X and Y are merged with the mean Z). Name the three columns X, Y, Z in this order; default: the sheet's first Z column with its Y and X.",
        Vec::new(),
        None,
    );
    add(
        "intersect",
        "Where two curves cross (name two columns).",
        vec![flag("touching", false, "Count points where the curves only touch.")],
        None,
    );
    v
}

/// The command-line names of tools whose hand-mapped version has another name.
const ALIASES: [(&str, &str); 2] = [("peaks", "find_peaks"), ("stats", "statistics")];

/// The options of a bridged tool the server does not take from the tool's `options`: the input,
/// the selection (`columns`, `book`, `sheet`, `rows`, `x_range` of the call), the files the command
/// line writes (`export`, `save`: use `export_data` and `save_project`) and the read options.
pub fn bridge_filter() -> Filter {
    Filter::excluding(["input", "y", "book", "sheet", "rows", "x_range", "export", "save"])
        .and_excluding(bridge::ids_of::<sheaf_ops::args::ReadOptions>())
}

/// A tool of `sheaf analyze` that has no hand-mapped version: its options come from clap.
pub struct Bridged {
    /// The name `analyze` takes: the command-line name with `_` for `-`.
    pub name: String,
    /// The subcommand of `sheaf analyze`.
    pub command: String,
    pub summary: String,
    pub schema: Value,
    pub second_column: Option<&'static str>,
}

fn hand_name_of(registry: &[ToolDesc], snake: &str) -> Option<&'static str> {
    let snake = ALIASES.iter().find(|(c, _)| *c == snake).map_or(snake, |(_, h)| h);
    registry.iter().find(|t| t.name == snake).map(|t| t.name)
}

/// Every subcommand of `sheaf analyze` that is not hand-mapped, in the command line's order.
pub fn bridged() -> Vec<Bridged> {
    let reg = registry();
    let analyze = bridge::subcommand(&["analyze"]).expect("the analyze command exists");
    let filter = bridge_filter();
    analyze
        .get_subcommands()
        .filter(|c| c.get_name() != "help")
        .filter(|c| hand_name_of(&reg, &c.get_name().replace('-', "_")).is_none())
        .map(|c| {
            let cmd = bridge::subcommand(&["analyze", c.get_name()]).expect("listed above");
            let schema = bridge::schema(&cmd, &filter);
            let about = c.get_about().map(|a| bridge::tool_text(&a.to_string())).unwrap_or_default();
            let summary = about.split_once(". ").map_or(about.as_str(), |(a, _)| a).trim_end_matches('.').to_string();
            let second_column =
                ["other", "response", "imaginary", "ref"].into_iter().find(|k| schema["properties"].get(*k).is_some());
            Bridged {
                name: c.get_name().replace('-', "_"),
                command: c.get_name().to_string(),
                summary,
                schema,
                second_column,
            }
        })
        .collect()
}

/// How `analyze` runs a tool name.
pub enum Which {
    /// A hand-mapped tool (the name as the registry has it).
    Hand(&'static str),
    /// A tool built from the command line's options.
    Bridged(Bridged),
}

/// Finds a tool by name: `_` or `-`, any case, the command-line names `peaks` and `stats` included.
pub fn resolve(name: &str) -> Result<Which> {
    let snake = name.trim().to_ascii_lowercase().replace('-', "_");
    let reg = registry();
    if let Some(h) = hand_name_of(&reg, &snake) {
        return Ok(Which::Hand(h));
    }
    let mut all = bridged();
    if let Some(i) = all.iter().position(|b| b.name == snake) {
        return Ok(Which::Bridged(all.swap_remove(i)));
    }
    let mut names: Vec<String> = reg.iter().map(|t| t.name.to_string()).collect();
    names.extend(all.iter().map(|b| b.name.clone()));
    bail!("unknown analysis tool \"{name}\" (tools: {})", names.join(", "))
}

/// The options of a bridged tool as `list_analysis_tools` shows them.
fn bridged_options(schema: &Value) -> Vec<Value> {
    let required: Vec<&str> =
        schema["required"].as_array().map(|r| r.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    let Some(props) = schema["properties"].as_object() else { return Vec::new() };
    props
        .iter()
        .map(|(name, p)| {
            let ty = match p["type"].as_str() {
                Some("array") => format!("[{}]", p["items"]["type"].as_str().unwrap_or("string")),
                Some(t) => t.to_string(),
                None => "string".to_string(),
            };
            let mut d = json!({
                "name": name, "type": ty, "default": p.get("default").cloned().unwrap_or(Value::Null),
                "description": p["description"], "required": required.contains(&name.as_str()),
            });
            let choices = if p["type"] == "array" { &p["items"]["enum"] } else { &p["enum"] };
            if choices.is_array() {
                d["choices"] = choices.clone();
            }
            d
        })
        .collect()
}

/// The registry as `list_analysis_tools` returns it: the hand-mapped tools, then every other tool
/// of `sheaf analyze` with the options of its command-line definition. A bridged tool also carries
/// `bridged: true` and its JSON Schema.
pub fn describe() -> Value {
    let mut tools: Vec<Value> = registry()
        .into_iter()
        .map(|t| {
            let options: Vec<Value> = t
                .options
                .iter()
                .map(|o| {
                    let mut d = json!({
                        "name": o.name, "type": o.ty, "default": o.default,
                        "description": o.description, "required": o.required,
                    });
                    if !o.choices.is_empty() {
                        d["choices"] = json!(o.choices);
                    }
                    d
                })
                .collect();
            json!({"name": t.name, "summary": t.summary, "options": options, "second_column": t.second_column})
        })
        .collect();
    tools.extend(bridged().into_iter().map(|b| {
        json!({
            "name": b.name, "summary": b.summary, "options": bridged_options(&b.schema),
            "second_column": b.second_column, "bridged": true, "input_schema": b.schema,
        })
    }));
    json!({ "tools": tools })
}

struct Opts<'a> {
    tool: &'a str,
    map: &'a Map<String, Value>,
}

impl Opts<'_> {
    fn get(&self, name: &str) -> Option<&Value> {
        self.map.get(name).filter(|v| !v.is_null())
    }

    fn bad(&self, name: &str, want: &str) -> anyhow::Error {
        anyhow!("option \"{name}\" of {} must be {want}", self.tool)
    }

    fn opt_num(&self, name: &str) -> Result<Option<f64>> {
        match self.get(name) {
            None => Ok(None),
            Some(v) => v.as_f64().filter(|x| x.is_finite()).map(Some).ok_or_else(|| self.bad(name, "a number")),
        }
    }

    fn num(&self, name: &str, default: f64) -> Result<f64> {
        Ok(self.opt_num(name)?.unwrap_or(default))
    }

    /// A whole number of at most `max`, so that no option can size an endless loop or a huge column.
    fn opt_usize_up_to(&self, name: &str, max: usize) -> Result<Option<usize>> {
        match self.get(name) {
            None => Ok(None),
            Some(v) => {
                let n = v
                    .as_u64()
                    .and_then(|x| usize::try_from(x).ok())
                    .ok_or_else(|| self.bad(name, "a whole number, 0 or more"))?;
                if n > max {
                    return Err(self.bad(name, &format!("a whole number of at most {max}")));
                }
                Ok(Some(n))
            }
        }
    }

    fn opt_usize(&self, name: &str) -> Result<Option<usize>> {
        self.opt_usize_up_to(name, OPTION_CAP)
    }

    fn usize_up_to(&self, name: &str, default: usize, max: usize) -> Result<usize> {
        Ok(self.opt_usize_up_to(name, max)?.unwrap_or(default))
    }

    fn usize(&self, name: &str, default: usize) -> Result<usize> {
        self.usize_up_to(name, default, OPTION_CAP)
    }

    fn flag(&self, name: &str, default: bool) -> Result<bool> {
        match self.get(name) {
            None => Ok(default),
            Some(v) => v.as_bool().ok_or_else(|| self.bad(name, "true or false")),
        }
    }

    fn opt_str(&self, name: &str) -> Result<Option<String>> {
        match self.get(name) {
            None => Ok(None),
            Some(v) => v.as_str().map(|s| Some(s.to_string())).ok_or_else(|| self.bad(name, "a string")),
        }
    }

    fn pick(&self, name: &str, default: &str, choices: &[&str]) -> Result<String> {
        let Some(v) = self.get(name) else { return Ok(default.to_string()) };
        let text = v.as_str().map(|s| s.trim().to_ascii_lowercase());
        match text {
            Some(t) if choices.contains(&t.as_str()) => Ok(t),
            _ => Err(self.bad(name, &format!("one of {}", choices.join(", ")))),
        }
    }

    fn strings(&self, name: &str) -> Result<Vec<String>> {
        match self.get(name) {
            None => Ok(Vec::new()),
            Some(Value::Array(a)) => a
                .iter()
                .map(|v| v.as_str().map(str::to_string).ok_or_else(|| self.bad(name, "a list of strings")))
                .collect(),
            Some(_) => Err(self.bad(name, "a list of strings")),
        }
    }

    fn numbers(&self, name: &str) -> Result<Vec<f64>> {
        match self.get(name) {
            None => Ok(Vec::new()),
            Some(Value::Array(a)) => a
                .iter()
                .map(|v| v.as_f64().filter(|x| x.is_finite()).ok_or_else(|| self.bad(name, "a list of numbers")))
                .collect(),
            Some(_) => Err(self.bad(name, "a list of numbers")),
        }
    }

    fn required_num(&self, name: &str) -> Result<f64> {
        self.opt_num(name)?.ok_or_else(|| anyhow!("{} needs the option \"{name}\"", self.tool))
    }
}

fn operand(o: &Opts) -> Result<(analysis::CurveOp, OperandSpec)> {
    let op = match o.pick("op", "add", MATH_OPS)?.as_str() {
        "add" => analysis::CurveOp::Add,
        "subtract" => analysis::CurveOp::Subtract,
        "multiply" => analysis::CurveOp::Multiply,
        "divide" => analysis::CurveOp::Divide,
        _ => analysis::CurveOp::Power,
    };
    let spec = OperandSpec {
        value: o.opt_num("value")?,
        reference: o.opt_str("reference")?,
        ref_file: None,
        ref_book: o.opt_str("ref_book")?,
        ref_sheet: o.opt_str("ref_sheet")?,
        common: o.flag("common", false)?,
    };
    Ok((op, spec))
}

/// Builds the job a tool name and its options stand for.
pub fn build(name: &str, options: &Map<String, Value>) -> Result<Job> {
    let reg = registry();
    let Some(desc) = reg.iter().find(|t| t.name == name) else {
        let names: Vec<&str> = reg.iter().map(|t| t.name).collect();
        bail!("unknown analysis tool \"{name}\" (tools: {})", names.join(", "));
    };
    for key in options.keys() {
        if !desc.options.iter().any(|o| o.name == key) {
            let names: Vec<&str> = desc.options.iter().map(|o| o.name).collect();
            let have = if names.is_empty() { "none".to_string() } else { names.join(", ") };
            bail!("unknown option \"{key}\" for {name} (options: {have})");
        }
    }
    let o = Opts { tool: name, map: options };
    Ok(match name {
        "integrate" => {
            let ranges = o.strings("ranges")?;
            let row_ranges = o.strings("row_ranges")?;
            let baseline = o.flag("end_point_baseline", false)?;
            if ranges.is_empty() && row_ranges.is_empty() {
                let area_type = if o.pick("area", "mathematical", &["mathematical", "absolute"])? == "absolute" {
                    analysis::AreaType::Absolute
                } else {
                    analysis::AreaType::Mathematical
                };
                Job::Tool(Tool::Integrate(analysis::IntegrateSpec {
                    area_type,
                    end_point_baseline: baseline,
                    cumulative: o.flag("cumulative", true)?,
                }))
            } else {
                if o.get("area").is_some() || o.get("cumulative").is_some() {
                    bail!(
                        "area and cumulative do not apply to ranges: the result table always has both the signed and \
                         the absolute area and no running integral"
                    );
                }
                let mut list = Vec::new();
                for text in &ranges {
                    let (a, b) = parse_pair::<f64>(text, "X range", "0.2:0.6")?;
                    list.push(analysis::RowRange::XBetween(a, b));
                }
                for text in &row_ranges {
                    let (a, b) = parse_pair::<usize>(text, "row range", "120:480")?;
                    list.push(analysis::RowRange::Rows(a, b));
                }
                Job::Tool(Tool::IntegrateRanges(analysis::IntegrateRangesSpec {
                    ranges: list,
                    end_point_baseline: baseline,
                }))
            }
        }
        "math" | "column_math" => {
            let (op, operand) = operand(&o)?;
            Job::Math { op, operand, column: name == "column_math" }
        }
        "subtract_ref" => {
            let (_, operand) = operand(&o)?;
            if operand.reference.is_none() {
                bail!("subtract_ref needs the option \"reference\" (use math with a value for a constant)");
            }
            Job::Math { op: analysis::CurveOp::Subtract, operand, column: false }
        }
        "smooth" => {
            let span = o.num("span", 0.1)?;
            let local = if span > 0.0 { analysis::LocalSpan::Proportion(span) } else { analysis::LocalSpan::Points };
            let method = match o
                .pick(
                    "method",
                    "savitzky_golay",
                    &[
                        "savitzky_golay",
                        "average",
                        "weighted_average",
                        "percentile",
                        "fft",
                        "lowess",
                        "loess",
                        "binomial",
                    ],
                )?
                .as_str()
            {
                "savitzky_golay" => analysis::SmoothMethod::SavitzkyGolay { poly_order: o.usize("poly", 2)? },
                "average" => analysis::SmoothMethod::AdjacentAveraging { weighted: false },
                "weighted_average" => analysis::SmoothMethod::AdjacentAveraging { weighted: true },
                "percentile" => analysis::SmoothMethod::Percentile { percent: o.num("percent", 50.0)? },
                "fft" => analysis::SmoothMethod::FftFilter,
                "lowess" => analysis::SmoothMethod::Lowess { span: local },
                "loess" => analysis::SmoothMethod::Loess { span: local },
                _ => analysis::SmoothMethod::Binomial { order: o.usize("order", 1)? },
            };
            let boundary =
                match o.pick("boundary", "none", &["none", "reflect", "repeat", "periodic", "extrapolate"])?.as_str() {
                    "none" => analysis::Boundary::None,
                    "reflect" => analysis::Boundary::Reflect,
                    "repeat" => analysis::Boundary::Repeat,
                    "periodic" => analysis::Boundary::Periodic,
                    _ => analysis::Boundary::Extrapolate,
                };
            Job::Tool(Tool::Smooth(analysis::SmoothOptions { method, npts: o.usize("points", 5)?, boundary }))
        }
        "differentiate" => Job::Tool(Tool::Differentiate(analysis::DiffOptions {
            order: o.usize("order", 1)?,
            smooth: o.flag("smooth", false)?,
            poly: o.usize("poly", 2)?,
            npts: o.usize("points", 20)?,
        })),
        "normalize" => {
            use analysis::NormalizeMethod as M;
            let method = match o
                .pick(
                    "method",
                    "max",
                    &[
                        "max", "range", "range100", "z_score", "min", "mean", "median", "sd", "norm", "mode", "sum",
                        "divide",
                    ],
                )?
                .as_str()
            {
                "max" => M::Max,
                "range" => M::ZeroToOne,
                "range100" => M::ZeroToHundred,
                "z_score" => M::ZScore,
                "min" => M::Min,
                "mean" => M::Mean,
                "median" => M::Median,
                "sd" => M::Sd,
                "norm" => M::Norm,
                "mode" => M::Mode,
                "sum" => M::Sum,
                _ => M::Divide(
                    o.opt_num("value")?.ok_or_else(|| anyhow!("method divide needs the divisor: option \"value\""))?,
                ),
            };
            Job::Tool(Tool::Normalize(method))
        }
        "baseline" => {
            let line = o.numbers("line")?;
            let spec = match (o.opt_num("constant")?, o.flag("end_points", false)?, line.as_slice()) {
                (Some(level), _, _) => BaselineSpec::Constant(level),
                (None, true, _) => BaselineSpec::EndPoints,
                (None, false, [x1, y1, x2, y2]) => BaselineSpec::Line((*x1, *y1), (*x2, *y2)),
                (None, false, []) => BaselineSpec::Minimum,
                _ => bail!("line needs four numbers: [x1, y1, x2, y2]"),
            };
            Job::Tool(Tool::SubtractBaseline(spec))
        }
        "interpolate" => {
            let method = match o.pick("method", "linear", &["linear", "spline", "akima", "bspline"])?.as_str() {
                "linear" => analysis::InterpMethod::Linear,
                "spline" => analysis::InterpMethod::CubicSpline,
                "akima" => analysis::InterpMethod::Akima,
                _ => analysis::InterpMethod::BSpline,
            };
            let range = match (o.opt_num("x_min")?, o.opt_num("x_max")?) {
                (None, None) => None,
                (Some(lo), Some(hi)) => Some((lo, hi)),
                _ => bail!("give both x_min and x_max, or neither"),
            };
            Job::Tool(Tool::Interpolate(InterpSpec {
                method,
                npts: o.usize_up_to("points", 100, ROW_CAP)?,
                range,
                smoothing: o.num("smoothing", 0.0)?,
                ..InterpSpec::default()
            }))
        }
        "find_peaks" => {
            let direction = match o.pick("direction", "both", &["both", "positive", "negative"])?.as_str() {
                "both" => analysis::PeakDirection::Both,
                "positive" => analysis::PeakDirection::Positive,
                _ => analysis::PeakDirection::Negative,
            };
            let filter = match (o.opt_num("min_height")?, o.opt_usize("count")?, o.flag("all", false)?) {
                (Some(h), _, _) => analysis::PeakFilter::Height(h),
                (None, Some(n), _) => analysis::PeakFilter::Number(n),
                (None, None, true) => analysis::PeakFilter::None,
                (None, None, false) => analysis::PeakFilter::Height(0.0),
            };
            Job::Tool(Tool::FindPeaks(analysis::PeakOptions {
                npts: o.usize("points", 2)?,
                direction,
                filter,
                ..Default::default()
            }))
        }
        "statistics" => {
            let mut spec = if o.flag("all", false)? {
                analysis::StatisticsSpec::all()
            } else {
                analysis::StatisticsSpec::default()
            };
            for n in o.strings("add")? {
                match n.trim().to_ascii_lowercase().as_str() {
                    "iqr" => spec.iqr = true,
                    "variance" | "var" => spec.variance = true,
                    "skewness" | "skew" => spec.skewness = true,
                    "kurtosis" | "kurt" => spec.kurtosis = true,
                    "cv" => spec.cv = true,
                    "limits" | "cl" | "ci" => spec.confidence_limits = true,
                    other => bail!("unknown quantity {other:?}; use iqr, variance, skewness, kurtosis, cv or limits"),
                }
            }
            spec.level = o.num("level", 0.95)?;
            Job::Tool(Tool::Statistics(spec))
        }
        "fft" => {
            let interval = o.opt_num("interval")?;
            let window = match o
                .pick(
                    "window",
                    "rectangle",
                    &["rectangle", "welch", "triangular", "bartlett", "hanning", "hamming", "blackman"],
                )?
                .as_str()
            {
                "rectangle" => analysis::FftWindow::Rectangle,
                "welch" => analysis::FftWindow::Welch,
                "triangular" => analysis::FftWindow::Triangular,
                "bartlett" => analysis::FftWindow::Bartlett,
                "hanning" => analysis::FftWindow::Hanning,
                "hamming" => analysis::FftWindow::Hamming,
                _ => analysis::FftWindow::Blackman,
            };
            let correction = match o.pick("correction", "amplitude", &["none", "amplitude", "power"])?.as_str() {
                "none" => analysis::WindowCorrection::None,
                "amplitude" => analysis::WindowCorrection::Amplitude,
                _ => analysis::WindowCorrection::Power,
            };
            let power = match o.pick("power", "msa", &["msa", "ssa", "tisa"])?.as_str() {
                "msa" => analysis::PowerNorm::Msa,
                "ssa" => analysis::PowerNorm::Ssa,
                _ => analysis::PowerNorm::Tisa,
            };
            let options = analysis::FftOptions {
                interval: interval.unwrap_or(1.0),
                window,
                correction,
                normalize: o.flag("normalize", false)?,
                two_sided: o.flag("two_sided", false)?,
                shift: o.flag("shift", true)?,
                unwrap: o.flag("unwrap", true)?,
                science: o.flag("science", false)?,
                power,
            };
            Job::Tool(Tool::Fft(analysis::FftSpec { options, auto_interval: interval.is_none() }))
        }
        "hilbert" => Job::Tool(Tool::Hilbert),
        "envelope" => {
            let kind = match o.pick("kind", "upper", &["upper", "lower", "both"])?.as_str() {
                "upper" => analysis::EnvelopeType::Upper,
                "lower" => analysis::EnvelopeType::Lower,
                _ => analysis::EnvelopeType::Both,
            };
            let options = analysis::EnvelopeOptions { kind, npts: o.num("npts", 2.0)? };
            Job::Tool(Tool::Envelope(analysis::EnvelopeSpec { options }))
        }
        "level_crossing" => {
            let index = match o.pick("row", "any", &["any", "left", "right"])?.as_str() {
                "any" => analysis::CrossingIndex::Nearest,
                "left" => analysis::CrossingIndex::Left,
                _ => analysis::CrossingIndex::Right,
            };
            Job::Tool(Tool::LevelCrossing(analysis::LevelCrossingSpec {
                level: o.required_num("level")?,
                index,
                min_gap: o.usize_up_to("min_gap", 1, ROW_CAP)?,
                details: o.flag("details", false)?,
            }))
        }
        "median_filter" => {
            let edge = match o.pick("edge", "shrink", &["shrink", "repeat", "reflect"])?.as_str() {
                "shrink" => analysis::MedianEdge::Shrink,
                "repeat" => analysis::MedianEdge::Repeat,
                _ => analysis::MedianEdge::Reflect,
            };
            Job::Tool(Tool::MedianFilter(analysis::MedianFilterSpec { npts: o.usize("points", 5)?, edge }))
        }
        "decimate" => {
            let filter = match o.pick("filter", "none", &["none", "ave", "fir"])?.as_str() {
                "none" => analysis::DecimateFilter::None,
                "ave" => analysis::DecimateFilter::MovingAverage,
                _ => analysis::DecimateFilter::Fir,
            };
            let options = analysis::DecimateOptions {
                factor: o.usize_up_to("factor", 2, ROW_CAP)?,
                filter,
                order: o.usize_up_to("order", 30, ROW_CAP)?,
            };
            Job::Tool(Tool::Decimate(analysis::DecimateSpec { options }))
        }
        "translate" => {
            let shift = match (o.opt_num("by")?, o.opt_num("at")?, o.opt_num("to")?) {
                (Some(d), _, _) => analysis::Shift::By(d),
                (None, Some(at), Some(to)) => analysis::Shift::PointTo { at, to },
                _ => bail!("give \"by\" AMOUNT, or \"at\" X together with \"to\" VALUE"),
            };
            let axis = if o.pick("axis", "x", &["x", "y"])? == "x" {
                analysis::TranslateAxis::X
            } else {
                analysis::TranslateAxis::Y
            };
            Job::Tool(Tool::Translate(analysis::TranslateSpec { axis, shift }))
        }
        "subtract_line" => Job::Tool(Tool::SubtractLine(analysis::SubtractLineSpec {
            p1: analysis::LinePoint { x: o.required_num("x1")?, y: o.opt_num("y1")? },
            p2: analysis::LinePoint { x: o.required_num("x2")?, y: o.opt_num("y2")? },
        })),
        "polygon_area" => Job::Tool(Tool::PolygonArea),
        "xyz_area" => Job::Tool(Tool::XyzArea),
        "intersect" => {
            Job::Tool(Tool::Intersect(analysis::IntersectSpec { include_touching: o.flag("touching", false)? }))
        }
        other => bail!("analysis tool \"{other}\" is listed but not built"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn every_tool_builds_with_no_options_or_its_required_ones() {
        for t in registry() {
            let mut m = Map::new();
            for o in t.options.iter().filter(|o| o.required) {
                m.insert(o.name.to_string(), json!(1.0));
            }
            if t.name == "translate" {
                m.insert("by".into(), json!(1.0));
            }
            if t.name == "subtract_ref" {
                m.insert("reference".into(), json!("B"));
            }
            assert!(build(t.name, &m).is_ok(), "{}: {:?}", t.name, build(t.name, &m).err());
        }
    }

    #[test]
    fn defaults_listed_are_the_defaults_used() {
        // The listed default of smooth.points is the one the builder uses.
        let Job::Tool(Tool::Smooth(s)) = build("smooth", &Map::new()).unwrap() else { panic!() };
        assert_eq!(s.npts, 5);
        let listed = registry().into_iter().find(|t| t.name == "smooth").unwrap();
        assert_eq!(listed.options.iter().find(|o| o.name == "points").unwrap().default, json!(5));
    }

    #[test]
    fn huge_integer_options_are_refused() {
        let e = build("interpolate", &opts(json!({"points": 4_000_000_000_000u64}))).err().unwrap().to_string();
        assert!(e.contains("at most 10000000"), "{e}");
        assert!(build("interpolate", &opts(json!({"points": 10_000_000}))).is_ok());
        for (tool, o) in [
            ("smooth", json!({"method": "binomial", "order": 4_000_000_000_000u64})),
            ("smooth", json!({"poly": 100000})),
            ("smooth", json!({"points": 100000})),
            ("differentiate", json!({"points": 100000})),
            ("decimate", json!({"order": 100_000_000})),
        ] {
            assert!(build(tool, &opts(o.clone())).is_err(), "{tool} {o}");
        }
    }

    #[test]
    fn defaults_follow_the_command_line() {
        let Job::Tool(Tool::Fft(f)) = build("fft", &Map::new()).unwrap() else { panic!() };
        assert_eq!(f.options.correction, analysis::WindowCorrection::Amplitude);
        let Job::Tool(Tool::Decimate(d)) = build("decimate", &Map::new()).unwrap() else { panic!() };
        assert_eq!(d.options.order, 30);
        let Job::Tool(Tool::Translate(t)) = build("translate", &opts(json!({"by": 1}))).unwrap() else { panic!() };
        assert_eq!(t.axis, analysis::TranslateAxis::X);
    }

    #[test]
    fn bad_options_are_named() {
        let e = build("smooth", &opts(json!({"pointz": 5}))).err().unwrap().to_string();
        assert!(e.contains("unknown option \"pointz\" for smooth"), "{e}");
        let e = build("smooth", &opts(json!({"method": "magic"}))).err().unwrap().to_string();
        assert!(e.contains("one of savitzky_golay"), "{e}");
        let e = build("normalize", &opts(json!({"method": "divide"}))).err().unwrap().to_string();
        assert!(e.contains("divisor"), "{e}");
        assert!(build("nothing", &Map::new()).is_err());
        assert!(build("level_crossing", &Map::new()).is_err());
    }
}
