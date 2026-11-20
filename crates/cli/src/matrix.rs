//! `sheaf matrix`: make matrices, change them, convert between worksheets and matrices, and plot
//! heat maps and contours without a window.

use crate::commands::{is_project, load_project, pick_book, pick_sheet, read_data_file, read_settings, save_project};
use anyhow::{Context, Result, anyhow, bail};
use sheaf_core::graph::{FieldPalette, FieldPlot};
use sheaf_core::matrix::{FlipDirection, Rotation};
use sheaf_core::matrix_convert::{ColumnParam, LabelSource, M2wMethod, W2mOptions, XyLayout, XyzOrder};
use sheaf_core::matrix_project::MatrixOp;
use sheaf_core::template::FieldTemplate;
use sheaf_core::{Designation, Project, Window, WindowId};
use sheaf_ops::args::*;
use sheaf_ops::pick::named_columns;
use std::path::Path;

pub fn matrix(a: MatrixArgs) -> Result<()> {
    match a.action {
        MatrixAction::New(a) => new(a),
        MatrixAction::Convert(a) => convert(*a),
        MatrixAction::Set(a) => set(a),
        MatrixAction::Transform(a) => transform(a),
        MatrixAction::ToSheet(a) => to_sheet(a),
        MatrixAction::Stats(a) => stats(a),
        MatrixAction::Plot(a) => plot(*a),
    }
}

/// A project, or a data file imported as a project with one workbook.
fn load_any(path: &Path, read: &ReadOptions) -> Result<Project> {
    if is_project(path) {
        return load_project(path);
    }
    let mut p = Project::new();
    p.adopt_workbook(read_data_file(path, &read_settings(read)?)?.workbook);
    Ok(p)
}

fn pick_matrix(project: &Project, name: Option<&str>) -> Result<WindowId> {
    match name {
        Some(n) => match project.window_by_name(n) {
            Some(Window::Matrix(m)) => Ok(m.id),
            Some(_) => bail!("'{n}' is not a matrix window"),
            None => {
                let have: Vec<&str> = project.matrix_books().map(|m| m.name.as_str()).collect();
                bail!(
                    "there is no matrix named '{n}' (matrices: {})",
                    if have.is_empty() { "none".into() } else { have.join(", ") }
                )
            }
        },
        None => project.matrix_books().next().map(|m| m.id).ok_or_else(|| anyhow!("the project has no matrix window")),
    }
}

fn pick_msheet(project: &Project, book: WindowId, name: Option<&str>) -> Result<usize> {
    let b = project.matrix_book(book).ok_or_else(|| anyhow!("matrix not found"))?;
    match name {
        Some(n) => b
            .sheets
            .iter()
            .position(|s| s.name.eq_ignore_ascii_case(n))
            .ok_or_else(|| anyhow!("matrix '{}' has no sheet named '{}'", b.name, n)),
        None => Ok(b.active_sheet.min(b.sheets.len().saturating_sub(1))),
    }
}

fn range_pair(text: &str, what: &str) -> Result<(f64, f64)> {
    sheaf_ops::analyze::parse_pair::<f64>(text, what, "0:10")
}

fn describe_sheet(project: &Project, id: WindowId) {
    if let Some(b) = project.matrix_book(id) {
        for s in &b.sheets {
            println!(
                "{}: sheet {} ({} columns x {} rows, x from {} to {}, y from {} to {})",
                b.name, s.name, s.cols, s.rows, s.x_from, s.x_to, s.y_from, s.y_to
            );
        }
    }
}

