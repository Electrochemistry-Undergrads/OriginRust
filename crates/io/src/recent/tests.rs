use super::*;
use std::sync::{Arc, Barrier};

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-io-recent-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    strip_verbatim(std::fs::canonicalize(&dir).unwrap())
}

fn touch(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, b"x").unwrap();
    p
}

fn names(list: &[PathBuf]) -> Vec<String> {
    list.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect()
}

#[test]
fn the_list_is_next_to_the_autosave_settings() {
    if let Some(p) = default_path() {
        assert_eq!(p.file_name().unwrap(), FILE_NAME);
        assert_eq!(p.parent(), recovery::default_settings_path().unwrap().parent());
    }
}

#[test]
fn a_new_list_is_empty() {
    let list = RecentList::default();
    assert!(list.projects.is_empty() && list.imports.is_empty());
}

#[test]
fn the_newest_project_comes_first() {
    let dir = scratch("order");
    let (a, b, c) = (touch(&dir, "a.sheaf"), touch(&dir, "b.sheaf"), touch(&dir, "c.sheaf"));
    let mut list = RecentList::default();
    for p in [&a, &b, &c] {
        list.push_project(p);
    }
    assert_eq!(names(&list.projects), ["c.sheaf", "b.sheaf", "a.sheaf"]);
    assert!(list.imports.is_empty());
}

#[test]
fn opening_a_project_again_moves_it_to_the_top_without_a_repeat() {
    let dir = scratch("repeat");
    let (a, b, c) = (touch(&dir, "a.sheaf"), touch(&dir, "b.sheaf"), touch(&dir, "c.sheaf"));
    let mut list = RecentList::default();
    for p in [&a, &b, &c, &a] {
        list.push_project(p);
    }
    assert_eq!(names(&list.projects), ["a.sheaf", "c.sheaf", "b.sheaf"]);
}

#[test]
fn each_list_keeps_fifteen_entries_and_drops_the_oldest() {
    let dir = scratch("cap");
    let mut list = RecentList::default();
    for i in 0..18 {
        list.push_project(&touch(&dir, &format!("p{i}.sheaf")));
        list.push_import(&[touch(&dir, &format!("d{i}.csv"))]);
    }
    assert_eq!(list.projects.len(), MAX_ENTRIES);
    assert_eq!(list.imports.len(), MAX_ENTRIES);
    assert_eq!(MAX_ENTRIES, 15, "Origin 2016 shows 15 recent projects");
    assert_eq!(names(&list.projects)[0], "p17.sheaf");
    assert_eq!(names(&list.projects)[14], "p3.sheaf");
    assert_eq!(names(&list.imports)[14], "d3.csv");
}

#[test]
fn a_batch_import_lists_the_last_file_first() {
    let dir = scratch("batch");
    let (a, b, c) = (touch(&dir, "a.csv"), touch(&dir, "b.csv"), touch(&dir, "c.csv"));
    let mut list = RecentList::default();
    list.push_import(&[&a, &b, &c, &a]);
    assert_eq!(names(&list.imports), ["a.csv", "c.csv", "b.csv"]);
}

#[test]
fn a_file_can_be_in_both_lists() {
    let dir = scratch("both");
    let p = touch(&dir, "x.sheaf");
    let mut list = RecentList::default();
    list.push_project(&p);
    list.push_import(&[&p]);
    assert_eq!(list.projects, list.imports);
}

#[test]
fn spellings_of_one_file_count_once_and_are_stored_canonical() {
    let dir = scratch("canon");
    let sub = dir.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    let p = touch(&dir, "a.sheaf");
    let mut list = RecentList::default();
    list.push_project(&p);
    list.push_project(&sub.join("..").join("a.sheaf"));
    list.push_project(&dir.join(".").join("a.sheaf"));
    assert_eq!(list.projects, vec![p]);
}

#[cfg(unix)]
#[test]
fn a_link_and_its_target_count_once() {
    let dir = scratch("link");
    let p = touch(&dir, "real.sheaf");
    let link = dir.join("alias.sheaf");
    std::os::unix::fs::symlink(&p, &link).unwrap();
    let mut list = RecentList::default();
    list.push_project(&link);
    list.push_project(&p);
    assert_eq!(list.projects, vec![p]);
}

