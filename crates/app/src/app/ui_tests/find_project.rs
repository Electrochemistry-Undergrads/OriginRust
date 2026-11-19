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
    h.state_mut().find_project.as_mut().unwrap().search(&project, &[]);
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

#[test]
fn comment_hits_open_the_sheet_and_follow_the_comments_checkbox() {
    let mut h = start(vec![data_file("find-project-comments")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = project(&h).workbooks().next().unwrap().id;
    h.state_mut().doc.apply("Comment on sheet", |p| {
        let mut control = sheaf_core::Worksheet::new("Control");
        control.comments = "Check the signal\nBaseline after calibration".into();
        p.workbook_mut(book).unwrap().sheets.push(control);
    });
    settle(&mut h);
    assert_eq!(project(&h).workbook(book).unwrap().active_sheet, 0);

    h.key_press(egui::Key::F3);
    settle(&mut h);
    h.state_mut().find_project.as_mut().unwrap().spec.text = "calibration".into();
    settle(&mut h);
    let find = By::new().role(egui::accesskit::Role::Button).label("Find");
    h.get(find.clone()).click();
    settle(&mut h);
    assert!(has_label(&h, "Found: 1"));
    let result = &h.state().find_project.as_ref().unwrap().hits.as_ref().unwrap()[0];
    assert_eq!((result.element, result.attribute), ("Sheet", "Comments"));
    let range = result.range.clone();
    click(&mut h, &range);
    assert_eq!(project(&h).workbook(book).unwrap().active_sheet, 1);

    click(&mut h, "Select...");
    h.get(By::new().role(egui::accesskit::Role::CheckBox).label("Comments")).click();
    settle(&mut h);
    h.get(find).click();
    settle(&mut h);
    assert!(has_label(&h, "Found: 0"));
    click(&mut h, "Cancel");
    h.key_press(egui::Key::F3);
    settle(&mut h);
    assert!(!h.state().find_project.as_ref().unwrap().spec.sheet_comments);
}

#[test]
fn within_uses_the_explorer_folder_and_preserves_the_selected_scope() {
    use crate::project_find::FindScope;
    use sheaf_core::project::Folder;

    let mut h = start(vec![data_file("find-within")]);
    let ctx = h.ctx.clone();
    h.state_mut().doc.apply("Arrange folders", |p| {
        let root_book = p.workbooks().next().unwrap().id;
        let books = [root_book, p.add_workbook(""), p.add_workbook("")];
        for id in books {
            p.workbook_mut(id).unwrap().long_name = "scope check".into();
        }
        p.root.folders = vec![Folder {
            name: "Trials".into(),
            windows: vec![books[1]],
            folders: vec![Folder { name: "Repeat".into(), windows: vec![books[2]], ..Folder::default() }],
            ..Folder::default()
        }];
    });
    h.state_mut().apply(Action::GoToFolder(vec![0]), &ctx);
    settle(&mut h);
    h.key_press(egui::Key::F3);
    settle(&mut h);
    let path = format!("/{}/Trials/", project(&h).root.name);
    assert!(has_label(&h, &path));
    h.state_mut().find_project.as_mut().unwrap().spec.text = "scope check".into();
    settle(&mut h);
    let find = By::new().role(egui::accesskit::Role::Button).label("Find");
    h.get(find.clone()).click();
    settle(&mut h);
    assert!(has_label(&h, "Found: 3"));

    h.get_by_value("Entire Project").click();
    settle(&mut h);
    click(&mut h, "Active Folder");
    h.get(find.clone()).click();
    settle(&mut h);
    assert!(has_label(&h, "Found: 1"));

    h.get_by_value("Active Folder").click();
    settle(&mut h);
    click(&mut h, "Active Folder and Subfolders");
    h.get(find).click();
    settle(&mut h);
    assert!(has_label(&h, "Found: 2"));
    click(&mut h, "Cancel");
    h.key_press(egui::Key::F3);
    settle(&mut h);
    assert_eq!(h.state().find_project.as_ref().unwrap().spec.within, FindScope::ActiveFolderAndSubfolders);
}
