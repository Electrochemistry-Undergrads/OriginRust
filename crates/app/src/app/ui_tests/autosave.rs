//! Autosave and recovery (T28): the timer with an injected clock, the clean exit, the start-up
//! question and the Autosave window. Every test uses its own folder under the temporary folder;
//! none touches the user's real recovery folder.

use super::*;
use sheaf_io::recovery::{self, Session};
use std::time::Duration;

const MINUTE: Duration = Duration::from_secs(60);

/// A fresh, empty folder for one test.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-app-autosave-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Starts the application with a recovery folder (and, when given, a settings file).
fn start_in(paths: Vec<PathBuf>, recovery_dir: &Path, settings_path: Option<PathBuf>) -> Harness<'static, SheafApp> {
    let startup =
        Startup { paths, recovery_dir: Some(recovery_dir.to_path_buf()), settings_path, ..Default::default() };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_step_dt(0.02)
        .build_eframe(|cc| SheafApp::new(cc, startup));
    settle(&mut h);
    h
}

/// Runs frames until the worker thread has written the recovery (the sidecar comes last).
fn wait_for_write(h: &mut Harness<'static, SheafApp>) {
    let sidecar = h.state().autosave.project_path().unwrap().with_extension("json");
    for _ in 0..500 {
        h.step();
        if sidecar.exists() && h.state().autosave.idle() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the recovery file was not written");
}

fn recovery_file(h: &Harness<'static, SheafApp>) -> PathBuf {
    h.state().autosave.project_path().unwrap()
}

/// What a crashed run leaves in `dir`: the project of `h` saved by a session that is then dropped
/// (which releases the lock and keeps the files, as a kill does).
fn plant_orphan(dir: &Path, project: &Project, original: Option<&Path>) -> String {
    let session = Session::start(dir).unwrap();
    session.save(project, original).unwrap();
    let id = session.id().to_string();
    drop(session);
    id
}

fn orphans(dir: &Path) -> usize {
    recovery::list_orphans(dir).len()
}

fn files_with_extension(dir: &Path, ext: &str) -> usize {
    std::fs::read_dir(dir).unwrap().flatten().filter(|e| e.path().extension().is_some_and(|x| x == ext)).count()
}

fn warnings_containing(h: &Harness<'static, SheafApp>, text: &str) -> usize {
    h.state().logs.messages.iter().filter(|m| m.text.contains(text)).count()
}

/// The project with the window places left out: a snapshot takes the places of the windows as they
/// are drawn, which a project that was never saved does not have yet.
fn content(project: &Project) -> Project {
    let mut p = project.clone();
    p.window_places.clear();
    p
}

/// A project from the test data file, written by a crashed run, and the project itself.
fn planted(dir: &Path, original: Option<&Path>) -> Project {
    let h = start(vec![data_file("autosave-source")]);
    let project = h.state().doc.project().clone();
    plant_orphan(dir, &project, original);
    project
}

#[test]
fn a_modified_project_is_written_after_the_interval_and_not_before() {
    let dir = scratch("interval");
    let mut h = start_in(vec![data_file("autosave-interval")], &dir, None);
    assert!(h.state().doc.dirty, "the import made the project modified");
    assert!(!recovery_file(&h).exists());
    h.state_mut().autosave.advance(5 * MINUTE);
    settle(&mut h);
    std::thread::sleep(Duration::from_millis(50));
    settle(&mut h);
    assert_eq!(h.state().autosave.writes_started(), 0, "five minutes are not ten");
    assert!(h.state().autosave.idle());
    assert!(!recovery_file(&h).exists(), "five minutes are not ten");
    h.state_mut().autosave.advance(5 * MINUTE + Duration::from_secs(1));
    wait_for_write(&mut h);
    assert_eq!(h.state().autosave.writes_started(), 1);
    let saved = recovery::load(&recovery::Recovery {
        id: String::new(),
        project: recovery_file(&h),
        sidecar: None,
        modified: std::time::SystemTime::now(),
    })
    .unwrap();
    assert_eq!(content(&saved), content(h.state().doc.project()));
    assert!(h.state().doc.dirty, "writing the recovery does not make the project clean");
    assert_eq!(h.state().doc.undo_label(), Some("Import data"), "and adds no undo step");
}

#[test]
fn a_project_that_did_not_change_is_not_written_again() {
    let dir = scratch("unchanged");
    let mut h = start_in(vec![data_file("autosave-unchanged")], &dir, None);
    h.state_mut().autosave.advance(11 * MINUTE);
    wait_for_write(&mut h);
    let file = recovery_file(&h);
    std::fs::remove_file(&file).unwrap();
    h.state_mut().autosave.advance(11 * MINUTE);
    settle(&mut h);
    std::thread::sleep(Duration::from_millis(50));
    settle(&mut h);
    assert!(!file.exists(), "the same revision is not written twice");
}

#[test]
fn a_clean_project_writes_nothing() {
    let dir = scratch("clean");
    let mut h = start_in(Vec::new(), &dir, None);
    assert!(!h.state().doc.dirty);
    h.state_mut().autosave.advance(30 * MINUTE);
    settle(&mut h);
    std::thread::sleep(Duration::from_millis(50));
    settle(&mut h);
    assert!(!recovery_file(&h).exists());
}

#[test]
fn autosave_switched_off_writes_nothing() {
    let dir = scratch("off");
    let mut h = start_in(vec![data_file("autosave-off")], &dir, None);
    h.state_mut().autosave.settings.enabled = false;
    h.state_mut().autosave.advance(30 * MINUTE);
    settle(&mut h);
    std::thread::sleep(Duration::from_millis(50));
    settle(&mut h);
    assert!(!recovery_file(&h).exists());
}

#[test]
fn the_interval_comes_from_the_settings() {
    let dir = scratch("minutes");
    let mut h = start_in(vec![data_file("autosave-minutes")], &dir, None);
    h.state_mut().autosave.settings.minutes = 2;
    h.state_mut().autosave.advance(3 * MINUTE);
    wait_for_write(&mut h);
    assert!(recovery_file(&h).exists());
}

#[test]
fn a_clean_exit_removes_the_recovery_file() {
    let dir = scratch("exit");
    let mut h = start_in(vec![data_file("autosave-exit")], &dir, None);
    h.state_mut().autosave.advance(11 * MINUTE);
    wait_for_write(&mut h);
    assert_eq!(files_with_extension(&dir, "sheaf"), 1);
    menu(&mut h, "File", "Exit");
    click(&mut h, "Don't save");
    assert!(h.state().quit_confirmed);
    eframe::App::on_exit(h.state_mut(), None);
    assert_eq!(files_with_extension(&dir, "sheaf"), 0);
    assert_eq!(files_with_extension(&dir, "json"), 0);
    assert_eq!(files_with_extension(&dir, "lock"), 0);
}

#[test]
fn an_exit_nobody_was_asked_about_keeps_the_changes_for_the_next_start() {
    let dir = scratch("forced");
    let mut h = start_in(vec![data_file("autosave-forced")], &dir, None);
    assert!(!recovery_file(&h).exists(), "the timer has not fired yet");
    eframe::App::on_exit(h.state_mut(), None);
    assert_eq!(orphans(&dir), 1, "the last state was written and the lock released");
}

#[test]
fn a_normal_save_clears_the_recovery_file() {
    let dir = scratch("save");
    let mut h = start_in(vec![data_file("autosave-save")], &dir, None);
    h.state_mut().autosave.advance(11 * MINUTE);
    wait_for_write(&mut h);
    assert!(recovery_file(&h).exists());
    let target = scratch("save-target").join("lab.sheaf");
    assert!(h.state_mut().save_to(target.clone()));
    settle(&mut h);
    assert!(!recovery_file(&h).exists());
    assert!(!recovery_file(&h).with_extension("json").exists());
    assert!(target.exists(), "the user's own file is the one that was written");
}

#[test]
fn a_new_project_clears_the_recovery_file() {
    let dir = scratch("new");
    let mut h = start_in(vec![data_file("autosave-new")], &dir, None);
    h.state_mut().autosave.advance(11 * MINUTE);
    wait_for_write(&mut h);
    menu_path(&mut h, &["File", "New", "Project"]);
    click(&mut h, "Don't save");
    assert!(!h.state().doc.dirty);
    assert!(!recovery_file(&h).exists());
}

/// A failed removal of the recovery files is tried again at the next frame, so a stale file does
/// not outlive a clean project.
#[cfg(unix)]
#[test]
fn a_failed_clear_is_retried_until_the_files_are_gone() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("clear-retry");
    let mut h = start_in(vec![data_file("autosave-clear-retry")], &dir, None);
    h.state_mut().autosave.advance(11 * MINUTE);
    wait_for_write(&mut h);
    let file = recovery_file(&h);
    assert!(file.exists());
    // A read-only folder: the files cannot be removed.
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o500)).unwrap();
    if std::fs::OpenOptions::new().write(true).create_new(true).open(dir.join("probe")).is_ok() {
        // Running with rights that ignore folder permissions (root): the failure cannot be made.
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        return;
    }
    let target = scratch("clear-retry-target").join("lab.sheaf");
    assert!(h.state_mut().save_to(target));
    settle(&mut h);
    assert!(file.exists(), "the folder is read-only, so the file is still there");
    assert_eq!(warnings_containing(&h, "Autosave could not write"), 1, "said once");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    settle(&mut h);
    assert!(!file.exists(), "the next frame removed it");
    assert!(!file.with_extension("json").exists());
    assert_eq!(warnings_containing(&h, "Autosave could not write"), 1);
}

