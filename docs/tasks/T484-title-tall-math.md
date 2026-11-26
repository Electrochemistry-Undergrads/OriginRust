# T484: Axis titles with tall math keep clear of the tick labels

**Status:** done 2026-11-26 (Claude Windows "claude:3", graphing "LaTeX math in graph text"). Left item of T483.
**Size:** S.

## What it does

The gap between an axis title and its tick labels (0.6 of the title size, measured from a letter's ascent or descent, as in Origin) is
measured from the ink of a stacked fraction, limits, stretchy delimiters or a matrix where they reach further (`text::tall_extent`); X,
Y and opposite-side titles alike. Plain titles and scripts keep their place.

## Tests

- `crates/plot/src/text_fraction_tests.rs`: `a_title_takes_more_room_only_for_tall_math`; every existing title placement test unchanged.
