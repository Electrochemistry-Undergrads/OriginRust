//! The Dialog Theme row of the import windows (T46): saved themes, `<last used>`, Delete, and the
//! rule that the file list and the target are not part of a theme. Every test passes a temporary
//! theme folder of its own; none reads or writes the user's settings folder.

use super::*;
use crate::actions::ImportPick;
use sheaf_io::ImportKind;
use sheaf_io::ascii::Delimiter;
use sheaf_io::batch::ImportMode;
use sheaf_io::themes::{self, AsciiSettings, ChiSettings, ExcelSettings, Kind, ThemeStore};

/// A fresh, empty folder for one test.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-app-themes-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Starts the application with this theme folder; the "restart" of a test starts it again.
fn start_with(themes_dir: Option<PathBuf>) -> Harness<'static, SheafApp> {
    let startup = Startup { themes_dir, ..Default::default() };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_step_dt(0.02)
        .build_eframe(|cc| SheafApp::new(cc, startup));
    settle(&mut h);
    h
}

fn csv(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, "t,a\n1,2.5\n2,4.5\n3,6.5\n").unwrap();
    path
}

fn open_ascii(h: &mut Harness<'static, SheafApp>, file: &Path) {
    h.state_mut().start_import(Some(vec![file.to_path_buf()]), ImportPick::Single);
    settle(h);
}

fn open_excel(h: &mut Harness<'static, SheafApp>, file: &Path) {
    h.state_mut().start_import(Some(vec![file.to_path_buf()]), ImportPick::Excel);
    settle(h);
}

fn ascii<'a>(h: &'a Harness<'static, SheafApp>) -> &'a crate::dialogs::import::ImportDialog {
    h.state().import.as_ref().expect("the import window is open")
}

fn ascii_mut<'a>(h: &'a mut Harness<'static, SheafApp>) -> &'a mut crate::dialogs::import::ImportDialog {
    h.state_mut().import.as_mut().expect("the import window is open")
}

fn excel_window<'a>(h: &'a mut Harness<'static, SheafApp>) -> &'a mut crate::dialogs::import_excel::ExcelImportDialog {
    h.state_mut().import_excel.as_mut().expect("the Excel window is open")
}

/// Sets the options a theme should bring back: semicolon, the CHI current inverted, a text type.
fn tweak(h: &mut Harness<'static, SheafApp>) {
    ascii_mut(h).edit_options(|kind, ascii, chi| {
        *kind = ImportKind::Ascii;
        ascii.delimiter = Delimiter::Semicolon;
        ascii.rename_sheet = false;
        chi.invert_current = true;
    });
    settle(h);
}

/// Menu of the row, then an entry of it.
fn row_menu(h: &mut Harness<'static, SheafApp>, entry: &str) {
    click(h, "Manage");
    click(h, entry);
}

/// Save As... and the name typed in the prompt.
fn save_as(h: &mut Harness<'static, SheafApp>, name: &str) {
    row_menu(h, "Save As...");
    // The box starts with the name of the selected theme.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text(name.to_string()));
    settle(h);
    // The prompt's Save, not the menu entry (the menu is closed by now).
    click(h, "Save");
}

/// Picks an entry of the Dialog Theme list.
fn choose(h: &mut Harness<'static, SheafApp>, shown: &str, entry: &str) {
    h.get_by_value(shown).click();
    settle(h);
    click(h, entry);
}

fn ok(h: &mut Harness<'static, SheafApp>) {
    // The window's OK is the only button of that name while the prompt is closed.
    click(h, "OK");
}

#[test]
fn without_a_theme_folder_the_window_has_no_theme_row() {
    let dir = scratch("none");
    let mut h = start_with(None);
    open_ascii(&mut h, &csv(&dir, "a.csv"));
    assert!(!has_label(&h, "Dialog Theme"));
    ok(&mut h);
    assert!(h.state().import.is_none());
    assert!(!dir.join("import-themes").exists());
}

