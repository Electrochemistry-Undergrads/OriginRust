//! A drawn object's right-click Cut and Copy and the page's Paste at the clicked point (T125,
//! graphing 17.8).
use super::*;
use sheaf_core::shape::{Shape, ShapeKind};

fn context(h: &mut Harness<'static, SheafApp>, point: (f64, f64), action: &str) {
    let at = frame_point(h, point.0, point.1);
    right_click_at(h, at);
    assert!(entry_enabled(h, action), "{action} is live");
    menu_entry(h, action).click();
    settle(h);
}

#[test]
fn copy_then_paste_puts_a_same_size_copy_at_the_clicked_point_and_cut_removes_the_source() {
    crate::shape_clip::clear();
    let mut h = start(vec![data_file("shape-clip")]);
    let id = graph(&h).id;
    let mut rect = Shape::new(ShapeKind::Rectangle, (0.2, 0.3), (0.4, 0.5));
    rect.fill = Some(sheaf_core::Color::rgba(220, 20, 20, 255));
    h.state_mut().doc.apply("Shapes", |p| p.graph_mut(id).unwrap().layers[0].shapes = vec![rect.clone()]);
    settle(&mut h);
    // Nothing copied yet: the page menu has no Paste.
    let empty = frame_point(&h, 0.45, 0.72);
    right_click_at(&mut h, empty);
    assert!(h.query_by_label("Paste").is_none(), "no Paste before a copy");
    close_menus(&mut h);

    context(&mut h, (0.3, 0.4), "Copy");
    assert_eq!(shapes(&h).len(), 1, "Copy changes nothing");
    context(&mut h, (0.45, 0.72), "Paste");
    let s = shapes(&h);
    assert_eq!(s.len(), 2);
    let copy = &s[1];
    let close = |a: f64, b: f64| (a - b).abs() < 0.01;
    assert!(close(copy.x1.min(copy.x2), 0.45) && close(copy.y1.min(copy.y2), 0.72), "{copy:?}");
    assert!(close((copy.x2 - copy.x1).abs(), 0.2) && close((copy.y2 - copy.y1).abs(), 0.2), "same size");
    assert_eq!((copy.kind, copy.fill), (rect.kind, rect.fill));
    assert_eq!(h.state().doc.undo_label(), Some("Paste drawn object"));

    context(&mut h, (0.3, 0.4), "Cut");
    assert_eq!(shapes(&h).len(), 1, "the source is gone");
    assert!(close(shapes(&h)[0].x1.min(shapes(&h)[0].x2), 0.45), "the copy stays");
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(shapes(&h).len(), 2, "one undo step brings the source back");
}

#[test]
fn a_text_label_is_cut_and_pasted_with_its_anchor_at_the_clicked_point() {
    // T147: a label's right-click Cut and Copy, pasted by the page's Paste.
    crate::shape_clip::clear();
    let labels = menus::graph_label_menu(0, 0);
    for want in [menus::Cmd::CutLabel { layer: 0, label: 0 }, menus::Cmd::CopyLabel { layer: 0, label: 0 }] {
        assert!(labels.iter().any(|e| matches!(e, menus::Entry::Item(i) if i.cmd.as_ref() == Some(&want))));
    }
    let mut h = start(vec![data_file("label-clip")]);
    let ctx = h.ctx.clone();
    let graph = project(&h).graphs().next().unwrap().id;
    h.state_mut().doc.apply("Labels", |p| {
        let l = &mut p.graph_mut(graph).unwrap().layers[0];
        let font = l.new_label_font();
        l.labels.push(sheaf_core::graph::TextLabel::new("note", 0.3, 0.3, font));
    });
    let texts = |h: &Harness<'static, SheafApp>| -> Vec<(String, f64, f64)> {
        project(h).graph(graph).unwrap().layers[0].labels.iter().map(|l| (l.text.clone(), l.x, l.y)).collect()
    };
    let n = texts(&h).len();
    h.state_mut().apply(Action::CopyLabel { graph, layer: 0, label: n - 1, cut: true }, &ctx);
    settle(&mut h);
    assert_eq!(texts(&h).len(), n - 1, "Cut deletes the label");
    // Paste at the page point of the frame's centre.
    let g = project(&h).graph(graph).unwrap();
    let f = g.absolute_frame(0).unwrap();
    let page = ((f.left + 0.5 * f.width) * g.page.width_pt, (f.top + 0.5 * f.height) * g.page.height_pt);
    h.state_mut().apply(Action::PasteShape { graph, layer: 0, page }, &ctx);
    settle(&mut h);
    let last = texts(&h).last().cloned().unwrap();
    assert_eq!(last.0, "note");
    assert!((last.1 - 0.5).abs() < 1e-9 && (last.2 - 0.5).abs() < 1e-9, "{last:?}");
    assert_eq!(h.state().doc.undo_label(), Some("Paste text"));
}
