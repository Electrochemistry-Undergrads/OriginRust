use super::*;
use sheaf_core::graph::LabelFrame;

#[test]
fn label_frames_and_margins_follow_group_editing_cancel_apply_and_history() {
    let mut h = start(vec![data_file("label-frames")]);
    let open = |h: &mut Harness<'static, SheafApp>| {
        open_props(h, 0, PropsTab::Plots, Some(0));
        click(h, "Label");
    };
    let frame = |h: &mut Harness<'static, SheafApp>, name: &str| {
        click(h, "Frame");
        click(h, name);
    };
    open(&mut h);
    assert!(is_disabled(&h, "Frame"));
    assert!(is_disabled(&h, "Margin (% Font Height)"));
    click(&mut h, "Enable");
    frame(&mut h, "Box");
    click(&mut h, "Margin (% Font Height)");
    key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("20".into()));
    settle(&mut h);
    key(&mut h, egui::Key::Enter);
    assert!(
        graph(&h).layers[0]
            .plots
            .iter()
            .all(|p| p.decor.labels.frame == LabelFrame::Box && p.decor.labels.margin_percent == 20.0)
    );
    click(&mut h, "Cancel");
    assert_eq!(graph(&h).layers[0].plots[0].decor.labels.frame, LabelFrame::None);
    open(&mut h);
    click(&mut h, "Enable");
    frame(&mut h, "Shadow");
    click(&mut h, "Apply");
    frame(&mut h, "Box");
    click(&mut h, "Cancel");
    assert_eq!(graph(&h).layers[0].plots[0].decor.labels.frame, LabelFrame::Shadow);
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert_eq!(graph(&h).layers[0].plots[0].decor.labels.frame, LabelFrame::None);
    menu(&mut h, "Edit", "Redo Change graph settings");
    assert_eq!(graph(&h).layers[0].plots[0].decor.labels.frame, LabelFrame::Shadow);
}
