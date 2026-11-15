use super::*;
use sheaf_io::ascii::ImportSelection;
use sheaf_io::batch::ImportMode;
use sheaf_io::excel::SheetRef;
use sheaf_io::themes::{record_last_used, save_theme};

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-ops-themes-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn busy_text_options() -> AsciiOptions {
    AsciiOptions {
        delimiter: Delimiter::Semicolon,
        header_lines: Some(4),
        long_name_line: Some(1),
        units_line: Some(2),
        comments_line: Some(3),
        rename_columns: false,
        short_name_line: 2,
        rename_sheet: false,
        rename_book: false,
        decimal_comma: true,
        thousands: Thousands::Dot,
        param_lines: vec![2, 3],
        param_name_from_first_cell: false,
        blocks: Blocks::Sheets,
        first_column_x: false,
        skip_rows: 2,
        max_rows: Some(50),
        rows: ImportSelection { first: 3, last: 40, read: 2, skip: 1 },
        columns: ImportSelection { first: 2, last: 9, read: 3, skip: 2 },
        dates: DateImport::AsText,
        date_order: DateOrder::Dmy,
    }
}

#[test]
fn a_text_theme_comes_back_as_the_same_text_options() {
    let dir = scratch("text-round");
    let delimiters = [
        Delimiter::Auto,
        Delimiter::Comma,
        Delimiter::Tab,
        Delimiter::Semicolon,
        Delimiter::Whitespace,
        Delimiter::Other('|'),
        Delimiter::Other(','),
        Delimiter::Other(' '),
        Delimiter::Multi("::".into()),
        Delimiter::Multi("tab".into()),
        Delimiter::Multi("\\t".into()),
        Delimiter::Multi("Comma".into()),
        Delimiter::Multi("space".into()),
        Delimiter::AnyOf { chars: "|;".into(), combine: true },
        Delimiter::Fixed(vec![4, 8]),
    ];
    for d in delimiters {
        let mut settings = AsciiSettings { ascii: busy_text_options(), ..AsciiSettings::default() };
        settings.ascii.delimiter = d.clone();
        save_theme(&dir, "busy", &settings).unwrap();
        let themed = load(&dir, "busy").unwrap();
        assert_eq!(themed.read.ascii_options().unwrap(), settings.ascii, "delimiter {d:?}");
    }
}

#[test]
fn a_chi_theme_comes_back_as_the_same_chi_options() {
    let dir = scratch("chi-round");
    let chi = ChiOptions {
        split_scans: false,
        split_segments: false,
        add_milliamp: false,
        add_charge: false,
        invert_current: true,
        exact_time: false,
    };
    save_theme(&dir, "raw", &ChiSettings { chi: chi.clone(), ..ChiSettings::default() }).unwrap();
    let themed = load(&dir, "raw").unwrap();
    assert_eq!(themed.read.chi_options(), chi);
}

#[test]
fn an_excel_theme_sets_the_shared_options_and_the_hidden_sheet_choice() {
    let dir = scratch("excel-round");
    let mut options = ExcelOptions {
        include_hidden: false,
        header_lines: Some(2),
        rows: ImportSelection { first: 2, last: 0, read: 1, skip: 0 },
        first_column_x: false,
        skip_rows: 1,
        sheets: vec![SheetRef::Name("kept out of the store".into())],
        ..ExcelOptions::default()
    };
    save_theme(
        &dir,
        "monthly",
        &ExcelSettings { options: options.clone(), partial_except_first: true, ..Default::default() },
    )
    .unwrap();
    let themed = load(&dir, "monthly").unwrap();
    assert!(themed.partial_except_first);
    options.sheets.clear();
    assert_eq!(themed.read.excel_options(), options);
}

#[test]
fn text_files_take_the_text_store_and_excel_files_the_excel_store_when_both_have_the_name() {
    let dir = scratch("both");
    let text = AsciiSettings {
        ascii: AsciiOptions { header_lines: Some(3), ..AsciiOptions::default() },
        data_type: ImportKind::Ascii,
        ..AsciiSettings::default()
    };
    let excel = ExcelSettings {
        options: ExcelOptions { header_lines: Some(7), include_hidden: false, ..ExcelOptions::default() },
        ..ExcelSettings::default()
    };
    save_theme(&dir, "both", &text).unwrap();
    save_theme(&dir, "both", &excel).unwrap();
    let r = load(&dir, "both").unwrap().read;
    assert_eq!(r.header_lines, Some(3));
    assert_eq!(r.format, DataFormat::Ascii);
    assert!(r.skip_hidden_sheets, "the Excel store still sets what only it has");
    assert_eq!(r.ascii_options().unwrap().header_lines, Some(3));
    assert_eq!(r.excel_options().header_lines, Some(7), "an Excel file is read with the Excel window's settings");
}

#[test]
fn last_used_gives_an_excel_file_the_excel_windows_header_and_rows() {
    let dir = scratch("last-used-both");
    let text = AsciiSettings {
        ascii: AsciiOptions { header_lines: Some(1), ..AsciiOptions::default() },
        ..AsciiSettings::default()
    };
    let excel = ExcelSettings {
        options: ExcelOptions {
            header_lines: Some(3),
            rows: ImportSelection { first: 5, last: 0, read: 1, skip: 0 },
            ..ExcelOptions::default()
        },
        ..ExcelSettings::default()
    };
    record_last_used(&dir, &text).unwrap();
    record_last_used(&dir, &excel).unwrap();
    let r = load(&dir, "<last used>").unwrap().read;
    let sheet = r.excel_options();
    assert_eq!((sheet.header_lines, sheet.rows.first), (Some(3), 5));
    let csv = r.ascii_options().unwrap();
    assert_eq!((csv.header_lines, csv.rows.first), (Some(1), 1));
}

