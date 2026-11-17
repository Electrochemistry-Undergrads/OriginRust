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
        let d = MergeGraphsDialog::new(project(&h), &path);
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
