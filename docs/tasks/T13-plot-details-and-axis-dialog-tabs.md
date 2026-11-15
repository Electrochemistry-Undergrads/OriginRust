# T13: Missing Plot Details tabs and controls

**Status:** base tabs merged from `plot-details` (2026-11-14); Data on Top of Axes, Skip Points, Label White Out, Offset multipliers, Label Frame/Margin, symbol gaps, Front/Behind line order and the sidelines/Inclusive fill modes completed in the follow-ups below. The measured parts were already on `main`. Measured against Origin (graphing spec 18, 18.1 to 18.5): Drop Lines and Label tabs, line Transparency, Fill Area Under Curve (Normal), the layer Background colour, the Group tab (Edit Mode, increments, members), Offset (X and Y offsets), Display, Size/Speed (units), Link Axes Scales, Stack, the page tabs Print/Dimensions, Display and Legends/Titles, the tree names, Layer Contents and Plot Setup as Origin's own windows, the Workbook and ">>" buttons. Ported from the branch onto those dialogs, **Sheaf's own and not measured** (spec 18.6): a hatch, gradient and transparency for the fill of columns, bars, areas and the fill under a curve (Pattern tab; for areas too); bar Width (in %) on Spacing; Label by a column of the project, a list of rows, decimals and offsets; subgroups by size on the Group tab; the layer Background gradient and border; the X Axes and Y Axes switches of Display; the page's border, layer icons switch, comments (hover text in the Object Manager), window title choice, Miscellaneous tab, and Print/Dimensions keeping the layers' size; the right Y layer no longer copies its parent's background; drop lines are drawn only for the kinds whose tab offers them; deleting a column resets the X error, base and data labels that name it; "Use this style for all plots" copies the look settings. Not taken from the branch because main's measured version replaced it: the second Layer Contents and Plot Setup dialogs, the second Group, Drop Lines, Label and Link tabs, its tree text, its Workbook button and side panel, line transparency as a percentage field. **Left:** the other fill modes and Origin's pattern list, several groups per layer, subgrouping by column label, Binned, Size/Speed X:Y ratio, caching and speed mode, label leader lines, print margins, writing the new settings to `.opj` (the writer names them in its save warnings). **Unmeasured:** everything in spec 18.6 (compare with Origin on the Windows machine). **Size:** M. **Model:** Sonnet. X:Y ratio (Size/Speed Link Axis Length to Scale) measured and done 2026-11-15, graphing 18.7a (branch `xy-ratio`).
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
with skipped errors. The full-data connecting line and axes remain unchanged by Skip Points alone. Symbol gaps and
No Line Break are covered by the later follow-up below; modern Smart Skip remains open; filtered-row frequency is Sheaf's existing
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

## Offset multipliers follow-up (2026-11-14)

The Offset tab now offers X and Y multipliers (default 1) beside its existing offsets, enabled
in Individual mode. Positive factors scale source values before the offsets. Origin 2016
measurements confirm fractional factors, unchanged error magnitudes and source label values;
stored zero/negative factors render as 1. Nonfinite factors also render as 1. Source data,
analysis and fit results remain unchanged. Followers inherit the leader's complete placement.
The Data Reader follows displayed points while reporting source values; the Integrate gadget's
range, shading and dragging follow placement while its area stays in source units.
Native and companion saves preserve the factors. The `.opj` writer explicitly warns that
multipliers are omitted, retaining the original data and its existing offset writing.
Separate Individual X/Y switches and specialized-coordinate parity remain open (graphing 18.1).

Validation on integrated `f14e20f` plus the docs-only T01 claim `89473a2`: formatting, strict
workspace all-target Clippy and 1,993 workspace tests pass (two ignored). Ten new regressions
cover model compatibility, Origin placements/error lengths/labels, raster and picking, log/range
handling, native/companion saves, dialog history, Data Reader and gadget source-unit behavior.

## Label Frame and Margin follow-up (2026-11-14)

The Label tab offers None, Box and Shadow, frame border/fill colors and Margin (% Font Height).
Origin 2016 exports establish the black initial colors, frame/margin versus White Out paint order,
page-aligned enclosing frame for rotated text and fixed lower-right shadow at two font sizes.
The shared screen/vector scene retains row selection, Skip Points, empty-text handling and clipping.
Existing approximate font metrics remain explicit in graphing 18; exact padding for other fonts
is not claimed. Old settings load with None/zero margin; native and companion saves preserve all
fields even while labels are disabled. The `.opj` writer already warns for unsupported label styles.
Leader lines, automatic repositioning and other remaining T13 controls stay open.

Validation on integrated `bf3dddd` plus the docs-only print/recovery claims: formatting, strict
workspace all-target Clippy and 2,142 workspace tests pass (three existing ignored). Seven new
core/plot/UI/IO regressions pass, including raster paint order, rotation/shadow geometry and all
vector backends. The full suite exposed a separate upstream autosave UI fixture collision;
`planted` now gives each recovery case its own input fixture, and all 25 autosave tests pass with
16 test threads. No autosave implementation changed.

