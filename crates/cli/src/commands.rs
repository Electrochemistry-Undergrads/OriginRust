//! What each subcommand does: read the flags, call `sheaf-ops` and print what it reports.

use crate::args::*;
use anyhow::{Result, bail};
use sheaf_core::template::Template;
use sheaf_core::{Project, WindowId};
use sheaf_io::Imported;
use sheaf_ops::cli_plot::{plot_spec, style_spec};
use sheaf_ops::import::{ImportLayout as OpsLayout, ImportRequest};
use sheaf_ops::render::{ExportSettings, MarginSettings};
use sheaf_ops::{DataFormat, ReadSettings, ThousandsSetting};
use std::path::{Path, PathBuf};

pub use sheaf_ops::pick::{pick_book, pick_sheet, require_positive};

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Info(a) => info(a),
        Command::Import(a) => import(a),
        Command::Plot(a) => plot(*a),
        Command::Export(a) => export(a),
        Command::ExportData(a) => export_data(a),
        Command::Analyze(a) => crate::analyze::analyze(a),
        Command::Fit(a) => crate::fit::fit(a),
    }
}

pub(crate) fn is_project(path: &Path) -> bool {
    sheaf_ops::session::is_project(path)
}

/// Origin files this run read as a project (Sheaf reads only part of them).
static FOREIGN_INPUTS: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

pub(crate) fn remember_foreign(paths: &[PathBuf]) {
    FOREIGN_INPUTS.lock().unwrap_or_else(|e| e.into_inner()).extend(paths.iter().cloned());
}

/// Opens a `.sheaf` project or imports an Origin `.opj` project.
pub(crate) fn load_project(path: &Path) -> Result<Project> {
    let opened = sheaf_ops::session::load_project(path)?;
    for w in &opened.warnings {
        eprintln!("note: {w}");
    }
    if opened.foreign {
        remember_foreign(&[path.to_path_buf()]);
    }
    Ok(opened.project)
}

/// The import options of the command line as the library takes them.
pub(crate) fn read_settings(r: &ReadOptions) -> ReadSettings {
    ReadSettings {
        format: match r.format {
            Format::Auto => DataFormat::Auto,
            Format::Chi => DataFormat::Chi,
            Format::Ascii => DataFormat::Ascii,
        },
        no_split_scans: r.no_split_scans,
        no_milliamp: r.no_milliamp,
        no_charge: r.no_charge,
        invert_current: r.invert_current,
        no_split_segments: r.no_split_segments,
        keep_file_times: r.keep_file_times,
        delimiter: r.delimiter.clone(),
        delimiter_chars: r.delimiter_chars.clone(),
        combine_delimiters: r.combine_delimiters,
        fixed_width: r.fixed_width.clone(),
        header_lines: r.header_lines,
        first_row: r.first_row,
        last_row: r.last_row,
        read_rows: r.read_rows,
        skip_rows: r.skip_rows,
        first_column: r.first_column,
        last_column: r.last_column,
        read_columns: r.read_columns,
        skip_columns: r.skip_columns,
        decimal_comma: r.decimal_comma,
        dates_as_text: r.dates_as_text,
        day_first: r.day_first,
        thousands: match r.thousands {
            ThousandsArg::Auto => ThousandsSetting::Auto,
            ThousandsArg::None => ThousandsSetting::None,
            ThousandsArg::Comma => ThousandsSetting::Comma,
            ThousandsArg::Dot => ThousandsSetting::Dot,
            ThousandsArg::Space => ThousandsSetting::Space,
            ThousandsArg::Apostrophe => ThousandsSetting::Apostrophe,
        },
        long_name_line: r.long_name_line,
        units_line: r.units_line,
        comments_line: r.comments_line,
        param_lines: r.param_lines.clone(),
        short_name_line: r.short_name_line,
        no_rename_columns: r.no_rename_columns,
        no_rename_sheet: r.no_rename_sheet,
        no_rename_book: r.no_rename_book,
        blocks_as_sheets: r.blocks_as_sheets,
        sheets: r.sheets.clone(),
        hidden_sheets: r.hidden_sheets,
    }
}

pub(crate) fn read_data_file(path: &Path, r: &ReadSettings) -> Result<Imported> {
    let imported = sheaf_ops::read::read_data_file(path, r)?;
    for w in &imported.warnings {
        eprintln!("warning: {w}");
    }
    Ok(imported)
}

fn describe(project: &Project) {
    let summary = sheaf_ops::info::summarize(project);
    for &(is_book, i) in &summary.order {
        if is_book {
            let b = &summary.workbooks[i];
            println!("workbook {}", b.title);
            for s in &b.sheets {
                println!("  sheet {} ({} columns, {} rows)", s.name, s.columns.len(), s.rows);
                for c in &s.columns {
                    let kind = if c.kind.is_empty() { String::new() } else { format!(" {}", c.kind) };
                    println!("    {:<4} {:<5} {:>8} rows{}  {}", c.short_name, c.designation, c.rows, kind, c.label);
                }
            }
        } else {
            let g = &summary.graphs[i];
            println!(
                "graph {} ({} layer(s), {} plot(s), {:.2} x {:.2} in)",
                g.title, g.layers, g.plots, g.width_in, g.height_in
            );
        }
    }
}

