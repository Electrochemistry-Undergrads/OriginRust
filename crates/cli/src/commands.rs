//! What each subcommand does.

use crate::args::*;
use anyhow::{Context, Result, anyhow, bail};
use sheaf_core::graph::{Graph, LineStyle, PlotKind, Stack, TickDirection, TickLabelSource};
use sheaf_core::panels::{MAX_PANELS, PanelLayout, PanelOrder};
use sheaf_core::project::Window;
use sheaf_core::template::{STACK_AUTO_DEFAULT, Template};
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
        Command::Plot(a) => plot(*a),
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
        split_segments: !r.no_split_segments,
        add_milliamp: !r.no_milliamp,
        add_charge: !r.no_charge,
        invert_current: r.invert_current,
        exact_time: !r.keep_file_times,
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
///
/// The options act on the first layer and the layers linked to it (its right Y axis, or the other
/// panels of a stack); with `every_panel` (a graph just made with `--panels`) they act on every
/// layer. Among those layers, the panels ([`Graph::panels`]) take `--x-*` and `--y-*` (with
/// `--x-labels`, `--x-label-angle`, `--stack`, `--no-y-tick-labels` and `--no-y-ticks`); a linked
/// X axis follows the first panel's. `--y2-*` set the first panel's right Y axis; `--bar-gap` and
/// `--bar-overlap` set every layer; colors, labels and line widths count the plots in legend order,
/// layer by layer. `--offsets` and `--waterfall` need a graph of one panel. Further layers of a
/// graph read from an Origin project are left alone.
fn apply_style(project: &mut Project, graph: WindowId, s: &StyleArgs, every_panel: bool) -> Result<()> {
    require_page_size("--width", s.width)?;
    require_page_size("--height", s.height)?;
    require_positive("--line-width", s.line_width)?;
    check_limits("x", s.x_min, s.x_max)?;
    check_limits("y", s.y_min, s.y_max)?;
    check_limits("y2", s.y2_min, s.y2_max)?;
    let colors: Vec<Color> =
        s.colors.iter().map(|c| c.parse::<Color>().map_err(|e| anyhow!("{e}"))).collect::<Result<_>>()?;
    let stack = stack_option(s)?;
    require_range("--x-label-angle", s.x_label_angle, -90.0, 90.0)?;
    require_range("--bar-gap", s.bar_gap, 0.0, 100.0)?;
    require_range("--bar-overlap", s.bar_overlap, -100.0, 100.0)?;
    let x_labels = s.x_labels.as_deref().map(|text| x_label_source(project, graph, text)).transpose()?;
    let g = project.graph_mut(graph).ok_or_else(|| anyhow!("graph not found"))?;
    // The layers the options act on, in legend order, and the panels among them.
    let roots: Vec<usize> =
        if every_panel { (0..g.layers.len()).filter(|&i| g.root_of(i) == i).collect() } else { vec![0] };
    let styled: Vec<usize> = roots.iter().flat_map(|&r| g.group(r)).collect();
    let panels = g.panels();
    let mains: Vec<usize> = styled.iter().copied().filter(|i| panels.contains(i)).collect();
    check_stack_fits(g, s, mains.len())?;
    let right = g.right_y_member(0);
    let wants_y2 = s.y2_title.is_some() || s.y2_min.is_some() || s.y2_max.is_some() || s.y2_step.is_some();
    if wants_y2 && right.is_none() {
        bail!(
            "the graph has no right Y axis for --y2-title, --y2-min, --y2-max or --y2-step; \
             make one with --y2 COLUMNS or -t double-y"
        );
    }
    if let Some(w) = s.width {
        g.page.width_pt = w * 72.0;
    }
    if let Some(h) = s.height {
        g.page.height_pt = h * 72.0;
    }
    if let Some(r) = right {
        let y2 = &mut g.layers[r].y_axis;
        if let Some(t) = &s.y2_title {
            y2.title = Some(t.clone());
        }
        y2.min = s.y2_min.or(y2.min);
        y2.max = s.y2_max.or(y2.max);
        y2.major_step = s.y2_step.or(y2.major_step);
    }
    if let Some(mode) = s.axis_colors {
        for &i in &styled {
            g.layers[i].y_axis.color_follows_plot = mode == AxisColors::Plot;
        }
    }
    for &i in &styled {
        let bars = &mut g.layers[i].bars;
        bars.gap_pct = s.bar_gap.unwrap_or(bars.gap_pct);
        bars.overlap_pct = s.bar_overlap.unwrap_or(bars.overlap_pct);
    }
    style_plots(g, &styled, s, &colors);
    for &i in &mains {
        let layer = &mut g.layers[i];
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
        if let Some(stack) = stack {
            layer.stack = stack;
        }
        if s.no_y_tick_labels {
            layer.y_axis.show_tick_labels = false;
        }
        if s.no_y_ticks {
            layer.y_axis.major_tick = TickDirection::None;
            layer.y_axis.minor_tick = TickDirection::None;
        }
        if let Some(source) = x_labels {
            layer.x_axis.tick_label_source = source;
        }
        layer.x_axis.tick_label_angle_deg = s.x_label_angle.unwrap_or(layer.x_axis.tick_label_angle_deg);
    }
    if !s.offsets.is_empty()
        && let Some(layer) = g.layers.first_mut()
    {
        layer.stack = Stack::Individual;
        for (column, y) in stack_members(layer).into_iter().zip(&s.offsets) {
            layer.plot_offset_mut(column).y = *y;
        }
    }
    Ok(())
}

