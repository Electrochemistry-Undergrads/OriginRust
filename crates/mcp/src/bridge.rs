//! From the command line's clap definitions to a JSON Schema, and from a JSON object back to a
//! command line that clap parses. `analyze` reaches every `sheaf analyze` tool this way: a new
//! option of the command line appears here without new code, and the values are checked by the
//! same clap types the command line uses.

use anyhow::{Result, anyhow, bail};
use clap::{Arg, ArgAction, Command, CommandFactory, FromArgMatches};
use serde_json::{Map, Value, json};
use sheaf_ops::args::Cli;
use std::any::TypeId;
use std::collections::HashSet;
use std::path::PathBuf;

/// The largest value of an integer option (points, window, order, bins): a bound against a request
/// that would allocate without limit.
pub const INTEGER_CAP: i64 = 10_000_000;

/// Which options of a command a tool offers.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    /// Option ids left out.
    pub exclude: HashSet<String>,
}

impl Filter {
    pub fn excluding<S: AsRef<str>>(ids: impl IntoIterator<Item = S>) -> Filter {
        Filter { exclude: ids.into_iter().map(|s| s.as_ref().to_string()).collect() }
    }

    /// Also leaves out these ids.
    pub fn and_excluding<S: AsRef<str>>(mut self, ids: impl IntoIterator<Item = S>) -> Filter {
        self.exclude.extend(ids.into_iter().map(|s| s.as_ref().to_string()));
        self
    }

    fn offers(&self, arg: &Arg) -> bool {
        let id = arg.get_id().as_str();
        !arg.is_positional()
            && !arg.is_hide_set()
            && arg.get_long().is_some()
            && !matches!(id, "help" | "version")
            && !self.exclude.contains(id)
    }
}

/// The ids of the options of an `Args` struct such as `ReadOptions`.
pub fn ids_of<A: clap::Args>() -> Vec<String> {
    A::augment_args(Command::new("x")).get_arguments().map(|a| a.get_id().to_string()).collect()
}

/// The subcommand at `path` (for example `["analyze", "integrate"]`) of the `sheaf` command line.
pub fn subcommand(path: &[&str]) -> Result<Command> {
    let mut cmd = Cli::command();
    for name in path {
        cmd = cmd.find_subcommand(name).cloned().ok_or_else(|| anyhow!("there is no command '{}'", path.join(" ")))?;
    }
    Ok(cmd)
}

fn is_type<T: 'static>(arg: &Arg) -> bool {
    arg.get_value_parser().type_id() == TypeId::of::<T>()
}

/// Whether the option's value is a file path (resolved through the roots).
pub fn is_path(arg: &Arg) -> bool {
    is_type::<PathBuf>(arg)
}

fn is_flag(arg: &Arg) -> bool {
    matches!(arg.get_action(), ArgAction::SetTrue | ArgAction::SetFalse)
}

fn is_list(arg: &Arg) -> bool {
    matches!(arg.get_action(), ArgAction::Append)
        || arg.get_value_delimiter().is_some()
        || arg.get_num_args().is_some_and(|n| n.max_values() > 1)
}

/// The JSON Schema type of one value of the option.
fn element_type(arg: &Arg) -> &'static str {
    if !arg.get_possible_values().is_empty() {
        return "string";
    }
    if is_type::<bool>(arg) {
        return "boolean";
    }
    if is_type::<f64>(arg) || is_type::<f32>(arg) {
        return "number";
    }
    macro_rules! ints {
        ($($t:ty),*) => { false $(|| is_type::<$t>(arg))* };
    }
    if ints!(usize, u8, u16, u32, u64, isize, i8, i16, i32, i64) { "integer" } else { "string" }
}

fn is_unsigned(arg: &Arg) -> bool {
    macro_rules! any {
        ($($t:ty),*) => { false $(|| is_type::<$t>(arg))* };
    }
    any!(usize, u8, u16, u32, u64)
}

/// The key a tool shows for an option: the long name with `_` instead of `-`.
pub fn key_of(long: &str) -> String {
    long.replace('-', "_")
}

