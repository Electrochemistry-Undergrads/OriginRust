use super::*;

fn page_and_frame(h: &Harness<'static, SheafApp>) -> ((f64, f64), sheaf_core::graph::Frame) {
    let g = graph(h);
    ((g.page.width_pt, g.page.height_pt), g.layers[0].frame)
}

#[test]
fn fit_page_to_layers_keeps_the_layer_size_and_is_one_undo_step() {
    let mut h = start(vec![data_file("fitpage-ok")]);
    let (page, frame) = page_and_frame(&h);
    let (lw, lh) = (frame.width * page.0, frame.height * page.1);
    menu_path(&mut h, &["Graph", "Fit Page to Layers..."]);
    assert!(h.state().fit_page.is_some());
    assert!(h.query_by_label("Margin Control").is_some() && h.query_by_label("Adjust Direction").is_some());
    click(&mut h, "OK");
    assert!(h.state().fit_page.is_none());
    let (page2, frame2) = page_and_frame(&h);
    assert_ne!(page2, page, "the page was fitted");
    // the layer keeps its size in points (Origin keeps it in dots)
    assert!((frame2.width * page2.0 - lw).abs() < 1e-6 && (frame2.height * page2.1 - lh).abs() < 1e-6);
    // the border is 5 percent of the old page on the left
    assert!((frame2.left * page2.0 - 0.05 * page.0).abs() < 1e-6, "{frame2:?}");
    assert_eq!(h.state().doc.undo_label(), Some("Fit Page to Layers"));
    menu(&mut h, "Edit", "Undo Fit Page to Layers");
    assert_eq!(page_and_frame(&h), (page, frame));
}

#[test]
fn cancel_changes_nothing_and_the_format_menu_opens_the_window() {
    let mut h = start(vec![data_file("fitpage-cancel")]);
    let before = page_and_frame(&h);
    let revision = h.state().doc.revision();
    menu_path(&mut h, &["Graph", "Fit Page to Layers..."]);
    click(&mut h, "Cancel");
    assert!(h.state().fit_page.is_none());
    assert_eq!(h.state().doc.revision(), revision);
    menu_path(&mut h, &["Format", "Fit Page To Layers..."]);
    assert!(h.state().fit_page.is_some(), "the window opens");
    assert_eq!(page_and_frame(&h), before, "nothing changes until OK");
    click(&mut h, "OK");
    assert_ne!(page_and_frame(&h), before);
    assert_eq!(h.state().doc.undo_label(), Some("Fit Page to Layers"));
}
