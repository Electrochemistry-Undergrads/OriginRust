//! Tools > Fitting Function Organizer (F9, T105, fitting.md "Fitting Function Organizer").

use super::*;

#[test]
fn f9_opens_the_organizer_and_a_function_shows_its_definition() {
    let mut h = start(vec![data_file("fit-organizer")]);
    show_window(&mut h, "Book1 - sample.csv");
    h.key_press(egui::Key::F9);
    settle(&mut h);
    assert!(h.state().fit_organizer.is_some(), "F9 opens the organizer");
    h.state_mut().fit_organizer.as_mut().unwrap().selected = Some("Gauss".into());
    settle(&mut h);
    assert!(h.query_by_label("y0,xc,w,A").is_some(), "the parameter names show");
    // Escape closes it, as the Close button does.
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().fit_organizer.is_none());
}
