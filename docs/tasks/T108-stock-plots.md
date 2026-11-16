# T108: High-Low-Close, Japanese Candlestick, OHLC Bar Chart

**Status:** claimed 2026-11-16 (Claude Windows "t30-origin", branch `stock-plots`).
**Size:** M. **Source:** greyed inventory package B4 (Plot > Stock > High-Low-Close, Japanese
Candlestick, OHLC Bar Chart; Plot > Specialized > High-Low-Close; the 2D Graphs toolbar's
High-Low-Close). OHLC-Volume and Stock > Line stay in B4.
**Spec:** `docs/spec/graphing.md` (a new section).

## Goal

Stock charts as Origin 2016's: per row of X, a high-low bar with a close tick (HLC), a candle body from open
to close with wicks to high and low (candlestick), or a bar with open and close ticks (OHLC).

## Origin facts

Measure in Origin 2016: which columns each takes, the shapes, widths and colours (rising and falling
candles), the axis ranges.

## Done when

The items are enabled and draw as measured (fixtures), saved in `.sheaf`, fmt/clippy/tests pass, inventory
and spec updated.
