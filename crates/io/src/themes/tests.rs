use super::*;
use crate::ascii::{AsciiOptions, Blocks, DateImport, Delimiter, ImportSelection, Thousands};
use crate::batch::{BatchLayout, BatchOptions, PlaceOptions};
use crate::excel::SheetRef;
use serde_json::json;
use sheaf_core::datetime::DateOrder;
use std::sync::{Arc, Barrier};

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-io-themes-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Every field of the text options away from its default.
fn busy_ascii() -> AsciiOptions {
    AsciiOptions {
        delimiter: Delimiter::AnyOf { chars: ";|".into(), combine: true },
        header_lines: Some(4),
        long_name_line: Some(2),
        units_line: Some(3),
        comments_line: Some(1),
        rename_columns: false,
        short_name_line: 5,
        rename_sheet: false,
        rename_book: false,
        decimal_comma: true,
        thousands: Thousands::Apostrophe,
        param_lines: vec![6, 7],
        param_name_from_first_cell: false,
        blocks: Blocks::Sheets,
        first_column_x: false,
        skip_rows: 2,
        max_rows: Some(100),
        rows: ImportSelection { first: 3, last: 50, read: 2, skip: 1 },
        columns: ImportSelection { first: 2, last: 9, read: 3, skip: 2 },
        dates: DateImport::AsText,
        date_order: DateOrder::Dmy,
    }
}

fn busy_chi() -> ChiOptions {
    ChiOptions {
        split_scans: false,
        split_segments: false,
        add_milliamp: false,
        add_charge: false,
        invert_current: true,
        exact_time: false,
    }
}

fn busy_excel() -> ExcelOptions {
    ExcelOptions {
        sheets: vec![SheetRef::Name("Data".into()), SheetRef::Index(2), SheetRef::NameOrIndex("2023".into())],
        include_hidden: false,
        header_lines: Some(3),
        long_name_line: Some(1),
        units_line: Some(2),
        comments_line: Some(3),
        rename_columns: false,
        short_name_line: 4,
        rename_book: false,
        param_lines: vec![5],
        param_name_from_first_cell: false,
        first_column_x: false,
        skip_rows: 1,
        max_rows: Some(20),
        rows: ImportSelection { first: 2, last: 30, read: 4, skip: 1 },
        columns: ImportSelection { first: 3, last: 8, read: 2, skip: 2 },
        dates: DateImport::AsText,
    }
}

fn round_trip<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: &T) {
    let text = serde_json::to_string(value).unwrap();
    let back: T = serde_json::from_str(&text).unwrap();
    assert_eq!(&back, value);
}

#[test]
fn the_folder_is_next_to_the_autosave_settings() {
    if let Some(d) = default_dir() {
        assert_eq!(d.file_name().unwrap(), FOLDER_NAME);
        assert_eq!(d.parent(), recovery::default_settings_path().unwrap().parent());
    }
}

#[test]
fn each_kind_has_its_own_file() {
    let dir = Path::new("themes");
    let files: Vec<PathBuf> = Kind::ALL.iter().map(|k| k.file(dir)).collect();
    assert_eq!(files, [dir.join("ascii.json"), dir.join("chi.json"), dir.join("excel.json")]);
}

#[test]
fn every_text_option_survives_a_round_trip() {
    round_trip(&busy_ascii());
    round_trip(&AsciiOptions::default());
}

#[test]
fn every_kind_of_delimiter_survives_a_round_trip() {
    for d in [
        Delimiter::Auto,
        Delimiter::Comma,
        Delimiter::Tab,
        Delimiter::Semicolon,
        Delimiter::Whitespace,
        Delimiter::Other('|'),
        Delimiter::AnyOf { chars: ",;".into(), combine: false },
        Delimiter::Multi("::".into()),
        Delimiter::Fixed(vec![8, 12, 10]),
    ] {
        round_trip(&d);
    }
}

#[test]
fn every_chi_option_survives_a_round_trip() {
    round_trip(&busy_chi());
    round_trip(&ChiOptions::default());
}

#[test]
fn every_excel_option_survives_a_round_trip() {
    round_trip(&busy_excel());
    round_trip(&ExcelOptions::default());
}