#[test]
fn a_relative_path_is_stored_absolute() {
    let mut list = RecentList::default();
    list.push_project(Path::new("no-such-folder-t38/x.sheaf"));
    assert!(list.projects[0].is_absolute());
    assert!(list.projects[0].ends_with("no-such-folder-t38/x.sheaf"));
}

#[test]
fn a_missing_file_keeps_its_name_and_gets_its_folder_canonical() {
    let dir = scratch("missing");
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    let p = normalize(&dir.join("sub").join("..").join("gone.sheaf"));
    assert_eq!(p, dir.join("gone.sheaf"));
}

#[test]
fn letter_case_decides_only_where_the_system_cares() {
    assert_eq!(key(Path::new("/A/Book.sheaf"), true), key(Path::new("/a/book.SHEAF"), true));
    assert_ne!(key(Path::new("/A/Book.sheaf"), false), key(Path::new("/a/book.SHEAF"), false));
}

#[cfg(any(windows, target_os = "macos"))]
#[test]
fn spellings_that_differ_in_case_count_once_on_this_system() {
    let dir = scratch("case");
    let p = touch(&dir, "Book.sheaf");
    let mut list = RecentList::default();
    list.push_project(&p);
    list.push_project(&dir.join("BOOK.SHEAF"));
    assert_eq!(list.projects.len(), 1);
}

#[cfg(not(any(windows, target_os = "macos")))]
#[test]
fn spellings_that_differ_in_case_are_two_files_on_this_system() {
    let dir = scratch("case");
    let mut list = RecentList::default();
    list.push_project(&touch(&dir, "Book.sheaf"));
    list.push_project(&touch(&dir, "BOOK.sheaf"));
    assert_eq!(list.projects.len(), 2);
}

#[test]
fn the_verbatim_prefix_of_windows_paths_is_removed() {
    assert_eq!(strip_verbatim_text(r"\\?\C:\Data\a.sheaf").as_deref(), Some(r"C:\Data\a.sheaf"));
    assert_eq!(strip_verbatim_text(r"\\?\UNC\srv\share\a.sheaf").as_deref(), Some(r"\\srv\share\a.sheaf"));
    assert_eq!(strip_verbatim_text(r"C:\Data\a.sheaf"), None);
    assert_eq!(strip_verbatim_text(r"\\?\Volume{1234}\a"), None);
    assert_eq!(strip_verbatim_text("/home/a"), None);
}

#[test]
fn the_lists_survive_a_round_trip_through_the_file() {
    let dir = scratch("round");
    let file = dir.join("settings").join(FILE_NAME);
    let mut list = RecentList::default();
    list.push_project(&touch(&dir, "a.sheaf"));
    list.push_project(&touch(&dir, "b.sheaf"));
    list.push_import(&[touch(&dir, "d.csv")]);
    list.save(&file).unwrap();
    assert_eq!(RecentList::load(&file), list);
}

#[test]
fn a_missing_file_gives_empty_lists() {
    let dir = scratch("none");
    assert_eq!(RecentList::load(&dir.join("nothing.json")), RecentList::default());
}

#[test]
fn a_damaged_file_gives_empty_lists_and_is_replaced_by_the_next_change() {
    let dir = scratch("damaged");
    let file = dir.join(FILE_NAME);
    for text in ["", "not json", "[1, 2]", "null", "{\"projects\": 5, \"imports\": \"x\"}", "\u{0}\u{1}"] {
        std::fs::write(&file, text).unwrap();
        assert_eq!(RecentList::load(&file), RecentList::default(), "{text:?}");
    }
    std::fs::write(&file, "not json").unwrap();
    let p = touch(&dir, "a.sheaf");
    let list = record_project(&file, &p).unwrap();
    assert_eq!(list.projects, vec![p]);
    assert_eq!(RecentList::load(&file), list);
}

