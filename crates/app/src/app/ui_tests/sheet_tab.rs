//! The sheet tab's Duplicate Without Data and Sheet Name to Show (T90, workbook-model.md "Sheet tab
//! names").

use super::*;
use sheaf_core::sheet_tab::SheetLabel;

/// T309: the corner menu's Add User Parameter Row... adds an empty row to every column.
#[test]
fn a_user_parameter_row_can_be_added_from_the_corner_menu() {
    assert!(crate::menus::corner_menu(&sheaf_core::Worksheet::new("Sheet1")).iter().any(|e| matches!(e,
        crate::menus::Entry::Item(i) if i.label == "Add User Parameter Row..."
            && i.cmd == Some(crate::menus::Cmd::Sheet(SheetOp::AddParamRow)))));
    let mut h = start(vec![data_file("param-row")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Sheet { book, op: SheetOp::AddParamRow }, &ctx);
    settle(&mut h);
    assert!(h.query_by_label("Name of the row").is_some());
    h.state_mut().param_row.as_mut().unwrap().2 = "Dose".into();
    click(&mut h, "OK");
    assert!(h.state().param_row.is_none(), "the window closes");
    let sheet = project(&h).workbook(book).unwrap().sheets[0].clone();
    assert_eq!(sheet.parameter_names(), ["Dose"]);
    assert!(h.query_by_label("Dose").is_some() || h.query_all_by_value("Dose").next().is_some(), "the row shows");
    assert_eq!(h.state().doc.undo_label(), Some("Add parameter row"));
    // A taken name keeps the window open and changes nothing.
    h.state_mut().apply(Action::Sheet { book, op: SheetOp::AddParamRow }, &ctx);
    settle(&mut h);
    h.state_mut().param_row.as_mut().unwrap().2 = "dose".into();
    click(&mut h, "OK");
    assert!(h.state().param_row.is_some());
    assert_eq!(project(&h).workbook(book).unwrap().sheets[0].parameter_names(), ["Dose"]);
}

/// T306: dragging a sheet tab onto another tab moves the sheet there, in one undo step.
#[test]
fn dragging_a_sheet_tab_onto_another_moves_the_sheet() {
    let mut h = start(vec![data_file("sheet-drag")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let ctx = h.ctx.clone();
    for _ in 0..2 {
        h.state_mut().apply(Action::Sheet { book, op: SheetOp::Add }, &ctx);
    }
    settle(&mut h);
    let names = |h: &Harness<'static, SheafApp>| -> Vec<String> {
        project(h).workbook(book).unwrap().sheets.iter().map(|s| s.name.clone()).collect()
    };
    let before = names(&h);
    assert_eq!(before.len(), 3);
    let from = h.get_by_label(&before[0]).rect().center();
    let to = h.get_by_label(&before[2]).rect().center();
    pause(&mut h);
    h.event(egui::Event::PointerMoved(from));
    h.step();
    h.event(egui::Event::PointerButton {
        pos: from,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.step();
    for k in 1..=8 {
        let t = k as f32 / 8.0;
        h.event(egui::Event::PointerMoved(from + (to - from) * t));
        h.step();
    }
    h.event(egui::Event::PointerButton {
        pos: to,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.step();
    settle(&mut h);
    let after = names(&h);
    assert_eq!(
        after,
        [before[1].clone(), before[2].clone(), before[0].clone()],
        "the first sheet went to the third place"
    );
    assert_eq!(project(&h).workbook(book).unwrap().active_sheet, 2, "the moved sheet is active");
    assert_eq!(h.state().doc.undo_label(), Some("Move worksheet"));
}

/// T277: the sheet tab's Activate This Sheet in All Books in Folder.
#[test]
fn a_sheet_becomes_active_in_the_other_books_of_the_folder() {
    let mut h = start(vec![data_file("sheet-tab")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let mut others = Vec::new();
    h.state_mut().doc.apply("Books", |p| {
        for name in ["Book2", "Book3"] {
            let b = p.add_workbook(name);
            p.workbook_mut(b).unwrap().sheets.push(sheaf_core::Worksheet::new(if name == "Book2" {
                "Fit"
            } else {
                "Other"
            }));
            others.push(b);
        }
        let wb = p.workbook_mut(book).unwrap();
        wb.sheets.push(sheaf_core::Worksheet::new("Fit"));
        wb.active_sheet = 1;
    });
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Sheet { book, op: SheetOp::ActivateInFolder }, &ctx);
    settle(&mut h);
    let p = project(&h);
    assert_eq!(p.workbook(others[0]).unwrap().active_sheet, 1, "Book2's Fit sheet is active");
    assert_eq!(p.workbook(others[1]).unwrap().active_sheet, 0, "Book3 has no Fit sheet");
    let entries = crate::menus::sheet_tab_menu(&p.workbook(book).unwrap().sheets[1]);
    assert!(entries.iter().any(|e| matches!(e, crate::menus::Entry::Item(i)
        if i.label == "Activate This Sheet in All Books in Folder"
            && i.cmd == Some(crate::menus::Cmd::Sheet(SheetOp::ActivateInFolder)))));
}

#[test]
fn duplicate_without_data_and_the_long_name_on_the_tab() {
    let mut h = start(vec![data_file("sheet-tab")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    h.state_mut().doc.apply("Long name", |p| p.workbook_mut(book).unwrap().sheets[0].long_name = "Scan".into());
    let ctx = h.ctx.clone();
    // Long Name: the tab shows the long name, one undo step.
    h.state_mut().apply(Action::Sheet { book, op: SheetOp::ShowName(SheetLabel::Long) }, &ctx);
    settle(&mut h);
    assert!(h.query_by_label("Scan").is_some(), "the tab shows the long name");
    assert_eq!(h.state().doc.undo_label(), Some("Sheet name to show"));
    // Duplicate Without Data: a new last sheet with the columns and no values, active.
    h.state_mut().apply(Action::Sheet { book, op: SheetOp::DuplicateWithoutData }, &ctx);
    settle(&mut h);
    let b = project(&h).workbook(book).unwrap();
    assert_eq!(b.sheets.len(), 2);
    assert_eq!(b.active_sheet, 1);
    let copy = &b.sheets[1];
    assert_eq!((copy.name.as_str(), copy.long_name.as_str()), ("Sheet1", "Scan 1"), "the first free SheetN");
    assert_eq!(copy.columns.len(), b.sheets[0].columns.len());
    assert!(copy.columns.iter().all(|c| c.data.is_empty()));
    assert_eq!(h.state().doc.undo_label(), Some("Duplicate sheet without data"));
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).workbook(book).unwrap().sheets.len(), 1);
}
