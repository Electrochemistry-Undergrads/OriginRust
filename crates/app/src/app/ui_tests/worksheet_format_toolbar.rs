use super::*;
use crate::text_format::Change;
use sheaf_core::cellformat::{CellFormat, FormatTarget, LabelKind};

fn start_sheet(tag: &str) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(tag)]);
    let id = project(&h).workbooks().next().unwrap().id;
    let title = project(&h).windows.iter().find(|w| w.id() == id).unwrap().title();
    show_window(&mut h, &title);
    h
}

fn enter_size(h: &mut Harness<'static, SheafApp>, value: &str) {
    click(h, "Format toolbar: Font Size");
    ctrl(h, egui::Key::A);
    h.event(egui::Event::Text(value.into()));
    h.run();
    h.key_press(egui::Key::Enter);
    settle(h);
}

fn apply(h: &mut Harness<'static, SheafApp>, change: Change) {
    let request = h.state().text_format().unwrap().request(change);
    h.state_mut().apply(Action::FormatText(request), &egui::Context::default());
    settle(h);
}

fn column_id(h: &Harness<'static, SheafApp>, name: &str) -> ColumnId {
    sheet(h).column_by_name(name).unwrap().id
}

#[test]
fn worksheet_font_controls_follow_the_selected_cells() {
    let mut h = start(vec![data_file("worksheet-toolbar-selection")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let title = project(&h).windows.iter().find(|w| w.id() == book).unwrap().title();
    show_window(&mut h, &title);
    click_text(&mut h, "4.5");
    assert!(matches!(book_ui(&h).sel, Sel::Cells { .. }));
    assert!(!h.get_by_label("Format toolbar: Bold").accesskit_node().is_disabled());
    click(&mut h, "Format toolbar: Bold");
    let b = sheet(&h).column_by_name("B").unwrap().id;
    assert!(sheet(&h).cell_format_at(b, 1).bold);
    assert!(!sheet(&h).cell_format_at(b, 0).bold);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
    assert_eq!(h.state().doc.undo_label(), Some("Format cells"));
}

#[test]
fn worksheet_size_and_color_keep_data_undo_and_no_ops() {
    let mut h = start_sheet("worksheet-toolbar-size");
    click_text(&mut h, "4.5");
    let original = project(&h).clone();
    assert_eq!(h.state().text_format().unwrap().font.size_pt, 9.0);
    let b = column_id(&h, "B");
    enter_size(&mut h, "20");
    assert_eq!(sheet(&h).cell_format_at(b, 1).size_pt, Some(20.0));
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
    let revision = h.state().doc.revision();
    let formatted = project(&h).clone();
    for text in ["20", "-1", "3", "97", "NaN", "inf", "bad"] {
        enter_size(&mut h, text);
        assert_eq!(h.state().doc.revision(), revision, "input {text}");
        assert_eq!(project(&h), &formatted);
    }
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &original);
    menu(&mut h, "Edit", "Redo");
    assert_eq!(project(&h), &formatted);
    click(&mut h, "Format toolbar: Font Color");
    click(&mut h, "Font Color: Red");
    assert_eq!(sheet(&h).cell_format_at(b, 1).text_color, Some(sheaf_core::Color::rgb(255, 0, 0)));
    assert_eq!(sheet(&h).cell_format_at(b, 0).text_color, None);
    enter_size(&mut h, "0");
    assert_eq!(sheet(&h).cell_format_at(b, 1).size_pt, None);
    assert_eq!(h.state().text_format().unwrap().font.size_pt, 9.0);
    assert!(h.get_by_label("Format toolbar: Increase Font Size").accesskit_node().is_disabled());
}

#[test]
fn mixed_worksheet_ranges_report_each_property_and_keep_the_first_bold_state() {
    let mut h = start_sheet("worksheet-toolbar-mixed");
    let id = project(&h).workbooks().next().unwrap().id;
    let b = column_id(&h, "B");
    h.state_mut().doc.apply("Seed mixed cells", |p| {
        p.workbook_mut(id).unwrap().sheet_mut().apply_cell_format(
            FormatTarget::cells(vec![b], 1, Some(1)),
            CellFormat { bold: Some(true), size_pt: Some(20.0), ..Default::default() },
        );
    });
    let state = h.state_mut().books.get_mut(&id).unwrap();
    state.sel = Sel::Cells { anchor: (1, 1), end: (1, 2) };
    state.selected = vec![b];
    settle(&mut h);
    let s = h.state().text_format().unwrap();
    assert!(s.mixed.size && s.mixed.bold);
    assert!(s.font.bold);
    assert_eq!(s.size_text(), "");
    click(&mut h, "Format toolbar: Bold");
    assert!(!sheet(&h).cell_format_at(b, 1).bold && !sheet(&h).cell_format_at(b, 2).bold);
    menu(&mut h, "Edit", "Undo");
    assert!(h.state().text_format().unwrap().mixed.bold);
    enter_size(&mut h, "18");
    assert_eq!(sheet(&h).cell_format_at(b, 1).size_pt, Some(18.0));
    assert_eq!(sheet(&h).cell_format_at(b, 2).size_pt, Some(18.0));
    assert!(!h.state().text_format().unwrap().mixed.size);
}

#[test]
fn worksheet_font_and_size_change_painted_cells_and_only_their_row_geometry() {
    let mut h = start_sheet("worksheet-toolbar-drawing");
    click_text(&mut h, "4.5");
    let before = grid_text(&h, "6.5").rect().center().y - grid_text(&h, "4.5").rect().center().y;
    enter_size(&mut h, "20");
    apply(&mut h, Change::Family("Courier New".into()));
    let after = grid_text(&h, "6.5").rect().center().y - grid_text(&h, "4.5").rect().center().y;
    assert!(after > before + 5.0, "large text gets row space: {before} -> {after}");
    let next = grid_text(&h, "8.5").rect().center().y - grid_text(&h, "6.5").rect().center().y;
    assert!((next - 16.0).abs() < 0.5, "unformatted rows retain default height");
    let mut found = false;
    fn visit(shape: &egui::Shape, found: &mut bool) {
        match shape {
            egui::Shape::Vec(v) => {
                for s in v {
                    visit(s, found);
                }
            }
            egui::Shape::Text(t) if t.galley.job.text == "4.5" => {
                let f = &t.galley.job.sections[0].format.font_id;
                assert!((f.size - 20.0 * 4.0 / 3.0).abs() < 0.01);
                assert_eq!(f.family, egui::FontFamily::Name("worksheet:Courier New:false:false".into()));
                *found = true;
            }
            _ => {}
        }
    }
    for s in &h.output().shapes {
        visit(&s.shape, &mut found);
    }
    assert!(found, "the actual painted cell uses the installed face and physical size");
}

#[test]
fn worksheet_labels_columns_and_whole_rows_keep_separate_targets() {
    let mut h = start_sheet("worksheet-toolbar-targets");
    let id = project(&h).workbooks().next().unwrap().id;
    let b = column_id(&h, "B");
    let state = h.state_mut().books.get_mut(&id).unwrap();
    state.selected = vec![b];
    state.sel = Sel::Columns;
    settle(&mut h);
    click(&mut h, "Format toolbar: Italic");
    assert!(sheet(&h).cell_format_at(b, 1_000_000).italic, "future rows are included");
    assert_eq!(sheet(&h).cell_formats.len(), 1, "one record for the whole column");
    let revision = h.state().doc.revision();
    apply(&mut h, Change::Italic(true));
    assert_eq!(h.state().doc.revision(), revision, "a uniform style is a no-op");
    let state = h.state_mut().books.get_mut(&id).unwrap();
    state.sel = Sel::Rows { anchor: 1, end: 1 };
    settle(&mut h);
    click(&mut h, "Format toolbar: Bold");
    for c in &sheet(&h).columns {
        assert!(sheet(&h).cell_format_at(c.id, 1).bold);
    }
    let state = h.state_mut().books.get_mut(&id).unwrap();
    state.sel = Sel::Label { column: b, row: LabelRow::LongName };
    state.selected = vec![b];
    settle(&mut h);
    click(&mut h, "Format toolbar: Bold");
    assert!(sheet(&h).format_index().resolve_label(sheet(&h), b, &LabelKind::LongName).bold);
    assert!(!sheet(&h).cell_format_at(b, 0).bold);
    assert!(!sheet(&h).format_index().resolve_label(sheet(&h), b, &LabelKind::Units).bold);
}

#[test]
fn stale_worksheet_palette_selection_revision_and_editing_requests_are_ignored() {
    let mut h = start_sheet("worksheet-toolbar-stale");
    click_text(&mut h, "4.5");
    let request = h.state().text_format().unwrap().request(Change::Bold(true));
    click_text(&mut h, "6.5");
    let before = project(&h).clone();
    h.state_mut().apply(Action::FormatText(request), &egui::Context::default());
    assert_eq!(project(&h), &before);
    click(&mut h, "Format toolbar: Font Color");
    let id = project(&h).workbooks().next().unwrap().id;
    h.state_mut().books.get_mut(&id).unwrap().sel = Sel::Cells { anchor: (1, 0), end: (1, 0) };
    settle(&mut h);
    click(&mut h, "Font Color: Red");
    assert_eq!(project(&h), &before);
    let request = h.state().text_format().unwrap().request(Change::Bold(true));
    h.state_mut().doc.apply("Other change", |p| p.workbook_mut(id).unwrap().name.push('x'));
    let changed = project(&h).clone();
    h.state_mut().apply(Action::FormatText(request), &egui::Context::default());
    assert_eq!(project(&h), &changed);
    let request = h.state().text_format().unwrap().request(Change::Bold(true));
    let b = column_id(&h, "B");
    h.state_mut().books.get_mut(&id).unwrap().editing = Some((b, 0, "2.5".into()));
    h.state_mut().apply(Action::FormatText(request), &egui::Context::default());
    assert_eq!(project(&h), &changed);
}

#[test]
fn worksheet_toolbar_formats_survive_project_and_companion_reopen() {
    let mut h = start_sheet("worksheet-toolbar-save");
    click_text(&mut h, "4.5");
    click(&mut h, "Format toolbar: Bold");
    enter_size(&mut h, "18");
    assert_eq!(sheet(&h).cell_format_at(column_id(&h, "B"), 1).size_pt, Some(18.0));
    apply(&mut h, Change::Family("Courier New".into()));
    let expected = sheet(&h).cell_formats.clone();
    let dir = save_dir("worksheet-toolbar-save");
    for ext in ["sheaf", "opj"] {
        let file = dir.join(format!("formatted.{ext}"));
        h.state_mut().save_to(file.clone());
        let back = start(vec![file]);
        assert_eq!(sheet(&back).cell_formats, expected, "{ext}");
        assert_eq!(column(&back, "B"), [2.5, 4.5, 6.5, 8.5]);
    }
}

#[test]
fn million_row_worksheet_toolbar_stays_one_range_and_draws_only_visible_cells() {
    let mut h = start_sheet("worksheet-toolbar-million");
    let id = project(&h).workbooks().next().unwrap().id;
    let b = column_id(&h, "B");
    h.state_mut().doc.apply("Seed large column", |p| {
        p.workbook_mut(id).unwrap().sheet_mut().columns[1].data =
            sheaf_core::column::ColumnData::numeric(vec![1.0; 1_000_000]);
    });
    let state = h.state_mut().books.get_mut(&id).unwrap();
    state.sel = Sel::Columns;
    state.selected = vec![b];
    settle(&mut h);
    click(&mut h, "Format toolbar: Bold");
    assert_eq!(sheet(&h).cell_formats.len(), 1);
    assert!(matches!(sheet(&h).cell_formats[0].target, FormatTarget::Cells { rows: None, .. }));
    assert!(sheet(&h).cell_format_at(b, 999_999).bold);
    assert!(sheet(&h).cell_format_at(b, 1_000_001).bold);
    let revision = h.state().doc.revision();
    apply(&mut h, Change::Bold(true));
    assert_eq!(h.state().doc.revision(), revision);
    assert_eq!(sheet(&h).columns[1].data.len(), 1_000_000);
    assert!(h.output().shapes.len() < 2_000, "the viewport, not a million cells, is painted");
}

#[test]
fn worksheet_font_picker_treats_current_default_as_no_op_and_applies_an_installed_family() {
    let mut h = start_sheet("worksheet-toolbar-family-picker");
    click_text(&mut h, "4.5");
    let original = project(&h).clone();
    let revision = h.state().doc.revision();
    apply(&mut h, Change::Family("Arial".into()));
    assert_eq!(project(&h), &original);
    assert_eq!(h.state().doc.revision(), revision);
    assert!(!h.get_by_label("Format toolbar: Font").accesskit_node().is_disabled());
    let family = sheaf_plot::SystemFonts::shared()
        .database()
        .faces()
        .flat_map(|f| f.families.iter().map(|(n, _)| n.clone()))
        .filter(|n| n != "Arial")
        .min()
        .unwrap();
    click(&mut h, "Format toolbar: Font");
    click(&mut h, &family);
    let b = column_id(&h, "B");
    assert_eq!(sheet(&h).cell_format_at(b, 1).font, Some(family.as_str()));
    assert_eq!(sheet(&h).cell_format_at(b, 0).font, None);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
}

#[test]
fn enlarged_worksheet_label_and_frozen_row_keep_their_geometry_while_scrolling() {
    let mut h = start_sheet("worksheet-toolbar-frozen-geometry");
    let id = project(&h).workbooks().next().unwrap().id;
    let b = column_id(&h, "B");
    h.state_mut().doc.apply("Seed large frozen cells", |p| {
        let ws = p.workbook_mut(id).unwrap().sheet_mut();
        ws.apply_cell_format(
            FormatTarget::cells(vec![b], 0, Some(0)),
            CellFormat { size_pt: Some(20.0), ..Default::default() },
        );
        ws.apply_cell_format(
            FormatTarget::label_rows(vec![b], vec![LabelKind::LongName]),
            CellFormat { size_pt: Some(20.0), ..Default::default() },
        );
    });
    settle(&mut h);
    click_text(&mut h, "4.5");
    key_with(&mut h, egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::F);
    assert_eq!(sheet(&h).frozen.rows, 1);
    assert_eq!(sheet(&h).frozen.columns, 1);
    let top = grid_text(&h, "2.5").rect().center().y;
    let label = grid_text(&h, "up").rect().center().y;
    let next = grid_text(&h, "4.5").rect().center().y;
    assert!(next - top > 20.0, "the frozen large cell has room");
    let at = grid_text(&h, "6.5").rect().center();
    h.event(egui::Event::PointerMoved(at));
    h.step();
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, -8.0),
        modifiers: egui::Modifiers::NONE,
    });
    h.run_steps(40);
    assert!(book_ui(&h).scroll_offsets().1 > 20.0);
    assert!((grid_text(&h, "2.5").rect().center().y - top).abs() < 0.5);
    assert!((grid_text(&h, "up").rect().center().y - label).abs() < 0.5);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
}