#[test]
fn a_bad_entry_is_skipped_and_the_rest_is_kept() {
    let dir = scratch("partial");
    let file = dir.join(FILE_NAME);
    let a = touch(&dir, "a.sheaf");
    let b = touch(&dir, "b.sheaf");
    let json = serde_json::json!({
        "projects": [a.to_str().unwrap(), 7, null, {"x": 1}, "relative/c.sheaf", "", b.to_str().unwrap(), a.to_str().unwrap()],
        "imports": "broken",
    });
    std::fs::write(&file, json.to_string()).unwrap();
    let list = RecentList::load(&file);
    assert_eq!(list.projects, vec![a, b]);
    assert!(list.imports.is_empty());
}

#[test]
fn a_hand_edited_long_list_is_cut_on_load() {
    let dir = scratch("long");
    let file = dir.join(FILE_NAME);
    let all: Vec<String> = (0..25).map(|i| dir.join(format!("p{i}.sheaf")).to_str().unwrap().to_string()).collect();
    std::fs::write(&file, serde_json::json!({ "projects": all }).to_string()).unwrap();
    let list = RecentList::load(&file);
    assert_eq!(list.projects.len(), MAX_ENTRIES);
    assert_eq!(names(&list.projects)[0], "p0.sheaf");
}

#[test]
fn an_unknown_key_in_the_file_is_ignored() {
    let dir = scratch("unknown");
    let file = dir.join(FILE_NAME);
    let a = touch(&dir, "a.sheaf");
    std::fs::write(&file, serde_json::json!({ "projects": [a.to_str().unwrap()], "books": ["x"] }).to_string())
        .unwrap();
    assert_eq!(RecentList::load(&file).projects, vec![a]);
}

#[test]
fn the_folder_is_created_on_the_first_change() {
    let dir = scratch("folder");
    let file = dir.join("a").join("b").join(FILE_NAME);
    let p = touch(&dir, "x.sheaf");
    record_project(&file, &p).unwrap();
    assert!(file.is_file());
}

#[test]
fn a_change_keeps_what_another_instance_added_in_the_meantime() {
    let dir = scratch("merge");
    let file = dir.join(FILE_NAME);
    let (a, b, c) = (touch(&dir, "a.sheaf"), touch(&dir, "b.sheaf"), touch(&dir, "c.sheaf"));
    // Instance one has read the file (empty) long ago; instance two then records b and an import.
    let stale = RecentList::load(&file);
    assert!(stale.projects.is_empty());
    record_project(&file, &b).unwrap();
    record_imports(&file, &[&c]).unwrap();
    // Instance one now records a: b and the import must still be there.
    let merged = record_project(&file, &a).unwrap();
    assert_eq!(names(&merged.projects), ["a.sheaf", "b.sheaf"]);
    assert_eq!(names(&merged.imports), ["c.sheaf"]);
    assert_eq!(RecentList::load(&file), merged);
}

