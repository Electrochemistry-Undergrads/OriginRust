//! Copy Format and Paste Format (brief T72): the Edit menu, the page, axis and legend context
//! menus, the Apply Formats window and the one undo step "Apply Theme".

use super::*;
use sheaf_core::Color;

fn edit_layer(h: &mut Harness<'static, SheafApp>, change: impl FnOnce(&mut sheaf_core::Layer)) {
    let id = *h.state().graphs.keys().next().unwrap();
    h.state_mut().doc.apply("test edit", |p| change(&mut p.graph_mut(id).unwrap().layers[0]));
    settle(h);
}

/// Right-click on the page margin, outside every layer.
fn right_click_page(h: &mut Harness<'static, SheafApp>) {
    let g = graph(h);
    let at = graph_ui(h).page_point(g.page.width_pt - 4.0, g.page.height_pt - 4.0).expect("page corner is drawn");
    right_click_at(h, at);
}

fn right_click_on(h: &mut Harness<'static, SheafApp>, kind: ObjectKind) {
    let at = on(h, 0, kind);
    right_click_at(h, at);
}

/// Copy Format from the page menu that is open on the page margin.
fn page_copy(h: &mut Harness<'static, SheafApp>, choice: &str) {
    right_click_page(h);
    pick(h, "Copy Format");
    pick(h, choice);
}

fn styled(h: &mut Harness<'static, SheafApp>) {
    edit_layer(h, |l| {
        l.x_axis.color = Color::RED;
        l.x_axis.tick_font.color = Color::BLUE;
        l.x_axis.tick_font.size_pt = 22.0;
        l.x_axis.line_width_pt = 3.0;
        l.x_axis.min = Some(1.0);
        l.x_axis.max = Some(3.0);
        l.background = Some(Color::rgb(1, 2, 3));
    });
}

fn plain(h: &mut Harness<'static, SheafApp>) {
    edit_layer(h, |l| {
        l.x_axis.color = Color::BLACK;
        l.x_axis.tick_font.color = Color::BLACK;
        l.x_axis.tick_font.size_pt = 10.0;
        l.x_axis.line_width_pt = 1.0;
        l.x_axis.min = None;
        l.x_axis.max = None;
        l.background = None;
    });
}

fn pick(h: &mut Harness<'static, SheafApp>, label: &str) {
    menu_entry(h, label).click();
    settle(h);
}

fn disabled(h: &Harness<'static, SheafApp>, label: &str) -> bool {
    menu_entry(h, label).accesskit_node().is_disabled()
}

#[test]
fn edit_menu_copy_format_all_then_paste_format_window_applies_in_one_undo_step() {
    let mut h = start(vec![data_file("fmt-edit")]);
    styled(&mut h);
    menu_path(&mut h, &["Edit", "Copy Format", "All"]);
    plain(&mut h);
    menu(&mut h, "Edit", "Paste Format...");
    click(&mut h, "Apply");
    let l = &graph(&h).layers[0];
    assert_eq!((l.x_axis.min, l.x_axis.max), (Some(1.0), Some(3.0)));
    assert_eq!((l.x_axis.color, l.x_axis.tick_font.color, l.x_axis.tick_font.size_pt), (Color::RED, Color::BLUE, 22.0));
    assert_eq!(l.x_axis.line_width_pt, 3.0);
    assert_eq!(l.background, Some(Color::rgb(1, 2, 3)));
    assert_eq!(h.state().doc.undo_label(), Some("Apply Theme"));
    click(&mut h, "Undo Apply");
    assert_eq!(graph(&h).layers[0].x_axis.color, Color::BLACK);
    click(&mut h, "Close");
    assert!(h.query_by_label("Apply").is_none(), "the window is closed");
}

