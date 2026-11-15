//! Generated project data only: parameters and their recalculation links must survive saving.
use sheaf_core::{ColumnData, Designation, Document, Project, RecalcMode};
use std::path::PathBuf;

struct Dir(PathBuf);

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn native_and_opj_companion_reopen_preserve_parameter_formulas_and_auto_recalculation() {
    let dir = Dir(std::env::temp_dir().join(format!("sheaf-parameter-labels-{}", std::process::id())));
    std::fs::create_dir(&dir.0).unwrap();
    for extension in ["sheaf", "opj"] {
        let mut project = Project::new();
        let book = project.add_workbook("");
        let a = project.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        let b = project.add_column(book, 0, Designation::Y, ColumnData::default()).unwrap();
        project.column_mut(a).unwrap().params.insert("Scale".into(), "2.5".into());
        project.set_column_values_with(b, "value(col(A)[Scale]$)*col(A)", None, RecalcMode::Auto).unwrap();
        let path = dir.0.join(format!("parameters.{extension}"));
        let saved = sheaf_io::save::save_project(&project, &path).unwrap();
        assert!(saved.main_written);
        let opened = sheaf_io::open_project(&path).unwrap();
        assert_eq!(opened.companion_used, extension == "opj");
        let mut doc = Document::new(opened.project);
        let sheet = &doc.project().workbooks().next().unwrap().sheets[0];
        let (a, b) = (sheet.columns[0].id, sheet.columns[1].id);
        assert_eq!(doc.project().numeric(b).unwrap(), [2.5, 5.0, 7.5]);
        doc.apply("Edit parameter", |p| {
            p.column_mut(a).unwrap().params.insert("Scale".into(), "5".into());
        });
        assert_eq!(doc.project().numeric(b).unwrap(), [5.0, 10.0, 15.0]);
        doc.undo();
        assert_eq!(doc.project().numeric(b).unwrap(), [2.5, 5.0, 7.5]);
    }
}
