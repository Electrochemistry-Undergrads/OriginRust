//! Underline, Superscript, Subscript, Greek, Fill Color and Line Color on selected graph objects.
use super::*;
use crate::object_format::{Change, Markup};
use sheaf_core::Color;
use sheaf_core::graph::{Font, TextLabel};
use sheaf_core::shape::{Shape, ShapeKind};

const RED: Color = Color::rgb(255, 0, 0);
const BLUE: Color = Color::rgb(0, 0, 255);

fn setup(tag: &str) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(tag)]);
    let id = graph(&h).id;
    let title = project(&h).windows.iter().find(|w| w.id() == id).unwrap().title();
    show_window(&mut h, &title);
    h.state_mut().doc.apply("Seed", |p| {
        let l = &mut p.graph_mut(id).unwrap().layers[0];
        l.labels.push(TextLabel::new("cm-2", 0.2, 0.2, Font::new("Arial", 18.0)));
        l.x_axis.title = Some("Time (s)".into());
        l.y_axis.title = None;
        let mut rect = Shape::new(ShapeKind::Rectangle, (0.2, 0.2), (0.4, 0.4));
        rect.fill = None;
        l.shapes = vec![rect, Shape::new(ShapeKind::Line, (0.5, 0.5), (0.8, 0.8))];
        l.legend.visible = true;
    });
    settle(&mut h);
    h
}

fn sel(h: &mut Harness<'static, SheafApp>, object: ObjectKind) {
    let id = graph(h).id;
    h.state_mut().graphs.get_mut(&id).unwrap().selected = Some((0, object));
    settle(h);
}

fn disabled(h: &Harness<'static, SheafApp>, name: &str) -> bool {
    h.get_by_label(&format!("Format toolbar: {name}")).accesskit_node().is_disabled()
}

const MARKUP: [(&str, &str); 4] =
    [("Underline", "\\u("), ("Superscript", "\\+("), ("Subscript", "\\-("), ("Greek", "\\g(")];

#[test]
fn each_markup_button_wraps_a_label_and_toggles_back() {
    let mut h = setup("fbr-markup");
    sel(&mut h, ObjectKind::Label(0));
    let before = project(&h).clone();
    for (name, prefix) in MARKUP {
        click(&mut h, &format!("Format toolbar: {name}"));
        assert_eq!(graph(&h).layers[0].labels[0].text, format!("{prefix}cm-2)"), "{name}");
        assert_eq!(h.state().doc.undo_label(), Some("Format graph text"));
        click(&mut h, &format!("Format toolbar: {name}"));
        assert_eq!(graph(&h).layers[0].labels[0].text, "cm-2", "{name} toggles off");
    }
    for _ in 0..8 {
        menu(&mut h, "Edit", "Undo");
    }
    assert_eq!(project(&h), &before);
}

#[test]
fn explicit_axis_titles_wrap_and_one_step_undoes() {
    let mut h = setup("fbr-title");
    sel(&mut h, ObjectKind::XTitle);
    let before = project(&h).clone();
    click(&mut h, "Format toolbar: Subscript");
    assert_eq!(graph(&h).layers[0].x_axis.title.as_deref(), Some("\\-(Time (s))"));
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
}

#[test]
fn markup_is_disabled_without_text_of_its_own() {
    let mut h = setup("fbr-disabled");
    for object in [ObjectKind::YTitle, ObjectKind::Legend, ObjectKind::Plot(0)] {
        sel(&mut h, object);
        for (name, _) in MARKUP {
            assert!(disabled(&h, name), "{name} on {object:?}");
        }
    }
    let id = graph(&h).id;
    h.state_mut().graphs.get_mut(&id).unwrap().selected = None;
    settle(&mut h);
    for (name, _) in MARKUP {
        assert!(disabled(&h, name), "{name} with nothing selected");
    }
    assert!(disabled(&h, "Fill Color") && disabled(&h, "Line Color"));
    h.state_mut().doc.apply("Empty", |p| p.graph_mut(id).unwrap().layers[0].x_axis.title = Some(String::new()));
    sel(&mut h, ObjectKind::XTitle);
    assert!(disabled(&h, "Underline"));
}

