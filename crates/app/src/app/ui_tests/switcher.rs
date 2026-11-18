//! The Window Switcher (Ctrl+Tab): order, wrap-around, hidden windows, cancel, folders, and the
//! worksheet grid keeping plain Tab.

use super::*;
use egui::Modifiers;

/// Three new workbooks activated in order: the first is `ids[0]` and the last, `ids[2]`, is active.
fn three() -> (Harness<'static, SheafApp>, Vec<WindowId>) {
    let mut h = start_auto_hide(vec![]);
    let ctx = h.ctx.clone();
    let mut ids = Vec::new();
    for _ in 0..3 {
        h.state_mut().apply(Action::NewWorkbook, &ctx);
        settle(&mut h);
        ids.push(project(&h).workbooks().last().unwrap().id);
    }
    // Activate in the order A, B, C whatever the creation did.
    for id in &ids {
        h.state_mut().focus(*id);
        settle(&mut h);
    }
    (h, ids)
}

fn tab_event(h: &mut Harness<'static, SheafApp>, modifiers: Modifiers) {
    press(h, egui::Key::Tab, modifiers);
}

/// A key pressed and released with these modifiers, which stay down afterwards.
fn press(h: &mut Harness<'static, SheafApp>, key: egui::Key, modifiers: Modifiers) {
    let ev = |pressed| egui::Event::Key { key, physical_key: Some(key), pressed, repeat: false, modifiers };
    h.input_mut().modifiers = modifiers;
    h.event(ev(true));
    h.step();
    h.event(ev(false));
    settle(h);
}

/// Ctrl+Tab with Ctrl staying down.
fn ctrl_tab(h: &mut Harness<'static, SheafApp>) {
    tab_event(h, Modifiers::CTRL);
}

fn ctrl_shift_tab(h: &mut Harness<'static, SheafApp>) {
    tab_event(h, Modifiers::CTRL | Modifiers::SHIFT);
}

fn release_ctrl(h: &mut Harness<'static, SheafApp>) {
    h.input_mut().modifiers = Modifiers::NONE;
    settle(h);
}

fn selected(h: &Harness<'static, SheafApp>) -> Option<WindowId> {
    h.state().switcher.as_ref().map(|s| s.selected())
}

fn active(h: &Harness<'static, SheafApp>) -> Option<WindowId> {
    h.state().workspace.active()
}

#[test]
fn ctrl_tab_selects_the_previous_window_and_releasing_ctrl_activates_it() {
    let (mut h, ids) = three();
    assert_eq!(active(&h), Some(ids[2]));
    ctrl_tab(&mut h);
    assert_eq!(selected(&h), Some(ids[1]), "the second card is the previous window");
    assert_eq!(active(&h), Some(ids[2]), "nothing changes while Ctrl is down");
    release_ctrl(&mut h);
    assert!(h.state().switcher.is_none());
    assert_eq!(active(&h), Some(ids[1]));
    // The most recent is now B, then C: another Ctrl+Tab goes back to C.
    ctrl_tab(&mut h);
    release_ctrl(&mut h);
    assert_eq!(active(&h), Some(ids[2]));
}

#[test]
fn the_cards_are_in_recent_order_with_a_card_per_window() {
    let (mut h, ids) = three();
    ctrl_tab(&mut h);
    let titles = h.state().switcher.as_ref().unwrap().titles().iter().map(|t| t.to_string()).collect::<Vec<_>>();
    let expect: Vec<String> = ids.iter().rev().map(|id| project(&h).window(*id).unwrap().title()).collect();
    assert_eq!(titles, expect);
    h.get_by_label(&format!("Switch to {}", expect[2]));
    release_ctrl(&mut h);
}

#[test]
fn a_second_tab_goes_on_and_the_selection_wraps_in_both_directions() {
    let (mut h, ids) = three();
    ctrl_tab(&mut h);
    ctrl_tab(&mut h);
    assert_eq!(selected(&h), Some(ids[0]));
    ctrl_tab(&mut h);
    assert_eq!(selected(&h), Some(ids[2]), "wraps to the first card");
    ctrl_shift_tab(&mut h);
    assert_eq!(selected(&h), Some(ids[0]));
    ctrl_shift_tab(&mut h);
    ctrl_shift_tab(&mut h);
    assert_eq!(selected(&h), Some(ids[2]));
    ctrl_shift_tab(&mut h);
    assert_eq!(selected(&h), Some(ids[0]), "and back round to the last card");
    release_ctrl(&mut h);
    assert_eq!(active(&h), Some(ids[0]));
}

#[test]
fn ctrl_shift_tab_starts_on_the_last_window() {
    let (mut h, ids) = three();
    ctrl_shift_tab(&mut h);
    assert_eq!(selected(&h), Some(ids[0]));
    release_ctrl(&mut h);
    assert_eq!(active(&h), Some(ids[0]));
}

