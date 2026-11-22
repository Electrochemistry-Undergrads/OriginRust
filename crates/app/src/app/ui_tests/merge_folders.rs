use super::*;
use sheaf_core::project::Folder;

const NAMES: [&str; 8] = ["RootZ", "RootA", "LocalZ", "LocalA", "ChildZ", "OtherZ", "ChildA", "DeepA"];

fn fixture(tag: &str) -> (Harness<'static, SheafApp>, Vec<WindowId>) {
    let mut h = start(vec![data_file(tag)]);
    let ys: Vec<_> = {
        let b = project(&h).workbooks().next().unwrap();
        b.sheet().y_columns().into_iter().map(|i| b.sheet().columns[i].id).collect()
    };
    for k in 1..8 {
        h.state_mut().doc.apply("Plot", |p| p.plot_columns(Template::Line, &[ys[k % ys.len()]]).unwrap());
    }
    let ids: Vec<_> = project(&h).graphs().map(|g| g.id).collect();
    h.state_mut().doc.apply("Folder fixture", |p| {
        for (&id, name) in ids.iter().zip(NAMES) {
            let g = p.graph_mut(id).unwrap();
            g.name = name.into();
            g.long_name.clear();
            g.layers[0].legend.text = Some(name.into());
        }
        p.root = Folder {
            name: "Folders".into(),
            windows: vec![ids[0]], // RootA is implicitly in the root.
            folders: vec![
                Folder {
                    name: "Scope".into(),
                    windows: vec![ids[3], ids[2]], // Membership order is not graph creation order.
                    folders: vec![
                        Folder { name: "Zulu".into(), windows: vec![ids[4]], ..Default::default() },
                        Folder {
                            name: "Alpha".into(),
                            windows: vec![ids[6]],
                            folders: vec![Folder { name: "Deep".into(), windows: vec![ids[7]], ..Default::default() }],
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
                Folder { name: "Outside".into(), windows: vec![ids[5]], ..Default::default() },
            ],
            ..Default::default()
        };
    });
    h.state_mut().panels.explorer.folder = vec![0];
    h.state_mut().focus(ids[2]);
    settle(&mut h);
    (h, ids)
}

fn open(h: &mut Harness<'static, SheafApp>) {
    menu_path(h, &["Graph", "Merge Graph Windows..."]);
    assert!(h.state().merge_graphs.is_some());
}

fn source(h: &mut Harness<'static, SheafApp>, current: &str, next: &str) {
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value(current)).click();
    settle(h);
    click(h, next);
}

#[test]
fn extra_grid_layers_are_included_in_preview_and_in_the_undoable_merge() {
    let (mut h, _) = fixture("merge-empty-grid-cells");
    let before = project(&h).clone();
    open(&mut h);
    click(&mut h, "Preview");
    assert!(h.query_by_label("8 source graphs, 8 layers").is_some());
    click(&mut h, "Add Extra Layer(s) for Grid");
    assert!(h.query_by_label("8 source graphs, 8 layers").is_none());
    click(&mut h, "Preview");
    assert!(h.query_by_label("8 source graphs, 9 layers").is_some());
    assert_eq!(project(&h), &before);
    click(&mut h, "OK");
    let merged = project(&h).graphs().last().unwrap();
    assert_eq!(merged.layers.len(), 9);
    let empty = &merged.layers[8];
    assert!(empty.plots.is_empty() && empty.link.is_none());
    assert!(empty.frame.left > merged.layers[7].frame.left);
    assert_eq!(empty.frame.top, merged.layers[7].frame.top);
    menu(&mut h, "Edit", "Undo Merge Graph Windows");
    assert_eq!(project(&h), &before);
}

#[test]
fn merge_panel_labels_follow_source_order_and_the_dialog_draft() {
    use sheaf_core::layers::NewLayerKind;
    let (mut h, ids) = fixture("merge-panel-labels");
    h.state_mut().doc.apply("Second axis", |p| {
        p.graph_mut(ids[3]).unwrap().add_layer(NewLayerKind::RightY, 0).unwrap();
    });
    settle(&mut h);
    let before = project(&h).clone();
    open(&mut h);
    source(&mut h, "All in Project", "All in Active Folder");
    click(&mut h, "Arrange Settings");
    click(&mut h, "Spacing (in % of Page Dimension)");
    click(&mut h, "Add Label");
    source(&mut h, "None", "Custom");
    click(&mut h, "Preview");
    assert!(h.query_by_label("2 source graphs, 3 layers").is_some());
    source(&mut h, "Top Left Outside", "Top Right Inside");
    assert!(h.query_by_label("2 source graphs, 3 layers").is_none());
    assert_eq!(project(&h), &before);
    click(&mut h, "OK");

    let merged = project(&h).graphs().last().unwrap();
    let titles: Vec<_> = merged.layers.iter().map(|l| l.labels.first().map(|label| label.text.as_str())).collect();
    assert_eq!(titles, [Some("Graph01"), Some("Graph02"), None]);
    assert!(merged.layers[0].labels[0].x > 0.5 && merged.layers[0].labels[0].y > 0.0);
    let svg = sheaf_plot::render_svg(project(&h), merged.id).unwrap();
    assert!(svg.contains("Graph01") && svg.contains("Graph02"));
    menu(&mut h, "Edit", "Undo Merge Graph Windows");
    assert_eq!(project(&h), &before);
}

#[test]
fn common_scale_preview_and_merge_use_the_sources_displayed_axis_ranges() {
    let (mut h, ids) = fixture("merge-common-scale");
    let (first, second) = (ids[2], ids[3]);
    h.state_mut().doc.apply("Set X limits", |p| {
        for (id, from, to) in [(first, 100.0, 1.0), (second, 1.0, 10.0)] {
            let axis = &mut p.graph_mut(id).unwrap().layers[0].x_axis;
            axis.scale = sheaf_core::graph::ScaleKind::Log10;
            axis.min = Some(from);
            axis.max = Some(to);
        }
    });
    settle(&mut h);
    let before = project(&h).clone();
    let resolved_y = [first, second].map(|id| {
        let axis = &sheaf_plot::layout_geometry(&before, id).unwrap()[0].y;
        axis.max - axis.min
    });
    assert_ne!(resolved_y[0], resolved_y[1]);
    open(&mut h);
    source(&mut h, "All in Project", "All in Active Folder");
    click(&mut h, "Preview");
    assert!(h.query_by_label("2 source graphs, 2 layers").is_some());
    click(&mut h, "Set Layer Width by Common Scale");
    click(&mut h, "Set Layer Height by Common Scale");
    assert!(h.query_by_label("2 source graphs, 2 layers").is_none());
    click(&mut h, "Preview");
    assert!(h.query_by_label("2 source graphs, 2 layers").is_some());
    assert_eq!(project(&h), &before);
    click(&mut h, "OK");
    let layers = &project(&h).graphs().last().unwrap().layers;
    assert_eq!(layers[0].legend.text.as_deref(), Some("LocalZ"));
    assert_eq!(layers[1].legend.text.as_deref(), Some("LocalA"));
    let (a, b) = (layers[0].frame, layers[1].frame);
    assert!((a.width / 99.0 - b.width / 9.0).abs() < 1e-9);
    assert!((a.height / resolved_y[0] - b.height / resolved_y[1]).abs() < 1e-9);
    assert!((b.top - a.top - a.height - 0.05).abs() < 1e-9);
    menu(&mut h, "Edit", "Undo Merge Graph Windows");
    assert_eq!(project(&h), &before);
}

#[test]
fn turning_off_source_grouping_reflows_the_preview_and_keeps_linked_axes_together() {
    use sheaf_core::layers::NewLayerKind;

    let (mut h, ids) = fixture("merge-source-grouping");
    h.state_mut().doc.apply("Independent layer and linked axis", |p| {
        let graph = p.graph_mut(ids[2]).unwrap();
        graph.add_layer(NewLayerKind::Normal, 0).unwrap();
        graph.add_layer(NewLayerKind::RightY, 0).unwrap();
        for (layer, to) in graph.layers.iter_mut().zip([10.0, 20.0, 10.0]) {
            layer.x_axis.min = Some(0.0);
            layer.x_axis.max = Some(to);
        }
        let axis = &mut p.graph_mut(ids[3]).unwrap().layers[0].x_axis;
        axis.min = Some(0.0);
        axis.max = Some(40.0);
    });
    settle(&mut h);
    let before = project(&h).clone();
    open(&mut h);
    source(&mut h, "All in Project", "All in Active Folder");
    click(&mut h, "Add Extra Layer(s) for Grid");
    click(&mut h, "Set Layer Width by Common Scale");
    click(&mut h, "Auto Preview");
    assert!(h.query_by_label("2 source graphs, 4 layers").is_some());
    click(&mut h, "Treat Each Source Graph as a Unit");
    assert!(h.query_by_label("2 source graphs, 5 layers").is_some());
    assert_eq!(project(&h), &before);
    click(&mut h, "OK");
    let merged = project(&h).graphs().last().unwrap();
    assert_eq!(merged.layers.len(), 5);
    assert_eq!(merged.link_parent(2), Some(0));
    assert!(merged.layers[1].link.is_none() && merged.layers[4].plots.is_empty());
    assert!((merged.layers[0].frame.width * 2.0 - merged.layers[1].frame.width).abs() < 1e-9);
    assert!((merged.layers[0].frame.width * 4.0 - merged.layers[3].frame.width).abs() < 1e-9);
    menu(&mut h, "Edit", "Undo Merge Graph Windows");
    assert_eq!(project(&h), &before);
}

#[test]
fn the_active_page_can_be_previewed_and_merged_in_a_one_graph_project() {
    let mut h = start(vec![data_file("merge-active-page")]);
    show_window(&mut h, "Graph1 - sample");
    let first = graph(&h).id;
    let before = project(&h).clone();
    open(&mut h);
    click(&mut h, "Preview");
    assert!(h.query_by_label("1 source graph, 1 layer").is_some());
    assert_eq!(project(&h), &before);

    click(&mut h, "OK");
    let mut expected = before.clone();
    expected
        .merge_graphs(&sheaf_core::merge::MergeOptions {
            sources: vec![first],
            allow_single_source: true,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(project(&h), &expected);
    menu(&mut h, "Edit", "Undo Merge Graph Windows");
    assert_eq!(project(&h), &before);
}

#[test]
fn open_folder_uses_current_window_visibility_and_includes_multilayer_graphs() {
    let (mut h, ids) = fixture("merge-folder-open");
    h.state_mut().doc.apply("Open graph fixture", |p| {
        p.root.windows.retain(|id| *id != ids[0]);
        p.root.folders[0].windows.extend(&ids[..2]);
        let g = p.graph_mut(ids[3]).unwrap();
        g.layers.push(g.layers[0].clone());
    });
    let app = h.state_mut();
    app.focus(ids[0]);
    app.workspace.hide(ids[0]);
    app.focus(ids[1]);
    app.workspace.minimise(ids[1]);
    app.focus(ids[3]);
    app.focus(ids[4]);
    app.focus(ids[2]);
    settle(&mut h);
    let before = project(&h).clone();
    open(&mut h);
    source(&mut h, "All in Project", "All in Active Folder (Open)");
    click(&mut h, "Auto Preview");
    assert!(h.query_by_label("2 source graphs, 3 layers").is_some());
    h.state_mut().workspace.hide(ids[3]);
    settle(&mut h);
    assert!(h.query_by_label("1 source graph, 1 layer").is_some());
    assert_eq!(project(&h), &before);

    click(&mut h, "OK");
    let mut expected = before;
    expected
        .merge_graphs(&sheaf_core::merge::MergeOptions {
            sources: vec![ids[2]],
            allow_single_source: true,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(project(&h), &expected);
}

#[test]
fn include_embedded_adds_graphs_from_the_folders_books_and_deduplicates_windows() {
    let (mut h, ids) = fixture("merge-folder-embedded");
    let book_id = project(&h).workbooks().next().unwrap().id;
    h.state_mut().doc.apply("Embedded graph fixture", |p| {
        let book = p.workbook_mut(book_id).unwrap();
        book.add_graph_sheet(ids[4], "ChildZ");
        book.add_graph_sheet(WindowId(u64::MAX), "Missing");
        let sheet = &mut book.sheets[0];
        let y = sheet.columns[1].id;
        sheet.set_cell_graph(y, 0, Some(ids[5]));
        sheet.set_cell_graph(y, 1, Some(ids[4]));
        sheet.set_cell_graph(y, 2, Some(ids[2]));
        p.root.folders[0].windows.push(book_id);
    });
    settle(&mut h);
    let before = project(&h).clone();
    open(&mut h);
    source(&mut h, "All in Project", "All in Active Folder (Include Embedded)");
    click(&mut h, "OK");

    let mut expected = before;
    let opts = sheaf_core::merge::MergeOptions {
        sources: [5, 4, 2, 3].map(|i| ids[i]).to_vec(),
        allow_single_source: true,
        ..Default::default()
    };
    expected.merge_graphs(&opts).unwrap();
    assert_eq!(project(&h), &expected);
}

#[test]
fn active_folder_merges_only_its_graphs_in_the_measured_order_and_undoes_once() {
    let (mut h, ids) = fixture("merge-folder-direct");
    let before = project(&h).clone();
    open(&mut h);
    source(&mut h, "All in Project", "All in Active Folder");
    assert!(h.query_by_label("ChildZ").is_none());
    click(&mut h, "OK");
    let mut expected = before.clone();
    let merged = expected
        .merge_graphs(&sheaf_core::merge::MergeOptions { sources: vec![ids[2], ids[3]], ..Default::default() })
        .unwrap();
    assert_eq!(project(&h), &expected);
    assert_eq!(h.state().active, Some(merged));
    assert_eq!(h.state().doc.undo_label(), Some("Merge Graph Windows"));
    menu(&mut h, "Edit", "Undo Merge Graph Windows");
    assert_eq!(project(&h), &before);
    menu(&mut h, "Edit", "Redo Merge Graph Windows");
    assert_eq!(project(&h), &expected);
}

#[test]
fn recursive_folder_uses_native_depth_first_name_order_and_renders_only_chosen_legends() {
    let (mut h, ids) = fixture("merge-folder-recursive");
    let dir = save_dir("merge-folder-recursive");
    let fixture = dir.join("fixture.sheaf");
    h.state_mut().save_to(fixture.clone());
    println!("MERGE_FOLDER_FIXTURE={}", fixture.display());
    let before = project(&h).clone();
    open(&mut h);
    source(&mut h, "All in Project", "All in Active Folder (Recursive)");
    click(&mut h, "OK");
    let mut expected = before;
    let sources = [2, 3, 6, 7, 4].map(|i| ids[i]).to_vec();
    let merged = expected.merge_graphs(&sheaf_core::merge::MergeOptions { sources, ..Default::default() }).unwrap();
    assert_eq!(project(&h), &expected);
    let g = project(&h).graph(merged).unwrap();
    let names: Vec<_> = g.layers.iter().map(|l| l.legend.text.as_deref().unwrap()).collect();
    assert_eq!(names, ["LocalZ", "LocalA", "ChildA", "DeepA", "ChildZ"]);
    let svg = sheaf_plot::render_svg(project(&h), merged).unwrap();
    std::fs::write(dir.join("recursive.svg"), &svg).unwrap();
    for name in names {
        assert!(svg.contains(name), "{name}");
    }
    assert!(!svg.contains("OtherZ") && !svg.contains("RootZ") && !svg.contains("RootA"));
    h.state_mut().save_to(dir.join("recursive.sheaf"));
}

#[test]
fn root_includes_implicit_graphs_and_cancel_discards_folder_modes() {
    let (mut h, ids) = fixture("merge-folder-root");
    h.state_mut().panels.explorer.folder.clear();
    h.state_mut().focus(ids[0]);
    settle(&mut h);
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    open(&mut h);
    source(&mut h, "All in Project", "All in Active Folder");
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    open(&mut h);
    source(&mut h, "All in Project", "All in Active Folder");
    click(&mut h, "OK");
    let mut expected = before;
    expected
        .merge_graphs(&sheaf_core::merge::MergeOptions { sources: vec![ids[0], ids[1]], ..Default::default() })
        .unwrap();
    assert_eq!(project(&h), &expected);
}

#[test]
fn folder_modes_keep_specified_ticks_and_existing_project_source_order() {
    let (mut h, ids) = fixture("merge-folder-specified");
    let before = project(&h).clone();
    open(&mut h);
    source(&mut h, "All in Project", "Specified");
    click(&mut h, "RootZ");
    source(&mut h, "Specified", "All in Active Folder (Recursive)");
    source(&mut h, "All in Active Folder (Recursive)", "Specified");
    assert_eq!(
        h.get_by_role_and_label(egui::accesskit::Role::CheckBox, "RootZ").accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::False)
    );
    click(&mut h, "OK");
    let mut expected = before;
    expected
        .merge_graphs(&sheaf_core::merge::MergeOptions { sources: ids[1..].to_vec(), ..Default::default() })
        .unwrap();
    assert_eq!(project(&h), &expected);
}

#[test]
fn the_folder_is_captured_and_tree_edits_cannot_retarget_a_merge() {
    let (mut h, ids) = fixture("merge-folder-captured");
    let before = project(&h).clone();
    open(&mut h);
    source(&mut h, "All in Project", "All in Active Folder");
    h.state_mut().panels.explorer.folder = vec![1];
    settle(&mut h);
    click(&mut h, "OK");
    let mut expected = before;
    expected
        .merge_graphs(&sheaf_core::merge::MergeOptions { sources: vec![ids[2], ids[3]], ..Default::default() })
        .unwrap();
    assert_eq!(project(&h), &expected);

    for mode in ["All in Active Folder", "All in Active Folder (Recursive)"] {
        for change in 0..4 {
            let (mut h, _) = fixture(&format!("merge-folder-stale-{mode}-{change}"));
            open(&mut h);
            source(&mut h, "All in Project", mode);
            h.state_mut().doc.apply("Change folder tree", |p| match change {
                0 => p.root.folders[0].name = "Renamed".into(),
                1 => p.root.folders.swap(0, 1),
                2 => {
                    p.root.folders.remove(0);
                }
                _ => {
                    let moved = p.root.folders[0].windows.pop().unwrap();
                    p.root.folders[1].windows.push(moved);
                }
            });
            settle(&mut h);
            let before = project(&h).clone();
            let revision = h.state().doc.revision();
            click(&mut h, "OK");
            assert!(h.state().merge_graphs.is_some());
            assert!(h.query_by_label_contains("Source folder tree changed").is_some());
            assert_eq!(project(&h), &before);
            assert_eq!(h.state().doc.revision(), revision);
            source(&mut h, mode, "All in Project");
            assert!(h.query_by_label_contains("Source folder tree changed").is_none());
            click(&mut h, "Cancel");
        }
    }
}

#[test]
fn empty_single_graph_and_invalid_folder_paths_do_not_create_history() {
    for (tag, path) in [("empty", vec![2]), ("single", vec![0, 0]), ("invalid", vec![99])] {
        let (mut h, _) = fixture(&format!("merge-folder-{tag}"));
        h.state_mut()
            .doc
            .apply("Empty folder", |p| p.root.folders.push(Folder { name: "Empty".into(), ..Default::default() }));
        let d = MergeGraphsDialog::new(project(&h), &path, h.state().active);
        h.state_mut().merge_graphs = Some(d);
        settle(&mut h);
        let before = project(&h).clone();
        let revision = h.state().doc.revision();
        source(&mut h, "All in Project", "All in Active Folder");
        click(&mut h, "OK");
        assert!(h.state().merge_graphs.is_some());
        assert!(
            h.query_by_label_contains(if tag == "invalid" {
                "Source folder tree changed"
            } else {
                "at least two graphs"
            })
            .is_some()
        );
        assert_eq!(project(&h), &before);
        assert_eq!(h.state().doc.revision(), revision);
    }
}

#[test]
fn a_multilayer_source_is_kept_together_and_source_removal_and_saves_keep_folder_scope() {
    let (mut h, ids) = fixture("merge-folder-save");
    h.state_mut().doc.apply("Second source layer", |p| {
        let g = p.graph_mut(ids[3]).unwrap();
        let mut layer = g.layers[0].clone();
        layer.legend.text = Some("LocalA second".into());
        g.layers.push(layer);
    });
    let before = project(&h).clone();
    open(&mut h);
    source(&mut h, "All in Project", "All in Active Folder");
    click(&mut h, "Keep Source Graphs");
    click(&mut h, "OK");
    let mut expected = before.clone();
    let merged = expected
        .merge_graphs(&sheaf_core::merge::MergeOptions {
            sources: vec![ids[2], ids[3]],
            keep_sources: false,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(project(&h), &expected);
    assert!(project(&h).graph(ids[2]).is_none() && project(&h).graph(ids[3]).is_none());
    let layers: Vec<_> =
        project(&h).graph(merged).unwrap().layers.iter().map(|l| l.legend.text.as_deref().unwrap()).collect();
    assert_eq!(layers, ["LocalZ", "LocalA", "LocalA second"]);
    menu(&mut h, "Edit", "Undo Merge Graph Windows");
    assert_eq!(project(&h), &before);
    menu(&mut h, "Edit", "Redo Merge Graph Windows");
    assert_eq!(project(&h), &expected);
    let saved_graph = project(&h).graph(merged).unwrap().clone();
    let saved_root = project(&h).root.clone();
    let dir = save_dir("merge-folder-save");
    for extension in ["sheaf", "opj"] {
        let file = dir.join(format!("folders.{extension}"));
        h.state_mut().save_to(file.clone());
        let back = start(vec![file]);
        assert_eq!(project(&back).graph(merged), Some(&saved_graph), "{extension}");
        assert_eq!(project(&back).root, saved_root, "{extension}");
    }
}
