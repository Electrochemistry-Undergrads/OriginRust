//! The tools: each reads its arguments, calls `sheaf-ops` and turns the result into JSON.

use crate::schemas::{self, Def};
use crate::server::Server;
use crate::{analysis_tools, b64, bridge};
use anyhow::{Context, anyhow};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};
use sheaf_core::column::{Column, ColumnData, Designation, RecalcMode};
use sheaf_core::project::Window;
use sheaf_core::template::Template;
use sheaf_core::{ColumnId, Project, WindowId};
use sheaf_ops::analyze::{Job, Outcome, Target};
use sheaf_ops::args::AnalyzeTool;
use sheaf_ops::cli_analyze::{job_from, power_spec};
use sheaf_ops::fit::{BandKind, FitRun, FitSpec, WeightKind};
use sheaf_ops::import::{ImportLayout, ImportRequest};
use sheaf_ops::pick::{named_columns, pick_book, pick_graph, pick_sheet};
use sheaf_ops::plot::{PanelsSpec, PlotSpec, Style};
use sheaf_ops::render::{ExportSettings, MarginSettings};
use sheaf_ops::tabular::SheetFormat;
use sheaf_ops::{DataFormat, ReadSettings, ThousandsSetting};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Most cells one reply carries.
pub const CELL_CAP: usize = 10_000;
/// Most rows `set_column_values` works on.
const ROW_CAP: usize = 10_000_000;
/// Largest image the server renders.
const PIXEL_CAP: f64 = 40_000_000.0;

/// The most pages one `export_graph` call writes into a multi-page PDF. Sheaf's own bound for
/// one tool call (not an Origin limit): each page costs a render and a conversion.
const MAX_EXPORT_PAGES: usize = 500;
/// Longest side of an image returned in a reply.
const IMAGE_SIDE: f64 = 1600.0;
/// Largest file `import_files` reads.
const FILE_CAP: u64 = 256 * 1024 * 1024;

pub struct Image {
    pub data: String,
    pub mime: &'static str,
}

/// A tool's successful result.
pub struct ToolOut {
    pub value: Value,
    pub image: Option<Image>,
}

impl ToolOut {
    fn json(value: Value) -> ToolOut {
        ToolOut { value, image: None }
    }

    pub fn into_result(self, structured: bool) -> Value {
        let mut value = self.value;
        if let Some(o) = value.as_object_mut() {
            o.entry("warnings").or_insert_with(|| json!([]));
        }
        let text = serde_json::to_string_pretty(&value).unwrap_or_default();
        let mut content = vec![json!({"type": "text", "text": text})];
        if let Some(img) = self.image {
            content.push(json!({"type": "image", "data": img.data, "mimeType": img.mime}));
        }
        let mut result = json!({"content": content, "isError": false});
        if structured && value.is_object() {
            result["structuredContent"] = value;
        }
        result
    }
}

/// A failed tool: the message and, sometimes, data worth keeping.
#[derive(Debug)]
pub struct ToolError {
    pub message: String,
    pub data: Option<Value>,
}

impl ToolError {
    fn new(message: impl Into<String>) -> ToolError {
        ToolError { message: message.into(), data: None }
    }

    pub fn into_result(self, structured: bool) -> Value {
        let mut result = json!({"content": [{"type": "text", "text": self.message}], "isError": true});
        if let (true, Some(data)) = (structured, self.data) {
            result["structuredContent"] = data;
        }
        result
    }
}

/// Rewrites a shared message for a request from JSON: the cause chain joined by `: `, no `error: `
/// prefix, and `--some-flag` as `some_flag`.
pub fn clean_message(text: &str) -> String {
    let text = text.strip_prefix("error: ").unwrap_or(text);
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

impl From<anyhow::Error> for ToolError {
    fn from(e: anyhow::Error) -> Self {
        ToolError::new(clean_message(&format!("{e:#}")))
    }
}

/// Messages of the root check already name what to do (`--root` is the server's own option).
impl From<String> for ToolError {
    fn from(e: String) -> Self {
        ToolError::new(e)
    }
}

type Res = Result<ToolOut, ToolError>;

fn args<T: DeserializeOwned>(v: &Value) -> Result<T, ToolError> {
    serde_json::from_value(v.clone()).map_err(|e| {
        let text = e.to_string().replace('`', "\"");
        ToolError::new(text)
    })
}

pub fn list() -> Vec<Value> {
    schemas::defs()
        .into_iter()
        .map(|d: Def| {
            json!({
                "name": d.name,
                "description": d.description,
                "inputSchema": d.schema,
                "annotations": {"readOnlyHint": d.read_only}
            })
        })
        .collect()
}

pub fn exists(name: &str) -> bool {
    schemas::defs().iter().any(|d| d.name == name)
}

pub fn call(server: &mut Server, name: &str, a: &Value) -> Res {
    match name {
        "new_project" => new_project(server, a),
        "open_project" => open_project(server, a),
        "save_project" => save_project(server, a),
        "import_files" => import_files(server, a),
        "list_contents" => list_contents(server, a),
        "read_columns" => read_columns(server, a),
        "set_column_values" => set_column_values(server, a),
        "plot" => plot(server, a),
        "export_graph" => export_graph(server, a),
        "export_data" => export_data(server, a),
        "list_analysis_tools" => {
            let _: Empty = args(a)?;
            Ok(ToolOut::json(analysis_tools::describe()))
        }
        "analyze" => analyze(server, a),
        "sort_sheet" => sort_sheet(server, a),
        "filter_sheet" => filter_sheet(server, a),
        "list_fit_models" => list_fit_models(a),
        "fit" => fit(server, a),
        other => Err(ToolError::new(format!("unknown tool: {other}"))),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}

// ----- names and values --------------------------------------------------------------

/// A workbook name as the project knows it: a long name is turned into the short name. A long
/// name that two workbooks share is refused, not resolved to the first.
fn book_name(project: &Project, name: Option<&str>) -> Result<Option<String>, ToolError> {
    let Some(n) = name else { return Ok(None) };
    if project.window_by_name(n).is_some() {
        return Ok(Some(n.to_string()));
    }
    let same: Vec<&str> = project
        .workbooks()
        .filter(|b| !b.long_name.is_empty() && b.long_name.eq_ignore_ascii_case(n))
        .map(|b| b.name.as_str())
        .collect();
    match same.as_slice() {
        [] => Ok(Some(n.to_string())),
        [one] => Ok(Some((*one).to_string())),
        many => Err(ToolError::new(format!(
            "the workbook name \"{n}\" is the long name of several workbooks ({}); use a short name",
            many.join(", ")
        ))),
    }
}

/// Refuses a column name that is no short name of the sheet and the long name of several columns
/// (the shared code would take the first one).
fn check_names(project: &Project, book: Option<&str>, sheet: Option<&str>, names: &[String]) -> Result<(), ToolError> {
    let Ok((b, i)) = locate_sheet(project, book, sheet) else { return Ok(()) };
    let Some(ws) = project.workbook(b).and_then(|b| b.sheets.get(i)) else { return Ok(()) };
    for n in names {
        let n = n.trim();
        if ws.columns.iter().any(|c| c.short_name.eq_ignore_ascii_case(n)) {
            continue;
        }
        let same: Vec<&str> = ws.columns.iter().filter(|c| c.long_name == n).map(|c| c.short_name.as_str()).collect();
        if same.len() > 1 {
            return Err(ToolError::new(format!(
                "the column name \"{n}\" is the long name of several columns ({}); use a short name",
                same.join(", ")
            )));
        }
    }
    Ok(())
}

fn locate_sheet(project: &Project, book: Option<&str>, sheet: Option<&str>) -> Result<(WindowId, usize), ToolError> {
    let name = book_name(project, book)?;
    let b = pick_book(project, name.as_deref())?;
    let s = pick_sheet(project, b, sheet)?;
    Ok((b, s))
}

fn qualified(project: &Project, id: ColumnId) -> String {
    match project.locate(id) {
        Some((b, s, c)) => format!("{}:{}:{}", b.name, s.name, c.short_name),
        None => "?".to_string(),
    }
}

fn sheet_name(project: &Project, book: WindowId, sheet: usize) -> String {
    project.workbook(book).and_then(|b| b.sheets.get(sheet)).map(|s| s.name.clone()).unwrap_or_default()
}

fn book_title(project: &Project, book: WindowId) -> String {
    project.workbook(book).map(|b| b.name.clone()).unwrap_or_default()
}

fn designation_name(d: Designation) -> &'static str {
    match d {
        Designation::X => "X",
        Designation::Y => "Y",
        Designation::Z => "Z",
        Designation::YError => "YError",
        Designation::XError => "XError",
        Designation::Label => "Label",
        Designation::Disregard => "Disregard",
        Designation::Group => "Group",
        Designation::Subject => "Subject",
    }
}

fn parse_designation(text: &str) -> Result<Designation, ToolError> {
    Designation::ALL.into_iter().find(|d| designation_name(*d).eq_ignore_ascii_case(text.trim())).ok_or_else(|| {
        ToolError::new(format!(
            "designation \"{text}\" is not one of X, Y, Z, YError, XError, Label, Disregard, Group, Subject"
        ))
    })
}

/// One cell as JSON: a number, a string, or null for a missing value. Infinite values are strings.
pub fn cell_value(c: &Column, row: usize) -> Value {
    let number = |x: f64| -> Value {
        if c.format.is_date_time() && x.is_finite() {
            return json!(c.cell_text(row));
        }
        if x.is_nan() {
            Value::Null
        } else if x == f64::INFINITY {
            json!("Infinity")
        } else if x == f64::NEG_INFINITY {
            json!("-Infinity")
        } else {
            json!(x)
        }
    };
    match &c.data {
        ColumnData::Numeric(v) => v.get(row).map_or(Value::Null, |x| number(*x)),
        ColumnData::Text(v) => v.get(row).filter(|t| !t.is_empty()).map_or(Value::Null, |t| json!(t)),
        ColumnData::Mixed { numbers, .. } => match c.data.text_cell(row) {
            Some(t) => json!(t),
            None => numbers.get(row).map_or(Value::Null, |x| number(*x)),
        },
    }
}

fn kind_name(c: &Column) -> &'static str {
    match &c.data {
        ColumnData::Numeric(_) if c.format.is_date_time() => "date",
        ColumnData::Numeric(_) => "numeric",
        ColumnData::Text(_) => "text",
        ColumnData::Mixed { .. } => "mixed",
    }
}