/// Wording of the command line made fit for a tool: a flag `--x-range` becomes `x_range`, and the
/// remark about project inputs is dropped.
pub fn tool_text(text: &str) -> String {
    let text = text
        .replace("sheaf fit --list", "list_fit_models")
        .replace("For a project input: the workbook", "The workbook")
        .replace("For a project input: ", "");
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let at_start = i == 0 || !(chars[i - 1].is_alphanumeric() || chars[i - 1] == '-');
        if at_start
            && chars[i] == '-'
            && chars.get(i + 1) == Some(&'-')
            && chars.get(i + 2).is_some_and(|c| c.is_ascii_lowercase())
        {
            i += 2;
            while i < chars.len() && (chars[i].is_ascii_lowercase() || chars[i].is_ascii_digit() || chars[i] == '-') {
                out.push(if chars[i] == '-' { '_' } else { chars[i] });
                i += 1;
            }
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn describe(arg: &Arg) -> String {
    let mut text = arg.get_help().or(arg.get_long_help()).map(|h| tool_text(&h.to_string())).unwrap_or_default();
    if is_path(arg) {
        text.push_str(" (a path inside the allowed folders)");
    }
    text
}

fn default_value(arg: &Arg, ty: &str) -> Option<Value> {
    let d = arg.get_default_values().first()?.to_str()?;
    Some(match ty {
        "number" => d.parse::<f64>().ok().and_then(serde_json::Number::from_f64).map(Value::Number)?,
        "integer" => d.parse::<i64>().ok().map(Value::from)?,
        "boolean" => Value::Bool(d == "true"),
        _ => Value::String(d.to_string()),
    })
}

/// The property of one option.
fn property(arg: &Arg) -> Value {
    let ty = element_type(arg);
    let mut item = Map::new();
    item.insert("type".into(), json!(ty));
    let values: Vec<Value> = arg
        .get_possible_values()
        .iter()
        .filter(|v| !v.is_hide_set())
        .map(|v| Value::String(v.get_name().to_string()))
        .collect();
    if !values.is_empty() {
        item.insert("enum".into(), Value::Array(values));
    }
    let mut prop = Map::new();
    let description = describe(arg);
    if is_flag(arg) {
        prop.insert("type".into(), json!("boolean"));
    } else if is_list(arg) {
        prop.insert("type".into(), json!("array"));
        let mut item = item;
        if ty == "integer" {
            item.insert("minimum".into(), json!(if is_unsigned(arg) { 0 } else { -INTEGER_CAP }));
            item.insert("maximum".into(), json!(INTEGER_CAP));
        }
        prop.insert("items".into(), Value::Object(item));
    } else {
        prop = item;
        if ty == "integer" {
            prop.insert("minimum".into(), json!(if is_unsigned(arg) { 0 } else { -INTEGER_CAP }));
            prop.insert("maximum".into(), json!(INTEGER_CAP));
        }
        if let Some(d) = default_value(arg, ty) {
            prop.insert("default".into(), d);
        }
    }
    if !description.is_empty() {
        prop.insert("description".into(), json!(description));
    }
    Value::Object(prop)
}

/// The JSON Schema (an object schema) of the options `filter` offers.
pub fn schema(cmd: &Command, filter: &Filter) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();
    for arg in cmd.get_arguments().filter(|a| filter.offers(a)) {
        let key = key_of(arg.get_long().expect("offered options have a long name"));
        properties.insert(key.clone(), property(arg));
        if arg.is_required_set() {
            required.push(json!(key));
        }
    }
    json!({ "type": "object", "properties": properties, "required": required, "additionalProperties": false })
}

fn kind_of(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a text",
        Value::Array(_) => "a list",
        Value::Object(_) => "an object",
    }
}

