//! A column's sampling interval (T84) survives saving: in the `.sheaf` file, and through the
//! companion of an `.opj` save, whose warnings name it (the `.opj` written here does not hold it).
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation, Project};
use std::path::PathBuf;

struct Dir(PathBuf);

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_interval_and_its_plot_survive_saving_as_sheaf_and_opj() {
    let dir = Dir(std::env::temp_dir().join(format!("sheaf-sampling-{}", std::process::id())));
    std::fs::create_dir(&dir.0).unwrap();
    for extension in ["sheaf", "opj"] {
        let mut project = Project::new();
        let book = project.add_workbook("");
        project.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        let y = project.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![4.0, 5.0, 6.0])).unwrap();
        project.set_sampling_interval(&[y], 0.0, 0.5, "Time", "s").unwrap();
        project.plot_columns(Template::Scatter, &[y]).unwrap();
        let path = dir.0.join(format!("interval.{extension}"));
        let saved = sheaf_io::save::save_project(&project, &path).unwrap();
        assert!(saved.main_written);
        if extension == "opj" {
            assert!(
                saved.warnings.iter().any(|w| w.contains("1 column(s) have a sampling interval")),
                "{:?}",
                saved.warnings
            );
        }
        let opened = sheaf_io::open_project(&path).unwrap().project;
        let sheet = &opened.workbooks().next().unwrap().sheets[0];
        let s = sheet.columns[1].sampling.clone().expect("the interval is kept");
        assert_eq!((s.x0, s.inc, s.long_name.as_str(), s.units.as_str()), (0.0, 0.5, "Time", "s"));
        let graph = opened.graphs().next().unwrap();
        assert_eq!(graph.layers[0].plots[0].x, None, "{extension}: the plot uses the interval");
    }
}
