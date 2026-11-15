use sheaf_core::graph::{PlotOffset, Stack};
use sheaf_core::stack::{Place, places};
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, ColumnId, Designation, Project};

#[test]
fn old_offsets_load_with_identity_multipliers_and_keep_the_old_json_shape() {
    let old = r#"{"column":3,"x":5.0,"y":7.0}"#;
    let mut offset: PlotOffset = serde_json::from_str(old).unwrap();
    assert_eq!((offset.x_multiplier, offset.y_multiplier), (1.0, 1.0));
    assert_eq!(serde_json::to_string(&offset).unwrap(), old);
    assert_eq!(PlotOffset::new(ColumnId(3)).x_multiplier, 1.0);
    offset.x_multiplier = 0.5;
    offset.y_multiplier = 3.0;
    assert_eq!(serde_json::from_str::<PlotOffset>(&serde_json::to_string(&offset).unwrap()).unwrap(), offset);
    let identity = Place::default();
    assert_eq!(identity.x(-0.0).to_bits(), (-0.0_f64).to_bits());
    assert_eq!(identity.y(-0.0).to_bits(), (-0.0_f64).to_bits());
    assert_eq!(identity.source_x(-0.0).to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn individual_scaling_follows_the_leader_without_changing_source_data_or_other_modes() {
    let mut p = Project::new();
    let book = p.add_workbook("Data");
    let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![2.0, 4.0, 6.0])).unwrap();
    let fit = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![2.0, 4.0, 6.0])).unwrap();
    let g = p.add_graph(Template::Line);
    p.add_plot(g, 0, y).unwrap();
    p.add_plot(g, 0, fit).unwrap();
    let layer = &mut p.graph_mut(g).unwrap().layers[0];
    let o = layer.plot_offset_mut(y);
    (o.x, o.y, o.x_multiplier, o.y_multiplier) = (5.0, 7.0, 2.0, 3.0);
    layer.plot_offset_mut(fit).follows = Some(y);
    for mode in [Stack::None, Stack::Constant { step: 10.0 }, Stack::Auto { gap_percent: 8.0 }, Stack::Individual] {
        p.graph_mut(g).unwrap().layers[0].stack = mode;
        let (placed, count) = places(&p, &p.graph(g).unwrap().layers[0]);
        assert_eq!(count, 1);
        assert_eq!(placed[0], placed[1], "follower inherits the complete placement");
        if mode == Stack::Individual {
            assert_eq!((placed[0].x(1.0), placed[0].y(2.0)), (7.0, 13.0));
            assert_eq!(placed[0].source_x(7.0), 1.0);
        } else {
            assert_eq!((placed[0].mx, placed[0].my), (1.0, 1.0));
        }
    }
    assert_eq!(p.numeric(y).unwrap(), [2.0, 4.0, 6.0]);
    assert_eq!(p.numeric(fit).unwrap(), [2.0, 4.0, 6.0]);
}

#[test]
fn non_positive_multipliers_render_as_one_as_in_origin_2016() {
    let mut p = Project::new();
    let b = p.add_workbook("Data");
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![2.0])).unwrap();
    let g = p.add_graph(Template::Scatter);
    p.add_plot(g, 0, y).unwrap();
    p.graph_mut(g).unwrap().layers[0].stack = Stack::Individual;
    for multiplier in [-3.0, 0.0, f64::NAN, f64::INFINITY, 0.5] {
        let layer = &mut p.graph_mut(g).unwrap().layers[0];
        let o = layer.plot_offset_mut(y);
        (o.x, o.y, o.x_multiplier, o.y_multiplier) = (5.0, 7.0, multiplier, multiplier);
        let placed = places(&p, &p.graph(g).unwrap().layers[0]).0[0];
        let expected = if multiplier == 0.5 { (5.5, 8.0) } else { (6.0, 9.0) };
        assert_eq!((placed.x(1.0), placed.y(2.0)), expected);
    }
}