fn scalar_text(arg: &Arg, name: &str, v: &Value) -> Result<String> {
    let integer = element_type(arg) == "integer";
    match v {
        Value::String(s) if integer => match s.parse::<i128>() {
            Ok(i) if i >= lowest(arg) as i128 && i <= INTEGER_CAP as i128 => Ok(i.to_string()),
            Ok(_) => bail!("option '{name}' must be between {} and {INTEGER_CAP}", lowest(arg)),
            Err(_) => Ok(s.clone()), // clap reports malformed integers
        },
        Value::String(s) => Ok(s.clone()),
        // JSON Schema calls 7.0 an integer, and some clients write every number with a fraction.
        Value::Number(n) if integer && n.as_i64().is_none() && n.as_u64().is_none() => match n.as_f64() {
            Some(f) if f.fract() == 0.0 && f.abs() < 9.0e15 => capped(arg, name, f as i64),
            _ => Ok(n.to_string()),
        },
        Value::Number(n) if integer => match n.as_i64() {
            Some(i) => capped(arg, name, i),
            None => bail!("option '{name}' must be between {} and {INTEGER_CAP}", lowest(arg)),
        },
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        _ => bail!("option '{name}' takes a text or a number, not {}", kind_of(v)),
    }
}

fn lowest(arg: &Arg) -> i64 {
    if is_unsigned(arg) { 0 } else { -INTEGER_CAP }
}

fn capped(arg: &Arg, name: &str, i: i64) -> Result<String> {
    if i < lowest(arg) || i > INTEGER_CAP {
        bail!("option '{name}' must be between {} and {INTEGER_CAP} (got {i})", lowest(arg));
    }
    Ok(i.to_string())
}

/// The valid keys of the options `filter` offers, sorted.
fn valid_keys(cmd: &Command, filter: &Filter) -> Vec<String> {
    let mut valid: Vec<String> =
        cmd.get_arguments().filter(|a| filter.offers(a)).filter_map(|a| a.get_long()).map(key_of).collect();
    valid.sort_unstable();
    valid
}

/// The command-line arguments (without the program and command names) for a JSON object of
/// options. A key is the option's long name with `_` or `-`. `resolve` is called for the value of
/// every option that holds a path and returns the path to use. Every option is written
/// `--name=VALUE`, so a value that starts with `-` (a negative number, a range such as `-0.5:0.1`)
/// is not mistaken for an option.
pub fn to_argv(
    cmd: &Command,
    filter: &Filter,
    options: &Map<String, Value>,
    resolve: &mut dyn FnMut(&str) -> Result<String>,
) -> Result<Vec<String>> {
    let mut argv = Vec::new();
    let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (key, value) in options {
        let wanted = key.replace('_', "-");
        let Some(arg) =
            cmd.get_arguments().filter(|a| filter.offers(a)).find(|a| a.get_long() == Some(wanted.as_str()))
        else {
            let valid = valid_keys(cmd, filter);
            if valid.is_empty() {
                bail!("unknown option '{key}'; this tool takes no options");
            }
            bail!("unknown option '{key}'; the options are: {}", valid.join(", "));
        };
        if value.is_null() {
            continue;
        }
        let long = arg.get_long().expect("offered options have a long name");
        if let Some(first) = seen.insert(long.to_string(), key.clone()) {
            bail!("option '{long}' is given twice, as '{first}' and '{key}'");
        }
        if is_flag(arg) {
            match value {
                Value::Bool(true) => argv.push(format!("--{long}")),
                Value::Bool(false) => {}
                _ => bail!("option '{key}' is a flag: use true or false, not {}", kind_of(value)),
            }
            continue;
        }
        // The elements of a list are kept whole (a column called "Current, mA" is one element, see
        // `parse`); a single text for a list option is split on the option's delimiter, as on the
        // command line.
        let mut texts: Vec<String> = Vec::new();
        match value {
            Value::Array(a) => {
                for item in a {
                    texts.push(scalar_text(arg, key, item)?);
                }
            }
            v => {
                let text = scalar_text(arg, key, v)?;
                match arg.get_value_delimiter() {
                    Some(d) if is_list(arg) => texts.extend(text.split(d).map(str::to_string)),
                    _ => texts.push(text),
                }
            }
        }
        for mut text in texts {
            if is_path(arg) {
                text = resolve(&text)?;
            }
            argv.push(format!("--{long}={text}"));
        }
    }
    Ok(argv)
}

