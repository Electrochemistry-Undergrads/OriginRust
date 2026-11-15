use sheaf_core::{ColumnData, Designation, Document, Project, RecalcMode};
use std::path::PathBuf;

struct Dir(PathBuf);

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn cross_sheet_formula_links_survive_native_and_opj_companion_save_and_reopen() {
    let dir = Dir(std::env::temp_dir().join(format!("sheaf-cross-sheet-{}", std::process::id())));
    std::fs::create_dir(&dir.0).unwrap();
    for extension in ["sheaf", "opj"] {
        let mut project = Project::new();
        let src = project.add_workbook("Src");
        project.workbook_mut(src).unwrap().name = "Src".into();
        let a = project.add_column(src, 0, Designation::X, ColumnData::numeric(vec![2.0, 4.0, 8.0])).unwrap();
        let dst = project.add_workbook("Dst");
        project.workbook_mut(dst).unwrap().name = "Dst".into();
        let b = project.add_column(dst, 0, Designation::Y, ColumnData::default()).unwrap();
        project.set_column_values_with(b, "[Src]Sheet1!col(A)*3", None, RecalcMode::Auto).unwrap();
        let path = dir.0.join(format!("cross-sheet.{extension}"));
        let saved = sheaf_io::save::save_project(&project, &path).unwrap();
        assert!(saved.main_written);
        if extension == "opj" {
            assert!(saved.warnings.iter().any(|w| w.contains("recalculate mode None")), "{:?}", saved.warnings);
        }
        let opened = sheaf_io::open_project(&path).unwrap();
        assert_eq!(opened.companion_used, extension == "opj");
        let mut doc = Document::new(opened.project);
        assert_eq!(doc.project().numeric(b).unwrap(), [6.0, 12.0, 24.0]);
        assert_eq!(doc.project().column(b).unwrap().formula.as_deref(), Some("[Src]Sheet1!col(A)*3"));
        doc.apply("Edit remote source", |p| p.column_mut(a).unwrap().data = ColumnData::numeric(vec![5.0, 7.0]));
        assert_eq!(doc.project().numeric(b).unwrap(), [15.0, 21.0]);
        doc.undo();
        assert_eq!(doc.project().numeric(b).unwrap(), [6.0, 12.0, 24.0]);
    }
}