/// The contents of the project (or one workbook) as `list_contents` shows them.
pub fn contents(project: &Project, only: Option<WindowId>) -> Value {
    fn folder(project: &Project, f: &sheaf_core::project::Folder) -> Value {
        let windows: Vec<&str> = f.windows.iter().filter_map(|id| project.window(*id).map(Window::name)).collect();
        let folders: Vec<Value> = f.folders.iter().map(|c| folder(project, c)).collect();
        json!({"name": f.name, "windows": windows, "folders": folders})
    }
    let workbooks: Vec<Value> = project
        .workbooks()
        .filter(|b| only.is_none_or(|id| id == b.id))
        .map(|b| {
            let sheets: Vec<Value> = b
                .sheets
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let columns: Vec<Value> = s
                        .columns
                        .iter()
                        .map(|c| {
                            json!({
                                "short_name": c.short_name, "long_name": c.long_name, "units": c.units,
                                "comments": c.comments, "designation": designation_name(c.designation),
                                "rows": c.len(), "kind": kind_name(c), "formula": c.formula,
                            })
                        })
                        .collect();
                    json!({"name": s.name, "rows": s.row_count(), "active": i == b.active_sheet, "columns": columns})
                })
                .collect();
            json!({"name": b.name, "long_name": b.long_name, "sheets": sheets})
        })
        .collect();
    let in_book = |g: &sheaf_core::graph::Graph| match only {
        None => true,
        Some(id) => g.referenced_columns().iter().any(|c| project.locate(*c).is_some_and(|(b, _, _)| b.id == id)),
    };
    let graphs: Vec<Value> = project
        .graphs()
        .filter(|g| in_book(g))
        .map(|g| {
            let plots: Vec<Value> = g
                .layers
                .iter()
                .flat_map(|l| &l.plots)
                .map(|p| json!({"y": qualified(project, p.y), "x": p.x.map(|x| qualified(project, x))}))
                .collect();
            json!({
                "name": g.name, "long_name": g.long_name, "layers": g.layers.len(), "plots": plots,
                "width_in": g.page.width_pt / 72.0, "height_in": g.page.height_pt / 72.0,
            })
        })
        .collect();
    json!({"folders": [folder(project, &project.root)], "workbooks": workbooks, "graphs": graphs})
}

fn plot_count(project: &Project, id: WindowId) -> usize {
    project.graph(id).map_or(0, |g| g.layers.iter().map(|l| l.plots.len()).sum())
}

/// The first rows of a sheet as headers and rows, for results that live in a sheet.
fn sheet_table(project: &Project, book: WindowId, sheet: usize) -> Value {
    let Some(ws) = project.workbook(book).and_then(|b| b.sheets.get(sheet)) else { return Value::Null };
    let headers: Vec<&str> = ws
        .columns
        .iter()
        .map(|c| if c.long_name.is_empty() { c.short_name.as_str() } else { c.long_name.as_str() })
        .collect();
    let max_rows = (CELL_CAP / ws.columns.len().max(1)).min(ws.row_count());
    let rows: Vec<Value> =
        (0..max_rows).map(|r| Value::Array(ws.columns.iter().map(|c| cell_value(c, r)).collect())).collect();
    json!({"sheet": ws.name, "headers": headers, "rows": rows, "total_rows": ws.row_count()})
}

impl Server {
    /// Runs `f` on the project; when it fails the project is put back as it was.
    fn transact<T>(&mut self, f: impl FnOnce(&mut Server) -> Result<T, ToolError>) -> Result<T, ToolError> {
        let before = self.project.clone();
        let result = f(self);
        if result.is_err() {
            self.project = before;
        }
        result
    }

    fn open_path(&self, text: &str) -> Result<PathBuf, ToolError> {
        Ok(self.roots.check(text)?)
    }

    fn suggested_for(&self, book: Option<&str>) -> Template {
        let name = book_name(&self.project, book)
            .ok()
            .flatten()
            .or_else(|| self.project.workbooks().next().map(|b| b.name.clone()));
        name.and_then(|n| self.suggested.get(&n.to_ascii_lowercase()).copied()).unwrap_or(Template::Line)
    }
}

// ----- project -----------------------------------------------------------------------