fn new(a: NewArgs) -> Result<()> {
    let mut project =
        if a.output.exists() && is_project(&a.output) { load_project(&a.output)? } else { Project::new() };
    let id = project.add_matrix_book(a.long_name.as_deref().unwrap_or(""));
    let mut ops = vec![MatrixOp::SetDimensions {
        rows: a.rows,
        cols: a.cols,
        ranges: match (&a.x_range, &a.y_range) {
            (None, None) => None,
            (x, y) => {
                let (x1, x2) = x.as_deref().map(|t| range_pair(t, "X range")).transpose()?.unwrap_or((1.0, 10.0));
                let (y1, y2) = y.as_deref().map(|t| range_pair(t, "Y range")).transpose()?.unwrap_or((1.0, 10.0));
                Some((x1, x2, y1, y2))
            }
        },
    }];
    if let Some(f) = &a.formula {
        ops.push(MatrixOp::SetValues(f.clone()));
    }
    for op in &ops {
        project.apply_matrix_op(id, 0, op)?;
    }
    describe_sheet(&project, id);
    save_project(&project, &a.output)
}

fn w2m_options(a: &ConvertArgs) -> Result<W2mOptions> {
    let layout = match a.layout {
        LayoutArg::None => XyLayout::None,
        LayoutArg::XAcross => XyLayout::XAcrossColumns,
        LayoutArg::YAcross => XyLayout::YAcrossColumns,
    };
    let labels = match a.labels.to_ascii_lowercase().as_str() {
        "none" => LabelSource::None,
        "row1" | "first-row" => LabelSource::FirstRow,
        other => {
            let param = other.strip_prefix("param:").map(str::trim).filter(|p| !p.is_empty()).ok_or_else(|| {
                anyhow!("--labels must be none, row1, or param:L (long name), param:U (units), param:C (comments) or param:NAME (a parameter row), not '{}'", a.labels)
            })?;
            LabelSource::Parameter(match param.to_ascii_uppercase().as_str() {
                "L" => ColumnParam::LongName,
                "U" => ColumnParam::Units,
                "C" => ColumnParam::Comments,
                _ => ColumnParam::User(a.labels[a.labels.find(':').map_or(0, |i| i + 1)..].trim().to_string()),
            })
        }
    };
    Ok(W2mOptions {
        layout,
        labels,
        coords_in_first_column: a.coords_in_first_column,
        tolerance_percent: a.tolerance,
        linear_fit: a.linear_fit,
        trim: !a.no_trim,
    })
}

fn convert(a: ConvertArgs) -> Result<()> {
    let mut project = load_any(&a.input, &a.read)?;
    let book = pick_book(&project, a.book.as_deref())?;
    let sheet = pick_sheet(&project, book, a.sheet.as_deref())?;
    let id = if a.xyz {
        let cols = if a.columns.len() == 3 {
            named_columns(&project, book, sheet, &a.columns)?
        } else if a.columns.is_empty() {
            let ws = &project.workbook(book).expect("picked above").sheets[sheet];
            let find = |d: Designation| ws.columns.iter().find(|c| c.designation == d).map(|c| c.id);
            match (find(Designation::X), find(Designation::Y), find(Designation::Z)) {
                (Some(x), Some(y), Some(z)) => vec![x, y, z],
                _ if ws.columns.len() >= 3 => ws.columns.iter().take(3).map(|c| c.id).collect(),
                _ => bail!("the sheet needs three columns (X, Y, Z); name them with --columns X,Y,Z"),
            }
        } else {
            bail!("--xyz needs exactly three columns: --columns X,Y,Z");
        };
        project.xyz_to_matrix(cols[0], cols[1], cols[2])?
    } else {
        let ids = if a.columns.is_empty() { None } else { Some(named_columns(&project, book, sheet, &a.columns)?) };
        project.worksheet_to_matrix(book, sheet, ids.as_deref(), &w2m_options(&a)?)?
    };
    describe_sheet(&project, id);
    save_project(&project, &a.output)
}

fn matrix_target(project: &Project, t: &MatrixTarget) -> Result<(WindowId, usize)> {
    let book = pick_matrix(project, t.matrix.as_deref())?;
    Ok((book, pick_msheet(project, book, t.sheet.as_deref())?))
}

