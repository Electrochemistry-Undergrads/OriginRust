use super::*;
use sheaf_core::graph::FillUnderMode;

#[test]
fn inclusive_fill_keeps_fill_controls_group_edits_and_dialog_history() {
    let mut h = start(vec![data_file("fill-inclusive")]);
    open_props(&mut h, 0, PropsTab::Plots, Some(0));
    click(&mut h, "Line");
    click(&mut h, "Fill Area Under Curve");
    click(&mut h, "Fill mode");
    click(&mut h, "Inclusive broken by missing values");
    assert!(has_label(&h, "Fill color"));
    assert!(graph(&h).layers[0].plots.iter().all(|p| {
        p.decor.fill_under.is_some() && p.decor.fill_under_mode == FillUnderMode::InclusiveBrokenByMissingValues
    }));
    click(&mut h, "Apply");
    click(&mut h, "Fill mode");
    click(&mut h, "Normal");
    click(&mut h, "Cancel");
    assert_eq!(graph(&h).layers[0].plots[0].decor.fill_under_mode, FillUnderMode::InclusiveBrokenByMissingValues);
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert!(graph(&h).layers[0].plots[0].decor.fill_under.is_none());
    menu(&mut h, "Edit", "Redo Change graph settings");
    assert_eq!(graph(&h).layers[0].plots[0].decor.fill_under_mode, FillUnderMode::InclusiveBrokenByMissingValues);
}
