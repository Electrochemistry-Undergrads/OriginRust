use super::*;

fn start_sheet(tag: &str) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(tag)]);
    show_window(&mut h, "Book1 - sample.csv");
    click_text(&mut h, "4.5");
    h
}

fn extra(h: &mut Harness<'static, SheafApp>, fill: bool, name: &str) {
    let prefix = if fill { "Fill Color" } else { "Font Color" };
    click(h, &format!("Format toolbar: {prefix}"));
    click(h, &format!("{prefix}: More Classic colors"));
    click(h, &format!("{prefix}: {name}"));
    assert!(h.query_by_label(&format!("{prefix}: Black")).is_none(), "the root menu must close too");
}

#[test]
fn extra_presets_change_only_the_selected_cell_and_close_both_menus() {
    let mut h = start_sheet("worksheet-classic-cell");
    let b = sheet(&h).column_by_name("B").unwrap().id;
    click(&mut h, "Format toolbar: Bold");
    click(&mut h, "Format toolbar: Underline");
    let before = project(&h).clone();
    extra(&mut h, false, "Dark Yellow");
    assert_eq!(sheet(&h).cell_format_at(b, 1).text_color, Some(sheaf_core::Color::rgb(128, 128, 0)));
    let text = project(&h).clone();
    extra(&mut h, true, "Dark Cyan");
    let style = sheet(&h).cell_format_at(b, 1);
    assert_eq!(style.fill, Some(sheaf_core::Color::rgb(0, 128, 128)));
    assert_eq!(style.text_color, Some(sheaf_core::Color::rgb(128, 128, 0)));
    assert!(style.bold && style.underline);
    assert_eq!(sheet(&h).cell_format_at(b, 0).fill, None);
    assert_eq!(sheet(&h).cell_format_at(b, 0).text_color, None);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
    assert_eq!(h.state().doc.undo_label(), Some("Format cells"));
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &text);
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
}

// Expected RGB values measured from both Origin 2016 worksheet palettes.
// Names: https://docs.originlab.com/labtalk/ref/list-of-colors/
const CLASSIC: [(&str, [u8; 3]); 24] = [
    ("Black", [0, 0, 0]),
    ("Red", [255, 0, 0]),
    ("Green", [0, 255, 0]),
    ("Blue", [0, 0, 255]),
    ("Cyan", [0, 255, 255]),
    ("Magenta", [255, 0, 255]),
    ("Yellow", [255, 255, 0]),
    ("Dark Yellow", [128, 128, 0]),
    ("Navy", [0, 0, 128]),
    ("Purple", [128, 0, 128]),
    ("Wine", [128, 0, 0]),
    ("Olive", [0, 128, 0]),
    ("Dark Cyan", [0, 128, 128]),
    ("Royal", [0, 0, 160]),
    ("Orange", [255, 128, 0]),
    ("Violet", [128, 0, 255]),
    ("Pink", [255, 0, 128]),
    ("White", [255, 255, 255]),
    ("LT Gray", [192, 192, 192]),
    ("Gray", [128, 128, 128]),
    ("LT Yellow", [255, 255, 128]),
    ("LT Cyan", [128, 255, 255]),
    ("LT Magenta", [255, 128, 255]),
    ("Dark Gray", [64, 64, 64]),
];

fn preset(h: &mut Harness<'static, SheafApp>, fill: bool, name: &str) {
    if ["Black", "Red", "Green", "Blue", "Cyan", "Magenta", "Yellow", "White"].contains(&name) {
        let prefix = if fill { "Fill Color" } else { "Font Color" };
        click(h, &format!("Format toolbar: {prefix}"));
        click(h, &format!("{prefix}: {name}"));
    } else {
        extra(h, fill, name);
    }
}

#[test]
fn all_24_classic_text_and_fill_choices_match_native_rgb_and_keep_values() {
    let mut h = start_sheet("worksheet-classic-all");
    let b = sheet(&h).column_by_name("B").unwrap().id;
    let columns = sheet(&h).columns.clone();
    preset(&mut h, false, "Red");
    for (name, [r, g, blue]) in CLASSIC {
        let expected = sheaf_core::Color::rgb(r, g, blue);
        preset(&mut h, false, name);
        assert_eq!(sheet(&h).cell_format_at(b, 1).text_color, Some(expected), "text: {name}");
        preset(&mut h, true, name);
        assert_eq!(sheet(&h).cell_format_at(b, 1).fill, Some(expected), "fill: {name}");
        assert_eq!(sheet(&h).cell_format_at(b, 0).text_color, None);
        assert_eq!(sheet(&h).cell_format_at(b, 0).fill, None);
    }
    assert_eq!(sheet(&h).columns, columns);
    let revision = h.state().doc.revision();
    preset(&mut h, true, "Dark Gray");
    assert_eq!(h.state().doc.revision(), revision, "repeating a preset creates no history");
    preset(&mut h, false, "Dark Gray");
    assert_eq!(h.state().doc.revision(), revision);
    click(&mut h, "Format toolbar: Fill Color");
    click(&mut h, "Fill Color: None");
    assert_eq!(sheet(&h).cell_format_at(b, 1).fill, None);
    assert_eq!(sheet(&h).cell_format_at(b, 1).text_color, Some(sheaf_core::Color::rgb(64, 64, 64)));
}

