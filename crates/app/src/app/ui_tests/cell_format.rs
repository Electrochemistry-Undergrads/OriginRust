use super::*;
use sheaf_core::Frozen;
use sheaf_core::cellformat::{CellFormat, FormatTarget, HAlign, LabelKind};
use sheaf_core::color::Color;
use sheaf_core::condformat;

fn start_book(tag: &str) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(tag)]);
    show_window(&mut h, "Book1 - sample.csv");
    h
}

fn col_id(h: &Harness<'static, SheafApp>, short: &str) -> sheaf_core::ColumnId {
    sheet(h).column_by_name(short).unwrap().id
}

fn rgb(c: Color) -> egui::Color32 {
    egui::Color32::from_rgb(c.r, c.g, c.b)
}

/// Every text the last frame painted: the galley's text, where it starts and its first format.
fn texts(h: &Harness<'static, SheafApp>) -> Vec<(String, egui::Pos2, egui::text::TextFormat)> {
    fn walk(shape: &egui::Shape, out: &mut Vec<(String, egui::Pos2, egui::text::TextFormat)>) {
        match shape {
            egui::Shape::Text(t) => {
                if let Some(s) = t.galley.job.sections.first() {
                    out.push((t.galley.text().to_string(), t.pos, s.format.clone()));
                }
            }
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &h.output().shapes {
        walk(&clipped.shape, &mut out);
    }
    out
}

/// The painted copies of `text`: bold text is drawn twice, a fraction of a pixel apart.
fn runs(h: &Harness<'static, SheafApp>, text: &str) -> Vec<(egui::Pos2, egui::text::TextFormat)> {
    texts(h).into_iter().filter(|(t, _, _)| t == text).map(|(_, p, f)| (p, f)).collect()
}

fn painted(h: &Harness<'static, SheafApp>) -> Vec<(egui::Rect, egui::Color32)> {
    fn walk(shape: &egui::Shape, out: &mut Vec<(egui::Rect, egui::Color32)>) {
        match shape {
            egui::Shape::Rect(r) => out.push((r.rect, r.fill)),
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &h.output().shapes {
        walk(&clipped.shape, &mut out);
    }
    out
}

/// The last filled rectangle behind the centre of the cell that shows `text`, among `colours`.
fn fill_behind(h: &Harness<'static, SheafApp>, text: &str, colours: &[egui::Color32]) -> Option<egui::Color32> {
    let centre = grid_text(h, text).rect().center();
    painted(h).into_iter().rev().find(|(r, c)| r.contains(centre) && colours.contains(c)).map(|(_, c)| c)
}

const PALE: Color = Color::rgb(255, 255, 153);

/// Opens Format Cells from the menu of the cell showing `text`.
fn open_from_cell(h: &mut Harness<'static, SheafApp>, text: &str) {
    click_text(h, text);
    right_click_text(h, text);
    menu_path(h, &["Format Cells..."]);
    assert!(h.state().cell_format.is_some(), "Format Cells opened");
}

#[test]
fn ok_from_the_cell_menu_formats_the_cell_in_one_undo_step() {
    let mut h = start_book("cfmt-ok");
    let before = h.state().doc.undo_label().map(str::to_string);
    open_from_cell(&mut h, "4.5");
    click(&mut h, "Bold");
    click(&mut h, "Italic");
    click(&mut h, "OK");
    assert!(h.state().cell_format.is_none());
    let b = col_id(&h, "B");
    assert_eq!(h.state().doc.undo_label(), Some("Format cells"));
    assert_eq!(sheet(&h).cell_formats.len(), 1, "one record");
    let f = sheet(&h).cell_format_at(b, 1);
    assert!(f.bold && f.italic && !f.underline);
    assert!(!sheet(&h).cell_format_at(b, 0).bold && !sheet(&h).cell_format_at(b, 2).bold);
    // Select another cell so the black selection does not change the colours.
    click_text(&mut h, "6.5");
    let drawn = runs(&h, "4.5");
    assert_eq!(drawn.len(), 2, "bold text is drawn twice");
    assert!(drawn.iter().all(|(_, f)| f.italics));
    assert!((drawn[1].0.x - drawn[0].0.x).abs() > 0.1 && (drawn[1].0.y - drawn[0].0.y).abs() < 0.01);
    assert_eq!(runs(&h, "6.5").len(), 1, "other cells are plain");
    assert!(!runs(&h, "6.5")[0].1.italics);
    // One undo undoes the whole format; redo brings it back.
    h.state_mut().doc.undo();
    settle(&mut h);
    assert!(sheet(&h).cell_formats.is_empty());
    assert_eq!(h.state().doc.undo_label().map(str::to_string), before);
    assert_eq!(runs(&h, "4.5").len(), 1);
    h.state_mut().doc.redo();
    settle(&mut h);
    assert!(sheet(&h).cell_format_at(b, 1).bold);
}

#[test]
fn cancel_and_the_close_box_change_nothing() {
    let mut h = start_book("cfmt-cancel");
    let before = h.state().doc.undo_label().map(str::to_string);
    open_from_cell(&mut h, "4.5");
    click(&mut h, "Bold");
    click(&mut h, "Cancel");
    assert!(h.state().cell_format.is_none());
    assert!(sheet(&h).cell_formats.is_empty());
    assert_eq!(h.state().doc.undo_label().map(str::to_string), before);
    // OK without a change does not make an undo step either.
    open_from_cell(&mut h, "4.5");
    click(&mut h, "OK");
    assert!(sheet(&h).cell_formats.is_empty());
    assert_eq!(h.state().doc.undo_label().map(str::to_string), before);
}

#[test]
fn apply_keeps_the_window_open_and_each_apply_is_one_undo_step() {
    let mut h = start_book("cfmt-apply");
    open_from_cell(&mut h, "4.5");
    click(&mut h, "Bold");
    click(&mut h, "Apply");
    assert!(h.state().cell_format.is_some(), "Apply keeps the window");
    let b = col_id(&h, "B");
    assert!(sheet(&h).cell_format_at(b, 1).bold);
    click(&mut h, "Underline");
    click(&mut h, "OK");
    let f = sheet(&h).cell_format_at(b, 1);
    assert!(f.bold && f.underline);
    h.state_mut().doc.undo();
    let f = sheet(&h).cell_format_at(b, 1);
    assert!(f.bold && !f.underline, "the second step only");
    h.state_mut().doc.undo();
    assert!(sheet(&h).cell_formats.is_empty());
}

#[test]
fn a_column_menu_formats_the_whole_column_as_one_record_and_new_rows_follow() {
    let mut h = start_book("cfmt-column");
    click_text(&mut h, "B(Y1)");
    right_click_text(&mut h, "B(Y1)");
    menu_path(&mut h, &["Format Cells..."]);
    click(&mut h, "Use fill");
    click(&mut h, "Center");
    click(&mut h, "OK");
    let b = col_id(&h, "B");
    assert_eq!(sheet(&h).cell_formats.len(), 1);
    assert_eq!(sheet(&h).cell_formats[0].target, FormatTarget::Cells { columns: vec![b], rows: None });
    assert_eq!(sheet(&h).cell_format_at(b, 3).fill, Some(PALE));
    assert_eq!(sheet(&h).cell_format_at(b, 3).h_align, Some(HAlign::Center));
    assert_eq!(sheet(&h).cell_format_at(b, 100).fill, Some(PALE), "beyond the data too");
    click_text(&mut h, "C(Y1)");
    assert_eq!(fill_behind(&h, "4.5", &[rgb(PALE)]), Some(rgb(PALE)));
    assert_eq!(fill_behind(&h, "30.5", &[rgb(PALE)]), None, "column C is plain");
    // Centred text sits in the middle of its cell.
    let cell = grid_text(&h, "4.5").rect();
    let head = grid_text(&h, "B(Y1)").rect();
    assert!((cell.center().x - head.center().x).abs() < 6.0, "{cell:?} {head:?}");
}

#[test]
fn the_format_menu_and_the_row_menu_open_the_window() {
    let mut h = start_book("cfmt-menus");
    click_text(&mut h, "4.5");
    menu_path(&mut h, &["Format", "Cell..."]);
    assert!(h.state().cell_format.is_some());
    click(&mut h, "Cancel");
    // The row-number menu: the leftmost label with the row's number.
    let at = h
        .query_all(By::new().role(egui::accesskit::Role::Label).value("2"))
        .map(|n| n.rect())
        .min_by(|a, b| a.min.x.total_cmp(&b.min.x))
        .unwrap()
        .center();
    h.event(egui::Event::PointerMoved(at));
    h.event(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.event(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    settle(&mut h);
    h.event(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Secondary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.event(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Secondary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    settle(&mut h);
    menu_path(&mut h, &["Format Cells..."]);
    assert!(h.state().cell_format.is_some(), "the row menu opens it");
    click(&mut h, "Bold");
    click(&mut h, "OK");
    let s = sheet(&h);
    assert_eq!(s.cell_formats.len(), 1);
    for c in &s.columns {
        assert!(s.cell_format_at(c.id, 1).bold && !s.cell_format_at(c.id, 0).bold, "row 2 of every column");
    }
}

#[test]
fn clear_formats_removes_the_formats_of_the_selection_in_one_step() {
    let mut h = start_book("cfmt-clear");
    let b = col_id(&h, "B");
    h.state_mut().doc.apply("seed", |p| {
        let book = p.workbooks().next().unwrap().id;
        p.workbook_mut(book).unwrap().sheet_mut().apply_cell_format(
            FormatTarget::whole_columns(vec![b]),
            CellFormat { bold: Some(true), ..Default::default() },
        );
    });
    settle(&mut h);
    open_from_cell(&mut h, "4.5");
    click(&mut h, "Clear Formats");
    assert!(h.state().cell_format.is_none());
    assert_eq!(h.state().doc.undo_label(), Some("Clear formats"));
    assert!(!sheet(&h).cell_format_at(b, 1).bold && sheet(&h).cell_format_at(b, 0).bold, "only that cell was cleared");
    h.state_mut().doc.undo();
    assert!(sheet(&h).cell_format_at(b, 1).bold);
}

#[test]
fn a_conditional_colour_overrides_the_manual_fill_and_the_manual_bold_stays() {
    let mut h = start_book("cfmt-cond");
    // Column C: a manual pale fill and bold, then Highlight Min Max column-wise.
    click_text(&mut h, "C(Y1)");
    right_click_text(&mut h, "C(Y1)");
    menu_path(&mut h, &["Format Cells..."]);
    click(&mut h, "Use fill");
    click(&mut h, "Bold");
    click(&mut h, "OK");
    click_text(&mut h, "C(Y1)");
    menu_path(&mut h, &["Worksheet", "Conditional Formatting", "Highlight Min Max..."]);
    click(&mut h, "Column-wise");
    click(&mut h, "OK");
    click_text(&mut h, "B(Y1)");
    let all = [rgb(PALE), rgb(condformat::MAX_FILL), rgb(condformat::MIN_FILL)];
    assert_eq!(fill_behind(&h, "30.5", &all), Some(rgb(condformat::MAX_FILL)), "the maximum is conditional");
    assert_eq!(fill_behind(&h, "0.5", &all), Some(rgb(condformat::MIN_FILL)));
    assert_eq!(fill_behind(&h, "20.5", &all), Some(rgb(PALE)), "the rest keeps the manual fill");
    for t in ["30.5", "20.5"] {
        assert_eq!(runs(&h, t).len(), 2, "{t} stays bold");
    }
    // The conditional font colour wins over a manual text colour too.
    let c = col_id(&h, "C");
    let f = sheet(&h).cell_format_at(c, 0).with_conditional(None);
    assert!(f.bold);
    let drawn = runs(&h, "30.5");
    assert!(drawn.iter().all(|(_, f)| f.color == rgb(condformat::MAX_FONT)));
}

/// A file of 20 rows and 12 columns, each value naming its place: (r + 1) * 100 + c + 1.
fn wide_file(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("wide.csv");
    let mut text = (0..12).map(|c| format!("c{c}")).collect::<Vec<_>>().join(",");
    text.push('\n');
    for r in 0..20 {
        text.push_str(&(0..12).map(|c| ((r + 1) * 100 + c + 1).to_string()).collect::<Vec<_>>().join(","));
        text.push('\n');
    }
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn a_frozen_overlay_cell_and_a_label_cell_are_drawn_styled() {
    let mut h = start(vec![wide_file("cfmt-frozen")]);
    show_window(&mut h, "Book1 - wide.csv");
    let a = col_id(&h, "A");
    h.state_mut().doc.apply("seed", |p| {
        let book = p.workbooks().next().unwrap().id;
        let s = p.workbook_mut(book).unwrap().sheet_mut();
        s.apply_cell_format(
            FormatTarget::whole_columns(vec![a]),
            CellFormat { italic: Some(true), fill: Some(Color::rgb(10, 200, 30)), ..Default::default() },
        );
        s.apply_cell_format(
            FormatTarget::label_rows(vec![a], vec![LabelKind::LongName]),
            CellFormat { bold: Some(true), text_color: Some(Color::rgb(200, 0, 0)), ..Default::default() },
        );
    });
    settle(&mut h);
    // Freeze the first column (a click in the second one), scroll right: A stays, styled.
    click_text(&mut h, "102");
    key_with(&mut h, egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::F);
    assert_eq!(sheet(&h).frozen, Frozen { rows: 0, columns: 1 });
    let at = grid_text(&h, "205").rect().center();
    h.event(egui::Event::PointerMoved(at));
    h.step();
    for _ in 0..4 {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(-6.0, 0.0),
            modifiers: egui::Modifiers::NONE,
        });
        h.step();
    }
    h.run_steps(40);
    assert!(book_ui(&h).scroll_offsets().0 > 20.0, "the sheet scrolled right");
    let cell = runs(&h, "101");
    assert_eq!(cell.len(), 1, "the frozen cell is drawn once");
    assert!(cell[0].1.italics);
    let green = [egui::Color32::from_rgb(10, 200, 30)];
    assert_eq!(fill_behind(&h, "101", &green), Some(green[0]), "its fill is painted");
    assert_eq!(fill_behind(&h, "1101", &green), Some(green[0]));
    // The label row cell of A: bold (two copies) in the manual colour.
    let label = runs(&h, "c0");
    assert_eq!(label.len(), 2, "{label:?}");
    assert!(label.iter().all(|(_, f)| f.color == rgb(Color::rgb(200, 0, 0))));
    // Cells of other columns are plain.
    let other = texts(&h).into_iter().find(|(t, _, _)| t.len() == 3 && t.starts_with('1') && t != "101").unwrap();
    assert!(!other.2.italics);
}

/// Selects the block of cells and opens Format Cells for it.
fn open_for_block(h: &mut Harness<'static, SheafApp>, from: (usize, usize), to: (usize, usize)) {
    let ctx = h.ctx.clone();
    let book = project(h).workbooks().next().unwrap().id;
    let sh = sheet(h).clone();
    h.state_mut().books.get_mut(&book).unwrap().select_cells(&sh, from, to);
    h.state_mut().apply(Action::FormatCellsDialog(book), &ctx);
    settle(h);
    assert!(h.state().cell_format.is_some());
}

#[test]
fn a_sort_moves_the_format_with_its_row() {
    let mut h = start_book("cfmt-sort");
    let book = project(&h).workbooks().next().unwrap().id;
    open_for_block(&mut h, (1, 1), (1, 1)); // B2 = 4.5
    click(&mut h, "Bold");
    click(&mut h, "OK");
    let b = col_id(&h, "B");
    h.state_mut().doc.apply("sort", |p| {
        p.sort_sheet(book, 0, b, true).unwrap();
    });
    settle(&mut h);
    // 2.5, 4.5, 6.5, 8.5 became 8.5, 6.5, 4.5, 2.5: the 4.5 is now in row 3 (index 2).
    let s = sheet(&h);
    assert_eq!(s.column(b).unwrap().numeric().unwrap()[2], 4.5);
    assert!(s.cell_format_at(b, 2).bold && !s.cell_format_at(b, 1).bold);
    click_text(&mut h, "6.5");
    assert_eq!(runs(&h, "4.5").len(), 2, "the moved cell is drawn bold");
}

#[test]
fn a_filtered_selection_formats_its_whole_row_span_and_hidden_rows_stay_formatted() {
    let mut h = start_book("cfmt-filtered");
    let book = project(&h).workbooks().next().unwrap().id;
    // Keep the rows whose B is above 3: rows 1, 2, 3 (4.5, 6.5, 8.5) show, row 0 is hidden.
    h.state_mut().doc.apply("filter", |p| {
        p.set_row_filter(book, 0, "col(B) > 3").unwrap();
    });
    settle(&mut h);
    assert!(grid_text(&h, "4.5").rect().width() > 0.0);
    open_for_block(&mut h, (1, 1), (1, 3));
    click(&mut h, "Italic");
    click(&mut h, "OK");
    let b = col_id(&h, "B");
    for r in 1..=3 {
        assert!(sheet(&h).cell_format_at(b, r).italic, "row {r}");
    }
    assert!(!sheet(&h).cell_format_at(b, 0).italic);
    // Removing the filter shows every row: only the selected ones are italic.
    h.state_mut().doc.apply("unfilter", |p| p.remove_row_filter(book, 0));
    settle(&mut h);
    click_text(&mut h, "30.5");
    assert!(runs(&h, "4.5")[0].1.italics);
    assert!(!runs(&h, "2.5")[0].1.italics);
}

/// Formats column B (bold) and the label cell of A (italic), through the model.
fn seed_formats(h: &mut Harness<'static, SheafApp>) {
    let (a, b) = (col_id(h, "A"), col_id(h, "B"));
    h.state_mut().doc.apply("seed", |p| {
        let book = p.workbooks().next().unwrap().id;
        let s = p.workbook_mut(book).unwrap().sheet_mut();
        s.apply_cell_format(
            FormatTarget::cells(vec![b], 1, None),
            CellFormat {
                bold: Some(true),
                font: Some("Courier New".into()),
                size_pt: Some(14.0),
                fill: Some(Color::rgb(1, 2, 3)),
                h_align: Some(HAlign::Right),
                ..Default::default()
            },
        );
        s.apply_cell_format(
            FormatTarget::label_rows(vec![a], vec![LabelKind::Units]),
            CellFormat { italic: Some(true), ..Default::default() },
        );
    });
    settle(h);
}

#[test]
fn formats_survive_save_and_reload_as_a_sheaf_project() {
    let dir = save_dir("cellfmt-sheaf");
    let path = dir.join("fmt.sheaf");
    let mut h = start_book("cfmt-save-sheaf");
    seed_formats(&mut h);
    h.state_mut().path = Some(path.clone());
    ctrl_s(&mut h);
    assert!(h.state().message.is_none(), "{:?}", h.state().message);
    let h2 = start(vec![path]);
    assert_eq!(sheet(&h2).cell_formats, sheet(&h).cell_formats);
    assert!(!sheet(&h2).cell_formats.is_empty());
}

#[test]
fn an_opj_save_warns_that_formats_are_not_in_it_and_the_companion_keeps_them() {
    let dir = save_dir("cellfmt-opj");
    let opj = dir.join("fmt.opj");
    let mut h = start_book("cfmt-save-opj");
    seed_formats(&mut h);
    h.state_mut().path = Some(opj.clone());
    ctrl_s(&mut h);
    let (title, text) = h.state().message.clone().expect("a message says what only the companion holds");
    assert!(title.contains("companion") && text.contains("cell formatting"), "{title}: {text}");
    let h2 = start(vec![opj]);
    assert_eq!(sheet(&h2).cell_formats, sheet(&h).cell_formats);
}

#[test]
fn duplicating_a_sheet_and_deleting_columns_keep_the_formats_on_the_right_columns() {
    let mut h = start_book("cfmt-dup");
    seed_formats(&mut h);
    let book = project(&h).workbooks().next().unwrap().id;
    h.state_mut().sheet_op(book, SheetOp::Duplicate);
    h.run();
    let b = project(&h).workbook(book).unwrap();
    let (orig, copy) = (&b.sheets[0], &b.sheets[1]);
    assert_eq!(copy.cell_formats.len(), orig.cell_formats.len());
    for r in &copy.cell_formats {
        for id in r.target.columns() {
            assert!(copy.column_index(*id).is_some() && orig.column_index(*id).is_none(), "the copy's own column");
        }
    }
    let cb = copy.column_by_name("B").unwrap().id;
    assert!(copy.cell_format_at(cb, 2).bold && !copy.cell_format_at(cb, 0).bold);
    // Deleting the column drops its formats with it (and undo brings them back).
    let ids = vec![col_id(&h, "B")];
    h.state_mut().doc.apply("Delete columns", |p| delete_columns(p, &ids));
    assert_eq!(sheet(&h).cell_formats.len(), 1, "only the label format of A is left");
    h.state_mut().doc.undo();
    assert_eq!(sheet(&h).cell_formats.len(), 2);
}
