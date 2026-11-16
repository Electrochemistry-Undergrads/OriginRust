//! A plot's right-click menu: Change X, Change Y and Ungroup (T73).
use super::*;

const A: &str = "[Book1]sample!A(X)";
const B: &str = "[Book1]sample!B(Y)";
const C: &str = "[Book1]sample!C(Y)";

fn open_plot_menu(h: &mut Harness<'static, SheafApp>, plot: usize) {
    let at = on(h, 0, ObjectKind::Plot(plot));
    click_at(h, at);
    right_click_at(h, at);
}

#[test]
fn change_y_replaces_the_y_column_and_leaves_the_axis_ranges() {
    let mut h = start(vec![data_file("chy")]);
    let before = graph(&h).clone();
    let (b, c) = (before.layers[0].plots[0].y, before.layers[0].plots[1].y);
    let x = before.layers[0].plots[0].x;
    assert_eq!(project(&h).plot_label(&before.layers[0].plots[0]), "up");
    let id = *h.state().graphs.keys().next().unwrap();
    let geo = sheaf_plot::layout_geometry(project(&h), id).unwrap().remove(0);
    let drawn_y = (geo.y.from_unit(0.0).min(geo.y.from_unit(1.0)), geo.y.from_unit(0.0).max(geo.y.from_unit(1.0)));
    open_plot_menu(&mut h, 0);
    menu_path(&mut h, &["Change Y"]);
    // The column the plot uses on the other axis is greyed, its own is ticked.
    if x.is_some() {
        assert!(is_disabled(&h, A));
    }
    assert!(has_label(&h, &format!("\u{2713} {B}")), "the plot's own column is ticked");
    menu_path(&mut h, &[C]);
    let g = graph(&h);
    assert_eq!(g.layers[0].plots[0].y, c);
    assert_eq!(g.layers[0].plots[1].y, c, "the other plot is untouched");
    assert_ne!(b, c);
    assert_eq!(project(&h).plot_label(&g.layers[0].plots[0]), "down", "the legend follows");
    // the Y axis keeps the range it was drawn with (fixed now); X is untouched
    let (lo, hi) = (g.layers[0].y_axis.min.expect("fixed"), g.layers[0].y_axis.max.expect("fixed"));
    assert_eq!((lo, hi), drawn_y, "the Y axis keeps the range it was drawn with");
    assert_eq!((g.layers[0].x_axis.min, g.layers[0].x_axis.max), (None, None));
    assert_eq!(h.state().doc.undo_label(), Some("Change Y"));
    menu(&mut h, "Edit", "Undo Change Y");
    assert_eq!(*graph(&h), before);
}

#[test]
fn change_x_sets_the_x_column_and_undo_restores() {
    let mut h = start(vec![data_file("chx")]);
    let before = graph(&h).clone();
    let y = before.layers[0].plots[0].y;
    open_plot_menu(&mut h, 0);
    menu_path(&mut h, &["Change X"]);
    assert!(is_disabled(&h, B), "the Y column is greyed in Change X");
    menu_path(&mut h, &[C]);
    let p = &graph(&h).layers[0].plots[0];
    assert_eq!((p.x, p.y), (Some(project(&h).workbooks().next().unwrap().sheet().column_by_name("C").unwrap().id), y));
    assert_eq!(h.state().doc.undo_label(), Some("Change X"));
    assert!(graph(&h).layers[0].x_axis.min.is_some(), "the X axis keeps its drawn range (fixed)");
    menu(&mut h, "Edit", "Undo Change X");
    assert_eq!(*graph(&h), before);
}

fn group_both(h: &mut Harness<'static, SheafApp>) {
    h.state_mut().doc.apply("Group", |p| {
        let id = p.graphs().next().unwrap().id;
        for pl in &mut p.graph_mut(id).unwrap().layers[0].plots {
            pl.group = Some(1);
        }
    });
    settle(h);
}

#[test]
fn a_grouped_plot_greys_change_xy_and_ungroup_keeps_the_colours() {
    let mut h = start(vec![data_file("chgrp")]);
    group_both(&mut h);
    let colors: Vec<_> = graph(&h).layers[0].plots.iter().map(|p| (p.line.color, p.symbol.color)).collect();
    open_plot_menu(&mut h, 0);
    assert!(is_disabled(&h, "Change X") && is_disabled(&h, "Change Y"));
    assert!(!is_disabled(&h, "Ungroup"));
    click(&mut h, "Ungroup");
    let g = graph(&h);
    assert!(g.layers[0].plots.iter().all(|p| p.group.is_none()));
    let after: Vec<_> = g.layers[0].plots.iter().map(|p| (p.line.color, p.symbol.color)).collect();
    assert_eq!(colors, after);
    assert_eq!(h.state().doc.undo_label(), Some("Ungroup"));
    open_plot_menu(&mut h, 0);
    assert!(!is_disabled(&h, "Change X \u{23f5}") && !is_disabled(&h, "Change Y \u{23f5}"));
    assert!(is_disabled(&h, "Ungroup"), "no group left");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    menu(&mut h, "Edit", "Undo Ungroup");
    assert!(graph(&h).layers[0].plots.iter().all(|p| p.group == Some(1)));
}

#[test]
fn ungroup_is_greyed_on_a_plot_in_no_group() {
    let mut h = start(vec![data_file("chnog")]);
    open_plot_menu(&mut h, 0);
    assert!(is_disabled(&h, "Ungroup"));
}
