use super::*;
use sheaf_core::graph::Space3dScale;
use sheaf_core::template::Template;

fn apply_cmd(h: &mut Harness<'static, SheafApp>, cmd: menus::Cmd) {
    let action = menus::resolve(&cmd, &h.state().menu_context()).unwrap();
    let ctx = h.ctx.clone();
    h.state_mut().apply(action, &ctx);
    settle(h);
}

#[test]
fn z_axis_opens_the_3d_axis_window_on_a_3d_layer_only_and_sets_the_scale_in_one_step() {
    // T164: Format > Axes > Z Axis... is greyed on a 2D layer.
    let mut h = start(vec![data_file("axis3d")]);
    assert!(menus::resolve(&menus::Cmd::ZAxis, &h.state().menu_context()).is_err());
    show_window(&mut h, "Book1 - sample.csv");
    click_text(&mut h, "B(Y1)");
    apply_cmd(&mut h, menus::Cmd::Plot(Template::Scatter3d));
    let g = project(&h).graphs().last().unwrap().id;
    // On the 3D layer it opens the 3D Axis window on Z.
    apply_cmd(&mut h, menus::Cmd::ZAxis);
    assert_eq!(h.state().axis3d.as_ref().map(|d| d.dim), Some(2));
    assert!(h.query_by_label("Increment").is_some());
    let z = Space3dScale { from: Some(-10.0), to: Some(10.0), step: Some(5.0) };
    h.state_mut().axis3d.as_mut().unwrap().scales[2] = z;
    click(&mut h, "OK");
    assert!(h.state().axis3d.is_none());
    let scales =
        |h: &Harness<'static, SheafApp>| project(h).graph(g).unwrap().layers[0].coords.space3d().unwrap().scales;
    assert_eq!(scales(&h)[2], z);
    assert_eq!(h.state().doc.undo_label(), Some("3D Axis"));
    menu(&mut h, "Edit", "Undo 3D Axis");
    assert_eq!(scales(&h)[2], Space3dScale::default());
    // T172: the window also turns and tilts the view, in the same undo step.
    apply_cmd(&mut h, menus::Cmd::ZAxis);
    assert!(h.query_by_label("Turn").is_some() && h.query_by_label("Tilt").is_some());
    {
        let d = h.state_mut().axis3d.as_mut().unwrap();
        (d.azimuth, d.elevation) = (-60.0, 70.0);
    }
    click(&mut h, "OK");
    let view = |h: &Harness<'static, SheafApp>| {
        let c = *project(h).graph(g).unwrap().layers[0].coords.space3d().unwrap();
        (c.azimuth_deg, c.elevation_deg)
    };
    assert_eq!(view(&h), (-60.0, 70.0));
    menu(&mut h, "Edit", "Undo 3D Axis");
    assert_eq!(view(&h), (40.0, 25.0));
}

#[test]
fn x_and_y_axis_on_a_3d_layer_open_the_3d_axis_window() {
    let mut h = start(vec![data_file("axis3d-xy")]);
    show_window(&mut h, "Book1 - sample.csv");
    click_text(&mut h, "B(Y1)");
    apply_cmd(&mut h, menus::Cmd::Plot(Template::Scatter3d));
    menu_path(&mut h, &["Format", "Axes", "Y Axis..."]);
    assert!(h.state().graph_props.is_none(), "not the 2D axis dialog: the layer's own axes are the view's frame");
    assert_eq!(h.state().axis3d.as_ref().map(|d| d.dim), Some(1));
    // A From above To is refused.
    h.state_mut().axis3d.as_mut().unwrap().scales[1] = Space3dScale { from: Some(3.0), to: Some(1.0), step: None };
    let revision = h.state().doc.revision();
    click(&mut h, "OK");
    assert!(h.state().axis3d.is_some());
    assert_eq!(h.state().doc.revision(), revision);
    click(&mut h, "Cancel");
    assert!(h.state().axis3d.is_none());
    menu_path(&mut h, &["Format", "Axes", "X Axis..."]);
    assert_eq!(h.state().axis3d.as_ref().map(|d| d.dim), Some(0));
}
