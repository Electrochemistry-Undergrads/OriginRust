use super::*;
use sheaf_core::condformat::{self, Rule};

fn start_book(tag: &str) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(tag)]);
    show_window(&mut h, "Book1 - sample.csv");
    h
}

fn select_in_state(h: &mut Harness<'static, SheafApp>, sel: Sel) {
    let book = project(h).workbooks().next().unwrap().id;
    let sheet = sheet(h).clone();
    let state = h.state_mut().books.get_mut(&book).unwrap();
    match sel {
        Sel::Cells { anchor, end } => state.select_cells(&sheet, anchor, end),
        _ => unreachable!(),
    }
}

fn rules(h: &Harness<'static, SheafApp>) -> Vec<condformat::CondFormat> {
    sheet(h).cond_formats.clone()
}

/// Every filled rectangle the last frame painted: (rect, colour), looking into shape groups.
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

fn rgb(c: sheaf_core::color::Color) -> egui::Color32 {
    egui::Color32::from_rgb(c.r, c.g, c.b)
}

/// The fill painted behind the cell that shows `text`, if it is one of the rule colours.
fn fill_behind(h: &Harness<'static, SheafApp>, text: &str) -> Option<egui::Color32> {
    let centre = grid_text(h, text).rect().center();
    painted(h)
        .into_iter()
        .rev()
        .find(|(r, c)| r.contains(centre) && [condformat::MAX_FILL, condformat::MIN_FILL].map(rgb).contains(c))
        .map(|(_, c)| c)
}

/// Selects column B from its head and adds Highlight Min Max, column-wise.
fn apply_column_wise(h: &mut Harness<'static, SheafApp>) {
    click_text(h, "B(Y1)");
    menu_path(h, &["Worksheet", "Conditional Formatting", "Highlight Min Max..."]);
    click(h, "Column-wise");
    click(h, "OK");
}

#[test]
fn min_max_column_wise_from_a_column_selection_paints_the_extremes() {
    let mut h = start_book("cf-colwise");
    apply_column_wise(&mut h);
    assert_eq!(h.state().doc.undo_label(), Some("Highlight min max"));
    let r = rules(&h);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].name, "C2R1-C2R0");
    assert!(matches!(r[0].rule, Rule::MinMax { dir: condformat::Direction::ColumnWise, .. }));
    // Select another column so the black selection does not hide the colours.
    click_text(&mut h, "C(Y1)");
    assert_eq!(fill_behind(&h, "8.5"), Some(rgb(condformat::MAX_FILL)));
    assert_eq!(fill_behind(&h, "2.5"), Some(rgb(condformat::MIN_FILL)));
    assert_eq!(fill_behind(&h, "4.5"), None);
    // The other columns are untouched.
    assert_eq!(fill_behind(&h, "30.5"), None);
    assert_eq!(fill_behind(&h, "0.5"), None);
}

#[test]
fn editing_a_value_moves_the_highlight_and_undo_brings_the_rule_back() {
    let mut h = start_book("cf-edit");
    apply_column_wise(&mut h);
    click_text(&mut h, "C(Y1)");
    edit_cell(&mut h, "B", 0, "100");
    assert_eq!(fill_behind(&h, "100"), Some(rgb(condformat::MAX_FILL)));
    assert_eq!(fill_behind(&h, "8.5"), None, "the old maximum is no longer one");
    assert_eq!(fill_behind(&h, "4.5"), Some(rgb(condformat::MIN_FILL)));

    // Remove Conditional Format on the column: whole rule gone, one undo step.
    click_text(&mut h, "B(Y1)");
    menu_path(&mut h, &["Worksheet", "Conditional Formatting", "Remove Conditional Format"]);
    assert!(rules(&h).is_empty());
    assert_eq!(h.state().doc.undo_label(), Some("Remove conditional format"));
    click_text(&mut h, "C(Y1)");
    assert_eq!(fill_behind(&h, "100"), None);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    settle(&mut h);
    assert_eq!(rules(&h).len(), 1);
    assert_eq!(fill_behind(&h, "100"), Some(rgb(condformat::MAX_FILL)));
    // Redo removes it again.
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z);
    settle(&mut h);
    assert!(rules(&h).is_empty());
}