#[test]
fn typed_options_win_over_the_excel_themes_shared_options_too() {
    let dir = scratch("typed-excel");
    let excel = ExcelSettings {
        options: ExcelOptions { header_lines: Some(3), skip_rows: 0, ..ExcelOptions::default() },
        ..ExcelSettings::default()
    };
    save_theme(&dir, "x", &excel).unwrap();
    let themed = load(&dir, "x").unwrap().read;
    let flags = ReadSettings { header_lines: Some(9), first_row: 2, ..ReadSettings::default() };
    let merged = merge(&themed, &flags, &given(&["header_lines"]));
    let sheet = merged.excel_options();
    assert_eq!(sheet.header_lines, Some(9));
    assert_eq!(sheet.rows.first, 1, "an option that was not typed keeps the theme's value");
}

#[test]
fn default_and_last_used_entries_resolve() {
    let dir = scratch("entries");
    assert_eq!(load(&dir, "<default>").unwrap().read, ReadSettings::default());
    let err = load(&dir, "<last used>").unwrap_err().to_string();
    assert!(err.contains("no <last used>"), "{err}");
    let used =
        AsciiSettings { ascii: AsciiOptions { header_lines: Some(5), ..Default::default() }, ..Default::default() };
    record_last_used(&dir, &used).unwrap();
    assert_eq!(load(&dir, "<LAST USED>").unwrap().read.header_lines, Some(5));
}

#[test]
fn a_chi_only_last_used_still_counts() {
    let dir = scratch("chi-last");
    record_last_used(
        &dir,
        &ChiSettings { chi: ChiOptions { invert_current: true, ..Default::default() }, ..Default::default() },
    )
    .unwrap();
    let themed = load(&dir, "<last used>").unwrap();
    assert!(themed.read.invert_current);
    assert_eq!(themed.read.header_lines, None);
}

#[test]
fn an_unknown_name_lists_the_themes_of_every_store_once() {
    let dir = scratch("unknown");
    save_theme(&dir, "CV three headers", &AsciiSettings::default()).unwrap();
    save_theme(&dir, "cv THREE headers", &ChiSettings::default()).unwrap();
    save_theme(&dir, "Monthly", &ExcelSettings::default()).unwrap();
    let err = load(&dir, "nope").unwrap_err().to_string();
    assert_eq!(err, "no import theme named 'nope'; saved themes: CV three headers, Monthly");
    let empty = load(&scratch("unknown-empty"), "nope").unwrap_err().to_string();
    assert_eq!(empty, "no import theme named 'nope'; no themes are saved");
}

#[test]
fn a_theme_that_cannot_be_read_is_an_error_not_an_unknown_name() {
    let dir = scratch("unreadable");
    std::fs::write(
        text_store_file(&dir),
        r#"{"version":1,"themes":[{"name":"future","settings":{"ascii":{"delimiter":"FromTheFuture"}}}]}"#,
    )
    .unwrap();
    let err = load(&dir, "future").unwrap_err().to_string();
    assert_eq!(err, "the import theme 'future' cannot be read");
}

fn text_store_file(dir: &Path) -> PathBuf {
    themes::Kind::Ascii.file(dir)
}

fn given(ids: &'static [&'static str]) -> impl Fn(&str) -> bool {
    move |id| ids.contains(&id)
}

#[test]
fn typed_options_win_and_the_others_keep_the_theme() {
    let themed = ReadSettings { header_lines: Some(3), first_row: 5, decimal_comma: true, ..ReadSettings::default() };
    let flags = ReadSettings { header_lines: Some(1), first_row: 1, ..ReadSettings::default() };
    // First row is typed (and equals the default): it wins. Header lines are not typed: the theme's stay.
    let merged = merge(&themed, &flags, &given(&["first_row"]));
    assert_eq!(merged.first_row, 1);
    assert_eq!(merged.header_lines, Some(3));
    assert!(merged.decimal_comma);
}

#[test]
fn a_typed_delimiter_replaces_the_themes_delimiter_whole() {
    let themed = ReadSettings { fixed_width: vec![4, 8], ..ReadSettings::default() };
    let flags = ReadSettings { delimiter: Some(";".into()), ..ReadSettings::default() };
    let merged = merge(&themed, &flags, &given(&["delimiter"]));
    assert_eq!(merged.delimiter.as_deref(), Some(";"));
    assert!(merged.fixed_width.is_empty());
    let untouched = merge(&themed, &flags, &given(&[]));
    assert_eq!(untouched.fixed_width, vec![4, 8]);
    assert_eq!(untouched.delimiter, None);
}

#[test]
fn no_flag_options_of_the_theme_survive_a_merge() {
    let mut themed = ReadSettings::default();
    themed.text_no_flag.first_column_x = false;
    let merged = merge(&themed, &ReadSettings::default(), &given(&["format"]));
    assert!(!merged.ascii_options().unwrap().first_column_x);
}

#[test]
fn the_entry_and_the_folder_come_from_the_flags() {
    assert_eq!(entry(None, false), None);
    assert_eq!(entry(Some("x"), false).as_deref(), Some("x"));
    assert_eq!(entry(None, true).as_deref(), Some("<last used>"));
    assert_eq!(themes_dir(Some(Path::new("/tmp/somewhere"))).unwrap(), PathBuf::from("/tmp/somewhere"));
}

#[test]
fn the_mode_and_graph_tick_of_a_theme_are_not_applied() {
    let dir = scratch("mode");
    let s = AsciiSettings { mode: ImportMode::NewSheet, plot: false, ..AsciiSettings::default() };
    save_theme(&dir, "m", &s).unwrap();
    assert_eq!(load(&dir, "m").unwrap().read, ReadSettings::default());
}
