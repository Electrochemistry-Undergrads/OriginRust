use super::*;
use crate::text_format::Change;
use sheaf_core::cellformat::{CellFormat, FormatTarget, LabelKind};

fn b(h: &Harness<'static, SheafApp>) -> ColumnId {
    sheet(h).column_by_name("B").unwrap().id
}
fn book(h: &Harness<'static, SheafApp>) -> WindowId {
    project(h).workbooks().next().unwrap().id
}

fn change(h: &mut Harness<'static, SheafApp>, change: Change) {
    let request = h.state().text_format().unwrap().request(change);
    h.state_mut().apply(Action::FormatText(request), &egui::Context::default());
    settle(h);
}

fn fill(h: &mut Harness<'static, SheafApp>, name: &str) {
    click(h, "Format toolbar: Fill Color");
    click(h, &format!("Fill Color: {name}"));
}

fn painted(h: &Harness<'static, SheafApp>, text: &str) -> Vec<egui::text::TextFormat> {
    fn walk(shape: &egui::Shape, text: &str, result: &mut Vec<egui::text::TextFormat>) {
        match shape {
            egui::Shape::Vec(v) => {
                for s in v {
                    walk(s, text, result);
                }
            }
            egui::Shape::Text(t) if t.galley.job.text == text => {
                result.extend(t.galley.job.sections.iter().map(|s| s.format.clone()))
            }
            _ => {}
        }
    }
    let mut result = Vec::new();
    for shape in &h.output().shapes {
        walk(&shape.shape, text, &mut result);
    }
    result
}

fn start_sheet(tag: &str) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(tag)]);
    let id = project(&h).workbooks().next().unwrap().id;
    let title = project(&h).windows.iter().find(|w| w.id() == id).unwrap().title();
    show_window(&mut h, &title);
    h
}

#[test]
fn worksheet_underline_button_targets_the_selected_cell() {
    let mut h = start_sheet("worksheet-controls-underline");
    click_text(&mut h, "4.5");
    assert!(!h.get_by_label("Format toolbar: Underline").accesskit_node().is_disabled());
    click(&mut h, "Format toolbar: Underline");
    let b = sheet(&h).column_by_name("B").unwrap().id;
    assert!(sheet(&h).cell_format_at(b, 1).underline);
    assert!(!sheet(&h).cell_format_at(b, 0).underline);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
    assert_eq!(h.state().doc.undo_label(), Some("Format cells"));
    click_text(&mut h, "6.5");
    assert!(painted(&h, "4.5").iter().any(|f| f.underline.width > 0.0), "the actual cell is underlined");
    menu(&mut h, "Edit", "Undo");
    assert!(!sheet(&h).cell_format_at(b, 1).underline);
    menu(&mut h, "Edit", "Redo");
    assert!(sheet(&h).cell_format_at(b, 1).underline);
}

#[test]
fn fill_colour_and_none_preserve_other_styles_and_skip_no_ops() {
    let mut h = start_sheet("worksheet-controls-fill");
    click_text(&mut h, "4.5");
    let col = b(&h);
    let columns = sheet(&h).columns.clone();
    click(&mut h, "Format toolbar: Underline");
    fill(&mut h, "Red");
    assert_eq!(sheet(&h).cell_format_at(col, 1).fill, Some(sheaf_core::Color::RED));
    assert!(sheet(&h).cell_format_at(col, 1).underline);
    assert_eq!(sheet(&h).cell_format_at(col, 0).fill, None);
    let revision = h.state().doc.revision();
    fill(&mut h, "Red");
    assert_eq!(h.state().doc.revision(), revision);
    fill(&mut h, "None");
    assert_eq!(sheet(&h).cell_format_at(col, 1).fill, None);
    assert!(sheet(&h).cell_format_at(col, 1).underline);
    let revision = h.state().doc.revision();
    fill(&mut h, "None");
    assert_eq!(h.state().doc.revision(), revision);
    menu(&mut h, "Edit", "Undo");
    assert_eq!(sheet(&h).cell_format_at(col, 1).fill, Some(sheaf_core::Color::RED));
    menu(&mut h, "Edit", "Redo");
    assert_eq!(sheet(&h).cell_format_at(col, 1).fill, None);
    assert_eq!(sheet(&h).columns, columns);
}

