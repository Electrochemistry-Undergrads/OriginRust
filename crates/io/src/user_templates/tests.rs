use super::*;
use sheaf_core::graph::{Dash, Font, SymbolShape, TextLabel};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, ColumnId, Designation};
use std::io::Write;

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("sheaf-io-usertemplates-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    /// A folder inside the scratch folder that does not exist yet (the store creates it).
    fn folder(&self) -> PathBuf {
        self.0.join("templates")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn book(p: &mut Project, long: &str, n: usize) -> Vec<ColumnId> {
    let b = p.add_workbook(long);
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, 2.0])).unwrap();
    (0..n).map(|i| p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![i as f64, 2.0, 5.0])).unwrap()).collect()
}

/// A template of a graph with a look that differs from every default.
fn styled_template(p: &mut Project, name: &str, ys: &[ColumnId]) -> UserTemplate {
    let g = p.plot_columns(Template::Line, ys).unwrap();
    let graph = p.graph_mut(g).unwrap();
    graph.page.width_pt = 400.0;
    graph.page.background = Color::rgb(250, 240, 230);
    let l = &mut graph.layers[0];
    l.x_axis.title = Some("Potential (V)".into());
    l.x_axis.min = Some(-1.0);
    l.legend.text = Some("\\l(1) %(1)".into());
    l.labels.push(TextLabel::new("%(1) at 5 mV/s", 0.2, 0.2, Font::new("Arial", 17.0)));
    l.plots[0].line.dash = Dash::Dash;
    l.plots[0].line.width_pt = 3.25;
    l.plots[0].symbol.shape = SymbolShape::TriangleUp;
    UserTemplate::from_graph(name, p.graph(g).unwrap()).unwrap()
}

fn sample(name: &str) -> UserTemplate {
    let mut p = Project::new();
    let ys = book(&mut p, "A", 2);
    styled_template(&mut p, name, &ys)
}

#[test]
fn a_saved_template_comes_back_with_every_look_setting() {
    let scratch = Scratch::new("round");
    let dir = scratch.folder();
    let t = sample("CV look");
    let saved = save(&dir, &t).unwrap();
    assert_eq!((saved.name.as_str(), saved.replaced), ("CV look", false));
    assert_eq!(saved.path, dir.join("CV look.sheaftemplate"));

    let back = load(&dir, "cv LOOK").unwrap();
    assert_eq!(back.name, "CV look");
    // The graph's own id and name are not part of the look.
    let mut want = t.graph.clone();
    want.name = "CV look".into();
    assert_eq!(back.graph, want);
    assert_eq!(load_file(&saved.path).unwrap(), back);
}