#[test]
fn a_new_window_shows_the_row_and_starts_from_default() {
    let dir = scratch("fresh");
    let mut h = start_with(Some(dir.join("themes")));
    open_ascii(&mut h, &csv(&dir, "a.csv"));
    assert!(has_label(&h, "Dialog Theme"));
    assert_eq!(ascii(&h).theme_entry(), "<default>");
    assert_eq!(ascii(&h).theme_state().0, AsciiSettings::default());
    assert!(!dir.join("themes").exists(), "opening a window writes nothing");
}

#[test]
fn save_as_writes_the_theme_and_a_restart_can_refill_the_window_from_it() {
    let dir = scratch("restart");
    let themes_dir = dir.join("themes");
    let file = csv(&dir, "a.csv");
    let mut h = start_with(Some(themes_dir.clone()));
    open_ascii(&mut h, &file);
    tweak(&mut h);
    let want = ascii(&h).theme_state();
    save_as(&mut h, "CHI CV 3 header lines");
    assert_eq!(ascii(&h).theme_entry(), "CHI CV 3 header lines");
    assert_eq!(ascii(&h).theme_names(), ["CHI CV 3 header lines"]);
    // Cancel records nothing: the theme is saved, `<last used>` is not.
    click(&mut h, "Cancel");
    assert!(h.state().import.is_none());
    let store = ThemeStore::load(&Kind::Ascii.file(&themes_dir));
    assert!(store.last_used.is_none(), "Cancel does not record <last used>");
    assert_eq!(store.names(), ["CHI CV 3 header lines"]);

    // A new run: the window opens at <default> (nothing was confirmed), the theme is in the list.
    let mut h = start_with(Some(themes_dir.clone()));
    open_ascii(&mut h, &file);
    assert_eq!(ascii(&h).theme_entry(), "<default>");
    assert_eq!(ascii(&h).theme_state().0, AsciiSettings::default());
    choose(&mut h, "<default>", "CHI CV 3 header lines");
    assert_eq!(ascii(&h).theme_entry(), "CHI CV 3 header lines");
    assert_eq!(ascii(&h).theme_state(), want, "every option is back, the CHI ones too");
    assert!(want.1.chi.invert_current);
    assert_eq!(ascii(&h).file_list(), [file], "the file list is not part of a theme");
}

#[test]
fn ok_records_last_used_and_a_fresh_window_opens_with_it() {
    let dir = scratch("last");
    let themes_dir = dir.join("themes");
    let file = csv(&dir, "a.csv");
    let mut h = start_with(Some(themes_dir.clone()));
    open_ascii(&mut h, &file);
    tweak(&mut h);
    let want = ascii(&h).theme_state();
    ok(&mut h);
    assert!(h.state().import.is_none());
    assert_eq!(project(&h).workbooks().count(), 1);

    // Same run, next window.
    open_ascii(&mut h, &file);
    assert_eq!(ascii(&h).theme_entry(), "<last used>");
    assert_eq!(ascii(&h).theme_state(), want);
    click(&mut h, "Cancel");

    // After a restart too, and the two stores each have their half.
    let mut h = start_with(Some(themes_dir.clone()));
    open_ascii(&mut h, &file);
    assert_eq!(ascii(&h).theme_entry(), "<last used>");
    assert_eq!(ascii(&h).theme_state(), want);
    assert_eq!(themes::last_used::<AsciiSettings>(&themes_dir).unwrap().ascii.delimiter, Delimiter::Semicolon);
    assert!(themes::last_used::<ChiSettings>(&themes_dir).unwrap().chi.invert_current);
}

#[test]
fn cancel_after_changes_keeps_the_earlier_last_used() {
    let dir = scratch("cancel");
    let themes_dir = dir.join("themes");
    let file = csv(&dir, "a.csv");
    let mut h = start_with(Some(themes_dir.clone()));
    open_ascii(&mut h, &file);
    tweak(&mut h);
    ok(&mut h);
    open_ascii(&mut h, &file);
    ascii_mut(&mut h).edit_options(|_, a, _| a.delimiter = Delimiter::Tab);
    click(&mut h, "Cancel");
    let last = themes::last_used::<AsciiSettings>(&themes_dir).unwrap();
    assert_eq!(last.ascii.delimiter, Delimiter::Semicolon);
}