fn info(a: InfoArgs) -> Result<()> {
    if is_project(&a.path) {
        describe(&load_project(&a.path)?);
        return Ok(());
    }
    let imported = read_data_file(&a.path, &read_settings(&a.read))?;
    println!("format: {}", imported.format);
    println!("suggested template: {}", imported.suggested_template.name());
    for (k, v) in &imported.meta {
        println!("  {k} = {v}");
    }
    let mut project = Project::new();
    project.adopt_workbook(imported.workbook);
    describe(&project);
    Ok(())
}

fn import(mut a: ImportArgs) -> Result<()> {
    a.output = sheaf_io::save::default_extension(&a.output);
    if a.files.iter().any(|f| f == &a.output) {
        bail!("the output '{}' is also one of the inputs", a.output.display());
    }
    let mut project = if a.append && a.output.exists() { load_project(&a.output)? } else { Project::new() };
    let request = ImportRequest {
        files: a.files.clone(),
        layout: match a.mode {
            ImportLayout::Books => OpsLayout::Books,
            ImportLayout::Sheets => OpsLayout::Sheets,
            ImportLayout::Columns => OpsLayout::Columns,
            ImportLayout::Rows => OpsLayout::Rows,
        },
        read: read_settings(&a.read),
        book_name: a.book_name.clone(),
        file_name_comments: a.file_name_comments,
        partial_except_first: a.partial_except_first,
        plot: a.plot,
    };
    let result = sheaf_ops::import::import_files(&mut project, &request)?;
    for w in &result.project_warnings {
        eprintln!("note: {w}");
    }
    remember_foreign(&result.foreign_inputs);
    for note in &result.notes {
        println!("{note}");
    }
    for item in &result.items {
        for w in &item.warnings {
            eprintln!("warning: {}: {w}", item.file.display());
        }
        println!("imported {} into {}", item.file.display(), item.book);
    }
    for e in &result.errors {
        eprintln!("error: {e}");
    }
    if result.items.is_empty() && result.data_files > 0 {
        bail!("none of the {} file(s) could be imported", result.data_files);
    }
    save_project(&project, &a.output)?;
    if !result.errors.is_empty() {
        bail!("{} file(s) could not be imported (the others were saved)", result.errors.len());
    }
    Ok(())
}

/// Saves the project (see [`sheaf_ops::session::save_project`]) and says what was written. When the
/// Origin writer refuses the project only the companion is written, and that is an error because
/// the file asked for does not exist.
pub(crate) fn save_project(project: &Project, path: &Path) -> Result<()> {
    // Saving over the Origin file this run read would replace it with a partial rewrite: keep a copy.
    let target = sheaf_io::save::default_extension(path);
    let over_input = {
        let inputs = FOREIGN_INPUTS.lock().unwrap_or_else(|e| e.into_inner());
        inputs.iter().any(|i| sheaf_ops::session::same_file(i, &target))
    };
    let report = sheaf_ops::session::save_project(project, path, over_input)?;
    if let Some(b) = &report.backup {
        println!("kept the original Origin file as {}", b.display());
    }
    report_save(&report)
}

fn report_save(report: &sheaf_ops::SaveReport) -> Result<()> {
    if let Some(reason) = &report.refusal {
        let companion = report.companion.as_ref().map(|c| c.display().to_string()).unwrap_or_default();
        println!("saved {companion} (the whole project)");
        bail!(
            "'{}' was not written because Origin cannot store the project: {reason}. Rename as it says and save again; the project is safe in '{companion}'",
            report.path.display()
        );
    }
    println!("saved {}", report.path.display());
    if let Some(c) = &report.companion {
        println!("saved {} (the whole project; the .opj cannot hold everything)", c.display());
        for w in &report.warnings {
            eprintln!("warning: only {} holds: {w}", c.display());
        }
    }
    if let Some(c) = &report.companion_removed {
        println!("removed {} (nothing is lost in the .opj any more)", c.display());
    }
    Ok(())
}

fn export_settings(dpi: f64, width: &Option<String>, height: &Option<String>, m: &MarginArgs) -> ExportSettings {
    ExportSettings {
        dpi,
        image_width: width.clone(),
        image_height: height.clone(),
        margin: MarginSettings { margin: m.margin.clone(), border_width: m.border_width },
    }
}

fn plot(a: PlotArgs) -> Result<()> {
    require_positive("--dpi", Some(a.dpi))?;
    let first = &a.inputs[0];
    if a.inputs.len() > 1 && a.inputs.iter().any(|p| is_project(p)) {
        bail!("a project cannot be combined with other inputs; give one project, or one or more data files");
    }
    let spec = plot_spec(&a);
    spec.validate()?;
    let read = read_settings(&a.read);
    let (mut project, suggested) = if is_project(first) {
        (load_project(first)?, Template::Line)
    } else {
        let mut p = Project::new();
        let mut template = Template::Line;
        for (i, input) in a.inputs.iter().enumerate() {
            let imported = read_data_file(input, &read)?;
            if i == 0 {
                template = imported.suggested_template;
            }
            p.adopt_workbook(imported.workbook);
        }
        (p, template)
    };
    let graph = sheaf_ops::plot::build_graph(&mut project, &spec, suggested)?;
    let settings = export_settings(a.dpi, &a.image_width, &a.image_height, &a.margin);
    sheaf_ops::render::write_graph(&project, graph, &a.output, &settings)?;
    println!("wrote {}", a.output.display());
    if let Some(path) = &a.save {
        save_project(&project, path)?;
    }
    Ok(())
}

