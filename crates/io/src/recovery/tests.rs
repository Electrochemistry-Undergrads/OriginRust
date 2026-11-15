use super::*;
use sheaf_core::column::{Column, ColumnData, Designation};
use sheaf_core::ids::{ColumnId, WindowId};
use sheaf_core::worksheet::{Workbook, Worksheet};
use std::ffi::OsString;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-io-recovery-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn project(values: &[f64]) -> Project {
    let mut sheet = Worksheet::new("Data");
    sheet.columns = vec![
        Column::new(ColumnId(0), "A", Designation::X, ColumnData::numeric(values.to_vec())),
        Column::new(ColumnId(0), "B", Designation::Y, ColumnData::numeric(values.iter().map(|v| v * 2.0).collect())),
    ];
    let mut book = Workbook::new(WindowId(0), "Book1");
    book.sheets = vec![sheet];
    let mut p = Project::new();
    p.adopt_named_workbook(book);
    p
}

fn set_mtime(path: &Path, when: SystemTime) {
    let f = OpenOptions::new().write(true).open(path).unwrap();
    f.set_modified(when).unwrap();
}

/// A recovery as a crashed instance leaves it: the session is dropped, which releases the lock but
/// keeps the files.
fn crashed(dir: &Path, p: &Project, original: Option<&Path>) -> String {
    let session = Session::start(dir).unwrap();
    session.save(p, original).unwrap();
    let id = session.id().to_string();
    drop(session);
    id
}

fn file_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> =
        std::fs::read_dir(dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
    let pairs: Vec<(String, String)> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    move |name| pairs.iter().find(|(k, _)| k == name).map(|(_, v)| OsString::from(v))
}

#[test]
fn the_folders_follow_each_platforms_convention() {
    let win = env(&[("LOCALAPPDATA", "C:/Users/a/AppData/Local"), ("APPDATA", "C:/Users/a/AppData/Roaming")]);
    assert_eq!(
        recovery_dir_for(Platform::Windows, &win),
        Some(PathBuf::from("C:/Users/a/AppData/Local").join("Sheaf").join("Recovery"))
    );
    assert_eq!(
        settings_path_for(Platform::Windows, &win),
        Some(PathBuf::from("C:/Users/a/AppData/Roaming").join("Sheaf").join("autosave.json"))
    );
    let mac = env(&[("HOME", "/Users/a")]);
    let support = PathBuf::from("/Users/a/Library/Application Support/Sheaf");
    assert_eq!(recovery_dir_for(Platform::Mac, &mac), Some(support.join("Recovery")));
    assert_eq!(settings_path_for(Platform::Mac, &mac), Some(support.join("autosave.json")));
    // XDG paths must pass the host platform's Path::is_absolute, even when simulating Linux.
    let root = std::env::current_dir().unwrap();
    let data = root.join("xdg-data");
    let config = root.join("xdg-config");
    let xdg = |name: &str| match name {
        "HOME" => Some(OsString::from("/home/a")),
        "XDG_DATA_HOME" => Some(data.as_os_str().to_os_string()),
        "XDG_CONFIG_HOME" => Some(config.as_os_str().to_os_string()),
        _ => None,
    };
    assert_eq!(recovery_dir_for(Platform::Other, &xdg), Some(data.join("sheaf").join("recovery")));
    assert_eq!(settings_path_for(Platform::Other, &xdg), Some(config.join("sheaf").join("autosave.json")));
}

#[test]
fn without_xdg_variables_the_home_folder_is_used_and_without_home_there_is_no_folder() {
    let home = env(&[("HOME", "/home/a"), ("XDG_DATA_HOME", "relative/dir"), ("XDG_CONFIG_HOME", "")]);
    assert_eq!(recovery_dir_for(Platform::Other, &home), Some(PathBuf::from("/home/a/.local/share/sheaf/recovery")));
    assert_eq!(settings_path_for(Platform::Other, &home), Some(PathBuf::from("/home/a/.config/sheaf/autosave.json")));
    let none = env(&[]);
    assert_eq!(recovery_dir_for(Platform::Other, &none), None);
    assert_eq!(recovery_dir_for(Platform::Mac, &none), None);
    assert_eq!(recovery_dir_for(Platform::Windows, &none), None);
    assert_eq!(settings_path_for(Platform::Windows, &env(&[("HOME", "/h")])), None);
}