#[test]
fn many_writers_at_once_lose_no_entry() {
    let dir = scratch("threads");
    let file = Arc::new(dir.join(FILE_NAME));
    let files: Vec<PathBuf> = (0..8).map(|i| touch(&dir, &format!("t{i}.sheaf"))).collect();
    let barrier = Arc::new(Barrier::new(files.len()));
    let handles: Vec<_> = files
        .into_iter()
        .map(|p| {
            let (file, barrier) = (file.clone(), barrier.clone());
            std::thread::spawn(move || {
                barrier.wait();
                record_project(&file, &p).unwrap();
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    let list = RecentList::load(&file);
    assert_eq!(list.projects.len(), 8);
}

#[test]
fn remove_drops_a_file_from_both_lists_by_any_spelling() {
    let dir = scratch("remove");
    let (a, b) = (touch(&dir, "a.sheaf"), touch(&dir, "b.sheaf"));
    let mut list = RecentList::default();
    list.push_project(&a);
    list.push_project(&b);
    list.push_import(&[&a]);
    list.remove(&dir.join(".").join("a.sheaf"));
    assert_eq!(list.projects, vec![b]);
    assert!(list.imports.is_empty());
}

#[test]
fn remove_still_finds_a_file_that_was_deleted_after_it_was_listed() {
    let dir = scratch("deleted");
    let file = dir.join(FILE_NAME);
    let a = touch(&dir, "a.sheaf");
    record_project(&file, &a).unwrap();
    std::fs::remove_file(&a).unwrap();
    assert!(forget(&file, &a).unwrap().projects.is_empty());
    assert!(RecentList::load(&file).projects.is_empty());
}

#[test]
fn remove_of_an_unknown_file_changes_nothing() {
    let dir = scratch("unknown-remove");
    let a = touch(&dir, "a.sheaf");
    let mut list = RecentList::default();
    list.push_project(&a);
    let before = list.clone();
    list.remove(&dir.join("other.sheaf"));
    assert_eq!(list, before);
}

#[test]
fn clearing_one_list_leaves_the_other() {
    let dir = scratch("clear");
    let file = dir.join(FILE_NAME);
    record_project(&file, &touch(&dir, "a.sheaf")).unwrap();
    record_imports(&file, &[touch(&dir, "d.csv")]).unwrap();
    let after = clear_projects(&file).unwrap();
    assert!(after.projects.is_empty());
    assert_eq!(names(&after.imports), ["d.csv"]);
    let after = clear_imports(&file).unwrap();
    assert_eq!(after, RecentList::default());
    assert_eq!(RecentList::load(&file), RecentList::default());
}

#[test]
fn a_path_with_spaces_and_non_ascii_letters_round_trips() {
    let dir = scratch("text");
    let file = dir.join(FILE_NAME);
    let p = touch(&dir, "Données été 2026 (v2).sheaf");
    record_project(&file, &p).unwrap();
    assert_eq!(RecentList::load(&file).projects, vec![p]);
}

#[cfg(unix)]
#[test]
fn a_path_that_is_not_text_round_trips_on_unix() {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    let dir = scratch("bytes");
    let file = dir.join(FILE_NAME);
    let name = std::ffi::OsString::from_vec(b"bad-\xff\xfe-name.sheaf".to_vec());
    let p = dir.join(&name);
    if std::fs::write(&p, b"x").is_err() {
        return; // a file system that refuses such names (APFS may); nothing to test here
    }
    let list = record_project(&file, &p).unwrap();
    assert_eq!(list.projects[0].file_name().unwrap().as_bytes(), name.as_bytes());
    let loaded = RecentList::load(&file);
    assert_eq!(loaded, list);
    assert!(loaded.projects[0].to_str().is_none());
}

#[cfg(unix)]
#[test]
fn raw_bytes_in_the_file_round_trip_even_when_the_file_does_not_exist() {
    use std::os::unix::ffi::OsStrExt;
    let dir = scratch("raw");
    let file = dir.join(FILE_NAME);
    let mut raw = dir.as_os_str().as_bytes().to_vec();
    raw.extend_from_slice(b"/caf\xe9.sheaf");
    let p = PathBuf::from(std::ffi::OsStr::from_bytes(&raw));
    let list = record_project(&file, &p).unwrap();
    assert_eq!(list.projects[0].as_os_str().as_bytes(), &raw[..]);
    assert_eq!(RecentList::load(&file), list);
}

#[test]
fn two_non_text_paths_that_print_alike_are_not_mistaken_for_one() {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let a = PathBuf::from(std::ffi::OsStr::from_bytes(b"/t/\xff.sheaf"));
        let b = PathBuf::from(std::ffi::OsStr::from_bytes(b"/t/\xfe.sheaf"));
        assert_ne!(key(&a, true), key(&b, true));
        assert_ne!(key(&a, false), key(&b, false));
    }
}

#[test]
fn the_newest_book_and_the_newest_graph_come_first_in_lists_of_their_own() {
    let dir = scratch("windows");
    let (b1, b2) = (touch(&dir, "a.sheafbook"), touch(&dir, "b.sheafbook"));
    let (g1, g2) = (touch(&dir, "a.sheafgraph"), touch(&dir, "b.sheafgraph"));
    let mut list = RecentList::default();
    for (kind, p) in [
        (WindowFileKind::Book, &b1),
        (WindowFileKind::Graph, &g1),
        (WindowFileKind::Book, &b2),
        (WindowFileKind::Graph, &g2),
        (WindowFileKind::Book, &b1),
    ] {
        list.push_window(kind, p);
    }
    assert_eq!(names(&list.books), ["a.sheafbook", "b.sheafbook"], "a repeat moves to the top");
    assert_eq!(names(&list.graphs), ["b.sheafgraph", "a.sheafgraph"]);
    assert!(list.projects.is_empty() && list.imports.is_empty());
}

#[test]
fn the_books_and_graphs_keep_fifteen_entries_each() {
    let dir = scratch("windowcap");
    let mut list = RecentList::default();
    for i in 0..20 {
        list.push_window(WindowFileKind::Book, &touch(&dir, &format!("b{i}.sheafbook")));
        list.push_window(WindowFileKind::Graph, &touch(&dir, &format!("g{i}.sheafgraph")));
    }
    assert_eq!((list.books.len(), list.graphs.len()), (MAX_ENTRIES, MAX_ENTRIES));
    assert_eq!(names(&list.books)[0], "b19.sheafbook");
}

#[test]
fn the_books_and_graphs_survive_a_round_trip_through_the_file() {
    let dir = scratch("windowround");
    let file = dir.join(FILE_NAME);
    let b = touch(&dir, "a.sheafbook");
    let g = touch(&dir, "a.sheafgraph");
    record_window(&file, WindowFileKind::Book, &b).unwrap();
    let list = record_window(&file, WindowFileKind::Graph, &g).unwrap();
    assert_eq!(RecentList::load(&file), list);
    assert_eq!(names(&list.books), ["a.sheafbook"]);
    assert_eq!(names(&list.graphs), ["a.sheafgraph"]);
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("\"books\"") && text.contains("\"graphs\""), "{text}");
}

#[test]
fn an_older_file_without_books_and_graphs_still_loads_and_keeps_its_lists_when_rewritten() {
    let dir = scratch("older");
    let file = dir.join(FILE_NAME);
    let (p, csv) = (touch(&dir, "a.sheaf"), touch(&dir, "d.csv"));
    let old = serde_json::json!({ "projects": [p.to_str().unwrap()], "imports": [csv.to_str().unwrap()] });
    std::fs::write(&file, old.to_string()).unwrap();
    let list = RecentList::load(&file);
    assert_eq!(names(&list.projects), ["a.sheaf"]);
    assert_eq!(names(&list.imports), ["d.csv"]);
    assert!(list.books.is_empty() && list.graphs.is_empty());
    let after = record_window(&file, WindowFileKind::Book, &touch(&dir, "x.sheafbook")).unwrap();
    assert_eq!(names(&after.projects), ["a.sheaf"]);
    assert_eq!(names(&after.books), ["x.sheafbook"]);
}

#[test]
fn remove_and_clear_treat_the_books_and_graphs_like_the_other_lists() {
    let dir = scratch("windowremove");
    let file = dir.join(FILE_NAME);
    let (b, g) = (touch(&dir, "a.sheafbook"), touch(&dir, "a.sheafgraph"));
    record_window(&file, WindowFileKind::Book, &b).unwrap();
    record_window(&file, WindowFileKind::Graph, &g).unwrap();
    let list = clear_books(&file).unwrap();
    assert!(list.books.is_empty() && list.graphs.len() == 1, "clearing the books leaves the graphs");
    let list = forget(&file, &g).unwrap();
    assert!(list.graphs.is_empty());
    record_window(&file, WindowFileKind::Graph, &g).unwrap();
    record_window(&file, WindowFileKind::Book, &b).unwrap();
    std::fs::remove_file(&b).unwrap();
    assert!(forget(&file, &b).unwrap().books.is_empty(), "a deleted file is still forgotten");
    let list = clear_graphs(&file).unwrap();
    assert!(list.graphs.is_empty());
}

#[test]
fn a_damaged_books_entry_is_skipped_and_the_other_lists_are_kept() {
    let dir = scratch("windowdamaged");
    let file = dir.join(FILE_NAME);
    let (p, g) = (touch(&dir, "a.sheaf"), touch(&dir, "a.sheafgraph"));
    let text = serde_json::json!({ "projects": [p.to_str().unwrap()], "books": 5, "graphs": [7, g.to_str().unwrap(), "relative.sheafgraph"] });
    std::fs::write(&file, text.to_string()).unwrap();
    let list = RecentList::load(&file);
    assert_eq!(names(&list.projects), ["a.sheaf"]);
    assert!(list.books.is_empty());
    assert_eq!(names(&list.graphs), ["a.sheafgraph"]);
}