#[test]
fn a_failing_write_is_reported_once() {
    let dir = scratch("failing");
    let mut h = start_in(vec![data_file("autosave-failing")], &dir, None);
    std::fs::remove_dir_all(&dir).unwrap();
    for round in 1..=3 {
        h.state_mut().autosave.advance(11 * MINUTE);
        // Run frames until this round's write has started and ended.
        for _ in 0..1000 {
            h.step();
            if h.state().autosave.writes_started() == round && h.state().autosave.idle() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(h.state().autosave.writes_started(), round, "the timer fired once per round");
        assert!(h.state().autosave.idle());
        h.step();
    }
    assert_eq!(warnings_containing(&h, "Autosave could not write"), 1);
}

#[test]
fn start_up_without_a_recovery_folder_neither_scans_nor_writes() {
    let mut h = start(vec![data_file("autosave-none")]);
    assert!(!h.state().autosave.has_session());
    h.state_mut().autosave.advance(30 * MINUTE);
    settle(&mut h);
    assert!(h.state().recovery_offer.is_none());
}

// ----- the start-up question ---------------------------------------------

#[test]
fn a_planted_orphan_shows_the_dialog_at_start_up() {
    let dir = scratch("offer");
    planted(&dir, Some(Path::new("/work/lab.sheaf")));
    let h = start_in(Vec::new(), &dir, None);
    assert!(has_label(&h, "Sheaf found work that was not saved"));
    assert!(has_label(&h, "lab.sheaf"));
    assert!(has_label_containing(&h, "Saved"));
    assert!(has_label(&h, "Recover"));
    assert!(has_label(&h, "Discard"));
    assert!(has_label(&h, "Later"));
}

#[test]
fn an_orphan_without_a_known_file_is_called_untitled() {
    let dir = scratch("untitled");
    planted(&dir, None);
    let h = start_in(Vec::new(), &dir, None);
    assert!(has_label(&h, "Untitled"));
}

#[test]
fn several_orphans_are_counted_and_the_buttons_act_on_the_newest() {
    let dir = scratch("several");
    let project = planted(&dir, Some(Path::new("/work/old.sheaf")));
    let old = recovery::list_orphans(&dir).remove(0);
    // Make the first one older, then plant a newer one.
    std::fs::OpenOptions::new()
        .write(true)
        .open(&old.project)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - 60 * MINUTE)
        .unwrap();
    plant_orphan(&dir, &project, Some(Path::new("/work/new.sheaf")));
    let mut h = start_in(Vec::new(), &dir, None);
    assert!(has_label(&h, "new.sheaf"));
    assert!(has_label_containing(&h, "There are 2 recoveries"));
    click(&mut h, "Discard");
    assert_eq!(orphans(&dir), 1);
    assert!(has_label(&h, "old.sheaf"), "the next one is offered");
    assert!(!has_label_containing(&h, "There are"));
}