#[test]
fn settings_default_to_on_every_ten_minutes() {
    let s = AutosaveSettings::default();
    assert!(s.enabled);
    assert_eq!(s.minutes, 10);
    assert_eq!(s.interval(), Duration::from_secs(600));
}

#[test]
fn settings_round_trip_and_create_the_folder() {
    let dir = scratch("settings");
    let path = dir.join("nested").join("autosave.json");
    let s = AutosaveSettings { enabled: false, minutes: 25 };
    s.save(&path).unwrap();
    assert_eq!(AutosaveSettings::load(&path), s);
    // No temporary file stays behind.
    assert_eq!(file_names(&dir.join("nested")), vec!["autosave.json".to_string()]);
}

#[test]
fn a_missing_or_corrupt_settings_file_gives_the_defaults() {
    let dir = scratch("settings-bad");
    assert_eq!(AutosaveSettings::load(&dir.join("none.json")), AutosaveSettings::default());
    let bad = dir.join("bad.json");
    std::fs::write(&bad, b"{ not json").unwrap();
    assert_eq!(AutosaveSettings::load(&bad), AutosaveSettings::default());
    std::fs::write(&bad, br#"{"enabled": "yes"}"#).unwrap();
    assert_eq!(AutosaveSettings::load(&bad), AutosaveSettings::default());
}

#[test]
fn a_settings_file_with_one_key_keeps_the_default_of_the_other() {
    let dir = scratch("settings-partial");
    let path = dir.join("a.json");
    std::fs::write(&path, br#"{"enabled": false}"#).unwrap();
    assert_eq!(AutosaveSettings::load(&path), AutosaveSettings { enabled: false, minutes: 10 });
    std::fs::write(&path, br#"{"minutes": 3}"#).unwrap();
    assert_eq!(AutosaveSettings::load(&path), AutosaveSettings { enabled: true, minutes: 3 });
}

#[test]
fn the_interval_is_kept_between_one_and_120_minutes() {
    let dir = scratch("settings-clamp");
    let path = dir.join("a.json");
    std::fs::write(&path, br#"{"enabled": true, "minutes": 0}"#).unwrap();
    assert_eq!(AutosaveSettings::load(&path).minutes, 1);
    std::fs::write(&path, br#"{"enabled": true, "minutes": 100000}"#).unwrap();
    assert_eq!(AutosaveSettings::load(&path).minutes, 120);
    AutosaveSettings { enabled: true, minutes: 500 }.save(&path).unwrap();
    assert_eq!(AutosaveSettings::load(&path).minutes, 120);
    assert_eq!(AutosaveSettings { enabled: true, minutes: 0 }.interval(), Duration::from_secs(60));
}

#[test]
fn a_session_writes_the_project_and_a_sidecar_that_load_reads_back() {
    let dir = scratch("roundtrip");
    let p = project(&[1.0, 2.0, 3.0]);
    let id = crashed(&dir, &p, Some(Path::new("/lab/run 7.opj")));
    let found = list_orphans(&dir);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, id);
    assert_eq!(found[0].project, dir.join(format!("{id}.sheaf")));
    let sidecar = found[0].sidecar.clone().unwrap();
    assert_eq!(sidecar.original_path(), Some(PathBuf::from("/lab/run 7.opj")));
    assert_eq!(sidecar.pid, std::process::id());
    assert_eq!(sidecar.sheaf_version, env!("CARGO_PKG_VERSION"));
    assert!(sidecar.saved_at > 1_700_000_000);
    assert_eq!(load(&found[0]).unwrap(), p);
}

#[test]
fn a_project_never_saved_has_no_original_in_the_sidecar() {
    let dir = scratch("untitled");
    crashed(&dir, &project(&[1.0]), None);
    let found = list_orphans(&dir);
    assert_eq!(found[0].sidecar.as_ref().unwrap().original, None);
}

#[test]
fn a_recovery_file_is_always_a_sheaf_project_never_an_opj() {
    let dir = scratch("native");
    let id = crashed(&dir, &project(&[1.0]), Some(Path::new("/lab/x.opj")));
    assert_eq!(file_names(&dir), vec![format!("{id}.json"), format!("{id}.lock"), format!("{id}.sheaf")]);
    assert!(project_file::load(&dir.join(format!("{id}.sheaf"))).is_ok());
}

#[test]
fn a_later_save_replaces_the_recovery_of_the_same_session() {
    let dir = scratch("resave");
    let session = Session::start(&dir).unwrap();
    session.save(&project(&[1.0]), None).unwrap();
    let second = project(&[1.0, 2.0, 3.0, 4.0]);
    session.save(&second, Some(Path::new("/x.sheaf"))).unwrap();
    drop(session);
    let found = list_orphans(&dir);
    assert_eq!(found.len(), 1);
    assert_eq!(load(&found[0]).unwrap(), second);
    assert_eq!(found[0].sidecar.as_ref().unwrap().original.as_deref(), Some("/x.sheaf"));
}

#[test]
fn two_live_sessions_in_one_folder_do_not_see_each_other() {
    let dir = scratch("two-live");
    let a = Session::start(&dir).unwrap();
    let b = Session::start(&dir).unwrap();
    assert_ne!(a.id(), b.id());
    a.save(&project(&[1.0]), None).unwrap();
    b.save(&project(&[2.0]), None).unwrap();
    assert!(list_orphans(&dir).is_empty());
    // Neither can be discarded or pruned while it lives.
    assert_eq!(prune(&dir, Duration::ZERO, SystemTime::now() + Duration::from_secs(3600)), 0);
    assert!(a.project_path().exists() && b.project_path().exists());
}

#[test]
fn an_abandoned_session_is_listed_and_the_live_one_is_not() {
    let dir = scratch("abandoned");
    let live = Session::start(&dir).unwrap();
    live.save(&project(&[1.0]), None).unwrap();
    let gone = crashed(&dir, &project(&[2.0]), None);
    let found = list_orphans(&dir);
    assert_eq!(found.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), vec![gone.as_str()]);
}

#[test]
fn dropping_a_session_keeps_its_recovery_files() {
    let dir = scratch("drop-keeps");
    let session = Session::start(&dir).unwrap();
    session.save(&project(&[1.0]), None).unwrap();
    let id = session.id().to_string();
    // A panic unwinds through the same drop.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _held = session;
        panic!("simulated crash");
    }));
    assert!(result.is_err());
    assert!(dir.join(format!("{id}.sheaf")).exists());
    assert!(dir.join(format!("{id}.json")).exists());
    assert_eq!(list_orphans(&dir).len(), 1);
}