/// Fails with a clear message unless `value` (when given) is a number from `lo` to `hi`.
fn require_range(option: &str, value: Option<f64>, lo: f64, hi: f64) -> Result<()> {
    match value {
        Some(v) if !(v.is_finite() && (lo..=hi).contains(&v)) => {
            bail!("{option} must be between {lo} and {hi} (got {v})")
        }
        _ => Ok(()),
    }
}

/// Reads the value of `--x-labels`: `auto`, `numbers`, or the short or long name of a column in the
/// sheet that holds the Y column of the graph's first curve.
fn x_label_source(project: &Project, graph: WindowId, text: &str) -> Result<TickLabelSource> {
    let name = text.trim();
    if name.eq_ignore_ascii_case("auto") {
        return Ok(TickLabelSource::Auto);
    }
    if name.eq_ignore_ascii_case("numbers") {
        return Ok(TickLabelSource::Numbers);
    }
    let first = project
        .graph(graph)
        .and_then(|g| g.layers.first())
        .and_then(|l| l.plots.first())
        .ok_or_else(|| anyhow!("--x-labels {name}: the graph has no curve whose sheet holds the column"))?;
    let (_, sheet, _) = project
        .locate(first.y)
        .ok_or_else(|| anyhow!("--x-labels {name}: the first curve's column is missing from the project"))?;
    match sheet.column_by_name(name) {
        Some(c) => Ok(TickLabelSource::Column { column: c.id }),
        None => {
            let have: Vec<&str> = sheet.columns.iter().map(|c| c.short_name.as_str()).collect();
            bail!(
                "--x-labels: no column named '{name}' in sheet '{}' (columns: {}); use auto, numbers or a column name",
                sheet.name,
                have.join(", ")
            )
        }
    }
}

/// The stack mode asked for with `--stack` or `--waterfall`, checked (`--offsets` is applied
/// separately because it also sets offsets).
fn stack_option(s: &StyleArgs) -> Result<Option<Stack>> {
    if let Some(text) = &s.stack {
        return parse_stack(text).map(Some);
    }
    if s.waterfall.is_empty() {
        return Ok(None);
    }
    let [dx, dy] = s.waterfall[..] else {
        bail!("--waterfall needs two numbers, DX,DY in percent (got {} number(s))", s.waterfall.len());
    };
    for v in [dx, dy] {
        if !(v.is_finite() && (-100.0..=100.0).contains(&v)) {
            bail!("--waterfall takes percentages from -100 to 100 (got {v})");
        }
    }
    Ok(Some(Stack::Waterfall { dx_percent: dx, dy_percent: dy }))
}