#[test]
fn remove_leaves_rules_that_do_not_touch_the_selection() {
    let mut h = start_book("cf-remove-other");
    apply_column_wise(&mut h);
    click_text(&mut h, "C(Y1)");
    menu_path(&mut h, &["Worksheet", "Conditional Formatting", "Remove Conditional Format"]);
    assert_eq!(rules(&h).len(), 1, "column C has no rule");
    assert!(h.state().status.contains("No conditional format"), "{}", h.state().status);
}

#[test]
fn top_bottom_opens_from_the_cell_menu_and_row_wise_is_the_min_max_default() {
    let mut h = start_book("cf-topbottom");
    // A block of cells B1:C4, from the cell menu.
    select_in_state(&mut h, Sel::Cells { anchor: (1, 0), end: (2, 3) });
    right_click_text(&mut h, "4.5");
    menu_path(&mut h, &["Conditional Formatting", "Highlight Top Bottom..."]);
    click(&mut h, "OK");
    let r = rules(&h);
    assert_eq!(r.len(), 1);
    assert!(matches!(r[0].rule, Rule::TopBottom { top: true, percent: false, n, .. } if n == 10.0));
    assert_eq!(r[0].name, "C2R1-C3R0");
    // Min Max with the defaults: row-wise.
    select_in_state(&mut h, Sel::Cells { anchor: (1, 0), end: (2, 1) });
    menu_path(&mut h, &["Worksheet", "Conditional Formatting", "Highlight Min Max..."]);
    click(&mut h, "OK");
    let r = rules(&h);
    assert_eq!(r.len(), 2);
    assert!(matches!(r[0].rule, Rule::MinMax { dir: condformat::Direction::RowWise, .. }), "the newest rule is first");
    assert_eq!(r[0].name, "C2R1-C3R2");
}

#[test]
fn a_selection_that_is_not_data_gives_a_message_and_no_rule() {
    let mut h = start_book("cf-none");
    let ctx = h.ctx.clone();
    let book = project(&h).workbooks().next().unwrap().id;
    h.state_mut().apply(Action::CondFormatDialog { book, kind: crate::dialogs::cond_format::CondKind::MinMax }, &ctx);
    settle(&mut h);
    assert!(h.state().cond_format.is_none());
    assert!(h.state().status.contains("Select"), "{}", h.state().status);
    assert!(rules(&h).is_empty());
}

#[test]
fn conditional_formats_survive_save_and_reload_through_the_companion() {
    let dir = save_dir("condformat");
    let opj = dir.join("cf.opj");
    let mut h = start_book("cf-save");
    apply_column_wise(&mut h);
    h.state_mut().path = Some(opj.clone());
    ctrl_s(&mut h);
    let (title, text) = h.state().message.clone().expect("a message says what only the companion holds");
    assert!(title.contains("companion") && text.contains("conditional formatting"), "{title}: {text}");
    let h2 = start(vec![opj]);
    let reloaded = project(&h2).workbooks().next().unwrap().sheet().cond_formats.clone();
    assert_eq!(reloaded, rules(&h));
}

#[test]
fn duplicate_sheet_keeps_the_rules_on_the_copys_own_columns() {
    let mut h = start_book("cf-dup-sheet");
    apply_column_wise(&mut h);
    let book = project(&h).workbooks().next().unwrap().id;
    h.state_mut().sheet_op(book, SheetOp::Duplicate);
    h.run();
    let b = project(&h).workbook(book).unwrap();
    let (orig, copy) = (&b.sheets[0], &b.sheets[1]);
    assert_eq!(copy.cond_formats.len(), orig.cond_formats.len());
    assert!(!copy.cond_formats.is_empty());
    for r in &copy.cond_formats {
        for id in &r.range.columns {
            assert!(copy.column_index(*id).is_some(), "rule column belongs to the copy");
            assert!(orig.column_index(*id).is_none());
        }
    }
}