#[test]
fn extra_presets_keep_whole_columns_compact_labels_separate_and_roundtrip() {
    use sheaf_core::cellformat::LabelKind;
    let mut h = start_sheet("worksheet-classic-targets");
    let book = project(&h).workbooks().next().unwrap().id;
    let b = sheet(&h).column_by_name("B").unwrap().id;
    let a = sheet(&h).column_by_name("A").unwrap().id;
    let state = h.state_mut().books.get_mut(&book).unwrap();
    state.sel = Sel::Columns;
    state.selected = vec![b];
    settle(&mut h);
    extra(&mut h, false, "Royal");
    assert_eq!(sheet(&h).cell_format_at(b, 1_000_000).text_color, Some(sheaf_core::Color::rgb(0, 0, 160)));
    assert_eq!(sheet(&h).cell_format_at(a, 1_000_000).text_color, None);
    assert_eq!(sheet(&h).format_index().resolve_label(sheet(&h), b, &LabelKind::LongName).text_color, None);
    assert!(sheet(&h).cell_formats.len() <= 1);
    let state = h.state_mut().books.get_mut(&book).unwrap();
    state.sel = Sel::Label { column: b, row: LabelRow::LongName };
    state.selected = vec![b];
    settle(&mut h);
    extra(&mut h, true, "LT Magenta");
    let before = sheet(&h).cell_formats.clone();
    assert!(before.len() <= 2);
    assert_eq!(sheet(&h).cell_format_at(b, 0).fill, None);
    let dir = save_dir("worksheet-classic-save");
    for ext in ["sheaf", "opj"] {
        let file = dir.join(format!("classic-colors.{ext}"));
        h.state_mut().save_to(file.clone());
        let back = start(vec![file]);
        let column = sheet(&back).column_by_name("B").unwrap().id;
        assert_eq!(sheet(&back).cell_formats, before);
        assert_eq!(sheet(&back).cell_format_at(column, 1_000_000).text_color, Some(sheaf_core::Color::rgb(0, 0, 160)));
        assert_eq!(
            sheet(&back).format_index().resolve_label(sheet(&back), column, &LabelKind::LongName).fill,
            Some(sheaf_core::Color::rgb(255, 128, 255))
        );
        assert_eq!(super::column(&back, "B"), [2.5, 4.5, 6.5, 8.5]);
    }
}

#[test]
fn extra_presets_reject_stale_selection_revision_and_project_generation() {
    for scenario in 0..3 {
        let mut h = start_sheet(&format!("worksheet-classic-stale-{scenario}"));
        let book = project(&h).workbooks().next().unwrap().id;
        click(&mut h, "Format toolbar: Font Color");
        click(&mut h, "Font Color: More Classic colors");
        match scenario {
            0 => h.state_mut().books.get_mut(&book).unwrap().sel = Sel::Cells { anchor: (1, 2), end: (1, 2) },
            1 => {
                h.state_mut().doc.apply("Unrelated change", |p| p.workbook_mut(book).unwrap().name.push('x'));
            }
            _ => {
                let replacement = project(&h).clone();
                h.state_mut().doc.replace(replacement);
            }
        }
        settle(&mut h);
        let before = project(&h).clone();
        let revision = h.state().doc.revision();
        if h.query_by_label("Font Color: Royal").is_some() {
            click(&mut h, "Font Color: Royal");
        }
        assert_eq!(project(&h), &before, "stale scenario {scenario}");
        assert_eq!(h.state().doc.revision(), revision);
        assert!(sheet(&h).cell_formats.is_empty());
    }
}

#[test]
fn classic_submenus_keep_custom_rgb_reachable_and_graph_palette_independent() {
    let mut h = start_sheet("worksheet-classic-graph-guard");
    extra(&mut h, false, "Olive");
    let before = project(&h).clone();
    click(&mut h, "Format toolbar: Font Color");
    click(&mut h, "Font Color: Custom...");
    assert!(h.state().worksheet_color.is_some());
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &before);
    let id = graph(&h).id;
    let title = project(&h).windows.iter().find(|w| w.id() == id).unwrap().title();
    show_window(&mut h, &title);
    let at = on(&h, 0, ObjectKind::Legend);
    click_at(&mut h, at);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Legend)));
    click(&mut h, "Format toolbar: Font Color");
    assert!(h.query_by_label("Font Color: Green").is_some());
    assert!(h.query_by_label("Font Color: More Classic colors").is_none());
    assert!(h.query_by_label("Font Color: Royal").is_none());
    assert!(h.query_by_label("Font Color: Custom...").is_none());
    assert_eq!(project(&h), &before);
}