#[test]
fn the_apply_formats_window_pastes_only_the_ticked_groups() {
    let mut h = start(vec![data_file("fmt-window")]);
    styled(&mut h);
    menu_path(&mut h, &["Edit", "Copy Format", "All"]);
    plain(&mut h);
    menu(&mut h, "Edit", "Paste Format...");
    click(&mut h, "Colors");
    click(&mut h, "Apply");
    let l = &graph(&h).layers[0];
    assert_eq!(l.x_axis.min, Some(1.0));
    assert_eq!(l.x_axis.color, Color::BLACK, "Colors is unticked");
    assert_eq!(l.x_axis.tick_font.size_pt, 22.0);
    // The same paste again changes nothing: no undo step.
    let undo = h.state().doc.undo_label().map(str::to_string);
    assert_eq!(undo.as_deref(), Some("Apply Theme"));
    let steps = h.state().doc.undo_label().map(str::to_string);
    click(&mut h, "Apply");
    assert_eq!(h.state().doc.undo_label().map(str::to_string), steps);
    menu(&mut h, "Edit", "Undo Apply Theme");
    assert_ne!(h.state().doc.undo_label(), Some("Apply Theme"), "the repeated paste made no step of its own");
}

#[test]
fn the_window_says_so_when_the_clipboard_is_empty() {
    let mut h = start(vec![data_file("fmt-empty")]);
    let before = h.state().doc.undo_label().map(str::to_string);
    menu(&mut h, "Edit", "Paste Format...");
    assert!(h.query_by_value("The clipboard holds no format.").is_some());
    assert!(disabled(&h, "Apply"));
    assert_eq!(h.state().doc.undo_label().map(str::to_string), before);
}

#[test]
fn page_menu_copy_colors_then_paste_format_changes_colours_only() {
    let mut h = start(vec![data_file("fmt-page")]);
    styled(&mut h);
    page_copy(&mut h, "Colors");
    plain(&mut h);
    right_click_page(&mut h);
    pick(&mut h, "Paste Format");
    let l = &graph(&h).layers[0];
    assert_eq!((l.x_axis.color, l.x_axis.tick_font.color), (Color::RED, Color::BLUE));
    assert_eq!(l.background, Some(Color::rgb(1, 2, 3)));
    assert_eq!(l.x_axis.tick_font.size_pt, 10.0, "no fonts");
    assert_eq!(l.x_axis.line_width_pt, 1.0, "no thickness");
    assert_eq!(l.x_axis.min, None, "no scales");
    assert_eq!(h.state().doc.undo_label(), Some("Apply Theme"));
    menu(&mut h, "Edit", "Undo Apply Theme");
    assert_eq!(graph(&h).layers[0].x_axis.color, Color::BLACK);
}

#[test]
fn page_menu_copy_format_lists_origins_choices_and_greys_the_unmeasured_ones() {
    let mut h = start(vec![data_file("fmt-pagelist")]);
    right_click_page(&mut h);
    pick(&mut h, "Copy Format");
    for label in ["Dimensions", "Scales", "Colors", "Fonts", "Background", "All Style Formats", "All"] {
        assert!(!disabled(&h, label), "{label} is enabled");
    }
    for label in ["Page Background", "Legend Translation Mode"] {
        assert!(disabled(&h, label), "{label} is greyed");
    }
}

#[test]
fn page_menu_paste_format_is_greyed_until_something_is_copied() {
    let mut h = start(vec![data_file("fmt-nopaste")]);
    right_click_page(&mut h);
    assert!(disabled(&h, "Paste Format"));
}

#[test]
fn page_menu_copy_dimensions_and_all_style_formats() {
    let mut h = start(vec![data_file("fmt-dims")]);
    edit_layer(&mut h, |l| {
        l.frame.left = 0.3;
        l.frame.width = 0.4;
        l.x_axis.max = Some(50.0);
    });
    page_copy(&mut h, "Dimensions");
    edit_layer(&mut h, |l| l.frame.left = 0.1);
    right_click_page(&mut h);
    pick(&mut h, "Paste Format");
    assert_eq!(graph(&h).layers[0].frame.left, 0.3);
    assert_eq!(graph(&h).layers[0].x_axis.max, Some(50.0), "the scale was not touched");
    // All Style Formats holds no dimensions, no scales and no text.
    styled(&mut h);
    page_copy(&mut h, "All Style Formats");
    plain(&mut h);
    edit_layer(&mut h, |l| l.frame.left = 0.2);
    right_click_page(&mut h);
    pick(&mut h, "Paste Format");
    let l = &graph(&h).layers[0];
    assert_eq!((l.x_axis.color, l.x_axis.tick_font.size_pt, l.x_axis.line_width_pt), (Color::RED, 22.0, 3.0));
    assert_eq!(l.frame.left, 0.2);
    assert_eq!(l.x_axis.min, None);
}

