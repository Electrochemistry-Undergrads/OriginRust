//! `--theme`, `--last-used` and `--themes-dir` (T46): saved import settings on the command line.
//! Every test gives its own themes folder; none reads the real settings folder.

use sheaf_io::ascii::{AsciiOptions, Delimiter, ImportSelection};
use sheaf_io::themes::{AsciiSettings, ChiSettings, ExcelSettings, record_last_used, save_theme};
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-themes-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct Run {
    ok: bool,
    stderr: String,
    project: sheaf_core::Project,
}

fn sheaf(args: &[String], envs: &[(&str, &Path)]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sheaf"));
    cmd.args(args).env_remove("SHEAF_THEMES_DIR");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output().expect("the program starts")
}

/// Imports `files` (name, text) into `out.sheaf` with the extra arguments.
fn import(dir: &Path, files: &[(&str, &str)], extra: &[&str], envs: &[(&str, &Path)]) -> Run {
    let mut args: Vec<String> = vec!["import".into()];
    for (name, text) in files {
        std::fs::write(dir.join(name), text).unwrap();
        args.push(dir.join(name).to_string_lossy().into_owned());
    }
    let out = dir.join("out.sheaf");
    let _ = std::fs::remove_file(&out);
    args.extend(["--into".into(), out.to_string_lossy().into_owned()]);
    args.extend(extra.iter().map(|s| s.to_string()));
    let o = sheaf(&args, envs);
    let project =
        if o.status.success() { sheaf_io::project_file::load(&out).unwrap() } else { sheaf_core::Project::new() };
    Run { ok: o.status.success(), stderr: String::from_utf8_lossy(&o.stderr).into_owned(), project }
}

fn columns(run: &Run) -> usize {
    run.project.workbooks().next().unwrap().sheets[0].columns.len()
}

fn rows(run: &Run) -> usize {
    run.project.workbooks().next().unwrap().sheets[0].columns[0].len()
}

const SEMI: &str = "t;i\n1;2\n2;3\n3;4\n4;5\n";

fn semicolon_theme() -> AsciiSettings {
    AsciiSettings {
        ascii: AsciiOptions { delimiter: Delimiter::Semicolon, ..AsciiOptions::default() },
        ..Default::default()
    }
}

