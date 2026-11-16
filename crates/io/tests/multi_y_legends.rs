use sheaf_core::graph::LegendPosition;
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation, Project};

fn assert_anchor(actual: LegendPosition, expected: LegendPosition) {
    let LegendPosition::ManualTopLeft { x, y } = actual else { panic!("wrong anchor: {actual:?}") };
    let LegendPosition::ManualTopLeft { x: ex, y: ey } = expected else { panic!() };
    // The JSON reader can choose the adjacent representable f64. Keep a one-ULP bound,
    // many orders below a device pixel, while requiring the saved anchor kind exactly.
    assert!(x.to_bits().abs_diff(ex.to_bits()) <= 1 && y.to_bits().abs_diff(ey.to_bits()) <= 1);
}

#[test]
fn multi_y_legend_native_anchors_round_trip_with_a_warning_for_the_origin_fixed_position() {
    let mut p = Project::new();
    let b = p.add_workbook("synthetic");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0])).unwrap();
    let ys: Vec<_> = (0..3)
        .map(|i| p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![i as f64, 1.0 + i as f64])).unwrap())
        .collect();
    let g = p.plot_columns(Template::Multi3YYy, &ys).unwrap();
    let anchor = p.graph(g).unwrap().layers[0].legend.position;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let dir = std::env::temp_dir().join(format!("sheaf-native-legend-anchor-{}-{stamp}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let native = sheaf_io::save::save_project(&p, &dir.join("anchor.sheaf")).unwrap();
    let opened = sheaf_io::open_project(&native.path).unwrap();
    assert_anchor(opened.project.graphs().next().unwrap().layers[0].legend.position, anchor);
    let origin = sheaf_io::save::save_project(&p, &dir.join("anchor.opj")).unwrap();
    assert!(origin.warnings.iter().any(|w| w.contains("top-left legend anchor")), "{:?}", origin.warnings);
    let opened = sheaf_io::open_project(&origin.path).unwrap();
    assert_anchor(opened.project.graphs().next().unwrap().layers[0].legend.position, anchor);
    let companion = origin.companion.unwrap();
    std::fs::rename(&companion, dir.join("saved-companion-backup")).unwrap();
    let bare = sheaf_io::open_project(&origin.path).unwrap();
    let LegendPosition::ManualTopLeft { x: ax, y: ay } = anchor else { panic!() };
    let LegendPosition::Manual { x, y } = bare.project.graphs().next().unwrap().layers[0].legend.position else {
        panic!()
    };
    assert!(x > ax && y > ay, "Native rectangle centre must be below/right of its top-left: {x},{y}");
}
