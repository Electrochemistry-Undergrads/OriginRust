//! The graph page's Send Graphs to PowerPoint... (T232, graphing.md 17.11).

use super::*;

#[test]
fn send_graphs_to_powerpoint_picks_the_graphs_and_writes_one_slide_each() {
    let entries = menus::graph_page_menu(0, (100.0, 80.0));
    assert!(entries.iter().any(|e| matches!(e, menus::Entry::Item(i)
        if i.label == "Send Graphs to PowerPoint..." && i.cmd == Some(menus::Cmd::SendGraphsToPowerPoint))));
    let mut h = start(vec![data_file("send-ppt")]);
    let graph = project(&h).graphs().next().unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::SendGraphsToPowerPoint(graph), &ctx);
    settle(&mut h);
    assert_eq!(h.state().send_ppt.as_ref().unwrap().chosen(), vec![graph], "the active graph by default");
    click(&mut h, "All graphs in the project (1)");
    assert!(h.state().send_ppt.as_ref().unwrap().all_graphs);
    click(&mut h, "Cancel");
    assert!(h.state().send_ppt.is_none());

    let dir = std::env::temp_dir().join(format!("sheaf-app-send-ppt-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("graphs.pptx");
    h.state_mut().apply(Action::WritePptx { graphs: vec![graph, graph], path: path.clone() }, &ctx);
    // The part names stand uncompressed in the zip's directory.
    let bytes = std::fs::read(&path).unwrap();
    let has = |name: &str| bytes.windows(name.len()).any(|w| w == name.as_bytes());
    assert!(has("ppt/slides/slide2.xml") && has("ppt/media/image2.png") && !has("ppt/slides/slide3.xml"));
    let _ = std::fs::remove_dir_all(&dir);
}
