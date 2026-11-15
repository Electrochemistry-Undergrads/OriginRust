use sheaf_core::graph::LayerDisplay;

#[test]
fn data_on_top_loads_old_settings_and_round_trips_when_enabled() {
    // Existing projects can have a Display object without the new field.
    let old = r#"{"show_data":false,"clip_data":true,"grid_on_top":true}"#;
    let mut display: LayerDisplay = serde_json::from_str(old).unwrap();
    assert!(!display.data_on_top);
    assert!(!display.show_data && display.grid_on_top);
    assert!(!serde_json::to_string(&display).unwrap().contains("data_on_top"));

    display.data_on_top = true;
    let json = serde_json::to_string(&display).unwrap();
    assert!(json.contains(r#""data_on_top":true"#));
    assert_eq!(serde_json::from_str::<LayerDisplay>(&json).unwrap(), display);
    assert!(!LayerDisplay::default().data_on_top);
}
