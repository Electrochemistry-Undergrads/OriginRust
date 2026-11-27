# T506: Chord diagram from a flow matrix

**Status:** done 2026-11-27 (Claude Windows "claude:3", graphing 14.34 "From a flow matrix"). Left item of T402
(claude:1's); Origin 2021's chord diagram was not measured.
**Size:** S. **Source:** T402 "Left".

## What it does

Plot > Categorical > Chord Diagram with one text column (the sources) and two or more numeric columns (one per target)
selected, or `sheaf plot -t chord --matrix [--group SOURCES] [-y TARGETS]`, reads a flow matrix: the flows go to a new
sheet `ChordFlows` (Source, Target, Weight) and the chord diagram plots them.

## Tests

- `crates/core/src/project.rs`: `a_chord_diagram_from_a_flow_matrix`.
- `crates/cli/tests/sankey.rs`: `a_chord_diagram_takes_its_ends_and_an_optional_weight` (`--matrix`).

## Left

- The rest of T402's list (arrow heads, ticks and values round the ring, label rotation, Plot Details). Rows and
  columns of the same name are already one node.
