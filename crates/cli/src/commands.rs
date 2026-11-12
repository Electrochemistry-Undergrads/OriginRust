//! What each subcommand does.

use crate::args::*;
use anyhow::{Context, Result, anyhow, bail};
use sheaf_core::graph::{Graph, LineStyle, Stack, TickDirection};

use sheaf_core::project::Window;
use sheaf_core::template::{STACK_AUTO_DEFAULT, Template};
use sheaf_core::{Color, ColumnId, Project, WindowId};
use sheaf_io::ascii::{AsciiOptions, Delimiter};
use sheaf_io::chi::ChiOptions;
use sheaf_io::{ImportKind, Imported, project_file};
use std::path::Path;

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
pub(crate) fn load_project(path: &Path) -> Result<Project> {
    let opened =
        sheaf_io::open_project(path).with_context(|| format!("could not open project '{}'", path.display()))?;
    for w in &opened.warnings {
        eprintln!("note: {w}");
    }
    Ok(opened.project)
}

fn chi_options(r: &ReadOptions) -> ChiOptions {
    ChiOptions {
        split_scans: !r.no_split_scans,
        add_milliamp: !r.no_milliamp,
        add_charge: !r.no_charge,
        invert_current: r.invert_current,
    }
}

fn ascii_options(r: &ReadOptions) -> Result<AsciiOptions> {
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
                        let kind = if c.data.is_numeric() { "" } else { " text" };
                        let mut label = c.long_name.clone();
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

fn import(a: ImportArgs) -> Result<()> {
    if a.files.iter().any(|f| f == &a.output) {
        bail!("the output '{}' is also one of the inputs", a.output.display());
    }
    let mut project = if a.append && a.output.exists() { load_project(&a.output)? } else { Project::new() };
    for file in &a.files {
        if is_project(file) {
            // An Origin project (or another Sheaf project): take over all its workbooks.
            let mut count = 0;
            for window in load_project(file)?.windows {
                if let Window::Workbook(book) = window {
                    project.adopt_named_workbook(book);
                    count += 1;
                }
            }
            println!("imported {count} workbook(s) from {}", file.display());
            continue;
        }
        let imported = read_data_file(file, &a.read)?;
        let template = imported.suggested_template;
        let book = project.adopt_workbook(imported.workbook);
        if a.plot {
            let sheet = project.workbook(book).map(|b| b.active_sheet).unwrap_or(0);
            let ys = project.default_plot_columns(book, sheet, template);
            if !ys.is_empty() {
                project.plot_columns(template, &ys)?;
            }
        }
        println!("imported {} as {}", file.display(), project.workbook(book).map(|b| b.name.as_str()).unwrap_or("?"));
    }
    save_project(&project, &a.output)?;
    Ok(())
}

/// Saves as a `.sheaf` project. Refuses to write over an Origin project file.
pub(crate) fn save_project(project: &Project, path: &Path) -> Result<()> {
    if path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("opj")) {
        bail!("'{}' is an Origin project name; save as a .sheaf file instead", path.display());
    }
    project_file::save(project, path).with_context(|| format!("could not save '{}'", path.display()))?;
    println!("saved {}", path.display());
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
/// The options act on the first layer and the layers linked to it (its right Y axis): `--x-*`
/// and `--y-*` set the first layer, whose X axis linked layers follow; `--y2-*` set the right Y
/// axis; colors, labels and line widths count the plots in legend order, first layer first.
/// Further layers of a graph read from an Origin project are left alone.
fn apply_style(project: &mut Project, graph: WindowId, s: &StyleArgs) -> Result<()> {
    require_page_size("--width", s.width)?;
    require_page_size("--height", s.height)?;
    require_positive("--line-width", s.line_width)?;
    check_limits("x", s.x_min, s.x_max)?;
    check_limits("y", s.y_min, s.y_max)?;
    check_limits("y2", s.y2_min, s.y2_max)?;
    let colors: Vec<Color> =
        s.colors.iter().map(|c| c.parse::<Color>().map_err(|e| anyhow!("{e}"))).collect::<Result<_>>()?;
    let stack = stack_option(s)?;
    let g = project.graph_mut(graph).ok_or_else(|| anyhow!("graph not found"))?;
    check_stack_fits(g, s)?;
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
    let group = g.group(0);
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
        for &i in &group {
            g.layers[i].y_axis.color_follows_plot = mode == AxisColors::Plot;
        }
    }
    style_plots(g, &group, s, &colors);
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
    if let Some(stack) = stack {
        layer.stack = stack;
    }
    if !s.offsets.is_empty() {
        layer.stack = Stack::Individual;
        let columns: Vec<ColumnId> = layer.plots.iter().map(|p| p.y).collect();
        for (column, y) in columns.into_iter().zip(&s.offsets) {
            layer.plot_offset_mut(column).y = *y;
        }
    }
    if s.no_y_tick_labels {
        layer.y_axis.show_tick_labels = false;
    }
    if s.no_y_ticks {
        layer.y_axis.major_tick = TickDirection::None;
        layer.y_axis.minor_tick = TickDirection::None;
    }
    Ok(())
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

/// Refuses stack options that do not fit the graph: more `--offsets` than the first layer has
/// curves, and a waterfall in a graph with a right Y axis (a waterfall is drawn only for a layer
/// on its own).
fn check_stack_fits(g: &Graph, s: &StyleArgs) -> Result<()> {
    let plots = g.layers.first().map_or(0, |l| l.plots.len());
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
                plot.line.color = *c;
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
    apply_style(&mut project, graph, &a.style)?;

    sheaf_plot::export_file(&project, graph, &a.output, a.dpi)
        .with_context(|| format!("could not write '{}'", a.output.display()))?;
    println!("wrote {}", a.output.display());
    if let Some(path) = &a.save {
        save_project(&project, path)?;
    }
    Ok(())
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

fn export(a: ExportArgs) -> Result<()> {
    require_positive("--dpi", Some(a.dpi))?;
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