## Symbol/line gaps follow-up (2026-11-14)

Line + Symbol offers Gap to Symbol on Line, and No Line Break for Skipped Point(s) under
Drop Lines when gaps and Skip Points are enabled. Origin 2016 horizontal exports confirm that
gaps otherwise apply at every source position, including skipped symbols; No Line Break keeps
only displayed-marker gaps while the connecting line still follows every raw source value.
Both switches default off. The shared scene removes line geometry, preserving backgrounds,
grids and fills; masks keep the same holes, and screen/raster/vector exports share the result.
Native and companion saves preserve the switches, with an explicit omission warning for `.opj`.
Current fixed 30% clearance and other limits are recorded in graphing 18; Front/Behind is covered below.

Eight new regressions cover old model defaults/group edits, raster/picking, skipped peaks and
range phase, missing values, dense/overlapping/coincident markers, all connection/export paths,
UI dependencies/Apply/Cancel/history and native/companion saves.
The clean three-point diagonal Origin control confirms identical line pixels for square/circle
markers at the same nominal size; the earlier invalid circular capture is excluded.

Validation on integrated `eaa6a5c`: formatting, strict workspace all-target Clippy and
2,226 workspace tests pass (three existing ignored). All reference workflows and the final
builder exited before merge; all eight new regressions pass.

## Draw Line in Front/Behind follow-up (2026-11-15)

The Line + Symbol Line tab offers the two Origin radios, defaulting to Behind. Front paints the
line over symbols in the data plot AND legend sample. The radios are disabled by Gap to Symbol,
which overrides the retained choice. Other plot kinds ignore it. Old projects keep Behind;
Dependent group edits and native/companion saves preserve the flag, and the `.opj` warning
explicitly names omitted draw order. No source values or axes change.

Origin 2016 verified exports show blue versus red at both data and legend marker centres;
control captures show Front still selected while disabled by gaps. The interrupted first export
sequence is excluded from raster evidence (graphing 18). Five new regressions cover plot/legend
pixels, shared vector backends, gap dependencies, dialog group editing/history and both saves;
the existing model regression also covers old defaults and copying the new field.

Validation on integrated Go To `cb1270a` plus the docs-only scope update: formatting, strict
workspace all-target Clippy and 2,243 workspace tests pass (three existing ignored). All five
new regressions and the extended model fixture pass. All Origin workflows/builders exited
before merge; shared pending files and graphify artifacts remain outside this commit.

## Sidelines fill modes follow-up (2026-11-15)

The Line tab offers Normal, Sidelines only and Sidelines and Base. Normal stays the old default;
the other two use an unfilled line-coloured outline to the frame bottom, with Base closing the
bottom edge. Origin 2016 verified mode/color/negative-axis examples establish this behavior and
the line legend sample. Verified missing cells show boundaries only at the outer endpoints,
without internal sidelines or a line through the isolated middle point. Invalid initial mode
selector and missing-value attempts are explicitly excluded in graphing 18.

One stroke preserves transparency at joins; existing curves, clipping and row selection stay
in use. Group editing, Apply/Cancel/history and native/companion saves preserve the mode; old
projects keep Normal and `.opj` retains the existing unsupported-fill warning. Eight new
core/plot/UI/IO regressions cover these behaviors, hidden-mask endpoint integration and every vector backend. Other fill modes,
custom baselines and specialised-coordinate parity remain open.

Validation on integrated violin `9af42a9` plus the docs-only scope updates: formatting, strict
workspace all-target Clippy and 2,609 workspace tests pass (five existing ignored). All eight
new regressions pass, along with the latest MCP attach tests after a Unix-only argument and
three Windows canonical-path fixture expectations were made portable. All reference workflows
and the validation builder exited; a native Plot Details preview is retained locally.

## Inclusive fill follow-up (2026-11-15)

The Line tab offers Inclusive broken by missing values. Each finite run fills to its own
endpoint chord and closes with the connecting stroke. Missing values separate the polygons;
a one-point run has no fill but retains its round stroke cap. Corrected Origin 2016 controls,
exports and explicit ISNA checks establish these rules; the invalid initial capture batch is
excluded (graphing 18). Fill styling and a filled legend remain active. Normal stays the default;
group editing, Apply/Cancel/history and native/companion saves retain the mode, with the existing
`.opj` warning. Four new renderer/UI regressions and the existing all-mode model/save tests cover
geometry, alpha joins, connections/exports, saved preferences and editing history. Other fill
modes, alternate baselines and specialised-coordinate parity remain open.

Validation on integrated `b66251e` plus the docs-only scope update: formatting, strict workspace
all-target Clippy and all 2,616 workspace tests pass (seven existing ignored). All four new
regressions and the extended all-mode model/save cases pass.
All three corrected Origin cases have verified applied controls and exports; all reference
workflows and builders exited before merge.

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
