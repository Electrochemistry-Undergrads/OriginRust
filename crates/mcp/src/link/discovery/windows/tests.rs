use super::*;
use crate::link::discovery::{self, Info};
use crate::link::transport::Endpoint;
use std::time::Duration;

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-win-private-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn public_descriptor(directory: bool) -> Local {
    let inherit = if directory { "OICI" } else { "" };
    from_sddl(&format!("D:P(A;{inherit};FA;;;WD)")).unwrap()
}

fn info() -> Info {
    Info {
        pid: 42,
        app: "test".into(),
        endpoint: Endpoint::Tcp { addr: "127.0.0.1:12345".parse().unwrap(), token: "synthetic".into() },
    }
}

#[test]
fn windows_runtime_privacy_token_file_is_private_from_creation_under_a_broad_parent() {
    let dir = scratch("create");
    set_dacl(&dir, &public_descriptor(true)).unwrap();
    let path = dir.join("new-token");
    let file = create_private(&path).unwrap();
    check(&path).unwrap();
    assert_eq!(file.metadata().unwrap().len(), 0, "private before token bytes are written");
    assert_eq!(create_private(&path).unwrap_err().kind(), io::ErrorKind::AlreadyExists);
    drop(file);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn windows_runtime_privacy_removes_broad_grants_only_from_the_runtime_objects() {
    let parent = scratch("inherit");
    set_dacl(&parent, &public_descriptor(true)).unwrap();
    let parent_before = security(&parent).unwrap();
    let run = parent.join("run");
    std::fs::create_dir_all(&run).unwrap();
    assert!(check(&run).is_err(), "the inherited Everyone grant must be refused");
    discovery::check_dir(&run, true).unwrap();
    check(&run).unwrap();
    discovery::write_info(&run, &info()).unwrap();
    check(&run.join(discovery::INFO_NAME)).unwrap();
    assert_eq!(discovery::read_pid(&run), Some(42));
    // The broad parent must still be broad: only the selected runtime folder is hardened.
    let user = current_user().unwrap();
    assert!(validate(&parent_before, user_sid(&user)).is_err());
    assert!(check(&parent).is_err());
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn windows_runtime_privacy_client_refuses_a_broad_token_without_repairing_it() {
    let run = scratch("token");
    discovery::check_dir(&run, true).unwrap();
    let lock = discovery::acquire_lock(&run).unwrap();
    discovery::write_info(&run, &info()).unwrap();
    let path = run.join(discovery::INFO_NAME);
    set_dacl(&path, &public_descriptor(false)).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert!(discovery::probe(&run, Duration::ZERO).unwrap_err().contains("mcp.json is not private"));
    assert!(check(&path).is_err(), "the client's check must leave rights unchanged");
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    drop(lock);
    std::fs::remove_dir_all(run).unwrap();
}

#[test]
fn windows_runtime_privacy_client_refuses_a_broad_folder_without_repairing_it() {
    let run = scratch("folder");
    set_dacl(&run, &public_descriptor(true)).unwrap();
    assert!(discovery::check_dir(&run, false).unwrap_err().contains("not private"));
    assert!(check(&run).is_err());
    discovery::check_dir(&run, true).unwrap();
    discovery::check_dir(&run, false).unwrap();
    std::fs::remove_dir_all(run).unwrap();
}

#[test]
fn windows_runtime_privacy_rejects_another_owner_even_with_a_private_dacl() {
    let user = current_user().unwrap();
    // A valid OS descriptor with a different owner, without changing any real owner.
    let other = from_sddl("O:WDD:P(A;;FA;;;SY)(A;;FA;;;BA)").unwrap();
    assert!(validate(&other, user_sid(&user)).unwrap_err().to_string().contains("another user"));
}
