use super::*;

#[test]
fn label_whiteout_is_disabled_with_labels_and_supports_apply_cancel_and_undo() {
    let mut h = start(vec![data_file("label-whiteout")]);
    let open = |h: &mut Harness<'static, SheafApp>| {
        open_props(h, 0, PropsTab::Plots, Some(0));
        click(h, "Label");
    };
    open(&mut h);
    assert!(is_disabled(&h, "White Out"));
    click(&mut h, "Enable");
    assert!(!is_disabled(&h, "White Out"));
    click(&mut h, "White Out");
    assert!(graph(&h).layers[0].plots[0].decor.labels.white_out);
    click(&mut h, "Cancel");
    assert!(!graph(&h).layers[0].plots[0].decor.labels.white_out);
    open(&mut h);
    click(&mut h, "Enable");
    click(&mut h, "White Out");
    click(&mut h, "Apply");
    click(&mut h, "White Out");
    click(&mut h, "Cancel");
    assert!(graph(&h).layers[0].plots[0].decor.labels.white_out);
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert!(!graph(&h).layers[0].plots[0].decor.labels.white_out);
    menu(&mut h, "Edit", "Redo Change graph settings");
    assert!(graph(&h).layers[0].plots[0].decor.labels.white_out);
}
