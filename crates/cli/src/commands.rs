//! What each subcommand does.

use crate::args::*;
use anyhow::{Context, Result, anyhow, bail};
use sheaf_core::graph::LineStyle;

use sheaf_core::project::Window;
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnId, Project, WindowId};
use sheaf_io::ascii::{AsciiOptions, Delimiter, Thousands};
use sheaf_io::batch::{BatchLayout, BatchOptions};
use sheaf_io::chi::ChiOptions;
use sheaf_io::{ImportKind, Imported};
use std::path::{Path, PathBuf};

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Info(a) => info(a),
        Command::Import(a) => import(a),
        Command::Plot(a) => plot(a),
        Command::Export(a) => export(a),
        Command::ExportData(a) => export_data(a),
        Command::Analyze(a) => crate::analyze::analyze(a),
        Command::Fit(a) => crate::fit::fit(a),
    }
}

pub(crate) fn is_project(path: &Path) -> bool {
    sheaf_io::is_project_file(path)
}

/// Opens a `.sheaf` project or imports an Origin `.opj` project.
/// Origin files this run read as a project (Sheaf reads only part of them).
static FOREIGN_INPUTS: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

pub(crate) fn load_project(path: &Path) -> Result<Project> {
    let opened =
        sheaf_io::open_project(path).with_context(|| format!("could not open project '{}'", path.display()))?;
    for w in &opened.warnings {
        eprintln!("note: {w}");
    }
    if opened.foreign {
        FOREIGN_INPUTS.lock().unwrap_or_else(|e| e.into_inner()).push(path.to_path_buf());
    }
    Ok(opened.project)
}

pub(crate) fn chi_options(r: &ReadOptions) -> ChiOptions {
    ChiOptions {
        split_scans: !r.no_split_scans,
        add_milliamp: !r.no_milliamp,
        add_charge: !r.no_charge,
        invert_current: r.invert_current,
    }
}

pub(crate) fn ascii_options(r: &ReadOptions) -> Result<AsciiOptions> {
    let mut o = AsciiOptions::default();
    if let Some(d) = &r.delimiter {
        o.delimiter = match d.to_ascii_lowercase().as_str() {
            "comma" | "," => Delimiter::Comma,
            "tab" | "\\t" => Delimiter::Tab,
            "semicolon" | ";" => Delimiter::Semicolon,
            "space" | "whitespace" | " " => Delimiter::Whitespace,
            other => {
                let mut chars = other.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => Delimiter::Other(c),
                    _ => bail!("--delimiter must be comma, tab, semicolon, space or a single character, not '{d}'"),
                }
            }
        };
    }
    o.header_lines = r.header_lines;
    o.decimal_comma = r.decimal_comma;
    o.thousands = match r.thousands {
        ThousandsArg::Auto => Thousands::Auto,
        ThousandsArg::None => Thousands::Off,
        ThousandsArg::Comma => Thousands::Comma,
        ThousandsArg::Dot => Thousands::Dot,
        ThousandsArg::Space => Thousands::Space,
        ThousandsArg::Apostrophe => Thousands::Apostrophe,
    };
    o.long_name_line = r.long_name_line;
    o.units_line = r.units_line;
    o.comments_line = r.comments_line;
    o.param_lines = r.param_lines.clone();
    if r.blocks_as_sheets {
        o.blocks = sheaf_io::ascii::Blocks::Sheets;
    }
    if r.dates_as_text {
        o.dates = sheaf_io::ascii::DateImport::AsText;
    }
    if r.day_first {
        o.date_order = sheaf_core::datetime::DateOrder::Dmy;
    }
    Ok(o)
}

pub(crate) fn read_data_file(path: &Path, r: &ReadOptions) -> Result<Imported> {
    let kind = match r.format {
        Format::Auto => ImportKind::Auto,
        Format::Chi => ImportKind::Chi,
        Format::Ascii => ImportKind::Ascii,
    };
    let imported = sheaf_io::import_file(path, kind, &chi_options(r), &ascii_options(r)?)
        .with_context(|| format!("could not import '{}'", path.display()))?;
    for w in &imported.warnings {
        eprintln!("warning: {w}");
    }
    Ok(imported)
}

