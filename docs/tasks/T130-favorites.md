# T130: Add Shortcut to Favorites

**Status:** done 2026-11-18 (Claude Windows "t30-origin", gui-look "Favorites"), **not measured** (Origin COM
unregistered on the test machine). Left: measure Origin's Favorites folder (place, menus, saving) and match it.
**Size:** S. **Source:** greyed inventory package B12 (graph page right-click > Add Shortcut to Favorites).
**Spec:** `docs/spec/gui-look.md` ("Favorites").

## Goal

The graph page's Add Shortcut to Favorites puts a shortcut to the graph in a Favorites folder of the Project
Explorer, which lists the shortcuts of every window added there.

## Done when

The item works, the explorer has the Favorites folder, the inventory no longer lists the item, and fmt,
strict Clippy and the full tests pass.
