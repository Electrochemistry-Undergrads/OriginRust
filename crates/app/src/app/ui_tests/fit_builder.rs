//! Tools > Fitting Function Builder... (T132, fitting.md "Fitting Function Builder").

use super::*;

#[test]
fn the_builder_saves_an_fdf_that_reads_back_as_a_fit_function() {
    let dir = std::env::temp_dir().join(format!("sheaf-fit-builder-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let startup =
        Startup { paths: vec![data_file("fit-builder")], fit_functions_dir: Some(dir.clone()), ..Default::default() };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .with_step_dt(0.02)
        .build_eframe(|cc| SheafApp::new(cc, startup));
    settle(&mut h);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::FitBuilder, &ctx);
    settle(&mut h);
    {
        let b = h.state_mut().fit_builder.as_mut().unwrap();
        b.name = "Decay2".into();
        b.params = "A=5, t=2, y0".into();
        b.formula = "y0 + A*exp(-x/t)".into();
    }
    h.get_by_label("Save").click();
    settle(&mut h);
    let text = std::fs::read_to_string(dir.join("Decay2.FDF")).unwrap();
    let f = sheaf_fit::fdf::parse_fdf(&text).unwrap();
    assert_eq!(f.name, "Decay2");
    assert_eq!(f.params.len(), 3);
    assert!(f.to_fit_function().is_ok());
    // A bad formula is reported in the window and nothing is written.
    h.state_mut().fit_builder.as_mut().unwrap().formula = "A*".into();
    h.state_mut().fit_builder.as_mut().unwrap().name = "Broken".into();
    h.get_by_label("Save").click();
    settle(&mut h);
    assert!(h.state().fit_builder.as_ref().unwrap().problem.is_some());
    assert!(!dir.join("Broken.FDF").exists());
    let _ = std::fs::remove_dir_all(&dir);
}
