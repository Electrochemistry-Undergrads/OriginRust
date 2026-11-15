//! The Excel import and export windows, and the routes that lead to them (menus, dropped files,
//! files given on the command line).

use super::*;
use crate::actions::ImportPick;
use sheaf_core::{Column, ColumnData, ColumnId, Designation, Worksheet};
use sheaf_io::excel::{ExcelExportOptions, ExcelOptions, read_workbook, write_xlsx};

fn column_with(id: u64, short: &str, designation: Designation, data: ColumnData, labels: [&str; 3]) -> Column {
    let mut c = Column::new(ColumnId(id), short, designation, data);
    c.long_name = labels[0].into();
    c.units = labels[1].into();
    c.comments = labels[2].into();
    c
}

fn sheet_of(name: &str, columns: Vec<Column>) -> Worksheet {
    Worksheet { name: name.into(), columns, filter: None }
}

/// A workbook of three sheets written by the exporter: two with data and header rows, one holding
/// text only.
fn excel_file(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-app-excel-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("scans.xlsx");
    let cv = sheet_of(
        "Scan 1",
        vec![
            column_with(
                1,
                "A",
                Designation::X,
                ColumnData::numeric(vec![-0.2, -0.1, 0.0, 0.1]),
                ["Potential", "V", "vs Ag/AgCl"],
            ),
            column_with(
                2,
                "B",
                Designation::Y,
                ColumnData::numeric(vec![1.5e-6, 3.25e-6, 4.5e-6, 6.0e-6]),
                ["Current", "A", ""],
            ),
        ],
    );
    let cv2 = sheet_of(
        "Scan 2",
        vec![
            column_with(3, "A", Designation::X, ColumnData::numeric(vec![0.0, 1.0]), ["Time", "s", ""]),
            column_with(4, "B", Designation::Y, ColumnData::numeric(vec![10.0, 20.0]), ["Signal", "", ""]),
        ],
    );
    let notes = sheet_of(
        "Notes",
        vec![
            column_with(5, "A", Designation::X, ColumnData::numeric(vec![1.0, 2.0]), ["Run", "", ""]),
            column_with(
                6,
                "B",
                Designation::Y,
                ColumnData::text(vec!["first".into(), "second".into()]),
                ["Remark", "", ""],
            ),
        ],
    );
    write_xlsx(&[&cv, &cv2, &notes], &path, &ExcelExportOptions::default()).unwrap();
    path
}

fn hidden_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../io/tests/fixtures/excel/hidden.xlsx")
}

fn sheet_names(h: &Harness<'static, SheafApp>, book: usize) -> Vec<String> {
    project(h).workbooks().nth(book).unwrap().sheets.iter().map(|s| s.name.clone()).collect()
}

#[test]
fn the_import_and_export_items_are_on_and_open_excel_stays_grey() {
    let mut h = start(vec![data_file("excel-menu")]);
    click(&mut h, "Title Book1 - sample");
    menu_entry(&h, "File").click();
    settle(&mut h);
    assert!(!entry_enabled(&h, "Open Excel..."), "Open Excel needs Excel itself and stays grey");
    menu_entry(&h, "New").click();
    settle(&mut h);
    assert!(!entry_enabled(&h, "Excel"), "New > Excel stays grey");
    close_menus(&mut h);
    assert_reachable(&mut h, &["File", "Import", "Excel (XLS, XLSX, XLSM)..."]);
    assert_reachable(&mut h, &["File", "Export", "Worksheet as Excel..."]);
}

#[test]
fn a_workbook_given_on_the_command_line_imports_as_one_book_with_a_sheet_per_sheet() {
    let file = excel_file("startup");
    let mut h = start(vec![file]);
    assert_eq!(project(&h).workbooks().count(), 1);
    assert_eq!(sheet_names(&h, 0), ["Scan 1", "Scan 2", "Notes"]);
    let book = project(&h).workbooks().next().unwrap();
    assert_eq!(book.long_name, "scans.xlsx");
    let first = &book.sheets[0];
    assert_eq!(first.columns[0].long_name, "Potential");
    assert_eq!(first.columns[0].units, "V");
    assert_eq!(first.columns[0].comments, "vs Ag/AgCl");
    assert_eq!(first.columns[1].numeric().unwrap(), [1.5e-6, 3.25e-6, 4.5e-6, 6.0e-6]);
    assert_eq!(book.sheets[2].columns[1].text().unwrap(), ["first", "second"]);
    assert_eq!(project(&h).graphs().count(), 0, "a workbook is not graphed on its own");
    assert_eq!(h.state().doc.undo_label(), Some("Import data"));
    assert!(h.state().status.starts_with("Imported 1 file(s), 3 Excel sheet(s)"), "{}", h.state().status);
    // One undo step takes the whole import back.
    menu(&mut h, "Edit", "Undo Import data");
    assert_eq!(project(&h).workbooks().count(), 0);
}

