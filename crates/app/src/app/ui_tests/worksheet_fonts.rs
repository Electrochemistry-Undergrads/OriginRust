use super::*;
use sheaf_core::cellformat::{CellFormat, FormatTarget, LabelKind};

fn start_sheet(tag: &str) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(tag)]);
    show_window(&mut h, "Book1 - sample.csv");
    h
}

fn open(h: &mut Harness<'static, SheafApp>) {
    h.key_press(egui::Key::F4);
    settle(h);
    assert!(h.state().worksheet_fonts.is_some());
    assert!(h.state().cell_format.is_none(), "worksheet scope has a separate dialog");
}

fn size(h: &mut Harness<'static, SheafApp>, pt: &str) {
    let at = h.get_by_label("Worksheet font size").rect().center();
    double_click_at(h, at);
    ctrl(h, egui::Key::A);
    h.event(egui::Event::Text(pt.into()));
    h.run();
    h.key_press(egui::Key::Enter);
    settle(h);
}

fn painted_size(h: &Harness<'static, SheafApp>, text: &str) -> Vec<f32> {
    fn walk(shape: &egui::Shape, text: &str, out: &mut Vec<f32>) {
        match shape {
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, text, out)),
            egui::Shape::Text(t) if t.galley.job.text == text => {
                out.extend(t.galley.job.sections.iter().map(|s| s.format.font_id.size));
            }
            _ => {}
        }
    }
    let mut sizes = vec![];
    for shape in &h.output().shapes {
        walk(&shape.shape, text, &mut sizes);
    }
    sizes
}

#[test]
fn f4_applies_to_data_without_a_selection_and_rebuilds_the_grid_cache() {
    let mut h = start_sheet("worksheet-fonts-f4");
    let id = project(&h).workbooks().next().unwrap().id;
    h.state_mut().books.get_mut(&id).unwrap().sel = Sel::None;
    h.state_mut().books.get_mut(&id).unwrap().selected.clear();
    settle(&mut h);
    let before = project(&h).clone();
    open(&mut h);
    size(&mut h, "18");
    click(&mut h, "Apply");
    assert!(h.state().worksheet_fonts.is_some());
    assert!(sheet(&h).cell_formats.is_empty(), "one sparse default, no per-cell records");
    assert_eq!(sheet(&h).columns, before.workbook(id).unwrap().sheet().columns);
    assert_eq!(h.state().doc.undo_label(), Some("Worksheet font"));
    assert_eq!(painted_size(&h, "4.5"), [24.0], "pt to screen px, including initially empty format cache");
    let b = sheet(&h).column_by_name("B").unwrap().id;
    assert!(sheet(&h).format_index().resolve_label(sheet(&h), b, &LabelKind::LongName).is_default());
    let rev = h.state().doc.revision();
    click(&mut h, "OK");
    assert!(h.state().worksheet_fonts.is_none());
    assert_eq!(h.state().doc.revision(), rev, "unchanged OK has no undo");
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
    assert_eq!(painted_size(&h, "4.5"), [12.0]);
    menu(&mut h, "Edit", "Redo");
    assert_eq!(painted_size(&h, "4.5"), [24.0]);
    h.state_mut().doc.apply("Add future column", |p| {
        p.add_column(id, 0, sheaf_core::Designation::Y, sheaf_core::ColumnData::numeric(vec![101.0])).unwrap();
    });
    settle(&mut h);
    let c = sheet(&h).columns.last().unwrap().id;
    assert_eq!(sheet(&h).cell_format_at(c, 999_999).size_pt, Some(18.0));
    assert_eq!(painted_size(&h, "101"), [24.0]);
}

