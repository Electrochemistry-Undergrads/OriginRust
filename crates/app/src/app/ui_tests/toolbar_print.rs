//! The Standard bar uses the same active-window Print command as the File menu.
use super::*;

const BUTTON: &str = "Standard toolbar: Print the active window";

#[test]
fn print_follows_the_active_window_and_dispatches_the_graph_command() {
    let empty = start(Vec::new());
    assert!(empty.get_by_label(BUTTON).accesskit_node().is_disabled());

    let mut h = start(vec![data_file("toolbar-print")]);
    show_window(&mut h, "Book1 - sample");
    let toolbar_enabled = !h.get_by_label(BUTTON).accesskit_node().is_disabled();
    click(&mut h, "File");
    assert_eq!(toolbar_enabled, entry_enabled(&h, "Print... Ctrl+P"));
    close_menus(&mut h);

    show_window(&mut h, "Graph1 - sample");
    let graph = h.state().active.expect("a graph is active");
    assert!(project(&h).graph(graph).is_some());
    assert!(!h.get_by_label(BUTTON).accesskit_node().is_disabled());
    click(&mut h, "File");
    assert!(entry_enabled(&h, "Print... Ctrl+P"));
    close_menus(&mut h);
    h.get_by_label(BUTTON).click();
    settle(&mut h);
    assert_eq!(h.state().active, Some(graph));
    assert!(h.state().status.contains("Printing is skipped under test"), "{}", h.state().status);
    assert!(h.state().message.is_none());

    show_window(&mut h, "Book1 - sample");
    let toolbar_enabled = !h.get_by_label(BUTTON).accesskit_node().is_disabled();
    click(&mut h, "File");
    assert_eq!(toolbar_enabled, entry_enabled(&h, "Print... Ctrl+P"));
    close_menus(&mut h);
}
