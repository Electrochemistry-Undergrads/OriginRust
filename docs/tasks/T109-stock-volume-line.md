# T109: OHLC-Volume and Stock > Line

**Status:** claimed 2026-11-16 (Claude Windows "t30-origin", branch `stock-volume`).
**Size:** M. **Source:** greyed inventory package B4 (Plot > Stock > OHLC-Volume and Line), the
items T108 left.
**Spec:** `docs/spec/graphing.md` (section 31, Stock plots).

## Goal

The last two Plot > Stock items as Origin 2016's: OHLC-Volume (price bars or candles with a volume
column chart, from Open, High, Low, Close and Volume columns) and Stock > Line.

## Steps

1. Measure both in Origin 2016 with tools/origin-ref: the columns they take and refuse, the layers,
   the drawing, colours, axes and titles.
2. Build them on the T108 stock plot kinds (`sheaf_core::stock`, `sheaf-plot` `stock_plot.rs`), with
   templates and the menu items enabled.
3. Tests, graphing.md 31, greyed inventory recount.

## Done when

Both menu items plot as measured, the inventory no longer lists them, and fmt, strict Clippy and the
full tests pass.