/// Reads the value of `--stack`: `none`, `auto`, `auto:GAP` or `step:STEP`.
fn parse_stack(text: &str) -> Result<Stack> {
    let t = text.trim().to_ascii_lowercase();
    let number = |v: &str| v.trim().parse::<f64>().ok().filter(|v| v.is_finite());
    if t == "none" {
        return Ok(Stack::None);
    }
    if t == "auto" {
        return Ok(STACK_AUTO_DEFAULT);
    }
    if let Some(gap) = t.strip_prefix("auto:") {
        return match number(gap) {
            Some(g) if g >= 0.0 => Ok(Stack::Auto { gap_percent: g }),
            _ => bail!("--stack auto:GAP needs a gap in percent that is a number of at least 0 (got '{gap}')"),
        };
    }
    if let Some(step) = t.strip_prefix("step:") {
        return match number(step) {
            Some(v) => Ok(Stack::Constant { step: v }),
            None => bail!("--stack step:STEP needs a step that is a number (got '{step}')"),
        };
    }
    bail!("--stack must be none, auto, auto:GAP or step:STEP (got '{text}')")
}

/// The Y columns that have a place of their own in a layer's stack, in plot order: every plotted
/// column once, except curves that follow another one (a fitted curve drawn over its data).
/// `--offsets` gives its values to these, so a fit curve does not take one.
fn stack_members(layer: &sheaf_core::graph::Layer) -> Vec<ColumnId> {
    let mut out: Vec<ColumnId> = Vec::new();
    for p in &layer.plots {
        if layer.follows(p.y).is_none() && !out.contains(&p.y) {
            out.push(p.y);
        }
    }
    out
}

/// Refuses stack options that do not fit the graph: `--offsets` or `--waterfall` on a graph of
/// several panels (`panels` is the number of panels the options act on), more `--offsets` than
/// the first layer has curves, and a waterfall in a graph with a right Y axis (a waterfall is drawn
/// only for a layer on its own).
fn check_stack_fits(g: &Graph, s: &StyleArgs, panels: usize) -> Result<()> {
    if panels > 1 && (!s.offsets.is_empty() || !s.waterfall.is_empty()) {
        bail!("--offsets and --waterfall work on a graph of one panel; this graph has {panels} panels");
    }
    let plots = g.layers.first().map_or(0, |l| stack_members(l).len());
    if s.offsets.len() > plots {
        bail!("--offsets gives {} values but the graph has {} curve(s)", s.offsets.len(), plots);
    }
    if !s.waterfall.is_empty() && g.group(0).len() > 1 {
        bail!("--waterfall cannot be used on a graph with a right Y axis");
    }
    Ok(())
}

/// Line widths, colors, labels and `--no-legend` for the plots of the layers in `group`, counted
/// in legend order (layer by layer, plots in order).
fn style_plots(g: &mut Graph, group: &[usize], s: &StyleArgs, colors: &[Color]) {
    let mut i = 0;
    for &li in group {
        let layer = &mut g.layers[li];
        if s.no_legend {
            layer.legend.visible = false;
        }
        for plot in &mut layer.plots {
            if let Some(w) = s.line_width {
                plot.line = LineStyle { width_pt: w, ..plot.line.clone() };
            }
            if let Some(c) = colors.get(i) {
                // A column plot's color is its fill; its outline keeps its own color.
                if plot.kind != PlotKind::Column {
                    plot.line.color = *c;
                }
                plot.symbol.color = *c;
            }
            if let Some(l) = s.labels.get(i) {
                plot.label = Some(l.clone());
            }
            i += 1;
        }
    }
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
    if !a.y2.is_empty() && a.template.as_deref().and_then(Template::from_name) == Some(Template::Waterfall) {
        bail!("a waterfall cannot have a right Y axis; leave out --y2 or choose another template");
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
    } else if let Some(spec) = &a.panels {
        panel_graph(&mut project, template_from(a.template.as_deref(), suggested)?, spec, &a)?
    } else if a.inputs.len() > 1 {
        overlay_files(&mut project, template_from(a.template.as_deref(), suggested)?, &a)?
    } else {
        let template = template_from(a.template.as_deref(), suggested)?;
        let book = pick_book(&project, a.book.as_deref())?;
        let sheet = pick_sheet(&project, book, a.sheet.as_deref())?;
        let (left, right) = axis_columns(&project, book, sheet, template, &a)?;
        let ys: Vec<ColumnId> = left.iter().chain(&right).copied().collect();
        if ys.is_empty() {
            bail!("there are no numeric Y columns to plot");
        }
        if a.y2.is_empty() && template != Template::DoubleY {
            project.plot_columns(template, &ys)?
        } else {
            require_two_axes(&left, &right)?;
            project.plot_columns_double_y(template, &left, &right)?
        }
    };
    apply_style(&mut project, graph, &a.style, a.panels.is_some())?;

    let size = export_size(a.image_width.as_deref(), a.image_height.as_deref(), a.dpi)?;
    sheaf_plot::export_file_sized(&project, graph, &a.output, a.dpi, size)
        .with_context(|| format!("could not write '{}'", a.output.display()))?;
    println!("wrote {}", a.output.display());
    if let Some(path) = &a.save {
        save_project(&project, path)?;
    }
    Ok(())
}