fn describe(project: &Project) {
    for w in &project.windows {
        match w {
            Window::Workbook(b) => {
                println!("workbook {}", b.title());
                for s in &b.sheets {
                    println!("  sheet {} ({} columns, {} rows)", s.name, s.columns.len(), s.row_count());
                    for c in &s.columns {
                        let kind = if !c.data.is_numeric() {
                            " text"
                        } else if c.format.is_date_time() {
                            " date"
                        } else {
                            ""
                        };
                        let mut label = c.long_name.clone();
                        // A date column shows its first and last value as they appear in the worksheet.
                        if c.format.is_date_time()
                            && let Some(v) = c.numeric()
                            && let (Some(first), Some(last)) =
                                (v.iter().find(|x| x.is_finite()), v.iter().rev().find(|x| x.is_finite()))
                        {
                            let sep = if label.is_empty() { "" } else { " " };
                            label.push_str(&format!(
                                "{sep}<{} to {}>",
                                c.format.format(*first),
                                c.format.format(*last)
                            ));
                        }
                        if !c.units.is_empty() {
                            label.push_str(&format!(" ({})", c.units));
                        }
                        if !c.comments.is_empty() {
                            label.push_str(&format!(" [{}]", c.comments.replace('\n', " / ")));
                        }
                        println!(
                            "    {:<4} {:<5} {:>8} rows{}  {}",
                            c.short_name,
                            c.designation.tag(),
                            c.len(),
                            kind,
                            label
                        );
                    }
                }
            }
            Window::Graph(g) => {
                let plots: usize = g.layers.iter().map(|l| l.plots.len()).sum();
                println!(
                    "graph {} ({} layer(s), {} plot(s), {:.2} x {:.2} in)",
                    g.title(),
                    g.layers.len(),
                    plots,
                    g.page.width_pt / 72.0,
                    g.page.height_pt / 72.0
                );
            }
        }
    }
}

fn info(a: InfoArgs) -> Result<()> {
    if is_project(&a.path) {
        describe(&load_project(&a.path)?);
        return Ok(());
    }
    let imported = read_data_file(&a.path, &a.read)?;
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
    // Projects are taken over as they are; everything else is a data file, folder or pattern.
    let mut data_files = Vec::new();
    for arg in &a.files {
        if is_project(arg) {
            let mut count = 0;
            for window in load_project(arg)?.windows {
                if let Window::Workbook(book) = window {
                    project.adopt_named_workbook(book);
                    count += 1;
                }
            }
            println!("imported {count} workbook(s) from {}", arg.display());
            continue;
        }
        let found = sheaf_io::batch::expand_inputs(arg).map_err(|e| anyhow!(e))?;
        if found.len() > 1 {
            println!("{} matches {} files", arg.display(), found.len());
        }
        data_files.extend(found);
    }
    let layout = match a.mode {
        ImportLayout::Books => BatchLayout::Books,
        ImportLayout::Sheets => BatchLayout::Sheets,
        ImportLayout::Columns => BatchLayout::Columns,
        ImportLayout::Rows => BatchLayout::Rows,
    };
    let kind = match a.read.format {
        Format::Auto => ImportKind::Auto,
        Format::Chi => ImportKind::Chi,
        Format::Ascii => ImportKind::Ascii,
    };
    let opts = BatchOptions {
        layout,
        kind,
        chi: chi_options(&a.read),
        ascii: ascii_options(&a.read)?,
        book_name: a.book_name.clone().unwrap_or_default(),
        file_name_in_comments: a.file_name_comments,
    };
    let result = sheaf_io::batch::import_batch(&mut project, &data_files, &opts);
    let mut plotted = std::collections::HashSet::new();
    for item in &result.items {
        for w in &item.warnings {
            eprintln!("warning: {}: {w}", item.path.display());
        }
        let name = project.workbook(item.book).map(|b| b.name.clone()).unwrap_or_default();
        println!("imported {} into {name}", item.path.display());
        // One graph per book, or per sheet when each file has its own sheet.
        let key = if layout == BatchLayout::Sheets { (item.book, item.sheet) } else { (item.book, 0) };
        if a.plot && plotted.insert(key) {
            let ys = project.default_plot_columns(item.book, item.sheet, item.suggested_template);
            if !ys.is_empty() {
                project.plot_columns(item.suggested_template, &ys)?;
            }
        }
    }
    for e in &result.errors {
        eprintln!("error: {e}");
    }
    if result.items.is_empty() && !data_files.is_empty() {
        bail!("none of the {} file(s) could be imported", data_files.len());
    }
    save_project(&project, &a.output)?;
    if !result.errors.is_empty() {
        bail!("{} file(s) could not be imported (the others were saved)", result.errors.len());
    }
    Ok(())
}

