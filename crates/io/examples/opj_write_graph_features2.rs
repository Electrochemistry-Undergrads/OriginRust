//! Writes sample projects for the second batch of graph features that are stored in the .opj file
//! (area plots, tick label number formats, plot groups of column plots, panels, X stack offsets),
//! for checking them in Origin 2016 with `tools/origin-ref/opj-verify-graph-features.ps1`.
//!
//! Usage: `cargo run -p sheaf-io --example opj_write_graph_features2 -- <output folder>`
//!
//! - `gw-area.opj`: area plots, one and two curves, with chosen colours.
//! - `gw-fmt.opj`: tick label number formats (decimal places, scientific).

use sheaf_core::Project;
use sheaf_core::color::Color;
use sheaf_core::graph::TickFormat;
use sheaf_core::column::{Column, ColumnData, Designation};
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

fn area() -> Project {
    let mut p = source();
    let c = ids(&p);
    let id = p.add_graph(Template::Area);
    p.add_plot_xy(id, 0, Some(c[0]), c[1], None).unwrap();
    name(&mut p, id, "Area1");
    {
        let pl = &mut p.graph_mut(id).unwrap().layers[0].plots[0];
        pl.symbol.color = Color::rgb(0, 160, 0);
        pl.line.color = Color::rgb(200, 0, 0);
        pl.line.width_pt = 2.0;
    }
    let id = p.add_graph(Template::Area);
    for y in [1, 2] {
        p.add_plot_xy(id, 0, Some(c[0]), c[y], None).unwrap();
    }
    name(&mut p, id, "Area2");
    p
}

fn fmt() -> Project {
    let mut p = source();
    let c = ids(&p);
    for (n, fx, fy) in [
        ("Fm0", TickFormat::Auto, TickFormat::Auto),
        ("Fm1", TickFormat::Decimal(0), TickFormat::Decimal(3)),
        ("Fm2", TickFormat::Scientific(2), TickFormat::Decimal(1)),
    ] {
        let id = p.add_graph(Template::Line);
        p.add_plot_xy(id, 0, Some(c[0]), c[1], None).unwrap();
        name(&mut p, id, n);
        let l = &mut p.graph_mut(id).unwrap().layers[0];
        l.x_axis.tick_format = fx;
        l.y_axis.tick_format = fy;
    }
    p
}

fn main() {
    let Some(dir) = std::env::args().nth(1) else {
        eprintln!("usage: opj_write_graph_features2 <output folder>");
        std::process::exit(2);
    };
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir).expect("the output folder can be made");
    for (name, project) in [("gw-area", area()), ("gw-fmt", fmt())] {
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
