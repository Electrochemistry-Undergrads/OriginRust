use super::*;
use sheaf_plot::scene::ObjectKind;

fn rpc(a: &mut Assistant, h: &mut Harness<'static, SheafApp>, method: &str, params: Value) -> Value {
    a.next += 1;
    let id = a.next;
    a.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
    let reply = a.wait(h, id);
    assert!(reply.get("error").is_none(), "{reply}");
    reply["result"].clone()
}

fn context(a: &mut Assistant, h: &mut Harness<'static, SheafApp>) -> Value {
    let result = rpc(a, h, "resources/read", json!({"uri": "sheaf://app"}));
    assert_eq!(result["contents"][0]["uri"], "sheaf://app");
    assert_eq!(result["contents"][0]["mimeType"], "application/json");
    serde_json::from_str(result["contents"][0]["text"].as_str().unwrap()).unwrap()
}

#[test]
fn app_context_reads_live_worksheet_selections_without_a_document_change() {
    let dirs = Dirs::new("ctxcells");
    let mut h = start_link(&dirs, false);
    tick_link(&mut h);
    let mut a = Assistant::start(&dirs);
    a.call(&mut h, "import_files", json!({"files": [dirs.csv()]}));
    settle(&mut h);
    let book = project(&h).workbooks().next().unwrap().id;
    let sheet = project(&h).workbook(book).unwrap().sheet().clone();
    h.state_mut().focus(book);
    h.state_mut().books.entry(book).or_default().select_cells(&sheet, (2, 3), (1, 1));
    settle(&mut h);
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    let undo = h.state().doc.undo_label().map(str::to_string);
    let listed = rpc(&mut a, &mut h, "resources/list", json!({}));
    assert!(listed["resources"].as_array().unwrap().iter().any(|r| r["uri"] == "sheaf://app"));
    let c = context(&mut a, &mut h);
    assert_eq!(c["active_window"]["kind"], "workbook");
    assert_eq!(c["active_window"]["name"], before.workbook(book).unwrap().name);
    assert_eq!(c["worksheet"]["name"], sheet.name);
    assert_eq!(c["worksheet"]["index"], 1);
    assert_eq!(c["selection"]["kind"], "cells");
    assert_eq!(c["selection"]["columns"], json!(["B", "C"]));
    assert_eq!(c["selection"]["rows"], json!({"from": 2, "to": 4}));
    assert_eq!(c["selection"]["active_cell"], json!({"column": "C", "row": 4}));

    let ui = h.state_mut().books.get_mut(&book).unwrap();
    ui.sel = Sel::Columns;
    ui.selected = vec![sheet.columns[2].id, sheet.columns[0].id];
    let c = context(&mut a, &mut h);
    assert_eq!(c["selection"]["kind"], "columns");
    assert_eq!(c["selection"]["columns"], json!(["A", "C"]));
    assert!(c["selection"]["rows"].is_null());

    h.state_mut().books.get_mut(&book).unwrap().sel = Sel::Rows { anchor: 2, end: 1 };
    let c = context(&mut a, &mut h);
    assert_eq!(c["selection"]["kind"], "rows");
    assert_eq!(c["selection"]["rows"], json!({"from": 2, "to": 3}));
    assert_eq!(c["selection"]["columns"], json!(["A", "B", "C"]));

    h.state_mut().books.get_mut(&book).unwrap().sel =
        Sel::Label { column: sheet.columns[1].id, row: LabelRow::LongName };
    let c = context(&mut a, &mut h);
    assert_eq!(c["selection"]["kind"], "label");
    assert_eq!(c["selection"]["label_row"], json!({"kind": "long_name"}));
    assert_eq!(c["selection"]["columns"], json!(["B"]));
    assert!(c["selection"]["rows"].is_null());

    h.state_mut().books.get_mut(&book).unwrap().sel = Sel::All;
    let c = context(&mut a, &mut h);
    assert_eq!(c["selection"]["kind"], "all");
    assert!(c["selection"]["rows"].is_null());
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    assert_eq!(h.state().doc.undo_label(), undo.as_deref());
    a.end();
}

#[test]
fn app_context_reads_under_dialogs_and_edits_without_committing_pending_text() {
    let dirs = Dirs::new("ctxbusy");
    let mut h = start_link(&dirs, false);
    tick_link(&mut h);
    let mut a = Assistant::start(&dirs);
    h.state_mut().about = true;
    settle(&mut h);
    let c = context(&mut a, &mut h);
    assert!(c["active_window"].is_null() && c["selection"].is_null());
    assert_eq!(c["dialog_open"], true);
    assert!(!h.state().doc.dirty && h.state().doc.undo_label().is_none());
    h.state_mut().about = false;
    a.call(&mut h, "import_files", json!({"files": [dirs.csv()]}));
    settle(&mut h);
    let book = project(&h).workbooks().next().unwrap().id;
    let column = project(&h).workbook(book).unwrap().sheet().columns[0].id;
    h.state_mut().focus(book);
    h.state_mut().books.entry(book).or_default().editing = Some((column, 0, "uncommitted-private-text".into()));
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    let undo = h.state().doc.undo_label().map(str::to_string);
    let c = context(&mut a, &mut h);
    assert_eq!(c["editing"], true);
    assert!(!c.to_string().contains("uncommitted-private-text"));
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    assert_eq!(h.state().doc.undo_label(), undo.as_deref());
    h.state_mut().books.get_mut(&book).unwrap().editing = None;
    a.end();
}

#[test]
fn app_context_reports_graph_objects_and_pinned_reader_rows_in_one_based_indices() {
    let dirs = Dirs::new("ctxgraph");
    let mut h = start_link(&dirs, false);
    tick_link(&mut h);
    let mut a = Assistant::start(&dirs);
    a.call(&mut h, "import_files", json!({"files": [dirs.csv()]}));
    let plotted = a.call(&mut h, "plot", json!({"y": ["B"]}));
    assert_eq!(plotted["isError"], false, "{plotted}");
    settle(&mut h);
    let graph = project(&h).graphs().next().unwrap().id;
    h.state_mut().focus(graph);
    let state = h.state_mut().graphs.entry(graph).or_default();
    state.active_layer = 0;
    state.selected = Some((0, ObjectKind::Plot(0)));
    state.reader = true;
    state.reader_point = Some(crate::reader::Point { layer: 0, plot: 0, row: 2 });
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    let c = context(&mut a, &mut h);
    assert_eq!(c["active_window"]["kind"], "graph");
    assert!(c["worksheet"].is_null());
    assert_eq!(c["active_layer"], 1);
    assert_eq!(c["selection"], json!({"kind": "plot", "layer": 1, "index": 1}));
    assert_eq!(c["data_reader"], json!({"enabled": true, "point": {"layer": 1, "plot": 1, "row": 3}}));
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    a.end();
}
