//! Gadgets > 2D Integrate (T189, graphing "2D Integrate gadget").

use super::*;
use crate::gadget::GadgetKind;

#[test]
fn the_2d_integrate_gadget_opens_on_a_heat_map_and_measures_the_cells_in_its_rectangle() {
    let mut h = start(vec![data_file("integrate-2d")]);
    // On a graph of curves it says what it needs.
    let ctx = h.ctx.clone();
    let line_graph = graph(&h).id;
    h.state_mut().apply(Action::StartIntegrate2d { graph: line_graph }, &ctx);
    settle(&mut h);
    assert!(h.state().graphs.get(&line_graph).and_then(|s| s.gadget).is_none());
    // A 4 x 4 grid with Z = x * y as a heat map.
    let g = h.state_mut().doc.apply("Grid", |p| {
        let book = p.add_workbook("Grid");
        let (mut xs, mut ys, mut zs) = (Vec::new(), Vec::new(), Vec::new());
        for r in 0..4 {
            for c in 0..4 {
                xs.push(c as f64);
                ys.push(r as f64);
                zs.push((c * r) as f64);
            }
        }
        let x = p.add_column(book, 0, sheaf_core::Designation::X, sheaf_core::ColumnData::numeric(xs)).unwrap();
        let y = p.add_column(book, 0, sheaf_core::Designation::Y, sheaf_core::ColumnData::numeric(ys)).unwrap();
        let z = p.add_column(book, 0, sheaf_core::Designation::Z, sheaf_core::ColumnData::numeric(zs)).unwrap();
        p.plot_xyz(sheaf_core::template::FieldTemplate::HeatMap, x, y, z).unwrap()
    });
    settle(&mut h);
    h.state_mut().apply(Action::StartIntegrate2d { graph: g }, &ctx);
    settle(&mut h);
    let gadget = h.state().graphs.get(&g).and_then(|s| s.gadget).expect("the gadget opened");
    assert_eq!(gadget.kind, GadgetKind::Integrate2d);
    // Its rectangle starts on the middle half of both axes.
    let (lo, hi) = (gadget.x0.min(gadget.x1), gadget.x0.max(gadget.x1));
    assert!(lo > -0.5 && hi < 3.5 && hi > lo);
    let layer = &project(&h).graph(g).unwrap().layers[0];
    let (cells, area) = sheaf_plot::field_cells(project(&h), layer).unwrap();
    assert_eq!((cells.len(), area), (16, 1.0));
    let whole = crate::gadget::volume_in(&cells, area, (-1.0, 4.0), (-1.0, 4.0)).unwrap();
    assert_eq!((whole.n, whole.volume, whole.max), (16, 36.0, 9.0));

    // T198: New Output adds a row to the results table, the second one below the first.
    let inside = gadget.volume(project(&h), g).unwrap();
    for _ in 0..2 {
        h.state_mut().apply(Action::Gadget { graph: g, op: crate::gadget::GadgetOp::NewOutput }, &ctx);
        settle(&mut h);
    }
    assert_eq!(h.state().doc.undo_label(), Some("2D Integrate"));
    let book = project(&h).workbooks().find(|b| b.long_name == crate::gadget::VOLUME_BOOK).unwrap();
    let sheet = &book.sheets[0];
    let names: Vec<&str> = sheet.columns.iter().map(|c| c.long_name.as_str()).collect();
    assert_eq!(names, crate::gadget::VOLUME_COLUMNS);
    let volume = sheet.columns[4].numeric().unwrap();
    assert_eq!(volume, [inside.volume, inside.volume]);
    assert_eq!(sheet.columns[5].numeric().unwrap()[0], inside.n as f64);
    assert_eq!(project(&h).workbooks().filter(|b| b.long_name == crate::gadget::VOLUME_BOOK).count(), 1);
}