fn path(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

#[test]
fn a_saved_theme_fills_the_import_options() {
    let dir = scratch("apply");
    let themes = dir.join("themes");
    save_theme(&themes, "CV semicolons", &semicolon_theme()).unwrap();
    // An unreadable guess would give one column: the theme gives two.
    let plain = import(&dir, &[("a.txt", SEMI)], &["--delimiter", "comma"], &[]);
    assert!(plain.ok, "{}", plain.stderr);
    assert_eq!(columns(&plain), 1);
    let themed = import(&dir, &[("a.txt", SEMI)], &["--theme", "CV semicolons", "--themes-dir", &path(&themes)], &[]);
    assert!(themed.ok, "{}", themed.stderr);
    assert_eq!(columns(&themed), 2);
}

#[test]
fn the_theme_name_is_matched_without_regard_to_letter_case() {
    let dir = scratch("case");
    let themes = dir.join("themes");
    save_theme(&themes, "CV semicolons", &semicolon_theme()).unwrap();
    let run = import(&dir, &[("a.txt", SEMI)], &["--theme", "cv SEMICOLONS", "--themes-dir", &path(&themes)], &[]);
    assert!(run.ok, "{}", run.stderr);
    assert_eq!(columns(&run), 2);
}

#[test]
fn options_typed_on_the_command_line_win_over_the_theme() {
    let dir = scratch("flags-win");
    let themes = dir.join("themes");
    let mut theme = semicolon_theme();
    theme.ascii.rows = ImportSelection { first: 3, last: 0, read: 1, skip: 0 };
    save_theme(&themes, "late start", &theme).unwrap();
    let t = path(&themes);
    // The theme alone starts at data row 3: two rows are left of four.
    let themed = import(&dir, &[("a.txt", SEMI)], &["--theme", "late start", "--themes-dir", &t], &[]);
    assert!(themed.ok, "{}", themed.stderr);
    assert_eq!((columns(&themed), rows(&themed)), (2, 2));
    // --first-row 1 is the default value, but it was typed: it wins.
    let typed =
        import(&dir, &[("a.txt", SEMI)], &["--theme", "late start", "--themes-dir", &t, "--first-row", "1"], &[]);
    assert!(typed.ok, "{}", typed.stderr);
    assert_eq!(rows(&typed), 4);
    // A typed delimiter replaces the theme's delimiter; the theme's row selection stays.
    let comma =
        import(&dir, &[("a.txt", SEMI)], &["--theme", "late start", "--themes-dir", &t, "--delimiter", "comma"], &[]);
    assert!(comma.ok, "{}", comma.stderr);
    let by_hand = import(&dir, &[("a.txt", SEMI)], &["--delimiter", "comma", "--first-row", "3"], &[]);
    assert_eq!((columns(&comma), rows(&comma)), (columns(&by_hand), rows(&by_hand)));
    assert_eq!(columns(&comma), 1);
}

#[test]
fn last_used_applies_the_settings_of_the_last_ok() {
    let dir = scratch("last-used");
    let themes = dir.join("themes");
    record_last_used(&themes, &semicolon_theme()).unwrap();
    let run = import(&dir, &[("a.txt", SEMI)], &["--last-used", "--themes-dir", &path(&themes)], &[]);
    assert!(run.ok, "{}", run.stderr);
    assert_eq!(columns(&run), 2);
    let same = import(&dir, &[("a.txt", SEMI)], &["--theme", "<last used>", "--themes-dir", &path(&themes)], &[]);
    assert_eq!(columns(&same), 2);
}

#[test]
fn the_command_line_never_changes_the_saved_themes() {
    let dir = scratch("read-only");
    let themes = dir.join("themes");
    record_last_used(&themes, &semicolon_theme()).unwrap();
    let before = std::fs::read(sheaf_io::themes::Kind::Ascii.file(&themes)).unwrap();
    let run =
        import(&dir, &[("a.txt", SEMI)], &["--last-used", "--themes-dir", &path(&themes), "--header-lines", "1"], &[]);
    assert!(run.ok, "{}", run.stderr);
    assert_eq!(std::fs::read(sheaf_io::themes::Kind::Ascii.file(&themes)).unwrap(), before);
}

#[test]
fn last_used_before_any_ok_is_an_error() {
    let dir = scratch("no-last");
    let run = import(&dir, &[("a.txt", SEMI)], &["--last-used", "--themes-dir", &path(&dir.join("themes"))], &[]);
    assert!(!run.ok);
    assert!(run.stderr.contains("no <last used> import settings are saved yet"), "{}", run.stderr);
}

#[test]
fn an_unknown_theme_lists_the_saved_ones() {
    let dir = scratch("unknown");
    let themes = dir.join("themes");
    save_theme(&themes, "CV semicolons", &semicolon_theme()).unwrap();
    save_theme(&themes, "Monthly", &ExcelSettings::default()).unwrap();
    save_theme(&themes, "Raw current", &ChiSettings::default()).unwrap();
    let run = import(&dir, &[("a.txt", SEMI)], &["--theme", "CV semicolon", "--themes-dir", &path(&themes)], &[]);
    assert!(!run.ok);
    assert!(
        run.stderr.contains("no import theme named 'CV semicolon'; saved themes: CV semicolons, Raw current, Monthly"),
        "{}",
        run.stderr
    );
    assert!(!dir.join("out.sheaf").exists(), "nothing is written after a refused theme");
    let none = import(&dir, &[("a.txt", SEMI)], &["--theme", "x", "--themes-dir", &path(&dir.join("empty"))], &[]);
    assert!(none.stderr.contains("no themes are saved"), "{}", none.stderr);
}

#[test]
fn the_default_entry_gives_the_built_in_settings() {
    let dir = scratch("default-entry");
    let run =
        import(&dir, &[("a.txt", SEMI)], &["--theme", "<default>", "--themes-dir", &path(&dir.join("none"))], &[]);
    assert!(run.ok, "{}", run.stderr);
    let plain = import(&dir, &[("a.txt", SEMI)], &[], &[]);
    assert_eq!(columns(&run), columns(&plain));
}

#[test]
fn the_themes_folder_comes_from_the_environment_and_the_flag_beats_it() {
    let dir = scratch("env");
    let good = dir.join("good");
    let other = dir.join("other");
    save_theme(&good, "semi", &semicolon_theme()).unwrap();
    let from_env = import(&dir, &[("a.txt", SEMI)], &["--theme", "semi"], &[("SHEAF_THEMES_DIR", &good)]);
    assert!(from_env.ok, "{}", from_env.stderr);
    assert_eq!(columns(&from_env), 2);
    let flag = import(
        &dir,
        &[("a.txt", SEMI)],
        &["--theme", "semi", "--themes-dir", &path(&good)],
        &[("SHEAF_THEMES_DIR", &other)],
    );
    assert!(flag.ok, "{}", flag.stderr);
    assert_eq!(columns(&flag), 2);
}

#[test]
fn a_folder_is_not_read_unless_a_theme_is_asked_for() {
    let dir = scratch("not-read");
    let bad = dir.join("themes");
    std::fs::create_dir_all(&bad).unwrap();
    std::fs::write(bad.join("ascii.json"), "{ not json").unwrap();
    let run = import(&dir, &[("a.txt", SEMI)], &["--delimiter", "comma"], &[("SHEAF_THEMES_DIR", &bad)]);
    assert!(run.ok, "{}", run.stderr);
    assert_eq!(std::fs::read_to_string(bad.join("ascii.json")).unwrap(), "{ not json");
    assert!(!bad.join("ascii.json.bad").exists());
}

#[test]
fn the_partial_import_tick_of_a_theme_leaves_the_first_file_whole() {
    let dir = scratch("partial");
    let themes = dir.join("themes");
    let theme = AsciiSettings {
        partial_except_first: true,
        ascii: AsciiOptions {
            rows: ImportSelection { first: 3, last: 0, read: 1, skip: 0 },
            ..AsciiOptions::default()
        },
        ..AsciiSettings::default()
    };
    save_theme(&themes, "later files", &theme).unwrap();
    let files = [("one.txt", "t,i\n1,2\n2,3\n3,4\n4,5\n"), ("two.txt", "t,i\n1,2\n2,3\n3,4\n4,5\n")];
    let run = import(&dir, &files, &["--theme", "later files", "--themes-dir", &path(&themes)], &[]);
    assert!(run.ok, "{}", run.stderr);
    let sheets: Vec<usize> = run.project.workbooks().map(|b| b.sheets[0].columns[0].len()).collect();
    assert_eq!(sheets.len(), 2);
    assert_eq!(sheets.iter().filter(|&&n| n == 4).count(), 1, "{sheets:?}");
    assert_eq!(sheets.iter().filter(|&&n| n == 2).count(), 1, "{sheets:?}");
}

#[test]
fn info_and_the_other_commands_that_read_files_take_a_theme_too() {
    let dir = scratch("info");
    let themes = dir.join("themes");
    save_theme(&themes, "semi", &semicolon_theme()).unwrap();
    std::fs::write(dir.join("a.txt"), SEMI).unwrap();
    let a = path(&dir.join("a.txt"));
    let t = path(&themes);
    let info =
        sheaf(&["info".into(), a.clone(), "--theme".into(), "semi".into(), "--themes-dir".into(), t.clone()], &[]);
    let text = String::from_utf8_lossy(&info.stdout).into_owned();
    assert!(info.status.success(), "{}", String::from_utf8_lossy(&info.stderr));
    assert!(text.contains("(2 columns, 4 rows)"), "{text}");
    let stats = sheaf(
        &["analyze".into(), "stats".into(), a.clone(), "--theme".into(), "semi".into(), "--themes-dir".into(), t],
        &[],
    );
    assert!(stats.status.success(), "{}", String::from_utf8_lossy(&stats.stderr));
    let bad = sheaf(&["info".into(), a, "--theme".into(), "gone".into(), "--themes-dir".into(), path(&themes)], &[]);
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("saved themes: semi"));
}

#[test]
fn a_theme_and_last_used_cannot_be_given_together() {
    let dir = scratch("conflict");
    let run = import(&dir, &[("a.txt", SEMI)], &["--theme", "x", "--last-used"], &[]);
    assert!(!run.ok);
    assert!(run.stderr.contains("cannot be used with"), "{}", run.stderr);
}