#[test]
fn manual_cell_size_wins_and_apply_cancel_are_separate_undo_steps() {
    let mut h = start_sheet("worksheet-fonts-manual");
    let id = project(&h).workbooks().next().unwrap().id;
    let b = sheet(&h).column_by_name("B").unwrap().id;
    h.state_mut().doc.apply("Manual font", |p| {
        p.workbook_mut(id).unwrap().sheet_mut().apply_cell_format(
            FormatTarget::cells(vec![b], 1, Some(1)),
            CellFormat { size_pt: Some(12.0), bold: Some(true), ..Default::default() },
        );
    });
    settle(&mut h);
    let original = project(&h).clone();
    menu_path(&mut h, &["Format", "Worksheet..."]);
    assert!(h.state().worksheet_fonts.is_some());
    size(&mut h, "20");
    click(&mut h, "Apply");
    let applied = project(&h).clone();
    let manual = sheet(&h).cell_format_at(b, 1);
    assert_eq!(manual.size_pt, Some(12.0));
    assert!(manual.bold);
    assert_eq!(sheet(&h).cell_format_at(b, 2).size_pt, Some(20.0));
    assert!(painted_size(&h, "4.5").iter().all(|s| *s == 16.0));
    assert_eq!(painted_size(&h, "6.5"), [20.0 * 4.0 / 3.0]);
    size(&mut h, "24");
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &applied);
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &original);
    menu(&mut h, "Edit", "Redo");
    assert_eq!(project(&h), &applied);
}

#[test]
fn cancel_escape_unchanged_ok_and_stale_targets_do_not_change_the_document() {
    let mut h = start_sheet("worksheet-fonts-stale");
    let original = project(&h).clone();
    let rev = h.state().doc.revision();
    open(&mut h);
    size(&mut h, "18");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().worksheet_fonts.is_none());
    assert_eq!(project(&h), &original);
    open(&mut h);
    click(&mut h, "OK");
    assert_eq!(h.state().doc.revision(), rev);
    open(&mut h);
    size(&mut h, "18");
    h.state_mut().doc.apply("Other change", |p| p.root.name = "Changed elsewhere".into());
    let changed = project(&h).clone();
    settle(&mut h);
    assert!(h.state().worksheet_fonts.is_none(), "a stale draft closes before it can write");
    assert_eq!(project(&h), &changed);
    open(&mut h);
    size(&mut h, "18");
    h.state_mut().doc.replace(changed.clone());
    settle(&mut h);
    assert!(h.state().worksheet_fonts.is_none(), "replacement of an identical project still invalidates the draft");
    assert_eq!(project(&h), &changed);
}

#[test]
fn worksheet_fonts_survive_native_project_and_opj_companion_reopen() {
    let mut h = start_sheet("worksheet-fonts-save");
    let file = data_file("worksheet-fonts-output");
    let names: Vec<_> = sheaf_plot::SystemFonts::shared()
        .database()
        .faces()
        .filter_map(|face| face.families.first().map(|(name, _)| name.clone()))
        .collect();
    let chosen = names
        .iter()
        .find(|n| n.as_str() == "Times New Roman")
        .unwrap_or_else(|| names.first().expect("installed fixture font"))
        .clone();
    open(&mut h);
    size(&mut h, "18");
    h.get_by_value("Automatic").click();
    settle(&mut h);
    click(&mut h, "Worksheet font search");
    h.event(egui::Event::Text(chosen.clone()));
    settle(&mut h);
    click(&mut h, &chosen);
    assert!(h.query_by_label("Worksheet font search").is_none(), "choosing a family closes the popup");
    click(&mut h, "OK");
    assert_eq!(sheet(&h).font_defaults.data.as_ref().unwrap().family, chosen);
    let svg = sheaf_plot::sheet_image_svg(sheet(&h)).unwrap();
    assert!(svg.contains(&chosen), "worksheet image uses the resolved default face");
    let opj = sheaf_io::opj::write(project(&h)).unwrap();
    assert!(
        opj.warnings.iter().any(|w| w.contains("cell formatting")),
        "default fonts alone are reported as unsupported OPJ formatting"
    );
    for extension in ["sheaf", "opj"] {
        let path = file.with_extension(extension);
        h.state_mut().save_to(path.clone());
        if extension == "opj" {
            assert!(sheaf_io::save::companion_path(&path).exists(), "defaults alone require a companion");
        }
        let back = start(vec![path]);
        let s = sheet(&back);
        assert_eq!(s.font_defaults, sheet(&h).font_defaults);
        assert!(s.cell_formats.is_empty());
        assert_eq!(s.cell_format_at(s.columns[0].id, 999_999).size_pt, Some(18.0));
        assert_eq!(column(&back, "B"), [2.5, 4.5, 6.5, 8.5]);
    }
}

fn category(h: &mut Harness<'static, SheafApp>, current: &str, next: &str) {
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value(current)).click();
    settle(h);
    h.get_by_role_and_label(egui::accesskit::Role::Button, next).click();
    settle(h);
}

