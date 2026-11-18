//! Analysis > Peaks and Baseline > Multiple Peak Fit (T128).

use super::*;

/// A CSV of two Gauss peaks (centres 3 and 7) on an offset of 1.
fn peaks_file() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-multipeak-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("peaks.csv");
    let mut text = String::from("x,y\n");
    for i in 0..=100 {
        let x = i as f64 * 0.1;
        let y =
            1.0 + 2.0 * (-(x - 3.0).powi(2) / (2.0 * 0.4 * 0.4)).exp() + (-(x - 7.0).powi(2) / (2.0 * 0.5 * 0.5)).exp();
        text += &format!("{x},{y}\n");
    }
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn multiple_peak_fit_picks_centres_with_the_data_reader_and_fits_them() {
    let mut h = start(vec![peaks_file()]);
    show_window(&mut h, "Book1 - peaks.csv");
    click_text(&mut h, "B(Y1)");
    menu_path(&mut h, &["Analysis", "Peaks and Baseline", "Multiple Peak Fit"]);
    let graph = h.state().multi_peak.as_ref().expect("the window opens on a new graph").graph;
    assert_eq!(project(&h).graph(graph).unwrap().layers[0].plots.len(), 1);
    click(&mut h, "OK");
    assert!(h.state().multi_peak.as_ref().unwrap().picking);
    assert!(h.state().graphs[&graph].reader, "the Data Reader is on while picking");
    // Pin points with the reader, as a click does, and pick them with Enter.
    for row in [31, 69] {
        h.state_mut().graphs.get_mut(&graph).unwrap().reader_point =
            Some(crate::reader::Point { layer: 0, plot: 0, row });
        h.key_press(egui::Key::Enter);
        settle(&mut h);
    }
    let centres: Vec<f64> = h.state().multi_peak.as_ref().unwrap().centers.iter().map(|c| c.0).collect();
    assert_eq!(centres.len(), 2);
    assert!((centres[0] - 3.1).abs() < 1e-9 && (centres[1] - 6.9).abs() < 1e-9, "{centres:?}");
    // Enter again on the same point picks nothing new.
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(h.state().multi_peak.as_ref().unwrap().centers.len(), 2);
    click(&mut h, "Fit");
    assert!(h.state().multi_peak.is_none(), "Fit closes the windows");
    assert!(!h.state().graphs[&graph].reader, "and turns the Data Reader off");
    assert_eq!(h.state().doc.undo_label(), Some("Multiple peak fit"));
    // Each peak and then the fitted sum are drawn on the graph as lines, the sum in red on top.
    let g = project(&h).graph(graph).unwrap();
    let plots = &g.layers[0].plots;
    assert_eq!(plots.len(), 4);
    assert!(plots[1..].iter().all(|p| p.kind == sheaf_core::graph::PlotKind::Line));
    let name = |i: usize| project(&h).column(plots[i].y).unwrap().long_name.clone();
    assert_eq!((name(1), name(2), name(3)), ("Peak 1 of y".into(), "Peak 2 of y".into(), "Peak fit of y".into()));
    assert_eq!(plots[3].line.color, sheaf_core::color::Color::rgb(220, 30, 30));
    assert_ne!(plots[1].line.color, plots[2].line.color);
    // The results sheet has one row per peak with the fitted centres.
    let b = project(&h).workbooks().next().unwrap();
    let results = b.sheets.last().unwrap();
    let col =
        |name: &str| results.columns.iter().find(|c| c.long_name == name).and_then(|c| c.numeric()).unwrap().to_vec();
    let centre = col("Fit center");
    assert!((centre[0] - 3.0).abs() < 1e-3 && (centre[1] - 7.0).abs() < 1e-3, "{centre:?}");
    // One undo takes the curve and the results away.
    h.state_mut().doc.undo();
    assert_eq!(project(&h).graph(graph).unwrap().layers[0].plots.len(), 1);
}

#[test]
fn cancel_in_get_points_turns_the_reader_off_and_changes_nothing() {
    let mut h = start(vec![peaks_file()]);
    show_window(&mut h, "Book1 - peaks.csv");
    click_text(&mut h, "B(Y1)");
    menu_path(&mut h, &["Analysis", "Peaks and Baseline", "Multiple Peak Fit"]);
    let graph = h.state().multi_peak.as_ref().unwrap().graph;
    click(&mut h, "OK");
    click(&mut h, "Cancel");
    assert!(h.state().multi_peak.is_none());
    assert!(!h.state().graphs[&graph].reader);
    assert_eq!(h.state().doc.undo_label(), Some("Plot"), "only the graph made for the fit");
}
