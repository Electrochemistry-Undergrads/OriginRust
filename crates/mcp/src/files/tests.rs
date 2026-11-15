use super::*;
use std::sync::atomic::{AtomicU32, Ordering};

struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Dir {
        static N: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!(
            "sheaf-mcp-files-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Dir(p.canonicalize().unwrap())
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn names(&self) -> Vec<String> {
        let mut v: Vec<String> =
            fs::read_dir(&self.0).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        v.sort();
        v
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn none() -> Protected {
    Protected::default()
}

fn message(r: Result<()>) -> String {
    format!("{:#}", r.unwrap_err())
}

#[test]
fn a_new_file_is_written_whole_and_no_temporary_file_is_left() {
    let d = Dir::new("new");
    write_new_bytes(&d.file("a.csv"), b"x,y\n1,2\n", &none()).unwrap();
    assert_eq!(fs::read(d.file("a.csv")).unwrap(), b"x,y\n1,2\n");
    assert_eq!(d.names(), ["a.csv"]);
}

#[test]
fn an_existing_file_is_never_replaced() {
    let d = Dir::new("exists");
    fs::write(d.file("a.csv"), "old").unwrap();
    let e = message(write_new_bytes(&d.file("a.csv"), b"new", &none()));
    assert!(e.starts_with("will not overwrite: '"), "{e}");
    assert!(e.contains("choose a new file name"), "{e}");
    assert_eq!(fs::read_to_string(d.file("a.csv")).unwrap(), "old");
    assert_eq!(d.names(), ["a.csv"]);
}

#[test]
fn a_folder_is_not_replaced_either() {
    let d = Dir::new("folder");
    fs::create_dir(d.file("out.png")).unwrap();
    assert!(message(write_new_bytes(&d.file("out.png"), b"x", &none())).contains("will not overwrite"));
    assert_eq!(d.names(), ["out.png"]);
}

#[cfg(unix)]
#[test]
fn links_are_never_followed_and_count_as_existing() {
    let d = Dir::new("links");
    let elsewhere = Dir::new("elsewhere");
    fs::write(elsewhere.file("target.txt"), "keep").unwrap();
    std::os::unix::fs::symlink(elsewhere.file("target.txt"), d.file("to_file.txt")).unwrap();
    std::os::unix::fs::symlink(elsewhere.file("nothing.txt"), d.file("dangling.txt")).unwrap();
    std::os::unix::fs::symlink(&elsewhere.0, d.file("to_folder")).unwrap();
    for name in ["to_file.txt", "dangling.txt", "to_folder"] {
        let e = message(write_new_bytes(&d.file(name), b"changed", &none()));
        assert!(e.contains("will not overwrite"), "{name}: {e}");
    }
    assert_eq!(fs::read_to_string(elsewhere.file("target.txt")).unwrap(), "keep");
    assert!(!elsewhere.file("nothing.txt").exists());
    assert_eq!(elsewhere.names(), ["target.txt"]);
}

#[cfg(unix)]
#[test]
fn a_named_pipe_is_not_written_to() {
    let d = Dir::new("fifo");
    let made = std::process::Command::new("mkfifo").arg(d.file("pipe")).status();
    if !made.is_ok_and(|s| s.success()) {
        return;
    }
    assert!(message(write_new_bytes(&d.file("pipe"), b"x", &none())).contains("will not overwrite"));
    assert_eq!(d.names(), ["pipe"]);
}

#[test]
fn a_missing_folder_and_a_relative_path_are_refused() {
    let d = Dir::new("missing");
    let e = message(write_new_bytes(&d.file("nope").join("a.csv"), b"x", &none()));
    assert!(e.contains("the folder does not exist"), "{e}");
    assert!(message(write_new_bytes(Path::new("a.csv"), b"x", &none())).contains("not an absolute path"));
    assert!(d.names().is_empty());
}

#[test]
fn a_failing_writer_leaves_no_file_and_names_the_real_path() {
    let d = Dir::new("fail");
    let target = d.file("fig.png");
    let r: Result<()> = write_new_with(&target, &none(), |tmp| {
        fs::write(tmp, "partial")?;
        bail!("could not write '{}': the disk is full", tmp.display())
    });
    let e = format!("{:#}", r.unwrap_err());
    assert!(e.contains(&target.display().to_string()), "{e}");
    assert!(!e.contains(".tmp"), "{e}");
    assert!(d.names().is_empty());
}

#[test]
fn a_path_writer_gets_a_temporary_name_with_the_extension_and_the_file_appears_whole() {
    let d = Dir::new("with");
    let target = d.file("book.xlsx");
    let seen = write_new_with(&target, &none(), |tmp| {
        assert_eq!(tmp.extension().and_then(|e| e.to_str()), Some("xlsx"));
        assert_eq!(tmp.parent(), target.parent());
        assert!(!target.exists());
        fs::write(tmp, "content")?;
        Ok(tmp.file_name().unwrap().to_string_lossy().into_owned())
    })
    .unwrap();
    assert!(seen.starts_with('.'), "{seen}");
    assert_eq!(fs::read_to_string(&target).unwrap(), "content");
    assert_eq!(d.names(), ["book.xlsx"]);
}

#[test]
fn a_file_that_appears_while_the_writer_runs_is_not_replaced() {
    let d = Dir::new("race");
    let target = d.file("out.csv");
    let r = write_new_with(&target, &none(), |tmp| {
        fs::write(tmp, "mine")?;
        fs::write(&target, "theirs")?;
        Ok(())
    });
    assert!(format!("{:#}", r.unwrap_err()).contains("will not overwrite"));
    assert_eq!(fs::read_to_string(&target).unwrap(), "theirs");
    assert_eq!(d.names(), ["out.csv"]);
}

#[test]
fn without_hard_links_the_file_is_renamed_into_place_unless_the_name_is_taken() {
    let d = Dir::new("rename");
    let unsupported = |_: &Path, _: &Path| Err(std::io::Error::from(ErrorKind::Unsupported));
    write_new_bytes_with(&d.file("a.csv"), b"one", &none(), unsupported).unwrap();
    assert_eq!(fs::read_to_string(d.file("a.csv")).unwrap(), "one");
    assert_eq!(d.names(), ["a.csv"]);
    // The name exists: the fall-back looks again and refuses.
    let e = message(write_new_bytes_with(&d.file("a.csv"), b"two", &none(), unsupported));
    assert!(e.contains("will not overwrite"), "{e}");
    assert_eq!(fs::read_to_string(d.file("a.csv")).unwrap(), "one");
    assert_eq!(d.names(), ["a.csv"]);
}

fn protected_in(d: &Dir) -> Protected {
    let open = OpenFiles {
        project: Some(d.file("work.opj")),
        origin: Some(d.file("origin.opju")),
        recovered_from: Some(d.file("lost.sheaf")),
    };
    protected_set(&open, &[d.file("recovery"), d.file("run")])
}

#[test]
fn every_name_of_the_open_project_is_protected() {
    let d = Dir::new("names");
    let p = protected_in(&d);
    let pid = std::process::id();
    for name in [
        "work.opj".to_string(),
        "work.opj.sheaf".to_string(),
        "work.opj.bak".to_string(),
        format!(".work.opj.{pid}.tmp"),
        format!(".work.opj.sheaf.{pid}.tmp"),
        "origin.opju".to_string(),
        "origin.opju.sheaf".to_string(),
        "origin.opju.bak".to_string(),
        "lost.sheaf".to_string(),
        "lost.sheaf.bak".to_string(),
    ] {
        let e = message(p.check(&d.file(&name)));
        assert!(e.starts_with("that name belongs to the project open in Sheaf: "), "{name}: {e}");
    }
    for name in ["data.csv", "work.png", "work.opj.csv", "work2.csv"] {
        assert!(p.check(&d.file(name)).is_ok(), "{name}");
    }
}

#[test]
fn project_backup_and_lock_names_are_refused_even_for_other_files() {
    let d = Dir::new("suffix");
    let p = protected_in(&d);
    for name in ["other.opj", "other.OPJU", "x.sheaf", "planted.bak", "mcp.lock", "a.csv.BAK"] {
        let e = message(p.check(&d.file(name)));
        assert!(e.contains("kept for project, backup and lock files"), "{name}: {e}");
    }
}

#[test]
fn a_planted_backup_of_an_origin_project_is_refused() {
    let d = Dir::new("planted");
    let p = protected_in(&d);
    let e = write_new_bytes(&d.file("origin.opju.bak"), b"fake", &p).unwrap_err();
    assert!(format!("{e:#}").contains("belongs to the project"), "{e:#}");
    assert!(d.names().is_empty());
}

#[test]
fn files_in_the_recovery_and_run_folders_are_refused() {
    let d = Dir::new("folders");
    fs::create_dir_all(d.file("recovery").join("session")).unwrap();
    fs::create_dir_all(d.file("run")).unwrap();
    let p = protected_in(&d);
    for path in [
        d.file("recovery").join("session").join("autosave.txt"),
        d.file("recovery").join("x.csv"),
        d.file("run").join("note.txt"),
    ] {
        let e = message(p.check(&path));
        assert!(e.contains("kept for Sheaf's own recovery and link files"), "{e}");
    }
    assert!(p.check(&d.file("recovery2").join("x.csv")).is_ok());
    assert!(p.check(&d.file("x.csv")).is_ok());
}

#[cfg(unix)]
#[test]
fn a_link_to_the_folder_does_not_hide_a_protected_name() {
    let d = Dir::new("viafolder");
    let real = d.file("real");
    fs::create_dir(&real).unwrap();
    std::os::unix::fs::symlink(&real, d.file("alias")).unwrap();
    let open = OpenFiles { project: Some(real.join("work.opj")), ..OpenFiles::default() };
    let p = protected_set(&open, &[]);
    assert!(p.check(&d.file("alias").join("work.opj.bak")).is_err());
    assert!(p.check(&d.file("alias").join("work.opj.sheaf")).is_err());
    assert!(p.check(&d.file("alias").join("work.csv")).is_ok());
    // The project path itself may be given through the link.
    let open = OpenFiles { project: Some(d.file("alias").join("work.opj")), ..OpenFiles::default() };
    assert!(protected_set(&open, &[]).check(&real.join("work.opj.bak")).is_err());
}

#[cfg(any(target_os = "macos", windows))]
#[test]
fn names_differing_only_in_case_are_the_same_name_here() {
    let d = Dir::new("case");
    let open = OpenFiles { project: Some(d.file("Work.opj")), ..OpenFiles::default() };
    let p = protected_set(&open, &[d.file("Recovery")]);
    assert!(message(p.check(&d.file("WORK.OPJ"))).contains("belongs to the project"));
    assert!(message(p.check(&d.file("work.OPJ.Bak"))).contains("belongs to the project"));
    assert!(p.check(&d.file("recovery").join("x.csv")).is_err());
}

#[test]
fn protected_names_are_refused_before_anything_is_written() {
    let d = Dir::new("before");
    let open = OpenFiles { project: Some(d.file("work.opj")), ..OpenFiles::default() };
    let p = protected_set(&open, &[]);
    assert!(write_new_with(&d.file("work.opj.sheaf"), &p, |_| Ok(())).is_err());
    assert!(check_new(&d.file("work.opj.bak"), &p).is_err());
    assert!(d.names().is_empty());
}

#[test]
fn names_that_windows_would_read_as_a_protected_name_are_refused_everywhere() {
    let d = Dir::new("winnames");
    let open = OpenFiles { project: None, origin: Some(d.file("x.opj")), recovered_from: None };
    let protected = protected_set(&open, &[]);
    for name in ["x.opj.bak.", "x.opj.bak ", "x.opj.bak::$DATA", "x.opj.bak:s", "x.opj.sheaf...", "plain.csv."] {
        let e = message(check_new(&d.file(name), &protected));
        assert!(e.contains("may not end in a dot or a space or contain ':'"), "{name}: {e}");
        assert!(write_new_bytes(&d.file(name), b"x", &protected).is_err(), "{name}");
    }
    assert!(d.names().is_empty(), "nothing was created: {:?}", d.names());
    assert!(check_new(&d.file("a.b.csv"), &protected).is_ok());
}