/// clap's message for people: the error lines, without the usage and help hints.
pub fn clap_message(e: &clap::Error) -> String {
    let full = e.render().to_string();
    let mut lines = Vec::new();
    for line in full.lines() {
        if line.starts_with("Usage:") || line.starts_with("For more information") {
            break;
        }
        lines.push(line);
    }
    let text = lines.join("\n");
    let text = text.trim();
    tool_text(text.strip_prefix("error: ").unwrap_or(text)).replace("@session", "<input>")
}

/// The command with the value delimiters of its options removed: every JSON array element is one
/// value, even when it holds a comma.
fn without_delimiters(cmd: Command) -> Command {
    cmd.mut_args(|a| if a.get_value_delimiter().is_some() { a.value_delimiter(None) } else { a })
        .mut_subcommands(without_delimiters)
}

/// Options that operate on the session, not CLI input/output files.
pub fn session_filter(command: &str) -> Filter {
    let excluded = if command == "plot" {
        vec!["inputs", "output", "dpi", "image_width", "image_height", "margin", "border_width", "save"]
    } else {
        vec!["input", "list", "plot", "dpi", "save", "export"]
    };
    Filter::excluding(excluded).and_excluding(ids_of::<sheaf_ops::args::ReadOptions>())
}

/// Parse a session operation, providing placeholders for required CLI files.
pub fn parse_session(
    command: &str,
    options: &Map<String, Value>,
    resolve: &mut dyn FnMut(&str) -> Result<String>,
) -> Result<Cli> {
    let cmd = subcommand(&[command])?;
    let mut argv = vec!["sheaf".to_string(), command.to_string(), "@session".to_string()];
    if command == "plot" {
        argv.push("--output=@session.png".to_string());
    }
    argv.extend(to_argv(&cmd, &session_filter(command), options, resolve)?);
    let matches =
        without_delimiters(Cli::command()).try_get_matches_from(argv).map_err(|e| anyhow!("{}", clap_message(&e)))?;
    Cli::from_arg_matches(&matches).map_err(|e| anyhow!("{}", clap_message(&e)))
}

/// The top-level selection of an `analyze` call, written as the command line's options.
#[derive(Debug, Default, Clone)]
pub struct Selection<'a> {
    pub columns: &'a [String],
    pub book: Option<&'a str>,
    pub sheet: Option<&'a str>,
    pub rows: Option<&'a str>,
    pub x_range: Option<&'a str>,
}

impl Selection<'_> {
    fn is_empty(&self) -> bool {
        self.columns.is_empty()
            && self.book.is_none()
            && self.sheet.is_none()
            && self.rows.is_none()
            && self.x_range.is_none()
    }

    fn argv(&self) -> Vec<String> {
        let mut v: Vec<String> = self.columns.iter().map(|c| format!("--y={c}")).collect();
        for (name, value) in
            [("book", self.book), ("sheet", self.sheet), ("rows", self.rows), ("x-range", self.x_range)]
        {
            if let Some(t) = value {
                v.push(format!("--{name}={t}"));
            }
        }
        v
    }
}