#[test]
fn a_project_file_without_a_lock_file_is_listed() {
    let dir = scratch("no-lock");
    let p = project(&[5.0]);
    let id = crashed(&dir, &p, None);
    std::fs::remove_file(dir.join(format!("{id}.lock"))).unwrap();
    let found = list_orphans(&dir);
    assert_eq!(found.len(), 1);
    assert_eq!(load(&found[0]).unwrap(), p);
}

#[test]
fn a_corrupt_project_file_is_listed_and_load_fails_cleanly() {
    let dir = scratch("corrupt");
    std::fs::write(dir.join("1-2-3.sheaf"), b"this is not a zip archive").unwrap();
    let found = list_orphans(&dir);
    assert_eq!(found.len(), 1);
    assert!(found[0].sidecar.is_none());
    assert!(load(&found[0]).is_err());
    // It can still be thrown away.
    discard(&found[0]).unwrap();
    assert!(list_orphans(&dir).is_empty());
}

#[test]
fn a_corrupt_or_missing_sidecar_still_lists_the_recovery() {
    let dir = scratch("bad-sidecar");
    let p = project(&[1.0, 2.0]);
    let a = crashed(&dir, &p, Some(Path::new("/a.opj")));
    let b = crashed(&dir, &p, Some(Path::new("/b.opj")));
    std::fs::write(dir.join(format!("{a}.json")), b"{ broken").unwrap();
    std::fs::remove_file(dir.join(format!("{b}.json"))).unwrap();
    let found = list_orphans(&dir);
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|r| r.sidecar.is_none()));
    assert!(found.iter().all(|r| load(r).unwrap() == p));
}