#[test]
fn recover_opens_the_project_modified_with_our_own_copy_and_removes_the_orphan() {
    let dir = scratch("recover");
    let original = scratch("recover-original").join("lab.sheaf");
    let project = planted(&dir, Some(&original));
    let mut h = start_in(Vec::new(), &dir, None);
    assert_eq!(orphans(&dir), 1);
    click(&mut h, "Recover");
    assert_eq!(content(h.state().doc.project()), content(&project), "the recovered data is the planted data");
    assert!(h.state().doc.dirty, "it is modified, so closing asks");
    assert!(h.state().path.is_none(), "it has no file name, so Save asks for one");
    assert_eq!(h.state().recovered_from.as_deref(), Some(original.as_path()));
    assert!(h.state().recovery_offer.is_none());
    assert_eq!(orphans(&dir), 0, "the orphan is gone");
    // This run's own copy exists and holds the project.
    let own = recovery_file(&h);
    assert!(own.exists());
    let loaded = sheaf_io::project_file::load(&own).unwrap();
    assert_eq!(content(&loaded), content(&project));
    assert_eq!(files_with_extension(&dir, "sheaf"), 1, "only our copy is left");
    assert!(!has_label(&h, "Sheaf found work that was not saved"));
}

#[test]
fn a_recovered_project_that_is_saved_goes_to_the_file_the_user_names() {
    let dir = scratch("recover-save");
    let project = planted(&dir, Some(Path::new("/work/lab.sheaf")));
    let mut h = start_in(Vec::new(), &dir, None);
    click(&mut h, "Recover");
    let target = scratch("recover-save-target").join("kept.sheaf");
    assert!(h.state_mut().save_to(target.clone()));
    settle(&mut h);
    assert_eq!(content(&sheaf_io::project_file::load(&target).unwrap()), content(&project));
    assert!(h.state().recovered_from.is_none());
    assert!(!recovery_file(&h).exists(), "saved, so the recovery copy is cleared");
}

