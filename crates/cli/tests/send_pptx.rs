//! `sheaf export project -o slides.pptx`: Send Graphs to PowerPoint (T232), one slide per graph.
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation, Project};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("sheaf-pptx-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

/// A project with `graphs` line graphs of one Y column; the second has a tall page.
fn fixture(dir: &Path, graphs: usize) -> PathBuf {
    let mut p = Project::new();
    let b = p.add_workbook("Data");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0., 1., 2.])).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1., 2., 4.])).unwrap();
    for i in 0..graphs {
        let g = p.add_graph(Template::Line);
        p.add_plot(g, 0, y).unwrap();
        if i == 1 {
            let page = &mut p.graph_mut(g).unwrap().page;
            (page.width_pt, page.height_pt) = (300., 400.);
        }
    }
    let path = dir.join("project.sheaf");
    sheaf_io::project_file::save(&p, &path).unwrap();
    path
}

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().unwrap()
}

fn entry(path: &Path, name: &str) -> Option<Vec<u8>> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut f = zip.by_name(name).ok()?;
    let mut v = Vec::new();
    f.read_to_end(&mut v).unwrap();
    Some(v)
}

#[test]
fn all_graphs_become_one_slide_each_with_the_graph_as_a_png() {
    let dir = scratch("all");
    let project = fixture(&dir, 2);
    let out = dir.join("slides.pptx");
    let o = cli(&["export", project.to_str().unwrap(), "--all-graphs", "--dpi", "96", "-o", out.to_str().unwrap()]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(String::from_utf8_lossy(&o.stdout).contains("(2 slides)"));
    let pres = String::from_utf8(entry(&out, "ppt/presentation.xml").unwrap()).unwrap();
    assert_eq!(pres.matches("<p:sldId ").count(), 2);
    let png = entry(&out, "ppt/media/image2.png").unwrap();
    let (w, h) = sheaf_io::pptx::png_size(&png).unwrap();
    assert!(h > w, "the second graph's tall page: {w} x {h}");
    assert!(entry(&out, "ppt/slides/slide3.xml").is_none());
}

#[test]
fn one_graph_by_name_gives_one_slide() {
    let dir = scratch("one");
    let project = fixture(&dir, 2);
    let out = dir.join("one.PPTX");
    let o =
        cli(&["export", project.to_str().unwrap(), "--graph", "Graph2", "--dpi", "72", "-o", out.to_str().unwrap()]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let pres = String::from_utf8(entry(&out, "ppt/presentation.xml").unwrap()).unwrap();
    assert_eq!(pres.matches("<p:sldId ").count(), 1);
    let slide = String::from_utf8(entry(&out, "ppt/slides/slide1.xml").unwrap()).unwrap();
    assert!(slide.contains(r#"name="Graph2""#), "{slide}");
}

#[test]
fn a_docx_output_puts_each_graph_on_its_own_page() {
    let dir = scratch("docx");
    let project = fixture(&dir, 2);
    let out = dir.join("graphs.docx");
    let o = cli(&["export", project.to_str().unwrap(), "--all-graphs", "--dpi", "72", "-o", out.to_str().unwrap()]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(String::from_utf8_lossy(&o.stdout).contains("(2 pages)"));
    let doc = String::from_utf8(entry(&out, "word/document.xml").unwrap()).unwrap();
    assert_eq!(doc.matches("<wp:inline ").count(), 2);
    assert!(doc.contains(">Graph2</w:t>"), "the caption names the graph");
    assert!(entry(&out, "word/media/image2.png").is_some());
}