fn is_opj(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("opj"))
}

/// Writes the whole project (or the data file as a one-workbook project) as an Origin project.
fn export_opj(a: &ExportArgs) -> Result<()> {
    if a.graph.is_some() {
        bail!("--graph is for figures; an .opj output holds the whole project");
    }
    if let (Ok(input), Ok(output)) = (std::fs::canonicalize(&a.project), std::fs::canonicalize(&a.output))
        && input == output
    {
        bail!("the output '{}' is also the input", a.output.display());
    }
    let project = if is_project(&a.project) {
        load_project(&a.project)?
    } else {
        let mut p = Project::new();
        p.adopt_workbook(read_data_file(&a.project, &read_settings(&a.read))?.workbook);
        p
    };
    save_project(&project, &a.output)
}

fn xlsx_request(x: &XlsxArgs) -> sheaf_ops::tabular::XlsxRequest {
    sheaf_ops::tabular::XlsxRequest {
        all_sheets: x.all_sheets,
        no_long_names: x.no_long_names,
        no_units: x.no_units,
        no_comments: x.no_comments,
        short_names: x.short_names,
    }
}

/// Writes a workbook of the project as an Excel file and says so.
fn write_xlsx(project: &Project, book: Option<&str>, sheet: Option<&str>, x: &XlsxArgs, output: &Path) -> Result<()> {
    let book = pick_book(project, book)?;
    let outcome = sheaf_ops::tabular::write_xlsx(project, book, sheet, &xlsx_request(x), output)?;
    for w in &outcome.warnings {
        eprintln!("warning: {w}");
    }
    println!(
        "wrote {} ({} sheet(s): {})",
        outcome.path.display(),
        outcome.sheet_names.len(),
        outcome.sheet_names.join(", ")
    );
    Ok(())
}

/// Writes one workbook of the project (or the data file as a one-workbook project) as `.xlsx`.
fn export_xlsx(a: &ExportArgs) -> Result<()> {
    if a.graph.is_some() {
        bail!("--graph is for figures; an .xlsx output holds a workbook (see --book and --sheet)");
    }
    if let (Ok(input), Ok(output)) = (std::fs::canonicalize(&a.project), std::fs::canonicalize(&a.output))
        && input == output
    {
        bail!("the output '{}' is also the input", a.output.display());
    }
    let project = if is_project(&a.project) {
        load_project(&a.project)?
    } else {
        let mut p = Project::new();
        p.adopt_workbook(read_data_file(&a.project, &read_settings(&a.read))?.workbook);
        p
    };
    write_xlsx(&project, a.book.as_deref(), a.sheet.as_deref(), &a.xlsx, &a.output)
}

fn export(a: ExportArgs) -> Result<()> {
    if is_opj(&a.output) {
        return export_opj(&a);
    }
    if sheaf_ops::tabular::is_xlsx_path(&a.output) {
        return export_xlsx(&a);
    }
    require_positive("--dpi", Some(a.dpi))?;
    if !is_project(&a.project) {
        bail!(
            "'{}' is not a project; figures are exported from a .sheaf or .opj project (use `sheaf plot` for a data file)",
            a.project.display()
        );
    }
    let mut project = load_project(&a.project)?;
    let graph: WindowId = sheaf_ops::pick::pick_graph(&project, a.graph.as_deref())?;
    sheaf_ops::plot::apply_style(&mut project, graph, &style_spec(&a.style), false)?;
    let settings = export_settings(a.dpi, &a.image_width, &a.image_height, &a.margin);
    sheaf_ops::render::write_graph(&project, graph, &a.output, &settings)?;
    println!("wrote {}", a.output.display());
    Ok(())
}

fn export_data(a: ExportDataArgs) -> Result<()> {
    let project = load_project(&a.project)?;
    let book = pick_book(&project, a.book.as_deref())?;
    if sheaf_ops::tabular::is_xlsx_path(&a.output) {
        return write_xlsx(&project, a.book.as_deref(), a.sheet.as_deref(), &a.xlsx, &a.output);
    }
    let sheet = pick_sheet(&project, book, a.sheet.as_deref())?;
    let ws = &project.workbook(book).expect("picked above").sheets[sheet];
    write_sheet(ws, &a.output)
}

/// Writes a sheet as delimited text and says so.
pub(crate) fn write_sheet(ws: &sheaf_core::Worksheet, path: &Path) -> Result<()> {
    sheaf_ops::tabular::write_sheet(ws, path)?;
    println!("wrote {}", path.display());
    Ok(())
}