#[test]
fn choosing_default_and_last_used_fills_the_window_and_leaves_the_target_alone() {
    let dir = scratch("choose");
    let themes_dir = dir.join("themes");
    let file = csv(&dir, "a.csv");
    let mut h = start_with(Some(themes_dir.clone()));
    open_ascii(&mut h, &file);
    tweak(&mut h);
    ok(&mut h);
    // A book is active now, so the window has a target and the import mode list is usable.
    open_ascii(&mut h, &file);
    ascii_mut(&mut h).edit_options(|_, a, _| a.delimiter = Delimiter::Tab);
    choose(&mut h, "<last used>", "<default>");
    assert_eq!(ascii(&h).theme_state(), (AsciiSettings::default(), ChiSettings::default()));
    choose(&mut h, "<default>", "<last used>");
    assert_eq!(ascii(&h).theme_state().0.ascii.delimiter, Delimiter::Semicolon);
    assert_eq!(ascii(&h).file_list(), [file]);
}

#[test]
fn last_used_before_any_ok_gives_the_defaults() {
    let dir = scratch("nolast");
    let mut h = start_with(Some(dir.join("themes")));
    open_ascii(&mut h, &csv(&dir, "a.csv"));
    tweak(&mut h);
    choose(&mut h, "<default>", "<last used>");
    assert_eq!(ascii(&h).theme_entry(), "<last used>");
    assert_eq!(ascii(&h).theme_state(), (AsciiSettings::default(), ChiSettings::default()));
    assert!(ascii(&h).theme_message().is_none());
}

#[test]
fn save_writes_over_the_selected_theme() {
    let dir = scratch("save");
    let themes_dir = dir.join("themes");
    let mut h = start_with(Some(themes_dir.clone()));
    open_ascii(&mut h, &csv(&dir, "a.csv"));
    tweak(&mut h);
    save_as(&mut h, "Mine");
    ascii_mut(&mut h).edit_options(|_, a, _| a.delimiter = Delimiter::Tab);
    row_menu(&mut h, "Save");
    assert!(!has_label(&h, "Save Dialog Theme As"), "a saved theme is written without asking");
    let saved = themes::theme::<AsciiSettings>(&themes_dir, "mine").unwrap();
    assert_eq!(saved.ascii.delimiter, Delimiter::Tab);
    assert_eq!(themes::names::<AsciiSettings>(&themes_dir), ["Mine"], "one theme, not two");
}

#[test]
fn save_on_a_built_in_entry_asks_for_a_name() {
    let dir = scratch("savebuiltin");
    let mut h = start_with(Some(dir.join("themes")));
    open_ascii(&mut h, &csv(&dir, "a.csv"));
    row_menu(&mut h, "Save");
    assert!(has_label(&h, "Save Dialog Theme As"));
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(!has_label(&h, "Save Dialog Theme As"));
    assert!(ascii(&h).theme_names().is_empty());
    assert!(!dir.join("themes").exists(), "a cancelled prompt writes nothing");
}

#[test]
fn the_name_prompt_refuses_an_empty_name_and_the_built_in_names() {
    let dir = scratch("names");
    let mut h = start_with(Some(dir.join("themes")));
    open_ascii(&mut h, &csv(&dir, "a.csv"));
    row_menu(&mut h, "Save As...");
    let prompt_save =
        |h: &Harness<'static, SheafApp>| h.query_all_by_label("Save").any(|n| !n.accesskit_node().is_disabled());
    assert!(!prompt_save(&h), "no name, no Save");
    h.event(egui::Event::Text("<default>".to_string()));
    settle(&mut h);
    assert!(!prompt_save(&h), "the built-in names are taken");
    assert!(has_label_containing(&h, "built-in entries"));
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(ascii(&h).theme_names().is_empty());
}

