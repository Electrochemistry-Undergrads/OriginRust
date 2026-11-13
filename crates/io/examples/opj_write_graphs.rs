//! Writes sample projects with graphs as Origin project files, for checking them in Origin itself
//! with `tools/origin-ref/opj-verify-graphs.ps1` (and for comparing the pictures with `sheaf export`).
//!
//! Usage: `cargo run -p sheaf-io --example opj_write_graphs -- <output folder>`
//!
//! - `g-basic.opj`: a line plot with two curves, a scatter plot on logarithmic axes, a line and
//!   symbol plot with a moved frame, and a column plot with error bars, a text label and a legend.
//! - `g-kinds.opj`: every symbol shape, line style and connection, line widths and colours.
//! - `g-axes.opj`: tick directions, tick label sizes, the closed box, grids, hidden labels.
//! - `g-layers.opj`: a graph with two layers.
//! - `g-scales.opj`: one graph for each scale type beyond linear and log10, with automatic limits.
//! - `g-cv.opj`, `g-it.opj`, `g-columns.opj`: figures in the style of the owner's usual ones (a
//!   cyclic voltammogram overlay, a current against time curve, and a column chart with station
//!   names, error bars and text labels), built from data made up here.

use sheaf_core::Project;
use sheaf_core::color::Color;
use sheaf_core::column::{Column, ColumnData, Designation};
use sheaf_core::graph::*;
use sheaf_core::ids::{ColumnId, WindowId};
use sheaf_core::template::Template;
use sheaf_core::worksheet::{Workbook, Worksheet};
use std::path::Path;

fn col(short: &str, d: Designation, data: ColumnData, long: &str, units: &str) -> Column {
    Column::new(ColumnId(0), short, d, data).with_labels(long, units, "")
}

fn nums(v: Vec<f64>) -> ColumnData {
    ColumnData::numeric(v)
}

fn source() -> Project {
    let x: Vec<f64> = (0..21).map(|i| f64::from(i) * 0.5).collect();
    let mut sheet = Worksheet::new("Sheet1");
    sheet.columns = vec![
        col("A", Designation::X, nums(x.clone()), "Time", "s"),
        col("B", Designation::Y, nums(x.iter().map(|x| x * x).collect()), "Squares", "mA"),
        col("C", Designation::Y, nums(x.iter().map(|x| 100.0 - 5.0 * x).collect()), "Line", "mA"),
        col("D", Designation::Y, nums(x.iter().map(|x| 10f64.powf(x / 5.0)).collect()), "Power", ""),
        col("E", Designation::Y, nums(x.iter().map(|x| x.sin() * 5.0).collect()), "Wave", "V"),
        col("F", Designation::YError, nums(x.iter().map(|x| 2.0 + x).collect()), "Error", "mA"),
    ];
    let mut book = Workbook::new(WindowId(0), "GSrc");
    book.sheets = vec![sheet];
    let mut p = Project::new();
    p.adopt_named_workbook(book);
    p
}

fn ids(p: &Project) -> Vec<ColumnId> {
    p.workbooks().next().unwrap().sheets[0].columns.iter().map(|c| c.id).collect()
}

fn add(p: &mut Project, template: Template, name: &str, kind: PlotKind, ys: &[(usize, Option<usize>)]) -> WindowId {
    let c = ids(p);
    let id = p.add_graph(template);
    for (y, err) in ys {
        p.add_plot_xy(id, 0, Some(c[0]), c[*y], err.map(|e| c[e])).unwrap();
    }
    let g = p.graph_mut(id).unwrap();
    g.name = name.to_string();
    g.layers[0].default_kind = kind;
    for plot in &mut g.layers[0].plots {
        plot.kind = kind;
    }
    id
}

