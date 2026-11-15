use sheaf_core::{ColumnData, Designation, Document, Project, RecalcMode};
use std::path::PathBuf;

struct Dir(PathBuf);
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn worksheet_cell_formulas_keep_their_remote_links_after_native_and_companion_reopen() {
    let dir = Dir(std::env::temp_dir().join(format!("sheaf-worksheet-substitution-{}", std::process::id())));
    std::fs::create_dir(&dir.0).unwrap();
    for extension in ["sheaf", "opj"] {
        let mut p = Project::new();
        let src = p.add_workbook("");
        p.workbook_mut(src).unwrap().name = "Src".into();
        let a = p.add_column(src, 0, Designation::Y, ColumnData::text(vec!["µ".into(), "2+3".into()])).unwrap();
        let dst = p.add_workbook("");
        let b = p.add_column(dst, 0, Designation::Y, ColumnData::default()).unwrap();
        let formula = r#""run:%([Src]Sheet1,A,i,.2)""#;
        p.set_column_values_with(b, formula, None, RecalcMode::Auto).unwrap();
        let path = dir.0.join(format!("cells.{extension}"));
        assert!(sheaf_io::save::save_project(&p, &path).unwrap().main_written);
        let opened = sheaf_io::open_project(&path).unwrap();
        assert_eq!(opened.companion_used, extension == "opj");
        let mut doc = Document::new(opened.project);
        assert_eq!(doc.project().column(b).unwrap().formula.as_deref(), Some(formula));
        assert_eq!(doc.project().column(b).unwrap().data.text_cell(0), Some("run:µ"));
        assert_eq!(doc.project().column(b).unwrap().data.text_cell(1), Some("run:2+3"));
        doc.apply("Change remote input", |p| p.column_mut(a).unwrap().data = ColumnData::text(vec!["new".into()]));
        assert_eq!(doc.project().column(b).unwrap().data.text_cell(0), Some("run:new"));
        assert_eq!(doc.project().column(b).unwrap().len(), 1);
        doc.undo();
        assert_eq!(doc.project().column(b).unwrap().data.text_cell(0), Some("run:µ"));
    }
}