fn new_project(s: &mut Server, a: &Value) -> Res {
    let _: Empty = args(a)?;
    s.project = Project::new();
    s.path = None;
    s.foreign.clear();
    s.suggested.clear();
    Ok(ToolOut::json(json!({"summary": contents(&s.project, None)})))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PathArgs {
    path: String,
}

fn open_project(s: &mut Server, a: &Value) -> Res {
    let a: PathArgs = args(a)?;
    let path = s.roots.check_open(&a.path)?;
    if !sheaf_ops::session::is_project(&path) {
        return Err(ToolError::new(format!(
            "'{}' is not a project (.opj, .opju or .sheaf); use import_files for data files",
            a.path
        )));
    }
    let loaded = sheaf_ops::session::load_project(&path)?;
    s.project = loaded.project;
    s.path = Some(path.clone());
    s.suggested.clear();
    s.foreign.clear();
    if loaded.foreign {
        s.foreign.push(path.clone());
    }
    Ok(ToolOut::json(json!({
        "path": path, "foreign": loaded.foreign, "companion_used": loaded.companion_used,
        "warnings": loaded.warnings, "summary": contents(&s.project, None),
    })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveArgs {
    path: Option<String>,
}

fn save_project(s: &mut Server, a: &Value) -> Res {
    let a: SaveArgs = args(a)?;
    let path = match (&a.path, &s.path) {
        (Some(p), _) => s.roots.check_save(p)?,
        (None, Some(p)) => s.roots.check_save(&p.display().to_string())?,
        (None, None) => return Err(ToolError::new("give a path: the project has not been opened or saved yet")),
    };
    let target = sheaf_io::save::default_extension(&path);
    let backup = s.foreign.iter().any(|f| sheaf_ops::session::same_file(f, &target));
    let report = sheaf_ops::session::save_project(&s.project, &path, backup)?;
    if let Some(reason) = &report.refusal {
        let companion = report.companion.as_ref().map(|c| c.display().to_string()).unwrap_or_default();
        return Err(ToolError {
            message: format!(
                "'{}' was not written because Origin cannot store the project: {reason}. Rename as it says and save again; the project is safe in '{companion}'",
                report.path.display()
            ),
            data: Some(json!({"companion": companion, "refusal": reason})),
        });
    }
    s.path = Some(report.path.clone());
    Ok(ToolOut::json(json!({
        "path": report.path, "companion": report.companion, "companion_removed": report.companion_removed,
        "backup": report.backup, "warnings": report.warnings,
    })))
}

// ----- import ------------------------------------------------------------------------

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
struct AsciiArgs {
    delimiter: Option<String>,
    header_lines: Option<usize>,
    long_name_line: Option<usize>,
    units_line: Option<usize>,
    comments_line: Option<usize>,
    decimal_comma: bool,
    thousands: Option<String>,
    skip_rows: usize,
    max_rows: usize,
    dates: Option<String>,
    date_order: Option<String>,
}

/// Which sheets of an Excel file to read. The header and row settings are those of `ascii`.
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
struct ExcelArgs {
    sheets: Vec<String>,
    include_hidden: bool,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
struct ChiArgs {
    split_scans: Option<bool>,
    split_segments: Option<bool>,
    add_milliamp: Option<bool>,
    add_charge: Option<bool>,
    invert_current: bool,
    exact_time: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
struct ImportArgs {
    files: Vec<String>,
    layout: Option<String>,
    book_name: Option<String>,
    format: Option<String>,
    file_name_in_comments: bool,
    ascii: AsciiArgs,
    chi: ChiArgs,
    excel: ExcelArgs,
    plot: bool,
}

fn one_of(field: &str, value: &str, choices: &[&str]) -> Result<String, ToolError> {
    let v = value.trim().to_ascii_lowercase();
    if choices.contains(&v.as_str()) {
        Ok(v)
    } else {
        Err(ToolError::new(format!("{field} must be one of {} (got \"{value}\")", choices.join(", "))))
    }
}

#[allow(clippy::field_reassign_with_default)]
fn read_settings(a: &ImportArgs) -> Result<ReadSettings, ToolError> {
    let mut r = ReadSettings::default();
    r.format = match one_of("format", a.format.as_deref().unwrap_or("auto"), &["auto", "chi", "ascii"])?.as_str() {
        "chi" => DataFormat::Chi,
        "ascii" => DataFormat::Ascii,
        _ => DataFormat::Auto,
    };
    let (x, c) = (&a.ascii, &a.chi);
    r.no_split_scans = c.split_scans == Some(false);
    r.no_split_segments = c.split_segments == Some(false);
    r.no_milliamp = c.add_milliamp == Some(false);
    r.no_charge = c.add_charge == Some(false);
    r.invert_current = c.invert_current;
    r.keep_file_times = c.exact_time == Some(false);
    r.delimiter = x.delimiter.clone().filter(|d| !d.eq_ignore_ascii_case("auto"));
    r.header_lines = x.header_lines;
    r.long_name_line = x.long_name_line;
    r.units_line = x.units_line;
    r.comments_line = x.comments_line;
    r.decimal_comma = x.decimal_comma;
    r.thousands = match one_of(
        "thousands",
        x.thousands.as_deref().unwrap_or("auto"),
        &["auto", "off", "comma", "dot", "space", "apostrophe"],
    )?
    .as_str()
    {
        "off" => ThousandsSetting::None,
        "comma" => ThousandsSetting::Comma,
        "dot" => ThousandsSetting::Dot,
        "space" => ThousandsSetting::Space,
        "apostrophe" => ThousandsSetting::Apostrophe,
        _ => ThousandsSetting::Auto,
    };
    let too_big = || ToolError::new("skip_rows and max_rows are too large");
    r.first_row = x.skip_rows.checked_add(1).ok_or_else(too_big)?;
    if x.max_rows > 0 {
        r.last_row = r.first_row.checked_add(x.max_rows - 1).ok_or_else(too_big)?;
    }
    r.dates_as_text = one_of("dates", x.dates.as_deref().unwrap_or("detect"), &["detect", "text"])? == "text";
    r.day_first = one_of("date_order", x.date_order.as_deref().unwrap_or("mdy"), &["mdy", "dmy"])? == "dmy";
    r.sheets = a.excel.sheets.iter().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
    r.hidden_sheets = a.excel.include_hidden;
    Ok(r)
}

/// Checks the files an import names (folders and wildcards expanded, every match checked).
fn expand_checked(s: &Server, texts: &[String]) -> Result<Vec<PathBuf>, ToolError> {
    let mut out = Vec::new();
    for text in texts {
        let path = s.open_path(text)?;
        if sheaf_ops::session::is_project(&path) {
            // A project is opened with its `.opj.sheaf` companion: check that too.
            out.push(s.roots.check_open(&path.display().to_string())?);
            continue;
        }
        for m in sheaf_io::batch::expand_inputs(&path).map_err(ToolError::new)? {
            let checked = s.roots.check_path(&m, &m.display().to_string())?;
            if let Ok(meta) = std::fs::metadata(&checked)
                && meta.is_file()
                && meta.len() > FILE_CAP
            {
                return Err(ToolError::new(format!("'{}' is larger than 256 MB", checked.display())));
            }
            out.push(checked);
        }
    }
    Ok(out)
}

fn import_files(s: &mut Server, a: &Value) -> Res {
    let a: ImportArgs = args(a)?;
    if a.files.is_empty() {
        return Err(ToolError::new("give at least one file in \"files\""));
    }
    let layout = match one_of("layout", a.layout.as_deref().unwrap_or("books"), &["books", "sheets"])?.as_str() {
        "sheets" => ImportLayout::Sheets,
        _ => ImportLayout::Books,
    };
    let request = ImportRequest {
        files: expand_checked(s, &a.files)?,
        layout,
        read: read_settings(&a)?,
        book_name: a.book_name.clone(),
        file_name_comments: a.file_name_in_comments,
        partial_except_first: false,
        plot: a.plot,
    };
    let report = s.transact(|s| Ok(sheaf_ops::import::import_files(&mut s.project, &request)?))?;
    s.foreign.extend(report.foreign_inputs.iter().cloned());
    for item in &report.items {
        if let Some(t) = Template::from_name(&item.suggested_template) {
            s.suggested.entry(item.book.to_ascii_lowercase()).or_insert(t);
        }
    }
    if report.items.is_empty() && report.data_files > 0 {
        return Err(ToolError::new(format!(
            "none of the {} file(s) could be imported: {}",
            report.data_files,
            report.errors.join("; ")
        )));
    }
    let imported: Vec<Value> = report
        .items
        .iter()
        .map(|i| {
            json!({
                "file": i.file, "book": i.book, "sheet": sheet_by_index(&s.project, &i.book, i.sheet),
                "format": i.format, "suggested_template": i.suggested_template, "warnings": i.warnings,
            })
        })
        .collect();
    Ok(ToolOut::json(json!({
        "imported": imported, "errors": report.errors, "graphs": report.graphs,
        "notes": report.notes, "warnings": report.project_warnings,
    })))
}

fn sheet_by_index(project: &Project, book: &str, sheet: usize) -> String {
    match project.window_by_name(book) {
        Some(Window::Workbook(b)) => b.sheets.get(sheet).map(|s| s.name.clone()).unwrap_or_default(),
        _ => String::new(),
    }
}

// ----- looking and editing -----------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BookArgs {
    book: Option<String>,
}

fn list_contents(s: &mut Server, a: &Value) -> Res {
    let a: BookArgs = args(a)?;
    let only = match &a.book {
        None => None,
        Some(_) => {
            let name = book_name(&s.project, a.book.as_deref())?;
            Some(pick_book(&s.project, name.as_deref())?)
        }
    };
    Ok(ToolOut::json(contents(&s.project, only)))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadArgs {
    book: Option<String>,
    sheet: Option<String>,
    columns: Option<Vec<String>>,
    from: Option<i64>,
    to: Option<i64>,
    limit: Option<i64>,
}

fn read_columns(s: &mut Server, a: &Value) -> Res {
    let a: ReadArgs = args(a)?;
    let (book, sheet) = locate_sheet(&s.project, a.book.as_deref(), a.sheet.as_deref())?;
    check_names(&s.project, a.book.as_deref(), a.sheet.as_deref(), a.columns.as_deref().unwrap_or_default())?;
    let wb = s.project.workbook(book).ok_or_else(|| anyhow!("workbook not found"))?;
    let ws = &wb.sheets[sheet];
    let ids: Vec<ColumnId> = match &a.columns {
        Some(names) if !names.is_empty() => named_columns(&s.project, book, sheet, names)?,
        _ => ws.columns.iter().map(|c| c.id).collect(),
    };
    let cols: Vec<&Column> = ids.iter().filter_map(|id| ws.column(*id)).collect();
    if cols.is_empty() {
        return Err(ToolError::new("the sheet has no columns"));
    }
    let total = cols.iter().map(|c| c.len()).max().unwrap_or(0);
    let from = a.from.unwrap_or(1);
    let to = a.to.unwrap_or(total as i64);
    if from < 1 {
        return Err(ToolError::new("from counts rows from 1"));
    }
    if let Some(l) = a.limit
        && l < 1
    {
        return Err(ToolError::new("limit must be at least 1"));
    }
    let cap = a.limit.map_or(CELL_CAP, |l| (l as usize).min(CELL_CAP));
    if cols.len() > cap {
        return Err(ToolError::new(format!(
            "{} columns exceed the cap of {cap} cells per reply; name fewer columns",
            cols.len()
        )));
    }
    let per_page = (cap / cols.len()).max(1);
    let (from, to) = (from as usize, (to.max(0) as usize).min(total));
    let last = if from > to { from - 1 } else { to.min(from + per_page - 1) };
    let rows: Vec<Value> =
        (from..=last).map(|r| Value::Array(cols.iter().map(|c| cell_value(c, r - 1)).collect())).collect();
    let truncated = last < to;
    let mut out = json!({
        "book": wb.name, "sheet": ws.name,
        "columns": cols.iter().map(|c| c.short_name.as_str()).collect::<Vec<_>>(),
        "from": from, "to": last, "total_rows": total, "rows": rows, "truncated": truncated,
    });
    if truncated {
        out["next"] = json!({"from": last + 1, "to": to});
    }
    Ok(ToolOut::json(out))
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
struct CreateArgs {
    designation: Option<String>,
    long_name: Option<String>,
    units: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SetArgs {
    book: Option<String>,
    sheet: Option<String>,
    column: String,
    formula: String,
    rows: Option<String>,
    create: Option<CreateArgs>,
}

/// "FROM:TO" as 0-based rows, an empty end meaning the first or the last row.
fn fill_range(text: &str, row_count: usize) -> Result<std::ops::Range<usize>, ToolError> {
    let bad = || ToolError::new(format!("'{text}' is not a valid row range; write it as FROM:TO, for example 1:100"));
    let (a, b) = text.split_once(':').ok_or_else(bad)?;
    let from: usize = if a.trim().is_empty() { 1 } else { a.trim().parse().map_err(|_| bad())? };
    let to: usize = if b.trim().is_empty() { row_count } else { b.trim().parse().map_err(|_| bad())? };
    if from < 1 || to < from {
        return Err(ToolError::new(format!("rows {from} to {to} are not a valid range (rows count from 1)")));
    }
    Ok(from - 1..to)
}

fn set_column_values(s: &mut Server, a: &Value) -> Res {
    let a: SetArgs = args(a)?;
    let (book, sheet) = locate_sheet(&s.project, a.book.as_deref(), a.sheet.as_deref())?;
    let rows_in_sheet = s.project.workbook(book).map_or(0, |b| b.sheets[sheet].row_count());
    let range = a.rows.as_deref().map(|t| fill_range(t, rows_in_sheet)).transpose()?;
    if rows_in_sheet.max(range.as_ref().map_or(0, |r| r.end)) > ROW_CAP {
        return Err(ToolError::new(format!("set_column_values works on at most {ROW_CAP} rows")));
    }
    let ws = &s.project.workbook(book).ok_or_else(|| ToolError::new("workbook not found"))?.sheets[sheet];
    let wanted = a.column.trim();
    // With `create` the name is a new short name: a long name of a column (the X column's, say) must
    // not make the call overwrite that column.
    if a.create.is_some() && !ws.columns.iter().any(|c| c.short_name.eq_ignore_ascii_case(wanted)) {
        let owners: Vec<&str> =
            ws.columns.iter().filter(|c| c.long_name == wanted).map(|c| c.short_name.as_str()).collect();
        if !owners.is_empty() {
            return Err(ToolError::new(format!(
                "\"{wanted}\" is the long name of column {}; give its short name to change it, or choose another name for the new column",
                owners.join(", ")
            )));
        }
    }
    check_names(&s.project, a.book.as_deref(), a.sheet.as_deref(), std::slice::from_ref(&a.column))?;
    let existing = s.project.workbook(book).and_then(|b| b.sheets[sheet].column_by_name(wanted)).map(|c| c.id);
    let (column, created) = match (existing, &a.create) {
        (Some(id), _) => (id, false),
        (None, Some(c)) => {
            let designation = match &c.designation {
                Some(d) => parse_designation(d)?,
                None => Designation::Y,
            };
            let name = a.column.trim();
            if name.is_empty() || name.chars().any(|ch| ch.is_whitespace() || ch == ':') {
                return Err(ToolError::new(format!("'{name}' is not a usable column name")));
            }
            let id = s
                .project
                .add_column(book, sheet, designation, ColumnData::numeric(Vec::new()))
                .map_err(|e| ToolError::new(e.to_string()))?;
            if let Some(col) = s.project.column_mut(id) {
                col.short_name = name.to_string();
                col.long_name = c.long_name.clone().unwrap_or_default();
                col.units = c.units.clone().unwrap_or_default();
            }
            (id, true)
        }
        (None, None) => {
            let id = named_columns(&s.project, book, sheet, std::slice::from_ref(&a.column)).map(|v| v[0]);
            return Err(match id {
                Err(e) => ToolError::from(e),
                Ok(_) => ToolError::new("column not found"),
            });
        }
    };
    let done = s.project.set_column_values(column, &a.formula, range.clone());
    if let Err(e) = done {
        if created && let Some(b) = s.project.workbook_mut(book) {
            b.sheets[sheet].columns.retain(|c| c.id != column);
        }
        return Err(ToolError::new(clean_message(&format!("could not set the column: {e}"))));
    }
    let col = s.project.column(column).ok_or_else(|| ToolError::new("column not found"))?;
    let recalculate = match col.recalc {
        RecalcMode::None => "none",
        RecalcMode::Auto => "auto",
        RecalcMode::Manual => "manual",
    };
    let rows = range.map_or_else(|| s.project.workbook(book).map_or(0, |b| b.sheets[sheet].row_count()), |r| r.len());
    Ok(ToolOut::json(json!({"column": col.short_name, "created": created, "rows": rows, "recalculate": recalculate})))
}

// ----- plot and export ---------------------------------------------------------------

#[derive(Deserialize)]
#[serde(untagged)]
enum Many<T> {
    One(T),
    List(Vec<T>),
}

fn many<T>(m: Option<Many<T>>) -> Vec<T> {
    match m {
        None => Vec::new(),
        Some(Many::One(x)) => vec![x],
        Some(Many::List(v)) => v,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PanelArgs {
    shape: String,
    per_panel: Option<Many<usize>>,
    y: Option<Vec<Vec<String>>>,
    gap: Option<Many<f64>>,
    margins: Option<Vec<f64>>,
    order: Option<String>,
    link: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
struct SizeArgs {
    width_in: Option<f64>,
    height_in: Option<f64>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
struct PlotArgs {
    template: Option<String>,
    book: Option<String>,
    sheet: Option<String>,
    y: Vec<String>,
    y2: Vec<String>,
    graph: Option<String>,
    panels: Option<PanelArgs>,
    overlay_files: Vec<String>,
    options: Option<Map<String, Value>>,
    style: Option<Map<String, Value>>,
    size: Option<SizeArgs>,
}

fn shape_text(o: &Map<String, Value>) -> Result<String, ToolError> {
    let kind = o.get("kind").and_then(Value::as_str).ok_or_else(|| ToolError::new("a shape needs \"kind\""))?;
    let n = |k: &str| {
        o.get(k).and_then(Value::as_f64).ok_or_else(|| ToolError::new(format!("a shape needs the number \"{k}\"")))
    };
    Ok(format!("{kind}@{},{},{},{}", n("x1")?, n("y1")?, n("x2")?, n("y2")?))
}

fn style_from(raw: Option<Map<String, Value>>, size: Option<&SizeArgs>) -> Result<Style, ToolError> {
    let mut map = raw.unwrap_or_default();
    let text_labels = map.remove("text_labels");
    let draw = map.remove("draw");
    let allowed: Vec<String> = match serde_json::to_value(Style::default()) {
        Ok(Value::Object(o)) => o.keys().filter(|k| *k != "label" && *k != "draw").cloned().collect(),
        _ => Vec::new(),
    };
    if let Some(bad) = map.keys().find(|k| !allowed.contains(k)) {
        return Err(ToolError::new(format!("unknown field \"{bad}\" in style")));
    }
    let mut style: Style = serde_json::from_value(Value::Object(map))
        .map_err(|e| ToolError::new(format!("style: {}", e.to_string().replace('`', "\""))))?;
    if let Some(Value::Array(items)) = text_labels {
        for item in items {
            let o = item.as_object().ok_or_else(|| ToolError::new("a text label is an object with text, x and y"))?;
            let text =
                o.get("text").and_then(Value::as_str).ok_or_else(|| ToolError::new("a text label needs \"text\""))?;
            let (x, y) = (o.get("x").and_then(Value::as_f64), o.get("y").and_then(Value::as_f64));
            let (Some(x), Some(y)) = (x, y) else {
                return Err(ToolError::new("a text label needs the numbers \"x\" and \"y\""));
            };
            style.label.push(format!("{text}@{x},{y}"));
        }
    }
    match draw {
        Some(Value::Array(items)) => {
            for item in items {
                style.draw.push(match item {
                    Value::String(t) => t,
                    Value::Object(o) => shape_text(&o)?,
                    _ => return Err(ToolError::new("a shape is an object with kind, x1, y1, x2 and y2")),
                });
            }
        }
        Some(_) => return Err(ToolError::new("draw must be a list of shapes")),
        None => {}
    }
    if let Some(sz) = size {
        if (sz.width_in.is_some() && style.width.is_some()) || (sz.height_in.is_some() && style.height.is_some()) {
            return Err(ToolError::new("give the page size in size or in style, not both"));
        }
        style.width = sz.width_in.or(style.width);
        style.height = sz.height_in.or(style.height);
    }
    Ok(style)
}

fn enum_from_text<T: DeserializeOwned>(field: &str, text: &str, choices: &str) -> Result<T, ToolError> {
    serde_json::from_value(Value::String(text.trim().to_ascii_lowercase()))
        .map_err(|_| ToolError::new(format!("panels.{field} must be one of {choices} (got \"{text}\")")))
}

fn panels_from(p: PanelArgs) -> Result<PanelsSpec, ToolError> {
    Ok(PanelsSpec {
        shape: p.shape,
        panel_y: p.y.unwrap_or_default().into_iter().map(|g| g.join(",")).collect(),
        per_panel: many(p.per_panel),
        gap: many(p.gap),
        margins: p.margins.unwrap_or_default(),
        order: p.order.as_deref().map(|o| enum_from_text("order", o, "bottom_up, top_down")).transpose()?,
        link: p.link.as_deref().map(|l| enum_from_text("link", l, "x, y, xy, none")).transpose()?,
    })
}

// Combine the structured interface with CLI option names before clap checks conflicts and requirements.
fn bridge_insert(map: &mut Map<String, Value>, key: &str, value: Value) -> Result<(), ToolError> {
    if value.is_null() {
        return Ok(());
    }
    map.retain(|k, v| k.replace('-', "_") != key || !v.is_null());
    if map.keys().any(|k| k.replace('-', "_") == key) {
        return Err(ToolError::new(format!("option '{key}' is given in both options and a structured argument")));
    }
    map.insert(key.to_string(), value);
    Ok(())
}

fn session_options(raw: &Value, command: &str) -> Result<Map<String, Value>, ToolError> {
    let obj = raw.as_object().ok_or_else(|| ToolError::new("arguments must be an object"))?;
    let mut map = match obj.get("options") {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(m)) => m.clone(),
        _ => return Err(ToolError::new("options must be an object")),
    };
    let common: &[&str] = if command == "plot" {
        &["template", "book", "sheet", "y", "y2", "graph"]
    } else {
        &[
            "model",
            "formula",
            "params",
            "fdf",
            "book",
            "sheet",
            "y",
            "rows",
            "x_range",
            "yerr",
            "xerr",
            "confidence",
            "bands",
            "max_iterations",
            "tolerance",
            "rank",
            "regress",
        ]
    };
    for key in common {
        if let Some(v) = obj.get(*key) {
            let normalized = if command == "fit" && *key == "bands" {
                v.as_str().map(|x| Value::String(x.trim().to_ascii_lowercase())).unwrap_or_else(|| v.clone())
            } else {
                v.clone()
            };
            bridge_insert(&mut map, key, normalized)?;
        }
    }
    if command == "plot" {
        if let Some(Value::Object(style)) = obj.get("style") {
            let converted = style_from(Some(style.clone()), None)?;
            let converted = serde_json::to_value(converted).map_err(|e| ToolError::new(e.to_string()))?;
            for key in style.keys() {
                let dest = if key == "text_labels" { "label" } else { key };
                bridge_insert(&mut map, dest, converted[dest].clone())?;
            }
        }
        if let Some(Value::Object(size)) = obj.get("size") {
            for (from, to) in [("width_in", "width"), ("height_in", "height")] {
                if let Some(v) = size.get(from) {
                    bridge_insert(&mut map, to, v.clone())?;
                }
            }
        }
        if let Some(Value::Object(p)) = obj.get("panels") {
            for (from, to) in [
                ("shape", "panels"),
                ("per_panel", "per_panel"),
                ("gap", "panel_gap"),
                ("margins", "panel_margins"),
                ("order", "panel_order"),
                ("link", "panel_link"),
            ] {
                if let Some(v) = p.get(from) {
                    let v = if from == "order" || from == "link" {
                        v.as_str()
                            .map(|s| Value::String(s.trim().to_ascii_lowercase().replace('_', "-")))
                            .unwrap_or_else(|| v.clone())
                    } else {
                        v.clone()
                    };
                    bridge_insert(&mut map, to, v)?;
                }
            }
            if let Some(Value::Array(groups)) = p.get("y") {
                let groups = groups
                    .iter()
                    .map(|g| {
                        g.as_array()
                            .map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(","))
                            .map(Value::String)
                            .unwrap_or_else(|| g.clone())
                    })
                    .collect();
                bridge_insert(&mut map, "panel_y", Value::Array(groups))?;
            }
        }
    } else {
        for (from, to) in [("weighting", "weight"), ("band_level", "level"), ("compare_with", "compare")] {
            if let Some(v) = obj.get(from) {
                let normalized = if from == "weighting" {
                    v.as_str().map(|x| Value::String(x.trim().to_ascii_lowercase())).unwrap_or_else(|| v.clone())
                } else {
                    v.clone()
                };
                bridge_insert(&mut map, to, normalized)?;
            }
        }
        if let Some(v) = obj.get("scale_errors").filter(|v| !v.is_null()) {
            bridge_insert(
                &mut map,
                "no_scale_errors",
                Value::Bool(!v.as_bool().ok_or_else(|| ToolError::new("scale_errors must be boolean"))?),
            )?;
        }
        for key in ["init", "fix", "bounds"] {
            if let Some(Value::Object(entries)) = obj.get(key) {
                let mut values = Vec::new();
                for (name, value) in entries {
                    let text = if key == "bounds" {
                        let a = value.as_array().filter(|a| a.len() == 2).ok_or_else(|| {
                            ToolError::new(format!("bounds of \"{name}\" must be [low, high], with null for no bound"))
                        })?;
                        let side = |v: &Value| if v.is_null() { String::new() } else { v.to_string() };
                        format!("{}:{}", side(&a[0]), side(&a[1]))
                    } else {
                        value.to_string()
                    };
                    values.push(Value::String(format!("{name}={text}")));
                }
                bridge_insert(&mut map, key, Value::Array(values))?;
            }
        }
        if let Some(Value::Object(global)) = obj.get("global") {
            bridge_insert(&mut map, "global", json!(true))?;
            if let Some(v) = global.get("share") {
                bridge_insert(&mut map, "share", v.clone())?;
            }
        }
    }
    Ok(map)
}

fn session_cli(s: &Server, command: &str, raw: &Value) -> Result<sheaf_ops::args::Cli, ToolError> {
    let options = session_options(raw, command)?;
    let roots = s.roots.clone();
    let mut resolve = |p: &str| roots.check(p).map(|p| p.to_string_lossy().into_owned()).map_err(|e| anyhow!("{e}"));
    Ok(bridge::parse_session(command, &options, &mut resolve)?)
}

fn plot(s: &mut Server, a_raw: &Value) -> Res {
    let a: PlotArgs = args(a_raw)?;
    if !a.overlay_files.is_empty() && (a.book.is_some() || a.sheet.is_some() || a.graph.is_some()) {
        return Err(ToolError::new("overlay_files cannot be combined with book, sheet or graph"));
    }
    if a.graph.is_some() && (a.template.is_some() || !a.y.is_empty() || !a.y2.is_empty()) {
        return Err(ToolError::new(
            "with graph, plot only restyles that graph; template, y and y2 are not used. Leave graph out to make a new graph",
        ));
    }
    let bridged = if a.options.is_some() {
        let sheaf_ops::args::Command::Plot(cli) = session_cli(s, "plot", a_raw)?.command else { unreachable!() };
        Some(sheaf_ops::cli_plot::plot_spec(&cli))
    } else {
        None
    };
    let mut spec = PlotSpec {
        template: a.template.clone(),
        y: a.y.clone(),
        y2: a.y2.clone(),
        book: book_name(&s.project, a.book.as_deref())?,
        sheet: a.sheet.clone(),
        graph: a.graph.clone(),
        panels: a.panels.map(panels_from).transpose()?,
        overlay: !a.overlay_files.is_empty(),
        overlay_books: Vec::new(),
        style: style_from(a.style, a.size.as_ref())?,
    };
    if let Some(mut parsed) = bridged {
        parsed.book = book_name(&s.project, parsed.book.as_deref())?;
        parsed.overlay = !a.overlay_files.is_empty();
        spec = parsed;
    }
    if !a.overlay_files.is_empty() && (spec.book.is_some() || spec.sheet.is_some() || spec.graph.is_some()) {
        return Err(ToolError::new("overlay_files cannot be combined with book, sheet or graph"));
    }
    if spec.graph.is_some() && (spec.template.is_some() || !spec.y.is_empty() || !spec.y2.is_empty()) {
        return Err(ToolError::new("with graph, plot only restyles that graph; template, y and y2 are not used"));
    }
    let mut ys = spec.y.clone();
    ys.extend(spec.y2.iter().cloned());
    check_names(&s.project, spec.book.as_deref(), spec.sheet.as_deref(), &ys)?;
    spec.validate()?;
    let overlay_files = a.overlay_files;
    let made = s.transact(|s| {
        let mut suggested = s.suggested_for(spec.book.as_deref());
        if !overlay_files.is_empty() {
            let request = ImportRequest {
                files: expand_checked(s, &overlay_files)?,
                layout: ImportLayout::Books,
                read: ReadSettings::default(),
                book_name: None,
                file_name_comments: false,
                partial_except_first: false,
                plot: false,
            };
            let report = sheaf_ops::import::import_files(&mut s.project, &request)?;
            if report.items.is_empty() {
                return Err(ToolError::new(format!("no file could be imported: {}", report.errors.join("; "))));
            }
            if let Some(t) = Template::from_name(&report.items[0].suggested_template) {
                suggested = t;
            }
            for item in &report.items {
                if let Some(t) = Template::from_name(&item.suggested_template) {
                    s.suggested.entry(item.book.to_ascii_lowercase()).or_insert(t);
                }
                if !spec.overlay_books.contains(&item.book) {
                    spec.overlay_books.push(item.book.clone());
                }
            }
        }
        let id = sheaf_ops::plot::build_graph(&mut s.project, &spec, suggested)?;
        Ok(id)
    })?;
    let name = s.project.graph(made).map(|g| g.name.clone()).unwrap_or_default();
    Ok(ToolOut::json(json!({
        "graph": name, "created": spec.graph.is_none(), "plots": plot_count(&s.project, made),
    })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportArgs {
    graph: Option<String>,
    /// Several graphs, in page order, for one multi-page PDF.
    graphs: Option<Vec<String>>,
    /// Every graph of the project, in project order.
    #[serde(default)]
    all_graphs: bool,
    path: Option<String>,
    format: Option<String>,
    dpi: Option<f64>,
    width: Option<String>,
    height: Option<String>,
    margin: Option<String>,
    return_image: Option<bool>,
}

const FORMATS: [&str; 10] = ["png", "svg", "pdf", "eps", "tiff", "tif", "emf", "jpg", "jpeg", "bmp"];

/// Width and height of a PNG from its header.
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || bytes[..8] != [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a] {
        return None;
    }
    let be = |i: usize| u32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    Some((be(16), be(20)))
}

/// The graphs `graphs` or `all_graphs` name, each resolved as `pick_graph` does; `None` when
/// neither is given.
fn export_graph_list(project: &Project, a: &ExportArgs) -> Result<Option<Vec<WindowId>>, ToolError> {
    if a.graph.is_some() && (a.graphs.is_some() || a.all_graphs) {
        return Err(ToolError::new(
            "give graph for one graph, or graphs or all_graphs for several; not graph together with them",
        ));
    }
    let ids = match (&a.graphs, a.all_graphs) {
        (Some(_), true) => return Err(ToolError::new("give graphs or all_graphs, not both")),
        (Some(names), false) if names.len() > MAX_EXPORT_PAGES => return Err(too_many_pages(names.len())),
        (Some(names), false) => names.iter().map(|n| pick_graph(project, Some(n))).collect::<Result<Vec<_>, _>>()?,
        (None, true) => project.graphs().map(|g| g.id).collect(),
        (None, false) => return Ok(None),
    };
    if ids.len() > MAX_EXPORT_PAGES {
        return Err(too_many_pages(ids.len()));
    }
    if ids.is_empty() {
        return Err(ToolError::new(if a.all_graphs { "the project has no graph" } else { "graphs is empty" }));
    }
    Ok(Some(ids))
}

fn too_many_pages(n: usize) -> ToolError {
    ToolError::new(format!(
        "{n} graphs is too many for one PDF; the limit is {MAX_EXPORT_PAGES} pages per call. Export them in several files"
    ))
}

fn check_dpi(dpi: f64) -> Result<(), ToolError> {
    if !(dpi.is_finite() && (36.0..=1200.0).contains(&dpi)) {
        return Err(ToolError::new(format!("dpi must be between 36 and 1200 (got {dpi})")));
    }
    Ok(())
}

/// Fails when the figure of `graph` at this size and dpi would pass the pixel cap.
fn check_pixels(project: &Project, graph: WindowId, size: sheaf_plot::ExportSize, dpi: f64) -> Result<(), ToolError> {
    let page = project.graph(graph).map(|g| (g.page.width_pt, g.page.height_pt)).unwrap_or((360.0, 252.0));
    let scale = size.scale(page.0, page.1).map_err(|e| ToolError::new(e.to_string()))?;
    let (est_w, est_h) = (page.0 / 72.0 * scale * dpi, page.1 / 72.0 * scale * dpi);
    if est_w * est_h > PIXEL_CAP {
        return Err(ToolError::new(format!(
            "the figure would be about {est_w:.0} x {est_h:.0} pixels; the limit is 40 million pixels. Lower dpi or the size"
        )));
    }
    Ok(())
}

/// Several graphs into one PDF, one page per graph (Origin 2018b's multi-page PDF export,
/// expGraph `multipage`). Sheaf's own rules: a path is needed, the format must be pdf, no image
/// is returned, and each page passes the same pixel cap as a one-graph export.
fn export_pages(s: &Server, a: &ExportArgs, ids: &[WindowId], dpi: f64) -> Res {
    let Some(path) = a.path.as_deref() else {
        return Err(ToolError::new("several graphs go into one PDF file; give a path ending in .pdf"));
    };
    let path = s.roots.check(path)?;
    let ext = path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase);
    let format = a.format.as_deref().map(str::to_ascii_lowercase).or(ext.clone());
    if format.as_deref() != Some("pdf") || ext.as_deref().is_some_and(|e| e != "pdf") {
        return Err(ToolError::new(
            "several graphs can only be exported as one multi-page PDF; use a path ending in .pdf, or one graph",
        ));
    }
    if a.return_image == Some(true) {
        return Err(ToolError::new(
            "return_image works with one graph only; export the graphs to the PDF, then one graph to look at",
        ));
    }
    let settings = ExportSettings {
        dpi,
        image_width: a.width.clone(),
        image_height: a.height.clone(),
        margin: MarginSettings { margin: a.margin.clone().unwrap_or_else(|| "auto".to_string()), border_width: None },
    };
    let (size, layout) = settings.resolve()?;
    for &g in ids {
        check_pixels(&s.project, g, size, dpi)?;
    }
    let bytes = sheaf_plot::render_multipage_pdf(&s.project, ids, size, layout)
        .context("could not render the graphs as one PDF")?;
    std::fs::write(&path, &bytes)
        .with_context(|| format!("could not write '{}'", path.display()))
        .map_err(ToolError::from)?;
    let names: Vec<String> = ids.iter().filter_map(|&g| s.project.graph(g)).map(|g| g.name.clone()).collect();
    Ok(ToolOut::json(json!({
        "graph": names.first(), "graphs": names, "pages": ids.len(), "path": path, "format": "pdf",
        "width_px": None::<u32>, "height_px": None::<u32>, "bytes": bytes.len(),
    })))
}

fn export_graph(s: &mut Server, a: &Value) -> Res {
    let a: ExportArgs = args(a)?;
    let list = export_graph_list(&s.project, &a)?;
    let dpi = a.dpi.unwrap_or(300.0);
    if let Some(ids) = &list
        && ids.len() > 1
    {
        check_dpi(dpi)?;
        return export_pages(s, &a, ids, dpi);
    }
    let graph = match list {
        Some(ids) => ids[0],
        None => pick_graph(&s.project, a.graph.as_deref())?,
    };
    let graph_name = s.project.graph(graph).map(|g| g.name.clone()).unwrap_or_default();
    check_dpi(dpi)?;
    let path = a.path.as_deref().map(|p| s.roots.check(p)).transpose()?;
    let from_path = path.as_ref().and_then(|p| p.extension()).and_then(|e| e.to_str()).map(str::to_ascii_lowercase);
    let same = |f: &str, e: &str| {
        let norm = |t: &str| match t.to_ascii_lowercase().as_str() {
            "jpeg" => "jpg".to_string(),
            "tif" => "tiff".to_string(),
            other => other.to_string(),
        };
        norm(f) == norm(e)
    };
    let format = match (&a.format, &from_path) {
        (Some(f), Some(e)) if !same(f, e) => {
            return Err(ToolError::new(format!("format \"{f}\" does not match the extension of the path (.{e})")));
        }
        (Some(f), _) => f.to_ascii_lowercase(),
        (None, Some(e)) => e.clone(),
        (None, None) => "png".to_string(),
    };
    if !FORMATS.contains(&format.as_str()) {
        return Err(ToolError::new(format!(
            "'{format}' is not a figure format; use png, svg, pdf, eps, tiff, emf, jpg or bmp"
        )));
    }
    let want_image = a.return_image.unwrap_or(path.is_none());
    if path.is_none() && format != "png" {
        return Err(ToolError::new(
            "without a path the figure is returned as an image, which is PNG; give a path for other formats",
        ));
    }
    let settings = ExportSettings {
        dpi,
        image_width: a.width.clone(),
        image_height: a.height.clone(),
        margin: MarginSettings { margin: a.margin.clone().unwrap_or_else(|| "auto".to_string()), border_width: None },
    };
    let (size, _) = settings.resolve()?;
    check_pixels(&s.project, graph, size, dpi)?;
    let bytes = sheaf_ops::render::render_graph(&s.project, graph, &settings, &format)?;
    if let Some(p) = &path {
        std::fs::write(p, &bytes)
            .with_context(|| format!("could not write '{}'", p.display()))
            .map_err(ToolError::from)?;
    }
    let dims = if format == "png" { png_size(&bytes) } else { None };
    let image = if want_image {
        let png = if format == "png" {
            bytes.clone()
        } else {
            sheaf_ops::render::render_graph(&s.project, graph, &settings, "png")?
        };
        let png = match png_size(&png) {
            Some((w, h)) if f64::from(w.max(h)) > IMAGE_SIDE => {
                let smaller =
                    ExportSettings { dpi: (dpi * IMAGE_SIDE / f64::from(w.max(h))).max(1.0), ..settings.clone() };
                sheaf_ops::render::render_graph(&s.project, graph, &smaller, "png")?
            }
            _ => png,
        };
        Some(Image { data: b64::encode(&png), mime: "image/png" })
    } else {
        None
    };
    let out = json!({
        "graph": graph_name, "path": path, "format": format,
        "width_px": dims.map(|d| d.0), "height_px": dims.map(|d| d.1), "bytes": bytes.len(),
    });
    Ok(ToolOut { value: out, image })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportDataArgs {
    book: Option<String>,
    sheet: Option<String>,
    path: String,
    format: Option<String>,
    #[serde(default)]
    all_sheets: bool,
}

fn export_data(s: &mut Server, a: &Value) -> Res {
    let a: ExportDataArgs = args(a)?;
    let path = s.roots.check(&a.path)?;
    let (book, sheet) = locate_sheet(&s.project, a.book.as_deref(), a.sheet.as_deref())?;
    let ws = &s.project.workbook(book).ok_or_else(|| anyhow!("workbook not found"))?.sheets[sheet];
    let chosen = a.format.as_deref().map(|f| one_of("format", f, &["csv", "tsv", "xlsx"])).transpose()?;
    if chosen.as_deref() == Some("xlsx") || (chosen.is_none() && sheaf_ops::tabular::is_xlsx_path(&path)) {
        let request = sheaf_ops::tabular::XlsxRequest { all_sheets: a.all_sheets, ..Default::default() };
        let out = sheaf_ops::tabular::write_xlsx(&s.project, book, a.sheet.as_deref(), &request, &path)?;
        return Ok(ToolOut::json(json!({
            "path": out.path, "sheets": out.sheet_names, "warnings": out.warnings,
            "rows": ws.row_count(), "columns": ws.columns.len(),
        })));
    }
    let format = match chosen.as_deref() {
        Some("tsv") => SheetFormat::Tsv,
        Some(_) => SheetFormat::Csv,
        None => SheetFormat::from_path(&path),
    };
    let bytes = sheaf_ops::tabular::sheet_to_bytes(ws, format)?;
    std::fs::write(&path, bytes)
        .with_context(|| format!("could not write '{}'", path.display()))
        .map_err(ToolError::from)?;
    Ok(ToolOut::json(json!({"path": path, "rows": ws.row_count(), "columns": ws.columns.len()})))
}

// ----- analysis ----------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AnalyzeArgs {
    tool: String,
    options: Option<Map<String, Value>>,
    book: Option<String>,
    sheet: Option<String>,
    columns: Option<Vec<String>>,
    rows: Option<String>,
    x_range: Option<String>,
    output: Option<String>,
}

fn report_json(project: &Project, r: &sheaf_core::analysis::Report) -> Value {
    let table = r.table.as_ref().map(|t| {
        let rows: Vec<Value> = t.rows.iter().map(|(label, values)| json!({"label": label, "values": values})).collect();
        json!({"headers": t.headers, "units": t.units, "rows": rows, "row_units": t.row_units})
    });
    json!({
        "book": book_title(project, r.book),
        "sheet": sheet_name(project, r.book, r.sheet),
        "new_columns": r.new_columns.iter().map(|c| qualified(project, *c)).collect::<Vec<_>>(),
        "new_sheet": r.new_sheet.map(|i| sheet_name(project, r.book, i)),
        "new_graph": r.new_graph.and_then(|g| project.graph(g)).map(|g| g.name.clone()),
        "table": table,
        "message": r.message,
        "notes": r.notes,
    })
}

fn analyze(s: &mut Server, a: &Value) -> Res {
    let a: AnalyzeArgs = args(a)?;
    let output = match a.output.as_deref() {
        None | Some("next_to_input") => sheaf_core::analysis::OutputTarget::NextToInput,
        Some("new_sheet") => sheaf_core::analysis::OutputTarget::NewSheet,
        Some(o) => return Err(ToolError::new(format!("output must be next_to_input or new_sheet (got \"{o}\")"))),
    };
    let options = a.options.clone().unwrap_or_default();
    let columns = a.columns.clone().unwrap_or_default();
    match analysis_tools::resolve(&a.tool)? {
        analysis_tools::Which::Hand(name) => {
            let job = analysis_tools::build(name, &options)?;
            check_names(&s.project, a.book.as_deref(), a.sheet.as_deref(), &columns)?;
            let target = Target {
                y: columns,
                book: book_name(&s.project, a.book.as_deref())?,
                sheet: a.sheet,
                rows: a.rows,
                x_range: a.x_range,
                read: ReadSettings::default(),
            };
            let run = s.transact(|s| Ok(sheaf_ops::analyze::run_job_to(&mut s.project, &target, job, &output)?))?;
            match run.outcome {
                Outcome::Report(r) => Ok(ToolOut::json(report_json(&s.project, &r))),
                _ => Err(ToolError::new("the tool gave no report")),
            }
        }
        analysis_tools::Which::Bridged(tool) => {
            check_names(&s.project, a.book.as_deref(), a.sheet.as_deref(), &columns)?;
            analyze_bridged(s, &tool, &options, &columns, &a, &output)
        }
    }
}

/// Runs a tool whose options are those of its command-line definition: the JSON becomes an argv
/// for the CLI's own clap types, `sheaf_ops::cli_analyze::job_from` makes the job, and the shared
/// `run_job` runs it, as `sheaf analyze` does.
fn analyze_bridged(
    s: &mut Server,
    tool: &analysis_tools::Bridged,
    options: &Map<String, Value>,
    columns: &[String],
    a: &AnalyzeArgs,
    output: &sheaf_core::analysis::OutputTarget,
) -> Res {
    let selection = bridge::Selection {
        columns,
        book: a.book.as_deref(),
        sheet: a.sheet.as_deref(),
        rows: a.rows.as_deref(),
        x_range: a.x_range.as_deref(),
    };
    let roots = s.roots.clone();
    let mut resolve = |text: &str| -> anyhow::Result<String> {
        roots.check(text).map(|p| p.display().to_string()).map_err(|e| anyhow!("{e}"))
    };
    let cli = bridge::parse(
        &["analyze", &tool.command],
        &analysis_tools::bridge_filter(),
        &selection,
        options,
        &mut resolve,
    )?;
    let sheaf_ops::args::Command::Analyze(parsed) = cli.command else {
        return Err(ToolError::new("internal error: the options did not parse as an analysis"));
    };
    if matches!(parsed.tool, AnalyzeTool::Power { .. }) {
        let (spec, _) = power_spec(parsed.tool)?;
        return s.transact(|s| {
            let (made, mut report) = sheaf_ops::analyze::run_power(&spec)?;
            let wb = made.workbook(report.book).ok_or_else(|| ToolError::new("the power table is missing"))?.clone();
            report.book = s.project.adopt_workbook(wb);
            // The message named the workbook of the temporary project; name the adopted one.
            if let Some((head, _)) = report.message.rsplit_once(" in workbook ") {
                report.message = format!("{head} in workbook {}", book_title(&s.project, report.book));
            }
            Ok(ToolOut::json(report_json(&s.project, &report)))
        });
    }
    let (target_args, job) = job_from(parsed.tool)?;
    let target = Target {
        y: target_args.y,
        book: book_name(&s.project, target_args.book.as_deref())?,
        sheet: target_args.sheet,
        rows: target_args.rows,
        x_range: target_args.x_range,
        read: ReadSettings::default(),
    };
    let run = s.transact(|s| Ok(sheaf_ops::analyze::run_job_to(&mut s.project, &target, job, output)?))?;
    for f in &run.foreign_inputs {
        if !s.foreign.contains(f) {
            s.foreign.push(f.clone());
        }
    }
    let mut out = match &run.outcome {
        Outcome::Report(r) => report_json(&s.project, r),
        Outcome::Sorted { by } => {
            let rows = s.project.workbook(run.book).map_or(0, |b| b.sheets[run.sheet].row_count());
            json!({"sheet": sheet_name(&s.project, run.book, run.sheet), "sorted_by": by, "rows": rows})
        }
        Outcome::Filtered { kept, before, sheet_name } => json!({"sheet": sheet_name, "rows": kept, "before": before}),
    };
    if !run.load_warnings.is_empty() {
        out["warnings"] = json!(run.load_warnings);
    }
    Ok(ToolOut::json(out))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SortArgs {
    book: Option<String>,
    sheet: Option<String>,
    key: String,
    descending: Option<bool>,
}

fn sort_sheet(s: &mut Server, a: &Value) -> Res {
    let a: SortArgs = args(a)?;
    check_names(&s.project, a.book.as_deref(), a.sheet.as_deref(), std::slice::from_ref(&a.key))?;
    let target = Target { book: book_name(&s.project, a.book.as_deref())?, sheet: a.sheet, ..Target::default() };
    let job = Job::Sort { by: a.key, descending: a.descending.unwrap_or(false) };
    let run = s.transact(|s| Ok(sheaf_ops::analyze::run_job(&mut s.project, &target, job)?))?;
    let rows = s.project.workbook(run.book).map_or(0, |b| b.sheets[run.sheet].row_count());
    Ok(ToolOut::json(json!({"sheet": sheet_name(&s.project, run.book, run.sheet), "rows": rows})))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FilterArgs {
    book: Option<String>,
    sheet: Option<String>,
    condition: String,
    in_place: Option<bool>,
}

fn filter_sheet(s: &mut Server, a: &Value) -> Res {
    let a: FilterArgs = args(a)?;
    let target = Target { book: book_name(&s.project, a.book.as_deref())?, sheet: a.sheet, ..Target::default() };
    let job = Job::Filter { condition: a.condition, in_place: a.in_place.unwrap_or(false) };
    let run = s.transact(|s| Ok(sheaf_ops::analyze::run_job(&mut s.project, &target, job)?))?;
    match run.outcome {
        Outcome::Filtered { kept, before, sheet_name } => {
            Ok(ToolOut::json(json!({"sheet": sheet_name, "rows": kept, "before": before})))
        }
        _ => Err(ToolError::new("the filter gave no result")),
    }
}

// ----- fitting -----------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CategoryArgs {
    category: Option<String>,
}

fn list_fit_models(a: &Value) -> Res {
    let a: CategoryArgs = args(a)?;
    let models: Vec<_> = sheaf_ops::fit::list_models()
        .into_iter()
        .filter(|m| a.category.as_deref().is_none_or(|c| m.category.eq_ignore_ascii_case(c)))
        .collect();
    Ok(ToolOut::json(json!({"models": models, "special": ["linear", "linearx", "poly:N", "formula", "fdf"]})))
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
struct GlobalArgs {
    share: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
struct FitArgs {
    model: Option<String>,
    formula: Option<String>,
    params: Vec<String>,
    fdf: Option<String>,
    book: Option<String>,
    sheet: Option<String>,
    y: Vec<String>,
    rows: Option<String>,
    x_range: Option<String>,
    init: BTreeMap<String, f64>,
    fix: BTreeMap<String, f64>,
    bounds: BTreeMap<String, Vec<Option<f64>>>,
    weighting: Option<String>,
    yerr: Vec<String>,
    xerr: Option<String>,
    confidence: Option<f64>,
    bands: Option<String>,
    band_level: Option<f64>,
    max_iterations: Option<usize>,
    tolerance: Option<f64>,
    scale_errors: Option<bool>,
    compare_with: Option<String>,
    rank: Vec<String>,
    global: Option<GlobalArgs>,
    regress: Vec<String>,
    plot: Option<String>,
    options: Option<Map<String, Value>>,
}

fn fit_spec(a: &FitArgs, project: &Project, fdf: Option<PathBuf>) -> Result<FitSpec, ToolError> {
    let modes = [a.compare_with.is_some(), !a.rank.is_empty(), a.global.is_some(), !a.regress.is_empty()];
    if modes.iter().filter(|m| **m).count() > 1 {
        return Err(ToolError::new("give only one of compare_with, rank, global and regress"));
    }
    let mut spec = FitSpec::default();
    if let Some(m) = &a.model {
        spec.model = m.clone();
    }
    spec.formula = a.formula.clone();
    spec.params = a.params.clone();
    spec.fdf = fdf;
    spec.init = a.init.iter().map(|(k, v)| format!("{k}={v}")).collect();
    spec.fix = a.fix.iter().map(|(k, v)| format!("{k}={v}")).collect();
    for (k, b) in &a.bounds {
        if b.len() != 2 {
            return Err(ToolError::new(format!("bounds of \"{k}\" must be [low, high], with null for no bound")));
        }
        let side = |v: Option<f64>| v.map(|x| x.to_string()).unwrap_or_default();
        spec.bounds.push(format!("{k}={}:{}", side(b[0]), side(b[1])));
    }
    spec.weight = match one_of(
        "weighting",
        a.weighting.as_deref().unwrap_or("none"),
        &["none", "instrumental", "statistical", "direct"],
    )?
    .as_str()
    {
        "instrumental" => WeightKind::Instrumental,
        "statistical" => WeightKind::Statistical,
        "direct" => WeightKind::Direct,
        _ => WeightKind::None,
    };
    spec.yerr = a.yerr.clone();
    spec.xerr = a.xerr.clone();
    spec.regress = a.regress.clone();
    spec.compare = a.compare_with.clone();
    spec.rank = a.rank.clone();
    spec.no_scale_errors = a.scale_errors == Some(false);
    if let Some(g) = &a.global {
        spec.global = true;
        spec.share = g.share.clone();
    }
    if let Some(c) = a.confidence {
        spec.confidence = c;
    }
    spec.bands =
        match one_of("bands", a.bands.as_deref().unwrap_or("none"), &["none", "confidence", "prediction", "both"])?
            .as_str()
        {
            "confidence" => BandKind::Confidence,
            "prediction" => BandKind::Prediction,
            "both" => BandKind::Both,
            _ => BandKind::None,
        };
    if let Some(l) = a.band_level {
        spec.level = l;
    }
    if let Some(n) = a.max_iterations {
        spec.max_iterations = n;
    }
    if let Some(t) = a.tolerance {
        spec.tolerance = t;
    }
    spec.y = a.y.clone();
    spec.book = book_name(project, a.book.as_deref())?;
    check_names(project, spec.book.as_deref(), a.sheet.as_deref(), &a.y)?;
    spec.sheet = a.sheet.clone();
    spec.rows = a.rows.clone();
    spec.x_range = a.x_range.clone();
    Ok(spec)
}

fn fit_json(project: &Project, o: &sheaf_core::fitting::FitOutcome) -> Value {
    let r = &o.report;
    let parameters: Vec<Value> = r
        .params
        .iter()
        .map(|p| {
            json!({
                "name": p.name, "value": p.value, "stderr": p.std_error, "t": p.t_value, "prob": p.prob_t,
                "lcl": p.lcl, "ucl": p.ucl, "fixed": p.fixed,
            })
        })
        .collect();
    json!({
        "model": r.model, "equation": r.equation,
        "book": book_title(project, o.book),
        "sheet": sheet_name(project, o.book, o.sheet),
        "param_sheet": sheet_name(project, o.book, o.param_sheet),
        "curve_sheet": sheet_name(project, o.book, o.curve_sheet),
        "fitted_column": qualified(project, o.fitted_column),
        "residual_column": qualified(project, o.residual_column),
        "parameters": parameters,
        "statistics": {
            "n": r.stats.n_points, "dof": r.stats.dof, "rss": r.stats.rss, "reduced_chi2": r.stats.reduced_chi_sqr,
            "r2": r.stats.r_squared, "adj_r2": r.stats.adj_r_squared, "pearson_r": r.stats.pearson_r,
            "iterations": r.stats.iterations,
        },
        "anova": r.anova,
        "derived": o.derived.iter().map(|(n, v)| json!({"name": n, "value": v})).collect::<Vec<_>>(),
        "warnings": r.warnings,
        "message": o.message,
    })
}

fn fit(s: &mut Server, a_raw: &Value) -> Res {
    let a: FitArgs = args(a_raw)?;
    let fdf = a.fdf.as_deref().map(|p| s.roots.check(p)).transpose()?;
    let plot_path = a.plot.as_deref().map(|p| s.roots.check(p)).transpose()?;
    let spec = if a.options.is_some() {
        let sheaf_ops::args::Command::Fit(cli) = session_cli(s, "fit", a_raw)?.command else { unreachable!() };
        let mut spec = sheaf_ops::cli_fit::fit_spec(&cli);
        spec.book = book_name(&s.project, spec.book.as_deref())?;
        check_names(&s.project, spec.book.as_deref(), spec.sheet.as_deref(), &spec.y)?;
        spec
    } else {
        fit_spec(&a, &s.project, fdf)?
    };
    if plot_path.is_some() && (spec.compare.is_some() || !spec.rank.is_empty() || !spec.regress.is_empty()) {
        return Err(ToolError::new("plot applies to a fit of a model, not to compare_with, rank or regress"));
    }
    let prepared = sheaf_ops::fit::prepare(&spec)?;
    let result = s.transact(|s| Ok(sheaf_ops::fit::run(&mut s.project, &spec, &prepared)?))?;
    let project = &mut s.project;
    let value = match result.run {
        FitRun::Fits { summary, outcomes, columns } => {
            let list: Vec<Value> = outcomes.iter().map(|o| fit_json(project, o)).collect();
            let image = match (&plot_path, outcomes.first(), columns.first()) {
                (Some(path), Some(o), Some(c)) => {
                    let graph = sheaf_ops::fit::fit_graph(project, o, *c)?;
                    sheaf_ops::render::write_graph(project, graph, path, &ExportSettings::default())?;
                    json!(path)
                }
                _ => Value::Null,
            };
            json!({"outcomes": list, "summary": summary, "image": image})
        }
        FitRun::Regression(o) => {
            let r = &o.report;
            let parameters: Vec<Value> = r
                .params
                .iter()
                .map(|p| json!({"name": p.name, "value": p.value, "stderr": p.std_error, "t": p.t_value, "prob": p.prob_t, "lcl": p.lcl, "ucl": p.ucl, "fixed": p.fixed}))
                .collect();
            json!({"outcomes": [{
                "model": r.model, "equation": r.equation, "book": book_title(project, o.book),
                "param_sheet": sheet_name(project, o.book, o.param_sheet),
                "fitted_column": qualified(project, o.fitted_column),
                "residual_column": qualified(project, o.residual_column),
                "parameters": parameters,
                "statistics": {"n": r.stats.n_points, "dof": r.stats.dof, "rss": r.stats.rss,
                    "reduced_chi2": r.stats.reduced_chi_sqr, "r2": r.stats.r_squared,
                    "adj_r2": r.stats.adj_r_squared, "pearson_r": r.stats.pearson_r, "iterations": r.stats.iterations},
                "anova": r.anova, "warnings": r.warnings, "message": o.message,
            }]})
        }
        FitRun::Comparison(o) => json!({
            "compare": {"message": o.message, "warnings": o.comparison.warnings, "table": sheet_table(project, o.book, o.sheet)}
        }),
        FitRun::Ranking(o) => json!({
            "rank": {
                "message": o.message,
                "skipped": o.skipped.iter().map(|(n, why)| json!({"model": n, "reason": why})).collect::<Vec<_>>(),
                "table": sheet_table(project, o.book, o.sheet),
            }
        }),
    };
    Ok(ToolOut::json(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_names_become_parameter_names() {
        assert_eq!(clean_message("--x-min and --x-max must differ"), "x_min and x_max must differ");
        assert_eq!(clean_message("error: --yerr names a column"), "yerr names a column");
        assert_eq!(clean_message("a -- b and x--y stay"), "a -- b and x--y stay");
        assert_eq!(clean_message("no flags here"), "no flags here");
    }

    #[test]
    fn every_example_fits_its_schema() {
        for d in schemas::defs() {
            schemas::check(&d.schema, &d.example, d.name).unwrap_or_else(|e| panic!("{e}"));
        }
        // The check itself rejects what it should.
        let d = schemas::defs().into_iter().find(|d| d.name == "set_column_values").unwrap();
        assert!(schemas::check(&d.schema, &json!({"column": "A"}), "x").is_err());
        assert!(schemas::check(&d.schema, &json!({"column": "A", "formula": "1", "oops": 1}), "x").is_err());
        assert!(schemas::check(&d.schema, &json!({"column": 1, "formula": "1"}), "x").is_err());
    }

    #[test]
    fn every_example_is_accepted_by_its_argument_parser() {
        let mut s = Server::new(crate::roots::Roots::default());
        for d in schemas::defs() {
            // Without data or a root the call fails, but never on the arguments themselves.
            if let Err(e) = call(&mut s, d.name, &d.example) {
                for bad in ["unknown field", "missing field", "invalid type"] {
                    assert!(!e.message.contains(bad), "{}: {}", d.name, e.message);
                }
            }
        }
    }

    #[test]
    fn row_ranges_read_open_ends() {
        assert_eq!(fill_range("2:5", 10).unwrap(), 1..5);
        assert_eq!(fill_range(":3", 10).unwrap(), 0..3);
        assert_eq!(fill_range("4:", 10).unwrap(), 3..10);
        assert!(fill_range("0:3", 10).is_err());
        assert!(fill_range("5:2", 10).is_err());
        assert!(fill_range("a", 10).is_err());
    }

    #[test]
    fn png_headers_are_read() {
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, b'I', b'H', b'D', b'R'];
        png.extend_from_slice(&640u32.to_be_bytes());
        png.extend_from_slice(&480u32.to_be_bytes());
        assert_eq!(png_size(&png), Some((640, 480)));
        assert_eq!(png_size(b"nope"), None);
    }

    fn sheet_with(rows: usize, cols: &[&str]) -> Server {
        let mut s = Server::new(crate::roots::Roots::default());
        let book = s.project.add_workbook("");
        for (i, name) in cols.iter().enumerate() {
            let id = s
                .project
                .add_column(
                    book,
                    0,
                    if i == 0 { Designation::X } else { Designation::Y },
                    ColumnData::numeric((0..rows).map(|r| (r * (i + 1)) as f64).collect()),
                )
                .unwrap();
            s.project.column_mut(id).unwrap().short_name = (*name).to_string();
        }
        s
    }

    #[test]
    fn read_columns_pages_at_the_cell_cap() {
        let mut s = sheet_with(12_000, &["A", "B"]);
        let first = read_columns(&mut s, &json!({})).unwrap().value;
        assert_eq!(first["total_rows"], 12_000);
        assert_eq!(first["from"], 1);
        assert_eq!(first["to"], 5_000);
        assert_eq!(first["truncated"], true);
        assert_eq!(first["rows"].as_array().unwrap().len(), 5_000);
        assert_eq!(first["next"], json!({"from": 5001, "to": 12000}));
        // Following next to the end reads every row once.
        let mut next = first["next"].clone();
        let mut seen = 5_000;
        loop {
            let page = read_columns(&mut s, &json!({"from": next["from"], "to": next["to"]})).unwrap().value;
            seen += page["rows"].as_array().unwrap().len();
            if page["truncated"] == false {
                assert!(page.get("next").is_none());
                break;
            }
            next = page["next"].clone();
        }
        assert_eq!(seen, 12_000);
        // A lower limit shrinks the page; a higher one cannot raise the cap.
        let small = read_columns(&mut s, &json!({"limit": 10})).unwrap().value;
        assert_eq!(small["rows"].as_array().unwrap().len(), 5);
        let capped = read_columns(&mut s, &json!({"limit": 1_000_000})).unwrap().value;
        assert_eq!(capped["rows"].as_array().unwrap().len(), 5_000);
        // One column holds twice as many rows.
        let one = read_columns(&mut s, &json!({"columns": ["B"]})).unwrap().value;
        assert_eq!(one["rows"].as_array().unwrap().len(), 10_000);
        assert_eq!(one["rows"][3], json!([6.0]));
    }

    #[test]
    fn read_columns_checks_its_range() {
        let mut s = sheet_with(5, &["A", "B"]);
        assert!(read_columns(&mut s, &json!({"from": 0})).is_err());
        let past = read_columns(&mut s, &json!({"from": 9})).unwrap().value;
        assert_eq!(past["rows"], json!([]));
        let e = read_columns(&mut s, &json!({"columns": ["Q"]})).err().unwrap();
        assert!(e.message.contains("column 'Q' was not found"), "{}", e.message);
    }

    #[test]
    fn missing_and_infinite_values_have_fixed_forms() {
        let mut s = sheet_with(3, &["A"]);
        let id = s.project.workbooks().next().unwrap().sheets[0].columns[0].id;
        *s.project.column_mut(id).unwrap().numeric_mut().unwrap() = vec![1.5, f64::NAN, f64::INFINITY];
        let r = read_columns(&mut s, &json!({})).unwrap().value;
        assert_eq!(r["rows"], json!([[1.5], [null], ["Infinity"]]));
    }

    #[test]
    fn set_column_values_creates_and_rolls_back() {
        let mut s = sheet_with(4, &["A", "B"]);
        let ok = set_column_values(&mut s, &json!({"column": "C", "formula": "col(B)*10", "create": {"units": "mA"}}))
            .unwrap()
            .value;
        assert_eq!(ok["created"], true);
        assert_eq!(ok["rows"], 4);
        let r = read_columns(&mut s, &json!({"columns": ["C"]})).unwrap().value;
        assert_eq!(r["rows"], json!([[0.0], [20.0], [40.0], [60.0]]));
        // A bad formula for a new column leaves no column behind.
        let before = contents(&s.project, None);
        assert!(set_column_values(&mut s, &json!({"column": "D", "formula": "col(Nope)+", "create": {}})).is_err());
        assert_eq!(contents(&s.project, None), before);
        // Without create, a missing column is an error naming the columns.
        let e = set_column_values(&mut s, &json!({"column": "Z", "formula": "1"})).err().unwrap();
        assert!(e.message.contains("was not found"), "{}", e.message);
    }

    #[test]
    fn analyze_matches_the_library_called_directly() {
        let mut s = sheet_with(50, &["A", "B"]);
        let id = s.project.workbooks().next().unwrap().sheets[0].columns[0].id;
        *s.project.column_mut(id).unwrap().numeric_mut().unwrap() = (0..50).map(|i| f64::from(i) * 0.1).collect();
        let b = s.project.workbooks().next().unwrap().sheets[0].columns[1].id;
        *s.project.column_mut(b).unwrap().numeric_mut().unwrap() =
            (0..50).map(|i| (f64::from(i) * 0.1).sin()).collect();
        let mut direct = s.project.clone();
        let out = analyze(&mut s, &json!({"tool": "statistics", "columns": ["B"]})).unwrap().value;
        let job = analysis_tools::build("statistics", &Map::new()).unwrap();
        let target = Target { y: vec!["B".into()], ..Target::default() };
        let run = sheaf_ops::analyze::run_job(&mut direct, &target, job).unwrap();
        let Outcome::Report(r) = run.outcome else { panic!() };
        let table = r.table.unwrap();
        assert_eq!(out["table"]["headers"], json!(table.headers));
        assert_eq!(out["table"]["rows"][0]["values"], json!(table.rows[0].1));
        assert_eq!(out["message"], json!(r.message));
        // Unknown tools and options are named in the error.
        let e = analyze(&mut s, &json!({"tool": "nope"})).err().unwrap();
        assert!(e.message.contains("unknown analysis tool"));
        let e = analyze(&mut s, &json!({"tool": "smooth", "options": {"pointz": 3}})).err().unwrap();
        assert!(e.message.contains("pointz"));
    }

    #[test]
    fn list_contents_names_columns_and_formulas() {
        let mut s = sheet_with(3, &["A", "B"]);
        set_column_values(&mut s, &json!({"column": "B", "formula": "col(A)+1"})).unwrap();
        let c = list_contents(&mut s, &json!({})).unwrap().value;
        let col = &c["workbooks"][0]["sheets"][0]["columns"][1];
        assert_eq!(col["short_name"], "B");
        assert_eq!(col["designation"], "Y");
        assert_eq!(col["kind"], "numeric");
        assert_eq!(col["formula"], "col(A)+1");
        assert_eq!(c["workbooks"][0]["sheets"][0]["active"], true);
        assert!(list_contents(&mut s, &json!({"book": "Nope"})).is_err());
    }
}
