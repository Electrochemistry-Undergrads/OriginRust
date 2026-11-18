//! Shared preferences do not bypass worksheet drafts, captured ranges or history.
use super::*;
use sheaf_core::Color;
use sheaf_core::cellformat::{CellFormat, FormatTarget, LabelKind};

const RGB: Color = Color::rgb(17, 131, 213);

fn profile(tag: &str) -> (PathBuf, PathBuf) {
    let data = data_file(tag);
    let file = data.parent().unwrap().join("custom-colors.json");
    (data, file)
}
fn seed(file: &Path) {
    std::fs::write(file, serde_json::to_vec(&serde_json::json!({"version":1,"colors":[RGB]})).unwrap()).unwrap();
}
fn start_with(data: &Path, file: &Path) -> Harness<'static, SheafApp> {
    let startup =
        Startup { paths: vec![data.to_path_buf()], custom_colors_path: Some(file.to_path_buf()), ..Default::default() };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_step_dt(0.02)
        .build_eframe(|cc| SheafApp::new(cc, startup));
    h.state_mut().panels.set_pinned(PanelId::Explorer, true);
    settle(&mut h);
    show_window(&mut h, "Book1 - sample.csv");
    click_text(&mut h, "4.5");
    h
}
fn custom(h: &mut Harness<'static, SheafApp>, fill: bool) {
    let prefix = if fill { "Fill Color" } else { "Font Color" };
    click(h, &format!("Format toolbar: {prefix}"));
    click(h, &format!("{prefix}: Custom..."));
}
fn choose(h: &mut Harness<'static, SheafApp>, prefix: &str) {
    click(h, &format!("{prefix}: Custom colors"));
    click(h, &format!("{prefix}: Custom 17,131,213"));
}
fn channel(h: &mut Harness<'static, SheafApp>, name: &str, value: &str) {
    click(h, &format!("Worksheet color: {name}"));
    ctrl(h, egui::Key::A);
    h.event(egui::Event::Text(value.to_string()));
    settle(h);
}
fn cells(h: &mut Harness<'static, SheafApp>) {
    menu(h, "Format", "Cell...");
    assert!(h.state().cell_format.is_some());
}

#[test]
fn worksheet_add_cancel_restart_and_text_fill_choices_preserve_values_and_history() {
    let (data, file) = profile("worksheet-wells-restart");
    let mut h = start_with(&data, &file);
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    custom(&mut h, false);
    for (name, value) in [("Red", "17"), ("Green", "131"), ("Blue", "213")] {
        channel(&mut h, name, value);
    }
    click(&mut h, "Worksheet color: Add to Custom");
    assert!(h.get_by_label("Worksheet color: Add to Custom").accesskit_node().is_disabled());
    assert_eq!(project(&h), &before);
    click(&mut h, "Cancel");
    assert_eq!(h.state().doc.revision(), revision);
    let saved = std::fs::read(&file).unwrap();
    drop(h);
    crate::custom_colors::initialize(None);
    let mut h = start_with(&data, &file);
    let b = sheet(&h).column_by_name("B").unwrap().id;
    let before = project(&h).clone();
    let columns = sheet(&h).columns.clone();
    for fill in [false, true] {
        custom(&mut h, fill);
        choose(&mut h, "Worksheet color");
        assert_eq!(project(&h), &before, "choosing a well is only a numeric draft");
        click(&mut h, "OK");
        let f = sheet(&h).cell_format_at(b, 1);
        assert_eq!(if fill { f.fill } else { f.text_color }, Some(RGB));
        assert_eq!(sheet(&h).cell_format_at(b, 0), before.workbooks().next().unwrap().sheets[0].cell_format_at(b, 0));
        assert_eq!(sheet(&h).columns, columns);
        let applied = project(&h).clone();
        let revision = h.state().doc.revision();
        custom(&mut h, fill);
        choose(&mut h, "Worksheet color");
        click(&mut h, "OK");
        assert_eq!(project(&h), &applied);
        assert_eq!(h.state().doc.revision(), revision);
        menu(&mut h, "Edit", "Undo");
        assert_eq!(project(&h), &before);
    }
    assert_eq!(std::fs::read(file).unwrap(), saved);
}

#[test]
fn format_cells_add_and_saved_choices_keep_apply_cancel_and_optional_color_semantics() {
    let (data, file) = profile("worksheet-wells-format-cells");
    let mut h = start_with(&data, &file);
    let book = project(&h).workbooks().next().unwrap().id;
    let b = sheet(&h).column_by_name("B").unwrap().id;
    h.state_mut().doc.apply("fixture", |p| {
        p.workbook_mut(book).unwrap().sheets[0].apply_cell_format(
            FormatTarget::Cells { columns: vec![b], rows: Some((1, Some(1))) },
            CellFormat { text_color: Some(RGB), ..Default::default() },
        )
    });
    settle(&mut h);
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    cells(&mut h);
    click(&mut h, "Cell text color: Add to Custom");
    assert!(h.get_by_label("Apply").accesskit_node().is_disabled());
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    assert!(file.is_file());
    click_text(&mut h, "6.5");
    let before = project(&h).clone();
    cells(&mut h);
    assert!(h.get_by_label("Cell text color: Add to Custom").accesskit_node().is_disabled());
    choose(&mut h, "Cell text color");
    choose(&mut h, "Cell fill color");
    assert_eq!(h.get_by_label("Use color").accesskit_node().toggled(), Some(egui::accesskit::Toggled::True));
    assert_eq!(h.get_by_label("Use fill").accesskit_node().toggled(), Some(egui::accesskit::Toggled::True));
    assert_eq!(project(&h), &before);
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &before);
    cells(&mut h);
    choose(&mut h, "Cell text color");
    choose(&mut h, "Cell fill color");
    click(&mut h, "Apply");
    let style = sheet(&h).cell_format_at(b, 2);
    assert_eq!(style.text_color, Some(RGB));
    assert_eq!(style.fill, Some(RGB));
    assert_eq!(sheet(&h).cell_format_at(b, 1).text_color, Some(RGB));
    assert!(h.get_by_label("Apply").accesskit_node().is_disabled());
    click(&mut h, "Cancel");
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
}