#[test]
fn arrow_keys_move_and_enter_confirms_with_ctrl_still_down() {
    let (mut h, ids) = three();
    ctrl_tab(&mut h);
    key_while_ctrl(&mut h, egui::Key::ArrowRight);
    assert_eq!(selected(&h), Some(ids[0]));
    key_while_ctrl(&mut h, egui::Key::ArrowLeft);
    assert_eq!(selected(&h), Some(ids[1]));
    key_while_ctrl(&mut h, egui::Key::ArrowLeft);
    assert_eq!(selected(&h), Some(ids[2]));
    key_while_ctrl(&mut h, egui::Key::Enter);
    assert!(h.state().switcher.is_none());
    assert_eq!(active(&h), Some(ids[2]));
    release_ctrl(&mut h);
}

fn key_while_ctrl(h: &mut Harness<'static, SheafApp>, key: egui::Key) {
    press(h, key, Modifiers::CTRL);
}

#[test]
fn escape_cancels_and_the_active_window_stays() {
    let (mut h, ids) = three();
    ctrl_tab(&mut h);
    key_while_ctrl(&mut h, egui::Key::Escape);
    assert!(h.state().switcher.is_none());
    release_ctrl(&mut h);
    assert_eq!(active(&h), Some(ids[2]));
}

#[test]
fn a_click_on_a_card_activates_that_window() {
    let (mut h, ids) = three();
    ctrl_tab(&mut h);
    let title = project(&h).window(ids[0]).unwrap().title();
    h.get_by_label(&format!("Switch to {title}")).click();
    settle(&mut h);
    assert!(h.state().switcher.is_none());
    assert_eq!(active(&h), Some(ids[0]));
    release_ctrl(&mut h);
    assert_eq!(active(&h), Some(ids[0]));
}

#[test]
fn a_hidden_window_is_listed_and_shown_when_chosen() {
    let (mut h, ids) = three();
    h.state_mut().workspace.hide(ids[0]);
    settle(&mut h);
    assert!(!h.state().workspace.is_shown(ids[0]));
    ctrl_shift_tab(&mut h);
    assert_eq!(selected(&h), Some(ids[0]), "the hidden window is the last card");
    release_ctrl(&mut h);
    assert!(h.state().workspace.is_shown(ids[0]));
    assert_eq!(active(&h), Some(ids[0]));
}

#[test]
fn closing_the_active_window_makes_the_one_behind_it_the_previous() {
    let (mut h, ids) = three();
    h.state_mut().workspace.hide(ids[2]);
    settle(&mut h);
    assert_eq!(active(&h), Some(ids[1]));
    ctrl_tab(&mut h);
    assert_eq!(selected(&h), Some(ids[0]), "A, not the window just closed");
    release_ctrl(&mut h);
}

#[test]
fn only_the_windows_of_the_selected_explorer_folder_are_listed() {
    let (mut h, ids) = three();
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::NewFolder(vec![]), &ctx);
    h.state_mut().apply(Action::MoveWindow { window: ids[2], folder: vec![0] }, &ctx);
    settle(&mut h);
    h.state_mut().panels.explorer.folder = vec![];
    ctrl_tab(&mut h);
    let n = h.state().switcher.as_ref().unwrap().titles().len();
    assert_eq!(n, 2, "the window in the other folder is left out");
    release_ctrl(&mut h);
    // The folder with one window has nothing to switch to.
    h.state_mut().panels.explorer.folder = vec![0];
    ctrl_tab(&mut h);
    assert!(h.state().switcher.is_none());
    release_ctrl(&mut h);
}

#[test]
fn with_one_window_or_none_ctrl_tab_does_nothing() {
    let mut h = start_auto_hide(vec![]);
    ctrl_tab(&mut h);
    assert!(h.state().switcher.is_none());
    release_ctrl(&mut h);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::NewWorkbook, &ctx);
    settle(&mut h);
    ctrl_tab(&mut h);
    assert!(h.state().switcher.is_none());
    release_ctrl(&mut h);
}

#[test]
fn the_window_menu_opens_it_and_it_stays_until_a_choice() {
    let (mut h, ids) = three();
    menu(&mut h, "Window", "Window Switcher");
    assert_eq!(selected(&h), Some(ids[1]));
    h.run_steps(5);
    assert!(h.state().switcher.is_some(), "no Ctrl to release when opened from the menu");
    tab_event(&mut h, Modifiers::NONE);
    assert_eq!(selected(&h), Some(ids[0]));
    key(&mut h, egui::Key::Enter);
    assert_eq!(active(&h), Some(ids[0]));
}

