//! Writes sample projects for the third batch of graph features of the .opj file (X offsets and
//! waterfalls, date tick labels, bar overlap, ...), for checking them in Origin with
//! `tools/origin-ref/opj-verify-graph-features.ps1` and `opj-read-features.ps1`.
//!
//! Usage: `cargo run -p sheaf-io --example opj_write_graph_features3 -- <output folder>`
//!
//! - `gx-offsets.opj`: curves with X and Y offsets (individual) and a waterfall (as offsets).
//! - `gx-dates.opj`: a graph whose X axis shows dates.
//! - `gx-places.opj`: four windows with places in the workspace (normal, minimised, maximised).

use sheaf_core::Project;
use sheaf_core::column::{Column, ColumnData, Designation};
use sheaf_core::datetime::{DatePattern, UNIX_EPOCH_VALUE};
use sheaf_core::graph::{Stack, TickFormat};
use sheaf_core::ids::{ColumnId, WindowId};
use sheaf_core::template::Template;
use sheaf_core::worksheet::{Workbook, Worksheet};
use std::path::Path;

fn col(short: &str, d: Designation, data: Vec<f64>, long: &str, units: &str) -> Column {
    Column::new(ColumnId(0), short, d, ColumnData::numeric(data)).with_labels(long, units, "")
}

fn source() -> Project {
    let x: Vec<f64> = (0..21).map(|i| f64::from(i) * 0.5).collect();
    let days: Vec<f64> = (0..21).map(|i| UNIX_EPOCH_VALUE + 20_400.0 + f64::from(i) * 3.0).collect();
    let mut sheet = Worksheet::new("Sheet1");
    sheet.columns = vec![
        col("A", Designation::X, x.clone(), "Time", "s"),
        col("B", Designation::Y, x.iter().map(|x| x * x).collect(), "Squares", "mA"),
        col("C", Designation::Y, x.iter().map(|x| 100.0 - 5.0 * x).collect(), "Line", "mA"),
        col("D", Designation::Y, x.iter().map(|x| 30.0 + x.sin() * 10.0).collect(), "Wave", "A"),
        col("E", Designation::X, days, "Date", ""),
        col("F", Designation::Y, x.iter().map(|x| 5.0 + x.cos()).collect(), "Cosine", "V"),
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

fn offsets() -> Project {
    let mut p = source();
    let c = ids(&p);
    let id = p.add_graph(Template::StackedLines);
    for y in [1, 2, 3] {
        p.add_plot_xy(id, 0, Some(c[0]), c[y], None).unwrap();
    }
    name(&mut p, id, "XOff");
    {
        let l = &mut p.graph_mut(id).unwrap().layers[0];
        l.stack = Stack::Individual;
        l.plot_offset_mut(c[1]).x = 0.0;
        l.plot_offset_mut(c[2]).x = 2.0;
        l.plot_offset_mut(c[2]).y = 40.0;
        l.plot_offset_mut(c[3]).x = 4.0;
        l.plot_offset_mut(c[3]).y = 80.0;
    }
    let id = p.add_graph(Template::Line);
    for y in [1, 2, 3] {
        p.add_plot_xy(id, 0, Some(c[0]), c[y], None).unwrap();
    }
    name(&mut p, id, "Waterf");
    p.graph_mut(id).unwrap().layers[0].stack = Stack::Waterfall { dx_percent: 10.0, dy_percent: 15.0 };
    p
}

fn dates() -> Project {
    let mut p = source();
    let c = ids(&p);
    let id = p.add_graph(Template::Line);
    p.add_plot_xy(id, 0, Some(c[4]), c[5], None).unwrap();
    name(&mut p, id, "Dates");
    p.graph_mut(id).unwrap().layers[0].x_axis.tick_format = TickFormat::DateTime(DatePattern::Mdy);
    p
}

fn places() -> Project {
    use sheaf_core::project::{WindowPlace, WindowState};
    let mut p = source();
    let c = ids(&p);
    let mut ids_w = vec![p.windows[0].id()];
    for (n, y) in [("P1", 1), ("P2", 2), ("P3", 3)] {
        let id = p.add_graph(Template::Line);
        p.add_plot_xy(id, 0, Some(c[0]), c[y], None).unwrap();
        name(&mut p, id, n);
        ids_w.push(id);
    }
    let spec = [
        (30.0, 40.0, 600.0, 300.0, WindowState::Normal),
        (200.0, 120.0, 500.0, 400.0, WindowState::Normal),
        (310.0, 90.0, 450.0, 350.0, WindowState::Minimised),
        (120.0, 60.0, 520.0, 380.0, WindowState::Maximised),
    ];
    p.window_places = ids_w
        .iter()
        .zip(spec)
        .map(|(&window, (x, y, width, height, state))| WindowPlace { window, x, y, width, height, state })
        .collect();
    p
}

fn main() {
    let Some(dir) = std::env::args().nth(1) else {
        eprintln!("usage: opj_write_graph_features3 <output folder>");
        std::process::exit(2);
    };
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir).expect("the output folder can be made");
    for (name, project) in [("gx-offsets", offsets()), ("gx-dates", dates()), ("gx-places", places())] {
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
