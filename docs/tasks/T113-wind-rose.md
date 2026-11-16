# T113: Wind Rose (Binned Data and Raw Data)

**Status:** claimed 2026-11-16 (Claude Windows "t30-origin", branch `wind-rose`).
**Size:** M. **Source:** greyed inventory package B4 (Plot > Specialized > Wind Rose - Binned Data,
Wind Rose - Raw Data; the Sheaf menu lists one "Wind Rose" entry).
**Spec:** `docs/spec/graphing.md` (a new section).

## Goal

Origin 2016's wind roses: stacked polar sectors by direction, coloured by speed band, from a binned
table (Binned Data) or from direction and speed columns (Raw Data).

## Steps

1. Measure both in Origin 2016 with tools/origin-ref: the columns they take, the sectors and speed
   bins, colours, the polar axes, labels and legend, and any dialog.
2. Build them on Sheaf's polar layers: the binning in `sheaf-calc` or `sheaf-core`, a plot kind or
   template, and the menu items.
3. Tests, a graphing.md section, greyed inventory recount.

## Done when

Both menu items plot as measured, the inventory no longer lists them, and fmt, strict Clippy and the
full tests pass.
