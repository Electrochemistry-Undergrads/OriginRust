//! Writes sample projects for the graph features that are stored in the .opj file itself (stacked
//! curve offsets, the right Y axis of a double-Y graph, bar spacing, axis sides and axis colours),
//! for checking them in Origin with `tools/origin-ref/opj-verify-graph-features.ps1` and for
//! comparing the pictures with `sheaf export`.
//!
//! Usage: `cargo run -p sheaf-io --example opj_write_graph_features -- <output folder>`
//!
//! - `gf-doubley.opj`: a double-Y graph (one curve on each side) and one with two curves per side.
//! - `gf-stack.opj`: stacked lines in the Auto, Constant and Individual modes.
//! - `gf-bars.opj`: column plots with gaps of 0, 20 and 60 percent and an overlap.
//! - `gf-axes.opj`: a top X axis, a right Y axis, hidden axes and axes in the colour of the plot.

use sheaf_core::Project;
use sheaf_core::column::{Column, ColumnData, Designation};
use sheaf_core::graph::*;
use sheaf_core::ids::{ColumnId, WindowId};
use sheaf_core::template::Template;
use sheaf_core::worksheet::{Workbook, Worksheet};
use std::path::Path;

fn col(short: &str, d: Designation, data: Vec<f64>, long: &str, units: &str) -> Column {
    Column::new(ColumnId(0), short, d, ColumnData::numeric(data)).with_labels(long, units, "")
}

fn source() -> Project {
    let x: Vec<f64> = (0..21).map(|i| f64::from(i) * 0.5).collect();
    let mut sheet = Worksheet::new("Sheet1");
    sheet.columns = vec![
        col("A", Designation::X, x.clone(), "Time", "s"),
        col("B", Designation::Y, x.iter().map(|x| x * x).collect(), "Squares", "mA"),
        col("C", Designation::Y, x.iter().map(|x| 100.0 - 5.0 * x).collect(), "Line", "mA"),
        col("D", Designation::Y, x.iter().map(|x| 1000.0 + 50.0 * x).collect(), "Offset", "V"),
        col("E", Designation::Y, x.iter().map(|x| x.sin() * 0.05).collect(), "Wave", "A"),
        col("F", Designation::Y, x.iter().map(|x| 5.0 + x.cos()).collect(), "Cosine", "V"),
        col("G", Designation::Y, vec![3.0, 5.0, 2.0, 6.0, 4.0, 7.0], "Bars", "n"),
        col("H", Designation::Y, vec![4.0, 2.0, 5.0, 3.0, 6.0, 2.0], "Bars2", "n"),
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

fn name(p: &mut Project, id: WindowId, n: &str) {
    p.graph_mut(id).unwrap().name = n.to_string();
}

fn doubley() -> Project {
    let mut p = source();
    let c = ids(&p);
    let g = p.plot_columns_double_y(Template::DoubleY, &[c[1]], &[c[4]]).unwrap();
    name(&mut p, g, "DY1");
    let g = p.plot_columns_double_y(Template::DoubleY, &[c[1], c[2]], &[c[4], c[5]]).unwrap();
    name(&mut p, g, "DY2");
    // a right axis added to a cyclic voltammogram: ticks in, closed box, black axes
    let id = p.add_graph(Template::CyclicVoltammetry);
    p.add_plot_xy(id, 0, Some(c[0]), c[4], None).unwrap();
    let right = p.add_right_y_layer(id, 0).unwrap();
    p.add_plot_xy(id, right, Some(c[0]), c[1], None).unwrap();
    p.graph_mut(id).unwrap().layers[right].y_axis.title = Some("Charge (C)".into());
    name(&mut p, id, "DY3");
    p
}

fn stack() -> Project {
    let mut p = source();
    let c = ids(&p);
    for (n, mode) in [
        ("SAuto", Stack::Auto { gap_percent: 10.0 }),
        ("SConst", Stack::Constant { step: 40.0 }),
        ("SInd", Stack::Individual),
    ] {
        let id = p.add_graph(Template::StackedLines);
        for y in [1, 2, 4, 5] {
            p.add_plot_xy(id, 0, Some(c[0]), c[y], None).unwrap();
        }
        name(&mut p, id, n);
        let l = &mut p.graph_mut(id).unwrap().layers[0];
        l.stack = mode;
        if mode == Stack::Individual {
            for (k, y) in [1, 2, 4, 5].into_iter().enumerate() {
                let o = l.plot_offset_mut(c[y]);
                o.y = 25.0 * k as f64;
                o.x = if k == 3 { 1.5 } else { 0.0 };
            }
        }
    }
    p
}

fn bars() -> Project {
    let mut p = source();
    let c = ids(&p);
    for (n, gap, overlap) in [("B0", 0.0, 0.0), ("B20", 20.0, 0.0), ("B60", 60.0, 0.0), ("BOv", 20.0, 50.0)] {
        let id = p.add_graph(Template::Column);
        for y in [6, 7] {
            p.add_plot_xy(id, 0, None, c[y], None).unwrap();
        }
        name(&mut p, id, n);
        p.graph_mut(id).unwrap().layers[0].bars = BarSpacing { gap_pct: gap, overlap_pct: overlap };
    }
    p
}

fn axes() -> Project {
    let mut p = source();
    let c = ids(&p);
    let add = |p: &mut Project, n: &str, f: &dyn Fn(&mut Layer)| {
        let id = p.add_graph(Template::Line);
        p.add_plot_xy(id, 0, Some(c[0]), c[1], None).unwrap();
        name(p, id, n);
        f(&mut p.graph_mut(id).unwrap().layers[0]);
    };
    add(&mut p, "AxTop", &|l| l.x_axis.side = AxisSide::Opposite);
    add(&mut p, "AxRight", &|l| l.y_axis.side = AxisSide::Opposite);
    add(&mut p, "AxHide", &|l| {
        l.x_axis.side = AxisSide::Hidden;
        l.y_axis.side = AxisSide::Hidden;
    });
    add(&mut p, "AxColor", &|l| {
        l.x_axis.color_follows_plot = true;
        l.y_axis.color_follows_plot = true;
    });
    p
}

fn main() {
    let Some(dir) = std::env::args().nth(1) else {
        eprintln!("usage: opj_write_graph_features <output folder>");
        std::process::exit(2);
    };
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir).expect("the output folder can be made");
    for (name, project) in [("gf-doubley", doubley()), ("gf-stack", stack()), ("gf-bars", bars()), ("gf-axes", axes())]
    {
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