/// Parses the options of the command at `path` the way the command line does. The input file the
/// command line requires gets a placeholder, which the callers ignore. `selection` is written as
/// the options `--y`, `--book`, `--sheet`, `--rows` and `--x-range`, so clap checks them as it does
/// on the command line.
pub fn parse(
    path: &[&str],
    filter: &Filter,
    selection: &Selection,
    options: &Map<String, Value>,
    resolve: &mut dyn FnMut(&str) -> Result<String>,
) -> Result<Cli> {
    let cmd = subcommand(path)?;
    let mut argv: Vec<String> = vec!["sheaf".into()];
    argv.extend(path.iter().map(|s| s.to_string()));
    let takes_data = cmd.get_arguments().any(|a| a.is_positional());
    if takes_data {
        argv.push("@session".into());
    } else if !selection.is_empty() {
        bail!(
            "{} works on no worksheet data, so columns, book, sheet, rows and x_range do not apply",
            path.last().unwrap_or(&"this tool")
        );
    }
    argv.extend(selection.argv());
    argv.extend(to_argv(&cmd, filter, options, resolve)?);
    let matches =
        without_delimiters(Cli::command()).try_get_matches_from(argv).map_err(|e| anyhow!("{}", clap_message(&e)))?;
    Cli::from_arg_matches(&matches).map_err(|e| anyhow!("{}", clap_message(&e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_ops::args::{AnalyzeTool, Command as Cmd};

    fn no_paths(p: &str) -> Result<String> {
        Ok(p.to_string())
    }

    fn analyze_filter() -> Filter {
        Filter::excluding(["input", "y", "book", "sheet", "rows", "x_range", "export", "save"])
            .and_excluding(ids_of::<sheaf_ops::args::ReadOptions>())
    }

    fn obj(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    fn analyze(sub: &str, sel: &Selection, options: Value) -> Result<AnalyzeTool> {
        match parse(&["analyze", sub], &analyze_filter(), sel, &obj(options), &mut no_paths)?.command {
            Cmd::Analyze(a) => Ok(a.tool),
            _ => panic!("analyze"),
        }
    }

    #[test]
    fn negative_numbers_arrays_and_selection_reach_clap() {
        let cols = vec!["Current, mA".to_string(), "C".to_string()];
        let sel = Selection { columns: &cols, x_range: Some("-0.5:0.1"), ..Selection::default() };
        let AnalyzeTool::Integrate { target, baseline, ranges, .. } =
            analyze("integrate", &sel, json!({"ranges": ["-0.5:0.1", "0.2:0.4"], "baseline": true})).unwrap()
        else {
            panic!("integrate")
        };
        assert_eq!(target.x_range.as_deref(), Some("-0.5:0.1"));
        assert_eq!(target.y, ["Current, mA", "C"], "a comma inside a column name is kept");
        assert_eq!(ranges, ["-0.5:0.1", "0.2:0.4"]);
        assert!(baseline);
        // A bare text for a list option splits on the delimiter, as on the command line.
        let AnalyzeTool::Integrate { ranges, .. } =
            analyze("integrate", &Selection::default(), json!({"ranges": "0.1:0.2,0.3:0.4"})).unwrap()
        else {
            panic!("integrate")
        };
        assert_eq!(ranges, ["0.1:0.2", "0.3:0.4"]);
    }

    #[test]
    fn whole_floats_are_integers_kebab_keys_work_and_negative_numbers_are_values() {
        let AnalyzeTool::Decimate { factor, .. } =
            analyze("decimate", &Selection::default(), json!({"factor": 4.0})).unwrap()
        else {
            panic!("decimate")
        };
        assert_eq!(factor, 4);
        assert!(analyze("translate", &Selection::default(), json!({"by": -2.5})).is_ok());
        assert!(analyze("integrate", &Selection::default(), json!({"row-ranges": ["1:5"]})).is_ok());
        assert!(analyze("integrate", &Selection::default(), json!({"row_ranges": ["1:5"]})).is_ok());
        let e = analyze("decimate", &Selection::default(), json!({"factor": 4_000_000_000_000u64})).unwrap_err();
        assert!(e.to_string().contains("between"), "{e}");
    }

    #[test]
    fn schemas_bound_integers_and_a_repeated_key_is_refused() {
        let cmd = subcommand(&["analyze", "histogram"]).unwrap();
        let s = schema(&cmd, &analyze_filter());
        assert_eq!(s["properties"]["bins"]["minimum"], 0);
        assert_eq!(s["properties"]["bins"]["maximum"], INTEGER_CAP);
        let e = analyze("histogram", &Selection::default(), json!({"bins": -3})).unwrap_err();
        assert!(e.to_string().contains("between 0 and"), "{e}");
        let e = analyze("histogram", &Selection::default(), json!({"no_graph": true, "no-graph": false})).unwrap_err();
        assert!(e.to_string().contains("twice"), "{e}");
    }

    #[test]
    fn an_unknown_key_names_the_valid_ones_and_left_out_options_are_unknown() {
        let e = analyze("integrate", &Selection::default(), json!({"nope": 1})).unwrap_err().to_string();
        assert!(e.contains("unknown option 'nope'") && e.contains("row_ranges") && e.contains("baseline"), "{e}");
        for key in ["save", "export", "input", "x_range", "y", "delimiter"] {
            let e = analyze("integrate", &Selection::default(), json!({ key: "a" })).unwrap_err().to_string();
            assert!(e.contains(&format!("unknown option '{key}'")), "{key}: {e}");
        }
    }

    #[test]
    fn clap_checks_values_and_the_message_has_no_usage_tail() {
        let e = analyze("smooth", &Selection::default(), json!({"method": "magic"})).unwrap_err().to_string();
        assert!(e.contains("magic") && !e.contains("Usage"), "{e}");
        let e = analyze("decimate", &Selection::default(), json!({"factor": "x"})).unwrap_err().to_string();
        assert!(e.contains("factor") && !e.contains("Usage"), "{e}");
        let sel = Selection { rows: Some("1:3"), x_range: Some("0:1"), ..Selection::default() };
        assert!(analyze("integrate", &sel, json!({})).is_err(), "rows and x_range exclude each other");
    }

    #[test]
    fn the_schema_describes_flags_lists_enums_and_numbers() {
        let cmd = subcommand(&["analyze", "smooth"]).unwrap();
        let s = schema(&cmd, &analyze_filter());
        assert_eq!(s["type"], "object");
        assert_eq!(s["additionalProperties"], false);
        let p = &s["properties"];
        assert!(p["method"]["enum"].as_array().unwrap().len() > 3);
        assert!(p.get("input").is_none() && p.get("export").is_none() && p.get("y").is_none());
        assert!(p.get("help").is_none());
        let cmd = subcommand(&["analyze", "integrate"]).unwrap();
        let s = schema(&cmd, &analyze_filter());
        assert_eq!(s["properties"]["baseline"]["type"], "boolean");
        assert_eq!(s["properties"]["ranges"]["type"], "array");
        assert_eq!(s["properties"]["row_ranges"]["items"]["type"], "string");
        let cmd = subcommand(&["analyze", "translate"]).unwrap();
        let s = schema(&cmd, &analyze_filter());
        assert_eq!(s["properties"]["by"]["type"], "number");
        let cmd = subcommand(&["analyze", "decimate"]).unwrap();
        let s = schema(&cmd, &analyze_filter());
        assert_eq!(s["properties"]["factor"]["type"], "integer");
    }

    #[test]
    fn path_options_go_through_the_resolver() {
        let mut resolve = |p: &str| -> Result<String> {
            if p.contains("..") { bail!("'{p}' is outside") } else { Ok(format!("/root/{p}")) }
        };
        let f = analyze_filter();
        let cli = parse(
            &["analyze", "math"],
            &f,
            &Selection::default(),
            &obj(json!({"ref": "B", "ref_file": "a.csv"})),
            &mut resolve,
        )
        .unwrap();
        let Cmd::Analyze(a) = cli.command else { panic!("analyze") };
        let AnalyzeTool::Math { operand, .. } = a.tool else { panic!("math") };
        assert_eq!(operand.ref_file.unwrap().to_str(), Some("/root/a.csv"));
        let outside = obj(json!({"ref": "B", "ref_file": "../a.csv"}));
        assert!(parse(&["analyze", "math"], &f, &Selection::default(), &outside, &mut resolve).is_err());
    }

    #[test]
    fn power_takes_no_selection() {
        let cols = vec!["B".to_string()];
        let sel = Selection { columns: &cols, ..Selection::default() };
        let e = analyze("power", &sel, json!({"sizes": [10, 20], "mean2": 1.0})).unwrap_err().to_string();
        assert!(e.contains("no worksheet data"), "{e}");
        assert!(analyze("power", &Selection::default(), json!({"sizes": [10, 20], "mean2": 1.0})).is_ok());
    }

    #[test]
    fn every_analyze_tool_has_an_object_schema() {
        let analyze = subcommand(&["analyze"]).unwrap();
        let mut count = 0;
        for sub in analyze.get_subcommands() {
            let cmd = subcommand(&["analyze", sub.get_name()]).unwrap();
            let s = schema(&cmd, &analyze_filter());
            assert_eq!(s["type"], "object", "{}", sub.get_name());
            assert!(s["properties"].is_object());
            count += 1;
        }
        assert!(count > 50, "{count} analysis tools");
    }
}