#[test]
fn saving_under_a_name_in_other_letter_case_replaces_the_theme() {
    let dir = scratch("case");
    let themes_dir = dir.join("themes");
    let mut h = start_with(Some(themes_dir.clone()));
    open_ascii(&mut h, &csv(&dir, "a.csv"));
    save_as(&mut h, "cv scans");
    tweak(&mut h);
    save_as(&mut h, "  CV   Scans ");
    assert_eq!(ascii(&h).theme_names(), ["CV Scans"], "one theme with the new spelling");
    assert_eq!(themes::theme::<AsciiSettings>(&themes_dir, "cv scans").unwrap().ascii.delimiter, Delimiter::Semicolon);
}

#[test]
fn delete_removes_the_theme_from_the_list_and_the_file_and_resets_the_window() {
    let dir = scratch("delete");
    let themes_dir = dir.join("themes");
    let file = csv(&dir, "a.csv");
    let mut h = start_with(Some(themes_dir.clone()));
    open_ascii(&mut h, &file);
    save_as(&mut h, "Keep");
    tweak(&mut h);
    save_as(&mut h, "Drop");
    assert_eq!(ascii(&h).theme_names(), ["Keep", "Drop"]);
    row_menu(&mut h, "Delete");
    assert_eq!(ascii(&h).theme_names(), ["Keep"]);
    assert_eq!(ascii(&h).theme_entry(), "<default>");
    assert_eq!(ascii(&h).theme_state().0, AsciiSettings::default());
    assert_eq!(themes::names::<AsciiSettings>(&themes_dir), ["Keep"]);
    assert_eq!(themes::names::<ChiSettings>(&themes_dir), ["Keep"], "the CHI half goes too");
    // Gone after a restart.
    click(&mut h, "Cancel");
    let mut h = start_with(Some(themes_dir));
    open_ascii(&mut h, &file);
    assert_eq!(ascii(&h).theme_names(), ["Keep"]);
}

#[test]
fn delete_is_off_for_the_built_in_entries() {
    let dir = scratch("nodelete");
    let mut h = start_with(Some(dir.join("themes")));
    open_ascii(&mut h, &csv(&dir, "a.csv"));
    click(&mut h, "Manage");
    assert!(is_disabled(&h, "Delete"));
}

#[test]
fn a_damaged_theme_file_opens_the_window_at_default() {
    let dir = scratch("damaged");
    let themes_dir = dir.join("themes");
    std::fs::create_dir_all(&themes_dir).unwrap();
    std::fs::write(Kind::Ascii.file(&themes_dir), "{ this is not json").unwrap();
    let mut h = start_with(Some(themes_dir));
    open_ascii(&mut h, &csv(&dir, "a.csv"));
    assert_eq!(ascii(&h).theme_entry(), "<default>");
    assert!(ascii(&h).theme_names().is_empty());
}

#[test]
fn a_drop_with_a_preset_comma_keeps_the_comma_over_last_used() {
    let dir = scratch("preset");
    let themes_dir = dir.join("themes");
    let file = csv(&dir, "a.csv");
    let mut h = start_with(Some(themes_dir));
    open_ascii(&mut h, &file);
    tweak(&mut h);
    ok(&mut h);
    h.state_mut().start_import(Some(vec![file]), ImportPick::Csv);
    settle(&mut h);
    assert_eq!(ascii(&h).delimiter(), Delimiter::Comma, "File > Import > CSV is comma whatever was used last");
}