#[test]
fn the_file_holds_no_worksheet_data_and_no_workbook() {
    let scratch = Scratch::new("nodata");
    let dir = scratch.folder();
    let saved = save(&dir, &sample("look")).unwrap();
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&saved.path).unwrap()).unwrap();
    let mut members: Vec<String> =
        (0..zip.len()).map(|i| zip.by_index(i).unwrap().name().unwrap().to_string()).collect();
    members.sort();
    assert_eq!(members, ["manifest.json", "project.json"]);
    let mut manifest = String::new();
    std::io::Read::read_to_string(&mut zip.by_name("manifest.json").unwrap(), &mut manifest).unwrap();
    assert!(manifest.contains(r#""template": "graph""#), "{manifest}");
    assert!(!manifest.contains("window"));
    let project = project_file_text(&saved.path);
    assert!(!project.contains("\"sheets\""), "no workbook in the file");
    // Every plot reads a slot number, not a column of the source book.
    let back = load_file(&saved.path).unwrap();
    assert_eq!(back.graph.referenced_columns(), vec![ColumnId(1), ColumnId(2)]);
}

fn project_file_text(path: &Path) -> String {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut text = String::new();
    std::io::Read::read_to_string(&mut zip.by_name("project.json").unwrap(), &mut text).unwrap();
    text
}

#[test]
fn a_saved_template_plots_other_columns_as_the_unsaved_one_does() {
    let scratch = Scratch::new("plot");
    let dir = scratch.folder();
    let mut p = Project::new();
    let ys = book(&mut p, "A", 2);
    let t = styled_template(&mut p, "look", &ys);
    save(&dir, &t).unwrap();
    let back = load(&dir, "look").unwrap();

    let others = book(&mut p, "B", 3);
    let direct = p.plot_with_user_template(&t, &others).unwrap();
    let from_file = p.plot_with_user_template(&back, &others).unwrap();
    let (a, b) = (p.graph(direct).unwrap(), p.graph(from_file).unwrap());
    assert_eq!((&a.page, &a.layers), (&b.page, &b.layers));
    assert_eq!(b.layers[0].plots.len(), 3);
}

#[test]
fn a_double_y_template_keeps_both_layers_in_the_file() {
    let scratch = Scratch::new("double");
    let dir = scratch.folder();
    let mut p = Project::new();
    let ys = book(&mut p, "A", 2);
    let g = p.plot_columns(Template::DoubleY, &ys).unwrap();
    save(&dir, &UserTemplate::from_graph("two axes", p.graph(g).unwrap()).unwrap()).unwrap();
    let back = load(&dir, "two axes").unwrap();
    assert_eq!(back.slots(), vec![(0, 0), (1, 0)]);
    let others = book(&mut p, "B", 2);
    let h = p.plot_with_user_template(&back, &others).unwrap();
    let layers = &p.graph(h).unwrap().layers;
    assert_eq!((layers.len(), layers[0].plots.len(), layers[1].plots.len()), (2, 1, 1));
    assert!(layers[1].link.is_some());
}

#[test]
fn the_list_is_sorted_by_name_and_leaves_out_what_is_not_a_template() {
    let scratch = Scratch::new("list");
    let dir = scratch.folder();
    assert!(list(&dir).is_empty(), "a missing folder is an empty list");
    for name in ["beta", "Alpha", "gamma"] {
        save(&dir, &sample(name)).unwrap();
    }
    std::fs::write(dir.join("notes.txt"), "hello").unwrap();
    std::fs::write(dir.join("broken.sheaftemplate"), "not an archive").unwrap();
    std::fs::create_dir(dir.join("folder.sheaftemplate")).unwrap();
    // A project and a window file with the template extension are not templates.
    let mut p = Project::new();
    let ys = book(&mut p, "A", 1);
    project_file::save(&p, &dir.join("project.sheaftemplate")).unwrap();
    let g = p.plot_columns(Template::Line, &ys).unwrap();
    crate::window_file::save_window(&p, g, &dir.join("window.sheaftemplate")).unwrap();

    assert_eq!(names(&dir), ["Alpha", "beta", "gamma"]);
    assert!(exists(&dir, "ALPHA") && !exists(&dir, "delta") && !exists(&dir, "project"));
    assert_eq!(find(&dir, " beta ").unwrap().path, dir.join("beta.sheaftemplate"));
}

#[test]
fn a_template_file_is_never_taken_for_a_project_or_a_window_file() {
    let scratch = Scratch::new("never");
    let dir = scratch.folder();
    let saved = save(&dir, &sample("look")).unwrap();
    let mut p = Project::new();
    let ys = book(&mut p, "A", 1);
    let project = scratch.0.join("p.sheaf");
    project_file::save(&p, &project).unwrap();
    let g = p.plot_columns(Template::Line, &ys).unwrap();
    let window = scratch.0.join("w.sheafgraph");
    crate::window_file::save_window(&p, g, &window).unwrap();

    assert!(matches!(project_file::load(&saved.path), Err(IoError::IsATemplateFile { .. })));
    assert!(matches!(crate::open_project(&saved.path), Err(IoError::IsATemplateFile { .. })));
    assert!(matches!(crate::window_file::load_window(&saved.path), Err(IoError::NotAWindowFile { .. })));
    assert!(matches!(load_file(&project), Err(IoError::NotATemplateFile { .. })));
    assert!(matches!(load_file(&window), Err(IoError::NotATemplateFile { .. })));
    assert!(is_template_file(&saved.path) && !is_template_file(&project) && !is_template_file(&window));
    assert!(!crate::window_file::is_window_file(&saved.path));

    // Renamed to .sheaf it is still a template and still no project.
    let renamed = scratch.0.join("renamed.sheaf");
    std::fs::copy(&saved.path, &renamed).unwrap();
    assert!(is_template_file(&renamed) && !crate::is_project_file(&renamed));
    assert!(load_file(&renamed).is_ok());
}

#[test]
fn names_are_cleaned_to_fit_a_file_name() {
    assert_eq!(clean_name("  CV \t look \n").unwrap(), "CV look");
    assert_eq!(clean_name("a/b\\c:d*e?f\"g<h>i|j").unwrap(), "a_b_c_d_e_f_g_h_i_j");
    assert_eq!(clean_name("..hidden.").unwrap(), "hidden");
    assert_eq!(clean_name("con").unwrap(), "con_");
    assert_eq!(clean_name("LPT3.txt").unwrap(), "LPT3.txt_");
    assert_eq!(clean_name("COM10").unwrap(), "COM10");
    assert_eq!(clean_name(&"x".repeat(100)).unwrap().chars().count(), MAX_NAME_CHARS);
    assert_eq!(clean_name("\u{3b1}\u{3b2} \u{e9}").unwrap(), "\u{3b1}\u{3b2} \u{e9}");
    for empty in ["", "   ", "\t\n", "...", " . "] {
        assert_eq!(clean_name(empty), Err(NameError::Empty), "{empty:?}");
    }
    assert!(matches!(save(Path::new("nowhere"), &sample("  ")), Err(TemplateError::Name(NameError::Empty))));
}

#[test]
fn saving_under_a_name_that_exists_replaces_it_and_takes_the_new_spelling() {
    let scratch = Scratch::new("replace");
    let dir = scratch.folder();
    save(&dir, &sample("Look")).unwrap();
    let again = save(&dir, &sample("LOOK")).unwrap();
    assert!(again.replaced);
    assert_eq!(names(&dir), ["LOOK"]);
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1, "no second file and no temporary file");
}

