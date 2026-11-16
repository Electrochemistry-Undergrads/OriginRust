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