/// One graph per scale type other than linear and log10, with automatic limits.
fn scales() -> Project {
    let mut p = source();
    for (i, kind) in ScaleKind::ALL.into_iter().enumerate().skip(2) {
        let column = if kind.is_percent() { 2 } else { 3 };
        let id = add(&mut p, Template::Scatter, &format!("GScale{i}"), PlotKind::Scatter, &[(column, None)]);
        p.graph_mut(id).unwrap().layers[0].y_axis.scale = kind;
    }
    p
}

fn basic() -> Project {
    let mut p = source();
    let a = add(&mut p, Template::Line, "GLine", PlotKind::Line, &[(1, None), (2, None)]);
    {
        let l = &mut p.graph_mut(a).unwrap().layers[0];
        l.x_axis.min = Some(0.0);
        l.x_axis.max = Some(10.0);
        l.x_axis.major_step = Some(2.0);
        l.y_axis.min = Some(0.0);
        l.y_axis.max = Some(100.0);
        l.y_axis.major_step = Some(20.0);
        l.x_axis.title = Some("Time (s)".into());
        l.y_axis.title = Some("Current (mA)".into());
        l.plots[0].line.color = Color::rgb(255, 0, 0);
        l.plots[0].line.width_pt = 2.0;
        l.plots[1].line.color = Color::rgb(0, 0, 255);
        l.plots[1].line.width_pt = 3.0;
        l.plots[1].line.dash = Dash::Dash;
    }
    let b = add(&mut p, Template::Scatter, "GScat", PlotKind::Scatter, &[(3, None)]);
    {
        let l = &mut p.graph_mut(b).unwrap().layers[0];
        l.x_axis.scale = ScaleKind::Log10;
        l.x_axis.min = Some(1.0);
        l.x_axis.max = Some(10.0);
        l.y_axis.scale = ScaleKind::Log10;
        l.y_axis.min = Some(1.0);
        l.y_axis.max = Some(100.0);
        l.plots[0].symbol.size_pt = 12.0;
        l.plots[0].symbol.color = Color::rgb(0, 0, 255);
        l.plots[0].symbol.shape = SymbolShape::Square;
    }
    let c = add(&mut p, Template::LineSymbol, "GLS", PlotKind::LineSymbol, &[(4, None)]);
    {
        let l = &mut p.graph_mut(c).unwrap().layers[0];
        l.frame = Frame { left: 0.2, top: 0.15, width: 0.6, height: 0.55 };
        l.x_axis.min = Some(2.0);
        l.x_axis.max = Some(8.0);
        l.x_axis.major_step = Some(2.0);
        l.y_axis.min = Some(-6.0);
        l.y_axis.max = Some(6.0);
        l.y_axis.major_step = Some(3.0);
        l.plots[0].line.width_pt = 2.0;
        l.plots[0].line.color = Color::rgb(0, 128, 0);
        l.plots[0].symbol.color = Color::rgb(255, 128, 0);
        l.plots[0].symbol.size_pt = 9.0;
        l.plots[0].symbol.shape = SymbolShape::Circle;
    }
    let d = add(&mut p, Template::Line, "GCol", PlotKind::Column, &[(1, Some(5))]);
    {
        let l = &mut p.graph_mut(d).unwrap().layers[0];
        l.y_axis.min = Some(0.0);
        l.y_axis.max = Some(120.0);
        l.y_axis.major_step = Some(30.0);
        l.plots[0].symbol.color = Color::rgb(0, 107, 172);
        l.plots[0].label = Some("Squares".into());
        l.labels.push(TextLabel {
            text: "peak A".into(),
            x: 0.3,
            y: 0.2,
            font: Font::new("Arial", 18.0),
            rotation_deg: 0.0,
        });
        l.legend.visible = true;
        l.legend.frame = true;
        l.legend.position = LegendPosition::TopLeft;
    }
    p
}

