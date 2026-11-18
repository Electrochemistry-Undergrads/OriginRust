//! View > View Windows, Window > Folders and folder Properties (T100, workbook-model.md "Folders").

use super::*;

#[test]
fn the_workspace_shows_the_active_folders_windows_and_follows_the_folder() {
    let mut h = start(vec![data_file("view-windows")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let ctx = h.ctx.clone();
    // A folder "Sub" holding a new workbook.
    h.state_mut().apply(Action::NewFolder(Vec::new()), &ctx);
    settle(&mut h);
    h.state_mut().apply(Action::NewWorkbook, &ctx);
    settle(&mut h);
    let other = project(&h).workbooks().map(|b| b.id).find(|b| *b != book).unwrap();
    h.state_mut().apply(Action::MoveWindow { window: other, folder: vec![0] }, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).view_windows, sheaf_core::project::ViewWindows::ActiveFolder, "Origin's default");
    // In the root folder the workbook of Sub is not drawn and not listed.
    h.state_mut().apply(Action::GoToFolder(Vec::new()), &ctx);
    settle(&mut h);
    assert_eq!(h.state().active, Some(book), "the root's window comes forward");
    let menu = h.state().menu_context();
    assert!(menu.windows.iter().all(|(id, _)| *id != other));
    assert_eq!(menu.folders.len(), 2, "{:?}", menu.folders);
    // Window > Folders > Sub: its workbook comes forward.
    h.state_mut().apply(Action::GoToFolder(vec![0]), &ctx);
    settle(&mut h);
    assert_eq!(h.state().active, Some(other));
    let menu = h.state().menu_context();
    assert!(menu.windows.iter().all(|(id, _)| *id != book));
    // None: every window shows again.
    h.state_mut().apply(Action::ViewWindows(sheaf_core::project::ViewWindows::None), &ctx);
    settle(&mut h);
    assert!(h.state().menu_context().windows.iter().any(|(id, _)| *id == book));
}

#[test]
fn folder_properties_rename_and_keep_comments() {
    let mut h = start(vec![]);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::NewFolder(Vec::new()), &ctx);
    settle(&mut h);
    h.state_mut().apply(Action::FolderProperties(vec![0]), &ctx);
    settle(&mut h);
    {
        let d = h.state_mut().folder_props.as_mut().expect("the window opened");
        d.name = "Runs".into();
        d.comments = "first week".into();
    }
    settle(&mut h);
    click(&mut h, "OK");
    let f = &project(&h).root.folders[0];
    assert_eq!((f.name.as_str(), f.comments.as_str()), ("Runs", "first week"));
    assert_eq!(h.state().doc.undo_label(), Some("Folder properties"));
}