/// The shape asked for with `--panels`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum PanelShape {
    Stack,
    Grid { rows: usize, cols: usize },
}

/// Reads the value of `--panels`: `stack`, or ROWSxCOLUMNS such as `2x2`.
fn parse_panels(text: &str) -> Result<PanelShape> {
    let t = text.trim().to_ascii_lowercase();
    if t == "stack" {
        return Ok(PanelShape::Stack);
    }
    let bad = || anyhow!("--panels must be 'stack' or ROWSxCOLUMNS such as 2x2 (got '{text}')");
    let (r, c) = t.split_once('x').ok_or_else(bad)?;
    let rows: usize = r.trim().parse().map_err(|_| bad())?;
    let cols: usize = c.trim().parse().map_err(|_| bad())?;
    if rows == 0 || cols == 0 || rows.saturating_mul(cols) > MAX_PANELS {
        bail!("--panels {text}: rows x columns must be from 1 to {MAX_PANELS} panels (got {rows} x {cols})");
    }
    Ok(PanelShape::Grid { rows, cols })
}

/// The curves of each panel, before `--per-panel`, and the legend text of the curves that need
/// one (several files: the file name).
///
/// With several files, each file is one group (its `-y` or default columns), or with `--panel-y`
/// each group takes the named columns of every file. With one input, each plotted Y column (`-y`
/// or the default columns) is a group, or `--panel-y` names the columns of each group.
fn panel_groups(
    project: &Project,
    template: Template,
    a: &PlotArgs,
) -> Result<(Vec<Vec<ColumnId>>, Vec<(ColumnId, String)>)> {
    let split = |spec: &String| -> Vec<String> { spec.split(',').map(|n| n.trim().to_string()).collect() };
    let mut labels = Vec::new();
    if a.inputs.len() > 1 {
        let books: Vec<WindowId> = project.workbooks().map(|b| b.id).collect();
        let mut sheets = Vec::new();
        for &book in &books {
            sheets.push((book, pick_sheet(project, book, a.sheet.as_deref())?));
        }
        let mut groups = Vec::new();
        if a.panel_y.is_empty() {
            for &(book, sheet) in &sheets {
                let ys = if a.y.is_empty() {
                    project.default_plot_columns(book, sheet, template)
                } else {
                    named_columns(project, book, sheet, &a.y)?
                };
                labels.extend(ys.iter().copied().zip(overlay_labels(project, book, &ys)));
                groups.push(ys);
            }
        } else {
            for spec in &a.panel_y {
                let mut group = Vec::new();
                for &(book, sheet) in &sheets {
                    let ys = named_columns(project, book, sheet, &split(spec))?;
                    labels.extend(ys.iter().copied().zip(overlay_labels(project, book, &ys)));
                    group.extend(ys);
                }
                groups.push(group);
            }
        }
        return Ok((groups, labels));
    }
    let book = pick_book(project, a.book.as_deref())?;
    let sheet = pick_sheet(project, book, a.sheet.as_deref())?;
    if !a.panel_y.is_empty() {
        let groups =
            a.panel_y.iter().map(|spec| named_columns(project, book, sheet, &split(spec))).collect::<Result<_>>()?;
        return Ok((groups, labels));
    }
    let ys = if a.y.is_empty() {
        project.default_plot_columns(book, sheet, template)
    } else {
        named_columns(project, book, sheet, &a.y)?
    };
    Ok((ys.into_iter().map(|y| vec![y]).collect(), labels))
}