#[test]
fn worksheet_header_fonts_reach_titles_and_row_numbers_with_one_undo() {
    let mut h = start_sheet("worksheet-header-fonts");
    let book = project(&h).workbooks().next().unwrap().id;
    h.state_mut().doc.apply("Seed label", |p| {
        p.workbook_mut(book).unwrap().sheet_mut().columns[0].long_name = "Alpha".into();
    });
    settle(&mut h);
    let before = project(&h).clone();
    let heading = sheaf_core::sheet_print::head_name(sheet(&h), 0);
    let before_heading = painted_size(&h, &heading);
    let before_caption = painted_size(&h, "Long Name");
    let before_row = painted_size(&h, "2");
    let before_data = painted_size(&h, "4.5");

    open(&mut h);
    category(&mut h, "Data", "Column Header");
    size(&mut h, "18");
    category(&mut h, "Column Header", "Row Header");
    size(&mut h, "12");
    assert_eq!(project(&h), &before);
    click(&mut h, "OK");

    assert_eq!(painted_size(&h, &heading), [24.0]);
    // Project Explorer also paints a column called Long Name.
    assert!(painted_size(&h, "Long Name").contains(&16.0));
    assert!(painted_size(&h, "2").contains(&16.0));
    assert_eq!(painted_size(&h, "4.5"), before_data);
    assert!(sheet(&h).format_index().is_empty());
    assert_eq!(h.state().doc.undo_label(), Some("Worksheet font"));
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
    assert_eq!(painted_size(&h, &heading), before_heading);
    assert_eq!(painted_size(&h, "Long Name"), before_caption);
    assert_eq!(painted_size(&h, "2"), before_row);
}

fn label_family_fixture() -> String {
    let names: Vec<_> = sheaf_plot::SystemFonts::shared()
        .database()
        .faces()
        .filter_map(|face| face.families.first().map(|(name, _)| name.clone()))
        .collect();
    names
        .iter()
        .find(|name| ["Times New Roman", "DejaVu Serif", "Liberation Serif"].contains(&name.as_str()))
        .or_else(|| names.first())
        .expect("installed font fixture")
        .clone()
}

fn family(h: &mut Harness<'static, SheafApp>, current: &str, next: &str) {
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value(current)).click();
    settle(h);
    if next != "Automatic" {
        click(h, "Worksheet font search");
        ctrl(h, egui::Key::A);
        h.event(egui::Event::Text(next.into()));
        settle(h);
    }
    h.get_by_role_and_label(egui::accesskit::Role::Button, next).click();
    settle(h);
}

#[test]
fn label_family_drafts_keep_sizes_sparse_and_apply_undo_cancel_and_automatic() {
    let mut h = start_sheet("worksheet-label-family-drafts");
    let chosen = label_family_fixture();
    let id = project(&h).workbooks().next().unwrap().id;
    h.state_mut().doc.apply("Seed label", |p| {
        p.workbook_mut(id).unwrap().sheet_mut().columns[0].long_name = "Alpha".into();
    });
    settle(&mut h);
    let before = project(&h).clone();
    open(&mut h);
    category(&mut h, "Data", "Long Name");
    family(&mut h, "Automatic", &chosen);
    category(&mut h, "Long Name", "Units");
    family(&mut h, "Automatic", &chosen);
    category(&mut h, "Units", "Comments");
    family(&mut h, "Automatic", &chosen);
    click(&mut h, "Apply");
    let s = sheet(&h);
    for kind in [LabelKind::LongName, LabelKind::Units, LabelKind::Comments] {
        assert_eq!(s.font_defaults.label_family(&kind), Some(chosen.as_str()));
        assert!(s.font_defaults.label_size(&kind).is_none(), "family edits must not create point-size overrides");
    }
    assert!(s.cell_formats.is_empty());
    assert!(s.font_defaults.data.is_none());
    assert_eq!(painted_size(&h, "4.5"), [12.0]);
    let svg = sheaf_plot::sheet_image_svg(s).unwrap();
    assert!(svg.contains(&chosen) && svg.contains("Alpha"));
    let applied = project(&h).clone();
    let rev = h.state().doc.revision();
    click(&mut h, "OK");
    assert_eq!(h.state().doc.revision(), rev);
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
    menu(&mut h, "Edit", "Redo");
    assert_eq!(project(&h), &applied);
    open(&mut h);
    category(&mut h, "Data", "Long Name");
    family(&mut h, &chosen, "Automatic");
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &applied);
    open(&mut h);
    category(&mut h, "Data", "Long Name");
    family(&mut h, &chosen, "Automatic");
    click(&mut h, "OK");
    assert!(sheet(&h).font_defaults.long_name_family.is_none());
    assert_eq!(sheet(&h).font_defaults.units_family.as_deref(), Some(chosen.as_str()));
    assert!(sheet(&h).font_defaults.long_name_size_pt.is_none());
}

