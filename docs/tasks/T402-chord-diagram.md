# T402: Chord diagrams

**Status:** done 2026-11-23 (Claude Windows "claude:1", graphing 14.34), not measured (Origin 2021 feature).
Plot > Categorical > Chord Diagram and `sheaf plot -t chord [--group S,T] [-y WEIGHT]`: directed flows of a source
and a target column, counted or weighted, drawn as arcs round a circle sized by each name's flows and ribbons
between them. `PlotKind::Chord`, `sheaf_calc::chord`.
**Size:** S. **Source:** HANDOFF C, origin-2026-2027-gaps "Chord and lollipop plots".

## Tests

- `crates/calc/src/chord.rs`: arcs by total with gaps, outgoing then incoming parts, bad rows, flows to self, many
  nodes.
- `crates/plot/src/chord.rs`: names, a translucent ribbon per flow and no axes in the SVG.
- `crates/cli/tests/sankey.rs`: `a_chord_diagram_takes_its_ends_and_an_optional_weight`.
- `crates/app/src/app/ui_tests.rs`: `plot_categorical_chord_diagram_joins_the_sheets_first_two_text_columns`.

## Left

- A matrix as the input, arrow heads, ticks and values round the ring, label rotation, Plot Details.
