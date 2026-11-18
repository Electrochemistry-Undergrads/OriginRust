//! File > Open Sample Projects (T133, gui-look "Sample Projects").

use super::*;

#[test]
fn a_sample_project_replaces_the_project() {
    let mut h = start(vec![]);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::SampleProjects, &ctx);
    settle(&mut h);
    h.get_by_label("Stock Prices").click();
    settle(&mut h);
    h.get_by_label("Open").click();
    settle(&mut h);
    assert!(h.state().sample_projects.is_none());
    let p = project(&h);
    assert_eq!(p.graphs().count(), 1);
    assert_eq!(p.graphs().next().unwrap().layers.len(), 2, "the OHLC-Volume graph");
    assert!(!h.state().doc.dirty, "a fresh sample has nothing to save");
}

#[test]
fn unsaved_changes_are_asked_about_first() {
    let mut h = start(vec![data_file("sample-dirty")]);
    let ctx = h.ctx.clone();
    h.state_mut().doc.apply("Change", |p| {
        p.add_workbook("x");
    });
    h.state_mut().apply(Action::OpenSample("Wind".into()), &ctx);
    settle(&mut h);
    assert!(project(&h).graphs().all(|g| g.layers[0].coords.polar().is_none()), "not opened yet");
    assert_eq!(h.state().pending_sample.as_deref(), Some("Wind"));
}