#[test]
fn deleting_all_columns_of_a_rule_drops_it_with_the_delete() {
    let mut h = start_book("cf-del-cols");
    apply_column_wise(&mut h);
    let ids: Vec<_> = sheet(&h).cond_formats[0].range.columns.clone();
    h.state_mut().doc.apply("Delete columns", |p| delete_columns(p, &ids));
    assert!(rules(&h).is_empty());
    h.state_mut().doc.undo();
    assert_eq!(rules(&h).len(), 1, "undo brings the rule back with the columns");
}

/// The fill painted behind the cell that shows `text`, if it is `colour`.
fn painted_behind(h: &Harness<'static, SheafApp>, text: &str, colour: sheaf_core::color::Color) -> bool {
    let centre = grid_text(h, text).rect().center();
    painted(h).into_iter().any(|(r, c)| r.contains(centre) && c == rgb(colour))
}

#[test]
fn conditional_heatmap_colours_a_column_from_its_minimum_to_its_maximum() {
    use sheaf_core::condformat::heatmap;
    let mut h = start_book("cf-heatmap");
    click_text(&mut h, "B(Y1)");
    menu_path(&mut h, &["Worksheet", "Conditional Formatting", "Conditional Heatmap..."]);
    click(&mut h, "OK");
    assert_eq!(h.state().doc.undo_label(), Some("Conditional heatmap"));
    let r = rules(&h);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].rule, Rule::heatmap(), "the defaults go into the rule unchanged");
    click_text(&mut h, "C(Y1)");
    // The minimum takes the From colour (white, so read from the evaluated styles).
    let s = sheet(&h).clone();
    let b = &s.columns[1];
    let min_row = b.numeric().unwrap().iter().position(|v| *v == 2.5).unwrap();
    let style = condformat::evaluate(&s, &s.cond_formats).get(b.id, min_row).unwrap();
    assert_eq!((style.fill, style.font), (Some(heatmap::DEFAULT_FROM), Some(sheaf_core::color::Color::BLACK)));
    assert!(painted_behind(&h, "8.5", heatmap::DEFAULT_TO), "the maximum takes the To colour");
    assert!(!painted_behind(&h, "30.5", heatmap::DEFAULT_TO), "other columns are untouched");
    // Undo removes the rule and its colours.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    settle(&mut h);
    assert!(rules(&h).is_empty());
    assert!(!painted_behind(&h, "8.5", heatmap::DEFAULT_TO));
}

#[test]
fn conditional_heatmap_options_reach_the_rule() {
    use sheaf_core::condformat::heatmap::{Mixing, Target};
    let mut h = start_book("cf-heatmap-options");
    click_text(&mut h, "B(Y1)");
    menu_path(&mut h, &["Worksheet", "Conditional Formatting", "Conditional Heatmap..."]);
    click(&mut h, "3-Color Limited Mixing");
    click(&mut h, "Color Text");
    click(&mut h, "Flip");
    click(&mut h, "OK");
    let r = rules(&h);
    let Rule::Heatmap(heat) = &r[0].rule else { panic!("{:?}", r[0].rule) };
    assert!(matches!(heat.mixing, Mixing::ThreeColor { .. }));
    assert_eq!(heat.target, Target::Text);
    assert!(heat.flip);
    assert!(!heat.contrast_text, "contrast text only goes with a fill");
    // Flipped, the maximum's text takes the From colour; no cell gets a fill.
    let s = sheet(&h).clone();
    let styles = condformat::evaluate(&s, &s.cond_formats);
    let b = &s.columns[1];
    let max_row = b.numeric().unwrap().iter().position(|v| *v == 8.5).unwrap();
    let style = styles.get(b.id, max_row).unwrap();
    assert_eq!((style.fill, style.font), (None, Some(sheaf_core::condformat::heatmap::DEFAULT_FROM)));
}
