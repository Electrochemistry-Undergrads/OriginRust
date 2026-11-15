# T01: Enable greyed menu items whose feature already exists

**Status:** done, merged to main (`a2fb77b`); selected-row sorting and Sort Columns/column-context Custom are also enabled; Graph > Add Error Bars now selects a plot and an existing numeric Y-error column (Sheaf's chooser, graphing 14.13a); Multi-Panel > Stack now exposes vertical stacks with assignment, spacing, orientation, legends and links (15.7a); see "Left" for what remains. **Size:** M. **Branch:** `menus-live`. **Model:** Sonnet.

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
| Graph > Add Error Bars... | same error-bar model; Sheaf's dialog chooses a plot and an existing numeric error column; Origin 2016 instead generates errors (14.13a) |
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

Wired (each has a UI test in `ui_tests.rs`): Plot > Multi-Panel > Vertical 2 Panel / Horizontal 2 Panel /
4 Panel / 9 Panel (`Cmd::PlotPanels`, `PanelLayout::grid`, one panel per selected Y column), Plot > Symbol >
Y Error (`Cmd::PlotYError`, scatter; plots anyway and says so when no Y error column follows),
Graph > Add Error Bars... (`Cmd::AddErrorBars`, a chooser for one plot and any numeric column across sheets/books;
defaults to the selected curve and its current or adjacent Y-error column), Graph > Legend > Update Legend and the same item of the
legend's right-click menu (`GraphOp::UpdateLegend`), Window > Refresh F5 (`Cmd::Refresh`), Worksheet > Sort
Range and Sort Columns, Ascending/Descending and explicit Each Column modes (row-aware core helpers);
Sort Range, Sort Columns, Sort Worksheet and column-header Custom use the existing nested sort
with captured selected-row bounds. The primary reference and regression scope are in workbook-model.md 13.1.
Multi-Panel > Stack... uses `Project::plot_panels` with Line, Scatter, Line+Symbol or Column,
automatic layer counts, repeated plot counts, layer order, page orientation, per-layer/no legends,
optional X or position-only links, page-percent gap/margins and captured selected-row bounds.
Creation is deferred to OK and is one undo step; invalid inputs and Cancel/Escape make no edits.
Origin 2016 defaults and assignment cases were measured (graphing.md 15.7a).

Left greyed, and why:

- Multi-Panel > Multiple Panels by Label... needs column-label grouping.
- Stack's horizontal direction, external/custom templates, whole-page legend, manual layer count,
  unit conversion and live preview remain disabled; their supporting behavior does not yet exist.

Measured Add Error Bars follow-up (2026-11-14):

- `worksheet -e err;` on a synthetic two-curve graph opens **Error Bars** with the first active
  curve's reference and **Percent of Data (%)**, **Standard Deviation of Data**, **Square Root of Data**
  and **Scaling Factor** controls. It does not offer the existing-column chooser assumed by the brief.
  The controls and selected percentage radio were captured in
  `re-workspace/scratch/t01-error-bars-dialog/dialog/first.controls.txt`; the picture was obscured by
  another foreground window, so it is not visual evidence. The numeric edits were empty in this run;
  no numeric default or generated output is inferred.
- This follow-up stays within the existing-model scope: Sheaf's chooser assigns or replaces one
  plot's `y_error` on OK, leaves other plots and error styling alone, supports Cancel/Escape and one
  Undo/Redo step, and refuses stale plot/source choices. Pure text columns are excluded. It creates
  no columns and changes no column designation. Origin's error generation remains open.

Other differences from Origin that were not measured (use `tools/origin-ref/` before relying on them):

- Update Legend: Origin's exact rule for custom text is not measured; Sheaf keeps custom lines, drops the
  lines of plots that are gone and appends the automatic line of each plot the text does not mention.
- Refresh recalculates the Manual columns marked pending in the active workbook; it does not reload
  imported files.
- The panel templates use `PanelLayout::grid` (even gaps); Origin's own PAN2VERT/PAN4/PAN9 rectangles
  are uneven (graphing.md section 15.4 table).
