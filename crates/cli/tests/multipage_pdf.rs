//! `sheaf export` and `sheaf print` with several graphs write one multi-page PDF (T32).
use sheaf_core::page_setup::{Margins, Orientation, Paper};
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation, Project};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Dir(PathBuf);
impl Dir {
    fn new(tag: &str) -> Self {
        let p = std::env::temp_dir().join(format!("sheaf-multipage-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Page widths of Graph1, Graph2 and Graph3, in points.
const WIDTHS: [f64; 3] = [400., 300., 500.];
/// Page heights: Graph2 is taller than wide, so it prints portrait.
const HEIGHTS: [f64; 3] = [200., 400., 250.];

/// A project with three graphs of different page widths, in project order Graph1..Graph3.
fn fixture(dir: &Path, graphs: usize) -> PathBuf {
    let mut p = Project::new();
    let b = p.add_workbook("Data");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0., 1., 2.])).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1., 2., 4.])).unwrap();
    for (&w, &h) in WIDTHS.iter().zip(&HEIGHTS).take(graphs) {
        let g = p.add_graph(Template::Line);
        p.add_plot(g, 0, y).unwrap();
        let page = &mut p.graph_mut(g).unwrap().page;
        page.width_pt = w;
        page.height_pt = h;
    }
    p.page_setup.paper = Paper::Letter;
    p.page_setup.orientation = Orientation::Auto;
    p.page_setup.margins = Margins::all(72.);
    let path = dir.join("project.sheaf");
    sheaf_io::project_file::save(&p, &path).unwrap();
    path
}

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().unwrap()
}
fn text(p: &Path) -> &str {
    p.to_str().unwrap()
}
fn ok(o: &Output) {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}

/// The `/Count n` of the page tree and the [x0 y0 x1 y1] of every `/MediaBox`, in file order.
fn pages(path: &Path) -> (usize, Vec<[f64; 4]>) {
    let bytes = std::fs::read(path).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    let pdf = String::from_utf8_lossy(&bytes);
    let count = pdf
        .split("/Count ")
        .nth(1)
        .and_then(|rest| rest.split(|c: char| !c.is_ascii_digit()).next())
        .and_then(|n| n.parse().ok())
        .expect("a page tree with /Count");
    let boxes = pdf
        .split("/MediaBox [")
        .skip(1)
        .map(|rest| {
            let numbers: Vec<f64> =
                rest.split(']').next().unwrap().split_whitespace().map(|n| n.parse().unwrap()).collect();
            [numbers[0], numbers[1], numbers[2], numbers[3]]
        })
        .collect();
    (count, boxes)
}
fn widths(boxes: &[[f64; 4]]) -> Vec<f64> {
    boxes.iter().map(|b| b[2] - b[0]).collect()
}
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.5
}

/// The width of each graph's own one-page PDF with `--margin page`.
fn one_page_widths(dir: &Path, project: &Path) -> Vec<f64> {
    (1..=3)
        .map(|i| {
            let out = dir.join(format!("one{i}.pdf"));
            let name = format!("Graph{i}");
            ok(&cli(&["export", text(project), "--graph", &name, "--margin", "page", "-o", text(&out)]));
            let (count, boxes) = pages(&out);
            assert_eq!((count, boxes.len()), (1, 1), "a one-graph PDF has one page");
            widths(&boxes)[0]
        })
        .collect()
}

#[test]
fn all_graphs_export_one_page_per_graph_in_project_order() {
    let dir = Dir::new("all");
    let project = fixture(&dir.0, 3);
    let single = one_page_widths(&dir.0, &project);
    for (w, page) in single.iter().zip(WIDTHS) {
        assert!(close(*w, page), "--margin page keeps the page width: {w} vs {page}");
    }
    let out = dir.0.join("all.pdf");
    let o = cli(&["export", text(&project), "--all-graphs", "--margin", "page", "-o", text(&out)]);
    ok(&o);
    assert!(String::from_utf8_lossy(&o.stdout).contains("3 pages"));
    let (count, boxes) = pages(&out);
    assert_eq!((count, boxes.len()), (3, 3));
    let got = widths(&boxes);
    assert!(got.iter().zip(&single).all(|(a, b)| close(*a, *b)), "project order: {got:?} vs {single:?}");
    // The extension is matched without regard to case.
    let upper = dir.0.join("ALL.PDF");
    ok(&cli(&["export", text(&project), "--all-graphs", "-o", text(&upper)]));
    assert_eq!(pages(&upper).0, 3);
}

#[test]
fn repeated_graph_gives_pages_in_the_order_given() {
    let dir = Dir::new("order");
    let project = fixture(&dir.0, 3);
    let out = dir.0.join("order.pdf");
    let o = cli(&[
        "export",
        text(&project),
        "--graph",
        "Graph3",
        "--graph",
        "Graph1",
        "--graph",
        "Graph3",
        "--margin",
        "page",
        "-o",
        text(&out),
    ]);
    ok(&o);
    let (count, boxes) = pages(&out);
    assert_eq!((count, boxes.len()), (3, 3), "a name given twice gets a page each time");
    let got = widths(&boxes);
    let want = [WIDTHS[2], WIDTHS[0], WIDTHS[2]];
    assert!(got.iter().zip(want).all(|(a, b)| close(*a, b)), "{got:?} vs {want:?}");
    let o = cli(&["export", text(&project), "--graph", "Graph1", "--graph", "missing", "-o", text(&out)]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("missing"));
}

