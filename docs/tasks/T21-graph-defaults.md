# T21: Graph defaults as Origin (colours, fonts, line widths, ticks, symbols, dashes)

**Status:** done, merged (`b015f97`). Follow-ups on 2026-11-15: stock Line/Scatter/Line+Symbol/Column legend anchor and opaque white background copied (graphing 2.4); the writer stores the major tick length (opj-format 10.3; the reader reads it, minor = half). Column and Waterfall/Stacked/DoubleY retain the earlier font/colour defaults. Left: the 0.5 pt dash lengths are not linear in width; other template legend placements and dimensions; a chosen minor tick length is still not stored. **Size:** M.
**Branch:** `graph-defaults2`. **Model:** Sonnet.
**Detail:** audit C "Different, checked in Origin" and "Different, not verified"; open question U1 (colour
order). **Spec:** `docs/spec/graphing.md` sections 1.5 (colour list), 2.1 (dash patterns), 3 (axis).

## Stock legend placement follow-up (2026-11-15)

Nine corrected stock-template/resize exports and two generated-OPJ acceptance exports were
inspected in Origin 2016. Line, Scatter, Line + Symbol and Column use an upper-left anchor
at about (0.8223, -0.1044) of the layer frame, growing right/down with their content.
These four templates use Automatic and the measured opaque white default background. Other templates and saved corner/manual positions retain
their placement; Double-Y was measured and has a different page-attached legend. The existing
position chooser can restore Automatic. Native/companion files retain it; raw OPJ writes the
same rectangle anchor and reports its fixed-centre import so a companion preserves the mode.
The renderer's existing font metrics, padding and row spacing remain separate from placement.
Five regressions cover render geometry, position history/Cancel, and native/OPJ persistence.

Validation: formatting, strict workspace all-target Clippy and all 2,964 workspace tests pass
(eight existing ignored), including seven reviewed SVG goldens. Separate Windows Recent Files
test-helper normalization preserves exact path/Unicode assertions and changes no product code.

## Today

- Default plot colours: Sheaf's own 9-colour cycle in `crates/core/src/template.rs`. graphing.md 1.5 has
  a 23-entry list from the owner's theme; opj-format.md says a group colours "black, red, green, ...".
- Built-in templates (`Template::new_graph` in template.rs) set tick label font 12 pt, axis line 1.0 pt.
- Dash patterns are Sheaf's own (`crates/plot/src/plots.rs` or `scene.rs`, search `DashDotDot`):
  DashDotDot is 6:3:1:3:1:3 x width; the spec's one measured pattern is 18:12:5:12:5:12.

## Origin facts

- Measured (audit C, `re-workspace/scratch/audit-2/gr*.lines.txt`): tick label font 18 pt
  (`layer.x.label.pt`), axis line 1.5 pt, no top axis by default, 1 minor tick between majors (8 on log).
- Not measured yet: the colour order of a fresh overlay of several Y columns, title font size, symbol
  size, tick length, default tick direction (Sheaf: Out, spec [REC]), legend default text. Measure each
  on a fresh `plotxy` of 4 Y columns (LabTalk `get`/`layer.` properties, `%(1,@C)` style colour queries,
  `expGraph` with an explicit folder to look at the picture).
- The CV template's style (no grid, ticks in, closed box, symmetric current axis) is the owner's figure
  style (LESSONS, figure-rendering.md); keep it, change only the generic templates.

## Steps

1. Measure the unmeasured items; record each in graphing.md with the LabTalk used.
2. Change template defaults; colour list as Origin's (Sheaf's own colour values are fine if Origin's
   list is a named palette, but the order and count must match).
3. Dash patterns from the measured ratio (measure the others: dash, dot, dash dot).
4. Regenerate golden images only after looking at each (LESSONS), name them in the commit.
5. Check the `.opj` writer still writes "automatic" colours correctly (opj-format.md 13.12).

## Done when

Each default measured and copied (or recorded as unmeasurable); goldens reviewed; checks pass.