#[test]
fn plain_tab_in_the_grid_moves_the_cell_and_ctrl_tab_still_opens_the_switcher() {
    let mut h = start(vec![data_file("switcher")]);
    show_window(&mut h, "Book1 - sample.csv");
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::NewWorkbook, &ctx);
    settle(&mut h);
    let first = project(&h).workbooks().next().unwrap().id;
    h.state_mut().focus(first);
    settle(&mut h);
    click_text(&mut h, "4.5");
    let cell = |h: &Harness<'static, SheafApp>| h.state().books[&first].sel.clone();
    assert_eq!(cell(&h), at(1, 1));
    tab_event(&mut h, Modifiers::NONE);
    assert_eq!(cell(&h), at(2, 1), "Tab moved the active cell");
    assert!(h.state().switcher.is_none());
    ctrl_tab(&mut h);
    assert!(h.state().switcher.is_some(), "Ctrl+Tab works with the grid focused");
    let moved = cell(&h);
    release_ctrl(&mut h);
    assert_eq!(active(&h), project(&h).workbooks().last().map(|b| b.id));
    assert_eq!(cell(&h), moved, "the grid did not take the Ctrl+Tab");
}

#[test]
fn restoring_a_project_makes_the_topmost_window_the_most_recent_one() {
    let (mut h, ids) = three();
    // A saved stacking order whose topmost window (ids[0]) is not the last one added.
    let places: Vec<sheaf_core::project::WindowPlace> = [ids[2], ids[1], ids[0]]
        .iter()
        .enumerate()
        .map(|(k, id)| sheaf_core::project::WindowPlace {
            window: *id,
            x: 20.0 + 30.0 * k as f64,
            y: 20.0,
            width: 400.0,
            height: 300.0,
            state: sheaf_core::project::WindowState::Normal,
        })
        .collect();
    h.state_mut().workspace.restore(&places);
    settle(&mut h);
    assert_eq!(active(&h), Some(ids[0]));
    ctrl_tab(&mut h);
    assert_ne!(selected(&h), Some(ids[0]), "the active window is not the first selection");
    assert_eq!(selected(&h), Some(ids[2]));
    release_ctrl(&mut h);
    assert_eq!(active(&h), Some(ids[2]));
}

#[test]
fn the_first_selection_skips_the_active_window_only_when_it_is_listed() {
    let (mut h, ids) = three();
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::NewFolder(vec![]), &ctx);
    // Folder 0 holds the two older windows; the active window (ids[2]) stays at the top level.
    h.state_mut().apply(Action::MoveWindow { window: ids[0], folder: vec![0] }, &ctx);
    h.state_mut().apply(Action::MoveWindow { window: ids[1], folder: vec![0] }, &ctx);
    settle(&mut h);
    h.state_mut().focus(ids[2]);
    settle(&mut h);
    h.state_mut().panels.explorer.folder = vec![0];
    ctrl_tab(&mut h);
    assert_eq!(selected(&h), Some(ids[1]), "the most recent window of the folder, not the second");
    release_ctrl(&mut h);
    assert_eq!(active(&h), Some(ids[1]));
}

#[test]
fn other_keys_are_swallowed_while_the_switcher_is_open() {
    let mut h = start(vec![data_file("switcher-modal")]);
    show_window(&mut h, "Book1 - sample.csv");
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::NewWorkbook, &ctx);
    settle(&mut h);
    let first = project(&h).workbooks().next().unwrap().id;
    h.state_mut().focus(first);
    settle(&mut h);
    click_text(&mut h, "4.5");
    let before = column(&h, "B");
    ctrl_tab(&mut h);
    assert!(h.state().switcher.is_some());
    press(&mut h, egui::Key::Delete, Modifiers::CTRL);
    h.event(egui::Event::Text("x".to_string()));
    settle(&mut h);
    assert!(h.state().switcher.is_some());
    assert_eq!(column(&h, "B"), before, "Delete did not clear the cell");
    assert!(h.state().books[&first].editing.is_none(), "the letter did not start an edit");
    key_while_ctrl(&mut h, egui::Key::Escape);
    release_ctrl(&mut h);
    assert!(h.state().switcher.is_none());
    // The menu-opened (sticky) switcher is modal too.
    menu(&mut h, "Window", "Window Switcher");
    assert!(h.state().switcher.is_some());
    press(&mut h, egui::Key::Delete, Modifiers::NONE);
    assert_eq!(column(&h, "B"), before);
    press(&mut h, egui::Key::Escape, Modifiers::NONE);
    assert!(h.state().switcher.is_none());
    // After Esc, typing edits normally.
    h.state_mut().focus(first);
    settle(&mut h);
    click_text(&mut h, "4.5");
    press(&mut h, egui::Key::Delete, Modifiers::NONE);
    assert_ne!(column(&h, "B"), before, "Delete works again once the switcher is closed");
}
