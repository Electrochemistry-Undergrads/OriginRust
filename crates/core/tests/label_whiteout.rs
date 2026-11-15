use sheaf_core::graph::DataLabels;

#[test]
fn old_data_label_settings_load_with_whiteout_off_and_enabled_whiteout_round_trips() {
    let old = r##"{"enable":true,"font":{"family":"Arial","size_pt":22.0,"color":"#000000"}}"##;
    let mut labels: DataLabels = serde_json::from_str(old).unwrap();
    assert!(labels.enable && !labels.white_out);
    assert!(!serde_json::to_string(&labels).unwrap().contains("white_out"));
    labels.white_out = true;
    let saved = serde_json::to_string(&labels).unwrap();
    assert!(saved.contains(r#""white_out":true"#));
    assert_eq!(serde_json::from_str::<DataLabels>(&saved).unwrap(), labels);
}