#[test]
fn the_batch_and_placement_options_survive_a_round_trip() {
    let batch = BatchOptions {
        layout: BatchLayout::Rows,
        kind: ImportKind::Chi,
        chi: busy_chi(),
        ascii: busy_ascii(),
        book_name: "Runs".into(),
        file_name_in_comments: true,
        partial_except_first: true,
    };
    let text = serde_json::to_string(&batch).unwrap();
    let back: BatchOptions = serde_json::from_str(&text).unwrap();
    assert_eq!(serde_json::to_value(&back).unwrap(), serde_json::to_value(&batch).unwrap());
    assert_eq!(back.layout, BatchLayout::Rows);
    assert_eq!(back.ascii, busy_ascii());
    round_trip(&PlaceOptions { rename_sheet: false });
    for mode in ImportMode::ALL {
        round_trip(&mode);
    }
    for kind in [ImportKind::Auto, ImportKind::Chi, ImportKind::Ascii] {
        round_trip(&kind);
    }
}

#[test]
fn the_window_settings_survive_a_round_trip() {
    round_trip(&AsciiSettings {
        data_type: ImportKind::Ascii,
        ascii: busy_ascii(),
        partial_except_first: true,
        other_delimiter: "~".into(),
        combine_delimiters: true,
        fixed_widths: "4 8".into(),
        mode: ImportMode::NewRows,
        plot: false,
    });
    round_trip(&ChiSettings { chi: busy_chi(), mode: ImportMode::NewSheet, plot: false });
    round_trip(&ExcelSettings {
        options: busy_excel(),
        partial_except_first: true,
        mode: ImportMode::NewColumns,
        plot: true,
    });
}

#[test]
fn a_file_from_an_older_sheaf_is_filled_from_the_defaults() {
    let old = json!({ "delimiter": "Comma", "header_lines": 3, "skip_rows": 7 });
    let opts: AsciiOptions = serde_json::from_value(old).unwrap();
    assert_eq!(opts.delimiter, Delimiter::Comma);
    assert_eq!(opts.header_lines, Some(3));
    assert_eq!(opts.skip_rows, 7);
    let expected =
        AsciiOptions { delimiter: Delimiter::Comma, header_lines: Some(3), skip_rows: 7, ..AsciiOptions::default() };
    assert_eq!(opts, expected);
    let chi: ChiOptions = serde_json::from_value(json!({ "invert_current": true })).unwrap();
    assert!(chi.invert_current && chi.split_scans);
    assert_eq!(serde_json::from_value::<ExcelOptions>(json!({})).unwrap(), ExcelOptions::default());
    assert_eq!(serde_json::from_value::<AsciiSettings>(json!({})).unwrap(), AsciiSettings::default());
    assert_eq!(serde_json::from_value::<ImportSelection>(json!({ "first": 4 })).unwrap().read, 1);
    let any: Delimiter = serde_json::from_value(json!({ "AnyOf": { "chars": "|" } })).unwrap();
    assert_eq!(any, Delimiter::AnyOf { chars: "|".into(), combine: false });
}

#[test]
fn fields_from_a_newer_sheaf_are_ignored() {
    let newer = json!({ "split_scans": false, "a_future_option": { "x": 1 } });
    let opts: ChiOptions = serde_json::from_value(newer).unwrap();
    assert!(!opts.split_scans && opts.add_charge);
    let w: AsciiSettings = serde_json::from_value(json!({ "plot": false, "unknown": [1, 2] })).unwrap();
    assert!(!w.plot);
}

#[test]
fn a_missing_store_is_empty() {
    let dir = scratch("missing");
    let store = ThemeStore::load(&Kind::Ascii.file(&dir));
    assert_eq!(store, ThemeStore::default());
    assert!(last_used::<AsciiSettings>(&dir).is_none());
    assert!(names::<AsciiSettings>(&dir).is_empty());
}

#[test]
fn last_used_comes_back_after_a_restart() {
    let dir = scratch("last");
    let settings = AsciiSettings { ascii: busy_ascii(), plot: false, ..AsciiSettings::default() };
    record_last_used(&dir, &settings).unwrap();
    assert_eq!(last_used::<AsciiSettings>(&dir), Some(settings.clone()));
    assert_eq!(resolve::<AsciiSettings>(&dir, LAST_USED_LABEL).unwrap(), settings);
    // Another kind has its own store.
    assert!(last_used::<ChiSettings>(&dir).is_none());
    let newer = AsciiSettings { fixed_widths: "7".into(), ..settings };
    record_last_used(&dir, &newer).unwrap();
    assert_eq!(last_used::<AsciiSettings>(&dir), Some(newer));
}

