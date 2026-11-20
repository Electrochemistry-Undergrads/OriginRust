//! View > Show > Labels and Data on a graph (T177, graphing 22.8).

use super::*;

#[test]
fn view_show_labels_and_data_toggle_every_layer_in_one_step_each() {
    let mut h = start(vec![data_file("show-elements")]);
    assert!(h.state().menu_context().show_labels && h.state().menu_context().show_data, "ticked on a new graph");
    menu_path(&mut h, &["View", "Show", "Labels"]);
    assert!(graph(&h).layers.iter().all(|l| !l.display.show_labels && l.display.show_data));
    assert!(!h.state().menu_context().show_labels);
    assert_eq!(h.state().doc.undo_label(), Some("Show Labels"));
    menu_path(&mut h, &["View", "Show", "Data"]);
    assert!(graph(&h).layers.iter().all(|l| !l.display.show_data));
    assert_eq!(h.state().doc.undo_label(), Some("Show Data"));
    menu(&mut h, "Edit", "Undo Show Data");
    menu(&mut h, "Edit", "Undo Show Labels");
    assert!(graph(&h).layers.iter().all(|l| l.display.show_labels && l.display.show_data));
}
