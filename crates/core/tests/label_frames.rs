use sheaf_core::Color;
use sheaf_core::graph::{DataLabels, LabelFrame};

#[test]
fn old_labels_keep_their_saved_shape_and_frame_settings_round_trip() {
    let defaults = DataLabels::default();
    let old = serde_json::to_string(&defaults).unwrap();
    for name in ["frame", "margin_percent", "frame_border", "frame_fill"] {
        assert!(!old.contains(name), "default field {name} is omitted");
    }
    let loaded: DataLabels = serde_json::from_str(&old).unwrap();
    assert_eq!(loaded, defaults);
    assert_eq!(loaded.frame, LabelFrame::None);
    assert_eq!(loaded.margin_percent, 0.0);
    assert_eq!((loaded.frame_border, loaded.frame_fill), (Color::BLACK, Color::BLACK));
    for frame in [LabelFrame::Box, LabelFrame::Shadow] {
        let labels = DataLabels {
            frame,
            margin_percent: 20.0,
            frame_border: Color::RED,
            frame_fill: Color::TRANSPARENT,
            ..defaults.clone()
        };
        assert!(!labels.is_default(), "disabled labels still retain their styling");
        assert_eq!(serde_json::from_str::<DataLabels>(&serde_json::to_string(&labels).unwrap()).unwrap(), labels);
    }
}