#[test]
fn a_saved_theme_comes_back_by_name_in_any_letter_case() {
    let dir = scratch("named");
    let s = ChiSettings { chi: busy_chi(), ..ChiSettings::default() };
    assert_eq!(save_theme(&dir, "  CHI   CV 3 header lines ", &s).unwrap(), "CHI CV 3 header lines");
    assert_eq!(names::<ChiSettings>(&dir), ["CHI CV 3 header lines"]);
    assert_eq!(theme::<ChiSettings>(&dir, "chi cv 3 HEADER lines"), Some(s.clone()));
    assert_eq!(resolve::<ChiSettings>(&dir, "CHI CV 3 header lines").unwrap(), s);
    assert!(theme::<ChiSettings>(&dir, "other").is_none());
}

#[test]
fn saving_under_a_taken_name_replaces_that_theme_in_place() {
    let dir = scratch("replace");
    let a = ExcelSettings::default();
    let b = ExcelSettings { plot: true, ..ExcelSettings::default() };
    save_theme(&dir, "Monthly", &a).unwrap();
    save_theme(&dir, "Other", &a).unwrap();
    assert_eq!(save_theme(&dir, "MONTHLY", &b).unwrap(), "MONTHLY");
    assert_eq!(names::<ExcelSettings>(&dir), ["MONTHLY", "Other"]);
    assert_eq!(theme::<ExcelSettings>(&dir, "monthly"), Some(b));
}

#[test]
fn deleting_a_theme_removes_only_that_theme() {
    let dir = scratch("delete");
    let s = AsciiSettings::default();
    save_theme(&dir, "A", &s).unwrap();
    save_theme(&dir, "B", &s).unwrap();
    record_last_used(&dir, &s).unwrap();
    assert!(delete_theme::<AsciiSettings>(&dir, "a").unwrap());
    assert!(!delete_theme::<AsciiSettings>(&dir, "a").unwrap());
    assert_eq!(names::<AsciiSettings>(&dir), ["B"]);
    assert!(last_used::<AsciiSettings>(&dir).is_some());
}

#[test]
fn names_are_cleaned_and_the_built_in_entries_are_refused() {
    assert_eq!(clean_name("  a \t b\n c ").unwrap(), "a b c");
    assert_eq!(clean_name("a\u{0}b").unwrap(), "ab");
    assert_eq!(clean_name("CV (3 \u{b5}A)").unwrap(), "CV (3 \u{b5}A)");
    assert_eq!(clean_name(""), Err(NameError::Empty));
    assert_eq!(clean_name(" \t\n"), Err(NameError::Empty));
    assert_eq!(clean_name("<default>"), Err(NameError::Reserved));
    assert_eq!(clean_name("<Last Used>"), Err(NameError::Reserved));
    assert_eq!(clean_name("a <b>").unwrap(), "a <b>");
    let long = "x".repeat(MAX_NAME_CHARS + 20);
    assert_eq!(clean_name(&long).unwrap().chars().count(), MAX_NAME_CHARS);
    // The cut does not leave a space at the end.
    let spaced = format!("{} tail", "y".repeat(MAX_NAME_CHARS - 1));
    assert_eq!(clean_name(&spaced).unwrap(), "y".repeat(MAX_NAME_CHARS - 1));
}

#[test]
fn a_refused_name_writes_nothing() {
    let dir = scratch("refused");
    let s = AsciiSettings::default();
    assert!(matches!(save_theme(&dir, "", &s), Err(ThemeError::Name(NameError::Empty))));
    assert!(matches!(save_theme(&dir, "<default>", &s), Err(ThemeError::Name(NameError::Reserved))));
    assert!(!Kind::Ascii.file(&dir).exists());
}

#[test]
fn the_default_entry_gives_the_built_in_settings() {
    let dir = scratch("default");
    assert_eq!(resolve::<AsciiSettings>(&dir, "<default>").unwrap(), AsciiSettings::default());
    assert_eq!(resolve::<ExcelSettings>(&dir, " <DEFAULT> ").unwrap(), ExcelSettings::default());
}

#[test]
fn an_unknown_theme_name_lists_the_saved_themes() {
    let dir = scratch("unknown");
    let s = AsciiSettings::default();
    save_theme(&dir, "First", &s).unwrap();
    save_theme(&dir, "Second", &s).unwrap();
    let err = resolve::<AsciiSettings>(&dir, "third").unwrap_err();
    assert!(matches!(err, ThemeError::Unknown { .. }));
    let text = err.to_string();
    assert!(text.contains("'third'") && text.contains("First, Second"), "{text}");
    let none = resolve::<ChiSettings>(&dir, "third").unwrap_err().to_string();
    assert!(none.contains("no themes are saved"), "{none}");
}