#[test]
fn axis_menu_copy_format_all_then_paste_format_to_all_changes_only_that_axis() {
    let mut h = start(vec![data_file("fmt-axis")]);
    styled(&mut h);
    edit_layer(&mut h, |l| l.y_axis.max = Some(99.0));
    let y_before = graph(&h).layers[0].y_axis.clone();
    right_click_on(&mut h, ObjectKind::XAxis);
    pick(&mut h, "Copy Format");
    assert!(!disabled(&h, "Styles"));
    assert!(disabled(&h, "Dimensions"));
    assert!(!disabled(&h, "Scales"));
    pick(&mut h, "All");
    plain(&mut h);
    right_click_on(&mut h, ObjectKind::XAxis);
    pick(&mut h, "Paste Format To");
    assert!(!disabled(&h, "Tick Direction") && !disabled(&h, "Title Text"), "the finer groups paste (T171)");
    assert!(disabled(&h, "Title Dimensions"), "no title geometry to paste");
    pick(&mut h, "All");
    let l = &graph(&h).layers[0];
    assert_eq!((l.x_axis.min, l.x_axis.max), (Some(1.0), Some(3.0)));
    assert_eq!((l.x_axis.color, l.x_axis.tick_font.color, l.x_axis.tick_font.size_pt), (Color::RED, Color::BLUE, 22.0));
    assert_eq!(l.y_axis, y_before, "the other axis is untouched");
    assert_eq!(l.background, None, "the layer is untouched");
    assert_eq!(h.state().doc.undo_label(), Some("Apply Theme"));
}

#[test]
fn the_clipboard_kind_chooses_the_paste_menu() {
    let mut h = start(vec![data_file("fmt-kinds")]);
    right_click_on(&mut h, ObjectKind::XAxis);
    assert!(not_toolbar(&h, "Paste Format To").is_empty(), "nothing copied");
    pick(&mut h, "Copy Format");
    pick(&mut h, "All");
    right_click_page(&mut h);
    assert!(h.query_by_label("Paste Format").is_none());
    pick(&mut h, "Paste Format To");
    assert!(disabled(&h, "Bottom Axes") && disabled(&h, "All Axes"), "its choices are unmeasured");
    // A graph format gives the axis menu no Paste Format To (no axis selected, so Edit copies the graph).
    h.state_mut().graphs.values_mut().next().unwrap().selected = None;
    menu_path(&mut h, &["Edit", "Copy Format", "All"]);
    right_click_on(&mut h, ObjectKind::XAxis);
    assert!(not_toolbar(&h, "Paste Format To").is_empty());
}

