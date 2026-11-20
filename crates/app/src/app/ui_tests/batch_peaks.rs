//! Peak Analyzer Save as Theme and Batch Peak Analysis Using Theme... (T169).

use super::*;

fn peak_file(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut text = String::from("x,a,b\n");
    for i in 0..=80 {
        let x = f64::from(i) * 0.25;
        let a = 0.1 + (-(x - 6.0).powi(2) / 0.5).exp() + 0.6 * (-(x - 14.0).powi(2) / 0.8).exp();
        let b = 0.2 + 0.8 * (-(x - 10.0).powi(2) / 0.6).exp();
        text.push_str(&format!("{x},{a},{b}\n"));
    }
    let path = dir.join("peaks.csv");
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn a_peak_analyzer_theme_is_saved_and_run_on_every_curve_of_the_sheet() {
    let file = peak_file("batch-peaks");
    let themes = file.parent().unwrap().join("peak-themes.json");
    let startup = Startup { paths: vec![file], peak_themes_path: Some(themes.clone()), ..Default::default() };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .with_step_dt(0.02)
        .build_eframe(|cc| SheafApp::new(cc, startup));
    h.state_mut().panels.set_pinned(PanelId::Explorer, true);
    settle(&mut h);
    let ctx = h.ctx.clone();
    let book = project(&h).workbooks().next().unwrap().id;
    let a = project(&h).workbook(book).unwrap().sheets[0].columns[1].id;

    // The Peak Analyzer saves its settings under a name.
    h.state_mut().apply(Action::Analyze { book, columns: vec![a], kind: AnalysisKind::PeakChain }, &ctx);
    settle(&mut h);
    h.get_by_label("Theme").click();
    h.query_all_by_role(egui::accesskit::Role::TextInput).last().expect("the theme name box").focus();
    settle(&mut h);
    h.event(egui::Event::Text("Two peaks".into()));
    settle(&mut h);
    click(&mut h, "Save as Theme");
    assert_eq!(sheaf_io::peak_themes::names(&themes), ["Two peaks"]);
    h.state_mut().analysis = None;
    settle(&mut h);

    // Batch Peak Analysis on the sheet's curves (none selected: both Y columns).
    let sheets = project(&h).workbook(book).unwrap().sheets.len();
    h.state_mut().apply(Action::BatchPeaks { book, columns: Vec::new() }, &ctx);
    settle(&mut h);
    assert_eq!(h.state().batch_peaks.as_ref().unwrap().columns.len(), 2);
    h.get_by_label("Batch Peak Analysis").get_by_label("OK").click();
    settle(&mut h);
    assert!(h.state().batch_peaks.is_none(), "OK runs and closes");
    assert_eq!(project(&h).workbook(book).unwrap().sheets.len(), sheets + 1, "one result sheet");
    assert_eq!(h.state().doc.undo_label(), Some("Batch peak analysis"));
}