/// Puts the curves of `groups`, in order, into panels of `--per-panel` curves each.
fn regroup(groups: Vec<Vec<ColumnId>>, per_panel: &[usize]) -> Result<Vec<Vec<ColumnId>>> {
    if per_panel.is_empty() {
        return Ok(groups);
    }
    if per_panel.contains(&0) {
        bail!("--per-panel counts must be 1 or more");
    }
    let curves: Vec<ColumnId> = groups.into_iter().flatten().collect();
    let sum: usize = per_panel.iter().sum();
    if sum != curves.len() {
        bail!("--per-panel counts add up to {sum} but there are {} curves", curves.len());
    }
    let mut rest = &curves[..];
    Ok(per_panel
        .iter()
        .map(|&n| {
            let (head, tail) = rest.split_at(n);
            rest = tail;
            head.to_vec()
        })
        .collect())
}

/// The layout of `--panels` with the `--panel-*` options applied.
fn panel_layout(shape: PanelShape, panels: usize, template: Template, a: &PlotArgs) -> Result<PanelLayout> {
    let mut layout = match shape {
        PanelShape::Stack => {
            if panels > MAX_PANELS {
                bail!("--panels stack can hold at most {MAX_PANELS} panels, but there are {panels} to stack");
            }
            PanelLayout::stack(panels)
        }
        PanelShape::Grid { rows, cols } => {
            PanelLayout::grid(rows, cols, &template.new_graph(WindowId(0), "").layers[0].frame)
        }
    };
    let check = |option: &str, v: f64, hi: f64| -> Result<f64> {
        if v.is_finite() && (0.0..=hi).contains(&v) {
            Ok(v)
        } else {
            bail!("{option} must be from 0 to {hi} percent of the page (got {v})")
        }
    };
    match a.panel_gap[..] {
        [] => {}
        [both] => {
            layout.h_gap = check("--panel-gap", both, 50.0)?;
            layout.v_gap = layout.h_gap;
        }
        [h, v] => {
            layout.h_gap = check("--panel-gap", h, 50.0)?;
            layout.v_gap = check("--panel-gap", v, 50.0)?;
        }
        _ => bail!("--panel-gap takes one number, or two as H,V (got {} numbers)", a.panel_gap.len()),
    }
    match a.panel_margins[..] {
        [] => {}
        [l, r, t, b] => {
            layout.margin_left = check("--panel-margins", l, 90.0)?;
            layout.margin_right = check("--panel-margins", r, 90.0)?;
            layout.margin_top = check("--panel-margins", t, 90.0)?;
            layout.margin_bottom = check("--panel-margins", b, 90.0)?;
        }
        _ => bail!("--panel-margins takes four numbers, L,R,T,B (got {} numbers)", a.panel_margins.len()),
    }
    if let Some(order) = a.panel_order {
        layout.order = match order {
            PanelOrderArg::BottomUp => PanelOrder::BottomUp,
            PanelOrderArg::TopDown => PanelOrder::TopDown,
        };
    }
    if let Some(link) = a.panel_link {
        (layout.link_x, layout.link_y) = match link {
            PanelLinkArg::X => (true, false),
            PanelLinkArg::Y => (false, true),
            PanelLinkArg::Xy => (true, true),
            PanelLinkArg::None => (false, false),
        };
    }
    Ok(layout)
}

