//! Edit > Find in Project (F3, T95, workbook-model.md "Find in Project").

use super::*;

#[test]
fn f3_opens_find_and_a_hit_selects_the_column() {
    let mut h = start(vec![data_file("find-project")]);
    show_window(&mut h, "Book1 - sample.csv");
    let c = column_id(&h, "C");
    let book = h.state().books.keys().next().copied().unwrap();
    h.state_mut().doc.apply("Label", |p| p.column_mut(c).unwrap().long_name = "Current density".into());
    // Select another column first, then search.
    click_text(&mut h, "B(Y1)");
    h.key_press(egui::Key::F3);
    settle(&mut h);
    assert!(h.state().find_project.is_some(), "F3 opens Find");
    h.state_mut().find_project.as_mut().unwrap().spec.text = "density".into();
    // The Find button (its label is also the window's title): run the same search.
    let project = h.state().doc.project().clone();
    h.state_mut().find_project.as_mut().unwrap().search(&project);
    settle(&mut h);
    let hits = h.state().find_project.as_ref().unwrap().hits.clone().unwrap();
    // The column's long name, and the legend of the graph that opening the file made.
    let column = hits.iter().find(|h| h.element == "Column").expect("the column is found");
    assert_eq!(column.attribute, "Long Name");
    assert!(hits.iter().all(|h| h.element == "Column" || h.attribute == "Legend"), "{hits:?}");
    assert!(h.query_by_label(&format!("Found: {}", hits.len())).is_some());
    let range = column.range.clone();
    click(&mut h, &range);
    assert_eq!(h.state().books.get(&book).unwrap().selected, [c], "the hit's column is selected");
    assert!(h.state().find_project.is_some(), "Find stays open");
}
