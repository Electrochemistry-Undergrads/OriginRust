//! File > New > Layout, the layout page window and Layout > Add Graph... (T237).

use super::*;

#[test]
fn a_layout_page_is_made_and_shows_an_added_graph() {
    let mut h = start(vec![data_file("layout-page")]);
    let ctx = h.ctx.clone();
    let ctx_graph = project(&h).graphs().next().map(|g| g.id);
    if ctx_graph.is_none() {
        let book = project(&h).workbooks().next().unwrap().id;
        let cols: Vec<_> = project(&h).workbook(book).unwrap().sheet().columns.iter().map(|c| c.id).collect();
        h.state_mut().apply(
            Action::PlotColumns { book, columns: vec![cols[1]], template: sheaf_core::template::Template::Line },
            &ctx,
        );
        settle(&mut h);
    }
    let graph = project(&h).graphs().next().unwrap().id;
    menu_path(&mut h, &["File", "New", "Layout"]);
    let layouts: Vec<_> = project(&h).layouts().map(|l| (l.id, l.name.clone())).collect();
    assert_eq!(layouts.len(), 1);
    let (id, name) = layouts[0].clone();
    assert_eq!(name, "Layout1");
    assert_eq!(h.state().doc.undo_label(), Some("New layout"));
    assert_eq!(h.state().menu_context().kind, menus::WindowKind::Layout, "the new page is active");
    // Layout > Add Graph... opens its window; OK adds the graph's picture.
    let action = menus::resolve(&menus::Cmd::LayoutAddGraph, &h.state().menu_context()).unwrap();
    h.state_mut().apply(action, &ctx);
    settle(&mut h);
    assert!(h.state().layout_add_graph.is_some());
    click(&mut h, "OK");
    let page = project(&h).layout(id).unwrap().clone();
    assert_eq!(page.graphs().collect::<Vec<_>>(), [graph]);
    assert_eq!(h.state().doc.undo_label(), Some("Add graph to layout"));
    // The page draws the picture (a texture for the graph) without trouble.
    settle(&mut h);
    assert!(h.state().layouts.contains_key(&id), "the layout window was drawn");
    // The Standard toolbar's New Layout makes another page.
    let action = menus::resolve(&menus::Cmd::NewLayout, &h.state().menu_context()).unwrap();
    h.state_mut().apply(action, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).layouts().count(), 2);
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).layouts().count(), 1);
}
