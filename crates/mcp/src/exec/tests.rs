use super::*;
use serde_json::json;
use sheaf_core::column::{ColumnData, Designation};

fn project() -> Project {
    let mut p = Project::new();
    let book = p.add_workbook("");
    for (i, name) in ["A", "B", "C", "D", "E"].iter().enumerate() {
        let designation = if i == 0 { Designation::X } else { Designation::Y };
        let id = p.add_column(book, 0, designation, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        p.column_mut(id).unwrap().short_name = (*name).to_string();
    }
    let id = p.workbooks().next().unwrap().sheets[0].columns[1].id;
    p.column_mut(id).unwrap().long_name = "Current".to_string();
    p
}

#[test]
fn undo_names_follow_the_table() {
    let p = project();
    let cases = [
        ("plot", json!({"y": ["B"]}), "AI: plot Book1 B(Y)"),
        ("plot", json!({"book": "book1", "y": ["B", "c"], "y2": ["D"]}), "AI: plot Book1 B(Y) C(Y) D(Y)"),
        ("plot", json!({"y": ["B", "C", "D", "E", "A"]}), "AI: plot Book1 B(Y) C(Y) D(Y) +2"),
        ("plot", json!({"graph": "Graph1"}), "AI: plot Graph1"),
        ("plot", json!({}), "AI: plot Book1"),
        ("plot", json!({"y": ["Current"]}), "AI: plot Book1 B(Y)"),
        ("plot", json!({"y": ["Zed"]}), "AI: plot Book1 Zed"),
        ("plot", json!({"book": "Nope", "y": ["B"]}), "AI: plot Nope B"),
        ("fit", json!({"model": "Gauss", "y": ["B"]}), "AI: fit Gauss Book1 B(Y)"),
        ("fit", json!({"options": {"model": "linear", "y": "C"}}), "AI: fit linear Book1 C(Y)"),
        ("fit", json!({}), "AI: fit Book1"),
        ("simulate_curve", json!({"function": "ExpDec1", "points": 5}), "AI: simulate_curve ExpDec1 Book1"),
        ("simulate_curve", json!({}), "AI: simulate_curve Gauss Book1"),
        ("analyze", json!({"tool": "integrate", "columns": ["B"]}), "AI: analyze integrate Book1 B(Y)"),
        ("set_column_values", json!({"column": "C", "formula": "A*2"}), "AI: set_column_values Book1 C = A*2"),
        ("sort_sheet", json!({"key": "B"}), "AI: sort_sheet Book1 B(Y)"),
        ("filter_sheet", json!({"condition": "B>1"}), "AI: filter_sheet Book1 B>1"),
        ("import_files", json!({"files": ["/data/run1.csv"]}), "AI: import_files run1.csv"),
        ("import_files", json!({"files": ["run1.csv", "run2.csv", "run3.csv"]}), "AI: import_files run1.csv +2"),
        ("import_files", json!({"files": ["C:\\data\\run1.csv", "x"]}), "AI: import_files run1.csv +1"),
        ("import_files", json!({}), "AI: import_files"),
    ];
    for (tool, args, want) in cases {
        assert_eq!(undo_label(tool, &args, &p), want, "{tool} {args}");
    }
}

#[test]
fn undo_names_of_an_empty_project_and_odd_arguments_do_not_fail() {
    let empty = Project::new();
    assert_eq!(undo_label("plot", &json!({"y": ["B"]}), &empty), "AI: plot B");
    assert_eq!(undo_label("sort_sheet", &json!(null), &empty), "AI: sort_sheet");
    assert_eq!(undo_label("fit", &json!({"y": 5, "model": 7, "book": []}), &empty), "AI: fit");
}

#[test]
fn undo_names_are_cleaned_and_cut_to_sixty_characters() {
    let p = project();
    let evil = json!({"column": "C", "formula": "A\u{202E}*\u{7}2\nfoo"});
    assert_eq!(undo_label("set_column_values", &evil, &p), "AI: set_column_values Book1 C = A*2 foo");
    let long = json!({"files": [format!("{}.csv", "x".repeat(200))]});
    let label = undo_label("import_files", &long, &p);
    assert_eq!(label.chars().count(), LABEL_CHARS);
    assert!(label.ends_with("..."), "{label}");
    assert!(label.starts_with("AI: import_files xxx"), "{label}");
}

#[test]
fn a_panic_in_a_mutating_tool_leaves_the_project_the_session_and_the_history_as_before() {
    let mut doc = Document::new(project());
    let before = doc.project().clone();
    let mut session = Session::default();
    session.suggested.insert("book1".into(), Template::Line);
    let policy = Policy::headless(Roots::default());
    let r = run_with(&mut doc, &mut session, &policy, "sort_sheet", &json!({}), |ctx, _, _| {
        ctx.project.add_workbook("Extra");
        ctx.session.suggested.clear();
        ctx.session.path = Some(PathBuf::from("changed"));
        panic!("injected");
    });
    let e = r.err().unwrap();
    assert!(e.message.starts_with("internal error"), "{}", e.message);
    assert_eq!(doc.project(), &before);
    assert!(doc.undo_label().is_none());
    assert!(!doc.dirty);
    assert_eq!(session.suggested.len(), 1);
    assert!(session.path.is_none());
}

#[test]
fn a_panic_in_a_tool_that_only_reads_changes_nothing_either() {
    let mut doc = Document::new(project());
    let before = doc.project().clone();
    let mut session = Session::default();
    let policy = Policy::headless(Roots::default());
    let r = run_with(&mut doc, &mut session, &policy, "list_contents", &json!({}), |ctx, _, _| {
        ctx.project.add_workbook("Extra");
        panic!("injected");
    });
    assert!(r.is_err());
    assert_eq!(doc.project(), &before);
}

#[test]
fn a_failing_call_restores_the_session_but_still_counts_as_a_change() {
    let mut doc = Document::new(project());
    let mut session = Session::default();
    let policy = Policy::headless(Roots::default());
    let r = run_with(&mut doc, &mut session, &policy, "plot", &json!({}), |ctx, _, _| {
        ctx.session.foreign.push(PathBuf::from("x.opj"));
        Err(ToolError::new("nope"))
    });
    assert!(r.is_err());
    assert!(session.foreign.is_empty());
    assert_eq!(session.mutating_calls(), 1);
    assert!(doc.undo_label().is_none());
}

#[test]
fn the_cap_refuses_the_call_with_the_owners_message_and_does_not_count_it() {
    let mut doc = Document::new(project());
    let mut session = Session::default();
    let policy = Policy::attached(Roots::default());
    assert_eq!(policy.change_cap, Some(1000));
    session.mutating_calls = 1000;
    let e = run_on_document(&mut doc, &mut session, &policy, "sort_sheet", &json!({"key": "B"})).err().unwrap();
    assert_eq!(
        e.message,
        "this connection has made 1000 changes; ask the owner to turn the link off and on, or restart the assistant"
    );
    assert_eq!(session.mutating_calls(), 1000);
    // A tool that does not change the project is not limited.
    assert!(run_on_document(&mut doc, &mut session, &policy, "list_contents", &json!({})).is_ok());
}
