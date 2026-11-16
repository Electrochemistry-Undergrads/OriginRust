//! Plot > Multi-Y: the 3Ys / 4Ys items and Multiple Y Axes... (T61).
use super::*;

#[test]
fn the_template_items_make_their_layers_in_one_undo_step() {
    for (item, layers) in [("3Ys Y-YY", 3), ("3Ys Y-Y-Y", 3), ("4Ys Y-YYY", 4), ("4Ys YY-YY", 4)] {
        let mut h = start(vec![data_file("multi-y-menu")]);
        let (b, c) = select_b_and_c(&mut h);
        let graphs = project(&h).graphs().count();
        menu_path(&mut h, &["Plot", "Multi-Y", item]);
        assert_eq!(project(&h).graphs().count(), graphs + 1, "{item}");
        let g = project(&h).graphs().last().unwrap();
        assert_eq!(g.layers.len(), layers, "{item}");
        assert_eq!(g.layers[0].plots[0].y, b);
        assert_eq!(g.layers[1].plots[0].y, c);
        assert_eq!(h.state().doc.undo_label(), Some("Plot columns"));
        menu(&mut h, "Edit", "Undo Plot columns");
        assert_eq!(project(&h).graphs().count(), graphs);
    }
}

#[test]
fn multiple_y_axes_dialog_builds_the_axes_it_summarises() {
    let mut h = start(vec![data_file("multi-y-dialog")]);
    let (b, c) = select_b_and_c(&mut h);
    let graphs = project(&h).graphs().count();
    menu_path(&mut h, &["Plot", "Multi-Y", "Multiple Y Axes..."]);
    assert!(h.state().multi_y.is_some());
    assert!(has_label(&h, "2 Y axes and 2 plots in total"));
    assert!(has_label(&h, "Actual number of plots in each layer : 1 1"));
    assert_eq!(project(&h).graphs().count(), graphs, "nothing is made before OK");
    click(&mut h, "OK");
    assert!(h.state().multi_y.is_none());
    let g = project(&h).graphs().last().unwrap();
    assert_eq!(g.layers.len(), 2);
    assert_eq!((g.layers[0].plots[0].y, g.layers[1].plots[0].y), (b, c));
    assert_eq!(h.state().active, Some(g.id));
    assert_eq!(h.state().doc.undo_label(), Some("Plot columns"));
}
