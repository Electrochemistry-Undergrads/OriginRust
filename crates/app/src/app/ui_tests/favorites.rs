//! Add Shortcut to Favorites and the Project Explorer's Favorites folder (T130, gui-look "Favorites").

use super::*;

#[test]
fn a_graph_shortcut_goes_to_favorites_and_the_folder_lists_it() {
    let mut h = start(vec![data_file("favorites")]);
    let ctx = h.ctx.clone();
    let graph = project(&h).graphs().next().unwrap().id;
    h.state_mut().apply(Action::AddFavorite(graph), &ctx);
    settle(&mut h);
    assert_eq!(project(&h).favorites, [graph]);
    h.state_mut().apply(Action::AddFavorite(graph), &ctx);
    assert_eq!(project(&h).favorites.len(), 1, "once");
    // The Favorites folder of the explorer lists the shortcut.
    h.get_by_label("Favorites").click();
    settle(&mut h);
    assert!(h.state().panels.explorer.favorites);
    h.state_mut().apply(Action::RemoveFavorite(graph), &ctx);
    settle(&mut h);
    assert!(project(&h).favorites.is_empty());
    // One undo step brings it back.
    h.state_mut().doc.undo();
    assert_eq!(project(&h).favorites, [graph]);
}