/// One graph with several panels (`--panels`), graphing spec section 15.6.
fn panel_graph(project: &mut Project, template: Template, spec: &str, a: &PlotArgs) -> Result<WindowId> {
    let shape = parse_panels(spec)?;
    if matches!(template, Template::DoubleY | Template::Waterfall) {
        bail!("the {} template cannot be used with --panels; choose another template", template.name());
    }
    let (groups, labels) = panel_groups(project, template, a)?;
    if groups.iter().all(|g| g.is_empty()) {
        bail!("there are no numeric Y columns to plot");
    }
    let groups = regroup(groups, &a.per_panel)?;
    let layout = panel_layout(shape, groups.len(), template, a)?;
    let graph = project.plot_panels(template, &groups, &layout)?;
    if let Some(g) = project.graph_mut(graph) {
        for plot in g.layers.iter_mut().flat_map(|l| l.plots.iter_mut()) {
            if let Some((_, label)) = labels.iter().find(|(c, _)| *c == plot.y) {
                plot.label = Some(label.clone());
            }
        }
    }
    Ok(graph)
}

/// The Y columns of one sheet for the left and the right axis.
///
/// With `--y2`, the right axis gets those columns and the left axis the `-y` columns (or the
/// default columns without the `--y2` ones). With the double-y template and no `--y2`, the
/// columns (`-y`, or every numeric Y column) alternate: 1st, 3rd, ... left; 2nd, 4th, ... right.
/// Otherwise every column is on the left axis.
fn axis_columns(
    project: &Project,
    book: WindowId,
    sheet: usize,
    template: Template,
    a: &PlotArgs,
) -> Result<(Vec<ColumnId>, Vec<ColumnId>)> {
    let named = |names: &[String]| named_columns(project, book, sheet, names);
    if a.y2.is_empty() {
        let ys = if a.y.is_empty() { project.default_plot_columns(book, sheet, template) } else { named(&a.y)? };
        if template == Template::DoubleY {
            return Ok(alternate(&ys));
        }
        return Ok((ys, Vec::new()));
    }
    let right = named(&a.y2)?;
    let left = if a.y.is_empty() {
        let mut ys = project.default_plot_columns(book, sheet, template);
        ys.retain(|c| !right.contains(c));
        ys
    } else {
        named(&a.y)?
    };
    if left.iter().any(|c| right.contains(c)) {
        bail!("a column cannot be on both axes; a column named with --y2 must not be named with -y");
    }
    Ok((left, right))
}

/// Splits columns as Origin's Double-Y command does: 1st, 3rd, ... left; 2nd, 4th, ... right.
fn alternate(ys: &[ColumnId]) -> (Vec<ColumnId>, Vec<ColumnId>) {
    (ys.iter().step_by(2).copied().collect(), ys.iter().skip(1).step_by(2).copied().collect())
}

/// A double-Y graph needs a curve for each axis.
fn require_two_axes(left: &[ColumnId], right: &[ColumnId]) -> Result<()> {
    if left.is_empty() || right.is_empty() {
        bail!(
            "a double-Y graph needs at least two Y columns, one for each axis (left: {}, right: {})",
            left.len(),
            right.len()
        );
    }
    Ok(())
}

/// The legend text of each column of one file in an overlay: the file name, plus the column when
/// the file gives several curves.
fn overlay_labels(project: &Project, book: WindowId, columns: &[ColumnId]) -> Vec<String> {
    let file = project.workbook(book).map(|b| b.long_name.clone()).unwrap_or_default();
    columns
        .iter()
        .map(|y| {
            let column = project
                .column(*y)
                .map(|c| c.comments.lines().next().filter(|l| !l.is_empty()).unwrap_or(c.display_name()).to_string());
            match (columns.len(), column) {
                (1, _) | (_, None) => file.clone(),
                (_, Some(c)) => format!("{file} {c}"),
            }
        })
        .collect()
}