/// Saves the project: `.opj` by default (a path without extension gets `.opj`), with a companion
/// `<name>.opj.sheaf` when the Origin file cannot hold everything; an explicit `.sheaf` path writes
/// a Sheaf project. When the Origin writer refuses the project only the companion is written, and
/// that is reported as an error because the file asked for does not exist.
pub(crate) fn save_project(project: &Project, path: &Path) -> Result<()> {
    // Saving over the Origin file this run read would replace it with a partial rewrite: keep a copy.
    let target = sheaf_io::save::default_extension(path);
    let over_input = {
        let inputs = FOREIGN_INPUTS.lock().unwrap_or_else(|e| e.into_inner());
        inputs.iter().any(|i| match (std::fs::canonicalize(i), std::fs::canonicalize(&target)) {
            (Ok(a), Ok(b)) => a == b,
            _ => *i == target,
        })
    };
    if over_input
        && let Some(b) = sheaf_io::save::backup_original(&target)
            .with_context(|| format!("could not keep a copy of '{}'", target.display()))?
    {
        println!("kept the original Origin file as {}", b.display());
    }
    let outcome =
        sheaf_io::save::save_project(project, path).with_context(|| format!("could not save '{}'", path.display()))?;
    report_save(&outcome)
}

fn report_save(outcome: &sheaf_io::save::SaveOutcome) -> Result<()> {
    if let Some(reason) = &outcome.refusal {
        let companion = outcome.companion.as_ref().map(|c| c.display().to_string()).unwrap_or_default();
        println!("saved {companion} (the whole project)");
        bail!(
            "'{}' was not written because Origin cannot store the project: {reason}. Rename as it says and save again; the project is safe in '{companion}'",
            outcome.path.display()
        );
    }
    println!("saved {}", outcome.path.display());
    if let Some(c) = &outcome.companion {
        println!("saved {} (the whole project; the .opj cannot hold everything)", c.display());
        for w in &outcome.warnings {
            eprintln!("warning: only {} holds: {w}", c.display());
        }
    }
    if let Some(c) = &outcome.companion_removed {
        println!("removed {} (nothing is lost in the .opj any more)", c.display());
    }
    Ok(())
}

/// Finds the Y columns named on the command line in one sheet.
pub(crate) fn named_columns(
    project: &Project,
    book: WindowId,
    sheet: usize,
    names: &[String],
) -> Result<Vec<ColumnId>> {
    let ws = project
        .workbook(book)
        .and_then(|b| b.sheets.get(sheet))
        .ok_or_else(|| anyhow!("the workbook has no such sheet"))?;
    names
        .iter()
        .map(|n| {
            ws.column_by_name(n.trim()).map(|c| c.id).ok_or_else(|| {
                let have: Vec<&str> = ws.columns.iter().map(|c| c.short_name.as_str()).collect();
                anyhow!("column '{}' was not found in sheet '{}' (columns: {})", n, ws.name, have.join(", "))
            })
        })
        .collect()
}