#[test]
fn legend_menu_copies_and_pastes_styles_as_one_undo_step() {
    use sheaf_core::graph::{Font, LegendPosition};

    let mut h = start(vec![data_file("fmt-legend")]);
    edit_layer(&mut h, |l| {
        l.legend.font = Font { color: Color::RED, bold: true, ..Font::new("Arial", 16.0) };
        l.legend.frame = true;
        l.legend.background = Some(Color::rgb(246, 225, 200));
    });
    let source = graph(&h).clone();
    right_click_on(&mut h, ObjectKind::Legend);
    assert!(not_toolbar(&h, "Paste Format To").is_empty());
    pick(&mut h, "Copy Format");
    assert!(!disabled(&h, "Styles"));
    for label in ["Dimensions", "All"] {
        assert!(disabled(&h, label), "{label} is greyed until measured");
    }
    let revision = h.state().doc.revision();
    pick(&mut h, "Styles");
    assert_eq!(h.state().doc.revision(), revision);
    assert_eq!(graph(&h), &source);
    assert_eq!(crate::format_clip_ui::kind(), Some(crate::format_clip_ui::ClipKind::Legend));

    edit_layer(&mut h, |l| {
        l.legend.font = Font { color: Color::BLUE, ..Font::new("Arial", 9.0) };
        l.legend.frame = false;
        l.legend.background = None;
        l.legend.position = LegendPosition::ManualTopLeft { x: 0.55, y: 0.2 };
        l.legend.text = Some(r"Target \l(1) %(1)".into());
        l.legend.columns = 2;
    });
    let before = graph(&h).clone();
    let mut expected = before.clone();
    let (from, to) = (&source.layers[0].legend, &mut expected.layers[0].legend);
    to.font.clone_from(&from.font);
    to.frame = from.frame;
    to.background = from.background;
    for _ in 0..2 {
        right_click_on(&mut h, ObjectKind::Legend);
        pick(&mut h, "Paste Format To");
        pick(&mut h, "Styles");
        assert_eq!(graph(&h), &expected);
    }
    menu(&mut h, "Edit", "Undo Apply Theme");
    assert_eq!(graph(&h), &before);
    menu(&mut h, "Edit", "Redo Apply Theme");
    assert_eq!(graph(&h), &expected);
}

#[test]
fn edit_menu_copy_format_lists_nine_choices_with_symbol_and_text_greyed() {
    let mut h = start(vec![data_file("fmt-editlist")]);
    menu(&mut h, "Edit", "Copy Format");
    for label in ["All", "All Style Formats", "Colors", "Fonts", "Background", "Dimensions", "Scales"] {
        assert!(!disabled(&h, label), "{label} is enabled");
    }
    for label in ["Symbol/Line/Fill", "Text"] {
        assert!(disabled(&h, label), "{label} is greyed until measured");
    }
}

#[test]
fn edit_copy_format_with_an_axis_selected_copies_the_axis_format() {
    let mut h = start(vec![data_file("fmt-editaxis")]);
    styled(&mut h);
    h.state_mut().graphs.values_mut().next().unwrap().selected = Some((0, ObjectKind::XAxis));
    settle(&mut h);
    menu_path(&mut h, &["Edit", "Copy Format", "All"]);
    assert_eq!(crate::format_clip_ui::kind(), Some(crate::format_clip_ui::ClipKind::Axis));
}

#[test]
fn the_window_follows_the_clipboard_while_it_is_open() {
    let mut h = start(vec![data_file("fmt-sync")]);
    menu_path(&mut h, &["Edit", "Copy Format", "Fonts"]);
    menu(&mut h, "Edit", "Paste Format...");
    assert!(h.query_by_label("Background").is_none());
    menu_path(&mut h, &["Edit", "Copy Format", "All"]);
    settle(&mut h);
    assert!(h.query_by_label("Background").is_some(), "the check boxes follow the new clip");
}

#[test]
fn axis_paste_format_to_line_takes_only_the_line_of_the_copied_axis() {
    let mut h = start(vec![data_file("fmt-axis-groups")]);
    styled(&mut h);
    edit_layer(&mut h, |l| l.x_axis.title = Some("Copied title".into()));
    right_click_on(&mut h, ObjectKind::XAxis);
    pick(&mut h, "Copy Format");
    pick(&mut h, "All");
    let before = graph(&h).layers[0].y_axis.clone();
    right_click_on(&mut h, ObjectKind::YAxis);
    pick(&mut h, "Paste Format To");
    pick(&mut h, "Line");
    let y = graph(&h).layers[0].y_axis.clone();
    assert_eq!((y.color, y.line_width_pt), (Color::RED, 3.0), "the line takes the copy");
    assert_eq!(y.tick_font, before.tick_font, "tick labels stay");
    assert_eq!((y.min, y.max), (before.min, before.max), "the scale stays");
    assert_eq!(h.state().doc.undo_label(), Some("Apply Theme"), "one undo step, as the other pastes");
    right_click_on(&mut h, ObjectKind::YAxis);
    pick(&mut h, "Paste Format To");
    pick(&mut h, "Title Text");
    assert_eq!(graph(&h).layers[0].y_axis.title.as_deref(), Some("Copied title"));
}
