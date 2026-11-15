use super::*;

#[test]
fn sort_x_check_box_is_on_the_line_tab_with_one_undo_step() {
    let mut h = start(vec![data_file("sort-x")]);
    open_props(&mut h, 0, PropsTab::Plots, Some(0));
    click(&mut h, "Line");
    assert!(has_label(&h, "Sort X for line connection"));
    assert!(!graph(&h).layers[0].plots[0].line.sort_x);
    click(&mut h, "Sort X for line connection");
    assert!(graph(&h).layers[0].plots[0].line.sort_x);
    click(&mut h, "Cancel");
    assert!(!graph(&h).layers[0].plots[0].line.sort_x);

    open_props(&mut h, 0, PropsTab::Plots, Some(0));
    click(&mut h, "Line");
    click(&mut h, "Sort X for line connection");
    click(&mut h, "Apply");
    assert!(graph(&h).layers[0].plots[0].line.sort_x);
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert!(!graph(&h).layers[0].plots[0].line.sort_x);
    menu(&mut h, "Edit", "Redo Change graph settings");
    assert!(graph(&h).layers[0].plots[0].line.sort_x);
}