fn kinds() -> Project {
    let mut p = source();
    let c = ids(&p);
    let shapes = SymbolShape::ALL;
    let g = p.add_graph(Template::Scatter);
    for (i, s) in shapes.iter().enumerate() {
        // one plot per shape, each on its own offset (by plotting the same column)
        p.add_plot_xy(g, 0, Some(c[0]), c[1 + i % 4], None).unwrap();
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        let pl = &mut l.plots[i];
        pl.symbol.shape = *s;
        pl.symbol.size_pt = 7.0;
        pl.symbol.color = Color::rgb(30 * i as u8, 255 - 30 * i as u8, 128);
        pl.kind = PlotKind::Scatter;
    }
    p.graph_mut(g).unwrap().name = "GShapes".into();
    let g = p.add_graph(Template::Line);
    for (i, d) in Dash::ALL.iter().enumerate() {
        p.add_plot_xy(g, 0, Some(c[0]), c[2], None).unwrap();
        let pl = &mut p.graph_mut(g).unwrap().layers[0].plots[i];
        pl.line.dash = *d;
        pl.line.width_pt = 1.0 + i as f64 * 0.5;
    }
    p.graph_mut(g).unwrap().name = "GDash".into();
    // ten copies of a coarse wave, one above the other, one line connection each
    let book = p.workbooks().next().unwrap().id;
    let coarse: Vec<f64> = (0..8).map(|i| ((i * 7) % 5) as f64 + f64::from(i % 2) * 2.0).collect();
    let mut shifted = Vec::new();
    for k in 0..10 {
        let y: Vec<f64> = coarse.iter().map(|v| v + 8.0 * f64::from(k)).collect();
        shifted.push(p.add_column(book, 0, Designation::Y, nums(y)).unwrap());
    }
    let g = p.add_graph(Template::Line);
    let conns = [
        Connect::Straight,
        Connect::Spline,
        Connect::StepHorizontal,
        Connect::StepVertical,
        Connect::Straight,
        Connect::Straight,
        Connect::Straight,
        Connect::Straight,
        Connect::Straight,
        Connect::Straight,
    ];
    for (i, k) in conns.iter().enumerate() {
        p.add_plot_xy(g, 0, Some(c[0]), shifted[i], None).unwrap();
        let pl = &mut p.graph_mut(g).unwrap().layers[0].plots[i];
        pl.line.connect = *k;
        pl.line.width_pt = 2.0;
        pl.line.color = Color::rgb(25 * i as u8, 0, 255 - 25 * i as u8);
    }
    p.graph_mut(g).unwrap().layers[0].legend.visible = false;
    p.graph_mut(g).unwrap().name = "GConn".into();
    let g = p.add_graph(Template::LineSymbol);
    p.add_plot_xy(g, 0, Some(c[0]), c[1], None).unwrap();
    p.add_plot_xy(g, 0, Some(c[0]), c[2], None).unwrap();
    {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        l.plots[0].symbol.filled = false;
        l.plots[0].symbol.shape = SymbolShape::Diamond;
        l.plots[0].symbol.size_pt = 10.0;
        l.plots[1].symbol.filled = true;
        l.plots[1].symbol.shape = SymbolShape::TriangleUp;
        l.plots[1].symbol.size_pt = 10.0;
    }
    p.graph_mut(g).unwrap().name = "GOpen".into();
    p
}

fn axes() -> Project {
    let mut p = source();
    let g = add(&mut p, Template::CyclicVoltammetry, "GBox", PlotKind::Line, &[(1, None), (2, None)]);
    {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        l.x_axis.tick_font.size_pt = 20.0;
        l.y_axis.tick_font.size_pt = 14.0;
        l.x_axis.line_width_pt = 2.0;
        l.y_axis.major_tick = TickDirection::Both;
        l.y_axis.minor_tick = TickDirection::In;
    }
    let g = add(&mut p, Template::Line, "GGrid", PlotKind::Line, &[(4, None)]);
    {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        l.x_axis.major_grid = Some(LineStyle::solid(Color::rgb(192, 192, 192), 0.5));
        l.y_axis.minor_grid = Some(LineStyle::solid(Color::rgb(192, 192, 192), 0.5));
        l.y_axis.show_tick_labels = false;
        l.x_axis.minor_ticks = 4;
        l.x_axis.major_step = Some(2.5);
        l.x_axis.min = Some(0.0);
        l.x_axis.max = Some(10.0);
    }
    p
}

