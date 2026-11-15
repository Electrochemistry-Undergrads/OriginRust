# T41: Rug plots (axis rug marks)

**Status:** done 2026-11-15 (session "mac-multipdf", macOS, no Origin), branch `rug-plot`: Axis dialog Rug
tab (Show, Data by Y column, Color, Thickness, Transparency, Size, Overlap, Gap, Margin, borders), Plot >
Symbol > Scatter + Rug, `-t scatter-rug`, MCP; histograms mark raw observations; `.opj` warns. Left:
Origin's defaults and placement unmeasured, rug hit-testing (graphing.md 18.10).
**Size:** M. **Source:** `origin-2026-2027-gaps.md` (Graphing: Rug plot, Origin 2019b).

## Goal

An axis can show a rug: one short line per observation of the layer's plots, in a strip beside the
axis, set on the Axis dialog's Rug tab; Plot > Basic 2D Scatter + Rug makes a scatter with rugs on
the bottom and left axes.

## Origin facts (docs, not measured)

- 2019b: "Rug Plot - Add rug marks to axes", to indicate data density. [W:originlab.com/2019b]
- Graph type "Scatter+Rug": Plot > Basic 2D: Scatter + Rug, template `ScatterRug.OTP`. A rug is
  not a density plot: each point is projected onto the axis as a thin line beside it; duplicate
  values overlap. [W:origin-help/scatter-rug-graph]
- Axis dialog, Rug tab (2D XY Cartesian): Show (rug panel for the selected axis); Data (default
  `<All>`, or one plot); Color (default Auto = the plot's colour); Thickness; Transparency (%) 0-100;
  Layout: Size(%) (panel height as a percentage of the average of layer width and height), Overlap
  (checked by default for built-in rug templates such as Scatter Rug), Gap Between Rugs(%) (only when
  Overlap is cleared), Margin(%) (below 50), Show Perpendicular Border, Show Parallel Border.
  [W:origin-help/axesref-rug]
- Not stated: default Size, Thickness, Transparency, Margin; inside/outside placement; ticks/labels
  shift. Sheaf's choices labelled unmeasured in graphing.md.

## Where

- `crates/core/src/graph.rs`: an optional `Rug` on each axis side (serde default none); Template
  `ScatterRug`; `.opj` save warns.
- `crates/plot`: draw the rug strip beside the axis (outside the frame, pushing tick labels and
  title out by the strip size + margin), one segment per plotted point of the chosen plots, colour
  per plot or fixed, transparency, overlap vs stacked rugs with gaps, borders; layout/margins so
  nothing overlaps; scene items for hit testing if axes are pickable.
- `crates/app/src/dialogs/graph_axis.rs`: Rug tab; menus: Plot > Basic 2D (or the Symbol group as
  T33 did) "Scatter + Rug"; CLI `-t scatter-rug`, MCP template.

## Done when

Rugs draw from the Axis dialog and the template, in the app, exports and CLI; settings save in
`.sheaf`; geometry, dialog and template tests; graphing.md records Origin's facts and choices.

## Don't

- Don't change axis breaks, X:Y ratio (`xy-ratio` session) or graphs without rugs (byte-identical).
