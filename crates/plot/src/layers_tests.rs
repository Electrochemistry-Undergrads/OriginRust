//! Layers added with Graph > New Layer(Axes) (T61, graphing spec section 21): an inset golden image
//! and the geometry of the other kinds.

use super::*;
use sheaf_core::layers::NewLayerKind;
use sheaf_core::template::Template;

/// A line graph with one curve and an inset-with-data layer in its top right corner.
fn with_inset(kind: NewLayerKind) -> (Project, WindowId) {
    let (x, shape) = super::tests::golden_data();
    let (mut p, g) = super::tests::project(Template::Line, x, vec![shape(30.0)]);
    p.graph_mut(g).unwrap().add_layer(kind, 0).unwrap();
    (p, g)
}

#[test]
fn golden_inset_with_data() {
    let (p, g) = with_inset(NewLayerKind::InsetWithData);
    super::tests::compare_golden("inset-with-data", &super::tests::svg_of(&p, g));
}

#[test]
fn an_inset_lies_in_the_top_right_quarter_of_its_parent_and_draws_its_own_axes() {
    let (p, g) = with_inset(NewLayerKind::InsetWithData);
    let graph = p.graph(g).unwrap();
    let pds: Vec<data::LayerData> = graph.layers.iter().map(|l| data::collect(&p, l)).collect();
    let geoms = layout::graph_geometry(&p, graph, &pds, &FixedMetrics);
    assert_eq!(geoms.len(), 2);
    let (a, b) = (geoms[0].frame, geoms[1].frame);
    let near = |x: f64, y: f64| (x - y).abs() < 1e-6;
    assert!(near(b.w, a.w * 0.25) && near(b.h, a.h * 0.25), "{a:?} {b:?}");
    assert!(near(b.x, a.x + a.w * 0.75) && near(b.y, a.y), "{a:?} {b:?}");
}

#[test]
fn every_kind_of_layer_renders() {
    for kind in NewLayerKind::ALL {
        let (p, g) = with_inset(kind);
        let svg = super::tests::svg_of(&p, g);
        assert!(svg.contains("<svg"), "{kind:?}");
    }
}