#[test]
fn recover_asks_about_unsaved_changes_first() {
    let dir = scratch("recover-dirty");
    planted(&dir, None);
    let mut h = start_in(vec![data_file("autosave-recover-dirty")], &dir, None);
    assert!(h.state().doc.dirty);
    let before = h.state().doc.project().clone();
    click(&mut h, "Recover");
    assert!(has_label(&h, "The project has unsaved changes."));
    assert_eq!(orphans(&dir), 1, "nothing is opened or removed before the answer");
    assert_eq!(h.state().doc.project(), &before);
    click(&mut h, "Cancel");
    assert_eq!(orphans(&dir), 1);
    assert!(has_label(&h, "Recover"), "the offer is still there");
    click(&mut h, "Recover");
    click(&mut h, "Don't save");
    assert_eq!(orphans(&dir), 0);
    assert!(h.state().doc.dirty);
}

#[test]
fn discard_removes_the_orphan() {
    let dir = scratch("discard");
    planted(&dir, None);
    let mut h = start_in(Vec::new(), &dir, None);
    click(&mut h, "Discard");
    assert_eq!(orphans(&dir), 0);
    assert_eq!(files_with_extension(&dir, "sheaf"), 0);
    assert_eq!(files_with_extension(&dir, "json"), 0);
    assert!(!h.state().doc.dirty, "nothing was opened");
    assert!(!has_label(&h, "Sheaf found work that was not saved"));
}

