use super::*;
use crate::link::transport::SOCKET_NAME;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sl-d-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    #[cfg(windows)]
    check_dir(&dir, true).unwrap();
    dir
}

fn info(dir: &Path) -> Info {
    Info { pid: 4242, app: "0.1".into(), endpoint: Endpoint::Unix(dir.join(SOCKET_NAME)) }
}

#[test]
fn mcp_json_round_trips_and_an_old_version_is_refused() {
    let dir = PathBuf::from("/run/sheaf");
    let i = info(&dir);
    assert_eq!(Info::from_json(&i.to_json()), Ok(i.clone()));
    let mut bad = i.to_json();
    bad["v"] = json!(2);
    assert!(Info::from_json(&bad).is_err());
    assert!(Info::from_json(&json!({"v": 1})).is_err());
}

#[test]
fn the_message_for_no_listener_names_each_folder_and_what_to_do() {
    let text = no_listener(&[(PathBuf::from("/a"), "no lock".into()), (PathBuf::from("/b"), "does not exist".into())]);
    assert!(text.contains("looked in: /a: no lock; /b: does not exist"), "{text}");
    assert!(text.contains("Tools > AI Assistants (MCP)") && text.contains("--mcp"), "{text}");
    assert!(no_listener(&[]).contains("no folder to look in"));
}

#[test]
fn a_missing_folder_is_not_made_by_the_client_and_is_made_by_the_app() {
    let base = scratch("make");
    let dir = base.join("a").join("b");
    assert_eq!(check_dir(&dir, false), Err("does not exist".to_string()));
    assert!(!dir.exists());
    check_dir(&dir, true).unwrap();
    assert!(dir.is_dir());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for d in [&dir, &base.join("a")] {
            assert_eq!(std::fs::metadata(d).unwrap().permissions().mode() & 0o777, 0o700, "{}", d.display());
        }
    }
    let _ = std::fs::remove_dir_all(&base);
}

#[cfg(unix)]
#[test]
fn an_open_folder_of_the_user_is_closed_and_one_of_another_user_is_refused() {
    use std::os::unix::fs::PermissionsExt;
    let base = scratch("mode");
    let dir = base.join("run");
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    check_dir(&dir, false).unwrap();
    assert_eq!(std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777, 0o700);

    // The same folder seen as belonging to someone else: refused, and its rights are not touched.
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    let real = own_uid(&dir).unwrap();
    let other = |_: &Path| Ok(real.wrapping_add(1));
    assert_eq!(check_dir_with(&dir, false, &other), Err("belongs to another user".to_string()));
    assert_eq!(std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777, 0o755);
    let _ = std::fs::remove_dir_all(&base);
}

#[cfg(unix)]
#[test]
fn a_link_or_a_plain_file_is_not_a_run_folder() {
    let base = scratch("link");
    let real = base.join("real");
    std::fs::create_dir(&real).unwrap();
    let link = base.join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    assert_eq!(check_dir(&link, false), Err("is a link, not a folder".to_string()));
    assert_eq!(check_dir(&link, true), Err("is a link, not a folder".to_string()));
    let file = base.join("file");
    std::fs::write(&file, b"x").unwrap();
    assert_eq!(check_dir(&file, true), Err("is not a folder".to_string()));
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn only_one_process_takes_the_lock_and_a_probe_sees_it() {
    let dir = scratch("lock");
    check_dir(&dir, false).unwrap();
    assert_eq!(probe(&dir, Duration::ZERO), Err("no lock".to_string()), "no lock file yet");
    let lock = acquire_lock(&dir).unwrap();
    assert!(matches!(acquire_lock(&dir), Err(LockError::Busy)), "a second holder is refused");
    assert_eq!(probe(&dir, Duration::ZERO), Err("mcp.json is missing".to_string()));
    write_info(&dir, &info(&dir)).unwrap();
    assert_eq!(probe(&dir, Duration::ZERO), Ok(info(&dir)));
    drop(lock);
    // The lock is gone and the old mcp.json says nothing any more.
    assert_eq!(probe(&dir, Duration::ZERO), Err("no lock".to_string()));
    assert!(acquire_lock(&dir).is_ok());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_socket_outside_the_folder_is_not_followed() {
    let dir = scratch("far");
    let _lock = acquire_lock(&dir).unwrap();
    let far = Info { pid: 1, app: String::new(), endpoint: Endpoint::Unix(PathBuf::from("/tmp/elsewhere/mcp.sock")) };
    write_info(&dir, &far).unwrap();
    assert!(probe(&dir, Duration::ZERO).unwrap_err().contains("outside the folder"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mcp_json_is_private_and_the_cleanup_leaves_folders_alone() {
    let dir = scratch("clean");
    write_info(&dir, &info(&dir)).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(dir.join(INFO_NAME)).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    assert_eq!(read_pid(&dir), Some(4242));
    std::fs::write(dir.join(SOCKET_NAME), b"stale").unwrap();
    remove_endpoint_files(&dir);
    assert!(!dir.join(INFO_NAME).exists() && !dir.join(SOCKET_NAME).exists());
    std::fs::create_dir(dir.join(SOCKET_NAME)).unwrap();
    remove_endpoint_files(&dir);
    assert!(dir.join(SOCKET_NAME).is_dir(), "a folder at that name is not removed");
    let _ = std::fs::remove_dir_all(&dir);
}