#[test]
fn label_family_only_defaults_save_reopen_and_reach_new_columns_without_data_fonts() {
    let mut h = start_sheet("worksheet-label-family-save");
    let chosen = label_family_fixture();
    let id = project(&h).workbooks().next().unwrap().id;
    open(&mut h);
    category(&mut h, "Data", "Long Name");
    family(&mut h, "Automatic", &chosen);
    size(&mut h, "18");
    category(&mut h, "Long Name", "Units");
    family(&mut h, "Automatic", &chosen);
    click(&mut h, "OK");
    h.state_mut().doc.apply("Future label", |p| {
        let col =
            p.add_column(id, 0, sheaf_core::Designation::Y, sheaf_core::ColumnData::numeric(vec![101.0])).unwrap();
        let c = p.workbook_mut(id).unwrap().sheet_mut().column_mut(col).unwrap();
        c.long_name = "Gamma".into();
        c.width = Some(20);
    });
    settle(&mut h);
    assert_eq!(painted_size(&h, "Gamma"), [24.0]);
    assert_eq!(painted_size(&h, "101"), [12.0]);
    let c = sheet(&h).columns.last().unwrap().id;
    assert_eq!(sheet(&h).format_index().resolve_label(sheet(&h), c, &LabelKind::LongName).font, Some(chosen.as_str()));
    assert!(sheaf_io::opj::write(project(&h)).unwrap().warnings.iter().any(|w| w.contains("cell formatting")));
    for extension in ["sheaf", "opj"] {
        let path = data_file("worksheet-label-family-output").with_extension(extension);
        h.state_mut().save_to(path.clone());
        if extension == "opj" {
            assert!(sheaf_io::save::companion_path(&path).exists());
        }
        let back = start(vec![path]);
        assert_eq!(sheet(&back).font_defaults, sheet(&h).font_defaults);
        assert!(sheet(&back).cell_formats.is_empty());
        assert!(sheet(&back).font_defaults.data.is_none());
        assert!(sheet(&back).font_defaults.units_size_pt.is_none());
        assert_eq!(column(&back, "B"), [2.5, 4.5, 6.5, 8.5]);
    }
}

#[test]
fn bulk_label_font_drafts_keep_specific_sizes_and_can_restore_inheritance() {
    let mut h = start_sheet("worksheet-label-bulk");
    let chosen = label_family_fixture();
    let book = project(&h).workbooks().next().unwrap().id;
    h.state_mut().doc.apply("Seed labels", |p| {
        let sheet = p.workbook_mut(book).unwrap().sheet_mut();
        sheet.columns[0].long_name = "Alpha".into();
        sheet.columns[0].units = "mV".into();
    });
    settle(&mut h);
    let before = project(&h).clone();
    open(&mut h);
    category(&mut h, "Data", "Column Labels");
    family(&mut h, "Automatic", &chosen);
    size(&mut h, "20");
    category(&mut h, "Column Labels", "Long Name");
    size(&mut h, "9");
    assert_eq!(project(&h), &before);
    click(&mut h, "Apply");
    assert_eq!(painted_size(&h, "Alpha"), [12.0]);
    assert_eq!(painted_size(&h, "mV"), [20.0 * 4.0 / 3.0]);
    assert_eq!(painted_size(&h, "4.5"), [12.0]);
    assert_eq!(sheet(&h).font_defaults.column_labels.family, chosen);
    assert_eq!(sheet(&h).font_defaults.long_name_size_pt, Some(9.0));
    assert!(sheet(&h).cell_formats.is_empty());

    let applied = project(&h).clone();
    size(&mut h, "0");
    click(&mut h, "OK");
    assert_eq!(sheet(&h).font_defaults.long_name_size_pt, None);
    assert_eq!(painted_size(&h, "Alpha"), [20.0 * 4.0 / 3.0]);
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &applied);
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
}