#[test]
fn last_used_before_any_import_is_an_error() {
    let dir = scratch("nolast");
    assert!(matches!(resolve::<AsciiSettings>(&dir, LAST_USED_LABEL), Err(ThemeError::NoLastUsed)));
}

#[test]
fn a_theme_of_the_excel_window_does_not_keep_the_sheets() {
    let dir = scratch("sheets");
    let s = ExcelSettings { options: busy_excel(), ..ExcelSettings::default() };
    save_theme(&dir, "No sheets", &s).unwrap();
    record_last_used(&dir, &s).unwrap();
    let want =
        ExcelSettings { options: ExcelOptions { sheets: Vec::new(), ..busy_excel() }, ..ExcelSettings::default() };
    assert_eq!(theme::<ExcelSettings>(&dir, "No sheets"), Some(want.clone()));
    assert_eq!(last_used::<ExcelSettings>(&dir), Some(want));
}

#[test]
fn a_corrupt_file_loads_empty_and_is_kept_aside_when_the_next_change_comes() {
    let dir = scratch("corrupt");
    let file = Kind::Ascii.file(&dir);
    std::fs::write(&file, b"{ this is not json").unwrap();
    assert_eq!(ThemeStore::load(&file), ThemeStore::default());
    assert!(last_used::<AsciiSettings>(&dir).is_none());
    save_theme(&dir, "Fresh", &AsciiSettings::default()).unwrap();
    assert_eq!(names::<AsciiSettings>(&dir), ["Fresh"]);
    assert_eq!(std::fs::read(dir.join("ascii.json.bad")).unwrap(), b"{ this is not json");
    // Other shapes of damage: not an object, empty file, binary.
    for bytes in [&b"[1,2]"[..], b"", b"\xff\xfe\x00"] {
        std::fs::write(&file, bytes).unwrap();
        assert_eq!(ThemeStore::load(&file), ThemeStore::default());
    }
}

#[test]
fn a_damaged_theme_is_skipped_and_the_others_are_kept() {
    let dir = scratch("damaged");
    let file = Kind::Ascii.file(&dir);
    let text = json!({
        "version": 1,
        "last_used": 5,
        "themes": [
            { "name": "good", "settings": { "plot": false } },
            { "settings": {} },
            { "name": 7, "settings": {} },
            { "name": "no settings" },
            { "name": "bad settings", "settings": [1] },
            { "name": "<default>", "settings": {} },
            { "name": "GOOD", "settings": { "plot": true } },
            "text",
            { "name": "also good", "settings": {} }
        ]
    });
    std::fs::write(&file, text.to_string()).unwrap();
    assert_eq!(names::<AsciiSettings>(&dir), ["good", "also good"]);
    assert!(last_used::<AsciiSettings>(&dir).is_none());
    assert!(!theme::<AsciiSettings>(&dir, "good").unwrap().plot);
}

#[test]
fn a_theme_that_does_not_fit_the_settings_is_listed_but_cannot_be_used() {
    let dir = scratch("unreadable");
    let file = Kind::Ascii.file(&dir);
    let text = json!({ "themes": [
        { "name": "future", "settings": { "ascii": { "delimiter": "FutureKind" } } },
        { "name": "plain", "settings": {} }
    ] });
    std::fs::write(&file, text.to_string()).unwrap();
    assert_eq!(names::<AsciiSettings>(&dir), ["future", "plain"]);
    assert!(theme::<AsciiSettings>(&dir, "future").is_none());
    assert!(matches!(resolve::<AsciiSettings>(&dir, "future"), Err(ThemeError::Unreadable { .. })));
    assert!(theme::<AsciiSettings>(&dir, "plain").is_some());
    // A change keeps the theme it could not read.
    save_theme(&dir, "new", &AsciiSettings::default()).unwrap();
    assert_eq!(names::<AsciiSettings>(&dir), ["future", "plain", "new"]);
}

#[test]
fn a_file_from_a_newer_sheaf_with_extra_members_loads() {
    let dir = scratch("newer");
    let text = json!({
        "version": 9,
        "future_member": true,
        "last_used": { "plot": false, "future_field": 1 },
        "themes": [{ "name": "t", "settings": { "mode": "NewSheet" }, "future": 1 }]
    });
    std::fs::write(Kind::Ascii.file(&dir), text.to_string()).unwrap();
    assert!(!last_used::<AsciiSettings>(&dir).unwrap().plot);
    assert_eq!(theme::<AsciiSettings>(&dir, "t").unwrap().mode, ImportMode::NewSheet);
}

