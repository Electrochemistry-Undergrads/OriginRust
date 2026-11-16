//! Graph > New Color Scale and the scale objects (T80).

use super::*;
use sheaf_core::layer_scale::LayerScale;

fn scales(h: &Harness<'static, SheafApp>) -> Vec<LayerScale> {
    graph(h).layers[0].scales.clone()
}

/// Makes the first plot's symbols take their colour from its own Y column (any numeric column will do).
fn map_colour(h: &mut Harness<'static, SheafApp>) {
    let id = graph(h).id;
    h.state_mut().doc.apply("Test: map colour", |p| {
        let plot = &mut p.graph_mut(id).unwrap().layers[0].plots[0];
        plot.kind = sheaf_core::graph::PlotKind::Scatter;
        plot.color_from = Some(plot.y);
    });
    settle(h);
}

#[test]
fn new_color_scale_is_grey_until_a_plot_is_colour_mapped() {
    let mut h = start(vec![data_file("cscale-grey")]);
    menu_path(&mut h, &["Graph"]);
    assert!(!entry_enabled(&h, "New Color Scale"));
    close_menus(&mut h);
    map_colour(&mut h);
    menu_path(&mut h, &["Graph"]);
    assert!(entry_enabled(&h, "New Color Scale"));
    close_menus(&mut h);
}

#[test]
fn the_menu_adds_a_scale_in_one_undo_step_that_a_click_selects_and_delete_removes() {
    let mut h = start(vec![data_file("cscale-add")]);
    map_colour(&mut h);
    menu(&mut h, "Graph", "New Color Scale");
    assert_eq!(scales(&h), vec![LayerScale::new(sheaf_core::layer_scale::LayerScaleKind::Color)]);
    assert_eq!(h.state().doc.undo_label(), Some("Add color scale"));
    menu(&mut h, "Edit", "Undo Add color scale");
    assert!(scales(&h).is_empty());
    menu(&mut h, "Edit", "Redo Add color scale");
    assert_eq!(scales(&h).len(), 1);
    settle(&mut h);
    let at = on(&h, 0, ObjectKind::Scale(0));
    click_at(&mut h, at);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Scale(0))));
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert!(scales(&h).is_empty());
    assert_eq!(h.state().doc.undo_label(), Some("Delete scale"));
    menu(&mut h, "Edit", "Undo Delete scale");
    assert_eq!(scales(&h).len(), 1);
}

#[test]
fn the_legend_menu_enables_the_scales_that_apply() {
    use crate::menus::{Cmd, Entry, graph_legend_menu};
    let enabled = |can_color, can_bubble| {
        let items: Vec<(String, bool)> = graph_legend_menu(0, can_color, can_bubble)
            .into_iter()
            .filter_map(|e| match e {
                Entry::Item(i) if i.label.contains("Scale") => {
                    Some((i.label, matches!(i.cmd, Some(Cmd::GraphEdit(_)))))
                }
                _ => None,
            })
            .collect();
        items
    };
    assert_eq!(enabled(false, false), vec![("Add Color Scale".into(), false), ("Add Bubble Scale".into(), false)]);
    assert_eq!(enabled(true, false), vec![("Add Color Scale".into(), true), ("Add Bubble Scale".into(), false)]);
    assert_eq!(enabled(false, true), vec![("Add Color Scale".into(), false), ("Add Bubble Scale".into(), true)]);
}