#[test]
fn later_keeps_the_orphan() {
    let dir = scratch("later");
    planted(&dir, None);
    let mut h = start_in(Vec::new(), &dir, None);
    click(&mut h, "Later");
    assert!(!has_label(&h, "Sheaf found work that was not saved"));
    assert_eq!(orphans(&dir), 1);
    // The next start offers it again.
    let again = start_in(Vec::new(), &dir, None);
    assert!(has_label(&again, "Recover"));
}

#[test]
fn a_recovery_that_cannot_be_read_says_so_and_stays() {
    let dir = scratch("corrupt");
    let id = plant_orphan(&dir, &Project::new(), None);
    std::fs::write(dir.join(format!("{id}.sheaf")), b"not a project").unwrap();
    let mut h = start_in(Vec::new(), &dir, None);
    click(&mut h, "Recover");
    assert!(has_label(&h, "Could not recover the project"));
    assert_eq!(orphans(&dir), 1);
    assert!(!h.state().doc.dirty);
}

#[test]
fn an_orphan_older_than_thirty_days_is_dropped_at_start_up() {
    let dir = scratch("old");
    planted(&dir, None);
    let file = recovery::list_orphans(&dir).remove(0).project;
    std::fs::OpenOptions::new()
        .write(true)
        .open(&file)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - 31 * 24 * 60 * MINUTE)
        .unwrap();
    let h = start_in(Vec::new(), &dir, None);
    assert!(!has_label(&h, "Recover"));
    assert_eq!(orphans(&dir), 0);
}

#[test]
fn our_own_session_is_never_offered_to_ourselves() {
    let dir = scratch("own");
    let mut h = start_in(vec![data_file("autosave-own")], &dir, None);
    h.state_mut().autosave.advance(11 * MINUTE);
    wait_for_write(&mut h);
    assert_eq!(orphans(&dir), 0, "a running instance's files are not orphans");
    let second = start_in(Vec::new(), &dir, None);
    assert!(!has_label(&second, "Recover"), "and a second instance does not see them either");
}

// ----- Tools > Autosave -----------------------------------------------------

#[test]
fn the_autosave_window_changes_and_keeps_the_settings() {
    let dir = scratch("window");
    let settings = scratch("window-settings").join("autosave.json");
    let mut h = start_in(vec![data_file("autosave-window")], &dir, Some(settings.clone()));
    assert!(h.state().autosave.settings.enabled);
    menu(&mut h, "Tools", "Autosave...");
    assert!(has_label(&h, "Autosave the project"));
    click(&mut h, "Autosave the project");
    click(&mut h, "OK");
    assert!(!h.state().autosave.settings.enabled, "applied at once");
    assert!(!recovery::AutosaveSettings::load(&settings).enabled, "and kept");
    h.state_mut().autosave.advance(30 * MINUTE);
    settle(&mut h);
    std::thread::sleep(Duration::from_millis(50));
    settle(&mut h);
    assert!(!recovery_file(&h).exists());
    // The next start reads them.
    let again = start_in(Vec::new(), &dir, Some(settings));
    assert!(!again.state().autosave.settings.enabled);
}

#[test]
fn cancel_in_the_autosave_window_changes_nothing() {
    let dir = scratch("window-cancel");
    let settings = scratch("window-cancel-settings").join("autosave.json");
    let mut h = start_in(Vec::new(), &dir, Some(settings.clone()));
    menu(&mut h, "Tools", "Autosave...");
    click(&mut h, "Autosave the project");
    click(&mut h, "Cancel");
    assert!(h.state().autosave.settings.enabled);
    assert!(!settings.exists());
    assert!(!has_label(&h, "Autosave the project"));
}

