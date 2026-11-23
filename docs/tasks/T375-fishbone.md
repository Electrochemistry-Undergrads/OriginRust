# T375: Cause-and-effect (fishbone) diagram

**Status:** done 2026-11-22 (Claude Windows "claude:1", graphing 17.13), not measured (Origin 2026 feature).
Plot > Categorical > Cause-and-Effect Diagram and `sheaf plot -t fishbone [--effect TEXT]` draw the chosen
columns as categories and their cells as causes: a spine arrow to the effect, slanted ribs alternately above
and below, a twig per cause; all ordinary drawn objects and labels of one layer, editable afterwards.
**Size:** S. **Source:** HANDOFF C, origin-2026-2027-gaps "Cause-and-effect and nested frequency charts".

## Tests

- `crates/core/src/fishbone.rs`: the layout (counts, sides, twigs flat on their ribs, inside the frame) and the
  graph from text columns skipping empty cells.
- `crates/cli/tests/fishbone.rs`: `-t fishbone --effect`, `-y`, `--effect` refused elsewhere.
- `crates/app/src/app/ui_tests.rs`: `plot_categorical_cause_and_effect_draws_the_selected_columns_as_categories`.

## Left

- Text measurement (twig length from the cause's width), nested sub-causes, an effect box, Origin's look;
  nested frequency charts.
