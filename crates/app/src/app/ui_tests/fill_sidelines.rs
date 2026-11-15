use super::*;
use sheaf_core::graph::FillUnderMode;

#[test]
fn fill_sidelines_keep_group_modes_and_support_apply_cancel_and_history() {
    let mut h = start(vec![data_file("fill-sidelines")]);
    let open = |h: &mut Harness<'static, SheafApp>| {
        open_props(h, 0, PropsTab::Plots, Some(0));
        click(h, "Line");
    };
    let pick = |h: &mut Harness<'static, SheafApp>, label: &str| {
        click(h, "Fill mode");
        click(h, label);
    };
    open(&mut h);
    assert!(is_disabled(&h, "Fill mode"));
    click(&mut h, "Fill Area Under Curve");
    pick(&mut h, "Sidelines only");
    assert!(graph(&h).layers[0].plots.iter().all(|p| p.decor.fill_under_mode == FillUnderMode::SidelinesOnly));
    assert!(!has_label(&h, "Fill color"));
    click(&mut h, "Cancel");
    assert!(graph(&h).layers[0].plots[0].decor.fill_under.is_none());
    assert_eq!(graph(&h).layers[0].plots[0].decor.fill_under_mode, FillUnderMode::Normal);
    open(&mut h);
    click(&mut h, "Fill Area Under Curve");
    pick(&mut h, "Sidelines and Base");
    click(&mut h, "Apply");
    click(&mut h, "Fill Area Under Curve");
    assert!(is_disabled(&h, "Fill mode"));
    click(&mut h, "Cancel");
    assert!(graph(&h).layers[0].plots[0].decor.fill_under.is_some());
    assert_eq!(graph(&h).layers[0].plots[0].decor.fill_under_mode, FillUnderMode::SidelinesAndBase);
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert!(graph(&h).layers[0].plots[0].decor.fill_under.is_none());
    menu(&mut h, "Edit", "Redo Change graph settings");
    assert_eq!(graph(&h).layers[0].plots[0].decor.fill_under_mode, FillUnderMode::SidelinesAndBase);
}
