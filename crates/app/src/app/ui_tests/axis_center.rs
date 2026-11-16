use super::*;

fn geometry(h: &Harness<'static, SheafApp>) -> sheaf_plot::LayerGeometry {
    let id = *h.state().graphs.keys().next().expect("a graph window");
    sheaf_plot::layout_geometry(h.state().doc.project(), id).unwrap().remove(0)
}

fn close(a: f64, b: f64, span: f64) -> bool {
    (a - b).abs() < 1e-3 * span
}

#[test]
fn center_on_x_keeps_the_span_and_puts_the_clicked_value_in_the_middle() {
    let mut h = start(vec![data_file("axcenter-x")]);
    let g = geometry(&h);
    let (lo, hi) = (g.x.min, g.x.max);
    // Right-click on the X axis at four fifths of its length.
    let value = g.x.from_unit(0.8);
    let on_axis = on(&h, 0, ObjectKind::XAxis);
    let at = egui::pos2(graph_ui(&h).page_point(g.frame.x + 0.8 * g.frame.w, 0.0).unwrap().x, on_axis.y);
    right_click_at(&mut h, at);
    click(&mut h, "Center");
    let l = &graph(&h).layers[0];
    let (a, b) = (l.x_axis.min.expect("fixed"), l.x_axis.max.expect("fixed"));
    assert!(close(b - a, hi - lo, hi - lo), "span kept: {a}..{b} from {lo}..{hi}");
    assert!(close((a + b) / 2.0, value, hi - lo), "middle {} is {value}", (a + b) / 2.0);
    assert!(l.y_axis.min.is_none() && l.y_axis.max.is_none(), "the other axis is untouched");
    assert_eq!(h.state().doc.undo_label(), Some("Axis center"));
    menu(&mut h, "Edit", "Undo Axis center");
    assert!(graph(&h).layers[0].x_axis.min.is_none());
}

#[test]
fn center_on_y_keeps_the_span_and_puts_the_clicked_value_in_the_middle() {
    let mut h = start(vec![data_file("axcenter-y")]);
    let g = geometry(&h);
    let (lo, hi) = (g.y.min, g.y.max);
    // Three tenths of the frame's height down from the top is seven tenths up the axis.
    let value = g.y.from_unit(0.7);
    let on_axis = on(&h, 0, ObjectKind::YAxis);
    let at = egui::pos2(on_axis.x, graph_ui(&h).page_point(0.0, g.frame.y + 0.3 * g.frame.h).unwrap().y);
    right_click_at(&mut h, at);
    click(&mut h, "Center");
    let l = &graph(&h).layers[0];
    let (a, b) = (l.y_axis.min.expect("fixed"), l.y_axis.max.expect("fixed"));
    assert!(close(b - a, hi - lo, hi - lo), "span kept: {a}..{b} from {lo}..{hi}");
    assert!(close((a + b) / 2.0, value, hi - lo), "middle {} is {value}", (a + b) / 2.0);
    assert!(l.x_axis.min.is_none() && l.x_axis.max.is_none(), "the other axis is untouched");
    assert_eq!(h.state().doc.undo_label(), Some("Axis center"));
    h.state_mut().doc.undo();
    assert!(graph(&h).layers[0].y_axis.min.is_none());
}

#[test]
fn fit_page_to_printer_sets_the_fallback_size_and_keeps_the_frames() {
    let mut h = start(vec![data_file("fitprinter")]);
    let g = graph(&h);
    let (page, frames) = ((g.page.width_pt, g.page.height_pt), g.layers.iter().map(|l| l.frame).collect::<Vec<_>>());
    let expect = h.state().doc.project().page_setup.fallback_printable_pt(page.0, page.1);
    assert!(page.0 > page.1, "a landscape page: Letter landscape is 768 x 576 pt");
    menu_path(&mut h, &["Format", "Fit Page To Printer"]);
    let g = graph(&h);
    assert_eq!((g.page.width_pt, g.page.height_pt), expect);
    assert_eq!(expect, (768.0, 576.0));
    assert_eq!(g.layers.iter().map(|l| l.frame).collect::<Vec<_>>(), frames);
    assert_eq!(h.state().doc.undo_label(), Some("Fit Page to Printer"));
    menu(&mut h, "Edit", "Undo Fit Page to Printer");
    let g = graph(&h);
    assert_eq!((g.page.width_pt, g.page.height_pt), page);
}

#[test]
fn center_keeps_an_axis_reversed_by_its_limits_reversed() {
    let mut h = start(vec![data_file("axcenter-rev")]);
    let id = *h.state().graphs.keys().next().unwrap();
    h.state_mut().doc.apply("reverse", |p| {
        let a = &mut p.graph_mut(id).unwrap().layers[0].x_axis;
        a.min = Some(10.0);
        a.max = Some(0.0);
    });
    settle(&mut h);
    let g = geometry(&h);
    let value = g.x.from_unit(0.8);
    let on_axis = on(&h, 0, ObjectKind::XAxis);
    let at = egui::pos2(graph_ui(&h).page_point(g.frame.x + 0.8 * g.frame.w, 0.0).unwrap().x, on_axis.y);
    right_click_at(&mut h, at);
    click(&mut h, "Center");
    let a = &graph(&h).layers[0].x_axis;
    let (mn, mx) = (a.min.unwrap(), a.max.unwrap());
    assert!(mn > mx, "still reversed: {mn}..{mx}");
    assert!(close(mn - mx, 10.0, 10.0) && close((mn + mx) / 2.0, value, 10.0), "{mn}..{mx}, middle {value}");
}