#[test]
fn the_excel_window_has_its_own_themes_and_never_stores_the_ticked_sheets() {
    let dir = scratch("excel");
    let themes_dir = dir.join("themes");
    let file = super::excel::excel_file("themes");
    let mut h = start_with(Some(themes_dir.clone()));
    open_excel(&mut h, &file);
    assert!(has_label(&h, "Dialog Theme"));
    excel_window(&mut h).edit_options(|o| {
        o.header_lines = Some(2);
        o.rename_book = false;
    });
    // Untick a sheet: that belongs to the file.
    click(&mut h, "Scan 2");
    save_as(&mut h, "Two header rows");
    assert_eq!(excel_window(&mut h).theme_names(), ["Two header rows"]);
    let stored = themes::theme::<ExcelSettings>(&themes_dir, "Two header rows").unwrap();
    assert_eq!(stored.options.header_lines, Some(2));
    assert!(!stored.options.rename_book);
    assert!(stored.options.sheets.is_empty(), "the ticked sheets are not stored");
    assert!(themes::names::<AsciiSettings>(&themes_dir).is_empty(), "the text window does not see it");

    // The same file again, a new window: choosing the theme brings the options and leaves the ticks.
    ok(&mut h);
    open_excel(&mut h, &file);
    assert_eq!(excel_window(&mut h).theme_entry(), "<last used>", "OK recorded the window");
    assert_eq!(excel_window(&mut h).theme_settings().options.header_lines, Some(2));
    choose(&mut h, "<last used>", "<default>");
    assert_eq!(excel_window(&mut h).theme_settings(), ExcelSettings::default());
    assert_eq!(excel_window(&mut h).selected_sheets(0), ["Scan 1", "Scan 2", "Notes"], "all sheets ticked as at open");
    choose(&mut h, "<default>", "Two header rows");
    assert_eq!(excel_window(&mut h).theme_settings().options.header_lines, Some(2));
    assert_eq!(excel_window(&mut h).selected_sheets(0), ["Scan 1", "Scan 2", "Notes"], "a theme leaves the ticks");
}

#[test]
fn the_wizard_starts_from_last_used_and_finish_records_it() {
    let dir = scratch("wizard");
    let themes_dir = dir.join("themes");
    let file = csv(&dir, "a.csv");
    let mut h = start_with(Some(themes_dir.clone()));
    open_ascii(&mut h, &file);
    tweak(&mut h);
    ok(&mut h);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::ImportWizard, &ctx);
    settle(&mut h);
    let wizard = h.state().import_wizard.as_ref().expect("the wizard is open");
    assert_eq!(wizard.theme_entry(), "<last used>");
    assert_eq!(wizard.dialog().theme_state().0.ascii.delimiter, Delimiter::Semicolon);
    assert!(has_label(&h, "Dialog Theme"));
}

#[test]
fn a_theme_with_a_value_this_version_does_not_know_is_listed_and_reports_why() {
    let dir = scratch("unreadable");
    let themes_dir = dir.join("themes");
    std::fs::create_dir_all(&themes_dir).unwrap();
    let json =
        r#"{"version":1,"themes":[{"name":"From the future","settings":{"ascii":{"delimiter":{"Telepathy":1}}}}]}"#;
    std::fs::write(Kind::Ascii.file(&themes_dir), json).unwrap();
    let mut h = start_with(Some(themes_dir.clone()));
    open_ascii(&mut h, &csv(&dir, "a.csv"));
    assert_eq!(ascii(&h).theme_names(), ["From the future"]);
    choose(&mut h, "<default>", "From the future");
    assert_eq!(ascii(&h).theme_entry(), "<default>", "the window is unchanged");
    assert!(ascii(&h).theme_message().unwrap().contains("cannot be read"));
    assert!(std::fs::read_to_string(Kind::Ascii.file(&themes_dir)).unwrap().contains("From the future"));
}

#[test]
fn finish_in_the_wizard_records_last_used_and_cancel_does_not() {
    let dir = scratch("wizardfinish");
    let themes_dir = dir.join("themes");
    let file = csv(&dir, "a.csv");
    let mut h = start_with(Some(themes_dir.clone()));
    let wizard = h.state().wizard(vec![file.clone()]);
    h.state_mut().import_wizard = Some(wizard);
    settle(&mut h);
    click(&mut h, "Cancel");
    assert!(h.state().import_wizard.is_none());
    assert!(themes::last_used::<AsciiSettings>(&themes_dir).is_none(), "Cancel records nothing");

    let wizard = h.state().wizard(vec![file]);
    h.state_mut().import_wizard = Some(wizard);
    settle(&mut h);
    click(&mut h, "Finish");
    assert!(h.state().import_wizard.is_none());
    assert_eq!(project(&h).workbooks().count(), 1);
    assert!(themes::last_used::<AsciiSettings>(&themes_dir).is_some(), "Finish records <last used>");
}

