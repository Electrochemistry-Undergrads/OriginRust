//! Plot > System Templates... (T104, graphing.md 24).

use super::*;
use sheaf_core::template::Template;

#[test]
fn system_templates_plot_the_selected_columns_with_the_picked_template() {
    let mut h = start(vec![data_file("system-templates")]);
    show_window(&mut h, "Book1 - sample.csv");
    let c = column_id(&h, "C");
    click_text(&mut h, "C(Y1)");
    menu_path(&mut h, &["Plot", "System Templates..."]);
    assert!(h.state().system_templates.is_some(), "the window opened");
    let before = project(&h).graphs().count();
    h.state_mut().system_templates.as_mut().unwrap().selected = Some(Template::Scatter);
    settle(&mut h);
    // The window's Plot button (the menu bar has a Plot menu too).
    h.query_all_by_label("Plot").last().expect("the Plot button").click();
    settle(&mut h);
    assert!(h.state().system_templates.is_none());
    assert_eq!(project(&h).graphs().count(), before + 1);
    let g = project(&h).graphs().last().unwrap();
    assert_eq!(g.layers[0].plots[0].y, c);
    assert_eq!(g.layers[0].plots[0].kind, sheaf_core::graph::PlotKind::Scatter);
}