#[test]
fn a_dropped_workbook_opens_the_excel_window_with_every_visible_sheet_ticked() {
    let file = excel_file("drop");
    let mut h = start(vec![data_file("excel-drop")]);
    h.state_mut().open_paths(vec![file], true);
    settle(&mut h);
    assert!(h.state().import.is_none(), "the ASCII window is not used for a workbook");
    assert!(h.state().import_excel.is_some());
    for label in ["Scan 1", "Scan 2", "Notes", "Select all", "Clear", "Import Options", "Header Rows"] {
        assert!(has_label(&h, label), "{label} is missing");
    }
    assert_eq!(h.state_mut().import_excel.as_mut().unwrap().selected_sheets(0), ["Scan 1", "Scan 2", "Notes"]);
    click(&mut h, "OK");
    assert!(h.state().import_excel.is_none());
    assert_eq!(project(&h).workbooks().count(), 2, "the sample and the Excel book");
    assert_eq!(sheet_names(&h, 1), ["Scan 1", "Scan 2", "Notes"]);
}

#[test]
fn ticking_sheets_limits_what_is_imported() {
    let file = excel_file("limit");
    let mut h = start(Vec::new());
    h.state_mut().start_import(Some(vec![file]), ImportPick::Excel);
    settle(&mut h);
    h.get_by_label("Scan 2").click();
    settle(&mut h);
    h.get_by_label("Notes").click();
    settle(&mut h);
    assert_eq!(h.state_mut().import_excel.as_mut().unwrap().selected_sheets(0), ["Scan 1"]);
    click(&mut h, "OK");
    assert_eq!(sheet_names(&h, 0), ["Scan 1"]);
    assert_eq!(project(&h).workbooks().next().unwrap().sheets[0].columns[0].long_name, "Potential");
}

#[test]
fn hidden_sheets_are_marked_and_ticked_as_origin_imports_them() {
    let mut h = start(Vec::new());
    h.state_mut().start_import(Some(vec![hidden_fixture()]), ImportPick::Excel);
    settle(&mut h);
    assert!(has_label(&h, "Visible"));
    assert!(has_label(&h, "Hidden (hidden)"));
    assert_eq!(h.state_mut().import_excel.as_mut().unwrap().selected_sheets(0), ["Visible", "Hidden", "Secret"]);
    click(&mut h, "Secret (hidden)");
    assert_eq!(h.state_mut().import_excel.as_mut().unwrap().selected_sheets(0), ["Visible", "Hidden"]);
    click(&mut h, "OK");
    assert_eq!(sheet_names(&h, 0), ["Visible", "Hidden"]);
    assert_eq!(project(&h).workbooks().next().unwrap().sheets[1].columns[0].numeric().unwrap(), [10.0, 20.0, 30.0]);
}

#[test]
fn clearing_every_sheet_reports_the_file_instead_of_importing() {
    let file = excel_file("clear");
    let mut h = start(Vec::new());
    h.state_mut().start_import(Some(vec![file]), ImportPick::Excel);
    settle(&mut h);
    click(&mut h, "Clear");
    click(&mut h, "OK");
    assert_eq!(project(&h).workbooks().count(), 0);
    let (_, text) = h.state().message.clone().expect("a message names the file");
    assert!(text.contains("scans.xlsx") && text.contains("no sheet is ticked"), "{text}");
}

#[test]
fn the_header_rows_page_shows_what_was_found_and_the_preview_names_each_row() {
    let file = excel_file("header");
    let mut h = start(Vec::new());
    h.state_mut().start_import(Some(vec![file]), ImportPick::Excel);
    settle(&mut h);
    click(&mut h, "Header Rows");
    for label in ["Lines before the data", "Line with the names", "Line with the units", "Line with comments"] {
        assert!(has_label(&h, label), "{label} is missing");
    }
    // The preview tags Excel rows 1 to 3 as the label rows and shows the column letters.
    assert!(has_label_containing(&h, "1 Name") && has_label_containing(&h, "2 Units"));
    assert!(has_label_containing(&h, "3 Comment"));
    let runs = h.state().import_excel.as_ref().unwrap().preview_runs();
    h.run_steps(10);
    assert_eq!(h.state().import_excel.as_ref().unwrap().preview_runs(), runs, "frames reuse the preview");
    // No row with names: the first row is data and the columns keep their letters.
    h.state_mut().import_excel.as_mut().unwrap().opts.long_name_line = Some(0);
    h.state_mut().import_excel.as_mut().unwrap().opts.units_line = Some(0);
    h.state_mut().import_excel.as_mut().unwrap().opts.comments_line = Some(0);
    h.state_mut().import_excel.as_mut().unwrap().opts.header_lines = Some(0);
    settle(&mut h);
    assert!(h.state().import_excel.as_ref().unwrap().preview_runs() > runs);
    click(&mut h, "OK");
    let book = project(&h).workbooks().next().unwrap();
    assert!(book.sheets[0].columns.iter().all(|c| c.long_name.is_empty()));
    assert_eq!(book.sheets[0].row_count(), 7, "the three label rows are now data rows");
}