#[test]
fn the_newest_recovery_comes_first() {
    let dir = scratch("order");
    let now = SystemTime::now();
    let old = crashed(&dir, &project(&[1.0]), None);
    let mid = crashed(&dir, &project(&[2.0]), None);
    let new = crashed(&dir, &project(&[3.0]), None);
    set_mtime(&dir.join(format!("{old}.sheaf")), now - Duration::from_secs(7200));
    set_mtime(&dir.join(format!("{mid}.sheaf")), now - Duration::from_secs(3600));
    set_mtime(&dir.join(format!("{new}.sheaf")), now);
    let ids: Vec<String> = list_orphans(&dir).into_iter().map(|r| r.id).collect();
    assert_eq!(ids, vec![new, mid, old]);
}

#[test]
fn temporary_files_and_unrelated_files_are_not_listed() {
    let dir = scratch("ignored");
    std::fs::write(dir.join(".1-2-3.sheaf.99.tmp"), b"half").unwrap();
    std::fs::write(dir.join(".hidden.sheaf"), b"x").unwrap();
    std::fs::write(dir.join("notes.txt"), b"x").unwrap();
    std::fs::write(dir.join("data.opj"), b"x").unwrap();
    std::fs::write(dir.join("a b.sheaf"), b"x").unwrap();
    std::fs::create_dir(dir.join("9-9-9.sheaf")).unwrap();
    std::fs::write(dir.join("1-2-3.lock"), b"").unwrap();
    assert!(list_orphans(&dir).is_empty());
    assert!(list_orphans(&dir.join("missing")).is_empty());
}

#[cfg(unix)]
#[test]
fn links_are_neither_listed_nor_followed() {
    let dir = scratch("links");
    let elsewhere = scratch("links-target");
    let target = elsewhere.join("precious.sheaf");
    project_file::save(&project(&[1.0]), &target).unwrap();
    std::os::unix::fs::symlink(&target, dir.join("4-5-6.sheaf")).unwrap();
    assert!(list_orphans(&dir).is_empty());
    // A link in the place of a sidecar or lock file is not removed through or followed either.
    let id = crashed(&dir, &project(&[2.0]), None);
    let other = elsewhere.join("other.json");
    std::fs::write(&other, b"keep").unwrap();
    std::fs::remove_file(dir.join(format!("{id}.json"))).unwrap();
    std::os::unix::fs::symlink(&other, dir.join(format!("{id}.json"))).unwrap();
    let found = list_orphans(&dir);
    assert_eq!(found.len(), 1);
    assert!(found[0].sidecar.is_none());
    discard(&found[0]).unwrap();
    assert!(target.exists());
    assert_eq!(std::fs::read(&other).unwrap(), b"keep");
}

#[test]
fn discard_removes_the_three_files_and_only_those() {
    let dir = scratch("discard");
    let keep = Session::start(&dir).unwrap();
    keep.save(&project(&[1.0]), None).unwrap();
    crashed(&dir, &project(&[2.0]), None);
    let found = list_orphans(&dir);
    discard(&found[0]).unwrap();
    assert!(list_orphans(&dir).is_empty());
    let mut expected =
        vec![format!("{}.json", keep.id()), format!("{}.lock", keep.id()), format!("{}.sheaf", keep.id())];
    expected.sort();
    assert_eq!(file_names(&dir), expected);
    // Discarding what is gone already is not an error.
    discard(&found[0]).unwrap();
}