pub(crate) fn pick_book(project: &Project, name: Option<&str>) -> Result<WindowId> {
    match name {
        Some(n) => match project.window_by_name(n) {
            Some(Window::Workbook(b)) => Ok(b.id),
            Some(_) => bail!("'{n}' is not a workbook"),
            None => bail!("there is no workbook named '{n}'"),
        },
        None => project.workbooks().next().map(|b| b.id).ok_or_else(|| anyhow!("the project has no workbook")),
    }
}

pub(crate) fn pick_sheet(project: &Project, book: WindowId, name: Option<&str>) -> Result<usize> {
    let b = project.workbook(book).ok_or_else(|| anyhow!("workbook not found"))?;
    match name {
        Some(n) => b
            .sheets
            .iter()
            .position(|s| s.name.eq_ignore_ascii_case(n))
            .ok_or_else(|| anyhow!("workbook '{}' has no sheet named '{}'", b.name, n)),
        None => Ok(b.active_sheet.min(b.sheets.len().saturating_sub(1))),
    }
}

fn pick_graph(project: &Project, name: Option<&str>) -> Result<WindowId> {
    match name {
        Some(n) => match project.window_by_name(n) {
            Some(Window::Graph(g)) => Ok(g.id),
            Some(_) => bail!("'{n}' is not a graph"),
            None => bail!("there is no graph named '{n}'"),
        },
        None => {
            let mut graphs = project.graphs();
            match (graphs.next(), graphs.next()) {
                (Some(g), None) => Ok(g.id),
                (None, _) => bail!("the project has no graph"),
                _ => {
                    let names: Vec<&str> = project.graphs().map(|g| g.name.as_str()).collect();
                    bail!("the project has several graphs; choose one with --graph ({})", names.join(", "))
                }
            }
        }
    }
}

/// Fails with a clear message unless `value` (when given) is a finite number above zero.
pub(crate) fn require_positive(option: &str, value: Option<f64>) -> Result<()> {
    match value {
        Some(v) if !(v.is_finite() && v > 0.0) => {
            bail!("{option} must be a number greater than 0 (got {v})")
        }
        _ => Ok(()),
    }
}

/// Smallest and largest page side the program accepts, in inches (0.1 in is 7.2 pt).
const PAGE_SIDE_RANGE_IN: (f64, f64) = (0.1, 100.0);

/// Like [`require_positive`] for a page side, which must also lie within `PAGE_SIDE_RANGE_IN`,
/// so that a tiny or huge value is refused here and not later by the renderer.
fn require_page_size(option: &str, value: Option<f64>) -> Result<()> {
    require_positive(option, value)?;
    let (lo, hi) = PAGE_SIDE_RANGE_IN;
    match value {
        Some(v) if v < lo || v > hi => {
            bail!("{option} must be between {lo} and {hi} inches (got {v})")
        }
        _ => Ok(()),
    }
}

/// Fails when the two limits of an axis are the same number, which has no width to draw.
/// A lower limit above the upper one is allowed: it draws a reversed axis.
fn check_limits(axis: &str, min: Option<f64>, max: Option<f64>) -> Result<()> {
    for (name, v) in [("min", min), ("max", max)] {
        if matches!(v, Some(v) if !v.is_finite()) {
            bail!("--{axis}-{name} must be a finite number");
        }
    }
    if let (Some(a), Some(b)) = (min, max)
        && a == b
    {
        bail!("--{axis}-min and --{axis}-max are both {a}; the axis would have no length");
    }
    if let (Some(a), Some(b)) = (min, max)
        && !(b - a).is_finite()
    {
        bail!("--{axis}-min and --{axis}-max are too far apart ({a} and {b}); use limits closer together");
    }
    Ok(())
}

