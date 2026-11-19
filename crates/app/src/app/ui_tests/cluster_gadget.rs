//! Gadgets > Cluster (T146, graphing "Cluster gadget").

use super::*;
use crate::gadget::GadgetKind;

#[test]
fn the_cluster_gadget_counts_the_points_inside_its_rectangle() {
    // The sample's "up" column: (1, 2.5), (2, 4.5), (3, 6.5), (4, 8.5).
    let mut h = start(vec![data_file("cluster-gadget")]);
    let ctx = h.ctx.clone();
    let graph = project(&h).graphs().next().unwrap().id;
    h.state_mut().apply(Action::StartCluster { graph }, &ctx);
    settle(&mut h);
    let mut g = h.state().graphs[&graph].gadget.expect("the gadget is open");
    assert_eq!(g.kind, GadgetKind::Cluster);
    assert!(g.y0 != g.y1, "the rectangle has a height");
    (g.x0, g.x1, g.y0, g.y1) = (1.5, 4.5, 4.0, 7.0);
    let (x, y) = crate::gadget::plot_xy(project(&h), graph, 0, 0).unwrap();
    let c = g.cluster(&x, &y).unwrap();
    assert_eq!((c.n, c.mean_x, c.mean_y), (2, 2.5, 5.5), "(2, 4.5) and (3, 6.5)");
    h.state_mut().graphs.get_mut(&graph).unwrap().gadget = Some(g);
    settle(&mut h);
    (g.y0, g.y1) = (20.0, 30.0);
    assert!(g.cluster(&x, &y).is_none());
}