#[test]
fn a_recovery_of_a_live_session_cannot_be_discarded() {
    let dir = scratch("discard-live");
    let live = Session::start(&dir).unwrap();
    let path = live.save(&project(&[1.0]), None).unwrap();
    let fake =
        Recovery { id: live.id().to_string(), project: path.clone(), sidecar: None, modified: SystemTime::now() };
    assert!(discard(&fake).is_err());
    assert!(path.exists());
}

#[test]
fn clear_removes_the_copy_and_the_session_can_save_again() {
    let dir = scratch("clear");
    let session = Session::start(&dir).unwrap();
    session.save(&project(&[1.0]), None).unwrap();
    session.clear().unwrap();
    assert_eq!(file_names(&dir), vec![format!("{}.lock", session.id())]);
    session.clear().unwrap();
    session.save(&project(&[2.0]), None).unwrap();
    assert!(session.project_path().exists());
}

#[test]
fn finish_removes_every_file_of_the_session() {
    let dir = scratch("finish");
    let session = Session::start(&dir).unwrap();
    session.save(&project(&[1.0]), None).unwrap();
    session.finish().unwrap();
    assert!(file_names(&dir).is_empty());
}

#[test]
fn finish_leaves_the_files_of_other_sessions() {
    let dir = scratch("finish-others");
    let other = crashed(&dir, &project(&[9.0]), None);
    let session = Session::start(&dir).unwrap();
    session.save(&project(&[1.0]), None).unwrap();
    session.finish().unwrap();
    assert_eq!(list_orphans(&dir)[0].id, other);
}

#[test]
fn a_session_can_save_from_a_worker_thread() {
    let dir = scratch("thread");
    let session = std::sync::Arc::new(Session::start(&dir).unwrap());
    let p = project(&[1.0, 2.0]);
    let worker = {
        let (session, p) = (session.clone(), p.clone());
        std::thread::spawn(move || session.save(&p, None))
    };
    worker.join().unwrap().unwrap();
    std::sync::Arc::into_inner(session).unwrap().finish().unwrap();
    assert!(file_names(&dir).is_empty());
}

#[test]
fn prune_drops_old_orphans_and_keeps_young_and_live_ones() {
    let dir = scratch("prune");
    let now = SystemTime::now();
    let day = Duration::from_secs(24 * 3600);
    let old = crashed(&dir, &project(&[1.0]), None);
    let young = crashed(&dir, &project(&[2.0]), None);
    let live = Session::start(&dir).unwrap();
    live.save(&project(&[3.0]), None).unwrap();
    set_mtime(&dir.join(format!("{old}.sheaf")), now - 31 * day);
    set_mtime(&dir.join(format!("{young}.sheaf")), now - 29 * day);
    set_mtime(&live.project_path(), now - 40 * day);
    assert_eq!(prune(&dir, MAX_AGE, now), 1);
    let ids: Vec<String> = list_orphans(&dir).into_iter().map(|r| r.id).collect();
    assert_eq!(ids, vec![young]);
    assert!(live.project_path().exists());
    assert!(!dir.join(format!("{old}.json")).exists());
    assert!(!dir.join(format!("{old}.lock")).exists());
}