#[test]
fn a_window_with_no_target_keeps_the_mode_of_a_theme_through_save_and_ok() {
    let dir = scratch("mode-no-target");
    let themes_dir = dir.join("themes");
    let file = csv(&dir, "a.csv");
    let append = AsciiSettings { mode: ImportMode::NewColumns, ..AsciiSettings::default() };
    themes::save_theme(&themes_dir, "CHI append", &append).unwrap();
    let mut h = start_with(Some(themes_dir.clone()));
    // No workbook is open, so the window has no target sheet.
    open_ascii(&mut h, &file);
    choose(&mut h, "<default>", "CHI append");
    ascii_mut(&mut h).edit_options(|_, a, _| a.delimiter = Delimiter::Semicolon);
    settle(&mut h);
    assert_eq!(ascii(&h).theme_state().0.mode, ImportMode::NewColumns, "the window still holds the theme's mode");
    row_menu(&mut h, "Save");
    let stored = themes::theme::<AsciiSettings>(&themes_dir, "CHI append").unwrap();
    assert_eq!(stored.ascii.delimiter, Delimiter::Semicolon);
    assert_eq!(stored.mode, ImportMode::NewColumns, "Save does not rewrite the mode");
    ok(&mut h);
    assert_eq!(
        themes::last_used::<AsciiSettings>(&themes_dir).unwrap().mode,
        ImportMode::NewColumns,
        "OK records the mode too"
    );
    assert_eq!(project(&h).workbooks().count(), 1, "with no target the data still went into a new book");
}

#[test]
fn the_import_windows_have_save_as_windows_of_their_own() {
    let dir = scratch("prompt-ids");
    let mut h = start_with(Some(dir.join("themes")));
    let text = csv(&dir, "a.csv");
    let book = super::excel::excel_file("prompt-ids");
    h.state_mut().start_import(Some(vec![text, book]), ImportPick::Multiple);
    settle(&mut h);
    let text_id = ascii(&h).theme_prompt_id();
    let excel_id = excel_window(&mut h).theme_prompt_id();
    assert_ne!(text_id, excel_id);
    let wizard = h.state().wizard(vec![csv(&dir, "b.csv")]);
    assert_ne!(wizard.dialog().theme_prompt_id(), text_id);
    assert_ne!(wizard.dialog().theme_prompt_id(), excel_id);
}

#[test]
fn the_wizard_closes_the_save_as_window_when_it_leaves_the_source_page() {
    let dir = scratch("wizard-prompt");
    let mut h = start_with(Some(dir.join("themes")));
    let wizard = h.state().wizard(vec![csv(&dir, "a.csv")]);
    h.state_mut().import_wizard = Some(wizard);
    settle(&mut h);
    row_menu(&mut h, "Save As...");
    assert!(h.state().import_wizard.as_ref().unwrap().dialog().theme_prompt_open());
    assert!(has_label(&h, "Theme name"));
    click(&mut h, "Next >>");
    assert!(!h.state().import_wizard.as_ref().unwrap().dialog().theme_prompt_open());
    click(&mut h, "<< Back");
    assert!(!has_label(&h, "Theme name"), "the window does not come back with a stale name and no focus");
}

#[test]
fn add_sparklines_in_import_options_gives_the_imported_sheet_its_sparklines_row() {
    let dir = scratch("sparklines");
    let mut h = start_with(Some(dir.join("themes")));
    let file = csv(&dir, "s.csv");
    open_ascii(&mut h, &file);
    assert_eq!(ascii(&h).theme_state().0.ascii.sparklines, sheaf_io::ascii::AddSparklines::No);
    // The list of Import Options: Origin's three choices.
    choose(&mut h, "No", "Yes(if less than 50 columns)");
    click(&mut h, "OK");
    let sheet = &project(&h).workbooks().next().unwrap().sheets[0];
    assert!(sheet.view.sparklines, "the Sparklines row is on");
    assert!(sheet.columns.iter().all(|c| c.sparkline));
}