#[test]
fn a_template_is_renamed_and_deleted_by_name_without_regard_to_case() {
    let scratch = Scratch::new("manage");
    let dir = scratch.folder();
    save(&dir, &sample("one")).unwrap();
    save(&dir, &sample("two")).unwrap();
    assert_eq!(rename(&dir, "ONE", "uno").unwrap(), "uno");
    assert_eq!(names(&dir), ["two", "uno"]);
    assert_eq!(load(&dir, "uno").unwrap().name, "uno");
    // A case-only change is a rename, an existing name is refused and nothing changes.
    assert_eq!(rename(&dir, "two", "TWO").unwrap(), "TWO");
    assert!(matches!(rename(&dir, "uno", "two"), Err(TemplateError::Exists { .. })));
    assert!(matches!(rename(&dir, "gone", "x"), Err(TemplateError::Unknown { .. })));
    assert!(matches!(rename(&dir, "uno", "  "), Err(TemplateError::Name(_))));
    assert_eq!(names(&dir), ["TWO", "uno"]);

    assert!(delete(&dir, "UNO").unwrap());
    assert!(!delete(&dir, "uno").unwrap());
    assert_eq!(names(&dir), ["TWO"]);
}

#[test]
fn an_unknown_name_is_an_error_that_lists_the_saved_templates() {
    let scratch = Scratch::new("unknown");
    let dir = scratch.folder();
    let none = load(&dir, "x").unwrap_err().to_string();
    assert!(none.contains("'x'") && none.contains("no templates are saved"), "{none}");
    save(&dir, &sample("alpha")).unwrap();
    save(&dir, &sample("beta")).unwrap();
    let e = load(&dir, "gamma").unwrap_err();
    assert!(matches!(&e, TemplateError::Unknown { available, .. } if available == &["alpha", "beta"]), "{e}");
    assert!(e.to_string().contains("alpha, beta"));
}

/// An archive with the given manifest and project descriptions.
fn archive(path: &Path, manifest: &str, project: Option<&str>) {
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("manifest.json", options).unwrap();
    zip.write_all(manifest.as_bytes()).unwrap();
    if let Some(project) = project {
        zip.start_file("project.json", options).unwrap();
        zip.write_all(project.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn a_damaged_or_newer_file_is_reported_for_itself_and_hides_no_other_template() {
    let scratch = Scratch::new("damaged");
    let dir = scratch.folder();
    save(&dir, &sample("good")).unwrap();
    let empty = serde_json::to_string(&Project::new()).unwrap();
    let manifest = |version: u32, marker: &str| {
        format!(r#"{{"format":"sheaf-project","version":{version},"template":"{marker}"}}"#)
    };
    archive(&dir.join("nograph.sheaftemplate"), &manifest(1, "graph"), Some(&empty));
    archive(&dir.join("chart.sheaftemplate"), &manifest(1, "chart"), Some(&empty));
    archive(&dir.join("newer.sheaftemplate"), &manifest(99, "graph"), Some(&empty));
    archive(&dir.join("noproject.sheaftemplate"), &manifest(1, "graph"), None);
    archive(&dir.join("badjson.sheaftemplate"), &manifest(1, "graph"), Some("{ not json"));

    assert!(load(&dir, "good").is_ok());
    assert!(matches!(load(&dir, "nograph"), Err(TemplateError::Io(IoError::Corrupt { .. }))));
    let chart = load(&dir, "chart").unwrap_err().to_string();
    assert!(chart.contains("chart"), "{chart}");
    assert!(matches!(load(&dir, "newer"), Err(TemplateError::Io(IoError::UnsupportedVersion { .. }))));
    assert!(matches!(load(&dir, "noproject"), Err(TemplateError::Io(IoError::Corrupt { .. }))));
    assert!(matches!(load(&dir, "badjson"), Err(TemplateError::Io(IoError::Json { .. }))));
    assert!(names(&dir).contains(&"good".to_string()), "the good one is still listed");
}

#[test]
fn a_template_written_by_a_later_version_with_more_fields_still_loads() {
    let scratch = Scratch::new("future");
    let dir = scratch.folder();
    let saved = save(&dir, &sample("look")).unwrap();
    // Add an unknown field to the manifest and to the graph, as a later version might.
    let project: serde_json::Value = serde_json::from_str(&project_file_text(&saved.path)).unwrap();
    let mut project = project;
    project["windows"][0]["future_field"] = serde_json::json!(true);
    let manifest = r#"{"format":"sheaf-project","version":1,"template":"graph","future":1}"#;
    archive(&dir.join("later.sheaftemplate"), manifest, Some(&project.to_string()));
    let t = load(&dir, "later").unwrap();
    assert_eq!(t.plot_count(), 2);
}

#[test]
fn the_default_folder_is_templates_beside_the_autosave_settings() {
    let settings = recovery::default_settings_path();
    let dir = default_dir();
    assert_eq!(dir, settings.and_then(|p| p.parent().map(|d| d.join("templates"))));
}

fn book_template(name: &str) -> BookTemplate {
    let mut p = Project::new();
    let b = p.add_workbook("Run log");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, 2.0])).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![3.0, 4.0, 5.0])).unwrap();
    let c = p.column_mut(y).unwrap();
    c.long_name = "Current".into();
    c.units = "uA".into();
    c.formula = Some("col(A)^2".into());
    p.workbook_mut(b).unwrap().sheets.push(sheaf_core::Worksheet::new("Notes"));
    BookTemplate::from_workbook(name, p.workbook(b).unwrap())
}

