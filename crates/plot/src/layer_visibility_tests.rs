//! Origin 2016 layer.show / layer -h controls, graphing 21.13.
use super::*;
use sheaf_core::graph::LegendScope;
use sheaf_core::layers::NewLayerKind;
use sheaf_core::template::Template;

fn fixture() -> (Project, WindowId) {
    let (mut p, g) = tests::project(Template::Line, vec![0., 1., 2.], vec![vec![0., 3., 1.]]);
    p.graph_mut(g).unwrap().add_layer(NewLayerKind::InsetWithData, 0).unwrap();
    (p, g)
}

#[test]
fn hidden_layers_draw_no_elements_and_leave_linked_geometry_and_other_layers_unchanged() {
    for hidden in 0..2 {
        let (mut p, g) = fixture();
        let before = layout::build_scene(&p, g, &FixedMetrics).unwrap();
        let geometry = layout_geometry(&p, g).unwrap();
        let original = p.graph(g).unwrap().clone();
        p.graph_mut(g).unwrap().layers[hidden].display.hidden = true;
        let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
        assert_eq!(scene.layers.len(), 2);
        let l = &scene.layers[hidden];
        assert!(l.grid.is_empty() && l.plots.is_empty() && l.overlay.is_empty() && l.objects.is_empty());
        assert_eq!(scene.layers[1 - hidden], before.layers[1 - hidden]);
        for (a, b) in geometry.iter().zip(layout_geometry(&p, g).unwrap()) {
            assert_eq!(
                (a.frame, a.clip, a.x.min, a.x.max, a.y.min, a.y.max),
                (b.frame, b.clip, b.x.min, b.x.max, b.y.min, b.y.max)
            );
        }
        let frame = geometry[hidden].frame;
        for at in [(frame.x, frame.y), (frame.x + frame.w / 2., frame.y + frame.h / 2.)] {
            assert!(
                !matches!(hit::hit_test(&scene, at, 0.), hit::Hit::Layer(i) | hit::Hit::Object {layer: i, ..} if i == hidden)
            );
        }
        p.graph_mut(g).unwrap().layers[hidden].display.hidden = false;
        assert_eq!(p.graph(g).unwrap(), &original);
        assert_eq!(layout::build_scene(&p, g, &FixedMetrics).unwrap(), before);
    }
}

#[test]
fn graph_legends_follow_their_owner_and_keep_entries_for_a_hidden_sibling() {
    let (mut p, g) = tests::project(Template::Line, vec![0., 1., 2.], vec![vec![0., 3., 1.], vec![8., 4., 6.]]);
    let graph = p.graph_mut(g).unwrap();
    let right = graph.add_layer(NewLayerKind::RightY, 0).unwrap();
    graph.move_plot_to_layer(0, 1, right).unwrap();
    graph.layers[0].legend.scope = LegendScope::Graph;
    let legend_items = |s: &scene::Scene| -> Vec<scene::Item> {
        s.layers
            .iter()
            .flat_map(|l| {
                l.objects
                    .iter()
                    .filter(|o| o.kind == scene::ObjectKind::Legend && o.layer == 0)
                    .flat_map(|o| l.overlay[o.items.clone()].iter().cloned())
            })
            .collect()
    };
    let original_legend = legend_items(&layout::build_scene(&p, g, &FixedMetrics).unwrap());
    assert!(!original_legend.is_empty());
    p.graph_mut(g).unwrap().layers[1].display.hidden = true;
    assert_eq!(legend_items(&layout::build_scene(&p, g, &FixedMetrics).unwrap()), original_legend);
    p.graph_mut(g).unwrap().layers[0].display.hidden = true;
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    assert!(legend_items(&scene).is_empty());
    for at in [(0., 0.), (scene.width / 2., scene.height / 2.)] {
        assert_eq!(hit::hit_test(&scene, at, 0.), hit::Hit::Page);
    }
    let raster = render_raster(&p, g, 0.25).unwrap();
    assert!(raster.rgba.chunks_exact(4).all(|px| px == [255, 255, 255, 255]));
    // The zero-size clip used by a hidden layer also remains valid export input.
    assert!(render_pdf(&p, g).unwrap().starts_with(b"%PDF"));
    assert!(render_eps(&p, g).unwrap().starts_with(b"%!PS"));
    assert!(!render_emf(&p, g).unwrap().is_empty());
}