fn layers() -> Project {
    let mut p = source();
    let g = add(&mut p, Template::Line, "GTwo", PlotKind::Line, &[(1, None)]);
    let c = ids(&p);
    let mut second = p.graph(g).unwrap().layers[0].clone();
    second.plots.clear();
    second.frame = Frame { left: 0.2, top: 0.5, width: 0.4, height: 0.3 };
    second.legend.visible = false;
    p.graph_mut(g).unwrap().layers.push(second);
    p.add_plot_xy(g, 1, Some(c[0]), c[2], None).unwrap();
    p.graph_mut(g).unwrap().layers[0].frame = Frame { left: 0.15, top: 0.08, width: 0.75, height: 0.35 };
    p
}

fn cv() -> Project {
    // three cyclic voltammograms of a made-up redox couple
    let n = 400;
    let mut sheet = Worksheet::new("Sheet1");
    let e: Vec<f64> = (0..n)
        .map(|i| {
            let t = i as f64 / (n - 1) as f64;
            if t < 0.5 { -0.2 + 1.2 * 2.0 * t } else { 1.0 - 1.2 * 2.0 * (t - 0.5) }
        })
        .collect();
    sheet.columns.push(col("A", Designation::X, nums(e.clone()), "Potential", "V"));
    for (k, scale) in [1.0, 2.0, 4.0].iter().enumerate() {
        let i: Vec<f64> = e
            .iter()
            .enumerate()
            .map(|(j, v)| {
                let up = j < n / 2;
                let peak = if up { 0.6 } else { 0.5 };
                scale * 1e-4 * (if up { 1.0 } else { -1.0 }) * (-((v - peak) / 0.08).powi(2)).exp() + scale * 1e-5 * v
            })
            .collect();
        sheet.columns.push(col(["B", "C", "D"][k], Designation::Y, nums(i), &format!("{} mV/s", 10 * (k + 1)), "A"));
    }
    let mut book = Workbook::new(WindowId(0), "CV");
    book.sheets = vec![sheet];
    let mut p = Project::new();
    p.adopt_named_workbook(book);
    let c = ids(&p);
    let g = p.add_graph(Template::CyclicVoltammetry);
    for y in &c[1..] {
        p.add_plot_xy(g, 0, Some(c[0]), *y, None).unwrap();
    }
    let gr = p.graph_mut(g).unwrap();
    gr.name = "GCV".into();
    gr.layers[0].x_axis.title = Some("Potential (V vs. Ag/AgCl)".into());
    gr.layers[0].y_axis.title = Some("Current (A)".into());
    p
}

/// The cyclic voltammogram with its legend in other places and its lines straight, to see what
/// Origin does with the legend icons.
fn legends() -> Project {
    let mut p = cv();
    let c = ids(&p);
    for (name, pos, connect) in [
        ("LegTR", LegendPosition::TopRight, Connect::Spline),
        ("LegBRStraight", LegendPosition::BottomRight, Connect::Straight),
        ("LegTRStraight", LegendPosition::TopRight, Connect::Straight),
    ] {
        let g = p.add_graph(Template::CyclicVoltammetry);
        for y in &c[1..] {
            p.add_plot_xy(g, 0, Some(c[0]), *y, None).unwrap();
        }
        let gr = p.graph_mut(g).unwrap();
        gr.name = name.into();
        gr.layers[0].legend.position = pos;
        for pl in &mut gr.layers[0].plots {
            pl.line.connect = connect;
        }
    }
    p
}

