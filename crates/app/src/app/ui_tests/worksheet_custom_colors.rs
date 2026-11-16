use super::*;

fn start_sheet(tag: &str) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(tag)]);
    show_window(&mut h, "Book1 - sample.csv");
    click_text(&mut h, "4.5");
    h
}

fn channel(h: &mut Harness<'static, SheafApp>, label: &str, value: &str) {
    click(h, &format!("Worksheet color: {label}"));
    ctrl(h, egui::Key::A);
    h.event(egui::Event::Text(value.into()));
    settle(h);
}

fn custom(h: &mut Harness<'static, SheafApp>, fill: bool) {
    let name = if fill { "Fill Color" } else { "Font Color" };
    click(h, &format!("Format toolbar: {name}"));
    click(h, &format!("{name}: Custom..."));
}

fn rgb(h: &mut Harness<'static, SheafApp>, values: [u8; 3]) {
    for (name, value) in ["Red", "Green", "Blue"].into_iter().zip(values) {
        channel(h, name, &value.to_string());
    }
}

#[test]
fn custom_text_color_changes_only_the_selected_cell_and_has_one_undo_step() {
    let mut h = start_sheet("worksheet-rgb-text");
    let b = sheet(&h).column_by_name("B").unwrap().id;
    let before = project(&h).clone();
    click(&mut h, "Format toolbar: Font Color");
    click(&mut h, "Font Color: Custom...");
    for (label, value) in [("Red", "17"), ("Green", "34"), ("Blue", "51")] {
        channel(&mut h, label, value);
    }
    assert_eq!(project(&h), &before, "editing the chooser is not an edit to the sheet");
    click(&mut h, "OK");
    assert_eq!(sheet(&h).cell_format_at(b, 1).text_color, Some(sheaf_core::Color::rgb(17, 34, 51)));
    assert_eq!(sheet(&h).cell_format_at(b, 0).text_color, None);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
    assert_eq!(h.state().doc.undo_label(), Some("Format cells"));
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
}

#[test]
fn custom_fill_validates_bytes_and_preserves_styles_through_cancel_none_and_noops() {
    let mut h = start_sheet("worksheet-rgb-fill");
    let b = sheet(&h).column_by_name("B").unwrap().id;
    click(&mut h, "Format toolbar: Bold");
    click(&mut h, "Format toolbar: Underline");
    let before = project(&h).clone();
    custom(&mut h, true);
    for invalid in ["-1", "256", "1.5", "", "NaN"] {
        channel(&mut h, "Red", invalid);
        assert!(h.get_by_label("OK").accesskit_node().is_disabled());
        assert_eq!(project(&h), &before);
    }
    for key in [egui::Key::B, egui::Key::I, egui::Key::U] {
        ctrl(&mut h, key);
    }
    assert_eq!(project(&h), &before, "dialog keys cannot format the underlying cells");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().worksheet_color.is_none());
    assert_eq!(project(&h), &before);
    custom(&mut h, true);
    rgb(&mut h, [32, 128, 224]);
    click(&mut h, "OK");
    let style = sheet(&h).cell_format_at(b, 1);
    assert_eq!(style.fill, Some(sheaf_core::Color::rgb(32, 128, 224)));
    assert!(style.bold && style.underline);
    assert_eq!(sheet(&h).cell_format_at(b, 0).fill, None);
    let revision = h.state().doc.revision();
    custom(&mut h, true);
    click(&mut h, "OK");
    assert_eq!(h.state().doc.revision(), revision, "the current RGB is an effective no-op");
    let filled = project(&h).clone();
    custom(&mut h, true);
    rgb(&mut h, [200, 100, 50]);
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &filled);
    click(&mut h, "Format toolbar: Fill Color");
    click(&mut h, "Fill Color: None");
    let style = sheet(&h).cell_format_at(b, 1);
    assert_eq!(style.fill, None);
    assert!(style.bold && style.underline);
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &filled);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
}

#[test]
fn custom_color_requests_keep_the_opening_target_revision_and_project_generation() {
    let mut h = start_sheet("worksheet-rgb-stale");
    let book = project(&h).workbooks().next().unwrap().id;
    let before = project(&h).clone();
    click(&mut h, "Format toolbar: Font Color");
    h.state_mut().books.get_mut(&book).unwrap().sel = Sel::Cells { anchor: (1, 2), end: (1, 2) };
    settle(&mut h);
    click(&mut h, "Font Color: Custom...");
    assert!(h.state().worksheet_color.is_none(), "a stale palette cannot open a new-target picker");
    assert_eq!(project(&h), &before);

    click_text(&mut h, "4.5");
    custom(&mut h, false);
    rgb(&mut h, [101, 44, 11]);
    h.state_mut().books.get_mut(&book).unwrap().sel = Sel::Cells { anchor: (1, 2), end: (1, 2) };
    settle(&mut h);
    click(&mut h, "OK");
    assert_eq!(project(&h), &before);

    custom(&mut h, true);
    rgb(&mut h, [22, 33, 44]);
    h.state_mut().doc.apply("Unrelated change", |p| p.workbook_mut(book).unwrap().name.push('x'));
    settle(&mut h);
    let changed = project(&h).clone();
    click(&mut h, "OK");
    assert_eq!(project(&h), &changed);
    assert_eq!(h.state().doc.undo_label(), Some("Unrelated change"));

    custom(&mut h, false);
    rgb(&mut h, [50, 60, 70]);
    let replacement = project(&h).clone();
    h.state_mut().doc.replace(replacement);
    settle(&mut h);
    let replaced = project(&h).clone();
    if h.state().worksheet_color.is_some() {
        click(&mut h, "OK");
    }
    assert_eq!(project(&h), &replaced);
    assert!(h.state().worksheet_color.is_none());
}