#[test]
fn ctrl_b_i_u_toggle_the_selected_cells_in_separate_undo_steps() {
    let mut h = start_sheet("worksheet-controls-keys");
    click_text(&mut h, "4.5");
    let col = b(&h);
    for key in [egui::Key::B, egui::Key::I, egui::Key::U] {
        ctrl(&mut h, key);
    }
    let f = sheet(&h).cell_format_at(col, 1);
    assert!(f.bold && f.italic && f.underline);
    assert_eq!(h.state().doc.undo_label(), Some("Format cells"));
    assert!(!sheet(&h).cell_format_at(col, 0).bold);
    menu(&mut h, "Edit", "Undo");
    let f = sheet(&h).cell_format_at(col, 1);
    assert!(f.bold && f.italic && !f.underline);
    menu(&mut h, "Edit", "Redo");
    assert!(sheet(&h).cell_format_at(col, 1).underline);
    for key in [egui::Key::B, egui::Key::I, egui::Key::U] {
        ctrl(&mut h, key);
    }
    let f = sheet(&h).cell_format_at(col, 1);
    assert!(!f.bold && !f.italic && !f.underline);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
}

#[test]
fn formatting_keys_do_not_escape_a_text_editor_or_an_empty_selection() {
    let mut h = start_sheet("worksheet-controls-key-context");
    click_text(&mut h, "4.5");
    let before = project(&h).clone();
    let rev = h.state().doc.revision();
    click(&mut h, "Format toolbar: Font Size");
    for key in [egui::Key::B, egui::Key::I, egui::Key::U] {
        ctrl(&mut h, key);
    }
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), rev);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    let id = book(&h);
    let state = h.state_mut().books.get_mut(&id).unwrap();
    state.sel = Sel::None;
    state.selected.clear();
    settle(&mut h);
    for key in [egui::Key::B, egui::Key::I, egui::Key::U] {
        ctrl(&mut h, key);
    }
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), rev);
}

#[test]
fn mixed_ranges_labels_and_whole_columns_keep_their_own_underline_and_fill() {
    let mut h = start_sheet("worksheet-controls-targets");
    let id = book(&h);
    let col = b(&h);
    h.state_mut().doc.apply("Seed mixed style", |p| {
        p.workbook_mut(id).unwrap().sheet_mut().apply_cell_format(
            FormatTarget::cells(vec![col], 1, Some(1)),
            CellFormat { underline: Some(true), fill: Some(sheaf_core::Color::BLUE), ..Default::default() },
        );
    });
    let state = h.state_mut().books.get_mut(&id).unwrap();
    state.sel = Sel::Cells { anchor: (1, 1), end: (1, 2) };
    state.selected = vec![col];
    settle(&mut h);
    let summary = h.state().text_format().unwrap();
    assert!(summary.mixed.underline && summary.mixed.fill);
    assert!(summary.worksheet.unwrap().underline);
    click(&mut h, "Format toolbar: Underline");
    assert!(!sheet(&h).cell_format_at(col, 1).underline && !sheet(&h).cell_format_at(col, 2).underline);
    fill(&mut h, "Green");
    for row in [1, 2] {
        assert_eq!(sheet(&h).cell_format_at(col, row).fill, Some(sheaf_core::Color::rgb(0, 255, 0)));
    }
    let state = h.state_mut().books.get_mut(&id).unwrap();
    state.sel = Sel::Label { column: col, row: LabelRow::LongName };
    state.selected = vec![col];
    settle(&mut h);
    ctrl(&mut h, egui::Key::U);
    assert!(sheet(&h).format_index().resolve_label(sheet(&h), col, &LabelKind::LongName).underline);
    ctrl(&mut h, egui::Key::U);
    let state = h.state_mut().books.get_mut(&id).unwrap();
    state.sel = Sel::Columns;
    state.selected = vec![col];
    settle(&mut h);
    ctrl(&mut h, egui::Key::U);
    assert!(sheet(&h).cell_format_at(col, 1_000_000).underline);
    assert!(!sheet(&h).format_index().resolve_label(sheet(&h), col, &LabelKind::LongName).underline);
}