/// Applies the style options of the command line to a graph.
fn apply_style(project: &mut Project, graph: WindowId, s: &StyleArgs) -> Result<()> {
    require_page_size("--width", s.width)?;
    require_page_size("--height", s.height)?;
    require_positive("--line-width", s.line_width)?;
    check_limits("x", s.x_min, s.x_max)?;
    check_limits("y", s.y_min, s.y_max)?;
    let colors: Vec<Color> =
        s.colors.iter().map(|c| c.parse::<Color>().map_err(|e| anyhow!("{e}"))).collect::<Result<_>>()?;
    let g = project.graph_mut(graph).ok_or_else(|| anyhow!("graph not found"))?;
    if let Some(w) = s.width {
        g.page.width_pt = w * 72.0;
    }
    if let Some(h) = s.height {
        g.page.height_pt = h * 72.0;
    }
    let Some(layer) = g.layers.first_mut() else {
        return Ok(());
    };
    if let Some(t) = &s.x_title {
        layer.x_axis.title = Some(t.clone());
    }
    if let Some(t) = &s.y_title {
        layer.y_axis.title = Some(t.clone());
    }
    layer.x_axis.min = s.x_min.or(layer.x_axis.min);
    layer.x_axis.max = s.x_max.or(layer.x_axis.max);
    layer.y_axis.min = s.y_min.or(layer.y_axis.min);
    layer.y_axis.max = s.y_max.or(layer.y_axis.max);
    layer.x_axis.major_step = s.x_step.or(layer.x_axis.major_step);
    layer.y_axis.major_step = s.y_step.or(layer.y_axis.major_step);
    if s.no_legend {
        layer.legend.visible = false;
    }
    for (i, plot) in layer.plots.iter_mut().enumerate() {
        if let Some(w) = s.line_width {
            plot.line = LineStyle { width_pt: w, ..plot.line.clone() };
        }
        if let Some(c) = colors.get(i) {
            plot.line.color = *c;
            plot.symbol.color = *c;
        }
        if let Some(l) = s.labels.get(i) {
            plot.label = Some(l.clone());
        }
    }
    Ok(())
}

fn template_from(name: Option<&str>, fallback: Template) -> Result<Template> {
    match name {
        None => Ok(fallback),
        Some(n) => Template::from_name(n).ok_or_else(|| {
            let names: Vec<&str> = Template::ALL.iter().map(Template::name).collect();
            anyhow!("unknown template '{}'; choose one of: {}", n, names.join(", "))
        }),
    }
}

fn plot(a: PlotArgs) -> Result<()> {
    require_positive("--dpi", Some(a.dpi))?;
    let first = &a.inputs[0];
    if a.inputs.len() > 1 && a.inputs.iter().any(|p| is_project(p)) {
        bail!("a project cannot be combined with other inputs; give one project, or one or more data files");
    }
    let (mut project, suggested) = if is_project(first) {
        (load_project(first)?, Template::Line)
    } else {
        let mut p = Project::new();
        let mut template = Template::Line;
        for (i, input) in a.inputs.iter().enumerate() {
            let imported = read_data_file(input, &a.read)?;
            if i == 0 {
                template = imported.suggested_template;
            }
            p.adopt_workbook(imported.workbook);
        }
        (p, template)
    };

    let graph = if let Some(name) = &a.graph {
        pick_graph(&project, Some(name))?
    } else if a.inputs.len() > 1 {
        overlay_files(&mut project, template_from(a.template.as_deref(), suggested)?, &a)?
    } else {
        let template = template_from(a.template.as_deref(), suggested)?;
        let book = pick_book(&project, a.book.as_deref())?;
        let sheet = pick_sheet(&project, book, a.sheet.as_deref())?;
        let ys = if a.y.is_empty() {
            project.default_plot_columns(book, sheet, template)
        } else {
            named_columns(&project, book, sheet, &a.y)?
        };
        if ys.is_empty() {
            bail!("there are no numeric Y columns to plot");
        }
        project.plot_columns(template, &ys)?
    };
    apply_style(&mut project, graph, &a.style)?;

    sheaf_plot::export_file(&project, graph, &a.output, a.dpi)
        .with_context(|| format!("could not write '{}'", a.output.display()))?;
    println!("wrote {}", a.output.display());
    if let Some(path) = &a.save {
        save_project(&project, path)?;
    }
    Ok(())
}

