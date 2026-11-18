//! Printing is tested with a vector PDF and a fake lp, never a physical printer.
use sheaf_core::page_setup::{Margins, Orientation, Paper};
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation, Project};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Dir(PathBuf);
impl Dir {
    fn new() -> Self {
        Self(tempfile_for_tests())
    }
}
fn tempfile_for_tests() -> PathBuf {
    let p =
        std::env::temp_dir().join(format!("sheaf-print-test-{}-{:?}", std::process::id(), std::thread::current().id()));
    std::fs::create_dir_all(&p).unwrap();
    p
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fixture(dir: &Path) -> PathBuf {
    let mut p = Project::new();
    let b = p.add_workbook("Data");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0., 1., 2.])).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1., 2., 4.])).unwrap();
    let g = p.add_graph(Template::Line);
    p.add_plot(g, 0, y).unwrap();
    p.graph_mut(g).unwrap().page.width_pt = 400.;
    p.graph_mut(g).unwrap().page.height_pt = 200.;
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

fn decoded_streams(pdf: &[u8]) -> String {
    use std::io::Read;
    let mut out = String::new();
    let mut from = 0;
    while let Some(pos) = pdf[from..].windows(7).position(|w| w == b"stream\n") {
        let start = from + pos + 7;
        let Some(end) = pdf[start..].windows(9).position(|w| w == b"endstream") else { break };
        let data = &pdf[start..start + end];
        let mut decoded = String::new();
        if flate2::read::ZlibDecoder::new(data).read_to_string(&mut decoded).is_ok() {
            out.push_str(&decoded);
            out.push('\n');
        } else if let Ok(text) = std::str::from_utf8(data) {
            out.push_str(text);
        }
        from = start + end + 9;
    }
    out
}

#[test]
fn paper_sized_pdf_selects_graph_and_refuses_invalid_inputs() {
    let dir = Dir::new();
    let project = fixture(&dir.0);
    let pdf = dir.0.join("print.pdf");
    let o = cli(&["print", text(&project), "--graph", "Graph1", "--output", text(&pdf)]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let bytes = std::fs::read(&pdf).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    let pdf_text = String::from_utf8_lossy(&bytes);
    assert!(pdf_text.contains("/MediaBox [0 0 792 612]"), "{pdf_text}");
    assert!(!pdf_text.contains("/Subtype /Image"), "printing stays vector");
    let streams = decoded_streams(&bytes);
    assert!(streams.contains(" re") || streams.contains(" m"), "PDF needs vector drawing: {streams}");
    assert!(streams.contains("1.62 0 0 1.62 72 144 cm"), "incorrect placement: {streams}");
    let mut changed = sheaf_io::project_file::load(&project).unwrap();
    changed.page_setup.margins = Margins { left: 36., right: 72., top: 100., bottom: 80. };
    sheaf_io::project_file::save(&changed, &project).unwrap();
    let o = cli(&["print", text(&project), "--output", text(&pdf)]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let streams = decoded_streams(&std::fs::read(&pdf).unwrap());
    assert!(streams.contains("1.71 0 0 1.71 36 145 cm"), "incorrect asymmetric placement: {streams}");
    let o = cli(&["print", text(&project), "--graph", "missing", "--output", text(&pdf)]);
    assert!(!o.status.success());
    let o = cli(&["print", text(&project), "--output", text(&project)]);
    assert!(!o.status.success());
    assert!(sheaf_io::project_file::load(&project).is_ok());
    let o = cli(&["print", text(&project), "--output", text(&pdf), "--printer", "office"]);
    assert!(!o.status.success());
}

#[test]
fn relative_output_and_hardlinked_target_preserve_the_project() {
    let dir = Dir::new();
    let project = fixture(&dir.0);
    let before = std::fs::read(&project).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf"))
        .current_dir(&dir.0)
        .args(["print", "project.sheaf", "--output", "relative.pdf"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(std::fs::read(dir.0.join("relative.pdf")).unwrap().starts_with(b"%PDF-"));
    let target = dir.0.join("hardlink.pdf");
    std::fs::hard_link(&project, &target).unwrap();
    let out = cli(&["print", text(&project), "--output", text(&target)]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(std::fs::read(&project).unwrap(), before);
    assert!(std::fs::read(target).unwrap().starts_with(b"%PDF-"));
}

#[cfg(unix)]
#[test]
fn fake_lp_gets_one_printer_argument_and_temp_is_removed_on_success_and_failure() {
    use std::os::unix::fs::PermissionsExt;
    let dir = Dir::new();
    let project = fixture(&dir.0);
    let lp = dir.0.join("lp");
    let log = dir.0.join("arguments");
    let script = "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$TEST_PRINT_ARGS\"\nfor last do :; done\ntest -f \"$last\" || exit 3\nif test \"$TEST_PRINT_FAIL\" = 1; then echo 'queue unavailable' >&2; exit 2; fi\necho queued\n";
    std::fs::write(&lp, script).unwrap();
    std::fs::set_permissions(&lp, std::fs::Permissions::from_mode(0o700)).unwrap();
    for fail in [false, true] {
        let out = Command::new(env!("CARGO_BIN_EXE_sheaf"))
            .args(["print", text(&project), "--printer", "office;touch injected"])
            .env("PATH", &dir.0)
            .env("TMPDIR", &dir.0)
            .env("TEST_PRINT_ARGS", &log)
            .env("TEST_PRINT_FAIL", if fail { "1" } else { "0" })
            .output()
            .unwrap();
        assert_eq!(out.status.success(), !fail, "{}", String::from_utf8_lossy(&out.stderr));
        if fail {
            assert!(String::from_utf8_lossy(&out.stderr).contains("queue unavailable"));
        }
        let args = std::fs::read_to_string(&log).unwrap();
        let lines: Vec<_> = args.lines().collect();
        assert_eq!(
            &lines[..13],
            [
                "-d",
                "office;touch injected",
                "-t",
                "Graph1",
                "-o",
                "media=Letter",
                "-o",
                "landscape",
                "-o",
                "print-scaling=none",
                "-o",
                "fit-to-page=Off",
                "--"
            ]
        );
        assert!(!Path::new(lines[13]).exists(), "temporary PDF must be removed");
        assert!(!dir.0.join("injected").exists());
    }
}
