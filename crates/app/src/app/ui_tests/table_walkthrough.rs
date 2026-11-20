//! Table UI walkthrough coverage (T212, known-issues "Continue table UI coverage"): row and column
//! operations, moving columns and switching sheets, each on a brand-new and an imported table.

use super::*;
use sheaf_core::sheetops::MoveTo;

/// The values of the first two columns; a new table gets 1, 2, 3 and 4, 5, 6 typed in first.
fn filled(imported: bool, tag: &str) -> (Harness<'static, SheafApp>, Vec<f64>, Vec<f64>) {
    let mut h = cell_regression_table(imported, tag);
    if !imported {
        for (col, values) in [(0, ["1", "2", "3"]), (1, ["4", "5", "6"])] {
            let at = worksheet_cell_pos(&h, col, 0);
            click_at(&mut h, at);
            for v in values {
                h.event(egui::Event::Text(v.into()));
                settle(&mut h);
                key(&mut h, egui::Key::Enter);
            }
        }
    }
    let a = column(&h, "A");
    let b = column(&h, "B");
    (h, a, b)
}

fn names(h: &Harness<'static, SheafApp>) -> Vec<String> {
    sheet(h).columns.iter().map(|c| c.short_name.clone()).collect()
}

fn close(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x == y || (x.is_nan() && y.is_nan()))
}

#[test]
fn cell_insert_and_delete_shift_only_the_selected_column() {
    for imported in [false, true] {
        let (mut h, a, b) = filled(imported, &format!("walk-cells-{imported}"));
        assert!(a.len() >= 3 && b.len() >= 3, "imported={imported}: {a:?} {b:?}");
        // Ctrl++ at A2 pushes A down from row 2; B stays.
        let at = worksheet_cell_pos(&h, 0, 1);
        click_at(&mut h, at);
        key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::Plus);
        let mut want = a.clone();
        want.insert(1, f64::NAN);
        assert!(close(&column(&h, "A"), &want), "imported={imported}: {:?}", column(&h, "A"));
        assert_eq!(column(&h, "B"), b, "imported={imported}");
        ctrl(&mut h, egui::Key::Z);
        assert_eq!(column(&h, "A"), a, "undo, imported={imported}");
        // Ctrl+- at A2 removes that cell only.
        let at = worksheet_cell_pos(&h, 0, 1);
        click_at(&mut h, at);
        key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::Minus);
        let mut want = a.clone();
        want.remove(1);
        assert_eq!(column(&h, "A"), want, "imported={imported}");
        assert_eq!(column(&h, "B"), b, "imported={imported}");
        ctrl(&mut h, egui::Key::Z);
        assert_eq!(column(&h, "A"), a, "undo, imported={imported}");
    }
}

#[test]
fn whole_columns_insert_and_delete_with_ctrl_space() {
    for imported in [false, true] {
        let (mut h, a, b) = filled(imported, &format!("walk-columns-{imported}"));
        let before = names(&h);
        // Ctrl+Space on A1 selects column A; Ctrl+- deletes it.
        let at = worksheet_cell_pos(&h, 0, 0);
        click_at(&mut h, at);
        key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::Space);
        assert_eq!(book_ui(&h).sel, Sel::Columns, "imported={imported}");
        key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::Minus);
        assert_eq!(sheet(&h).columns.len(), before.len() - 1, "imported={imported}");
        assert_eq!(sheet(&h).columns[0].numeric().map(<[f64]>::to_vec), Some(b.clone()), "B moved first");
        ctrl(&mut h, egui::Key::Z);
        assert_eq!(names(&h), before, "undo, imported={imported}");
        assert_eq!(column(&h, "A"), a);
        // Ctrl++ on a selected column inserts an empty one before it.
        let at = worksheet_cell_pos(&h, 0, 0);
        click_at(&mut h, at);
        key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::Space);
        key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::Plus);
        assert_eq!(sheet(&h).columns.len(), before.len() + 1, "imported={imported}");
        assert!(
            sheet(&h).columns[0].is_empty()
                || sheet(&h).columns[0].numeric().is_some_and(|v| v.iter().all(|x| x.is_nan()))
        );
        assert_eq!(sheet(&h).columns[1].numeric().map(<[f64]>::to_vec), Some(a.clone()), "A moved right");
        ctrl(&mut h, egui::Key::Z);
        assert_eq!(names(&h), before, "undo, imported={imported}");
    }
}