#[test]
fn shared_wells_keep_whole_columns_compact_and_label_formats_independent() {
    let (data, file) = profile("worksheet-wells-targets");
    seed(&file);
    let mut h = start_with(&data, &file);
    let book = project(&h).workbooks().next().unwrap().id;
    let b = sheet(&h).column_by_name("B").unwrap().id;
    let a = sheet(&h).column_by_name("A").unwrap().id;
    let columns = sheet(&h).columns.clone();
    let state = h.state_mut().books.get_mut(&book).unwrap();
    state.sel = Sel::Columns;
    state.selected = vec![b];
    settle(&mut h);
    custom(&mut h, false);
    choose(&mut h, "Worksheet color");
    click(&mut h, "OK");
    assert_eq!(sheet(&h).cell_format_at(b, 1_000_000).text_color, Some(RGB));
    assert_eq!(sheet(&h).cell_format_at(a, 1_000_000).text_color, None);
    assert_eq!(sheet(&h).format_index().resolve_label(sheet(&h), b, &LabelKind::LongName).text_color, None);
    assert_eq!(sheet(&h).cell_formats.len(), 1);
    let state = h.state_mut().books.get_mut(&book).unwrap();
    state.sel = Sel::Label { column: b, row: LabelRow::LongName };
    state.selected = vec![b];
    settle(&mut h);
    custom(&mut h, true);
    choose(&mut h, "Worksheet color");
    click(&mut h, "OK");
    assert_eq!(sheet(&h).format_index().resolve_label(sheet(&h), b, &LabelKind::LongName).fill, Some(RGB));
    assert_eq!(sheet(&h).cell_format_at(b, 0).fill, None);
    assert_eq!(sheet(&h).cell_formats.len(), 2);
    assert_eq!(sheet(&h).columns, columns);
}

#[test]
fn saved_drafts_cannot_bypass_toolbar_target_revision_or_generation_guards() {
    for scenario in 0..3 {
        let (data, file) = profile(&format!("worksheet-wells-stale-{scenario}"));
        seed(&file);
        let mut h = start_with(&data, &file);
        let book = project(&h).workbooks().next().unwrap().id;
        custom(&mut h, false);
        choose(&mut h, "Worksheet color");
        match scenario {
            0 => h.state_mut().books.get_mut(&book).unwrap().sel = Sel::Cells { anchor: (1, 2), end: (1, 2) },
            1 => {
                h.state_mut().doc.apply("Other", |p| p.workbook_mut(book).unwrap().name.push('x'));
            }
            _ => {
                let copy = project(&h).clone();
                h.state_mut().doc.replace(copy);
            }
        }
        settle(&mut h);
        let before = project(&h).clone();
        let revision = h.state().doc.revision();
        if h.state().worksheet_color.is_some() {
            click(&mut h, "OK");
        }
        assert_eq!(project(&h), &before);
        assert_eq!(h.state().doc.revision(), revision);
    }
}

#[test]
fn invalid_add_and_remove_are_preference_only_and_applied_colors_survive_both_project_formats() {
    let (data, file) = profile("worksheet-wells-remove-save");
    seed(&file);
    let mut h = start_with(&data, &file);
    let before = project(&h).clone();
    custom(&mut h, true);
    channel(&mut h, "Red", "256");
    assert!(h.get_by_label("Worksheet color: Add to Custom").accesskit_node().is_disabled());
    choose(&mut h, "Worksheet color");
    click(&mut h, "OK");
    let expected = sheet(&h).clone();
    let selected = project(&h).clone();
    let revision = h.state().doc.revision();
    custom(&mut h, false);
    click(&mut h, "Worksheet color: Custom colors");
    right_click(&mut h, "Worksheet color: Custom 17,131,213");
    click(&mut h, "Remove custom color");
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &selected);
    assert_eq!(h.state().doc.revision(), revision);
    assert_eq!(sheet(&h), &expected);
    for ext in ["sheaf", "opj"] {
        let path = data.parent().unwrap().join(format!("worksheet-wells.{ext}"));
        sheaf_io::save::save_project(project(&h), &path).unwrap();
        let back = sheaf_io::open_project(&path).unwrap();
        assert_eq!(back.project.workbooks().next().unwrap().sheets[0], expected);
    }
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
    let record: serde_json::Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    assert_eq!(record["colors"], serde_json::json!([]));
}