/// One graph with the chosen (or default) Y columns of every imported file.
/// Legend entries name the file, plus the column when a file gives several curves.
/// With `--y2` (or the double-y template) each file's columns are split between the axes as for a
/// single file, and the right axis lists its curves after the left axis' curves.
fn overlay_files(project: &mut Project, template: Template, a: &PlotArgs) -> Result<WindowId> {
    let books: Vec<WindowId> = project.workbooks().map(|b| b.id).collect();
    let (mut left, mut right) = (Vec::new(), Vec::new());
    let (mut left_labels, mut right_labels) = (Vec::new(), Vec::new());
    for book in books {
        let sheet = pick_sheet(project, book, a.sheet.as_deref())?;
        let (l, r) = axis_columns(project, book, sheet, template, a)?;
        // Labels are made over the file's columns together, so that a file with one curve per
        // axis still names the column.
        let all: Vec<ColumnId> = l.iter().chain(&r).copied().collect();
        let labels = overlay_labels(project, book, &all);
        left_labels.extend_from_slice(&labels[..l.len()]);
        right_labels.extend_from_slice(&labels[l.len()..]);
        left.extend(l);
        right.extend(r);
    }
    if left.is_empty() && right.is_empty() {
        bail!("there are no numeric Y columns to plot");
    }
    let two_axes = !a.y2.is_empty() || template == Template::DoubleY;
    let graph = if two_axes {
        require_two_axes(&left, &right)?;
        project.plot_columns_double_y(template, &left, &right)?
    } else {
        project.plot_columns(template, &left)?
    };
    if let Some(g) = project.graph_mut(graph) {
        let right_layer = g.right_y_member(0);
        let mut set = |layer: Option<usize>, labels: Vec<String>| {
            if let Some(layer) = layer.and_then(|i| g.layers.get_mut(i)) {
                for (plot, label) in layer.plots.iter_mut().zip(labels) {
                    plot.label = Some(label);
                }
            }
        };
        set(Some(0), left_labels);
        set(right_layer.filter(|_| two_axes), right_labels);
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
    apply_style(&mut project, graph, &a.style, false)?;
    let size = export_size(a.image_width.as_deref(), a.image_height.as_deref(), a.dpi)?;
    sheaf_plot::export_file_sized(&project, graph, &a.output, a.dpi, size)
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

/// The export size from `--image-width` and `--image-height`.
fn export_size(width: Option<&str>, height: Option<&str>, dpi: f64) -> Result<sheaf_plot::ExportSize> {
    let inches = |text: Option<&str>, what: &str| -> Result<Option<f64>> {
        text.map(|t| parse_length_inches(t, dpi).with_context(|| format!("--image-{what} '{t}'"))).transpose()
    };
    Ok(sheaf_plot::ExportSize { width_in: inches(width, "width")?, height_in: inches(height, "height")? })
}

/// A length with its unit (`in`, `cm`, `mm`, `pt` or `px`, the last at `dpi`) in inches.
pub(crate) fn parse_length_inches(text: &str, dpi: f64) -> Result<f64> {
    let t = text.trim().to_ascii_lowercase();
    let split = t.find(|c: char| c.is_ascii_alphabetic()).unwrap_or(t.len());
    let (number, unit) = t.split_at(split);
    let value: f64 =
        number.trim().parse().map_err(|_| anyhow::anyhow!("'{text}' is not a length such as 3.5in or 8.5cm"))?;
    let per_inch = match unit.trim() {
        "in" | "inch" | "inches" => 1.0,
        "cm" => 2.54,
        "mm" => 25.4,
        "pt" => 72.0,
        "px" if dpi > 0.0 => dpi,
        "" => bail!("'{text}' needs a unit: in, cm, mm, pt or px"),
        other => bail!("'{other}' is not a unit; use in, cm, mm, pt or px"),
    };
    if !(value.is_finite() && value > 0.0) {
        bail!("a length must be positive, not '{text}'");
    }
    Ok(value / per_inch)
}

#[cfg(test)]
mod length_tests {
    use super::parse_length_inches;

    #[test]
    fn lengths_need_a_unit_and_convert_to_inches() {
        let close = |a: f64, b: f64| (a - b).abs() < 1e-12;
        assert!(close(parse_length_inches("3.5in", 300.0).unwrap(), 3.5));
        assert!(close(parse_length_inches("8.5 cm", 300.0).unwrap(), 8.5 / 2.54));
        assert!(close(parse_length_inches("85mm", 300.0).unwrap(), 85.0 / 25.4));
        assert!(close(parse_length_inches("252pt", 300.0).unwrap(), 3.5));
        assert!(close(parse_length_inches("1050PX", 300.0).unwrap(), 3.5));
        for bad in ["3.5", "3.5furlong", "-2in", "0cm", "in", "abc"] {
            assert!(parse_length_inches(bad, 300.0).is_err(), "{bad}");
        }
    }
}