fn set(a: SetArgs) -> Result<()> {
    let mut project = load_project(&a.project)?;
    let (book, sheet) = matrix_target(&project, &a.target)?;
    if a.cols.is_some() || a.rows.is_some() || a.x_range.is_some() || a.y_range.is_some() {
        let s = project.matrix_sheet(book, sheet)?;
        let (x1, x2) =
            a.x_range.as_deref().map(|t| range_pair(t, "X range")).transpose()?.unwrap_or((s.x_from, s.x_to));
        let (y1, y2) =
            a.y_range.as_deref().map(|t| range_pair(t, "Y range")).transpose()?.unwrap_or((s.y_from, s.y_to));
        let op = MatrixOp::SetDimensions {
            rows: a.rows.unwrap_or(s.rows),
            cols: a.cols.unwrap_or(s.cols),
            ranges: Some((x1, x2, y1, y2)),
        };
        project.apply_matrix_op(book, sheet, &op)?;
    }
    if let Some(f) = &a.formula {
        project.apply_matrix_op(book, sheet, &MatrixOp::SetValues(f.clone()))?;
    }
    describe_sheet(&project, book);
    save_project(&project, a.output.as_deref().unwrap_or(&a.project))
}

fn transform(a: TransformArgs) -> Result<()> {
    let mut project = load_project(&a.project)?;
    let (book, sheet) = matrix_target(&project, &a.target)?;
    let op = match a.op {
        TransformKind::Transpose => MatrixOp::Transpose,
        TransformKind::FlipHorizontal => {
            MatrixOp::Flip { direction: FlipDirection::Horizontal, coordinates: a.coordinates }
        }
        TransformKind::FlipVertical => {
            MatrixOp::Flip { direction: FlipDirection::Vertical, coordinates: a.coordinates }
        }
        TransformKind::RotateCw => MatrixOp::Rotate(Rotation::Cw90),
        TransformKind::RotateCcw => MatrixOp::Rotate(Rotation::Ccw90),
        TransformKind::Rotate180 => MatrixOp::Rotate(Rotation::Half),
        TransformKind::Expand => MatrixOp::Expand { cols: a.cols, rows: a.rows },
        TransformKind::Shrink => MatrixOp::Shrink { cols: a.cols, rows: a.rows },
    };
    project.apply_matrix_op(book, sheet, &op)?;
    describe_sheet(&project, book);
    save_project(&project, a.output.as_deref().unwrap_or(&a.project))
}

fn to_sheet(a: ToSheetArgs) -> Result<()> {
    let mut project = load_project(&a.project)?;
    let (book, sheet) = matrix_target(&project, &a.target)?;
    let method = match a.method {
        ToSheetMethod::Direct => M2wMethod::Direct {
            layout: match a.layout {
                LayoutArg::None => XyLayout::None,
                LayoutArg::XAcross => XyLayout::XAcrossColumns,
                LayoutArg::YAcross => XyLayout::YAcrossColumns,
            },
            labels: if a.layout == LayoutArg::None { LabelSource::None } else { LabelSource::FirstRow },
            coords_in_first_column: a.layout != LayoutArg::None,
        },
        ToSheetMethod::Xyz => M2wMethod::Xyz {
            order: if a.y_const_first { XyzOrder::YConstFirst } else { XyzOrder::XConstFirst },
            skip_missing: a.skip_missing,
        },
    };
    let id = project.matrix_to_worksheet(book, sheet, &method)?;
    if let Some(path) = &a.export {
        let wb = project.workbook(id).expect("just added");
        crate::commands::write_sheet(&wb.sheets[0], path)?;
    }
    println!("added workbook {}", project.window(id).map(|w| w.name().to_string()).unwrap_or_default());
    if a.export.is_none() || a.output.is_some() {
        save_project(&project, a.output.as_deref().unwrap_or(&a.project))?;
    }
    Ok(())
}

fn stats(a: StatsArgs) -> Result<()> {
    let project = load_project(&a.project)?;
    let (book, sheet) = matrix_target(&project, &a.target)?;
    let s = project.matrix_stats(book, sheet)?;
    let f = crate::analyze::short_number;
    println!("n        {}", s.n);
    println!("missing  {}", s.missing);
    println!("mean     {}", f(s.mean));
    println!("sd       {}", f(s.sd));
    println!("min      {}", f(s.min));
    println!("max      {}", f(s.max));
    println!("sum      {}", f(s.sum));
    Ok(())
}