#[test]
fn moving_a_column_and_switching_sheets_keep_the_data() {
    for imported in [false, true] {
        let (mut h, a, b) = filled(imported, &format!("walk-move-{imported}"));
        let before = names(&h);
        let a_id = sheet(&h).columns[0].id;
        let at = worksheet_cell_pos(&h, 0, 0);
        click_at(&mut h, at);
        key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::Space);
        let action = menus::resolve(&menus::Cmd::MoveColumns(MoveTo::Right), &h.state().menu_context()).unwrap();
        let ctx = h.ctx.clone();
        h.state_mut().apply(action, &ctx);
        settle(&mut h);
        assert_eq!(sheet(&h).columns[1].id, a_id, "A moved one place right, imported={imported}");
        assert_eq!(sheet(&h).columns[1].numeric().map(<[f64]>::to_vec), Some(a.clone()));
        ctrl(&mut h, egui::Key::Z);
        assert_eq!(names(&h), before, "undo, imported={imported}");

        // A second sheet: type there, step back with Ctrl+PageUp and forward with Ctrl+PageDown.
        let book = h.state().books.keys().next().copied().unwrap();
        h.state_mut().apply(Action::Sheet { book, op: SheetOp::Add }, &ctx);
        settle(&mut h);
        let active = |h: &Harness<'static, SheafApp>| project(h).workbook(book).unwrap().active_sheet;
        assert_eq!(active(&h), 1);
        let at = worksheet_cell_pos(&h, 0, 0);
        click_at(&mut h, at);
        h.event(egui::Event::Text("9".into()));
        settle(&mut h);
        key(&mut h, egui::Key::Enter);
        assert_eq!(sheet(&h).columns[0].cell_text(0), "9", "imported={imported}");
        key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::PageUp);
        assert_eq!(active(&h), 0, "imported={imported}");
        assert_eq!((column(&h, "A"), column(&h, "B")), (a.clone(), b.clone()), "the first sheet is untouched");
        key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::PageDown);
        assert_eq!(active(&h), 1);
        assert_eq!(sheet(&h).columns[0].cell_text(0), "9", "the typed value is kept, imported={imported}");
    }
}

#[test]
fn sheet_insert_duplicate_delete_and_a_paste_across_sheets() {
    // T215: sheet controls on both kinds of table, each step one undo step.
    for imported in [false, true] {
        let (mut h, a, _b) = filled(imported, &format!("walk-sheets-{imported}"));
        let book = h.state().books.keys().next().copied().unwrap();
        let ctx = h.ctx.clone();
        let sheets = |h: &Harness<'static, SheafApp>| project(h).workbook(book).unwrap().sheets.len();
        let active = |h: &Harness<'static, SheafApp>| project(h).workbook(book).unwrap().active_sheet;
        let a_of = |h: &Harness<'static, SheafApp>, s: usize| {
            project(h).workbook(book).unwrap().sheets[s].columns.first().and_then(|c| c.numeric()).map(<[f64]>::to_vec)
        };
        // Insert puts the new sheet before the active one and goes to it.
        h.state_mut().apply(Action::Sheet { book, op: SheetOp::Insert }, &ctx);
        settle(&mut h);
        assert_eq!((sheets(&h), active(&h)), (2, 0), "imported={imported}");
        assert_eq!(a_of(&h, 1), Some(a.clone()), "the data sheet moved to second place");
        h.state_mut().apply(Action::Undo, &ctx);
        settle(&mut h);
        assert_eq!(sheets(&h), 1, "undo, imported={imported}");

        // Duplicate copies the data into independent columns.
        h.state_mut().apply(Action::Sheet { book, op: SheetOp::Duplicate }, &ctx);
        settle(&mut h);
        assert_eq!((sheets(&h), active(&h)), (2, 1), "imported={imported}");
        assert_eq!(a_of(&h, 1), Some(a.clone()));
        let at = worksheet_cell_pos(&h, 0, 0);
        click_at(&mut h, at);
        h.event(egui::Event::Text("42".into()));
        settle(&mut h);
        key(&mut h, egui::Key::Enter);
        assert_eq!(a_of(&h, 1).unwrap()[0], 42.0, "imported={imported}");
        assert_eq!(a_of(&h, 0), Some(a.clone()), "the source sheet is untouched");

        // Delete removes the duplicate; undo brings it back with the typed value.
        h.state_mut().apply(Action::Sheet { book, op: SheetOp::Delete }, &ctx);
        settle(&mut h);
        assert_eq!((sheets(&h), active(&h)), (1, 0), "imported={imported}");
        h.state_mut().apply(Action::Undo, &ctx);
        settle(&mut h);
        assert_eq!(sheets(&h), 2);
        assert_eq!(a_of(&h, 1).unwrap()[0], 42.0, "imported={imported}");

        // Copy A1:A3 of the first sheet, paste it into B1 of the second.
        h.state_mut().apply(Action::SelectSheet { book, sheet: 0 }, &ctx);
        settle(&mut h);
        let at = worksheet_cell_pos(&h, 0, 0);
        click_at(&mut h, at);
        key_with(&mut h, egui::Modifiers::SHIFT, egui::Key::ArrowDown);
        key_with(&mut h, egui::Modifiers::SHIFT, egui::Key::ArrowDown);
        ctrl(&mut h, egui::Key::C);
        let copied = h.state().copied.clone();
        key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::PageDown);
        assert_eq!(active(&h), 1);
        let at = worksheet_cell_pos(&h, 1, 0);
        click_at(&mut h, at);
        paste(&mut h, &copied);
        let b1 = project(&h).workbook(book).unwrap().sheets[1].columns[1].numeric().unwrap()[..3].to_vec();
        assert_eq!(b1, a[..3], "imported={imported}");
        assert_eq!(a_of(&h, 0), Some(a.clone()), "copying leaves the source alone");
    }
}