#[test]
fn a_workbook_template_round_trips_beside_graph_templates_with_its_own_names() {
    let scratch = Scratch::new("books");
    let dir = scratch.folder();
    save(&dir, &sample("Log")).unwrap();
    let saved = save_book(&dir, &book_template("Log")).unwrap();
    assert_eq!(saved.path.extension().unwrap(), BOOK_EXTENSION);
    assert!(!saved.replaced, "a graph template of the same name is another template");
    assert_eq!(names(&dir), ["Log"]);
    assert_eq!(names_of(&dir, TemplateKind::Workbook), ["Log"]);
    assert_eq!(kind_of_file(&saved.path), Some(TemplateKind::Workbook));
    assert!(!is_template_file(&saved.path), "not a graph template");

    let back = load_book(&dir, "log").unwrap();
    assert_eq!(back.name, "Log");
    assert_eq!(back.book.sheets.len(), 2);
    let y = &back.book.sheets[0].columns[1];
    assert_eq!((y.long_name.as_str(), y.units.as_str()), ("Current", "uA"));
    assert_eq!(y.formula.as_deref(), Some("col(A)^2"));
    assert!(back.book.sheets[0].columns.iter().all(|c| c.is_empty()), "no values in the file");

    // The graph template of that name is still there and still a graph.
    assert!(load(&dir, "Log").is_ok());
    // A workbook file is not read as a graph template, and the reverse.
    let wrong = load_file(&saved.path).unwrap_err().to_string();
    assert!(wrong.contains("workbook template"), "{wrong}");
    assert!(load_book_file(&find(&dir, "Log").unwrap().path).is_err());
}

#[test]
fn workbook_templates_are_replaced_renamed_and_deleted_by_kind() {
    let scratch = Scratch::new("books-manage");
    let dir = scratch.folder();
    save(&dir, &sample("Graph look")).unwrap();
    save_book(&dir, &book_template("first")).unwrap();
    assert!(save_book(&dir, &book_template("FIRST")).unwrap().replaced);
    assert_eq!(names_of(&dir, TemplateKind::Workbook), ["FIRST"]);
    save_book(&dir, &book_template("Other")).unwrap();
    assert!(matches!(rename_of(&dir, "first", "other", TemplateKind::Workbook), Err(TemplateError::Exists { .. })));
    // A graph template's name does not clash with a workbook template.
    assert_eq!(rename_of(&dir, "first", "Graph look", TemplateKind::Workbook).unwrap(), "Graph look");
    assert_eq!(names_of(&dir, TemplateKind::Workbook), ["Graph look", "Other"]);
    assert!(delete_of(&dir, "graph LOOK", TemplateKind::Workbook).unwrap());
    assert_eq!(names(&dir), ["Graph look"], "the graph template stays");
    assert_eq!(names_of(&dir, TemplateKind::Workbook), ["Other"]);
    let err = load_book(&dir, "missing").unwrap_err().to_string();
    assert!(err.contains("Other"), "{err}");
}
