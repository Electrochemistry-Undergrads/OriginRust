//! Graph > New Table and editing a table (T136, graphing "Tables").

use super::*;

#[test]
fn a_new_table_is_made_and_edited_cell_by_cell() {
    let mut h = start(vec![data_file("graph-table")]);
    let ctx = h.ctx.clone();
    let graph = project(&h).graphs().next().unwrap().id;
    let labels = |h: &Harness<'static, SheafApp>| project(h).graph(graph).unwrap().layers[0].labels.clone();
    let before = labels(&h).len();
    h.state_mut().apply(Action::NewTable { graph, layer: 0 }, &ctx);
    settle(&mut h);
    {
        let e = h.state_mut().table_editor.as_mut().unwrap();
        assert_eq!((e.cells.len(), e.cells[0].len()), (8, 5), "Origin 2016's 8 rows of 5 columns (T395)");
        e.cells[0][0] = "Sample".into();
        e.cells[1][0] = "A".into();
        e.cells[1][1] = "1.5".into();
    }
    h.get_by_label("Add Column").click();
    settle(&mut h);
    h.get_by_label("OK").click();
    settle(&mut h);
    let all = labels(&h);
    assert_eq!(all.len(), before + 1);
    let t = all.last().unwrap();
    assert!(t.table);
    assert_eq!(t.cells()[1], ["A", "1.5", "", "", "", ""]);
    // A double-click on the table (EditText) opens the cell editor, not the text editor.
    let k = all.len() - 1;
    h.state_mut().apply(Action::EditText { graph, layer: 0, label: k }, &ctx);
    settle(&mut h);
    h.state_mut().table_editor.as_mut().unwrap().cells[2][3] = "x".into();
    h.get_by_label("OK").click();
    settle(&mut h);
    assert_eq!(labels(&h)[k].cells()[2][3], "x");
    // One undo step each.
    h.state_mut().doc.undo();
    assert_eq!(labels(&h)[k].cells()[2][3], "");
}