/// One graph with the chosen (or default) Y columns of every imported file.
/// Legend entries name the file, plus the column when a file gives several curves.
fn overlay_files(project: &mut Project, template: Template, a: &PlotArgs) -> Result<WindowId> {
    let books: Vec<WindowId> = project.workbooks().map(|b| b.id).collect();
    let mut ys: Vec<ColumnId> = Vec::new();
    let mut labels: Vec<String> = Vec::new();
    for book in books {
        let sheet = pick_sheet(project, book, a.sheet.as_deref())?;
        let columns = if a.y.is_empty() {
            project.default_plot_columns(book, sheet, template)
        } else {
            named_columns(project, book, sheet, &a.y)?
        };
        let file = project.workbook(book).map(|b| b.long_name.clone()).unwrap_or_default();
        for y in &columns {
            let column = project
                .column(*y)
                .map(|c| c.comments.lines().next().filter(|l| !l.is_empty()).unwrap_or(c.display_name()).to_string());
            labels.push(match (columns.len(), column) {
                (1, _) | (_, None) => file.clone(),
                (_, Some(c)) => format!("{file} {c}"),
            });
        }
        ys.extend(columns);
    }
    if ys.is_empty() {
        bail!("there are no numeric Y columns to plot");
    }
    let graph = project.plot_columns(template, &ys)?;
    if let Some(layer) = project.graph_mut(graph).and_then(|g| g.layers.first_mut()) {
        for (plot, label) in layer.plots.iter_mut().zip(labels) {
            plot.label = Some(label);
        }
    }
    Ok(graph)
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
        p.adopt_workbook(read_data_file(&a.project, &a.read)?.workbook);
        p
    };
    save_project(&project, &a.output)
}

fn export(a: ExportArgs) -> Result<()> {
    if is_opj(&a.output) {
        return export_opj(&a);
    }
    require_positive("--dpi", Some(a.dpi))?;
    if !is_project(&a.project) {
        bail!(
            "'{}' is not a project; figures are exported from a .sheaf or .opj project (use `sheaf plot` for a data file)",
            a.project.display()
        );
    }
    let mut project = load_project(&a.project)?;
    let graph = pick_graph(&project, a.graph.as_deref())?;
    apply_style(&mut project, graph, &a.style)?;
    sheaf_plot::export_file(&project, graph, &a.output, a.dpi)
        .with_context(|| format!("could not write '{}'", a.output.display()))?;
    println!("wrote {}", a.output.display());
    Ok(())
}

fn export_data(a: ExportDataArgs) -> Result<()> {
    let project = load_project(&a.project)?;
    let book = pick_book(&project, a.book.as_deref())?;
    let sheet = pick_sheet(&project, book, a.sheet.as_deref())?;
    let ws = &project.workbook(book).expect("picked above").sheets[sheet];
    write_sheet(ws, &a.output)
}

/// Writes a sheet as delimited text: tabs for .tsv and .txt, commas otherwise.
pub(crate) fn write_sheet(ws: &sheaf_core::Worksheet, path: &Path) -> Result<()> {
    let tabs = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("tsv") || e.eq_ignore_ascii_case("txt"));
    let opts = sheaf_io::export::AsciiExportOptions {
        delimiter: if tabs { '\t' } else { ',' },
        ..sheaf_io::export::AsciiExportOptions::default()
    };
    let file = std::fs::File::create(path).with_context(|| format!("could not create '{}'", path.display()))?;
    let mut out = std::io::BufWriter::new(file);
    sheaf_io::export::write_delimited(ws, &mut out, &opts)
        .with_context(|| format!("could not write '{}'", path.display()))?;
    println!("wrote {}", path.display());
    Ok(())
}