#[test]
fn new_sheets_mode_adds_the_workbook_sheets_to_the_active_book() {
    let file = excel_file("newsheets");
    let mut h = start(vec![data_file("excel-newsheets")]);
    click(&mut h, "Title Book1 - sample");
    h.state_mut().start_import(Some(vec![file]), ImportPick::Excel);
    settle(&mut h);
    click(&mut h, "Import Mode");
    h.state_mut().import_excel.as_mut().unwrap().set_mode(sheaf_io::batch::ImportMode::NewSheet);
    settle(&mut h);
    click(&mut h, "OK");
    assert_eq!(project(&h).workbooks().count(), 1);
    assert_eq!(sheet_names(&h, 0), ["sample", "Scan 1", "Scan 2", "Notes"]);
}

#[test]
fn a_workbook_chosen_with_an_ascii_item_still_goes_to_the_excel_window() {
    let file = excel_file("ascii-item");
    let mut h = start(Vec::new());
    h.state_mut().start_import(Some(vec![file, data_file("excel-mixed")]), ImportPick::Multiple);
    assert!(h.state().import.is_some() && h.state().import_excel.is_some());
}

#[test]
fn warnings_of_the_reader_reach_the_messages_log() {
    let charts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../io/tests/fixtures/excel/charts.xlsx");
    let h = start(vec![charts]);
    assert_eq!(sheet_names(&h, 0), ["Data"]);
    let logged = h.state().logs.messages.iter().any(|m| m.text.contains("sheet 'ChartSheet' is a chart"));
    assert!(logged, "the skipped sheet is in the log");
    assert!(h.state().message.is_some());
}

#[test]
fn exporting_writes_the_ticked_sheets_and_reads_back_equal() {
    let source = excel_file("export-src");
    let mut h = start(vec![source.clone()]);
    let book = project(&h).workbooks().next().unwrap().id;
    let out = std::env::temp_dir().join(format!("sheaf-app-excel-export-{}", std::process::id())).join("out.xlsx");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    let _ = std::fs::remove_file(&out);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::ExportExcel(book), &ctx);
    settle(&mut h);
    for label in ["Scan 1", "Scan 2", "Notes", "Short name", "Long name", "Units", "Comments", "Save as...", "Close"] {
        assert!(has_label(&h, label), "{label} is missing");
    }
    h.get_by_role_and_label(egui::accesskit::Role::CheckBox, "Notes").click();
    settle(&mut h);
    h.state_mut().export_excel.as_mut().unwrap().chosen = Some(out.clone());
    click(&mut h, "Save as...");
    assert!(h.state().export_excel.is_none(), "the window closes after saving");
    assert!(h.state().status.starts_with("Wrote "), "{}", h.state().status);

    let back = read_workbook(&out, &ExcelOptions::default()).unwrap().workbook;
    let names: Vec<&str> = back.sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Scan 1", "Scan 2"], "the unticked sheet is left out");
    let original = project(&h).workbooks().next().unwrap();
    for (a, b) in original.sheets.iter().zip(&back.sheets) {
        assert_eq!(a.columns.len(), b.columns.len());
        for (x, y) in a.columns.iter().zip(&b.columns) {
            assert_eq!((&x.long_name, &x.units, &x.comments), (&y.long_name, &y.units, &y.comments));
            assert_eq!(x.numeric().unwrap(), y.numeric().unwrap());
        }
    }
}

#[test]
fn exporting_over_a_file_that_is_not_a_workbook_fails_and_leaves_it() {
    let source = excel_file("export-guard");
    let mut h = start(vec![source]);
    let book = project(&h).workbooks().next().unwrap().id;
    let out = std::env::temp_dir().join(format!("sheaf-app-excel-guard-{}.xlsx", std::process::id()));
    std::fs::write(&out, "precious notes").unwrap();
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::ExportExcel(book), &ctx);
    settle(&mut h);
    h.state_mut().export_excel.as_mut().unwrap().chosen = Some(out.clone());
    click(&mut h, "Save as...");
    assert_eq!(std::fs::read_to_string(&out).unwrap(), "precious notes");
    assert!(h.state().message.as_ref().is_some_and(|(title, _)| title == "Could not export to Excel"));
    assert!(h.state().export_excel.is_some(), "the window stays so that another name can be chosen");
}