#[test]
fn several_graphs_need_a_pdf_output() {
    let dir = Dir::new("refuse");
    let project = fixture(&dir.0, 3);
    for (args, name) in [(vec!["--graph", "Graph1", "--graph", "Graph2"], "two.png"), (vec!["--all-graphs"], "all.svg")]
    {
        let out = dir.0.join(name);
        let mut full = vec!["export", text(&project)];
        full.extend(args);
        full.extend(["-o", text(&out)]);
        let o = cli(&full);
        assert!(!o.status.success(), "{name}");
        let err = String::from_utf8_lossy(&o.stderr);
        assert!(err.contains("multi-page PDF") && err.contains(".pdf"), "{err}");
        assert!(!out.exists(), "nothing is written");
    }
    let o = cli(&["export", text(&project), "--all-graphs", "--graph", "Graph1", "-o", "x.pdf"]);
    assert!(!o.status.success(), "--all-graphs conflicts with --graph");
    let o = cli(&["export", text(&project), "--all-graphs", "-o", text(&dir.0.join("p.opj"))]);
    assert!(!o.status.success(), "an .opj output holds the whole project");
}

#[test]
fn image_width_applies_to_every_page() {
    let dir = Dir::new("width");
    let project = fixture(&dir.0, 3);
    let out = dir.0.join("width.pdf");
    ok(&cli(&["export", text(&project), "--all-graphs", "--image-width", "2in", "-o", text(&out)]));
    let (count, boxes) = pages(&out);
    assert_eq!((count, boxes.len()), (3, 3));
    assert!(widths(&boxes).iter().all(|w| close(*w, 144.)), "{boxes:?}");
}

#[test]
fn all_graphs_with_one_graph_writes_one_page_and_keeps_other_formats() {
    let dir = Dir::new("one");
    let project = fixture(&dir.0, 1);
    let out = dir.0.join("one.pdf");
    ok(&cli(&["export", text(&project), "--all-graphs", "-o", text(&out)]));
    assert_eq!(
        pages(&out),
        pages(&{
            let plain = dir.0.join("plain.pdf");
            ok(&cli(&["export", text(&project), "-o", text(&plain)]));
            plain
        })
    );
    assert_eq!(pages(&out).1.len(), 1);
    let png = dir.0.join("one.png");
    ok(&cli(&["export", text(&project), "--all-graphs", "-o", text(&png)]));
    assert!(std::fs::read(&png).unwrap().starts_with(b"\x89PNG"));
}

#[test]
fn print_all_graphs_writes_paper_sized_pages() {
    let dir = Dir::new("print");
    let project = fixture(&dir.0, 3);
    let out = dir.0.join("print.pdf");
    ok(&cli(&["print", text(&project), "--all-graphs", "--output", text(&out)]));
    let (count, boxes) = pages(&out);
    assert_eq!((count, boxes.len()), (3, 3));
    // Letter on every sheet, each turned by Page Setup's Auto orientation: Graph2 is taller
    // than wide, so its sheet is portrait.
    let sizes: Vec<(f64, f64)> = boxes.iter().map(|b| (b[2] - b[0], b[3] - b[1])).collect();
    let want = [(792., 612.), (612., 792.), (792., 612.)];
    assert!(sizes.iter().zip(want).all(|(a, b)| close(a.0, b.0) && close(a.1, b.1)), "{sizes:?}");
    let o = cli(&["print", text(&project), "--graph", "Graph2", "--graph", "Graph1", "--output", text(&out)]);
    ok(&o);
    assert_eq!(pages(&out).0, 2);
    ok(&cli(&["print", text(&project), "--graph", "Graph2", "--output", text(&out)]));
    assert_eq!(pages(&out).0, 1);
}

#[cfg(unix)]
#[test]
fn print_submits_one_lp_job_per_graph() {
    use std::os::unix::fs::PermissionsExt;
    let dir = Dir::new("lp");
    let project = fixture(&dir.0, 3);
    let lp = dir.0.join("lp");
    let log = dir.0.join("jobs");
    let script = "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$TEST_PRINT_ARGS\"\nfor last do :; done\ntest -f \"$last\" || exit 3\necho queued\n";
    std::fs::write(&lp, script).unwrap();
    std::fs::set_permissions(&lp, std::fs::Permissions::from_mode(0o700)).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf"))
        .args(["print", text(&project), "--graph", "Graph2", "--graph", "Graph1"])
        .env("PATH", &dir.0)
        .env("TMPDIR", &dir.0)
        .env("TEST_PRINT_ARGS", &log)
        .output()
        .unwrap();
    ok(&out);
    let jobs = std::fs::read_to_string(&log).unwrap();
    let jobs: Vec<&str> = jobs.lines().collect();
    assert_eq!(jobs.len(), 2, "{jobs:?}");
    // Each job keeps its own graph's orientation.
    assert!(jobs[0].starts_with("-t Graph2 -o media=Letter -o portrait"), "{}", jobs[0]);
    assert!(jobs[1].starts_with("-t Graph1 -o media=Letter -o landscape"), "{}", jobs[1]);
}