#[test]
fn custom_colors_keep_whole_columns_compact_and_label_targets_independent() {
    use sheaf_core::cellformat::LabelKind;
    let mut h = start_sheet("worksheet-rgb-targets");
    let book = project(&h).workbooks().next().unwrap().id;
    let b = sheet(&h).column_by_name("B").unwrap().id;
    let a = sheet(&h).column_by_name("A").unwrap().id;
    let columns = sheet(&h).columns.clone();
    let state = h.state_mut().books.get_mut(&book).unwrap();
    state.sel = Sel::Columns;
    state.selected = vec![b];
    settle(&mut h);
    custom(&mut h, false);
    rgb(&mut h, [45, 67, 89]);
    click(&mut h, "OK");
    assert_eq!(sheet(&h).cell_format_at(b, 1_000_000).text_color, Some(sheaf_core::Color::rgb(45, 67, 89)));
    assert_eq!(sheet(&h).cell_format_at(a, 1_000_000).text_color, None);
    assert_eq!(sheet(&h).format_index().resolve_label(sheet(&h), b, &LabelKind::LongName).text_color, None);
    assert!(sheet(&h).cell_formats.len() <= 1);
    let state = h.state_mut().books.get_mut(&book).unwrap();
    state.sel = Sel::Label { column: b, row: LabelRow::LongName };
    state.selected = vec![b];
    settle(&mut h);
    custom(&mut h, true);
    rgb(&mut h, [80, 90, 100]);
    click(&mut h, "OK");
    assert_eq!(
        sheet(&h).format_index().resolve_label(sheet(&h), b, &LabelKind::LongName).fill,
        Some(sheaf_core::Color::rgb(80, 90, 100))
    );
    assert_eq!(sheet(&h).cell_format_at(b, 0).fill, None);
    assert!(sheet(&h).cell_formats.len() <= 2);
    assert_eq!(sheet(&h).columns, columns);
}

#[test]
fn custom_rgb_text_and_fill_survive_project_and_companion_reopen() {
    let mut h = start_sheet("worksheet-rgb-save");
    custom(&mut h, false);
    rgb(&mut h, [17, 34, 51]);
    click(&mut h, "OK");
    custom(&mut h, true);
    rgb(&mut h, [32, 128, 224]);
    click(&mut h, "OK");
    let before = sheet(&h).cell_formats.clone();
    let dir = save_dir("worksheet-rgb-save");
    for ext in ["sheaf", "opj"] {
        let file = dir.join(format!("custom-colors.{ext}"));
        h.state_mut().save_to(file.clone());
        let back = start(vec![file]);
        let b = sheet(&back).column_by_name("B").unwrap().id;
        let style = sheet(&back).cell_format_at(b, 1);
        assert_eq!(style.text_color, Some(sheaf_core::Color::rgb(17, 34, 51)));
        assert_eq!(style.fill, Some(sheaf_core::Color::rgb(32, 128, 224)));
        assert_eq!(sheet(&back).cell_formats, before);
        assert_eq!(column(&back, "B"), [2.5, 4.5, 6.5, 8.5]);
    }
}

#[test]
fn worksheet_custom_chooser_does_not_extend_the_graph_format_palette() {
    let mut h = start(vec![data_file("worksheet-rgb-graph-guard")]);
    let id = graph(&h).id;
    let title = project(&h).windows.iter().find(|w| w.id() == id).unwrap().title();
    show_window(&mut h, &title);
    let at = on(&h, 0, ObjectKind::Legend);
    click_at(&mut h, at);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Legend)));
    let before = project(&h).clone();
    click(&mut h, "Format toolbar: Font Color");
    assert!(h.query_by_label("Font Color: Custom...").is_none());
    assert!(h.state().worksheet_color.is_none());
    assert_eq!(project(&h), &before);
}

#[test]
fn enter_accepts_valid_rgb_and_unchanged_default_color_has_no_undo() {
    let mut h = start_sheet("worksheet-rgb-enter");
    let b = sheet(&h).column_by_name("B").unwrap().id;
    let revision = h.state().doc.revision();
    custom(&mut h, false);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(h.state().worksheet_color.is_none());
    assert_eq!(h.state().doc.revision(), revision);
    assert_eq!(sheet(&h).cell_format_at(b, 1).text_color, None);
    custom(&mut h, true);
    rgb(&mut h, [11, 22, 33]);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(h.state().worksheet_color.is_none());
    assert_eq!(sheet(&h).cell_format_at(b, 1).fill, Some(sheaf_core::Color::rgb(11, 22, 33)));
    let before = project(&h).clone();
    custom(&mut h, true);
    channel(&mut h, "Red", "256");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(h.state().worksheet_color.is_some());
    assert_eq!(project(&h), &before);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().worksheet_color.is_none());
}