fn it() -> Project {
    let n = 300;
    let t: Vec<f64> = (0..n).map(|i| i as f64 * 0.2).collect();
    let i: Vec<f64> = t.iter().map(|t| 2e-5 * (-t / 15.0).exp() + 1e-6).collect();
    let mut sheet = Worksheet::new("Sheet1");
    sheet.columns =
        vec![col("A", Designation::X, nums(t), "Time", "s"), col("B", Designation::Y, nums(i), "Current", "A")];
    let mut book = Workbook::new(WindowId(0), "IT");
    book.sheets = vec![sheet];
    let mut p = Project::new();
    p.adopt_named_workbook(book);
    let c = ids(&p);
    let g = p.add_graph(Template::Line);
    p.add_plot_xy(g, 0, Some(c[0]), c[1], None).unwrap();
    let gr = p.graph_mut(g).unwrap();
    gr.name = "GIT".into();
    gr.layers[0].x_axis.title = Some("Time (s)".into());
    gr.layers[0].y_axis.title = Some("Current (A)".into());
    gr.layers[0].plots[0].line.color = Color::rgb(0, 0, 255);
    gr.layers[0].plots[0].line.width_pt = 2.0;
    gr.layers[0].legend.visible = false;
    p
}

fn columns() -> Project {
    let stations = ["North", "East", "South", "West", "Pond", "Creek"];
    let mut sheet = Worksheet::new("Sheet1");
    sheet.columns = vec![
        col("A", Designation::X, ColumnData::text(stations.iter().map(|s| s.to_string()).collect()), "Station", ""),
        col("B", Designation::Y, nums(vec![12.0, 18.5, 9.0, 21.0, 15.5, 7.5]), "Nitrate", "ppm"),
        col("C", Designation::YError, nums(vec![1.5, 2.0, 1.0, 2.5, 1.8, 0.9]), "Error", "ppm"),
    ];
    let mut book = Workbook::new(WindowId(0), "Water");
    book.sheets = vec![sheet];
    let mut p = Project::new();
    p.adopt_named_workbook(book);
    let c = ids(&p);
    let g = p.add_graph(Template::Line);
    p.add_plot_xy(g, 0, Some(c[0]), c[1], Some(c[2])).unwrap();
    let gr = p.graph_mut(g).unwrap();
    gr.name = "GColumns".into();
    let l = &mut gr.layers[0];
    l.plots[0].kind = PlotKind::Column;
    l.plots[0].symbol.color = Color::rgb(0, 107, 172);
    l.x_axis.title = Some("\\b(Station)".into());
    l.y_axis.title = Some("\\b(Nitrate (ppm))".into());
    l.y_axis.min = Some(0.0);
    l.y_axis.max = Some(30.0);
    l.legend.visible = false;
    for (i, s) in ["a", "b", "a", "c", "b", "d"].iter().enumerate() {
        l.labels.push(TextLabel {
            text: (*s).to_string(),
            x: 0.07 + 0.15 * i as f64,
            y: 0.1 + 0.05 * (i % 3) as f64,
            font: Font::new("Arial", 20.0),
            rotation_deg: 0.0,
        });
    }
    p
}

fn main() {
    let Some(dir) = std::env::args().nth(1) else {
        eprintln!("usage: opj_write_graphs <output folder>");
        std::process::exit(2);
    };
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir).expect("the output folder can be made");
    for (name, project) in [
        ("g-basic", basic()),
        ("g-kinds", kinds()),
        ("g-axes", axes()),
        ("g-layers", layers()),
        ("g-cv", cv()),
        ("g-legend", legends()),
        ("g-it", it()),
        ("g-columns", columns()),
        ("g-scales", scales()),
    ] {
        let path = dir.join(format!("{name}.opj"));
        match sheaf_io::opj::write_file(&project, &path) {
            Ok(warnings) => {
                println!("wrote {}", path.display());
                for w in warnings {
                    println!("  warning: {w}");
                }
            }
            Err(e) => println!("{name}: {e}"),
        }
    }
}
