//! Tools > Options... (T134, gui-look "Options").

use super::*;

#[test]
fn options_set_the_autosave_and_list_the_folders() {
    let mut h = start(vec![data_file("options")]);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Options, &ctx);
    settle(&mut h);
    assert!(h.query_by_label("Open/Close").is_some());
    h.state_mut().options.as_mut().unwrap().autosave.minutes = 7;
    h.get_by_label("File Locations").click();
    settle(&mut h);
    assert!(h.query_by_label("Fitting functions").is_some());
    h.get_by_label("OK").click();
    settle(&mut h);
    assert!(h.state().options.is_none());
    assert_eq!(h.state().autosave.settings.minutes, 7, "the Autosave setting is applied");
}