#[test]
fn the_autosave_menu_entry_is_enabled_and_options_stays_grey() {
    let mut h = start(Vec::new());
    h.get_by_label("Tools").click();
    settle(&mut h);
    assert!(!menu_entry(&h, "Autosave...").accesskit_node().is_disabled());
    assert!(menu_entry(&h, "Options...").accesskit_node().is_disabled());
}

#[test]
fn undo_to_saved_clears_recovery_and_redo_writes_it_again() {
    let dir = scratch("undo-saved");
    let mut h = start_in(vec![data_file("autosave-undo-saved")], &dir, None);
    let target = scratch("undo-saved-target").join("saved.sheaf");
    assert!(h.state_mut().save_to(target));
    h.state_mut().doc.apply("Edit", |p| p.add_workbook("after save"));
    h.state_mut().autosave.advance(11 * MINUTE);
    wait_for_write(&mut h);
    assert!(recovery_file(&h).exists());
    let invalid_target = dir.join("missing-folder").join("cannot-save.sheaf");
    assert!(!h.state_mut().save_to(invalid_target));
    assert!(h.state().doc.dirty, "a failed save does not move the checkpoint");
    h.state_mut().doc.undo();
    assert!(!h.state().doc.dirty);
    settle(&mut h);
    assert!(!recovery_file(&h).exists());
    h.state_mut().doc.redo();
    assert!(h.state().doc.dirty);
    settle(&mut h);
    h.state_mut().autosave.advance(11 * MINUTE);
    wait_for_write(&mut h);
    assert!(recovery_file(&h).exists());
}

#[test]
fn undo_while_worker_is_blocked_schedules_cleanup_after_the_write() {
    let dir = scratch("undo-during-write");
    let mut h = start_in(vec![data_file("autosave-undo-pending")], &dir, None);
    let target = scratch("undo-pending-target").join("saved.sheaf");
    assert!(h.state_mut().save_to(target));
    h.state_mut().doc.apply("Edit", |p| p.add_workbook("after save"));
    let release = h.state_mut().autosave.pause_next_write();
    let project = h.state().doc.project().clone();
    let revision = h.state().doc.revision();
    h.state_mut().autosave.start_write(project, None, revision);
    h.state_mut().doc.undo();
    assert!(!h.state().doc.dirty);
    let ctx = egui::Context::default();
    // Drain egui's initial repaint requests, so this assertion checks our cleanup polling.
    for _ in 0..4 {
        let _ = ctx.run(egui::RawInput::default(), |_| {});
    }
    let idle = ctx.run(egui::RawInput::default(), |_| {});
    assert_eq!(idle.viewport_output[&egui::ViewportId::ROOT].repaint_delay, Duration::MAX);
    let output = ctx.run(egui::RawInput::default(), |ctx| h.state_mut().autosave_step(ctx));
    assert!(h.state().autosave.write_pending());
    assert!(output.viewport_output[&egui::ViewportId::ROOT].repaint_delay <= Duration::from_millis(100));
    release.send(()).unwrap();
    assert!(h.state_mut().autosave.finish_write().unwrap().is_ok());
    assert!(recovery_file(&h).exists(), "the old snapshot finished after undo");
    let _ = ctx.run(egui::RawInput::default(), |ctx| h.state_mut().autosave_step(ctx));
    assert!(!recovery_file(&h).exists(), "clean state removes the late recovery");
}

#[test]
fn undo_in_a_recovered_project_remains_dirty_until_saved() {
    let dir = scratch("recover-undo");
    planted(&dir, None);
    let mut h = start_in(Vec::new(), &dir, None);
    click(&mut h, "Recover");
    h.state_mut().doc.apply("Edit", |p| p.add_workbook("new"));
    h.state_mut().doc.undo();
    assert!(h.state().doc.dirty);
    settle(&mut h);
    assert!(recovery_file(&h).exists());
    let target = scratch("recover-undo-target").join("saved.sheaf");
    assert!(h.state_mut().save_to(target));
    settle(&mut h);
    assert!(!h.state().doc.dirty);
    assert!(!recovery_file(&h).exists());
}
