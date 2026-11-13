# T01: Enable greyed menu items whose feature already exists

**Status:** open. **Size:** M. **Branch:** `menus-live`. **Model:** Sonnet.

## Goal

Some menu items are greyed ("Not available in Sheaf yet") although the core or the CLI already does
the work. Wire them to the existing code so a user who knows Origin finds them live. Do **not** build
new features here; if an item needs real new code, leave it greyed and list it in the brief's
"Left" section.

## Where

- `crates/app/src/menus.rs`: helpers `grey`, `grey_keyed`, `greys`, `grey_sub` make greyed entries;
  `item(label, Cmd::...)` / `keyed(...)` make live ones. The module doc (top of file) states the rule.
- `crates/app/src/actions.rs`: the `Cmd` enum and its handlers.
- Menu test in `crates/app/src/app/ui_tests.rs` requires every template to be in the menu; use
  `menu_path(&mut h, &["Plot", ...])` as the other UI tests do.

## Candidates (check each in code before wiring)

| Menu item (menus.rs label) | Existing code to call |
|---|---|
| Plot > Multi-Panel > Vertical 2 Panel, Horizontal 2 Panel, 4 Panel, 9 Panel, Stack... | `crates/core/src/panels.rs` `PanelLayout::stack(n)` / `PanelLayout::grid(rows, cols, frame)`; CLI `sheaf plot --panels` (`crates/cli/src/args.rs`, field `panels`) shows how a graph is built from them |
| Plot > Symbol > Y Error | Y error bars exist (symmetric Y only, `crates/plot/src/plots.rs`); a plot of Y with a `YError` column. Origin's Y Error template is a scatter with error bars from the adjacent `YError` column |
| Graph > Add Error Bars... | same error-bar model; Origin adds an error-bar plot to the selected plot (minimal dialog: choose the column) |
| Graph > Legend > Update Legend | Reconstruct Legend exists (`Cmd::ReconstructLegend`, Ctrl+L); Update Legend keeps custom text and refreshes entries (graphing.md section 6) |
| Window > Refresh (F5) | redraw + recalculate pending formula columns (recalculation exists, `Project::set_column_values` modes) |
| Worksheet > Sort Columns / Sort Range | `Project::sort_sheet` in `crates/core/src/analysis.rs` sorts the whole sheet by one key; Sort Range needs a row/column subset argument (small core change is fine) |

Not candidates (need new features; covered by other briefs): XY Error (T12), Stacked/Fill Area
(T12), Mask/Hide/Filter (T10), Exchange X-Y Axes (T11), Data menu (T18).

## Steps

1. For each candidate, confirm the code path works from the CLI or a core test.
2. Replace `grey(...)` with a live item and a `Cmd` variant; follow how `Cmd::Plot(Template::...)`
   items are handled. Multi-Panel 2/4/9 panel: Origin makes one panel per selected Y column (2 panel
   = two Y columns); measure in Origin with `tools/origin-ref/` if unsure.
3. One UI test per wired item (menu path → expected graph/sheet state).

## Done when

The wired items are live, each has a UI test, checks pass, feature-parity rows updated.

## Don't

- Don't remove greyed items for features that do not exist; greying is the deliberate Origin-look rule.
- Don't touch dialogs that other briefs own (T03, T05, T09).

## Left

(fill in)