#[test]
fn label_category_drafts_apply_together_with_manual_priority_and_one_undo() {
    let mut h = start_sheet("worksheet-label-font-drafts");
    let id = project(&h).workbooks().next().unwrap().id;
    let b = sheet(&h).columns[1].id;
    h.state_mut().doc.apply("Seed label styles", |p| {
        let sheet = p.workbook_mut(id).unwrap().sheet_mut();
        sheet.columns[0].long_name = "Alpha".into();
        sheet.columns[1].long_name = "Beta".into();
        sheet.columns[0].units = "mV".into();
        sheet.columns[0].comments = "Note".into();
        sheet.apply_cell_format(
            FormatTarget::label_rows(vec![b], vec![LabelKind::LongName]),
            CellFormat { size_pt: Some(12.0), bold: Some(true), ..Default::default() },
        );
    });
    settle(&mut h);
    let before = project(&h).clone();
    open(&mut h);
    category(&mut h, "Data", "Long Name");
    size(&mut h, "18");
    category(&mut h, "Long Name", "Units");
    size(&mut h, "20");
    category(&mut h, "Units", "Comments");
    size(&mut h, "24");
    category(&mut h, "Comments", "Long Name");
    click(&mut h, "Apply");
    assert_eq!(painted_size(&h, "Alpha"), [24.0]);
    assert_eq!(painted_size(&h, "Beta"), [16.0]);
    assert_eq!(painted_size(&h, "mV"), [20.0 * 4.0 / 3.0]);
    assert_eq!(painted_size(&h, "Note"), [32.0]);
    assert_eq!(painted_size(&h, "4.5"), [12.0]);
    assert!(sheet(&h).font_defaults.data.is_none());
    assert_eq!(sheet(&h).cell_formats.len(), 1);
    let applied = project(&h).clone();
    let rev = h.state().doc.revision();
    click(&mut h, "OK");
    assert_eq!(h.state().doc.revision(), rev);
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
    menu(&mut h, "Edit", "Redo");
    assert_eq!(project(&h), &applied);
    open(&mut h);
    category(&mut h, "Data", "Comments");
    size(&mut h, "30");
    category(&mut h, "Comments", "Units");
    size(&mut h, "40");
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &applied);
}

#[test]
fn label_only_defaults_reopen_in_both_project_formats_and_style_future_columns() {
    let mut h = start_sheet("worksheet-label-font-save");
    let id = project(&h).workbooks().next().unwrap().id;
    open(&mut h);
    category(&mut h, "Data", "Long Name");
    size(&mut h, "18");
    category(&mut h, "Long Name", "Units");
    size(&mut h, "12");
    category(&mut h, "Units", "Comments");
    size(&mut h, "24");
    click(&mut h, "OK");
    h.state_mut().doc.apply("New column", |p| {
        let col =
            p.add_column(id, 0, sheaf_core::Designation::Y, sheaf_core::ColumnData::numeric(vec![101.0])).unwrap();
        let column = p.workbook_mut(id).unwrap().sheet_mut().column_mut(col).unwrap();
        column.long_name = "Gamma".into();
        column.width = Some(20);
    });
    settle(&mut h);
    assert_eq!(painted_size(&h, "Gamma"), [24.0]);
    assert_eq!(painted_size(&h, "101"), [12.0]);
    let out = sheaf_io::opj::write(project(&h)).unwrap();
    assert!(out.warnings.iter().any(|w| w.contains("cell formatting")));
    let svg = sheaf_plot::sheet_image_svg(sheet(&h)).unwrap();
    assert!(svg.contains("Gamma"));
    for extension in ["sheaf", "opj"] {
        let path = data_file("worksheet-label-font-output").with_extension(extension);
        h.state_mut().save_to(path.clone());
        if extension == "opj" {
            assert!(sheaf_io::save::companion_path(&path).exists());
        }
        let back = start(vec![path]);
        assert_eq!(sheet(&back).font_defaults, sheet(&h).font_defaults);
        assert!(sheet(&back).cell_formats.is_empty());
        let s = sheet(&back);
        for c in &s.columns {
            assert_eq!(s.format_index().resolve_label(s, c.id, &LabelKind::LongName).size_pt, Some(18.0));
            assert!(s.cell_format_at(c.id, 999_999).is_default());
        }
        assert_eq!(column(&back, "B"), [2.5, 4.5, 6.5, 8.5]);
    }
}