#[test]
fn a_file_of_a_newer_layout_is_kept_aside_before_it_is_written_again() {
    let dir = scratch("newer-backup");
    let file = Kind::Ascii.file(&dir);
    let text = json!({
        "version": 2,
        "themes": [{ "name": "t", "settings": { "mode": "NewSheet" } }]
    })
    .to_string();
    std::fs::write(&file, &text).unwrap();
    save_theme(&dir, "u", &AsciiSettings::default()).unwrap();
    assert_eq!(std::fs::read_to_string(dir.join("ascii.json.bad")).unwrap(), text);
    assert_eq!(names::<AsciiSettings>(&dir), ["t", "u"], "what this version can read is carried over");
}

#[test]
fn a_file_whose_themes_are_not_a_list_is_kept_aside_before_it_is_written_again() {
    let dir = scratch("themes-not-list");
    let file = Kind::Ascii.file(&dir);
    let text = json!({ "version": 1, "themes": { "t": {} } }).to_string();
    std::fs::write(&file, &text).unwrap();
    assert!(names::<AsciiSettings>(&dir).is_empty());
    save_theme(&dir, "u", &AsciiSettings::default()).unwrap();
    assert_eq!(std::fs::read_to_string(dir.join("ascii.json.bad")).unwrap(), text);
}

#[test]
fn a_file_that_cannot_be_read_is_an_error_and_is_left_alone() {
    let dir = scratch("unreadable-file");
    // A folder in the place of the file: reading it fails with something other than "not found".
    let file = Kind::Ascii.file(&dir);
    std::fs::create_dir_all(&file).unwrap();
    assert!(save_theme(&dir, "u", &AsciiSettings::default()).is_err());
    assert!(file.is_dir(), "nothing was replaced");
    assert!(!dir.join("ascii.json.bad").exists());
}

#[test]
fn the_file_is_pretty_text_with_a_version() {
    let dir = scratch("text");
    save_theme(&dir, "A", &ChiSettings::default()).unwrap();
    let text = std::fs::read_to_string(Kind::Chi.file(&dir)).unwrap();
    assert!(text.contains("\"version\": 1") && text.contains("\"name\": \"A\""), "{text}");
}

#[test]
fn two_writers_do_not_lose_each_others_themes() {
    let dir = scratch("writers");
    let writers = 6;
    let each = 5;
    let gate = Arc::new(Barrier::new(writers));
    let handles: Vec<_> = (0..writers)
        .map(|w| {
            let (dir, gate) = (dir.clone(), gate.clone());
            std::thread::spawn(move || {
                gate.wait();
                for i in 0..each {
                    save_theme(&dir, &format!("w{w}-{i}"), &AsciiSettings::default()).unwrap();
                    record_last_used(&dir, &AsciiSettings { fixed_widths: format!("{w}"), ..AsciiSettings::default() })
                        .unwrap();
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    let mut all = names::<AsciiSettings>(&dir);
    assert_eq!(all.len(), writers * each);
    all.sort();
    all.dedup();
    assert_eq!(all.len(), writers * each);
    assert!(last_used::<AsciiSettings>(&dir).is_some());
}

#[test]
fn a_change_keeps_what_another_instance_wrote_since_this_one_read() {
    let dir = scratch("merge");
    let file = Kind::Ascii.file(&dir);
    save_theme(&dir, "mine", &AsciiSettings::default()).unwrap();
    let stale = ThemeStore::load(&file);
    save_theme(&dir, "theirs", &AsciiSettings::default()).unwrap();
    assert_eq!(stale.names(), ["mine"]);
    update(&file, |s| s.last_used = Some(json!({}))).unwrap();
    assert_eq!(names::<AsciiSettings>(&dir), ["mine", "theirs"]);
    assert!(last_used::<AsciiSettings>(&dir).is_some());
}

#[test]
fn the_folder_is_made_when_the_first_theme_is_saved() {
    let dir = scratch("mkdir").join("deeper").join(FOLDER_NAME);
    assert!(!dir.exists());
    record_last_used(&dir, &AsciiSettings::default()).unwrap();
    assert!(Kind::Ascii.file(&dir).exists());
    assert!(Kind::Chi.file(&dir).parent().unwrap().is_dir());
}