fn plot(a: PlotMatrixArgs) -> Result<()> {
    if !(a.dpi.is_finite() && a.dpi > 0.0) {
        bail!("--dpi must be a positive number");
    }
    let mut project = load_any(&a.input, &a.read)?;
    let template = FieldTemplate::from_name(&a.template).ok_or_else(|| {
        let names: Vec<&str> = FieldTemplate::ALL.iter().map(FieldTemplate::name).collect();
        anyhow!("unknown template '{}'; the templates are {}", a.template, names.join(", "))
    })?;
    let use_columns = !a.xyz.is_empty() || project.matrix_books().next().is_none();
    let graph = if use_columns {
        let book = pick_book(&project, a.book.as_deref())?;
        let sheet = pick_sheet(&project, book, a.sheet.as_deref())?;
        let ids = if a.xyz.len() == 3 {
            named_columns(&project, book, sheet, &a.xyz)?
        } else if a.xyz.is_empty() {
            let ws = &project.workbook(book).expect("picked above").sheets[sheet];
            let find = |d: Designation| ws.columns.iter().find(|c| c.designation == d).map(|c| c.id);
            match (find(Designation::X), find(Designation::Y), find(Designation::Z)) {
                (Some(x), Some(y), Some(z)) => vec![x, y, z],
                _ if ws.columns.len() >= 3 => ws.columns.iter().take(3).map(|c| c.id).collect(),
                _ => bail!(
                    "the project has no matrix and the sheet has no three columns for X, Y and Z; name them with --xyz X,Y,Z"
                ),
            }
        } else {
            bail!("--xyz needs exactly three columns: --xyz X,Y,Z");
        };
        project.plot_xyz(template, ids[0], ids[1], ids[2])?
    } else {
        let book = pick_matrix(&project, a.matrix.as_deref())?;
        let sheet = pick_msheet(&project, book, a.sheet.as_deref())?;
        project.plot_matrix(template, book, sheet)?
    };
    {
        let g = project.graph_mut(graph).expect("just added");
        if let Some(w) = a.width {
            g.page.width_pt = w * 72.0;
        }
        if let Some(h) = a.height {
            g.page.height_pt = h * 72.0;
        }
        let field: &mut FieldPlot = g.layers[0].field.as_mut().expect("a field template makes a field");
        if let Some(n) = a.major {
            field.levels.major = n.max(1);
        }
        if let Some(n) = a.minor {
            field.levels.minor = n.max(1);
        }
        if a.z_min.is_some() {
            field.levels.min = a.z_min;
        }
        if a.z_max.is_some() {
            field.levels.max = a.z_max;
        }
        if a.no_lines {
            field.lines = false;
        }
        if a.lines {
            field.lines = true;
        }
        if a.labels {
            field.labels = true;
        }
        if a.no_scale {
            field.scale.visible = false;
        }
        if let Some(t) = &a.scale_title {
            field.scale.title = Some(t.clone());
        }
        if let Some(p) = a.palette {
            field.palette = match p {
                PaletteArg::Rainbow => FieldPalette::Rainbow,
                PaletteArg::BlueWhiteRed => FieldPalette::BlueWhiteRed,
                PaletteArg::Gray => FieldPalette::Gray,
            };
        }
        if let Some(t) = &a.x_title {
            g.layers[0].x_axis.title = Some(t.clone());
        }
        if let Some(t) = &a.y_title {
            g.layers[0].y_axis.title = Some(t.clone());
        }
    }
    sheaf_plot::export_file(&project, graph, &a.output, a.dpi)
        .with_context(|| format!("could not write '{}'", a.output.display()))?;
    println!("wrote {}", a.output.display());
    if let Some(path) = &a.save {
        save_project(&project, path)?;
    }
    Ok(())
}