#[test]
fn prune_removes_old_leftovers_but_not_young_ones_or_a_live_lock() {
    let dir = scratch("prune-leftovers");
    let now = SystemTime::now();
    let day = Duration::from_secs(24 * 3600);
    let live = Session::start(&dir).unwrap();
    let stale_lock = dir.join("7-7-7.lock");
    let stale_json = dir.join("8-8-8.json");
    let stale_tmp = dir.join(".9-9-9.sheaf.1.tmp");
    let young_lock = dir.join("6-6-6.lock");
    let unrelated = dir.join("notes.txt");
    for f in [&stale_lock, &stale_json, &stale_tmp, &young_lock, &unrelated] {
        std::fs::write(f, b"x").unwrap();
    }
    for f in [&stale_lock, &stale_json, &stale_tmp, &unrelated, &live.lock_path()] {
        set_mtime(f, now - 60 * day);
    }
    assert_eq!(prune(&dir, MAX_AGE, now), 0);
    assert!(!stale_lock.exists() && !stale_json.exists() && !stale_tmp.exists());
    assert!(young_lock.exists() && unrelated.exists() && live.lock_path().exists());
}

#[test]
fn files_that_are_not_ours_are_never_listed_or_pruned() {
    let dir = scratch("not-ours");
    let now = SystemTime::now();
    let day = Duration::from_secs(24 * 3600);
    let names = [
        "notes.sheaf",
        "thesis.json",
        "config.json",
        "package.lock",
        ".x.tmp",
        ".notes.sheaf.1.tmp",
        "1-2.sheaf",
        "1-2-3-4.json",
        "a-b-c.sheaf",
    ];
    for name in names {
        std::fs::write(dir.join(name), b"x").unwrap();
        set_mtime(&dir.join(name), now - 60 * day);
    }
    assert!(list_orphans(&dir).is_empty());
    assert_eq!(prune(&dir, MAX_AGE, now), 0);
    for name in names {
        assert!(dir.join(name).exists(), "{name} is the user's and must stay");
    }
}

#[test]
fn prune_in_a_missing_folder_does_nothing() {
    assert_eq!(prune(&scratch("prune-none").join("missing"), MAX_AGE, SystemTime::now()), 0);
}

#[test]
fn start_creates_the_folder_and_ids_are_unique_within_a_process() {
    let dir = scratch("start").join("a").join("b");
    let first = Session::start(&dir).unwrap();
    let second = Session::start(&dir).unwrap();
    assert!(dir.is_dir());
    assert_ne!(first.id(), second.id());
    assert!(first.id().starts_with(&format!("{}-", std::process::id())));
    assert!(first.lock_path().exists());
}

#[test]
fn the_run_folder_candidates_come_in_order_without_duplicates() {
    let root = std::env::current_dir().unwrap();
    let forced = root.join("forced");
    let runtime = root.join("runtime");
    let all = |name: &str| match name {
        "SHEAF_MCP_RUN_DIR" => Some(forced.as_os_str().to_os_string()),
        "XDG_RUNTIME_DIR" => Some(runtime.as_os_str().to_os_string()),
        "HOME" => Some(OsString::from("/home/a")),
        _ => None,
    };
    assert_eq!(
        run_dir_candidates_for(Platform::Other, &all),
        vec![forced.clone(), runtime.join("sheaf"), PathBuf::from("/home/a/.local/share/sheaf/run")]
    );
    // Only Linux has a runtime folder; the variable is ignored elsewhere.
    assert_eq!(
        run_dir_candidates_for(Platform::Mac, &all),
        vec![forced.clone(), PathBuf::from("/home/a/Library/Application Support/Sheaf/run")]
    );
    let win = env(&[("LOCALAPPDATA", "C:/Users/a/AppData/Local")]);
    assert_eq!(
        run_dir_candidates_for(Platform::Windows, &win),
        vec![PathBuf::from("C:/Users/a/AppData/Local").join("Sheaf").join("run")]
    );
    // A relative override or runtime folder is not used.
    let relative = env(&[("SHEAF_MCP_RUN_DIR", "run"), ("XDG_RUNTIME_DIR", "rt"), ("HOME", "/home/a")]);
    assert_eq!(
        run_dir_candidates_for(Platform::Other, &relative),
        vec![PathBuf::from("/home/a/.local/share/sheaf/run")]
    );
    assert!(run_dir_candidates_for(Platform::Other, &env(&[])).is_empty());
}
