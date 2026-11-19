//! Column > Fill Column With patterns (T74): A set of Numbers, Date/Time values, Text&Numeric values.

use super::*;

#[test]
fn a_set_of_numbers_fills_the_selected_column_in_one_undo_step() {
    let mut h = start(vec![data_file("fill-numbers")]);
    show_window(&mut h, "Book1 - sample.csv");
    let b = column_id(&h, "B");
    click_text(&mut h, "B(Y1)");
    menu_path(&mut h, &["Column", "Fill Column With", "A set of Numbers..."]);
    assert!(h.state().fill_pattern.is_some(), "the window opened");
    // Origin's defaults: 1 to 10 by 1.
    click(&mut h, "OK");
    assert!(h.state().fill_pattern.is_none(), "OK fills and closes");
    let filled = project(&h).column(b).unwrap().numeric().unwrap().to_vec();
    assert_eq!(filled, (1..=10).map(f64::from).collect::<Vec<_>>());
    assert_eq!(h.state().doc.undo_label(), Some("Fill column"));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).column(b).unwrap().numeric().unwrap(), [2.5, 4.5, 6.5, 8.5]);
}

#[test]
fn a_set_of_text_items_fills_with_quotes_keeping_spaces() {
    let mut h = start(vec![data_file("fill-text")]);
    show_window(&mut h, "Book1 - sample.csv");
    let c = column_id(&h, "C");
    let ctx = h.ctx.clone();
    h.state_mut()
        .apply(Action::FillPattern { kind: crate::dialogs::fill_pattern::FillKind::Text, columns: vec![c] }, &ctx);
    settle(&mut h);
    h.query_all_by_role(egui::accesskit::Role::TextInput).last().expect("the text box").focus();
    settle(&mut h);
    h.event(egui::Event::Text(r#"Boston "New York""#.into()));
    settle(&mut h);
    click(&mut h, "OK");
    let col = project(&h).column(c).unwrap();
    assert_eq!((col.data.text_cell(0), col.data.text_cell(1)), (Some("Boston"), Some("New York")));
    assert_eq!(&col.numeric().unwrap()[2..], [10.5, 0.5], "the rows below keep their values");
}

#[test]
fn row_numbers_and_random_numbers_fill_at_once_in_one_undo_step() {
    // Column > Fill Column With > Row Numbers / Uniform / Normal Random Numbers (T63 group H).
    let mut h = start(vec![data_file("fill-quick")]);
    show_window(&mut h, "Book1 - sample.csv");
    let b = column_id(&h, "B");
    click_text(&mut h, "B(Y1)");
    menu_path(&mut h, &["Column", "Fill Column With", "Row Numbers"]);
    assert!(h.state().fill_pattern.is_none(), "no window");
    assert_eq!(project(&h).column(b).unwrap().numeric().unwrap(), [1.0, 2.0, 3.0, 4.0]);
    assert_eq!(h.state().doc.undo_label(), Some("Fill column"));
    menu_path(&mut h, &["Column", "Fill Column With", "Uniform Random Numbers"]);
    let u = project(&h).column(b).unwrap().numeric().unwrap().to_vec();
    assert!(u.len() == 4 && u.iter().all(|v| (0.0..1.0).contains(v)), "{u:?}");
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).column(b).unwrap().numeric().unwrap(), [1.0, 2.0, 3.0, 4.0], "one step back");
    let c = column_id(&h, "C");
    h.state_mut()
        .apply(Action::QuickFill { kind: sheaf_core::fill_pattern::QuickFill::Normal, columns: vec![b, c] }, &ctx);
    settle(&mut h);
    for col in [b, c] {
        let v = project(&h).column(col).unwrap().numeric().unwrap().to_vec();
        assert!(v.len() == 4 && v.iter().all(|x| x.is_finite()), "{v:?}");
    }
    assert_ne!(project(&h).numeric(b).unwrap(), project(&h).numeric(c).unwrap(), "each column its own numbers");
}