#[test]
fn a_stale_request_is_rejected_without_history() {
    let mut h = setup("fbr-stale");
    sel(&mut h, ObjectKind::Label(0));
    let request = h.state().object_format().unwrap().request(Change::Markup(Markup::Greek));
    h.state_mut().doc.apply("Other change", |p| p.graph_mut(graph_id(&p.clone())).unwrap().name.push('x'));
    let changed = project(&h).clone();
    h.state_mut().apply(Action::FormatObject(request), &egui::Context::default());
    assert_eq!(project(&h), &changed);
    assert_eq!(h.state().doc.undo_label(), Some("Other change"));
    // The same request for a different selection is also ignored.
    let request = h.state().object_format().unwrap().request(Change::Markup(Markup::Greek));
    sel(&mut h, ObjectKind::XTitle);
    h.state_mut().apply(Action::FormatObject(request), &egui::Context::default());
    assert_eq!(project(&h), &changed);
}

fn graph_id(project: &Project) -> WindowId {
    project.graphs().next().unwrap().id
}

#[test]
fn fill_color_sets_and_clears_a_rectangle_and_keeps_it_selected() {
    let mut h = setup("fbr-fill");
    sel(&mut h, ObjectKind::Shape(0));
    let before = project(&h).clone();
    click(&mut h, "Format toolbar: Fill Color");
    click(&mut h, "Fill Color: Red");
    assert_eq!(graph(&h).layers[0].shapes[0].fill, Some(RED));
    assert_eq!(h.state().doc.undo_label(), Some("Format graph object"));
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Shape(0))));
    click(&mut h, "Format toolbar: Fill Color");
    click(&mut h, "Fill Color: None");
    assert_eq!(graph(&h).layers[0].shapes[0].fill, None);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Shape(0))));
    menu(&mut h, "Edit", "Undo");
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
}

#[test]
fn fill_is_disabled_on_a_line_shape_but_line_color_works() {
    let mut h = setup("fbr-line");
    sel(&mut h, ObjectKind::Shape(1));
    assert!(disabled(&h, "Fill Color"));
    assert!(!disabled(&h, "Line Color"));
    click(&mut h, "Format toolbar: Line Color");
    click(&mut h, "Line Color: Blue");
    assert_eq!(graph(&h).layers[0].shapes[1].color, BLUE);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Shape(1))));
    sel(&mut h, ObjectKind::Shape(0));
    click(&mut h, "Format toolbar: Line Color");
    click(&mut h, "Line Color: Red");
    assert_eq!(graph(&h).layers[0].shapes[0].color, RED);
    sel(&mut h, ObjectKind::Label(0));
    assert!(disabled(&h, "Line Color") && disabled(&h, "Fill Color"));
}

#[test]
fn legend_background_takes_fill_color_but_not_line_color() {
    let mut h = setup("fbr-legend");
    sel(&mut h, ObjectKind::Legend);
    assert!(disabled(&h, "Line Color"));
    click(&mut h, "Format toolbar: Fill Color");
    click(&mut h, "Fill Color: Blue");
    assert_eq!(graph(&h).layers[0].legend.background, Some(BLUE));
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Legend)));
}

#[test]
fn formatted_text_and_shape_colours_survive_both_file_formats() {
    let mut h = setup("fbr-save");
    sel(&mut h, ObjectKind::Label(0));
    click(&mut h, "Format toolbar: Superscript");
    sel(&mut h, ObjectKind::Shape(0));
    click(&mut h, "Format toolbar: Fill Color");
    click(&mut h, "Fill Color: Red");
    click(&mut h, "Format toolbar: Line Color");
    click(&mut h, "Line Color: Blue");
    let layer = graph(&h).layers[0].clone();
    let dir = save_dir("fbr-save");
    for extension in ["sheaf", "opj"] {
        let file = dir.join(format!("formatted.{extension}"));
        h.state_mut().save_to(file.clone());
        let back = start(vec![file]);
        let got = &graph(&back).layers[0];
        assert_eq!(got.labels[0].text, "\\+(cm-2)", "{extension}");
        assert_eq!(got.shapes[0].fill, layer.shapes[0].fill, "{extension}");
        assert_eq!(got.shapes[0].color, layer.shapes[0].color, "{extension}");
    }
}
