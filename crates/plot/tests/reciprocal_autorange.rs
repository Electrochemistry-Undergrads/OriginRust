use sheaf_core::Project;
use sheaf_core::graph::{Axis, ScaleKind};
use sheaf_core::template::Template;
use sheaf_plot::ticks::resolve_axis;

fn reciprocal_axis() -> Axis {
    let mut project = Project::new();
    let graph = project.add_graph(Template::Line);
    let mut axis = project.graph(graph).unwrap().layers[0].y_axis.clone();
    axis.scale = ScaleKind::Reciprocal;
    axis
}

#[test]
fn narrow_reciprocal_axes_show_the_reference_increment_and_labels() {
    let axis = reciprocal_axis();
    let small = resolve_axis(&axis, Some((2.0, 3.0)));
    assert_eq!((small.min, small.max, small.step), (1.75, 3.25, 0.5));
    assert_eq!(small.major, [2.0, 2.5, 3.0]);
    assert_eq!(small.labels, ["2.0", "2.5", "3.0"]);
    let translated = resolve_axis(&axis, Some((1000.0, 1001.0)));
    assert_eq!((translated.min, translated.max, translated.step), (999.75, 1001.25, 0.5));
    assert_eq!(translated.labels, ["1000.0", "1000.5", "1001.0"]);
}

#[test]
fn an_explicit_reciprocal_increment_stays_in_control() {
    let mut axis = reciprocal_axis();
    axis.major_step = Some(0.25);
    let resolved = resolve_axis(&axis, Some((2.0, 3.0)));
    assert_eq!(resolved.step, 0.25);
    assert!(resolved.major.contains(&2.25));
}
