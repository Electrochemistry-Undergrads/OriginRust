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

#[test]
fn layout_items_are_moved_resized_and_deleted_with_the_mouse() {
    let mut h = start(Vec::new());
    let ctx = h.ctx.clone();
    let mut id = sheaf_core::WindowId(0);
    h.state_mut().doc.apply("Layout", |p| {
        let g = p.add_graph(sheaf_core::template::Template::Line);
        id = p.add_layout();
        p.layout_mut(id).unwrap().add_graph(g, 4.0 / 3.0);
    });
    settle(&mut h);
    h.state_mut().focus(id);
    settle(&mut h);
    let page = h.state().layouts.get(&id).and_then(|l| l.page_on_screen).expect("the page was drawn");
    let item = |h: &Harness<'static, SheafApp>| project(h).layout(id).unwrap().items[0];
    let at = |r: egui::Rect, fx: f64, fy: f64| {
        egui::pos2(r.left() + fx as f32 * r.width(), r.top() + fy as f32 * r.height())
    };
    // Drag from the middle of the item: it moves by the same page fraction.
    let before = item(&h);
    let from = at(page, before.x + before.w / 2.0, before.y + before.h / 2.0);
    let to = from + egui::vec2(page.width() * 0.1, page.height() * 0.05);
    drag(&mut h, from, to);
    let moved = item(&h);
    assert!((moved.x - before.x - 0.1).abs() < 0.01 && (moved.y - before.y - 0.05).abs() < 0.01, "{moved:?}");
    assert_eq!((moved.w, moved.h), (before.w, before.h));
    assert_eq!(h.state().doc.undo_label(), Some("Move layout item"));
    // The selected item's corner handle resizes it.
    let corner = at(page, moved.x + moved.w, moved.y + moved.h);
    drag(&mut h, corner, corner + egui::vec2(page.width() * 0.05, page.height() * 0.05));
    let resized = item(&h);
    assert!((resized.w - moved.w - 0.05).abs() < 0.01 && resized.x == moved.x, "{resized:?}");
    assert_eq!(h.state().doc.undo_label(), Some("Resize layout item"));
    // Delete removes the selected item, one undo step.
    let middle = at(page, resized.x + resized.w / 2.0, resized.y + resized.h / 2.0);
    h.event(egui::Event::PointerMoved(middle));
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert!(project(&h).layout(id).unwrap().items.is_empty());
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).layout(id).unwrap().items.len(), 1);
}
