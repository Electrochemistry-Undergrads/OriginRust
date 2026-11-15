# T13: Missing Plot Details tabs and controls

**Status:** base tabs merged from `plot-details` (2026-11-14); Data on Top of Axes, Skip Points and Label White Out completed in the follow-ups below. The measured parts were already on `main`. Measured against Origin (graphing spec 18, 18.1 to 18.5): Drop Lines and Label tabs, line Transparency, Fill Area Under Curve (Normal), the layer Background colour, the Group tab (Edit Mode, increments, members), Offset (X and Y offsets), Display, Size/Speed (units), Link Axes Scales, Stack, the page tabs Print/Dimensions, Display and Legends/Titles, the tree names, Layer Contents and Plot Setup as Origin's own windows, the Workbook and ">>" buttons. Ported from the branch onto those dialogs, **Sheaf's own and not measured** (spec 18.6): a hatch, gradient and transparency for the fill of columns, bars, areas and the fill under a curve (Pattern tab; for areas too); bar Width (in %) on Spacing; Label by a column of the project, a list of rows, decimals and offsets; subgroups by size on the Group tab; the layer Background gradient and border; the X Axes and Y Axes switches of Display; the page's border, layer icons switch, comments (hover text in the Object Manager), window title choice, Miscellaneous tab, and Print/Dimensions keeping the layers' size; the right Y layer no longer copies its parent's background; drop lines are drawn only for the kinds whose tab offers them; deleting a column resets the X error, base and data labels that name it; "Use this style for all plots" copies the look settings. Not taken from the branch because main's measured version replaced it: the second Layer Contents and Plot Setup dialogs, the second Group, Drop Lines, Label and Link tabs, its tree text, its Workbook button and side panel, line transparency as a percentage field. **Left:** the other fill modes and Origin's pattern list, the Offset tab's Multiplier, several groups per layer, subgrouping by column label, Binned, Size/Speed X:Y ratio, caching and speed mode, label frame/margin/leader lines, No Line Break / Gap to Symbol, print margins, writing the new settings to `.opj` (the writer names them in its save warnings). **Unmeasured:** everything in spec 18.6 (compare with Origin on the Windows machine). **Size:** M. **Model:** Sonnet.
**Detail:** audit B "Plot Details", "Layer Contents (F12) and Plot Setup".
**Spec:** `docs/spec/gui-look.md` (Plot Details), `docs/spec/ui-command-map.md` D1/D3.

## Data on Top of Axes follow-up (2026-11-14)

Done: the Display checkbox is enabled, with the unchecked default recorded in the Origin 2016
capture. The shared scene draws axes before data when enabled, while retaining clipping,
independent grid ordering, legends and annotations. Screen/raster, SVG/PDF, EPS and EMF use
that order; picking follows visible data/axis overlap. Tests cover overlap pixels, clipping,
vector record order, polar/ternary scene branches, old project settings and Apply/Cancel/Undo/Redo.
The native/companion representation stores the setting; `.opj` alone still warns, as for the other
Display settings. Other T13 leftovers listed above remain open.

Validation: formatting and strict workspace all-target Clippy pass; `cargo test --workspace`
passes 1,916 tests (two ignored), including all six new regressions. The parity row remains partial
(48 done / 136 partial / 54 missing of 238 rows).

## Skip Points follow-up (2026-11-14)

Implemented the Drop Lines Data Points Display Control using the existing saved symbol frequency.
Skip Points now selects the same rows for drop lines, symbols, error bars and data labels.
Origin 2016 PNG measurements establish the frequency, plot-range phase and absence of a forced
last point. Native/companion saves preserve the setting, and the `.opj` warning includes line plots
with skipped errors. The full-data connecting line and axes remain unchanged. No Line Break /
Gap to Symbol and modern Smart Skip modes remain open; filtered-row frequency is Sheaf's existing
compacted display order, not yet Origin-measured. See graphing section 18.

Validation after incorporating origin/main `2b16f6b`: formatting, strict all-target workspace
Clippy and 1,957 workspace tests pass (two ignored). All seven new regressions pass. The newly
merged MCP test fixture also needed a separate Windows-only dead-field portability fix.

## Label White Out follow-up (2026-11-14)

The Label tab now offers White Out (off by default), drawing an opaque white background for each
visible nonempty data label. Rotation, anchors, offsets, row selection and Skip Points are
retained. Origin 2016 exported examples confirm the white fill and rotation (graphing section 18);
exact font-box padding outside the measured Arial example remains approximate. Old settings
load with White Out off; native and `.opj` companion saves retain enabled settings. The existing
export-format smoke test exercises it, and six focused pixel/geometry/model/UI/IO regressions
cover the behavior. Label frames, editable margins and leader lines remain open.

Validation on integrated `756d12d`: formatting, strict workspace all-target Clippy and 1,968
workspace tests pass (two ignored), including all six new regressions.

## Today

`crates/app/src/dialogs/graph_details.rs` (plot, layer tabs) and `graph_props.rs` (page node titled
"Page Properties"). Window 900x560 (Origin about 1005x650). Layer Contents (F12) and Plot Setup
both open the Plot Details layer tab (`menus.rs`).

## Items

1. Plot tabs: Drop Lines, Label, Offset tabs; Line transparency; Fill Area Under Curve; Pattern
   colour/transparency/gradient (Pattern row and Spacing "Width (in %)" are greyed today).
2. Group tab: Edit Mode, increment rows, Group Members, Subgrouping (after `opj-groups-frames`
   merges, which models plot groups).
3. Layer: Size/Speed, Display tabs; Link Axes Scales; Background tab (greyed).
4. Page: Print/Dimensions, Legends/Titles, Miscellaneous, Display.
5. Plot tree text `[Book1]Sheet1! "Conc"(X), "Signal"(Y) [1*:10*]` instead of the short name.
6. Separate Layer Contents dialog (two lists, add/remove) and Plot Setup dialog, as Origin.
7. "Workbook" button and ">>" side panel.

Keep Sheaf's extra Left/Right Y choice on the Group tab (double-Y needs it).

## Done when

Each item done or noted; screenshots via `--show graph-settings:plots|page`; UI tests; checks pass.