#[test]
fn an_open_worksheet_fill_palette_keeps_its_original_selection_and_revision() {
    let mut h = start_sheet("worksheet-controls-stale-fill");
    click_text(&mut h, "4.5");
    click(&mut h, "Format toolbar: Fill Color");
    let before = project(&h).clone();
    let id = book(&h);
    h.state_mut().books.get_mut(&id).unwrap().sel = Sel::Cells { anchor: (1, 2), end: (1, 2) };
    settle(&mut h);
    click(&mut h, "Fill Color: Red");
    assert_eq!(project(&h), &before);
    let request = h.state().text_format().unwrap().request(Change::Fill(Some(sheaf_core::Color::RED)));
    h.state_mut().doc.apply("Unrelated change", |p| p.workbook_mut(id).unwrap().name.push('x'));
    let changed = project(&h).clone();
    h.state_mut().apply(Action::FormatText(request), &egui::Context::default());
    assert_eq!(project(&h), &changed);
}

#[test]
fn formatting_a_locked_formula_column_preserves_its_values_and_formula() {
    let mut h = start_sheet("worksheet-controls-locked");
    click_text(&mut h, "4.5");
    let id = book(&h);
    let col = b(&h);
    h.state_mut().doc.apply("Seed formula", |p| {
        let c = &mut p.workbook_mut(id).unwrap().sheet_mut().columns[1];
        c.formula = Some("A * 2".into());
        c.recalc = sheaf_core::RecalcMode::Auto;
    });
    settle(&mut h);
    assert!(sheet(&h).columns[1].is_locked());
    let before = sheet(&h).columns.clone();
    let state = h.state_mut().books.get_mut(&id).unwrap();
    state.sel = Sel::Cells { anchor: (1, 1), end: (1, 1) };
    state.selected = vec![col];
    settle(&mut h);
    ctrl(&mut h, egui::Key::U);
    fill(&mut h, "Blue");
    assert!(sheet(&h).cell_format_at(col, 1).underline);
    assert_eq!(sheet(&h).cell_format_at(col, 1).fill, Some(sheaf_core::Color::BLUE));
    assert_eq!(sheet(&h).columns, before);
}

#[test]
fn toolbar_underline_and_fill_survive_project_and_companion_reopen() {
    let mut h = start_sheet("worksheet-controls-save");
    click_text(&mut h, "4.5");
    click(&mut h, "Format toolbar: Underline");
    fill(&mut h, "Blue");
    let before = sheet(&h).cell_formats.clone();
    let dir = save_dir("worksheet-controls-save");
    for ext in ["sheaf", "opj"] {
        let file = dir.join(format!("controls.{ext}"));
        h.state_mut().save_to(file.clone());
        let back = start(vec![file]);
        let col = b(&back);
        let style = sheet(&back).cell_format_at(col, 1);
        assert!(style.underline);
        assert_eq!(style.fill, Some(sheaf_core::Color::BLUE));
        assert_eq!(sheet(&back).cell_formats, before);
        assert_eq!(column(&back, "B"), [2.5, 4.5, 6.5, 8.5]);
    }
}

#[test]
fn multiple_formatting_keys_in_one_frame_keep_their_order_and_undo_history() {
    let mut h = start_sheet("worksheet-controls-key-batch");
    click_text(&mut h, "4.5");
    let col = b(&h);
    for key in [egui::Key::B, egui::Key::I, egui::Key::B] {
        for pressed in [true, false] {
            h.event(egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            });
        }
    }
    h.run();
    settle(&mut h);
    let style = sheet(&h).cell_format_at(col, 1);
    assert!(!style.bold && style.italic);
    menu(&mut h, "Edit", "Undo");
    let style = sheet(&h).cell_format_at(col, 1);
    assert!(style.bold && style.italic);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
    let rev = h.state().doc.revision();
    change(&mut h, Change::Underline(false));
    assert_eq!(h.state().doc.revision(), rev);
}
